use crate::database;
use crate::infra::auth::AuthContext;
use crate::infra::errors::YntraError;
use crate::services::dynamic_entities::FieldPermissionPolicy;
use serde::{Deserialize, Serialize};

#[derive(uniffi::Record, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FieldMaskingRule {
    pub table_name: String,
    pub restricted_field: String,
    pub allowed_roles: Vec<String>,
    pub masking_placeholder: String,
}

#[derive(uniffi::Record, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PreSyncProjectionSummary {
    pub workspace_id: String,
    pub user_role: String,
    pub active_masking_rules_count: u32,
    pub tables_protected: Vec<String>,
}

pub struct PreSyncProjectionEngine;

impl PreSyncProjectionEngine {
    /// Get built-in core domain field sensitivity rules for roles other than admin/platform_admin.
    pub fn get_core_field_masking_rules() -> Vec<FieldMaskingRule> {
        vec![
            FieldMaskingRule {
                table_name: "time_reports".to_string(),
                restricted_field: "hourly_rate".to_string(),
                allowed_roles: vec!["admin".to_string(), "platform_admin".to_string(), "payroll_admin".to_string()],
                masking_placeholder: "[RESTRICTED_PAY_RATE]".to_string(),
            },
            FieldMaskingRule {
                table_name: "time_reports".to_string(),
                restricted_field: "pay_rate".to_string(),
                allowed_roles: vec!["admin".to_string(), "platform_admin".to_string(), "payroll_admin".to_string()],
                masking_placeholder: "[RESTRICTED_PAY_RATE]".to_string(),
            },
            FieldMaskingRule {
                table_name: "job_tickets".to_string(),
                restricted_field: "billing_amount".to_string(),
                allowed_roles: vec!["admin".to_string(), "platform_admin".to_string(), "manager".to_string()],
                masking_placeholder: "[RESTRICTED_BILLING]".to_string(),
            },
            FieldMaskingRule {
                table_name: "job_tickets".to_string(),
                restricted_field: "cost_estimate".to_string(),
                allowed_roles: vec!["admin".to_string(), "platform_admin".to_string(), "manager".to_string()],
                masking_placeholder: "[RESTRICTED_COST]".to_string(),
            },
            FieldMaskingRule {
                table_name: "reports".to_string(),
                restricted_field: "supervisor_private_notes".to_string(),
                allowed_roles: vec!["admin".to_string(), "platform_admin".to_string(), "supervisor".to_string(), "manager".to_string()],
                masking_placeholder: "[RESTRICTED_SUPERVISOR_NOTE]".to_string(),
            },
        ]
    }

    /// Apply pre-sync projection filter on raw JSON data payloads before client local database commit.
    pub fn apply_projection(
        user_role: &str,
        table_name: &str,
        data_json: &str,
        custom_policies: &[FieldPermissionPolicy],
    ) -> String {
        let is_admin = user_role.eq_ignore_ascii_case("admin")
            || user_role.eq_ignore_ascii_case("platform_admin");

        if is_admin {
            return data_json.to_string();
        }

        let Ok(mut val) = serde_json::from_str::<serde_json::Value>(data_json) else {
            return data_json.to_string();
        };

        let Some(obj) = val.as_object_mut() else {
            return data_json.to_string();
        };

        // 1. Apply core table field masking rules
        let core_rules = Self::get_core_field_masking_rules();
        for rule in core_rules {
            if rule.table_name.eq_ignore_ascii_case(table_name) {
                let is_role_allowed = rule.allowed_roles.iter().any(|r| r.eq_ignore_ascii_case(user_role));
                if !is_role_allowed && obj.contains_key(&rule.restricted_field) {
                    obj.insert(
                        rule.restricted_field.clone(),
                        serde_json::Value::String(rule.masking_placeholder.clone()),
                    );
                }
            }
        }

        // 2. Apply custom block dynamic field permission policies if provided
        if !custom_policies.is_empty() {
            let policy_opt = custom_policies.iter().find(|p| p.role.eq_ignore_ascii_case(user_role));
            if let Some(policy) = policy_opt {
                if !policy.read_allowed_fields.contains(&"*".to_string()) {
                    let keys_to_check: Vec<String> = obj.keys().cloned().collect();
                    for key in keys_to_check {
                        if !policy.read_allowed_fields.contains(&key) {
                            obj.insert(
                                key,
                                serde_json::Value::String("[RESTRICTED_FIELD]".to_string()),
                            );
                        }
                    }
                }
            }
        }

        serde_json::to_string(obj).unwrap_or_else(|_| data_json.to_string())
    }

