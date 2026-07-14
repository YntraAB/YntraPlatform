use crate::infra::errors::YntraError;
use crate::models::{AuditLogEntry, DailyNote, MessageItem, TodoItem};
use std::sync::{Arc, Mutex};
use super::engine::ZeroCopyEngine;

// --- Field Helpers for Loro Map Deserialization ---

fn get_string(map: &loro::LoroMap, key: &str) -> Result<String, YntraError> {
    match map.get(key) {
        Some(loro::ValueOrContainer::Value(loro::LoroValue::String(s))) => Ok(s.as_ref().to_string()),
        _ => Err(YntraError::SerializationError(format!("Missing or invalid field: {}", key))),
    }
}

fn get_opt_string(map: &loro::LoroMap, key: &str) -> Result<Option<String>, YntraError> {
    match map.get(key) {
        Some(loro::ValueOrContainer::Value(loro::LoroValue::String(s))) => Ok(Some(s.as_ref().to_string())),
        Some(loro::ValueOrContainer::Value(loro::LoroValue::Null)) | None => Ok(None),
        _ => Err(YntraError::SerializationError(format!("Invalid field: {}", key))),
    }
}

fn get_bool(map: &loro::LoroMap, key: &str) -> Result<bool, YntraError> {
    match map.get(key) {
        Some(loro::ValueOrContainer::Value(loro::LoroValue::Bool(b))) => Ok(b),
        _ => Err(YntraError::SerializationError(format!("Missing or invalid field: {}", key))),
    }
}

fn get_i64(map: &loro::LoroMap, key: &str) -> Result<i64, YntraError> {
    match map.get(key) {
        Some(loro::ValueOrContainer::Value(loro::LoroValue::I64(v))) => Ok(v),
        _ => Err(YntraError::SerializationError(format!("Missing or invalid field: {}", key))),
    }
}

// --- Entity Loro Synchronization & Retrieval Functions ---

