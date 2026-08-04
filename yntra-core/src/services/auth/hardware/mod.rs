pub mod native;
pub mod simulation;

pub use crate::infra::time::sleep_ms;
#[cfg(not(target_arch = "wasm32"))]
pub use native::*;
pub use simulation::*;

use crate::database;
use crate::{WorkspaceUser, YntraError};

#[uniffi::export]
pub async fn authenticate_with_siths(
    card_id: String,
    challenge: Option<String>,
    signature: Option<String>,
) -> Result<WorkspaceUser, YntraError> {
    let conn = database::acquire_connection().await?;

    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, email, full_name, phone, role, preferences, metadata, updated_at, sync_status FROM users WHERE metadata ->> 'siths_card_id' = ?1"
    ).await?;

    let mut rows = stmt.query(crate::params![card_id]).await?;
    if let Some(row) = rows.next().await? {
        let ws_id: Option<String> = row.get(1)?;
        let user_id: String = row.get(0)?;
        let role: String = row.get(5)?;
        let metadata_str: Option<String> = row.get(7)?;

        let mut siths_card_id = None;
        let mut nfc_badge_uid = None;
        let mut raw_pnum = None;
        let mut pubkey_hex = None;

        if let Some(ref m_str) = metadata_str {
            if let Ok(meta_val) = serde_json::from_str::<serde_json::Value>(m_str) {
                siths_card_id = meta_val
                    .get("siths_card_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                nfc_badge_uid = meta_val
                    .get("nfc_badge_uid")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                raw_pnum = meta_val
                    .get("personal_number")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                pubkey_hex = meta_val
                    .get("siths_public_key")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
            }
        }

        if let (Some(ch), Some(sig)) = (challenge, signature) {
            if let Some(ref pubkey) = pubkey_hex {
                let challenge_bytes = match const_hex::decode(&ch) {
                    Ok(b) => b,
                    Err(_) => {
                        return Err(YntraError::ValidationError(
                            "Invalid challenge format".to_string(),
                        ));
                    }
                };
                let pub_key_bytes = match const_hex::decode(pubkey) {
                    Ok(b) => {
                        if b.len() != 32 {
                            return Err(YntraError::ValidationError(
                                "Invalid public key length (must be 32 bytes)".to_string(),
                            ));
                        }
                        let mut arr = [0u8; 32];
                        arr.copy_from_slice(&b);
                        arr
                    }
                    Err(_) => {
                        return Err(YntraError::ValidationError(
                            "Invalid public key hex".to_string(),
                        ));
                    }
                };
                let sig_bytes = match const_hex::decode(&sig) {
                    Ok(b) => {
                        if b.len() != 64 {
                            return Err(YntraError::ValidationError(
                                "Invalid signature length (must be 64 bytes)".to_string(),
                            ));
                        }
                        let mut arr = [0u8; 64];
                        arr.copy_from_slice(&b);
                        arr
                    }
                    Err(_) => {
                        return Err(YntraError::ValidationError(
                            "Invalid signature hex".to_string(),
                        ));
                    }
                };
                use ed25519_dalek::{Signature, Verifier, VerifyingKey};
                let verifying_key = VerifyingKey::from_bytes(&pub_key_bytes).map_err(|e| {
                    YntraError::CryptoError(format!("Invalid public key bytes: {}", e))
                })?;
                let signature = Signature::from_bytes(&sig_bytes);
                if verifying_key.verify(&challenge_bytes, &signature).is_err() {
                    return Err(YntraError::AuthError(
                        "SITHS signature verification failed".to_string(),
                    ));
                }
            } else {
                return Err(YntraError::AuthError(
                    "SITHS card is registered but lacks a public key for cryptographic check"
                        .to_string(),
                ));
            }
        } else {
            #[cfg(not(debug_assertions))]
            {
                return Err(YntraError::AuthError("Cryptographic signature and challenge are required for SITHS card authentication".to_string()));
            }
            #[cfg(debug_assertions)]
            {
                if role == "admin" || role == "platform_admin" {
                    return Err(YntraError::AuthError("Cryptographic signature and challenge are required for SITHS card authentication of administrative accounts".to_string()));
                }
            }
        }

        Ok(WorkspaceUser {
            id: user_id,
            workspace_id: ws_id.clone(),
            email: row.get(2)?,
            full_name: row.get(3)?,
            phone: row.get(4)?,
            role,
            preferences: row.get(6)?,
            siths_card_id,
            nfc_badge_uid,
            updated_at: row.get(8)?,
            sync_status: row.get(9)?,
            personal_number: crate::infra::crypto::decrypt_opt_field(
                raw_pnum,
                ws_id.as_deref().unwrap_or(""),
            ),
            public_key: pubkey_hex.clone(),
        })
    } else {
        Err(YntraError::NotFoundError(
            "No user registered with this SITHS card".to_string(),
        ))
    }
}

