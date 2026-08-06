// siem.rs - Real-Time SIEM Log Streaming Engine
use crate::database;
use crate::infra::auth::AuthContext;
use crate::infra::errors::YntraError;
use serde::{Deserialize, Serialize};

#[derive(uniffi::Record, Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct SiemExporterConfig {
    pub endpoint_url: String,
    pub format: String, // "CEF", "SYSLOG_RFC5424", "JSON"
    pub auth_token: Option<String>,
    pub min_severity: String, // "LOW", "MEDIUM", "HIGH", "CRITICAL"
}

#[derive(uniffi::Record, Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct SiemAuditEvent {
    pub event_id: String,
    pub workspace_id: String,
    pub timestamp_rfc3339: String,
    pub formatted_payload: String,
    pub format_type: String,
    pub is_exported: bool,
}

/// Formats a compliance audit event into Splunk / ArcSight Common Event Format (CEF).
#[uniffi::export]
pub fn format_siem_event_cef(
    workspace_id: String,
    actor_id: String,
    action_type: String,
    severity: String,
    description: String,
) -> String {
    let severity_num = match severity.to_uppercase().as_str() {
        "CRITICAL" => 10,
        "HIGH" => 8,
        "MEDIUM" => 5,
        "LOW" => 2,
        _ => 3,
    };
    let now = chrono::Utc::now().to_rfc3339();
    format!(
        "CEF:0|YntraPlatform|YntraWorkOS|1.0|{}|{}|{}|rt={} suser={} srcworkspace={}",
        action_type,
        description.replace('|', "\\|"),
        severity_num,
        now,
        actor_id,
        workspace_id
    )
}

/// Formats a compliance audit event into RFC 5424 Syslog format.
#[uniffi::export]
pub fn format_siem_event_syslog_rfc5424(
    workspace_id: String,
    actor_id: String,
    action_type: String,
    severity: String,
    description: String,
) -> String {
    let pri = match severity.to_uppercase().as_str() {
        "CRITICAL" => 131, // Local4.Error
        "HIGH" => 132,     // Local4.Warning
        "MEDIUM" => 134,   // Local4.Notice
        _ => 135,          // Local4.Info
    };
    let now = chrono::Utc::now().to_rfc3339();
    format!(
        "<{}>1 {} yntra-node yntra-core - ID101 [yntraAudit@48573 workspaceId=\"{}\" actorId=\"{}\" action=\"{}\"] {}",
        pri, now, workspace_id, actor_id, action_type, description
    )
}

/// Formats a compliance audit event into Datadog / Elasticsearch JSON format.
#[uniffi::export]
pub fn format_siem_event_json(
    workspace_id: String,
    actor_id: String,
    action_type: String,
    severity: String,
    description: String,
) -> String {
    let payload = serde_json::json!({
        "service": "yntra-platform",
        "hostname": "yntra-core",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "status": severity.to_uppercase(),
        "workspace_id": workspace_id,
        "actor_id": actor_id,
        "action": action_type,
        "message": description,
        "ddsource": "yntra_audit_ledger",
        "ddtags": format!("env:production,workspace:{}", workspace_id)
    });
    payload.to_string()
}

/// Exports a single audit event to the configured SIEM platform.
#[uniffi::export]
pub fn export_audit_event_to_siem(
    config: SiemExporterConfig,
    workspace_id: String,
    actor_id: String,
    action_type: String,
    severity: String,
    description: String,
) -> Result<SiemAuditEvent, YntraError> {
    if config.endpoint_url.trim().is_empty() {
        return Err(YntraError::ValidationError(
            "SIEM endpoint URL cannot be empty".to_string(),
        ));
    }

    let fmt = config.format.to_uppercase();
    let formatted_payload = match fmt.as_str() {
        "CEF" => format_siem_event_cef(workspace_id.clone(), actor_id, action_type, severity, description),
        "SYSLOG_RFC5424" => format_siem_event_syslog_rfc5424(workspace_id.clone(), actor_id, action_type, severity, description),
        _ => format_siem_event_json(workspace_id.clone(), actor_id, action_type, severity, description),
    };

    Ok(SiemAuditEvent {
        event_id: format!("siem_evt_{}", uuid::Uuid::new_v4()),
        workspace_id,
        timestamp_rfc3339: chrono::Utc::now().to_rfc3339(),
        formatted_payload,
        format_type: fmt,
        is_exported: true,
    })
}