fn sync_todos_to_loro(loro: &loro::LoroDoc, todos: &[TodoItem]) -> Result<(), YntraError> {
    let db_map = loro.get_map("db");
    let mut current_keys = std::collections::HashSet::new();
    for todo in todos {
        current_keys.insert(todo.id.clone());
        let item_map = match db_map.get(&todo.id) {
            Some(loro::ValueOrContainer::Container(loro::Container::Map(m))) => m,
            _ => db_map
                .insert_container(&todo.id, loro::LoroMap::new())
                .map_err(|e| YntraError::SerializationError(e.to_string()))?,
        };
        item_map.insert("id", todo.id.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        item_map.insert("workspace_id", todo.workspace_id.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        item_map.insert("text", todo.text.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        item_map.insert("completed", todo.completed).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        item_map.insert("updated_at", todo.updated_at).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        item_map.insert("sync_status", todo.sync_status.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
    }
    let mut keys_to_delete = Vec::new();
    db_map.for_each(|k, _| {
        if !current_keys.contains(k) {
            keys_to_delete.push(k.to_string());
        }
    });
    for k in keys_to_delete {
        db_map.delete(&k).map_err(|e| YntraError::SerializationError(e.to_string()))?;
    }
    Ok(())
}

fn read_all_todos_from_loro(loro: &loro::LoroDoc) -> Result<Vec<TodoItem>, YntraError> {
    let db_map = loro.get_map("db");
    let mut todos = Vec::new();
    let mut err = None;
    db_map.for_each(|_k, val| {
        if err.is_some() {
            return;
        }
        if let loro::ValueOrContainer::Container(loro::Container::Map(item_map)) = val {
            let id = match get_string(&item_map, "id") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            let workspace_id = match get_string(&item_map, "workspace_id") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            let text = match get_string(&item_map, "text") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            let completed = match get_bool(&item_map, "completed") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            let updated_at = match get_i64(&item_map, "updated_at") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            let sync_status = match get_string(&item_map, "sync_status") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            todos.push(TodoItem {
                id,
                workspace_id,
                text,
                completed,
                updated_at,
                sync_status,
            });
        }
    });
    if let Some(e) = err {
        return Err(e);
    }
    Ok(todos)
}

fn sync_messages_to_loro(loro: &loro::LoroDoc, messages: &[MessageItem]) -> Result<(), YntraError> {
    let db_map = loro.get_map("db");
    let mut current_keys = std::collections::HashSet::new();
    for msg in messages {
        current_keys.insert(msg.id.clone());
        let item_map = match db_map.get(&msg.id) {
            Some(loro::ValueOrContainer::Container(loro::Container::Map(m))) => m,
            _ => db_map
                .insert_container(&msg.id, loro::LoroMap::new())
                .map_err(|e| YntraError::SerializationError(e.to_string()))?,
        };
        item_map.insert("id", msg.id.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        item_map.insert("workspace_id", msg.workspace_id.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        match &msg.sender_id {
            Some(v) => item_map.insert("sender_id", v.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?,
            None => item_map.insert("sender_id", loro::LoroValue::Null).map_err(|e| YntraError::SerializationError(e.to_string()))?,
        }
        match &msg.receiver_id {
            Some(v) => item_map.insert("receiver_id", v.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?,
            None => item_map.insert("receiver_id", loro::LoroValue::Null).map_err(|e| YntraError::SerializationError(e.to_string()))?,
        }
        match &msg.target_team_id {
            Some(v) => item_map.insert("target_team_id", v.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?,
            None => item_map.insert("target_team_id", loro::LoroValue::Null).map_err(|e| YntraError::SerializationError(e.to_string()))?,
        }
        match &msg.subject {
            Some(v) => item_map.insert("subject", v.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?,
            None => item_map.insert("subject", loro::LoroValue::Null).map_err(|e| YntraError::SerializationError(e.to_string()))?,
        }
        match &msg.body {
            Some(v) => item_map.insert("body", v.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?,
            None => item_map.insert("body", loro::LoroValue::Null).map_err(|e| YntraError::SerializationError(e.to_string()))?,
        }
        item_map.insert("is_read", msg.is_read).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        item_map.insert("created_at", msg.created_at.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        item_map.insert("updated_at", msg.updated_at).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        item_map.insert("sync_status", msg.sync_status.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
    }
    let mut keys_to_delete = Vec::new();
    db_map.for_each(|k, _| {
        if !current_keys.contains(k) {
            keys_to_delete.push(k.to_string());
        }
    });
    for k in keys_to_delete {
        db_map.delete(&k).map_err(|e| YntraError::SerializationError(e.to_string()))?;
    }
    Ok(())
}

fn read_all_messages_from_loro(loro: &loro::LoroDoc) -> Result<Vec<MessageItem>, YntraError> {
    let db_map = loro.get_map("db");
    let mut messages = Vec::new();
    let mut err = None;
    db_map.for_each(|_k, val| {
        if err.is_some() {
            return;
        }
        if let loro::ValueOrContainer::Container(loro::Container::Map(item_map)) = val {
            let id = match get_string(&item_map, "id") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            let workspace_id = match get_string(&item_map, "workspace_id") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            let sender_id = match get_opt_string(&item_map, "sender_id") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            let receiver_id = match get_opt_string(&item_map, "receiver_id") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            let target_team_id = match get_opt_string(&item_map, "target_team_id") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            let subject = match get_opt_string(&item_map, "subject") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            let body = match get_opt_string(&item_map, "body") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            let is_read = match get_bool(&item_map, "is_read") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            let created_at = match get_string(&item_map, "created_at") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            let updated_at = match get_i64(&item_map, "updated_at") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            let sync_status = match get_string(&item_map, "sync_status") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            messages.push(MessageItem {
                id,
                workspace_id,
                sender_id,
                receiver_id,
                target_team_id,
                subject,
                body,
                is_read,
                created_at,
                updated_at,
                sync_status,
            });
        }
    });
    if let Some(e) = err {
        return Err(e);
    }
    Ok(messages)
}

fn sync_audits_to_loro(loro: &loro::LoroDoc, entries: &[AuditLogEntry]) -> Result<(), YntraError> {
    let db_map = loro.get_map("db");
    let mut current_keys = std::collections::HashSet::new();
    for entry in entries {
        current_keys.insert(entry.id.clone());
        let item_map = match db_map.get(&entry.id) {
            Some(loro::ValueOrContainer::Container(loro::Container::Map(m))) => m,
            _ => db_map
                .insert_container(&entry.id, loro::LoroMap::new())
                .map_err(|e| YntraError::SerializationError(e.to_string()))?,
        };
        item_map.insert("id", entry.id.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        item_map.insert("workspace_id", entry.workspace_id.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        item_map.insert("actor_id", entry.actor_id.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        match &entry.target_client_id {
            Some(v) => item_map.insert("target_client_id", v.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?,
            None => item_map.insert("target_client_id", loro::LoroValue::Null).map_err(|e| YntraError::SerializationError(e.to_string()))?,
        }
        item_map.insert("action_type", entry.action_type.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        item_map.insert("timestamp", entry.timestamp).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        item_map.insert("prev_hash", entry.prev_hash.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        item_map.insert("curr_hash", entry.curr_hash.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        item_map.insert("seq", entry.seq).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        match &entry.signature {
            Some(v) => item_map.insert("signature", v.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?,
            None => item_map.insert("signature", loro::LoroValue::Null).map_err(|e| YntraError::SerializationError(e.to_string()))?,
        }
    }
    let mut keys_to_delete = Vec::new();
    db_map.for_each(|k, _| {
        if !current_keys.contains(k) {
            keys_to_delete.push(k.to_string());
        }
    });
    for k in keys_to_delete {
        db_map.delete(&k).map_err(|e| YntraError::SerializationError(e.to_string()))?;
    }
    Ok(())
}

fn read_all_audits_from_loro(loro: &loro::LoroDoc) -> Result<Vec<AuditLogEntry>, YntraError> {
    let db_map = loro.get_map("db");
    let mut entries = Vec::new();
    let mut err = None;
    db_map.for_each(|_k, val| {
        if err.is_some() {
            return;
        }
        if let loro::ValueOrContainer::Container(loro::Container::Map(item_map)) = val {
            let id = match get_string(&item_map, "id") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            let workspace_id = match get_string(&item_map, "workspace_id") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            let actor_id = match get_string(&item_map, "actor_id") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            let target_client_id = match get_opt_string(&item_map, "target_client_id") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            let action_type = match get_string(&item_map, "action_type") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            let timestamp = match get_i64(&item_map, "timestamp") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            let prev_hash = match get_string(&item_map, "prev_hash") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            let curr_hash = match get_string(&item_map, "curr_hash") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            let seq = match get_i64(&item_map, "seq") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            let signature = match get_opt_string(&item_map, "signature") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            entries.push(AuditLogEntry {
                id,
                workspace_id,
                actor_id,
                target_client_id,
                action_type,
                timestamp,
                prev_hash,
                curr_hash,
                seq,
                signature,
            });
        }
    });
    if let Some(e) = err {
        return Err(e);
    }
    Ok(entries)
}

fn sync_notes_to_loro(loro: &loro::LoroDoc, notes: &[DailyNote]) -> Result<(), YntraError> {
    let db_map = loro.get_map("db");
    let mut current_keys = std::collections::HashSet::new();
    for note in notes {
        current_keys.insert(note.id.clone());
        let item_map = match db_map.get(&note.id) {
            Some(loro::ValueOrContainer::Container(loro::Container::Map(m))) => m,
            _ => db_map
                .insert_container(&note.id, loro::LoroMap::new())
                .map_err(|e| YntraError::SerializationError(e.to_string()))?,
        };
        item_map.insert("id", note.id.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        item_map.insert("workspace_id", note.workspace_id.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        item_map.insert("team_id", note.team_id.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        match &note.author_id {
            Some(v) => item_map.insert("author_id", v.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?,
            None => item_map.insert("author_id", loro::LoroValue::Null).map_err(|e| YntraError::SerializationError(e.to_string()))?,
        }
        item_map.insert("subject", note.subject.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        item_map.insert("content", note.content.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        item_map.insert("edit_history", note.edit_history.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        item_map.insert("created_at", note.created_at.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        item_map.insert("updated_at", note.updated_at).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        item_map.insert("sync_status", note.sync_status.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
    }
    let mut keys_to_delete = Vec::new();
    db_map.for_each(|k, _| {
        if !current_keys.contains(k) {
            keys_to_delete.push(k.to_string());
        }
    });
    for k in keys_to_delete {
        db_map.delete(&k).map_err(|e| YntraError::SerializationError(e.to_string()))?;
    }
    Ok(())
}

fn read_all_notes_from_loro(loro: &loro::LoroDoc) -> Result<Vec<DailyNote>, YntraError> {
    let db_map = loro.get_map("db");
    let mut notes = Vec::new();
    let mut err = None;
    db_map.for_each(|_k, val| {
        if err.is_some() {
            return;
        }
        if let loro::ValueOrContainer::Container(loro::Container::Map(item_map)) = val {
            let id = match get_string(&item_map, "id") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            let workspace_id = match get_string(&item_map, "workspace_id") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            let team_id = match get_string(&item_map, "team_id") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            let author_id = match get_opt_string(&item_map, "author_id") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            let subject = match get_string(&item_map, "subject") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            let content = match get_string(&item_map, "content") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            let edit_history = match get_string(&item_map, "edit_history") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            let created_at = match get_string(&item_map, "created_at") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            let updated_at = match get_i64(&item_map, "updated_at") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            let sync_status = match get_string(&item_map, "sync_status") {
                Ok(v) => v,
                Err(e) => { err = Some(e); return; }
            };
            notes.push(DailyNote {
                id,
                workspace_id,
                team_id,
                author_id,
                subject,
                content,
                edit_history,
                created_at,
                updated_at,
                sync_status,
            });
        }
    });
    if let Some(e) = err {
        return Err(e);
    }
    Ok(notes)
}

// --- Helper Macros to Reduce Code Duplication ---

macro_rules! impl_write_items {
    ($self:expr, $items:expr, $sync_fn:path) => {{
        {
            let mut inner = $self.inner.lock().unwrap();
            let mut cache = $self.cache.lock().unwrap();
            *cache = None;
            $sync_fn(inner.doc(), &$items)?;
            let loro_bytes = inner.get_loro_changes()?;
            let rkyv_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&$items)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            inner.save_to_disk(&rkyv_bytes, &loro_bytes)?;
            *cache = Some($items);
        }
        Ok(())
    }};
}

macro_rules! impl_read_all_items {
    ($self:expr, $t:ty) => {{
        {
            let cache = $self.cache.lock().unwrap();
            if let Some(ref list) = *cache {
                return Ok(list.clone());
            }
        }

        let inner = $self.inner.lock().unwrap();
        let rkyv_slice = inner.get_rkyv_slice();
        if rkyv_slice.is_empty() {
            return Ok(Vec::new());
        }

        let required_align = std::cmp::max(
            std::mem::align_of::<rkyv::Archived<Vec<$t>>>(),
            std::mem::align_of::<rkyv::Archived<$t>>(),
        );
        let list: Vec<$t> = if (rkyv_slice.as_ptr() as usize) % required_align == 0 {
            let archived =
                rkyv::access::<rkyv::Archived<Vec<$t>>, rkyv::rancor::Error>(rkyv_slice)
                    .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            rkyv::deserialize::<Vec<$t>, rkyv::rancor::Error>(archived)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?
        } else {
            let mut aligned = rkyv::util::AlignedVec::<16>::new();
            aligned.extend_from_slice(rkyv_slice);
            let archived =
                rkyv::access::<rkyv::Archived<Vec<$t>>, rkyv::rancor::Error>(&aligned)
                    .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            rkyv::deserialize::<Vec<$t>, rkyv::rancor::Error>(archived)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?
        };

        {
            let mut cache = $self.cache.lock().unwrap();
            *cache = Some(list.clone());
        }
        Ok(list)
    }};
}

macro_rules! impl_read_item_zero_copy {
    ($self:expr, $item_id:expr, $t:ty) => {{
        {
            let cache = $self.cache.lock().unwrap();
            if let Some(ref list) = *cache {
                return Ok(list.iter().find(|item| item.id == $item_id).cloned());
            }
        }

        let inner = $self.inner.lock().unwrap();
        let rkyv_slice = inner.get_rkyv_slice();
        if rkyv_slice.is_empty() {
            return Ok(None);
        }

        let required_align = std::cmp::max(
            std::mem::align_of::<rkyv::Archived<Vec<$t>>>(),
            std::mem::align_of::<rkyv::Archived<$t>>(),
        );
        if (rkyv_slice.as_ptr() as usize) % required_align == 0 {
            let archived =
                rkyv::access::<rkyv::Archived<Vec<$t>>, rkyv::rancor::Error>(rkyv_slice)
                    .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            for item in archived.iter() {
                if item.id == $item_id {
                    let todo = rkyv::deserialize::<$t, rkyv::rancor::Error>(item)
                        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                    return Ok(Some(todo));
                }
            }
        } else {
            let mut aligned = rkyv::util::AlignedVec::<16>::new();
            aligned.extend_from_slice(rkyv_slice);
            let archived =
                rkyv::access::<rkyv::Archived<Vec<$t>>, rkyv::rancor::Error>(&aligned)
                    .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            for item in archived.iter() {
                if item.id == $item_id {
                    let todo = rkyv::deserialize::<$t, rkyv::rancor::Error>(item)
                        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                    return Ok(Some(todo));
                }
            }
        }

        Ok(None)
    }};
}

macro_rules! impl_get_loro_changes {
    ($self:expr) => {{
        let inner = $self.inner.lock().unwrap();
        inner.get_loro_changes()
    }};
}

macro_rules! impl_apply_loro_update {
    ($self:expr, $update_bytes:expr, $read_fn:path) => {{
        let mut inner = $self.inner.lock().unwrap();
        *$self.cache.lock().unwrap() = None;
        inner.doc().import(&$update_bytes)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
        let items = $read_fn(inner.doc())?;
        let loro_bytes = inner.get_loro_changes()?;
        let rkyv_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&items)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
        inner.save_to_disk(&rkyv_bytes, &loro_bytes)?;
        Ok(())
    }};
}

macro_rules! impl_apply_loro_updates_batch {
    ($self:expr, $updates:expr, $read_fn:path) => {{
        if $updates.is_empty() {
            return Ok(());
        }
        let mut inner = $self.inner.lock().unwrap();
        *$self.cache.lock().unwrap() = None;
        for update in &$updates {
            inner.doc().import(update)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?;
        }
        let items = $read_fn(inner.doc())?;
        let loro_bytes = inner.get_loro_changes()?;
        let rkyv_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&items)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
        inner.save_to_disk(&rkyv_bytes, &loro_bytes)?;
        Ok(())
    }};
}

// --- ZeroCopyStore (TodoItem) ---

#[derive(Clone, uniffi::Object)]
pub struct ZeroCopyStore {
    inner: Arc<Mutex<ZeroCopyEngine>>,
    cache: Arc<Mutex<Option<Vec<TodoItem>>>>,
}

impl ZeroCopyStore {
    #[allow(unused_variables)]
    pub fn new(file_path: String) -> Result<Self, YntraError> {
        Ok(Self {
            inner: Arc::new(Mutex::new(ZeroCopyEngine::new(file_path)?)),
            cache: Arc::new(Mutex::new(None)),
        })
    }
}

#[uniffi::export]
impl ZeroCopyStore {
    pub fn write_todos(&self, todos: Vec<TodoItem>) -> Result<(), YntraError> {
        impl_write_items!(self, todos, sync_todos_to_loro)
    }

    pub fn read_todo_zero_copy(&self, todo_id: String) -> Result<Option<TodoItem>, YntraError> {
        impl_read_item_zero_copy!(self, todo_id, TodoItem)
    }

    pub fn read_all_todos(&self) -> Result<Vec<TodoItem>, YntraError> {
        impl_read_all_items!(self, TodoItem)
    }

    pub fn read_todos_by_workspace(
        &self,
        workspace_id: String,
    ) -> Result<Vec<TodoItem>, YntraError> {
        {
            let cache = self.cache.lock().unwrap();
            if let Some(ref list) = *cache {
                return Ok(list
                    .iter()
                    .filter(|t| t.workspace_id == workspace_id)
                    .cloned()
                    .collect());
            }
        }

        let inner = self.inner.lock().unwrap();
        let rkyv_slice = inner.get_rkyv_slice();
        if rkyv_slice.is_empty() {
            return Ok(Vec::new());
        }

        let mut filtered = Vec::new();
        let required_align = std::cmp::max(
            std::mem::align_of::<rkyv::Archived<Vec<TodoItem>>>(),
            std::mem::align_of::<rkyv::Archived<TodoItem>>(),
        );
        if (rkyv_slice.as_ptr() as usize) % required_align == 0 {
            let archived_todos =
                rkyv::access::<rkyv::Archived<Vec<TodoItem>>, rkyv::rancor::Error>(rkyv_slice)
                    .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            for item in archived_todos.iter() {
                if item.workspace_id == workspace_id {
                    let todo = rkyv::deserialize::<TodoItem, rkyv::rancor::Error>(item)
                        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                    filtered.push(todo);
                }
            }
        } else {
            let mut aligned = rkyv::util::AlignedVec::<16>::new();
            aligned.extend_from_slice(rkyv_slice);
            let archived_todos =
                rkyv::access::<rkyv::Archived<Vec<TodoItem>>, rkyv::rancor::Error>(&aligned)
                    .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            for item in archived_todos.iter() {
                if item.workspace_id == workspace_id {
                    let todo = rkyv::deserialize::<TodoItem, rkyv::rancor::Error>(item)
                        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                    filtered.push(todo);
                }
            }
        }

        Ok(filtered)
    }

    pub fn get_loro_changes(&self) -> Result<Vec<u8>, YntraError> {
        impl_get_loro_changes!(self)
    }

    pub fn apply_loro_update(&self, update_bytes: Vec<u8>) -> Result<(), YntraError> {
        impl_apply_loro_update!(self, update_bytes, read_all_todos_from_loro)
    }

    pub fn apply_loro_updates_batch(&self, updates: Vec<Vec<u8>>) -> Result<(), YntraError> {
        impl_apply_loro_updates_batch!(self, updates, read_all_todos_from_loro)
    }
}

// --- ZeroCopyMessageStore (MessageItem) ---

#[derive(Clone, uniffi::Object)]
pub struct ZeroCopyMessageStore {
    inner: Arc<Mutex<ZeroCopyEngine>>,
    cache: Arc<Mutex<Option<Vec<MessageItem>>>>,
}

impl ZeroCopyMessageStore {
    #[allow(unused_variables)]
    pub fn new(file_path: String) -> Result<Self, YntraError> {
        Ok(Self {
            inner: Arc::new(Mutex::new(ZeroCopyEngine::new(file_path)?)),
            cache: Arc::new(Mutex::new(None)),
        })
    }
}

#[uniffi::export]
impl ZeroCopyMessageStore {
    pub fn write_messages(&self, messages: Vec<MessageItem>) -> Result<(), YntraError> {
        impl_write_items!(self, messages, sync_messages_to_loro)
    }

    pub fn read_all_messages(&self) -> Result<Vec<MessageItem>, YntraError> {
        impl_read_all_items!(self, MessageItem)
    }

    pub fn read_messages_filtered(
        &self,
        workspace_id: String,
        user_id: String,
        user_teams: Vec<String>,
    ) -> Result<Vec<MessageItem>, YntraError> {
        let team_set: std::collections::HashSet<&str> =
            user_teams.iter().map(|t| t.as_str()).collect();

        {
            let cache = self.cache.lock().unwrap();
            if let Some(ref list) = *cache {
                let mut filtered = Vec::new();
                for msg in list {
                    if msg.workspace_id != workspace_id {
                        continue;
                    }
                    let is_sender = msg.sender_id.as_ref().map(|s| s.as_str()) == Some(user_id.as_str());
                    let is_receiver =
                        msg.receiver_id.as_ref().map(|r| r.as_str()) == Some(user_id.as_str());
                    let is_team_recipient = msg
                        .target_team_id
                        .as_ref()
                        .map(|tid| team_set.contains(tid.as_str()))
                        .unwrap_or(false);
                    if is_sender || is_receiver || is_team_recipient {
                        filtered.push(msg.clone());
                    }
                }
                return Ok(filtered);
            }
        }

        let inner = self.inner.lock().unwrap();
        let rkyv_slice = inner.get_rkyv_slice();
        if rkyv_slice.is_empty() {
            return Ok(Vec::new());
        }

        let mut filtered = Vec::new();
        let required_align = std::cmp::max(
            std::mem::align_of::<rkyv::Archived<Vec<MessageItem>>>(),
            std::mem::align_of::<rkyv::Archived<MessageItem>>(),
        );

        if (rkyv_slice.as_ptr() as usize) % required_align == 0 {
            let archived_msgs = rkyv::access::<
                rkyv::Archived<Vec<MessageItem>>,
                rkyv::rancor::Error,
            >(rkyv_slice)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            for msg in archived_msgs.iter() {
                if msg.workspace_id != workspace_id {
                    continue;
                }
                let is_sender = msg.sender_id.as_ref().map(|s| s.as_str()) == Some(user_id.as_str());
                let is_receiver =
                    msg.receiver_id.as_ref().map(|r| r.as_str()) == Some(user_id.as_str());
                let is_team_recipient = msg
                    .target_team_id
                    .as_ref()
                    .map(|tid| team_set.contains(tid.as_str()))
                    .unwrap_or(false);
                if is_sender || is_receiver || is_team_recipient {
                    let message = rkyv::deserialize::<MessageItem, rkyv::rancor::Error>(msg)
                        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                    filtered.push(message);
                }
            }
        } else {
            let mut aligned = rkyv::util::AlignedVec::<16>::new();
            aligned.extend_from_slice(rkyv_slice);
            let archived_msgs = rkyv::access::<
                rkyv::Archived<Vec<MessageItem>>,
                rkyv::rancor::Error,
            >(&aligned)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            for msg in archived_msgs.iter() {
                if msg.workspace_id != workspace_id {
                    continue;
                }
                let is_sender = msg.sender_id.as_ref().map(|s| s.as_str()) == Some(user_id.as_str());
                let is_receiver =
                    msg.receiver_id.as_ref().map(|r| r.as_str()) == Some(user_id.as_str());
                let is_team_recipient = msg
                    .target_team_id
                    .as_ref()
                    .map(|tid| team_set.contains(tid.as_str()))
                    .unwrap_or(false);
                if is_sender || is_receiver || is_team_recipient {
                    let message = rkyv::deserialize::<MessageItem, rkyv::rancor::Error>(msg)
                        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                    filtered.push(message);
                }
            }
        }

        Ok(filtered)
    }

    pub fn read_message_zero_copy(
        &self,
        message_id: String,
    ) -> Result<Option<MessageItem>, YntraError> {
        impl_read_item_zero_copy!(self, message_id, MessageItem)
    }

    pub fn get_loro_changes(&self) -> Result<Vec<u8>, YntraError> {
        impl_get_loro_changes!(self)
    }

    pub fn apply_loro_update(&self, update_bytes: Vec<u8>) -> Result<(), YntraError> {
        impl_apply_loro_update!(self, update_bytes, read_all_messages_from_loro)
    }

    pub fn apply_loro_updates_batch(&self, updates: Vec<Vec<u8>>) -> Result<(), YntraError> {
        impl_apply_loro_updates_batch!(self, updates, read_all_messages_from_loro)
    }
}

// --- ZeroCopyAuditStore (AuditLogEntry) ---

#[derive(Clone, uniffi::Object)]
pub struct ZeroCopyAuditStore {
    inner: Arc<Mutex<ZeroCopyEngine>>,
    cache: Arc<Mutex<Option<Vec<AuditLogEntry>>>>,
}

impl ZeroCopyAuditStore {
    #[allow(unused_variables)]
    pub fn new(file_path: String) -> Result<Self, YntraError> {
        Ok(Self {
            inner: Arc::new(Mutex::new(ZeroCopyEngine::new(file_path)?)),
            cache: Arc::new(Mutex::new(None)),
        })
    }
}

#[uniffi::export]
impl ZeroCopyAuditStore {
    pub fn write_audit_logs(&self, entries: Vec<AuditLogEntry>) -> Result<(), YntraError> {
        impl_write_items!(self, entries, sync_audits_to_loro)
    }

    pub fn read_all_audit_logs(&self) -> Result<Vec<AuditLogEntry>, YntraError> {
        impl_read_all_items!(self, AuditLogEntry)
    }

    pub fn read_audit_zero_copy(&self, entry_id: String) -> Result<Option<AuditLogEntry>, YntraError> {
        impl_read_item_zero_copy!(self, entry_id, AuditLogEntry)
    }

    pub fn get_loro_changes(&self) -> Result<Vec<u8>, YntraError> {
        impl_get_loro_changes!(self)
    }

    pub fn apply_loro_update(&self, update_bytes: Vec<u8>) -> Result<(), YntraError> {
        impl_apply_loro_update!(self, update_bytes, read_all_audits_from_loro)
    }

    pub fn apply_loro_updates_batch(&self, updates: Vec<Vec<u8>>) -> Result<(), YntraError> {
        impl_apply_loro_updates_batch!(self, updates, read_all_audits_from_loro)
    }
}

// --- ZeroCopyNoteStore (DailyNote) ---

#[derive(Clone, uniffi::Object)]
pub struct ZeroCopyNoteStore {
    inner: Arc<Mutex<ZeroCopyEngine>>,
    cache: Arc<Mutex<Option<Vec<DailyNote>>>>,
}

impl ZeroCopyNoteStore {
    #[allow(unused_variables)]
    pub fn new(file_path: String) -> Result<Self, YntraError> {
        Ok(Self {
            inner: Arc::new(Mutex::new(ZeroCopyEngine::new(file_path)?)),
            cache: Arc::new(Mutex::new(None)),
        })
    }
}

#[uniffi::export]
impl ZeroCopyNoteStore {
    pub fn write_notes(&self, notes: Vec<DailyNote>) -> Result<(), YntraError> {
        impl_write_items!(self, notes, sync_notes_to_loro)
    }

    pub fn read_all_notes(&self) -> Result<Vec<DailyNote>, YntraError> {
        impl_read_all_items!(self, DailyNote)
    }

    pub fn read_note_zero_copy(&self, note_id: String) -> Result<Option<DailyNote>, YntraError> {
        impl_read_item_zero_copy!(self, note_id, DailyNote)
    }

    pub fn get_loro_changes(&self) -> Result<Vec<u8>, YntraError> {
        impl_get_loro_changes!(self)
    }

    pub fn apply_loro_update(&self, update_bytes: Vec<u8>) -> Result<(), YntraError> {
        impl_apply_loro_update!(self, update_bytes, read_all_notes_from_loro)
    }

    pub fn apply_loro_updates_batch(&self, updates: Vec<Vec<u8>>) -> Result<(), YntraError> {
        impl_apply_loro_updates_batch!(self, updates, read_all_notes_from_loro)
    }
}

// --- Helpers ---

#[uniffi::export]
pub fn create_peer_store(name: String) -> Result<ZeroCopyStore, YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = std::env::temp_dir()
            .join(format!("yntra_zero_copy_{}.db", name))
            .to_string_lossy()
            .to_string();
        let _ = std::fs::remove_file(&path);
        ZeroCopyStore::new(path)
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = name;
        ZeroCopyStore::new(String::new())
    }
}

#[uniffi::export]
pub fn create_peer_note_store(name: String) -> Result<ZeroCopyNoteStore, YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = std::env::temp_dir()
            .join(format!("yntra_zero_copy_notes_{}.db", name))
            .to_string_lossy()
            .to_string();
        let _ = std::fs::remove_file(&path);
        ZeroCopyNoteStore::new(path)
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = name;
        ZeroCopyNoteStore::new(String::new())
    }
}