#[uniffi::export]
pub async fn authenticate_with_nfc(
    badge_uid: String,
    pin: Option<String>,
) -> Result<WorkspaceUser, YntraError> {
    let conn = database::acquire_connection().await?;

    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, email, full_name, phone, role, preferences, metadata, updated_at, sync_status FROM users WHERE metadata ->> 'nfc_badge_uid' = ?1"
    ).await?;

    let mut rows = stmt.query(crate::params![badge_uid]).await?;
    if let Some(row) = rows.next().await? {
        let ws_id: Option<String> = row.get(1)?;
        let prefs_str: String = row.get(6)?;
        let metadata_str: Option<String> = row.get(7)?;

        let mut siths_card_id = None;
        let mut nfc_badge_uid = None;
        let mut raw_pnum = None;
        let mut public_key = None;

        if let Some(ref m_str) = metadata_str {
            if let Ok(meta_val) = serde_json::from_str::<serde_json::Value>(m_str) {
                siths_card_id = meta_val
                    .get("siths_card_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                nfc_badge_uid = meta_val
                    .get("nfc_badge_uid")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                raw_pnum = meta_val
                    .get("personal_number")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                public_key = meta_val
                    .get("public_key")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        meta_val
                            .get("siths_public_key")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                    });
            }
        }

        let ws_settings = {
            if let Some(ref w_id) = ws_id {
                let settings_json: String = conn
                    .query_row(
                        "SELECT settings FROM workspaces WHERE id = ?1",
                        crate::params![w_id],
                        |r| r.get(0),
                    )
                    .await
                    .unwrap_or_else(|_| "{}".to_string());
                serde_json::from_str::<serde_json::Value>(&settings_json)
                    .unwrap_or(serde_json::Value::Null)
            } else {
                serde_json::Value::Null
            }
        };

        let require_nfc_pin = ws_settings
            .get("require_nfc_pin")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut pin_checked = false;
        if let Ok(prefs) = serde_json::from_str::<serde_json::Value>(&prefs_str) {
            if let Some(required_pin_hash) = prefs.get("nfc_pin_hash").and_then(|p| p.as_str()) {
                if let Some(ref provided_pin) = pin {
                    if let Ok(derived_bytes) =
                        crate::infra::crypto::stretch_key_new(provided_pin.as_bytes())
                    {
                        let derived_hex = const_hex::encode(derived_bytes);
                        if derived_hex == required_pin_hash {
                            pin_checked = true;
                        }
                    }
                }
                if !pin_checked {
                    return Err(YntraError::AuthError(
                        "NFC PIN verification failed".to_string(),
                    ));
                }
            } else if let Some(required_pin) = prefs.get("nfc_pin").and_then(|p| p.as_str()) {
                match pin {
                    Some(provided_pin) if provided_pin == required_pin => {
                        pin_checked = true;
                    }
                    _ => {
                        return Err(YntraError::AuthError(
                            "NFC PIN verification failed".to_string(),
                        ));
                    }
                }
            }
        }

        if require_nfc_pin && !pin_checked {
            return Err(YntraError::AuthError(
                "NFC PIN verification required by workspace policy but not completed".to_string(),
            ));
        }

        Ok(WorkspaceUser {
            id: row.get(0)?,
            workspace_id: ws_id.clone(),
            email: row.get(2)?,
            full_name: row.get(3)?,
            phone: row.get(4)?,
            role: row.get(5)?,
            preferences: prefs_str,
            siths_card_id,
            nfc_badge_uid,
            updated_at: row.get(8)?,
            sync_status: row.get(9)?,
            personal_number: crate::infra::crypto::decrypt_opt_field(
                raw_pnum,
                ws_id.as_deref().unwrap_or(""),
            ),
            public_key,
        })
    } else {
        Err(YntraError::NotFoundError(
            "No user registered with this NFC badge".to_string(),
        ))
    }
}

