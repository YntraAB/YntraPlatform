use crate::YntraError;
use crate::database;
use std::sync::{Mutex, OnceLock};

#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct ClientSyncProfile {
    pub profile_mode: String, // "full", "thin_metadata_only", "custom_subset"
    pub max_history_days: u32,
    pub sync_media_attachments: bool,
    pub active_tables_json: String,
    pub updated_at_ms: i64,
}

#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct ThinSyncTableSummary {
    pub table_name: String,
    pub total_rows: u32,
    pub active_rows: u32,
    pub expired_rows: u32,
    pub estimated_bytes: u64,
}

static ACTIVE_SYNC_PROFILE: OnceLock<Mutex<ClientSyncProfile>> = OnceLock::new();

fn get_profile_lock() -> &'static Mutex<ClientSyncProfile> {
    ACTIVE_SYNC_PROFILE.get_or_init(|| {
        Mutex::new(ClientSyncProfile {
            profile_mode: "full".to_string(),
            max_history_days: 30,
            sync_media_attachments: true,
            active_tables_json: "[\"job_tickets\", \"messages\", \"notes\", \"todos\", \"events\", \"time_reports\"]".to_string(),
            updated_at_ms: crate::infra::time::get_current_time_ms(),
        })
    })
}

/// Set active client synchronization profile mode (Full, Thin Metadata Only, or Custom Subset).
#[uniffi::export]
pub fn set_client_sync_profile(profile: ClientSyncProfile) -> Result<(), YntraError> {
    let mut lock = get_profile_lock()
        .lock()
        .map_err(|_| YntraError::CryptoError("Sync profile lock poisoned".to_string()))?;
    *lock = profile;
    crate::infra::observer::notify_observers();
    Ok(())
}

/// Get currently active client synchronization profile.
#[uniffi::export]
pub fn get_client_sync_profile() -> Result<ClientSyncProfile, YntraError> {
    let lock = get_profile_lock()
        .lock()
        .map_err(|_| YntraError::CryptoError("Sync profile lock poisoned".to_string()))?;
    Ok(lock.clone())
}

/// Get estimated disk space and active vs expired row counts per table under current sync profile.
#[uniffi::export]
pub async fn get_thin_sync_tables_summary(
    requester_user_id: String,
    workspace_id: String,
) -> Result<Vec<ThinSyncTableSummary>, YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let profile = get_client_sync_profile()?;
    let now_ms = crate::infra::time::get_current_time_ms();
    let cutoff_ms = if profile.profile_mode == "full" {
        0i64
    } else {
        now_ms - ((profile.max_history_days as i64) * 86_400_000)
    };

    let tables = vec![
        "job_tickets",
        "messages",
        "notes",
        "todos",
        "events",
        "time_reports",
        "reports",
    ];

    let mut summaries = Vec::new();

    for table in tables {
        let total_sql = format!("SELECT count(*) FROM {} WHERE workspace_id = ?1", table);
        let total_rows: i64 = conn
            .query_row(&total_sql, crate::params![&workspace_id], |r| r.get(0))
            .await
            .unwrap_or(0);

        let active_sql = format!(
            "SELECT count(*) FROM {} WHERE workspace_id = ?1 AND updated_at >= ?2",
            table
        );
        let active_rows: i64 = conn
            .query_row(&active_sql, crate::params![&workspace_id, cutoff_ms], |r| r.get(0))
            .await
            .unwrap_or(total_rows);

        let expired_rows = if total_rows > active_rows {
            (total_rows - active_rows) as u32
        } else {
            0u32
        };

        // Estimate average row size ~ 512 bytes
        let estimated_bytes = (total_rows as u64) * 512;

        summaries.push(ThinSyncTableSummary {
            table_name: table.to_string(),
            total_rows: total_rows as u32,
            active_rows: active_rows as u32,
            expired_rows,
            estimated_bytes,
        });
    }

    Ok(summaries)
}

