use crate::database;
use crate::observer::notify_observers;
use crate::{Team, TeamEvent, YntraError};

#[uniffi::export]
pub async fn get_teams(requester_user_id: String) -> Result<Vec<Team>, YntraError> {
    let conn = database::acquire_connection().await?;
    
    let requester_ws: String = conn.query_row(
        "SELECT workspace_id FROM users WHERE id = ?1",
        crate::params![&requester_user_id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::AuthError("Requester user not found".to_string()))?;

    let mut stmt = conn.prepare("SELECT id, workspace_id, name, updated_at, sync_status FROM teams WHERE workspace_id = ?1").await?;

    let list = stmt.query_map(crate::params![&requester_ws], |row| {
        Ok(Team {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            name: row.get(2)?,
            updated_at: row.get(3)?,
            sync_status: row.get(4)?,
        })
    }).await?;

    Ok(list)
}

#[uniffi::export]
pub async fn get_events(requester_user_id: String, team_id: Option<String>) -> Result<Vec<TeamEvent>, YntraError> {
    let conn = database::acquire_connection().await?;

    let requester_ws: String = conn.query_row(
        "SELECT workspace_id FROM users WHERE id = ?1",
        crate::params![&requester_user_id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::AuthError("Requester user not found".to_string()))?;

    let (query, params) = match team_id {
        Some(tid) => (
            "SELECT id, workspace_id, user_id, team_id, assignee_id, title, start_time, end_time, metadata, updated_at, sync_status FROM events WHERE team_id = ?1 AND workspace_id = ?2".to_string(),
            vec![tid, requester_ws],
        ),
        None => (
            "SELECT id, workspace_id, user_id, team_id, assignee_id, title, start_time, end_time, metadata, updated_at, sync_status FROM events WHERE workspace_id = ?1".to_string(),
            vec![requester_ws],
        ),
    };

    let mut stmt = conn.prepare(&query).await?;
    let list = stmt.query_map(crate::rusqlite::params_from_iter(params), |row| {
        Ok(TeamEvent {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            user_id: row.get(2)?,
            team_id: row.get(3)?,
            assignee_id: row.get(4)?,
            title: row.get(5)?,
            start_time: row.get(6)?,
            end_time: row.get(7)?,
            metadata: row.get(8)?,
            updated_at: row.get(9)?,
            sync_status: row.get(10)?,
        })
    }).await?;

    Ok(list)
}

#[uniffi::export]
pub async fn add_event(
    workspace_id: String,
    title: String,
    start_time: String,
    end_time: String,
    team_id: Option<String>,
    assignee_id: Option<String>,
    recipient_id: Option<String>,
) -> Result<TeamEvent, YntraError> {
    let id = uuid::Uuid::new_v4().to_string();
    let now_ms = crate::infra::time::get_current_time_ms();
    let event = TeamEvent {
        id: id.clone(),
        workspace_id,
        user_id: recipient_id,
        team_id: team_id.clone(),
        assignee_id: assignee_id.clone(),
        title,
        start_time,
        end_time,
        metadata: "{}".to_string(),
        updated_at: now_ms,
        sync_status: "pending".to_string(),
    };

    let conn = database::acquire_connection().await?;

    conn.execute(
        "INSERT INTO events (id, workspace_id, user_id, team_id, assignee_id, title, start_time, end_time, metadata, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, '{}', ?9, 'pending')",
        crate::params![
            &event.id,
            &event.workspace_id,
            &event.user_id,
            &event.team_id,
            &event.assignee_id,
            &event.title,
            &event.start_time,
            &event.end_time,
            &event.updated_at
        ],
    ).await?;

    notify_observers();
    Ok(event)
}

#[uniffi::export]
#[allow(clippy::too_many_arguments)]
pub async fn add_event_with_metadata(
    workspace_id: String,
    title: String,
    start_time: String,
    end_time: String,
    team_id: Option<String>,
    assignee_id: Option<String>,
    recipient_id: Option<String>,
    metadata: String,
) -> Result<TeamEvent, YntraError> {
    let id = uuid::Uuid::new_v4().to_string();
    let now_ms = crate::infra::time::get_current_time_ms();
    let event = TeamEvent {
        id: id.clone(),
        workspace_id,
        user_id: recipient_id,
        team_id: team_id.clone(),
        assignee_id: assignee_id.clone(),
        title,
        start_time,
        end_time,
        metadata: metadata.clone(),
        updated_at: now_ms,
        sync_status: "pending".to_string(),
    };

    let conn = database::acquire_connection().await?;

    conn.execute(
        "INSERT INTO events (id, workspace_id, user_id, team_id, assignee_id, title, start_time, end_time, metadata, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'pending')",
        crate::params![
            &event.id,
            &event.workspace_id,
            &event.user_id,
            &event.team_id,
            &event.assignee_id,
            &event.title,
            &event.start_time,
            &event.end_time,
            &event.metadata,
            &event.updated_at
        ],
    ).await?;

    notify_observers();
    Ok(event)
}

#[uniffi::export]
pub async fn delete_event(id: String) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;

    conn.execute("DELETE FROM events WHERE id = ?1", crate::params![id]).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn add_team_via_directory(workspace_id: String, name: String) -> Result<Team, YntraError> {
    let id = uuid::Uuid::new_v4().to_string();
    let now_ms = crate::infra::time::get_current_time_ms();
    let item = Team {
        id: id.clone(),
        workspace_id: workspace_id.clone(),
        name,
        updated_at: now_ms,
        sync_status: "pending".to_string(),
    };

    let conn = database::acquire_connection().await?;

    conn.execute(
        "INSERT INTO teams (id, workspace_id, name, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, 'pending')",
        crate::params![&item.id, &item.workspace_id, &item.name, &item.updated_at],
    ).await?;

    notify_observers();
    Ok(item)
}

#[uniffi::export]
pub async fn update_event_time(id: String, start_time: String, end_time: String) -> Result<(), YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms();
    let conn = database::acquire_connection().await?;

    conn.execute(
        "UPDATE events SET start_time = ?1, end_time = ?2, updated_at = ?3, sync_status = 'pending' WHERE id = ?4",
        crate::params![start_time, end_time, now_ms, id],
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
#[allow(clippy::too_many_arguments)]
pub async fn update_event(
    id: String,
    title: String,
    start_time: String,
    end_time: String,
    team_id: Option<String>,
    assignee_id: Option<String>,
    recipient_id: Option<String>,
    metadata: String,
) -> Result<(), YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms();
    let conn = database::acquire_connection().await?;

    conn.execute(
        "UPDATE events SET title = ?1, start_time = ?2, end_time = ?3, team_id = ?4, assignee_id = ?5, user_id = ?6, metadata = ?7, updated_at = ?8, sync_status = 'pending' WHERE id = ?9",
        crate::params![title, start_time, end_time, team_id, assignee_id, recipient_id, metadata, now_ms, id],
    ).await?;

    notify_observers();
    Ok(())
}
