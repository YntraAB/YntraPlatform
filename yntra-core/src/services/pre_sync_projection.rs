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
}
