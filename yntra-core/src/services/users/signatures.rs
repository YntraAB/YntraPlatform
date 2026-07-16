use crate::YntraError;
use crate::database;

pub async fn ensure_user_role_signature(
    conn: &database::DbConnection,
    user_id: &str,
    role: &str,
    workspace_id: &str,
) -> Result<(), YntraError> {
    let mut cached_pk = None;
    let mut cached_sk = None;
    ensure_user_role_signature_impl(
        conn,
        user_id,
        role,
        workspace_id,
        &mut cached_pk,
        &mut cached_sk,
    )
    .await
}

pub async fn ensure_user_role_signature_impl(
    conn: &database::DbConnection,
    user_id: &str,
    role: &str,
    workspace_id: &str,
    cached_pk: &mut Option<String>,
    cached_sk: &mut Option<String>,
) -> Result<(), YntraError> {
    let needs_signature = role != "anonymous" && role != "deleted";
    if !needs_signature {
        let current_sig: Option<String> = conn
            .query_row(
                "SELECT role_signature FROM users WHERE id = ?1",
                crate::params![user_id],
                |r| r.get(0),
            )
            .await
            .ok()
            .flatten();
        if current_sig.is_some() {
            let now_ms = crate::infra::time::get_current_time_ms();
            conn.execute(
                "UPDATE users SET role_signature = NULL, updated_at = ?1, sync_status = 'pending' WHERE id = ?2",
                crate::params![now_ms, user_id],
            ).await?;
        }
        return Ok(());
    }

    // 1. Check if workspace already has a public key configured (with caching)
    if cached_pk.is_none() {
        let creator_pk: Option<String> = conn
            .query_row(
                "SELECT creator_public_key FROM workspaces WHERE id = ?1",
                crate::params![workspace_id],
                |r| Ok(r.get(0)?),
            )
            .await
            .ok()
            .flatten();
        *cached_pk = Some(creator_pk.unwrap_or_default());
    }

    let private_key_setting = format!("creator_private_key_{}", workspace_id);
    if cached_sk.is_none() {
        let creator_sk: Option<String> =
            crate::infra::crypto::get_local_secret(&private_key_setting).await?;
        *cached_sk = Some(creator_sk.unwrap_or_default());
    }

    let creator_pk_val = cached_pk.as_ref().unwrap();
    let creator_sk_val = cached_sk.as_ref().unwrap();

    let pk_is_empty = creator_pk_val.trim().is_empty();
    let sk_is_empty = creator_sk_val.trim().is_empty();

    let mut active_sk = if !sk_is_empty {
        Some(creator_sk_val.clone())
    } else {
        None
    };

    // 2. If not configured, generate keypair and store them
    if pk_is_empty {
        if sk_is_empty {
            let keys = crate::infra::crypto::generate_workspace_keypair()?;
            let pub_hex = keys.public_key();
            let priv_hex = keys.private_key();

            // SOTA: Write private key to secure keyring first to prevent permanent lockout on keyring write failure.
            crate::infra::crypto::set_local_secret(&private_key_setting, &priv_hex).await?;

            let now_ms = crate::infra::time::get_current_time_ms();
            if let Err(e) = conn.execute(
                "UPDATE workspaces SET creator_public_key = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
                crate::params![pub_hex.clone(), now_ms, workspace_id],
            ).await {
                // Best effort rollback: clean up keyring secret to avoid leaving orphaned key if DB update fails.
                let _ = crate::infra::crypto::set_local_secret(&private_key_setting, "").await;
                return Err(e);
            }

            *cached_pk = Some(pub_hex.clone());
            *cached_sk = Some((*priv_hex).clone());
            active_sk = Some((*priv_hex).clone());
        } else {
            let private_key_bytes = zeroize::Zeroizing::new(
                const_hex::decode(creator_sk_val)
                    .map_err(|e| crate::infra::errors::YntraError::CryptoError(e.to_string()))?,
            );
            let mut private_key_array = zeroize::Zeroizing::new([0u8; 32]);
            if private_key_bytes.len() != 32 {
                return Err(crate::infra::errors::YntraError::CryptoError(
                    "Invalid private key length".to_string(),
                ));
            }
            private_key_array.copy_from_slice(&private_key_bytes[..32]);
            let signing_key = ed25519_dalek::SigningKey::from_bytes(&private_key_array);
            let pub_hex = const_hex::encode(signing_key.verifying_key().to_bytes());
            let now_ms = crate::infra::time::get_current_time_ms();
            conn.execute(
                "UPDATE workspaces SET creator_public_key = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
                crate::params![pub_hex, now_ms, workspace_id],
            ).await?;
            *cached_pk = Some(pub_hex);
        }
    } else if sk_is_empty {
        return Ok(());
    }

    // 3. Generate role signature and save to users table
    if let Some(ref sk) = active_sk {
        let sig = crate::infra::crypto::generate_role_signature(sk, user_id, role, workspace_id)?;
        let current_sig: Option<String> = conn
            .query_row(
                "SELECT role_signature FROM users WHERE id = ?1",
                crate::params![user_id],
                |r| r.get(0),
            )
            .await
            .ok()
            .flatten();
        if current_sig.as_ref() != Some(&sig) {
            let now_ms = crate::infra::time::get_current_time_ms();
            conn.execute(
                "UPDATE users SET role_signature = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
                crate::params![&sig, now_ms, user_id],
            ).await?;
        }
    }

    Ok(())
}

#[uniffi::export]
pub async fn reconcile_role_signatures(requester_user_id: String) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "admin" || auth.role == "platform_admin" {
        let mut to_sign = Vec::new();
        if let Ok(mut check_stmt) = conn.prepare(
            "SELECT id, role FROM users WHERE workspace_id = ?1 AND role_signature IS NULL AND role NOT IN ('anonymous', 'deleted')"
        ).await {
            if let Ok(mut rows) = check_stmt.query(crate::params![&auth.workspace_id]).await {
                while let Ok(Some(row)) = rows.next().await {
                    if let (Ok(u_id), Ok(u_role)) = (row.get::<String>(0), row.get::<String>(1)) {
                        to_sign.push((u_id, u_role));
                    }
                }
            }
        }
        if !to_sign.is_empty() {
            conn.begin_transaction().await?;
            let res = async {
                let mut cached_pk = None;
                let mut cached_sk = None;
                for (u_id, u_role) in to_sign {
                    ensure_user_role_signature_impl(
                        &conn,
                        &u_id,
                        &u_role,
                        &auth.workspace_id,
                        &mut cached_pk,
                        &mut cached_sk,
                    )
                    .await?;
                }
                Ok::<(), YntraError>(())
            }
            .await;

            match res {
                Ok(_) => conn.commit().await?,
                Err(e) => {
                    let _ = conn.rollback().await;
                    return Err(e);
                }
            }
        }
    }
    Ok(())
}
