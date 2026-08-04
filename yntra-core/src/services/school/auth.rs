use crate::database;
use crate::infra::errors::YntraError;

pub async fn verify_school_write_zkp(
    conn: &database::DbConnection,
    requester_user_id: &str,
    role: &str,
    role_proof: Option<String>,
) -> Result<(), YntraError> {
    let is_dev_bypass =
        !crate::infra::auth::is_production() && requester_user_id.starts_with("test-");

    if !is_dev_bypass {
        let (u_role, workspace_id, role_signature, creator_public_key): (
            String,
            String,
            Option<String>,
            Option<String>,
        ) = match conn
            .query_row(
                "SELECT u.role, u.workspace_id, u.role_signature, w.creator_public_key \
                 FROM users u \
                 JOIN workspaces w ON u.workspace_id = w.id \
                 WHERE u.id = ?1",
                crate::params![requester_user_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .await
        {
            Ok(val) => val,
            Err(_) => {
                return Err(YntraError::AuthError(
                    "User or workspace not found".to_string(),
                ));
            }
        };

        let sig = match role_signature {
            Some(s) => s,
            None => {
                if !crate::infra::auth::is_production() {
                    return Ok(());
                } else {
                    return Err(YntraError::AuthError(
                        "Missing role signature: offline database tampering suspected".to_string(),
                    ));
                }
            }
        };
        let pk = match creator_public_key {
            Some(p) => p,
            None => {
                if !crate::infra::auth::is_production() {
                    return Ok(());
                } else {
                    return Err(YntraError::AuthError(
                        "Workspace public key not found".to_string(),
                    ));
                }
            }
        };

        if !crate::infra::crypto::verify_role_signature(
            &pk,
            requester_user_id,
            &u_role,
            &workspace_id,
            &sig,
        ) {
            return Err(YntraError::CryptoError(
                "Role signature verification failed: offline database tampering detected"
                    .to_string(),
            ));
        }
    }

    let is_proof_required = if crate::infra::auth::is_production() {
        true
    } else {
        role_proof.is_some()
    };

    if is_proof_required {
        let proof = role_proof.ok_or_else(|| {
            YntraError::AuthError(
                "Zero-Knowledge Role Proof is required for write operations".to_string(),
            )
        })?;

        let metadata_str: Option<String> = conn
            .query_row(
                "SELECT metadata FROM users WHERE id = ?1",
                crate::params![requester_user_id],
                |r| r.get(0),
            )
            .await
            .ok()
            .flatten();

        let public_key_hex = if let Some(ref meta) = metadata_str {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(meta) {
                val.get("public_key")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        val.get("siths_public_key")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                    })
                    .unwrap_or_default()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        if public_key_hex.is_empty() {
            return Err(YntraError::AuthError(
                "Cryptographic role verification failed: User public key not found".to_string(),
            ));
        }

        let trust = crate::ZkCryptoTrust::new();
        if !trust.verify_proof(
            proof,
            requester_user_id.to_string(),
            role.to_string(),
            public_key_hex,
        ) {
            return Err(YntraError::CryptoError(
                "Zero-Knowledge Role Proof verification failed: privilege escalation or local database tampering suspected".to_string(),
            ));
        }
    }

    Ok(())
}

#[uniffi::export]
pub async fn check_school_permission(
    requester_user_id: String,
    permission_name: String,
) -> Result<bool, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    Ok(has_school_permission(&auth, &permission_name))
}

pub fn has_school_permission(auth: &crate::AuthContext, permission_name: &str) -> bool {
    if auth.role == "platform_admin"
        || auth.role == "admin"
        || auth.role == "school-admin"
        || auth.role == "role-school-admin"
        || auth.role == "principal"
        || auth.role == "role-school-principal"
    {
        return true;
    }
    if let Some(ref settings_str) = auth.workspace_settings {
        if let Ok(settings_val) = serde_json::from_str::<serde_json::Value>(settings_str) {
            if let Some(roles_arr) = settings_val.get("roles").and_then(|r| r.as_array()) {
                for r in roles_arr {
                    let r_id = r.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    let r_name = r.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let is_match = r_id == auth.role
                        || r_id.ends_with(&format!("-{}", auth.role))
                        || r_id
                            .strip_prefix("role-")
                            .map(|s| s == auth.role)
                            .unwrap_or(false)
                        || r_id
                            .strip_prefix("role-school-")
                            .map(|s| s == auth.role)
                            .unwrap_or(false)
                        || r_name.to_lowercase() == auth.role.to_lowercase();
                    if is_match {
                        if let Some(permissions) = r.get("permissions") {
                            if let Some(val) =
                                permissions.get(permission_name).and_then(|v| v.as_bool())
                            {
                                return val;
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

pub fn verify_school_permission(
    auth: &crate::AuthContext,
    permission_name: &str,
) -> Result<(), YntraError> {
    if has_school_permission(auth, permission_name) {
        Ok(())
    } else {
        Err(YntraError::AuthError(format!(
            "Access denied: role '{}' does not have permission '{}'",
            auth.role, permission_name
        )))
    }
}

pub async fn verify_student_access(
    conn: &database::DbConnection,
    auth: &crate::AuthContext,
    student_id: &str,
) -> Result<(), YntraError> {
    if auth.role == "platform_admin" {
        return Ok(());
    }

    let role_lower = auth.role.to_lowercase();
    if role_lower == "student" || role_lower == "role-school-student" {
        let profile_user_id: Option<String> = conn
            .query_row(
                "SELECT user_id FROM student_profiles WHERE id = ?1 AND workspace_id = ?2",
                crate::params![student_id, &auth.workspace_id],
                |r| r.get(0),
            )
            .await
            .map_err(|_| {
                YntraError::NotFoundError(format!("Student profile not found: {}", student_id))
            })?;

        if let Some(uid) = profile_user_id {
            if uid == auth.user_id {
                return Ok(());
            }
        }
        return Err(YntraError::AuthError(
            "Access denied: You can only view your own student records".to_string(),
        ));
    } else if role_lower == "parent" || role_lower == "role-school-parent" {
        let linked: Option<i64> = conn
            .query_row(
                "SELECT 1 FROM student_parents WHERE student_id = ?1 AND parent_user_id = ?2 AND workspace_id = ?3",
                crate::params![student_id, &auth.user_id, &auth.workspace_id],
                |r| r.get(0),
            )
            .await
            .ok();

        if linked.is_some() {
            return Ok(());
        }
        return Err(YntraError::AuthError(
            "Access denied: You are not linked to this student".to_string(),
        ));
    }

    Ok(())
}