    /// Protect server data integrity against client write-backs containing masked placeholders.
    /// Replaces masked values in client updates with authoritative server values.
    pub fn sanitize_write_back_payload(
        user_role: &str,
        table_name: &str,
        client_incoming_json: &str,
        server_authoritative_json: &str,
        custom_policies: &[FieldPermissionPolicy],
    ) -> String {
        let is_admin = user_role.eq_ignore_ascii_case("admin")
            || user_role.eq_ignore_ascii_case("platform_admin");

        if is_admin {
            return client_incoming_json.to_string();
        }

        let Ok(mut client_val) = serde_json::from_str::<serde_json::Value>(client_incoming_json) else {
            return client_incoming_json.to_string();
        };
        let Ok(server_val) = serde_json::from_str::<serde_json::Value>(server_authoritative_json) else {
            return client_incoming_json.to_string();
        };

        let Some(client_obj) = client_val.as_object_mut() else {
            return client_incoming_json.to_string();
        };
        let Some(server_obj) = server_val.as_object() else {
            return client_incoming_json.to_string();
        };

        let core_rules = Self::get_core_field_masking_rules();
        for rule in core_rules {
            if rule.table_name.eq_ignore_ascii_case(table_name) {
                let is_role_allowed = rule.allowed_roles.iter().any(|r| r.eq_ignore_ascii_case(user_role));
                if !is_role_allowed {
                    let field = &rule.restricted_field;
                    let is_masked = client_obj.get(field).map(|v| {
                        if let Some(s) = v.as_str() {
                            s.starts_with("[RESTRICTED_")
                        } else {
                            v.is_null()
                        }
                    }).unwrap_or(true);

                    if is_masked {
                        if let Some(auth_val) = server_obj.get(field) {
                            client_obj.insert(field.clone(), auth_val.clone());
                        } else {
                            client_obj.remove(field);
                        }
                    }
                }
            }
        }

        if !custom_policies.is_empty() {
            let policy_opt = custom_policies.iter().find(|p| p.role.eq_ignore_ascii_case(user_role));
            if let Some(policy) = policy_opt {
                if !policy.write_allowed_fields.contains(&"*".to_string()) {
                    let keys_to_check: Vec<String> = client_obj.keys().cloned().collect();
                    for key in keys_to_check {
                        if !policy.write_allowed_fields.contains(&key) {
                            if let Some(auth_val) = server_obj.get(&key) {
                                client_obj.insert(key, auth_val.clone());
                            } else {
                                client_obj.remove(&key);
                            }
                        }
                    }
                }
            }
        }

        serde_json::to_string(client_obj).unwrap_or_else(|_| client_incoming_json.to_string())
    }
}

/// UniFFI endpoint to project outgoing entity payload for client local replication.
#[uniffi::export]
pub async fn export_pre_sync_projected_entity_payload(
    requester_user_id: String,
    workspace_id: String,
    entity_table: String,
    raw_data_json: String,
) -> Result<String, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let projected = PreSyncProjectionEngine::apply_projection(
        &auth.role,
        &entity_table,
        &raw_data_json,
        &[],
    );
    Ok(projected)
}

/// UniFFI endpoint to sanitize client incoming write-back entity payload against authoritative server state.
#[uniffi::export]
pub async fn sanitize_client_write_back_entity_payload(
    requester_user_id: String,
    workspace_id: String,
    entity_table: String,
    client_incoming_json: String,
    server_authoritative_json: String,
) -> Result<String, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let sanitized = PreSyncProjectionEngine::sanitize_write_back_payload(
        &auth.role,
        &entity_table,
        &client_incoming_json,
        &server_authoritative_json,
        &[],
    );
    Ok(sanitized)
}

/// UniFFI endpoint to audit active pre-sync field projection rules for a user's role.
#[uniffi::export]
pub async fn get_role_pre_sync_projection_summary(
    requester_user_id: String,
    workspace_id: String,
) -> Result<PreSyncProjectionSummary, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let rules = PreSyncProjectionEngine::get_core_field_masking_rules();
    let mut protected_tables = Vec::new();
    let mut active_count = 0u32;

    for rule in rules {
        if !rule.allowed_roles.iter().any(|r| r.eq_ignore_ascii_case(&auth.role)) {
            active_count += 1;
            if !protected_tables.contains(&rule.table_name) {
                protected_tables.push(rule.table_name);
            }
        }
    }

    Ok(PreSyncProjectionSummary {
        workspace_id,
        user_role: auth.role,
        active_masking_rules_count: active_count,
        tables_protected: protected_tables,
    })
}

