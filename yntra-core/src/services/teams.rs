#[cfg(not(target_arch = "wasm32"))]
use crate::database;
use crate::observer::notify_observers;
use crate::{Team, TeamEvent, YntraError};

#[cfg(target_arch = "wasm32")]
use crate::wasm_store;

#[uniffi::export]
pub async fn get_teams() -> Result<Vec<Team>, YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;

        let mut stmt = conn.prepare("SELECT id, workspace_id, name, updated_at, sync_status FROM teams").await?;

        let list = stmt.query_map((), |row| {
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

    #[cfg(target_arch = "wasm32")]
    {
        let store = wasm_store::get_store().lock().unwrap();
        Ok(store.teams.clone())
    }
}

#[uniffi::export]
pub async fn get_events(team_id: Option<String>) -> Result<Vec<TeamEvent>, YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;

        let query = match team_id {
            Some(_) => {
                "SELECT id, workspace_id, user_id, team_id, assignee_id, title, start_time, end_time, metadata, updated_at, sync_status FROM events WHERE team_id = ?1"
            }
            None => {
                "SELECT id, workspace_id, user_id, team_id, assignee_id, title, start_time, end_time, metadata, updated_at, sync_status FROM events"
            }
        };

        let mut stmt = conn.prepare(query).await?;

        let params: Vec<String> = match team_id {
            Some(tid) => vec![tid],
            None => vec![],
        };

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

    #[cfg(target_arch = "wasm32")]
    {
        let store = wasm_store::get_store().lock().unwrap();
        if let Some(tid) = team_id {
            Ok(store
                .events
                .iter()
                .filter(|e| e.team_id == Some(tid.clone()))
                .cloned()
                .collect())
        } else {
            Ok(store.events.clone())
        }
    }
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

    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;

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
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        store.events.push(event.clone());
        notify_observers();
    }

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

    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;

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
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        store.events.push(event.clone());
        notify_observers();
    }

    Ok(event)
}

#[uniffi::export]
pub async fn delete_event(id: String) -> Result<(), YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;

        conn.execute("DELETE FROM events WHERE id = ?1", crate::params![id]).await?;

        notify_observers();
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        store.events.retain(|e| e.id != id);
        notify_observers();
        Ok(())
    }
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

    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;

        conn.execute(
            "INSERT INTO teams (id, workspace_id, name, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, 'pending')",
            crate::params![&item.id, &item.workspace_id, &item.name, &item.updated_at],
        ).await?;

        notify_observers();
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        store.teams.push(item.clone());
        notify_observers();
    }

    Ok(item)
}

#[uniffi::export]
pub async fn update_event_time(id: String, start_time: String, end_time: String) -> Result<(), YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms();
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;

        conn.execute(
            "UPDATE events SET start_time = ?1, end_time = ?2, updated_at = ?3, sync_status = 'pending' WHERE id = ?4",
            crate::params![start_time, end_time, now_ms, id],
        ).await?;

        notify_observers();
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        if let Some(ev) = store.events.iter_mut().find(|e| e.id == id) {
            ev.start_time = start_time;
            ev.end_time = end_time;
            ev.updated_at = now_ms;
            ev.sync_status = "pending".to_string();
        }
        notify_observers();
        Ok(())
    }
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
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;

        conn.execute(
            "UPDATE events SET title = ?1, start_time = ?2, end_time = ?3, team_id = ?4, assignee_id = ?5, user_id = ?6, metadata = ?7, updated_at = ?8, sync_status = 'pending' WHERE id = ?9",
            crate::params![title, start_time, end_time, team_id, assignee_id, recipient_id, metadata, now_ms, id],
        ).await?;

        notify_observers();
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        if let Some(ev) = store.events.iter_mut().find(|e| e.id == id) {
            ev.title = title;
            ev.start_time = start_time;
            ev.end_time = end_time;
            ev.team_id = team_id;
            ev.assignee_id = assignee_id;
            ev.user_id = recipient_id;
            ev.metadata = metadata;
            ev.updated_at = now_ms;
            ev.sync_status = "pending".to_string();
        }
        notify_observers();
        Ok(())
    }
}
