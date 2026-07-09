#![allow(unused)]

use crate::{JournalEntry, MedicationItem, YntraError};

#[uniffi::export]
pub async fn get_medications(client_id: String, actor_id: String) -> Result<Vec<MedicationItem>, YntraError> {
    Ok(vec![])
}

#[uniffi::export]
pub async fn get_journals(client_id: String, actor_id: String) -> Result<Vec<JournalEntry>, YntraError> {
    Ok(vec![])
}

#[uniffi::export]
pub async fn add_journal_entry(
    workspace_id: String,
    client_id: String,
    author_id: String,
    content: String,
) -> Result<JournalEntry, YntraError> {
    Err(YntraError::AuthError("Healthcare module is deprecated".to_string()))
}

#[uniffi::export]
pub async fn add_medication(
    workspace_id: String,
    client_id: String,
    actor_id: String,
    name: String,
    dosage: String,
    frequency: String,
    instructions: String,
) -> Result<MedicationItem, YntraError> {
    Err(YntraError::AuthError("Healthcare module is deprecated".to_string()))
}

#[cfg(test)]
mod tests {
    // Tests are removed since the healthcare module has been deprecated and its database tables removed.
}