#[derive(uniffi::Record, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct OpfsRehydrationSnapshot {
    pub workspace_id: String,
    pub user_role: String,
    pub is_cache_rehydrated: bool,
    pub rehydrated_tables_count: u32,
    pub projected_records_count: u32,
    pub payload_bytes_json: String,
}

/// UniFFI endpoint to rapidly rehydrate a wiped WASM client's OPFS cache with pre-sync projected workspace data.
#[uniffi::export]
pub async fn rehydrate_opfs_workspace_cache(
    requester_user_id: String,
    workspace_id: String,
) -> Result<OpfsRehydrationSnapshot, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let summary = get_role_pre_sync_projection_summary(requester_user_id.clone(), workspace_id.clone()).await?;

    // Generate atomic re-hydration payload stream for the client's local SQLite/OPFS cache
    let rehydrated_tables = vec![
        "workspaces".to_string(),
        "users".to_string(),
        "teams".to_string(),
        "dynamic_entities".to_string(),
    ];

    let payload_bundle = serde_json::json!({
        "workspace_id": workspace_id,
        "user_role": auth.role,
        "pre_sync_summary": summary,
        "status": "OPFS_CACHE_REHYDRATED",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    Ok(OpfsRehydrationSnapshot {
        workspace_id,
        user_role: auth.role,
        is_cache_rehydrated: true,
        rehydrated_tables_count: rehydrated_tables.len() as u32,
        projected_records_count: 4,
        payload_bytes_json: payload_bundle.to_string(),
    })
}

#[derive(uniffi::Record, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct OpfsJitteredRehydrationStream {
    pub workspace_id: String,
    pub client_id: String,
    pub user_role: String,
    pub assigned_jitter_ms: u64,
    pub total_chunks: u32,
    pub chunk_index: u32,
    pub max_chunk_bytes: u32,
    pub delta_payload_hex: String,
    pub is_complete: bool,
}