/// Purge local database records exceeding the active thin sync retention cutoff date.
#[uniffi::export]
pub async fn purge_expired_thin_sync_cache(
    requester_user_id: String,
    workspace_id: String,
) -> Result<u32, YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let profile = get_client_sync_profile()?;
    if profile.profile_mode == "full" {
        return Ok(0); // Full replication retains all history
    }

    let now_ms = crate::infra::time::get_current_time_ms();
    let cutoff_ms = now_ms - ((profile.max_history_days as i64) * 86_400_000);

    let tables = vec![
        "messages",
        "notes",
        "todos",
        "events",
        "time_reports",
        "reports",
    ];

    let mut total_purged = 0u32;

    for table in tables {
        let delete_sql = format!(
            "DELETE FROM {} WHERE workspace_id = ?1 AND updated_at < ?2 AND sync_status = 'synced'",
            table
        );
        let purged = conn
            .execute(&delete_sql, crate::params![&workspace_id, cutoff_ms])
            .await
            .unwrap_or(0);

        total_purged += purged as u32;
    }

    if total_purged > 0 {
        crate::infra::observer::notify_observers();
    }

    Ok(total_purged)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_thin_sync_profile_and_pruning() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        let ws_id = format!("ws-thin-{}", crate::infra::time::get_current_time_ms());
        conn.execute(
            "INSERT INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'Thin WS', '[]', '{}')",
            crate::params![&ws_id],
        )
        .await
        .unwrap();

        conn.execute(
            "INSERT INTO teams (id, workspace_id, name) VALUES ('t-1', ?1, 'Team A')",
            crate::params![&ws_id],
        )
        .await
        .unwrap();

        conn.execute(
            "INSERT INTO users (id, workspace_id, email, role) VALUES ('u-1', ?1, 'user@test.com', 'user')",
            crate::params![&ws_id],
        )
        .await
        .unwrap();

        let now_ms = crate::infra::time::get_current_time_ms();
        let old_ms = now_ms - (60 * 86_400_000); // 60 days ago

        // Insert recent note and expired note
        conn.execute(
            "INSERT INTO notes (id, workspace_id, team_id, author_id, subject, content, created_at, updated_at, sync_status) VALUES ('note-recent', ?1, 't-1', 'u-1', 'Recent', 'Content', '2026-08-04', ?2, 'synced')",
            crate::params![&ws_id, now_ms],
        )
        .await
        .unwrap();

        conn.execute(
            "INSERT INTO notes (id, workspace_id, team_id, author_id, subject, content, created_at, updated_at, sync_status) VALUES ('note-old', ?1, 't-1', 'u-1', 'Old', 'Content', '2026-06-01', ?2, 'synced')",
            crate::params![&ws_id, old_ms],
        )
        .await
        .unwrap();

        // Configure thin sync profile (30 days limit)
        let profile = ClientSyncProfile {
            profile_mode: "thin_metadata_only".to_string(),
            max_history_days: 30,
            sync_media_attachments: false,
            active_tables_json: "[\"notes\"]".to_string(),
            updated_at_ms: now_ms,
        };
        set_client_sync_profile(profile).unwrap();

        let summary = get_thin_sync_tables_summary("u-1".to_string(), ws_id.clone())
            .await
            .unwrap();
        let notes_sum = summary.iter().find(|s| s.table_name == "notes").unwrap();
        assert_eq!(notes_sum.total_rows, 2);
        assert_eq!(notes_sum.active_rows, 1);
        assert_eq!(notes_sum.expired_rows, 1);

        // Purge expired cache
        let purged = purge_expired_thin_sync_cache("u-1".to_string(), ws_id.clone())
            .await
            .unwrap();
        assert_eq!(purged, 1, "Should purge 1 note older than 30 days cutoff");

        // Verify remaining note
        let remaining_sum = get_thin_sync_tables_summary("u-1".to_string(), ws_id.clone())
            .await
            .unwrap();
        let notes_sum2 = remaining_sum.iter().find(|s| s.table_name == "notes").unwrap();
        assert_eq!(notes_sum2.total_rows, 1);

        // Reset to full
        set_client_sync_profile(ClientSyncProfile {
            profile_mode: "full".to_string(),
            max_history_days: 30,
            sync_media_attachments: true,
            active_tables_json: "[]".to_string(),
            updated_at_ms: now_ms,
        })
        .unwrap();
    }
}
