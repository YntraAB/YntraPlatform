use crate::database;
use crate::observer::notify_observers;
use crate::{MessageItem, YntraError};

#[uniffi::export]
pub async fn get_messages(user_id: String) -> Result<Vec<MessageItem>, YntraError> {
    let conn = database::acquire_connection().await?;

    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, sender_id, receiver_id, target_team_id, subject, body, is_read, created_at, updated_at, sync_status
         FROM messages
         WHERE sender_id = ?1 OR receiver_id = ?1 OR target_team_id IN (
             SELECT team_id FROM team_members WHERE user_id = ?1
         )
         ORDER BY created_at ASC"
    ).await?;

    let list = stmt.query_map(crate::params![user_id], |row| {
        let is_read_int: i32 = row.get(7)?;
        Ok(MessageItem {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            sender_id: row.get(2)?,
            receiver_id: row.get(3)?,
            target_team_id: row.get(4)?,
            subject: row.get(5)?,
            body: row.get(6)?,
            is_read: is_read_int != 0,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
            sync_status: row.get(10)?,
        })
    }).await?;

    Ok(list)
}

#[uniffi::export]
pub async fn send_message(
    workspace_id: String,
    sender_id: String,
    receiver_id: Option<String>,
    team_id: Option<String>,
    subject: String,
    body: String,
) -> Result<MessageItem, YntraError> {
    let id = uuid::Uuid::new_v4().to_string();
    let created_at = crate::infra::time::get_current_datetime_str();
    let now_ms = crate::infra::time::get_current_time_ms();
    let item = MessageItem {
        id: id.clone(),
        workspace_id,
        sender_id: Some(sender_id),
        receiver_id,
        target_team_id: team_id,
        subject: Some(subject),
        body: Some(body),
        is_read: false,
        created_at,
        updated_at: now_ms,
        sync_status: "pending".to_string(),
    };

    let conn = database::acquire_connection().await?;

    conn.execute(
        "INSERT INTO messages (id, workspace_id, sender_id, receiver_id, target_team_id, subject, body, is_read, created_at, updated_at, sync_status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?9, 'pending')",
        crate::params![
            &item.id,
            &item.workspace_id,
            &item.sender_id,
            &item.receiver_id,
            &item.target_team_id,
            &item.subject,
            &item.body,
            &item.created_at,
            &item.updated_at
        ],
    ).await?;

    notify_observers();

    Ok(item)
}

#[uniffi::export]
pub async fn mark_message_read(id: String) -> Result<(), YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms();
    let conn = database::acquire_connection().await?;

    conn.execute("UPDATE messages SET is_read = 1, updated_at = ?1, sync_status = 'pending' WHERE id = ?2", crate::params![now_ms, id]).await?;

    notify_observers();
    Ok(())
}
