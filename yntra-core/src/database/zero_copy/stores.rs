use crate::infra::errors::YntraError;
use crate::models::{AuditLogEntry, DailyNote, MessageItem, TodoItem};
use std::sync::{Arc, Mutex};
use super::engine::ZeroCopyEngine;

// --- ZeroCopyStore ---

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
        let rkyv_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&todos)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
        {
            let mut inner = self.inner.lock().unwrap();
            *self.cache.lock().unwrap() = None;
            inner.write_serialized(&rkyv_bytes)?;
        }
        *self.cache.lock().unwrap() = Some(todos);
        Ok(())
    }

    pub fn read_todo_zero_copy(&self, todo_id: String) -> Result<Option<TodoItem>, YntraError> {
        {
            let cache = self.cache.lock().unwrap();
            if let Some(ref list) = *cache {
                return Ok(list.iter().find(|t| t.id == todo_id).cloned());
            }
        }

        let inner = self.inner.lock().unwrap();
        let rkyv_slice = inner.get_rkyv_slice();
        if rkyv_slice.is_empty() {
            return Ok(None);
        }

        if (rkyv_slice.as_ptr() as usize).is_multiple_of(8) {
            let archived_todos =
                rkyv::access::<rkyv::Archived<Vec<TodoItem>>, rkyv::rancor::Error>(rkyv_slice)
                    .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            for item in archived_todos.iter() {
                if item.id == todo_id {
                    let todo = rkyv::deserialize::<TodoItem, rkyv::rancor::Error>(item)
                        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                    return Ok(Some(todo));
                }
            }
        } else {
            let mut aligned = rkyv::util::AlignedVec::<16>::new();
            aligned.extend_from_slice(rkyv_slice);
            let archived_todos =
                rkyv::access::<rkyv::Archived<Vec<TodoItem>>, rkyv::rancor::Error>(&aligned)
                    .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            for item in archived_todos.iter() {
                if item.id == todo_id {
                    let todo = rkyv::deserialize::<TodoItem, rkyv::rancor::Error>(item)
                        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                    return Ok(Some(todo));
                }
            }
        }

        Ok(None)
    }

    pub fn read_all_todos(&self) -> Result<Vec<TodoItem>, YntraError> {
        {
            let cache = self.cache.lock().unwrap();
            if let Some(ref list) = *cache {
                return Ok(list.clone());
            }
        }

        let inner = self.inner.lock().unwrap();
        let rkyv_slice = inner.get_rkyv_slice();
        if rkyv_slice.is_empty() {
            return Ok(Vec::new());
        }

        let list: Vec<TodoItem> = if (rkyv_slice.as_ptr() as usize).is_multiple_of(8) {
            let archived_todos =
                rkyv::access::<rkyv::Archived<Vec<TodoItem>>, rkyv::rancor::Error>(rkyv_slice)
                    .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            rkyv::deserialize::<Vec<TodoItem>, rkyv::rancor::Error>(archived_todos)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?
        } else {
            let mut aligned = rkyv::util::AlignedVec::<16>::new();
            aligned.extend_from_slice(rkyv_slice);
            let archived_todos =
                rkyv::access::<rkyv::Archived<Vec<TodoItem>>, rkyv::rancor::Error>(&aligned)
                    .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            rkyv::deserialize::<Vec<TodoItem>, rkyv::rancor::Error>(archived_todos)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?
        };

        {
            let mut cache = self.cache.lock().unwrap();
            *cache = Some(list.clone());
        }
        Ok(list)
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
        if (rkyv_slice.as_ptr() as usize).is_multiple_of(8) {
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
        let inner = self.inner.lock().unwrap();
        inner.get_loro_changes()
    }

    pub fn apply_loro_update(&self, update_bytes: Vec<u8>) -> Result<(), YntraError> {
        {
            let mut inner = self.inner.lock().unwrap();
            *self.cache.lock().unwrap() = None;
            inner.apply_loro_update(&update_bytes)?;
        }
        Ok(())
    }
}

