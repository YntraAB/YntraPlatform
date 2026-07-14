use crate::infra::errors::YntraError;

pub struct ZeroCopyEngine {
    #[cfg(not(target_arch = "wasm32"))]
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
        let _ = &file_path;
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
            Ok(Self {
                buffer: rkyv::util::AlignedVec::<16>::new(),
                loro,
            })
        }
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
            // Drop memory mapping and close file to release Windows OS locks
            self.mmap = None;
            self.file = None;

            let temp_path = format!("{}.tmp", self._file_path);

            {
                use std::io::Write;
                let mut temp_file = std::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(&temp_path)
                    .map_err(|e| YntraError::DbError(e.to_string()))?;

                temp_file
                    .write_all(&rkyv_len.to_be_bytes())
                    .map_err(|e| YntraError::DbError(e.to_string()))?;
                // Write 8 padding bytes to align rkyv_bytes to 16-byte boundary
                temp_file
                    .write_all(&[0u8; 8])
                    .map_err(|e| YntraError::DbError(e.to_string()))?;
                temp_file
                    .write_all(rkyv_bytes)
                    .map_err(|e| YntraError::DbError(e.to_string()))?;
                temp_file
                    .write_all(loro_bytes)
                    .map_err(|e| YntraError::DbError(e.to_string()))?;

                temp_file
                    .sync_all()
                    .map_err(|e| YntraError::DbError(e.to_string()))?;
            }

            std::fs::rename(&temp_path, &self._file_path)
                .map_err(|e| YntraError::DbError(e.to_string()))?;

            let file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&self._file_path)
                .map_err(|e| YntraError::DbError(e.to_string()))?;

            let m = unsafe {
                memmap2::MmapMut::map_mut(&file)
                    .map_err(|e| YntraError::DbError(e.to_string()))?
            };

            self.file = Some(file);
            self.mmap = Some(m);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let mut buf = rkyv::util::AlignedVec::<16>::new();
            buf.extend_from_slice(&rkyv_len.to_be_bytes());
            buf.extend_from_slice(&[0u8; 8]); // 8 padding bytes for 16-byte alignment
            buf.extend_from_slice(rkyv_bytes);
            buf.extend_from_slice(loro_bytes);
            self.buffer = buf;
        }
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn load_loro_from_mmap(&mut self) -> Result<(), YntraError> {
        if let Some(ref m) = self.mmap {
            if m.len() >= 16 {
                let rkyv_len = u64::from_be_bytes(m[0..8].try_into().unwrap()) as usize;
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

    pub fn get_loro_changes(&self) -> Result<Vec<u8>, YntraError> {
        self.loro
            .export(loro::ExportMode::Snapshot)
            .map_err(|e| YntraError::SerializationError(e.to_string()))
    }

    pub fn apply_loro_update(&mut self, update_bytes: &[u8]) -> Result<(), YntraError> {
        self.loro
            .import(update_bytes)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;

        let map = self.loro.get_map("db");
        if let Some(val) = map.get("bytes") {
            if let Some(val_ref) = val.as_value() {
                if let Some(bytes) = val_ref.as_binary() {
                    let loro_bytes = self
                        .loro
                        .export(loro::ExportMode::Snapshot)
                        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                    self.save_to_disk(bytes, &loro_bytes)?;
                }
            }
        }
        Ok(())
    }
    pub fn write_serialized(&mut self, rkyv_bytes: &[u8]) -> Result<(), YntraError> {
        let map = self.loro.get_map("db");
        let _ = map.insert("bytes", rkyv_bytes.to_vec());

        let loro_bytes = self
            .loro
            .export(loro::ExportMode::Snapshot)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;

        self.save_to_disk(rkyv_bytes, &loro_bytes)
    }

    pub fn get_rkyv_slice(&self) -> &[u8] {
        let bytes = self.get_bytes();
        if bytes.len() < 16 {
            return &[];
        }
        let rkyv_len = u64::from_be_bytes(bytes[0..8].try_into().unwrap()) as usize;
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
}
