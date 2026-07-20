use crate::ZeroCopyNoteStore;
use crate::infra::errors::YntraError;
use std::collections::HashMap;
use std::sync::{Mutex, LazyLock};

static NOTE_STORES: LazyLock<Mutex<HashMap<String, ZeroCopyNoteStore>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[cfg(target_arch = "wasm32")]
fn get_note_store_path(workspace_id: &str) -> String {
    format!("yntra_zero_copy_notes_{}.db", workspace_id)
}

#[cfg(not(target_arch = "wasm32"))]
fn get_note_store_path(workspace_id: &str) -> String {
    if cfg!(test) {
        std::env::temp_dir()
            .join(format!("yntra_zero_copy_notes_{}.db", workspace_id))
            .to_string_lossy()
            .to_string()
    } else {
        crate::database::native::get_database_path(&format!("yntra_zero_copy_notes_{}.db", workspace_id))
    }
}

pub fn get_note_store(workspace_id: &str) -> ZeroCopyNoteStore {
    let mut stores = NOTE_STORES.lock().unwrap_or_else(|e| e.into_inner());
    stores
        .entry(workspace_id.to_string())
        .or_insert_with(|| {
            let path = get_note_store_path(workspace_id);
            ZeroCopyNoteStore::new(path).expect("Failed to initialize ZeroCopyNoteStore")
        })
        .clone()
}

pub async fn load_notes_from_opfs_internal(workspace_id: &str) -> Result<(), YntraError> {
    let store = get_note_store(workspace_id);
    store.load_from_opfs().await?;
    Ok(())
}

#[uniffi::export]
pub async fn load_notes_from_opfs(workspace_id: String) -> Result<(), YntraError> {
    load_notes_from_opfs_internal(&workspace_id).await
}