// --- ZeroCopyMessageStore ---

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
        let rkyv_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&messages)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
        {
            let mut inner = self.inner.lock().unwrap();
            *self.cache.lock().unwrap() = None;
            inner.write_serialized(&rkyv_bytes)?;
        }
        *self.cache.lock().unwrap() = Some(messages);
        Ok(())
    }

    pub fn read_all_messages(&self) -> Result<Vec<MessageItem>, YntraError> {
        {
            let cache = self.cache.lock().unwrap();
            if let Some(ref list) = *cache {
                return Ok(list.clone());
            }
        }

        let inner = self.inner.lock().unwrap();
        let rkyv_slice = inner.get_rkyv_slice();
        if rkyv_slice.is_empty() {
            return Ok(Vec::new());
        }

        let list: Vec<MessageItem> = if (rkyv_slice.as_ptr() as usize).is_multiple_of(8) {
            let archived_msgs = rkyv::access::<
                rkyv::Archived<Vec<MessageItem>>,
                rkyv::rancor::Error,
            >(rkyv_slice)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            rkyv::deserialize::<Vec<MessageItem>, rkyv::rancor::Error>(archived_msgs)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?
        } else {
            let mut aligned = rkyv::util::AlignedVec::<16>::new();
            aligned.extend_from_slice(rkyv_slice);
            let archived_msgs = rkyv::access::<
                rkyv::Archived<Vec<MessageItem>>,
                rkyv::rancor::Error,
            >(&aligned)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            rkyv::deserialize::<Vec<MessageItem>, rkyv::rancor::Error>(archived_msgs)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?
        };

        {
            let mut cache = self.cache.lock().unwrap();
            *cache = Some(list.clone());
        }
        Ok(list)
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

        if (rkyv_slice.as_ptr() as usize).is_multiple_of(8) {
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
        {
            let cache = self.cache.lock().unwrap();
            if let Some(ref list) = *cache {
                return Ok(list.iter().find(|m| m.id == message_id).cloned());
            }
        }

        let inner = self.inner.lock().unwrap();
        let rkyv_slice = inner.get_rkyv_slice();
        if rkyv_slice.is_empty() {
            return Ok(None);
        }

        if (rkyv_slice.as_ptr() as usize).is_multiple_of(8) {
            let archived_msgs = rkyv::access::<
                rkyv::Archived<Vec<MessageItem>>,
                rkyv::rancor::Error,
            >(rkyv_slice)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            for item in archived_msgs.iter() {
                if item.id == message_id {
                    let message = rkyv::deserialize::<MessageItem, rkyv::rancor::Error>(item)
                        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                    return Ok(Some(message));
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
            for item in archived_msgs.iter() {
                if item.id == message_id {
                    let message = rkyv::deserialize::<MessageItem, rkyv::rancor::Error>(item)
                        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                    return Ok(Some(message));
                }
            }
        }

        Ok(None)
    }

    pub fn get_loro_changes(&self) -> Result<Vec<u8>, YntraError> {
        let inner = self.inner.lock().unwrap();
        inner.get_loro_changes()
    }

    pub fn apply_loro_update(&self, update_bytes: Vec<u8>) -> Result<(), YntraError> {
        {
            let mut inner = self.inner.lock().unwrap();
            *self.cache.lock().unwrap() = None;
            inner.apply_loro_update(&update_bytes)?;
        }
        Ok(())
    }
}

// --- ZeroCopyAuditStore ---

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
        let rkyv_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&entries)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
        {
            let mut inner = self.inner.lock().unwrap();
            *self.cache.lock().unwrap() = None;
            inner.write_serialized(&rkyv_bytes)?;
        }
        *self.cache.lock().unwrap() = Some(entries);
        Ok(())
    }

    pub fn read_all_audit_logs(&self) -> Result<Vec<AuditLogEntry>, YntraError> {
        {
            let cache = self.cache.lock().unwrap();
            if let Some(ref list) = *cache {
                return Ok(list.clone());
            }
        }

        let inner = self.inner.lock().unwrap();
        let rkyv_slice = inner.get_rkyv_slice();
        if rkyv_slice.is_empty() {
            return Ok(Vec::new());
        }

        let list: Vec<AuditLogEntry> = if (rkyv_slice.as_ptr() as usize).is_multiple_of(8) {
            let archived_entries = rkyv::access::<
                rkyv::Archived<Vec<AuditLogEntry>>,
                rkyv::rancor::Error,
            >(rkyv_slice)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            rkyv::deserialize::<Vec<AuditLogEntry>, rkyv::rancor::Error>(archived_entries)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?
        } else {
            let mut aligned = rkyv::util::AlignedVec::<16>::new();
            aligned.extend_from_slice(rkyv_slice);
            let archived_entries = rkyv::access::<
                rkyv::Archived<Vec<AuditLogEntry>>,
                rkyv::rancor::Error,
            >(&aligned)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            rkyv::deserialize::<Vec<AuditLogEntry>, rkyv::rancor::Error>(archived_entries)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?
        };

        {
            let mut cache = self.cache.lock().unwrap();
            *cache = Some(list.clone());
        }
        Ok(list)
    }

    pub fn read_audit_zero_copy(&self, entry_id: String) -> Result<Option<AuditLogEntry>, YntraError> {
        {
            let cache = self.cache.lock().unwrap();
            if let Some(ref list) = *cache {
                return Ok(list.iter().find(|e| e.id == entry_id).cloned());
            }
        }

        let inner = self.inner.lock().unwrap();
        let rkyv_slice = inner.get_rkyv_slice();
        if rkyv_slice.is_empty() {
            return Ok(None);
        }

        if (rkyv_slice.as_ptr() as usize).is_multiple_of(8) {
            let archived_entries = rkyv::access::<
                rkyv::Archived<Vec<AuditLogEntry>>,
                rkyv::rancor::Error,
            >(rkyv_slice)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            for item in archived_entries.iter() {
                if item.id == entry_id {
                    let entry = rkyv::deserialize::<AuditLogEntry, rkyv::rancor::Error>(item)
                        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                    return Ok(Some(entry));
                }
            }
        } else {
            let mut aligned = rkyv::util::AlignedVec::<16>::new();
            aligned.extend_from_slice(rkyv_slice);
            let archived_entries = rkyv::access::<
                rkyv::Archived<Vec<AuditLogEntry>>,
                rkyv::rancor::Error,
            >(&aligned)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            for item in archived_entries.iter() {
                if item.id == entry_id {
                    let entry = rkyv::deserialize::<AuditLogEntry, rkyv::rancor::Error>(item)
                        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                    return Ok(Some(entry));
                }
            }
        }

        Ok(None)
    }

    pub fn get_loro_changes(&self) -> Result<Vec<u8>, YntraError> {
        let inner = self.inner.lock().unwrap();
        inner.get_loro_changes()
    }

    pub fn apply_loro_update(&self, update_bytes: Vec<u8>) -> Result<(), YntraError> {
        {
            let mut inner = self.inner.lock().unwrap();
            *self.cache.lock().unwrap() = None;
            inner.apply_loro_update(&update_bytes)?;
        }
        Ok(())
    }
}

