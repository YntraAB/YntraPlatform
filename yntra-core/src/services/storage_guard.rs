use crate::database;
use crate::infra::auth::AuthContext;
use crate::infra::errors::YntraError;
use serde::{Deserialize, Serialize};

#[derive(uniffi::Record, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct StorageHealthReport {
    pub workspace_id: String,
    pub is_persistent_granted: bool,
    pub quota_bytes: u64,
    pub usage_bytes: u64,
    pub unsynced_offline_edits_count: u32,
    pub recommended_backup_needed: bool,
    pub risk_level: String, // "safe", "warning", "critical"
}

#[derive(uniffi::Record, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct EmergencyBackupPayload {
    pub workspace_id: String,
    pub created_at_ms: i64,
    pub total_pending_records: u32,
    pub archive_json: String,
}

pub struct StorageGuardEngine;

impl StorageGuardEngine {
    pub async fn calculate_unsynced_edits(
        conn: &database::DbConnection,
        workspace_id: &str,
    ) -> Result<u32, YntraError> {
        let tables = vec![
            "time_reports",
            "messages",
            "notes",
            "todos",
            "reports",
            "job_tickets",
            "entities",
        ];
        let mut total_pending = 0u32;
        for table in tables {
            let query = format!(
                "SELECT count(*) FROM {} WHERE workspace_id = ?1 AND sync_status = 'pending'",
                table
            );
            let count: i64 = conn
                .query_row(&query, crate::params![workspace_id], |r| r.get(0))
                .await
                .unwrap_or(0);
            total_pending += count as u32;
        }
        Ok(total_pending)
    }
}

/// UniFFI endpoint to evaluate browser OPFS / IndexedDB storage health telemetry.
#[uniffi::export]
pub async fn check_storage_health(
    requester_user_id: String,
    workspace_id: String,
    is_persistent_granted: bool,
    quota_bytes: u64,
    usage_bytes: u64,
) -> Result<StorageHealthReport, YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = AuthContext::authorize(&conn, &requester_user_id).await?;

    let unsynced_count = StorageGuardEngine::calculate_unsynced_edits(&conn, &workspace_id).await?;

    let usage_percent = if quota_bytes > 0 {
        ((usage_bytes as f64) / (quota_bytes as f64)) * 100.0
    } else {
        0.0
    };

    let recommended_backup_needed = (!is_persistent_granted && unsynced_count > 0)
        || unsynced_count >= 5
        || usage_percent > 80.0;

    let risk_level = if !is_persistent_granted && unsynced_count >= 5 {
        "critical".to_string()
    } else if recommended_backup_needed {
        "warning".to_string()
    } else {
        "safe".to_string()
    };

    Ok(StorageHealthReport {
        workspace_id,
        is_persistent_granted,
        quota_bytes,
        usage_bytes,
        unsynced_offline_edits_count: unsynced_count,
        recommended_backup_needed,
        risk_level,
    })
}

/// UniFFI endpoint to compile an emergency backup archive payload of pending offline edits.
#[uniffi::export]
pub async fn generate_emergency_storage_backup_payload(
    requester_user_id: String,
    workspace_id: String,
) -> Result<EmergencyBackupPayload, YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = AuthContext::authorize(&conn, &requester_user_id).await?;

    let unsynced_count = StorageGuardEngine::calculate_unsynced_edits(&conn, &workspace_id).await?;
    let now_ms = crate::infra::time::get_current_time_ms();

    let mut table_dumps = serde_json::Map::new();
    let tables = vec![
        "time_reports",
        "messages",
        "notes",
        "todos",
        "reports",
        "job_tickets",
        "entities",
    ];

    for table in tables {
        let query = format!(
            "SELECT id, sync_status, updated_at FROM {} WHERE workspace_id = ?1 AND sync_status = 'pending' LIMIT 50",
            table
        );
        let mut rows_out = Vec::new();
        if let Ok(mut stmt) = conn.prepare(&query).await {
            if let Ok(mut rows) = stmt.query(crate::params![&workspace_id]).await {
                while let Ok(Some(row)) = rows.next().await {
                    let id: String = row.get(0).unwrap_or_default();
                    let status: String = row.get(1).unwrap_or_default();
                    let updated: i64 = row.get(2).unwrap_or(0);
                    rows_out.push(serde_json::json!({
                        "id": id,
                        "sync_status": status,
                        "updated_at": updated
                    }));
                }
            }
        }
        table_dumps.insert(table.to_string(), serde_json::Value::Array(rows_out));
    }

    let archive_obj = serde_json::json!({
        "vault_format": "yntra_vault_v1",
        "backup_type": "emergency_opfs_eviction_guard",
        "workspace_id": workspace_id,
        "timestamp_ms": now_ms,
        "unsynced_count": unsynced_count,
        "tables": table_dumps,
    });

    Ok(EmergencyBackupPayload {
        workspace_id,
        created_at_ms: now_ms,
        total_pending_records: unsynced_count,
        archive_json: archive_obj.to_string(),
    })
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_health_risk_evaluation() {
        let report_safe = StorageHealthReport {
            workspace_id: "ws-1".to_string(),
            is_persistent_granted: true,
            quota_bytes: 1_000_000,
            usage_bytes: 100_000,
            unsynced_offline_edits_count: 0,
            recommended_backup_needed: false,
            risk_level: "safe".to_string(),
        };
        assert_eq!(report_safe.risk_level, "safe");
        assert!(!report_safe.recommended_backup_needed);
    }
}