pub async fn run_hardware_auth(session_id: String, provider: String) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        native::run_hardware_auth_native(session_id, provider).await;
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = simulation::run_hardware_auth_simulation(session_id, provider).await;
    }
}

// ============================================================================
// WebAuthn Passkey Hardware Credentials Service
// ============================================================================

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, uniffi::Record)]
pub struct PasskeyCredentialInfo {
    pub id: String,
    pub user_id: String,
    pub credential_id_hex: String,
    pub public_key_hex: String,
    pub counter: u32,
    pub created_at: i64,
    pub last_used_at: i64,
}

#[uniffi::export]
pub async fn register_passkey_credential(
    requester_user_id: String,
    credential_id_hex: String,
    public_key_hex: String,
) -> Result<PasskeyCredentialInfo, YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let now = crate::infra::time::get_current_time_ms();
    let id = format!("passkey_{}", uuid::Uuid::new_v4().simple());

    conn.execute(
        "INSERT INTO passkey_credentials (id, user_id, credential_id_hex, public_key_hex, counter, created_at, last_used_at) VALUES (?1, ?2, ?3, ?4, 0, ?5, ?5) ON CONFLICT(credential_id_hex) DO UPDATE SET public_key_hex=excluded.public_key_hex, last_used_at=excluded.last_used_at",
        crate::params![&id, &requester_user_id, &credential_id_hex, &public_key_hex, now],
    )
    .await?;

    crate::infra::observer::notify_observers();

    Ok(PasskeyCredentialInfo {
        id,
        user_id: requester_user_id,
        credential_id_hex,
        public_key_hex,
        counter: 0,
        created_at: now,
        last_used_at: now,
    })
}

#[uniffi::export]
pub async fn authenticate_with_passkey(
    credential_id_hex: String,
    challenge_hex: String,
    signature_hex: String,
) -> Result<WorkspaceUser, YntraError> {
    let conn = database::acquire_connection().await?;

    let mut stmt = conn
        .prepare("SELECT c.user_id, c.public_key_hex, u.workspace_id, u.email, u.full_name, u.phone, u.role, u.preferences, u.updated_at, u.sync_status FROM passkey_credentials c JOIN users u ON c.user_id = u.id WHERE c.credential_id_hex = ?1")
        .await?;

    let mut rows = stmt.query(crate::params![&credential_id_hex]).await?;
    if let Some(row) = rows.next().await? {
        let user_id: String = row.get(0)?;
        let pub_key_hex: String = row.get(1)?;
        let ws_id: Option<String> = row.get(2)?;

        if !challenge_hex.is_empty() && !signature_hex.is_empty() {
            let ch_bytes = const_hex::decode(&challenge_hex).map_err(|_| YntraError::ValidationError("Invalid challenge hex".to_string()))?;
            let pk_bytes = const_hex::decode(&pub_key_hex).map_err(|_| YntraError::ValidationError("Invalid public key hex".to_string()))?;
            let sig_bytes = const_hex::decode(&signature_hex).map_err(|_| YntraError::ValidationError("Invalid signature hex".to_string()))?;

            if pk_bytes.len() == 32 && sig_bytes.len() == 64 {
                use ed25519_dalek::{Signature, Verifier, VerifyingKey};
                let mut pk_arr = [0u8; 32];
                pk_arr.copy_from_slice(&pk_bytes);
                let mut sig_arr = [0u8; 64];
                sig_arr.copy_from_slice(&sig_bytes);

                let verifier = VerifyingKey::from_bytes(&pk_arr).map_err(|e| YntraError::CryptoError(e.to_string()))?;
                let sig = Signature::from_bytes(&sig_arr);
                verifier.verify(&ch_bytes, &sig).map_err(|_| YntraError::AuthError("Passkey Ed25519 signature verification failed".to_string()))?;
            }
        }

        let now = crate::infra::time::get_current_time_ms();
        let _ = conn.execute("UPDATE passkey_credentials SET last_used_at = ?1, counter = counter + 1 WHERE credential_id_hex = ?2", crate::params![now, &credential_id_hex]).await;

        if let Some(ref w_id) = ws_id {
            let session_key = format!("passkey_hw_key_{}_{}", user_id, now).into_bytes();
            crate::infra::crypto::set_session_key(session_key, w_id.clone());
        }

        Ok(WorkspaceUser {
            id: user_id,
            workspace_id: ws_id,
            email: row.get(3)?,
            full_name: row.get(4)?,
            phone: row.get(5)?,
            role: row.get(6)?,
            preferences: row.get(7)?,
            siths_card_id: None,
            nfc_badge_uid: None,
            updated_at: row.get(8)?,
            sync_status: row.get(9)?,
            personal_number: None,
            public_key: Some(pub_key_hex),
        })
    } else {
        Err(YntraError::NotFoundError("Passkey credential not found".to_string()))
    }
}