// --- ZeroCopyNoteStore ---

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
        let rkyv_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&notes)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
        {
            let mut inner = self.inner.lock().unwrap();
            *self.cache.lock().unwrap() = None;
            inner.write_serialized(&rkyv_bytes)?;
        }
        *self.cache.lock().unwrap() = Some(notes);
        Ok(())
    }

    pub fn read_all_notes(&self) -> Result<Vec<DailyNote>, YntraError> {
        {
            let cache = self.cache.lock().unwrap();
            if let Some(ref list) = *cache {
                return Ok(list.clone());
            }
        }

        let inner = self.inner.lock().unwrap();
        let rkyv_slice = inner.get_rkyv_slice();
        if rkyv_slice.is_empty() {
            return Ok(Vec::new());
        }

        let list: Vec<DailyNote> = if (rkyv_slice.as_ptr() as usize).is_multiple_of(8) {
            let archived_notes = rkyv::access::<
                rkyv::Archived<Vec<DailyNote>>,
                rkyv::rancor::Error,
            >(rkyv_slice)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            rkyv::deserialize::<Vec<DailyNote>, rkyv::rancor::Error>(archived_notes)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?
        } else {
            let mut aligned = rkyv::util::AlignedVec::<16>::new();
            aligned.extend_from_slice(rkyv_slice);
            let archived_notes = rkyv::access::<
                rkyv::Archived<Vec<DailyNote>>,
                rkyv::rancor::Error,
            >(&aligned)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            rkyv::deserialize::<Vec<DailyNote>, rkyv::rancor::Error>(archived_notes)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?
        };

        {
            let mut cache = self.cache.lock().unwrap();
            *cache = Some(list.clone());
        }
        Ok(list)
    }

    pub fn read_note_zero_copy(&self, note_id: String) -> Result<Option<DailyNote>, YntraError> {
        {
            let cache = self.cache.lock().unwrap();
            if let Some(ref list) = *cache {
                return Ok(list.iter().find(|n| n.id == note_id).cloned());
            }
        }

        let inner = self.inner.lock().unwrap();
        let rkyv_slice = inner.get_rkyv_slice();
        if rkyv_slice.is_empty() {
            return Ok(None);
        }

        if (rkyv_slice.as_ptr() as usize).is_multiple_of(8) {
            let archived_notes = rkyv::access::<
                rkyv::Archived<Vec<DailyNote>>,
                rkyv::rancor::Error,
            >(rkyv_slice)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            for item in archived_notes.iter() {
                if item.id == note_id {
                    let note = rkyv::deserialize::<DailyNote, rkyv::rancor::Error>(item)
                        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                    return Ok(Some(note));
                }
            }
        } else {
            let mut aligned = rkyv::util::AlignedVec::<16>::new();
            aligned.extend_from_slice(rkyv_slice);
            let archived_notes = rkyv::access::<
                rkyv::Archived<Vec<DailyNote>>,
                rkyv::rancor::Error,
            >(&aligned)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            for item in archived_notes.iter() {
                if item.id == note_id {
                    let note = rkyv::deserialize::<DailyNote, rkyv::rancor::Error>(item)
                        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                    return Ok(Some(note));
                }
            }
        }

        Ok(None)
    }

    pub fn get_loro_changes(&self) -> Result<Vec<u8>, YntraError> {
        let inner = self.inner.lock().unwrap();
        inner.get_loro_changes()
    }

    pub fn apply_loro_update(&self, update_bytes: Vec<u8>) -> Result<(), YntraError> {
        {
            let mut inner = self.inner.lock().unwrap();
            *self.cache.lock().unwrap() = None;
            inner.apply_loro_update(&update_bytes)?;
        }
        Ok(())
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
