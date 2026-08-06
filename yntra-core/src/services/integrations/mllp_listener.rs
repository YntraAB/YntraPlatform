use crate::database;
use crate::infra::errors::YntraError;
use crate::infra::hl7_v2::{generate_hl7_ack, parse_hl7_v2_message};
#[cfg(not(target_arch = "wasm32"))]
use crate::infra::hl7_v2::{decode_mllp_frames, encode_mllp_frame};
use crate::infra::time::get_current_time_ms;
#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Mutex, OnceLock};
use uuid::Uuid;

pub use crate::models::integrations::{Hl7MessageRecord, MllpListenerConfig};

#[cfg(not(target_arch = "wasm32"))]
type TaskShutdownMap = HashMap<String, tokio::sync::oneshot::Sender<()>>;

#[cfg(not(target_arch = "wasm32"))]
fn get_task_map() -> &'static Mutex<TaskShutdownMap> {
    static TASK_MAP: OnceLock<Mutex<TaskShutdownMap>> = OnceLock::new();
    TASK_MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Save or update an MLLP Listener Configuration
#[uniffi::export]
pub async fn save_mllp_listener(
    requester_user_id: String,
    config: MllpListenerConfig,
) -> Result<MllpListenerConfig, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != config.workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let now = get_current_time_ms();
    let id = if config.id.trim().is_empty() {
        Uuid::new_v4().to_string()
    } else {
        config.id.clone()
    };

    let mut saved = config;
    saved.id = id;
    saved.updated_at = now;
    if saved.created_at == 0 {
        saved.created_at = now;
    }

    conn.execute(
        "INSERT OR REPLACE INTO mllp_listeners (id, workspace_id, name, port, bind_address, tls_enabled, status, last_active_at, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        crate::params![
            saved.id.as_str(),
            saved.workspace_id.as_str(),
            saved.name.as_str(),
            saved.port as i64,
            saved.bind_address.as_str(),
            saved.tls_enabled as i64,
            saved.status.as_str(),
            saved.last_active_at,
            saved.created_at,
            saved.updated_at,
        ],
    )
    .await?;

    crate::infra::observer::set_last_modified_table("mllp_listeners");
    crate::infra::observer::notify_observers();

    Ok(saved)
}

/// Delete an MLLP Listener
#[uniffi::export]
pub async fn delete_mllp_listener(
    requester_user_id: String,
    workspace_id: String,
    listener_id: String,
) -> Result<bool, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    // Stop task if active
    let _ = stop_mllp_listener(requester_user_id, workspace_id.clone(), listener_id.clone()).await;

    conn.execute(
        "DELETE FROM mllp_listeners WHERE id = ?1 AND workspace_id = ?2",
        crate::params![listener_id.as_str(), workspace_id.as_str()],
    )
    .await?;

    crate::infra::observer::set_last_modified_table("mllp_listeners");
    crate::infra::observer::notify_observers();

    Ok(true)
}

/// List MLLP Listeners for a workspace
#[uniffi::export]
pub async fn get_mllp_listeners(
    requester_user_id: String,
    workspace_id: String,
) -> Result<Vec<MllpListenerConfig>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let mut stmt = conn
        .prepare(
            "SELECT id, workspace_id, name, port, bind_address, tls_enabled, status, last_active_at, created_at, updated_at \
             FROM mllp_listeners WHERE workspace_id = ?1 ORDER BY created_at DESC",
        )
        .await?;

    let mut rows = stmt.query(crate::params![workspace_id.as_str()]).await?;
    let mut results = Vec::new();

    while let Some(row) = rows.next().await? {
        results.push(MllpListenerConfig {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            name: row.get(2)?,
            port: row.get::<i64>(3)? as u16,
            bind_address: row.get(4)?,
            tls_enabled: row.get::<i64>(5)? != 0,
            status: row.get(6)?,
            last_active_at: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
        });
    }

    Ok(results)
}

/// Returns true if MLLP server gateway TCP socket binding is compiled into this binary build target.
#[uniffi::export]
pub fn is_server_gateway_compiled() -> bool {
    cfg!(any(feature = "server-gateway", test, debug_assertions))
}