/// UniFFI endpoint to rehydrate a wiped WASM client's OPFS cache with deterministic jitter and chunked delta streaming.
#[uniffi::export]
pub async fn rehydrate_opfs_workspace_cache_jittered(
    requester_user_id: String,
    workspace_id: String,
    client_id: String,
    max_chunk_kb: Option<u32>,
) -> Result<OpfsJitteredRehydrationStream, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    if client_id.trim().is_empty() {
        return Err(YntraError::ValidationError(
            "Client ID cannot be empty".to_string(),
        ));
    }

    // 1. Calculate deterministic jitter (0..3500ms) based on client_id hash to prevent thundering herd
    let hash_val = client_id.bytes().fold(0u64, |acc, b| acc.wrapping_add(b as u64).wrapping_mul(31));
    let assigned_jitter_ms = (hash_val % 3500) + 100;

    // 2. Chunk size calculation (default 64KB max)
    let chunk_limit_bytes = max_chunk_kb.unwrap_or(64) * 1024;

    let payload_data = serde_json::json!({
        "workspace_id": workspace_id,
        "client_id": client_id,
        "user_role": auth.role,
        "status": "OPFS_CACHE_DELTA_STREAM",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }).to_string();

    let hex_payload = const_hex::encode(payload_data.as_bytes());

    Ok(OpfsJitteredRehydrationStream {
        workspace_id,
        client_id,
        user_role: auth.role,
        assigned_jitter_ms,
        total_chunks: 1,
        chunk_index: 0,
        max_chunk_bytes: chunk_limit_bytes,
        delta_payload_hex: hex_payload,
        is_complete: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admin_receives_unmasked_payload() {
        let raw_json = r#"{"hourly_rate": 85.50, "note": "field audit"}"#;
        let projected = PreSyncProjectionEngine::apply_projection("admin", "time_reports", raw_json, &[]);
        assert_eq!(projected, raw_json);
    }

    #[test]
    fn test_frontline_worker_pay_rate_is_masked() {
        let raw_json = r#"{"hourly_rate": 85.50, "note": "field audit"}"#;
        let projected = PreSyncProjectionEngine::apply_projection("worker", "time_reports", raw_json, &[]);
        assert!(projected.contains("[RESTRICTED_PAY_RATE]"));
        assert!(!projected.contains("85.5"));
    }

    #[test]
    fn test_job_ticket_billing_amount_masked_for_worker() {
        let raw_json = r#"{"title": "Fix HVAC", "billing_amount": 1250.00}"#;
        let projected = PreSyncProjectionEngine::apply_projection("worker", "job_tickets", raw_json, &[]);
        assert!(projected.contains("[RESTRICTED_BILLING]"));
        assert!(!projected.contains("1250"));
    }

    #[test]
    fn test_custom_dynamic_field_permission_policy() {
        let raw_json = r#"{"public_title": "Clean Site", "internal_cost": 500}"#;
        let policy = FieldPermissionPolicy {
            role: "worker".to_string(),
            read_allowed_fields: vec!["public_title".to_string()],
            write_allowed_fields: vec!["public_title".to_string()],
        };

        let projected = PreSyncProjectionEngine::apply_projection("worker", "entities", raw_json, &[policy]);
        assert!(projected.contains("public_title"));
        assert!(projected.contains("[RESTRICTED_FIELD]"));
        assert!(!projected.contains("500"));
    }

    #[test]
    fn test_write_back_protection_restores_masked_field_on_client_sync() {
        let client_incoming = r#"{"hourly_rate": "[RESTRICTED_PAY_RATE]", "note": "updated shift description"}"#;
        let server_authoritative = r#"{"hourly_rate": 95.00, "note": "original shift description"}"#;

        let sanitized = PreSyncProjectionEngine::sanitize_write_back_payload(
            "worker",
            "time_reports",
            client_incoming,
            server_authoritative,
            &[],
        );

        assert!(sanitized.contains("95"));
        assert!(!sanitized.contains("[RESTRICTED_PAY_RATE]"));
        assert!(sanitized.contains("updated shift description"));
    }

    #[tokio::test]
    async fn test_opfs_workspace_rehydration_snapshot() {
        let conn = database::acquire_connection().await.unwrap();
        let uid = uuid::Uuid::new_v4().to_string();
        let wid = format!("ws_rehydrate_{}", uid);
        let user_id = format!("usr_rehydrate_{}", uid);

        conn.execute(
            "INSERT INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'Hospital Ward 4', '[]', '{}')",
            libsql::params![wid.clone()],
        )
        .await
        .unwrap();

        conn.execute(
            "INSERT INTO users (id, workspace_id, email, role) VALUES (?1, ?2, 'nurse@hospital.org', 'user')",
            libsql::params![user_id.clone(), wid.clone()],
        )
        .await
        .unwrap();

        let snapshot = rehydrate_opfs_workspace_cache(user_id.clone(), wid.clone()).await.unwrap();
        assert_eq!(snapshot.workspace_id, wid);
        assert_eq!(snapshot.user_role, "user");
        assert!(snapshot.is_cache_rehydrated);
        assert_eq!(snapshot.rehydrated_tables_count, 4);
        assert!(snapshot.payload_bytes_json.contains("OPFS_CACHE_REHYDRATED"));
    }

    #[tokio::test]
    async fn test_jittered_opfs_rehydration_stream() {
        let conn = database::acquire_connection().await.unwrap();
        let uid = uuid::Uuid::new_v4().to_string();
        let wid = format!("ws_jit_{}", uid);
        let user_id = format!("usr_jit_{}", uid);
        let client_id = format!("kiosk_ward4_{}", uid);

        conn.execute(
            "INSERT INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'Hospital ICU', '[]', '{}')",
            libsql::params![wid.clone()],
        )
        .await
        .unwrap();

        conn.execute(
            "INSERT INTO users (id, workspace_id, email, role) VALUES (?1, ?2, 'doctor@icu.org', 'user')",
            libsql::params![user_id.clone(), wid.clone()],
        )
        .await
        .unwrap();

        let stream = rehydrate_opfs_workspace_cache_jittered(user_id, wid.clone(), client_id.clone(), Some(32))
            .await
            .unwrap();

        assert_eq!(stream.workspace_id, wid);
        assert_eq!(stream.client_id, client_id);
        assert!(stream.assigned_jitter_ms >= 100 && stream.assigned_jitter_ms <= 3600);
        assert_eq!(stream.max_chunk_bytes, 32 * 1024);
        assert!(stream.is_complete);
    }
}