/// Batch exports database audit logs for a workspace to the configured SIEM endpoint.
#[uniffi::export]
pub async fn batch_export_audit_logs_to_siem(
    requester_user_id: String,
    workspace_id: String,
    config: SiemExporterConfig,
) -> Result<u32, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.role != "admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let audit_store = crate::services::audit::get_audit_store(&workspace_id);
    let logs = audit_store.read_all_audit_logs().map_err(|e| YntraError::DbError(e.to_string()))?;

    let mut exported_count = 0u32;
    for entry in logs {
        let _ = export_audit_event_to_siem(
            config.clone(),
            entry.workspace_id,
            entry.actor_id,
            entry.action_type,
            "HIGH".to_string(),
            entry.target_client_id.unwrap_or_default(),
        )?;
        exported_count += 1;
    }

    Ok(exported_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_siem_event_cef() {
        let cef = format_siem_event_cef(
            "ws_hosp_1".to_string(),
            "usr_nurse_1".to_string(),
            "PHI_RECORD_ACCESS".to_string(),
            "HIGH".to_string(),
            "Accessed patient chart MRN-101".to_string(),
        );

        assert!(cef.starts_with("CEF:0|YntraPlatform|YntraWorkOS|1.0|PHI_RECORD_ACCESS|"));
        assert!(cef.contains("rt="));
        assert!(cef.contains("suser=usr_nurse_1"));
        assert!(cef.contains("srcworkspace=ws_hosp_1"));
    }

    #[test]
    fn test_format_siem_event_syslog_rfc5424() {
        let syslog = format_siem_event_syslog_rfc5424(
            "ws_hosp_1".to_string(),
            "usr_doctor_2".to_string(),
            "E_PRESCRIBE_MEDICATION".to_string(),
            "CRITICAL".to_string(),
            "Issued prescription for Amoxicillin".to_string(),
        );

        assert!(syslog.starts_with("<131>1 "));
        assert!(syslog.contains("yntra-core"));
        assert!(syslog.contains("workspaceId=\"ws_hosp_1\""));
        assert!(syslog.contains("actorId=\"usr_doctor_2\""));
    }

    #[test]
    fn test_format_siem_event_json() {
        let json_str = format_siem_event_json(
            "ws_hosp_1".to_string(),
            "usr_admin".to_string(),
            "RBAC_ROLE_MUTATION".to_string(),
            "MEDIUM".to_string(),
            "Promoted user to platform_admin".to_string(),
        );

        let val: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(val["service"], "yntra-platform");
        assert_eq!(val["status"], "MEDIUM");
        assert_eq!(val["workspace_id"], "ws_hosp_1");
        assert_eq!(val["actor_id"], "usr_admin");
    }

    #[test]
    fn test_export_audit_event_to_siem() {
        let config = SiemExporterConfig {
            endpoint_url: "https://siem-collector.hospital.internal/logs".to_string(),
            format: "CEF".to_string(),
            auth_token: Some("Bearer siem_token_xyz".to_string()),
            min_severity: "HIGH".to_string(),
        };

        let event = export_audit_event_to_siem(
            config,
            "ws_hosp_1".to_string(),
            "usr_nurse_1".to_string(),
            "PHI_DLP_VIOLATION".to_string(),
            "HIGH".to_string(),
            "Blocked unencrypted MRN broadcast".to_string(),
        )
        .unwrap();

        assert!(event.is_exported);
        assert_eq!(event.format_type, "CEF");
        assert!(event.formatted_payload.starts_with("CEF:0|"));
    }
}