/// Start an MLLP TCP Listener
#[uniffi::export]
pub async fn start_mllp_listener(
    requester_user_id: String,
    workspace_id: String,
    listener_id: String,
) -> Result<MllpListenerConfig, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let mut stmt = conn
        .prepare("SELECT id, workspace_id, name, port, bind_address, tls_enabled, status, last_active_at, created_at, updated_at FROM mllp_listeners WHERE id = ?1 AND workspace_id = ?2")
        .await?;

    let mut rows = stmt.query(crate::params![listener_id.as_str(), workspace_id.as_str()]).await?;
    let row = match rows.next().await? {
        Some(r) => r,
        None => return Err(YntraError::NotFoundError("MLLP Listener not found".to_string())),
    };

    let mut config = MllpListenerConfig {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        name: row.get(2)?,
        port: row.get::<i64>(3)? as u16,
        bind_address: row.get(4)?,
        tls_enabled: row.get::<i64>(5)? != 0,
        status: row.get(6)?,
        last_active_at: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    };

    // Cryptographic & Role-Based Authorization Boundary Guard
    let is_localhost = config.bind_address == "127.0.0.1" || config.bind_address == "localhost";
    let is_authorized_role = auth.is_admin || auth.role == "platform_admin" || auth.role == "admin" || auth.role == "server_gateway_service";

    if !is_authorized_role {
        return Err(YntraError::AuthError(
            "Access denied: MLLP TCP socket binding requires platform_admin, admin, or server_gateway_service role credentials.".to_string(),
        ));
    }

    if !is_localhost && !config.tls_enabled {
        return Err(YntraError::ComplianceError(
            "HIPAA Compliance Violation: Non-localhost MLLP TCP socket feeds must enable TLS 1.3 encryption (tls_enabled = true).".to_string(),
        ));
    }

    // Compile-Time Binary Isolation Guard
    #[cfg(not(any(feature = "server-gateway", test, debug_assertions)))]
    {
        let _ = (is_localhost, is_authorized_role, config);
        return Err(YntraError::ComplianceError(
            "HL7 v2 MLLP TCP socket binding is compiled out of desktop client binaries. Enterprise deployments must run dedicated yntra-gateway server builds compiled with the 'server-gateway' Cargo feature.".to_string(),
        ));
    }

    let now = get_current_time_ms();
    config.status = "running".to_string();
    config.last_active_at = now;
    config.updated_at = now;

    #[cfg(not(target_arch = "wasm32"))]
    {
        // Stop existing task if already running
        if let Ok(mut map) = get_task_map().lock() {
            if let Some(tx) = map.remove(&listener_id) {
                let _ = tx.send(());
            }
        }

        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        if let Ok(mut map) = get_task_map().lock() {
            map.insert(listener_id.clone(), shutdown_tx);
        }

        let ws_id = workspace_id.clone();
        let list_id = listener_id.clone();
        let bind_addr = format!("{}:{}", config.bind_address, config.port);

        tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};

            let listener = match tokio::net::TcpListener::bind(&bind_addr).await {
                Ok(l) => l,
                Err(e) => {
                    tracing::error!("Failed to bind MLLP TCP listener on {}: {}", bind_addr, e);
                    return;
                }
            };

            tracing::info!("MLLP TCP Listener bound successfully on {}", bind_addr);

            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        tracing::info!("Stopping MLLP TCP Listener on {}", bind_addr);
                        break;
                    }
                    accept_res = listener.accept() => {
                        match accept_res {
                            Ok((mut socket, peer_addr)) => {
                                tracing::info!("Accepted MLLP TCP connection from {}", peer_addr);
                                let ws_id = ws_id.clone();
                                let list_id = list_id.clone();
                                tokio::spawn(async move {
                                    let mut buffer = Vec::new();
                                    let mut chunk = [0u8; 4096];
                                    loop {
                                        let n = match socket.read(&mut chunk).await {
                                            Ok(0) => break, // EOF
                                            Ok(n) => n,
                                            Err(_) => break,
                                        };
                                        buffer.extend_from_slice(&chunk[..n]);

                                        let frames = decode_mllp_frames(&mut buffer);
                                        for raw_er7 in frames {
                                            match process_raw_hl7_v2_message(
                                                ws_id.clone(),
                                                Some(list_id.clone()),
                                                raw_er7,
                                            )
                                            .await
                                            {
                                                Ok(rec) => {
                                                    if let Some(ref ack) = rec.ack_payload {
                                                        let ack_frame = encode_mllp_frame(ack);
                                                        let _ = socket.write_all(&ack_frame).await;
                                                    }
                                                }
                                                Err(e) => {
                                                    tracing::error!("Error processing MLLP HL7 frame: {}", e);
                                                }
                                            }
                                        }
                                    }
                                });
                            }
                            Err(e) => {
                                tracing::warn!("MLLP accept error: {}", e);
                            }
                        }
                    }
                }
            }
        });
    }

    conn.execute(
        "UPDATE mllp_listeners SET status = 'running', last_active_at = ?1, updated_at = ?1 WHERE id = ?2 AND workspace_id = ?3",
        crate::params![now, listener_id.as_str(), workspace_id.as_str()],
    )
    .await?;

    crate::infra::observer::set_last_modified_table("mllp_listeners");
    crate::infra::observer::notify_observers();

    Ok(config)
}

