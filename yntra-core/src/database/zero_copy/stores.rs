use crate::infra::errors::YntraError;
use crate::models::{AuditLogEntry, DailyNote, MessageItem, TodoItem};
use std::sync::{Arc, Mutex};
use super::engine::ZeroCopyEngine;
use super::MutexExt;

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
        let _item_map = match db_map.get(&todo.id) {
            Some(loro::ValueOrContainer::Container(loro::Container::Map(m))) => {
                if get_string(&m, "id").ok().as_ref() != Some(&todo.id) {
                    m.insert("id", todo.id.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
                if get_string(&m, "workspace_id").ok().as_ref() != Some(&todo.workspace_id) {
                    m.insert("workspace_id", todo.workspace_id.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
                if get_string(&m, "text").ok().as_ref() != Some(&todo.text) {
                    m.insert("text", todo.text.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
                if get_bool(&m, "completed").ok() != Some(todo.completed) {
                    m.insert("completed", todo.completed).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
                if get_i64(&m, "updated_at").ok() != Some(todo.updated_at) {
                    m.insert("updated_at", todo.updated_at).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
                if get_string(&m, "sync_status").ok().as_ref() != Some(&todo.sync_status) {
                    m.insert("sync_status", todo.sync_status.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
                m
            }
            _ => {
                let m = db_map
                    .insert_container(&todo.id, loro::LoroMap::new())
                    .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                m.insert("id", todo.id.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                m.insert("workspace_id", todo.workspace_id.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                m.insert("text", todo.text.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                m.insert("completed", todo.completed).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                m.insert("updated_at", todo.updated_at).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                m.insert("sync_status", todo.sync_status.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                m
            }
        };
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

pub(crate) fn read_all_todos_from_loro(loro: &loro::LoroDoc) -> Result<Vec<TodoItem>, YntraError> {
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
        let _item_map = match db_map.get(&msg.id) {
            Some(loro::ValueOrContainer::Container(loro::Container::Map(m))) => {
                if get_string(&m, "id").ok().as_ref() != Some(&msg.id) {
                    m.insert("id", msg.id.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
                if get_string(&m, "workspace_id").ok().as_ref() != Some(&msg.workspace_id) {
                    m.insert("workspace_id", msg.workspace_id.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
                if get_opt_string(&m, "sender_id").ok() != Some(msg.sender_id.clone()) {
                    match &msg.sender_id {
                        Some(v) => m.insert("sender_id", v.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?,
                        None => m.insert("sender_id", loro::LoroValue::Null).map_err(|e| YntraError::SerializationError(e.to_string()))?,
                    };
                }
                if get_opt_string(&m, "receiver_id").ok() != Some(msg.receiver_id.clone()) {
                    match &msg.receiver_id {
                        Some(v) => m.insert("receiver_id", v.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?,
                        None => m.insert("receiver_id", loro::LoroValue::Null).map_err(|e| YntraError::SerializationError(e.to_string()))?,
                    };
                }
                if get_opt_string(&m, "target_team_id").ok() != Some(msg.target_team_id.clone()) {
                    match &msg.target_team_id {
                        Some(v) => m.insert("target_team_id", v.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?,
                        None => m.insert("target_team_id", loro::LoroValue::Null).map_err(|e| YntraError::SerializationError(e.to_string()))?,
                    };
                }
                if get_opt_string(&m, "subject").ok() != Some(msg.subject.clone()) {
                    match &msg.subject {
                        Some(v) => m.insert("subject", v.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?,
                        None => m.insert("subject", loro::LoroValue::Null).map_err(|e| YntraError::SerializationError(e.to_string()))?,
                    };
                }
                if get_opt_string(&m, "body").ok() != Some(msg.body.clone()) {
                    match &msg.body {
                        Some(v) => m.insert("body", v.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?,
                        None => m.insert("body", loro::LoroValue::Null).map_err(|e| YntraError::SerializationError(e.to_string()))?,
                    };
                }
                if get_bool(&m, "is_read").ok() != Some(msg.is_read) {
                    m.insert("is_read", msg.is_read).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
                if get_string(&m, "created_at").ok().as_ref() != Some(&msg.created_at) {
                    m.insert("created_at", msg.created_at.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
                if get_i64(&m, "updated_at").ok() != Some(msg.updated_at) {
                    m.insert("updated_at", msg.updated_at).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
                if get_string(&m, "sync_status").ok().as_ref() != Some(&msg.sync_status) {
                    m.insert("sync_status", msg.sync_status.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
                m
            }
            _ => {
                let m = db_map
                    .insert_container(&msg.id, loro::LoroMap::new())
                    .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                m.insert("id", msg.id.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                m.insert("workspace_id", msg.workspace_id.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                match &msg.sender_id {
                    Some(v) => m.insert("sender_id", v.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?,
                    None => m.insert("sender_id", loro::LoroValue::Null).map_err(|e| YntraError::SerializationError(e.to_string()))?,
                }
                match &msg.receiver_id {
                    Some(v) => m.insert("receiver_id", v.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?,
                    None => m.insert("receiver_id", loro::LoroValue::Null).map_err(|e| YntraError::SerializationError(e.to_string()))?,
                }
                match &msg.target_team_id {
                    Some(v) => m.insert("target_team_id", v.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?,
                    None => m.insert("target_team_id", loro::LoroValue::Null).map_err(|e| YntraError::SerializationError(e.to_string()))?,
                }
                match &msg.subject {
                    Some(v) => m.insert("subject", v.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?,
                    None => m.insert("subject", loro::LoroValue::Null).map_err(|e| YntraError::SerializationError(e.to_string()))?,
                }
                match &msg.body {
                    Some(v) => m.insert("body", v.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?,
                    None => m.insert("body", loro::LoroValue::Null).map_err(|e| YntraError::SerializationError(e.to_string()))?,
                }
                m.insert("is_read", msg.is_read).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                m.insert("created_at", msg.created_at.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                m.insert("updated_at", msg.updated_at).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                m.insert("sync_status", msg.sync_status.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                m
            }
        };
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
        let _item_map = match db_map.get(&entry.id) {
            Some(loro::ValueOrContainer::Container(loro::Container::Map(m))) => {
                if get_string(&m, "id").ok().as_ref() != Some(&entry.id) {
                    m.insert("id", entry.id.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
                if get_string(&m, "workspace_id").ok().as_ref() != Some(&entry.workspace_id) {
                    m.insert("workspace_id", entry.workspace_id.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
                if get_string(&m, "actor_id").ok().as_ref() != Some(&entry.actor_id) {
                    m.insert("actor_id", entry.actor_id.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
                if get_opt_string(&m, "target_client_id").ok() != Some(entry.target_client_id.clone()) {
                    match &entry.target_client_id {
                        Some(v) => m.insert("target_client_id", v.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?,
                        None => m.insert("target_client_id", loro::LoroValue::Null).map_err(|e| YntraError::SerializationError(e.to_string()))?,
                    };
                }
                if get_string(&m, "action_type").ok().as_ref() != Some(&entry.action_type) {
                    m.insert("action_type", entry.action_type.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
                if get_i64(&m, "timestamp").ok() != Some(entry.timestamp) {
                    m.insert("timestamp", entry.timestamp).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
                if get_string(&m, "prev_hash").ok().as_ref() != Some(&entry.prev_hash) {
                    m.insert("prev_hash", entry.prev_hash.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
                if get_string(&m, "curr_hash").ok().as_ref() != Some(&entry.curr_hash) {
                    m.insert("curr_hash", entry.curr_hash.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
                if get_i64(&m, "seq").ok() != Some(entry.seq) {
                    m.insert("seq", entry.seq).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
                if get_opt_string(&m, "signature").ok() != Some(entry.signature.clone()) {
                    match &entry.signature {
                        Some(v) => m.insert("signature", v.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?,
                        None => m.insert("signature", loro::LoroValue::Null).map_err(|e| YntraError::SerializationError(e.to_string()))?,
                    };
                }
                m
            }
            _ => {
                let m = db_map
                    .insert_container(&entry.id, loro::LoroMap::new())
                    .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                m.insert("id", entry.id.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                m.insert("workspace_id", entry.workspace_id.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                m.insert("actor_id", entry.actor_id.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                match &entry.target_client_id {
                    Some(v) => m.insert("target_client_id", v.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?,
                    None => m.insert("target_client_id", loro::LoroValue::Null).map_err(|e| YntraError::SerializationError(e.to_string()))?,
                }
                m.insert("action_type", entry.action_type.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                m.insert("timestamp", entry.timestamp).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                m.insert("prev_hash", entry.prev_hash.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                m.insert("curr_hash", entry.curr_hash.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                m.insert("seq", entry.seq).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                match &entry.signature {
                    Some(v) => m.insert("signature", v.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?,
                    None => m.insert("signature", loro::LoroValue::Null).map_err(|e| YntraError::SerializationError(e.to_string()))?,
                }
                m
            }
        };
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
        let _item_map = match db_map.get(&note.id) {
            Some(loro::ValueOrContainer::Container(loro::Container::Map(m))) => {
                if get_string(&m, "id").ok().as_ref() != Some(&note.id) {
                    m.insert("id", note.id.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
                if get_string(&m, "workspace_id").ok().as_ref() != Some(&note.workspace_id) {
                    m.insert("workspace_id", note.workspace_id.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
                if get_string(&m, "team_id").ok().as_ref() != Some(&note.team_id) {
                    m.insert("team_id", note.team_id.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
                if get_opt_string(&m, "author_id").ok() != Some(note.author_id.clone()) {
                    match &note.author_id {
                        Some(v) => m.insert("author_id", v.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?,
                        None => m.insert("author_id", loro::LoroValue::Null).map_err(|e| YntraError::SerializationError(e.to_string()))?,
                    };
                }
                if get_string(&m, "subject").ok().as_ref() != Some(&note.subject) {
                    m.insert("subject", note.subject.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
                if get_string(&m, "content").ok().as_ref() != Some(&note.content) {
                    m.insert("content", note.content.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
                if get_string(&m, "edit_history").ok().as_ref() != Some(&note.edit_history) {
                    m.insert("edit_history", note.edit_history.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
                if get_string(&m, "created_at").ok().as_ref() != Some(&note.created_at) {
                    m.insert("created_at", note.created_at.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
                if get_i64(&m, "updated_at").ok() != Some(note.updated_at) {
                    m.insert("updated_at", note.updated_at).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
                if get_string(&m, "sync_status").ok().as_ref() != Some(&note.sync_status) {
                    m.insert("sync_status", note.sync_status.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
                m
            }
            _ => {
                let m = db_map
                    .insert_container(&note.id, loro::LoroMap::new())
                    .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                m.insert("id", note.id.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                m.insert("workspace_id", note.workspace_id.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                m.insert("team_id", note.team_id.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                match &note.author_id {
                    Some(v) => m.insert("author_id", v.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?,
                    None => m.insert("author_id", loro::LoroValue::Null).map_err(|e| YntraError::SerializationError(e.to_string()))?,
                }
                m.insert("subject", note.subject.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                m.insert("content", note.content.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                m.insert("edit_history", note.edit_history.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                m.insert("created_at", note.created_at.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                m.insert("updated_at", note.updated_at).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                m.insert("sync_status", note.sync_status.clone()).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                m
            }
        };
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

pub(crate) fn read_all_notes_from_loro(loro: &loro::LoroDoc) -> Result<Vec<DailyNote>, YntraError> {
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
    ($self:expr, $items:expr, $sync_fn:path, $table_name:expr) => {{
        {
            let mut inner = $self.inner.lock_poison_safe();
            let mut cache = $self.cache.lock_poison_safe();
            let mut sorted_items = $items;
            sorted_items.sort_by(|a, b| a.id.cmp(&b.id));

            let modified_ids: Vec<String> = if let Some(ref old_list) = *cache {
                let mut ids = Vec::new();
                for new_item in &sorted_items {
                    if let Some(old_item) = old_list.iter().find(|o| o.id == new_item.id) {
                        if old_item != new_item {
                            ids.push(new_item.id.clone());
                        }
                    } else {
                        ids.push(new_item.id.clone());
                    }
                }
                ids
            } else {
                sorted_items.iter().map(|item| item.id.clone()).collect()
            };

            *cache = None;
            $sync_fn(inner.doc(), &sorted_items)?;
            let loro_bytes = inner.get_loro_changes()?;
            let rkyv_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&sorted_items)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            inner.save_to_disk(&rkyv_bytes, &loro_bytes)?;
            *cache = Some(sorted_items);

            for id in modified_ids {
                crate::infra::observer::set_last_modified_record($table_name, &id);
            }
        }
        crate::infra::observer::notify_observers();
        Ok(())
    }};
}

macro_rules! impl_get_count {
    ($self:expr, $t:ty) => {{
        let inner = $self.inner.lock_poison_safe();
        let rkyv_slice = inner.get_rkyv_slice();
        if rkyv_slice.is_empty() {
            return Ok(0);
        }
        let required_align = std::cmp::max(
            std::mem::align_of::<rkyv::Archived<Vec<$t>>>(),
            std::mem::align_of::<rkyv::Archived<$t>>(),
        );
        if (rkyv_slice.as_ptr() as usize) % required_align == 0 {
            let archived = rkyv::access::<rkyv::Archived<Vec<$t>>, rkyv::rancor::Error>(rkyv_slice)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            Ok(archived.len() as u32)
        } else {
            let mut aligned = vec![0u8; rkyv_slice.len() + required_align];
            let offset = required_align - (aligned.as_ptr() as usize % required_align);
            let aligned_slice = &mut aligned[offset..offset + rkyv_slice.len()];
            aligned_slice.copy_from_slice(rkyv_slice);
            let archived = rkyv::access::<rkyv::Archived<Vec<$t>>, rkyv::rancor::Error>(aligned_slice)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            Ok(archived.len() as u32)
        }
    }};
}

macro_rules! impl_get_at {
    ($self:expr, $index:expr, $t:ty) => {{
        let inner = $self.inner.lock_poison_safe();
        let rkyv_slice = inner.get_rkyv_slice();
        if rkyv_slice.is_empty() {
            return Ok(None);
        }
        let required_align = std::cmp::max(
            std::mem::align_of::<rkyv::Archived<Vec<$t>>>(),
            std::mem::align_of::<rkyv::Archived<$t>>(),
        );
        if (rkyv_slice.as_ptr() as usize) % required_align == 0 {
            let archived = rkyv::access::<rkyv::Archived<Vec<$t>>, rkyv::rancor::Error>(rkyv_slice)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            if ($index as usize) < archived.len() {
                let archived_item = &archived[$index as usize];
                let deserialized = rkyv::deserialize::<$t, rkyv::rancor::Error>(archived_item)
                    .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                Ok(Some(deserialized))
            } else {
                Ok(None)
            }
        } else {
            let mut aligned = vec![0u8; rkyv_slice.len() + required_align];
            let offset = required_align - (aligned.as_ptr() as usize % required_align);
            let aligned_slice = &mut aligned[offset..offset + rkyv_slice.len()];
            aligned_slice.copy_from_slice(rkyv_slice);
            let archived = rkyv::access::<rkyv::Archived<Vec<$t>>, rkyv::rancor::Error>(aligned_slice)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            if ($index as usize) < archived.len() {
                let archived_item = &archived[$index as usize];
                let deserialized = rkyv::deserialize::<$t, rkyv::rancor::Error>(archived_item)
                    .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                Ok(Some(deserialized))
            } else {
                Ok(None)
            }
        }
    }};
}

macro_rules! impl_read_all_items {
    ($self:expr, $t:ty) => {{
        {
            let cache = $self.cache.lock_poison_safe();
            if let Some(ref list) = *cache {
                return Ok(list.clone());
            }
        }

        let inner = $self.inner.lock_poison_safe();
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
            let mut cache = $self.cache.lock_poison_safe();
            *cache = Some(list.clone());
        }
        Ok(list)
    }};
}

macro_rules! impl_read_item_zero_copy {
    ($self:expr, $item_id:expr, $t:ty) => {{
        {
            let cache = $self.cache.lock_poison_safe();
            if let Some(ref list) = *cache {
                return Ok(list.iter().find(|item| item.id == $item_id).cloned());
            }
        }

        let inner = $self.inner.lock_poison_safe();
        let rkyv_slice = inner.get_rkyv_slice();
        if rkyv_slice.is_empty() {
            return Ok(None);
        }

        let required_align = std::cmp::max(
            std::mem::align_of::<rkyv::Archived<Vec<$t>>>(),
            std::mem::align_of::<rkyv::Archived<$t>>(),
        );
        let item_opt = if (rkyv_slice.as_ptr() as usize) % required_align == 0 {
            let archived =
                rkyv::access::<rkyv::Archived<Vec<$t>>, rkyv::rancor::Error>(rkyv_slice)
                    .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            
            let mut low = 0;
            let mut high = archived.len();
            let mut found = None;
            while low < high {
                let mid = low + (high - low) / 2;
                let item = &archived[mid];
                match item.id.as_str().cmp(&$item_id) {
                    std::cmp::Ordering::Equal => {
                        let deserialized = rkyv::deserialize::<$t, rkyv::rancor::Error>(item)
                            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                        found = Some(deserialized);
                        break;
                    }
                    std::cmp::Ordering::Less => {
                        low = mid + 1;
                    }
                    std::cmp::Ordering::Greater => {
                        high = mid;
                    }
                }
            }
            found
        } else {
            let mut aligned = rkyv::util::AlignedVec::<16>::new();
            aligned.extend_from_slice(rkyv_slice);
            let archived =
                rkyv::access::<rkyv::Archived<Vec<$t>>, rkyv::rancor::Error>(&aligned)
                    .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            
            let mut low = 0;
            let mut high = archived.len();
            let mut found = None;
            while low < high {
                let mid = low + (high - low) / 2;
                let item = &archived[mid];
                match item.id.as_str().cmp(&$item_id) {
                    std::cmp::Ordering::Equal => {
                        let deserialized = rkyv::deserialize::<$t, rkyv::rancor::Error>(item)
                            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                        found = Some(deserialized);
                        break;
                    }
                    std::cmp::Ordering::Less => {
                        low = mid + 1;
                    }
                    std::cmp::Ordering::Greater => {
                        high = mid;
                    }
                }
            }
            found
        };

        Ok(item_opt)
    }};
}

macro_rules! impl_get_loro_changes {
    ($self:expr) => {{
        let inner = $self.inner.lock_poison_safe();
        inner.get_loro_changes()
    }};
}

macro_rules! impl_apply_loro_update {
    ($self:expr, $update_bytes:expr, $read_fn:path) => {{
        let mut inner = $self.inner.lock_poison_safe();
        *$self.cache.lock_poison_safe() = None;
        inner.doc().import(&$update_bytes)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
        let mut items = $read_fn(inner.doc())?;
        items.sort_by(|a, b| a.id.cmp(&b.id));
        let loro_bytes = inner.get_loro_changes()?;
        let rkyv_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&items)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
        inner.save_to_disk(&rkyv_bytes, &loro_bytes)?;
        crate::infra::observer::notify_observers();
        Ok(())
    }};
}

macro_rules! impl_apply_loro_updates_batch {
    ($self:expr, $updates:expr, $read_fn:path) => {{
        if $updates.is_empty() {
            return Ok(());
        }
        let mut inner = $self.inner.lock_poison_safe();
        *$self.cache.lock_poison_safe() = None;
        for update in &$updates {
            inner.doc().import(update)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?;
        }
        let mut items = $read_fn(inner.doc())?;
        items.sort_by(|a, b| a.id.cmp(&b.id));
        let loro_bytes = inner.get_loro_changes()?;
        let rkyv_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&items)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
        inner.save_to_disk(&rkyv_bytes, &loro_bytes)?;
        crate::infra::observer::notify_observers();
        Ok(())
    }};
}

// --- Unified Macro to Define Zero-Copy Stores ---

macro_rules! define_zero_copy_store {
    (
        $struct_name:ident,
        $item_ty:ty,
        $write_fn_name:ident,
        $read_zc_fn_name:ident,
        $read_all_fn_name:ident,
        $get_count_fn_name:ident,
        $get_at_fn_name:ident,
        $sync_fn:path,
        $read_fn:path,
        $table_name:expr,
        $comment_struct:expr,
        $comment_write:expr,
        $comment_read_zc:expr,
        $comment_read_all:expr,
        $comment_count:expr,
        $comment_at:expr
    ) => {
        #[doc = $comment_struct]
        #[derive(Clone, uniffi::Object)]
        pub struct $struct_name {
            inner: Arc<Mutex<ZeroCopyEngine>>,
            cache: Arc<Mutex<Option<Vec<$item_ty>>>>,
        }

        impl $struct_name {
            /// Creates a new instance of the zero-copy store at the given file path.
            #[allow(unused_variables)]
            pub fn new(file_path: String) -> Result<Self, YntraError> {
                Ok(Self {
                    inner: Arc::new(Mutex::new(ZeroCopyEngine::new(file_path)?)),
                    cache: Arc::new(Mutex::new(None)),
                })
            }
        }

        #[uniffi::export]
        impl $struct_name {
            #[doc = $comment_write]
            pub fn $write_fn_name(&self, items: Vec<$item_ty>) -> Result<(), YntraError> {
                impl_write_items!(self, items, $sync_fn, $table_name)
            }

            #[doc = $comment_read_zc]
            pub fn $read_zc_fn_name(&self, item_id: String) -> Result<Option<$item_ty>, YntraError> {
                impl_read_item_zero_copy!(self, item_id, $item_ty)
            }

            #[doc = $comment_read_all]
            pub fn $read_all_fn_name(&self) -> Result<Vec<$item_ty>, YntraError> {
                impl_read_all_items!(self, $item_ty)
            }

            #[doc = $comment_count]
            pub fn $get_count_fn_name(&self) -> Result<u32, YntraError> {
                impl_get_count!(self, $item_ty)
            }

            #[doc = $comment_at]
            pub fn $get_at_fn_name(&self, index: u32) -> Result<Option<$item_ty>, YntraError> {
                impl_get_at!(self, index, $item_ty)
            }

            /// Returns the raw Loro change log history from the local database.
            pub fn get_loro_changes(&self) -> Result<Vec<u8>, YntraError> {
                impl_get_loro_changes!(self)
            }

            /// Applies a remote Loro update to the local document and persists it.
            pub fn apply_loro_update(&self, update_bytes: Vec<u8>) -> Result<(), YntraError> {
                impl_apply_loro_update!(self, update_bytes, $read_fn)
            }

            /// Applies a batch of remote Loro updates to the local document.
            pub fn apply_loro_updates_batch(&self, updates: Vec<Vec<u8>>) -> Result<(), YntraError> {
                impl_apply_loro_updates_batch!(self, updates, $read_fn)
            }

            /// Loads the database bytes from OPFS and refreshes the cache.
            pub async fn load_from_opfs(&self) -> Result<(), YntraError> {
                let file_path = {
                    let inner = self.inner.lock_poison_safe();
                    inner.file_path().to_string()
                };
                let bytes = load_from_opfs_by_path(&file_path).await?;
                if let Some(bytes) = bytes {
                    let mut inner = self.inner.lock_poison_safe();
                    inner.load_from_bytes(&bytes)?;
                }
                let mut cache = self.cache.lock_poison_safe();
                *cache = None;
                Ok(())
            }

            /// Returns the raw rkyv serialized binary buffer from the database.
            pub fn get_rkyv_bytes(&self) -> Result<Vec<u8>, YntraError> {
                let inner = self.inner.lock_poison_safe();
                Ok(inner.get_rkyv_slice().to_vec())
            }
        }
    };
}

define_zero_copy_store!(
    ZeroCopyStore,
    TodoItem,
    write_todos,
    read_todo_zero_copy,
    read_all_todos,
    get_todos_count,
    get_todo_at,
    sync_todos_to_loro,
    read_all_todos_from_loro,
    "todos",
    "Zero-copy store for managing TodoItem entities.",
    "Writes a list of todo items to the database.",
    "Reads a single todo item using binary search optimization.",
    "Reads all todo items from the database.",
    "Returns the total count of todo items.",
    "Returns a todo item at the specified index."
);

#[uniffi::export]
impl ZeroCopyStore {
    /// Reads all todo items for a specific workspace ID.
    pub fn read_todos_by_workspace(&self, workspace_id: String) -> Result<Vec<TodoItem>, YntraError> {
        let all_todos = self.read_all_todos()?;
        Ok(all_todos
            .into_iter()
            .filter(|t| t.workspace_id == workspace_id)
            .collect())
    }
}

define_zero_copy_store!(
    ZeroCopyMessageStore,
    MessageItem,
    write_messages,
    read_message_zero_copy,
    read_all_messages,
    get_messages_count,
    get_message_at,
    sync_messages_to_loro,
    read_all_messages_from_loro,
    "messages",
    "Zero-copy store for managing MessageItem entities.",
    "Writes a list of message items to the database.",
    "Reads a single message item using binary search optimization.",
    "Reads all message items from the database.",
    "Returns the total count of message items.",
    "Returns a message item at the specified index."
);

#[uniffi::export]
impl ZeroCopyMessageStore {
    /// Reads and filters message items based on workspace, user, and team membership.
    pub fn read_messages_filtered(
        &self,
        workspace_id: String,
        user_id: String,
        user_teams: Vec<String>,
    ) -> Result<Vec<MessageItem>, YntraError> {
        let team_set: std::collections::HashSet<&str> =
            user_teams.iter().map(|t| t.as_str()).collect();

        let all_msgs = self.read_all_messages()?;
        let mut filtered = Vec::new();
        for msg in all_msgs {
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
                filtered.push(msg);
            }
        }
        Ok(filtered)
    }
}

define_zero_copy_store!(
    ZeroCopyAuditStore,
    AuditLogEntry,
    write_audit_logs,
    read_audit_zero_copy,
    read_all_audit_logs,
    get_audits_count,
    get_audit_at,
    sync_audits_to_loro,
    read_all_audits_from_loro,
    "audits",
    "Zero-copy store for managing AuditLogEntry entities.",
    "Writes a list of audit logs to the database.",
    "Reads a single audit log using binary search optimization.",
    "Reads all audit logs from the database.",
    "Returns the total count of audit logs.",
    "Returns an audit log at the specified index."
);

define_zero_copy_store!(
    ZeroCopyNoteStore,
    DailyNote,
    write_notes,
    read_note_zero_copy,
    read_all_notes,
    get_notes_count,
    get_note_at,
    sync_notes_to_loro,
    read_all_notes_from_loro,
    "notes",
    "Zero-copy store for managing DailyNote entities.",
    "Writes a list of daily notes to the database.",
    "Reads a single daily note using binary search optimization.",
    "Reads all daily notes from the database.",
    "Returns the total count of daily notes.",
    "Returns a daily note at the specified index."
);

// --- Helpers ---

/// Creates a new ZeroCopyStore peer instance.
#[uniffi::export]
pub fn create_peer_store(name: String) -> Result<ZeroCopyStore, YntraError> {
    if name.chars().any(|c| !c.is_alphanumeric() && c != '_' && c != '-') {
        return Err(YntraError::DbError("Invalid peer store name: must be alphanumeric, underscores, or hyphens".to_string()));
    }
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

/// Creates a new ZeroCopyNoteStore peer instance.
#[uniffi::export]
pub fn create_peer_note_store(name: String) -> Result<ZeroCopyNoteStore, YntraError> {
    if name.chars().any(|c| !c.is_alphanumeric() && c != '_' && c != '-') {
        return Err(YntraError::DbError("Invalid peer store name: must be alphanumeric, underscores, or hyphens".to_string()));
    }
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

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = yntra_load_store_bin, catch)]
    async fn js_load_store_bin_stores(file_name: &str) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;
}

/// Loads a store binary payload from OPFS.
pub async fn load_from_opfs_by_path(file_path: &str) -> Result<Option<Vec<u8>>, YntraError> {
    #[cfg(target_arch = "wasm32")]
    {
        crate::wait_for_js_bridge().await;
        let fut = js_load_store_bin_stores(file_path);
        let send_fut = crate::database::wasm::SendFuture::new(fut);
        match send_fut.await {
            Ok(js_val) => {
                if !js_val.is_null() && !js_val.is_undefined() {
                    let array = js_sys::Uint8Array::new(&js_val);
                    let bytes = array.to_vec();
                    if !bytes.is_empty() {
                        return Ok(Some(bytes));
                    }
                }
            }
            Err(e) => {
                let msg = e.as_string().unwrap_or_else(|| "Unknown OPFS load error".to_string());
                return Err(YntraError::DbError(msg));
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = file_path;
    }
    Ok(None)
}