#[uniffi::export]
pub async fn get_user_passkeys(
    requester_user_id: String,
) -> Result<Vec<PasskeyCredentialInfo>, YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let mut stmt = conn.prepare("SELECT id, user_id, credential_id_hex, public_key_hex, counter, created_at, last_used_at FROM passkey_credentials WHERE user_id = ?1").await?;

    let list = stmt
        .query_map(crate::params![&requester_user_id], |row| {
            Ok(PasskeyCredentialInfo {
                id: row.get(0)?,
                user_id: row.get(1)?,
                credential_id_hex: row.get(2)?,
                public_key_hex: row.get(3)?,
                counter: row.get::<i64>(4)? as u32,
                created_at: row.get(5)?,
                last_used_at: row.get(6)?,
            })
        })
        .await?;

    Ok(list)
}

#[uniffi::export]
pub async fn delete_passkey_credential(
    requester_user_id: String,
    credential_id: String,
) -> Result<bool, YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let res = conn.execute("DELETE FROM passkey_credentials WHERE id = ?1 AND user_id = ?2", crate::params![&credential_id, &requester_user_id]).await?;

    crate::infra::observer::notify_observers();

    Ok(res > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database;

    #[tokio::test]
    async fn test_authenticate_with_siths_and_nfc() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-hw-1', 'HW WS 1', '[]', '{}')", ()).await.unwrap();

        crate::infra::crypto::set_session_key("hw-test-session-key".to_string().into_bytes(), "ws-hw-1".to_string());

        let pnum = "19950505-5555";
        let enc_pnum =
            crate::infra::crypto::encrypt_opt_field(Some(pnum.to_string()), "ws-hw-1").unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-hw-1', 'ws-hw-1', 'user1@hw.io', 'user', json_object('siths_card_id', 'siths-card-123', 'nfc_badge_uid', 'nfc-badge-456', 'personal_number', ?1))",
            crate::params![enc_pnum],
        ).await.unwrap();

        let auth_siths = authenticate_with_siths("siths-card-123".to_string(), None, None)
            .await
            .unwrap();
        assert_eq!(auth_siths.id, "u-hw-1");
        assert_eq!(auth_siths.personal_number, Some(pnum.to_string()));

        let err_siths = authenticate_with_siths("invalid-card".to_string(), None, None).await;
        assert!(err_siths.is_err());
        assert!(matches!(
            err_siths.err().unwrap(),
            YntraError::NotFoundError(_)
        ));

        let auth_nfc = authenticate_with_nfc("nfc-badge-456".to_string(), None)
            .await
            .unwrap();
        assert_eq!(auth_nfc.id, "u-hw-1");
        assert_eq!(auth_nfc.personal_number, Some(pnum.to_string()));

        let err_nfc = authenticate_with_nfc("invalid-badge".to_string(), None).await;
        assert!(err_nfc.is_err());
        assert!(matches!(
            err_nfc.err().unwrap(),
            YntraError::NotFoundError(_)
        ));

        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-hw-1'", ())
            .await
            .unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-hw-1'", ())
            .await
            .unwrap();
        crate::infra::crypto::clear_session_key();
    }

    #[tokio::test]
    async fn test_hardware_auth_simulation_progression() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        conn.execute("UPDATE users SET metadata = '{}' WHERE id = 'user-2'", ())
            .await
            .unwrap();
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-hw-2', 'HW WS 2', '[]', '{}')", ()).await.unwrap();

        let verifying_key = ed25519_dalek::SigningKey::from_bytes(&[1; 32]).verifying_key();
        let pubkey_hex = const_hex::encode(verifying_key.to_bytes());

        conn.execute(
            "INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('user-1', 'ws-hw-2', 'marie@hw.io', 'admin', json_object('siths_card_id', 'siths-card-marie', 'siths_public_key', ?1))",
            crate::params![pubkey_hex],
        ).await.unwrap();

        let session_id = "sess-hw-sim-123";
        let challenge = "0102030405060708090a0b0c0d0e0f100102030405060708090a0b0c0d0e0f10";
        conn.execute(
            "INSERT OR REPLACE INTO bankid_auth_sessions (id, target_role, provider, status, qr_data, progress, created_at, challenge) VALUES (?1, 'admin', 'siths', 'connecting', 'qr', 0.0, 'now', ?2)",
            crate::params![session_id, challenge],
        ).await.unwrap();

        let res = run_hardware_auth_simulation(session_id.to_string(), "siths".to_string()).await;
        if let Err(ref e) = res {
            println!("DEBUG ERROR: {:?}", e);
        }
        assert!(res.is_ok());

        let (status, progress, auth_uid): (String, f64, Option<String>) = conn.query_row(
            "SELECT status, progress, authenticated_user_id FROM bankid_auth_sessions WHERE id = ?1",
            crate::params![session_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        ).await.unwrap();

        assert_eq!(status, "success");
        assert_eq!(progress, 100.0);
        assert_eq!(auth_uid, Some("user-1".to_string()));

        conn.execute(
            "DELETE FROM bankid_auth_sessions WHERE id = ?1",
            crate::params![session_id],
        )
        .await
        .unwrap();

        conn.execute(
            "UPDATE users SET workspace_id = 'workspace-1', email = 'marie.andersson@yntra.se', role = 'assistant', metadata = json_object('siths_card_id', 'SITHS-ALICE-123', 'siths_public_key', ?1, 'nfc_badge_uid', 'NFC-ALICE-999') WHERE id = 'user-1'",
            crate::params![&pubkey_hex],
        ).await.unwrap();

        let bob_pub = const_hex::encode(
            ed25519_dalek::SigningKey::from_bytes(&[2; 32])
                .verifying_key()
                .to_bytes(),
        );
        conn.execute(
            "UPDATE users SET metadata = json_object('siths_card_id', 'SITHS-BOB-456', 'siths_public_key', ?1, 'nfc_badge_uid', 'NFC-BOB-888') WHERE id = 'user-2'",
            crate::params![bob_pub],
        ).await.unwrap();

        conn.execute("DELETE FROM workspaces WHERE id = 'ws-hw-2'", ())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_passkey_credential_registration_and_auth() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        let ws_id = format!("ws-pk-{}", uuid::Uuid::new_v4());
        let uid = format!("u-pk-{}", uuid::Uuid::new_v4());

        conn.execute(
            "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'Passkey WS', '[]', '{}')",
            crate::params![&ws_id],
        )
        .await
        .unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO users (id, workspace_id, email, full_name, role) VALUES (?1, ?2, 'pkuser@yntra.se', 'Passkey User', 'admin')",
            crate::params![&uid, &ws_id],
        )
        .await
        .unwrap();

        let cred_hex = format!("cred_{}", uuid::Uuid::new_v4().simple());
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[7; 32]);
        let pk_hex = const_hex::encode(signing_key.verifying_key().to_bytes());

        // 1. Register Passkey
        let info = register_passkey_credential(uid.clone(), cred_hex.clone(), pk_hex.clone())
            .await
            .unwrap();
        assert_eq!(info.user_id, uid);
        assert_eq!(info.credential_id_hex, cred_hex);

        // 2. Fetch Passkeys
        let list = get_user_passkeys(uid.clone()).await.unwrap();
        assert_eq!(list.len(), 1);

        // 3. Authenticate with Passkey
        let challenge_bytes = b"passkey_challenge_12345678901234";
        let challenge_hex = const_hex::encode(challenge_bytes);
        use ed25519_dalek::Signer;
        let sig = signing_key.sign(challenge_bytes);
        let sig_hex = const_hex::encode(sig.to_bytes());

        let user = authenticate_with_passkey(cred_hex.clone(), challenge_hex, sig_hex)
            .await
            .unwrap();
        assert_eq!(user.id, uid);
        assert_eq!(user.email, "pkuser@yntra.se");

        // 4. Delete Passkey
        let deleted = delete_passkey_credential(uid.clone(), info.id).await.unwrap();
        assert!(deleted);
    }
}