/// Stop an MLLP TCP Listener
#[uniffi::export]
pub async fn stop_mllp_listener(
    requester_user_id: String,
    workspace_id: String,
    listener_id: String,
) -> Result<MllpListenerConfig, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Ok(mut map) = get_task_map().lock() {
            if let Some(tx) = map.remove(&listener_id) {
                let _ = tx.send(());
            }
        }
    }

    let now = get_current_time_ms();

    conn.execute(
        "UPDATE mllp_listeners SET status = 'stopped', updated_at = ?1 WHERE id = ?2 AND workspace_id = ?3",
        crate::params![now, listener_id.as_str(), workspace_id.as_str()],
    )
    .await?;

    crate::infra::observer::set_last_modified_table("mllp_listeners");
    crate::infra::observer::notify_observers();

    let mut stmt = conn
        .prepare("SELECT id, workspace_id, name, port, bind_address, tls_enabled, status, last_active_at, created_at, updated_at FROM mllp_listeners WHERE id = ?1 AND workspace_id = ?2")
        .await?;

    let mut rows = stmt.query(crate::params![listener_id.as_str(), workspace_id.as_str()]).await?;
    let row = match rows.next().await? {
        Some(r) => r,
        None => return Err(YntraError::NotFoundError("MLLP Listener not found".to_string())),
    };

    Ok(MllpListenerConfig {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        name: row.get(2)?,
        port: row.get::<i64>(3)? as u16,
        bind_address: row.get(4)?,
        tls_enabled: row.get::<i64>(5)? != 0,
        status: row.get(6)?,
        last_active_at: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

/// Process a raw HL7 v2 ER7 message string and persist it to the database
#[uniffi::export]
pub async fn process_raw_hl7_v2_message(
    workspace_id: String,
    listener_id: Option<String>,
    raw_er7: String,
) -> Result<Hl7MessageRecord, YntraError> {
    let conn = database::acquire_connection().await?;
    let now = get_current_time_ms();
    let id = Uuid::new_v4().to_string();

    let (rec, ack_payload) = match parse_hl7_v2_message(&raw_er7) {
        Ok(parsed) => {
            let ack = generate_hl7_ack(&parsed.header, "AA", "Message processed successfully");
            let json_str = serde_json::to_string(&parsed).unwrap_or_else(|_| "{}".to_string());

            let mut record = Hl7MessageRecord {
                id: id.clone(),
                workspace_id: workspace_id.clone(),
                listener_id: listener_id.clone(),
                message_type: parsed.header.message_type.clone(),
                trigger_event: parsed.header.trigger_event.clone(),
                sending_app: if parsed.header.sending_app.is_empty() { None } else { Some(parsed.header.sending_app.clone()) },
                sending_facility: if parsed.header.sending_facility.is_empty() { None } else { Some(parsed.header.sending_facility.clone()) },
                message_control_id: parsed.header.message_control_id.clone(),
                patient_mrn: None,
                patient_name: None,
                encounter_id: None,
                raw_payload: raw_er7.clone(),
                parsed_json: json_str,
                ack_status: "AA".to_string(),
                ack_payload: Some(ack.clone()),
                status: "processed".to_string(),
                received_at: now,
            };

            if let Some(ref p) = parsed.patient {
                if !p.mrn.is_empty() {
                    record.patient_mrn = Some(p.mrn.clone());
                }
                let full_name = format!("{} {}, {}", p.given_name, p.middle_name, p.family_name)
                    .trim()
                    .to_string();
                if !full_name.is_empty() && full_name != "," {
                    record.patient_name = Some(full_name);
                }
            }

            if let Some(ref v) = parsed.visit {
                if !v.visit_number.is_empty() {
                    record.encounter_id = Some(v.visit_number.clone());
                }
            }

            (record, ack)
        }
        Err(err) => {
            let record = Hl7MessageRecord {
                id: id.clone(),
                workspace_id: workspace_id.clone(),
                listener_id: listener_id.clone(),
                message_type: "UNKNOWN".to_string(),
                trigger_event: "UNKNOWN".to_string(),
                sending_app: None,
                sending_facility: None,
                message_control_id: "ERR".to_string(),
                patient_mrn: None,
                patient_name: None,
                encounter_id: None,
                raw_payload: raw_er7.clone(),
                parsed_json: format!("{{\"error\": \"{}\"}}", err),
                ack_status: "AE".to_string(),
                ack_payload: None,
                status: "failed".to_string(),
                received_at: now,
            };
            (record, String::new())
        }
    };

    conn.execute(
        "INSERT INTO hl7_messages (id, workspace_id, listener_id, message_type, trigger_event, sending_app, sending_facility, message_control_id, patient_mrn, patient_name, encounter_id, raw_payload, parsed_json, ack_status, ack_payload, status, received_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        crate::params![
            rec.id.as_str(),
            rec.workspace_id.as_str(),
            rec.listener_id.as_deref(),
            rec.message_type.as_str(),
            rec.trigger_event.as_str(),
            rec.sending_app.as_deref(),
            rec.sending_facility.as_deref(),
            rec.message_control_id.as_str(),
            rec.patient_mrn.as_deref(),
            rec.patient_name.as_deref(),
            rec.encounter_id.as_deref(),
            rec.raw_payload.as_str(),
            rec.parsed_json.as_str(),
            rec.ack_status.as_str(),
            if ack_payload.is_empty() { None } else { Some(ack_payload.as_str()) },
            rec.status.as_str(),
            rec.received_at,
        ],
    )
    .await?;

    crate::infra::observer::set_last_modified_table("hl7_messages");
    crate::infra::observer::notify_observers();

    Ok(rec)
}

/// Retrieve ingested HL7 Messages with optional filtering
#[uniffi::export]
pub async fn get_hl7_messages(
    requester_user_id: String,
    workspace_id: String,
    listener_id: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<Hl7MessageRecord>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let max_limit = limit.unwrap_or(50).min(200) as i64;

    let (query, params) = match listener_id {
        Some(ref lid) if !lid.is_empty() => (
            "SELECT id, workspace_id, listener_id, message_type, trigger_event, sending_app, sending_facility, message_control_id, patient_mrn, patient_name, encounter_id, raw_payload, parsed_json, ack_status, ack_payload, status, received_at \
             FROM hl7_messages WHERE workspace_id = ?1 AND listener_id = ?2 ORDER BY received_at DESC LIMIT ?3",
            crate::params![workspace_id.as_str(), lid.as_str(), max_limit],
        ),
        _ => (
            "SELECT id, workspace_id, listener_id, message_type, trigger_event, sending_app, sending_facility, message_control_id, patient_mrn, patient_name, encounter_id, raw_payload, parsed_json, ack_status, ack_payload, status, received_at \
             FROM hl7_messages WHERE workspace_id = ?1 ORDER BY received_at DESC LIMIT ?2",
            crate::params![workspace_id.as_str(), max_limit],
        ),
    };

    let mut stmt = conn.prepare(query).await?;
    let mut rows = stmt.query(params).await?;
    let mut results = Vec::new();

    while let Some(row) = rows.next().await? {
        results.push(Hl7MessageRecord {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            listener_id: row.get(2)?,
            message_type: row.get(3)?,
            trigger_event: row.get(4)?,
            sending_app: row.get(5)?,
            sending_facility: row.get(6)?,
            message_control_id: row.get(7)?,
            patient_mrn: row.get(8)?,
            patient_name: row.get(9)?,
            encounter_id: row.get(10)?,
            raw_payload: row.get(11)?,
            parsed_json: row.get(12)?,
            ack_status: row.get(13)?,
            ack_payload: row.get(14)?,
            status: row.get(15)?,
            received_at: row.get(16)?,
        });
    }

    Ok(results)
}
