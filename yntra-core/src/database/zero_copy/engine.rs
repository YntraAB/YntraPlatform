use crate::infra::errors::YntraError;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = yntra_save_store_bin, catch)]
    async fn js_save_store_bin(file_name: &str, data: &js_sys::Uint8Array) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;

    #[wasm_bindgen(js_name = yntra_load_store_bin, catch)]
    async fn js_load_store_bin(file_name: &str) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;
}

pub struct ZeroCopyEngine {
    _file_path: String,
    #[cfg(not(target_arch = "wasm32"))]
    file: Option<std::fs::File>,
    #[cfg(not(target_arch = "wasm32"))]
    mmap: Option<memmap2::MmapMut>,
    #[cfg(target_arch = "wasm32")]
    buffer: rkyv::util::AlignedVec<16>,
    loro: loro::LoroDoc,
}

impl ZeroCopyEngine {
    pub fn new(file_path: String) -> Result<Self, YntraError> {
        let loro = loro::LoroDoc::new();

        #[cfg(not(target_arch = "wasm32"))]
        {
            let file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&file_path)
                .map_err(|e| YntraError::DbError(e.to_string()))?;

            let metadata = file
                .metadata()
                .map_err(|e| YntraError::DbError(e.to_string()))?;
            let len = metadata.len();

            let mmap = if len > 0 {
                let m = unsafe {
                    memmap2::MmapMut::map_mut(&file)
                        .map_err(|e| YntraError::DbError(e.to_string()))?
                };
                Some(m)
            } else {
                None
            };

            let mut engine = Self {
                _file_path: file_path,
                file: Some(file),
                mmap,
                loro,
            };

            engine.load_loro_from_mmap()?;

            Ok(engine)
        }

        #[cfg(target_arch = "wasm32")]
        {
            let buffer = rkyv::util::AlignedVec::<16>::new();
            let engine = Self {
                _file_path: file_path,
                buffer,
                loro,
            };
            Ok(engine)
        }
    }

    pub fn file_path(&self) -> &str {
        &self._file_path
    }

    pub fn get_bytes(&self) -> &[u8] {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.mmap.as_ref().map(|m| &m[..]).unwrap_or(&[])
        }
        #[cfg(target_arch = "wasm32")]
        {
            &self.buffer
        }
    }

    pub fn save_to_disk(&mut self, rkyv_bytes: &[u8], loro_bytes: &[u8]) -> Result<(), YntraError> {
        let rkyv_len = rkyv_bytes.len() as u64;
        #[cfg(not(target_arch = "wasm32"))]
        {
            let total_size = 16 + rkyv_bytes.len() + loro_bytes.len();
            
            // Check if size has changed, and if so, resize the file and recreate mapping
            let current_size = self.mmap.as_ref().map(|m| m.len()).unwrap_or(0);
            if current_size != total_size {
                self.mmap = None;
                let file = self.file.as_ref().ok_or_else(|| YntraError::DbError("Database file closed".to_string()))?;
                file.set_len(total_size as u64)
                    .map_err(|e| YntraError::DbError(format!("Failed to resize database file: {}", e)))?;
                let m = unsafe {
                    memmap2::MmapMut::map_mut(file)
                        .map_err(|e| YntraError::DbError(format!("Failed to remap database file: {}", e)))?
                };
                self.mmap = Some(m);
            }

            // Write data directly into the memory mapped slice
            if let Some(ref mut m) = self.mmap {
                m[0..8].copy_from_slice(&rkyv_len.to_be_bytes());
                m[8..16].copy_from_slice(&[0u8; 8]);
                
                let rkyv_end = 16 + rkyv_bytes.len();
                m[16..rkyv_end].copy_from_slice(rkyv_bytes);
                m[rkyv_end..total_size].copy_from_slice(loro_bytes);
                
                m.flush()
                    .map_err(|e| YntraError::DbError(format!("Failed to flush database changes: {}", e)))?;
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let mut buf = rkyv::util::AlignedVec::<16>::new();
            buf.extend_from_slice(&rkyv_len.to_be_bytes());
            buf.extend_from_slice(&[0u8; 8]); // 8 padding bytes for 16-byte alignment
            buf.extend_from_slice(rkyv_bytes);
            buf.extend_from_slice(loro_bytes);

            // Save to OPFS asynchronously in a spawned task
            let file_path = self._file_path.clone();
            let bytes_vec = buf.to_vec();
            wasm_bindgen_futures::spawn_local(async move {
                let array = js_sys::Uint8Array::from(&bytes_vec[..]);
                let _ = js_save_store_bin(&file_path, &array).await;
            });

            self.buffer = buf;
        }
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn load_loro_from_mmap(&mut self) -> Result<(), YntraError> {
        if let Some(ref m) = self.mmap {
            if m.len() >= 16 {
                let rkyv_len = usize::try_from(u64::from_be_bytes(m[0..8].try_into().unwrap()))
                    .map_err(|e| YntraError::DbError(format!("Database size overflow: {}", e)))?;
                if let Some(loro_offset) = rkyv_len.checked_add(16) {
                    if m.len() >= loro_offset {
                        if m.len() > loro_offset {
                            let loro_bytes = &m[loro_offset..];
                            if let Err(e) = self.loro.import(loro_bytes) {
                                tracing::warn!("Failed to import Loro state on startup: {:?}", e);
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    fn load_loro_from_buffer(&mut self) -> Result<(), YntraError> {
        let len = self.buffer.len();
        if len >= 16 {
            let rkyv_len = usize::try_from(u64::from_be_bytes(self.buffer[0..8].try_into().unwrap()))
                .map_err(|e| YntraError::DbError(format!("Database size overflow: {}", e)))?;
            if let Some(loro_offset) = rkyv_len.checked_add(16) {
                if len >= loro_offset {
                    if len > loro_offset {
                        let loro_bytes = &self.buffer[loro_offset..];
                        if let Err(e) = self.loro.import(loro_bytes) {
                            tracing::warn!("Failed to import Loro state on startup: {:?}", e);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn get_loro_changes(&self) -> Result<Vec<u8>, YntraError> {
        self.loro
            .export(loro::ExportMode::Snapshot)
            .map_err(|e| YntraError::SerializationError(e.to_string()))
    }

    pub fn get_rkyv_slice(&self) -> &[u8] {
        let bytes = self.get_bytes();
        if bytes.len() < 16 {
            return &[];
        }
        let rkyv_len = match usize::try_from(u64::from_be_bytes(bytes[0..8].try_into().unwrap())) {
            Ok(len) => len,
            Err(_) => return &[],
        };
        if let Some(total_len) = rkyv_len.checked_add(16) {
            if bytes.len() >= total_len {
                return &bytes[16..total_len];
            }
        }
        &[]
    }

    pub fn doc(&self) -> &loro::LoroDoc {
        &self.loro
    }

    #[allow(unused_variables)]
    pub fn load_from_bytes(&mut self, bytes: &[u8]) -> Result<(), YntraError> {
        #[cfg(target_arch = "wasm32")]
        {
            self.buffer.clear();
            self.buffer.extend_from_slice(bytes);
            self.load_loro_from_buffer()?;
        }
        Ok(())
    }
}
