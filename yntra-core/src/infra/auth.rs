use crate::database::DbConnection;
use crate::YntraError;

#[derive(Clone, Debug)]
pub struct AuthContext {
    pub user_id: String,
    pub role: String,
    pub workspace_id: String,
    pub is_admin: bool,
}

impl AuthContext {
    pub async fn authorize(conn: &DbConnection, user_id: &str) -> Result<Self, YntraError> {
        let row: Option<(String, Option<String>, Option<String>)> = conn.query_row(
            "SELECT role, workspace_id, role_signature FROM users WHERE id = ?1",
            crate::params![user_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        ).await.ok();

        match row {
            Some((role, Some(ws_id), role_sig)) => {
                let creator_pk: Option<String> = conn.query_row(
                    "SELECT creator_public_key FROM workspaces WHERE id = ?1",
                    crate::params![&ws_id],
                    |r| Ok(r.get(0)?)
                ).await.unwrap_or(None);

                if let Some(pk) = creator_pk {
                    if !pk.trim().is_empty() {
                        let is_admin = role == "admin" || role == "platform_admin";
                        if is_admin {
                            let signature_str = role_sig.unwrap_or_default();
                            let is_valid = crate::infra::crypto::verify_role_signature(
                                &pk,
                                user_id,
                                &role,
                                &ws_id,
                                &signature_str
                            );
                            if !is_valid {
                                return Err(YntraError::AuthError("Cryptographic signature verification failed for user role (possible privilege escalation detected)".to_string()));
                            }
                        }
                    }
                }

                let is_admin = role == "admin" || role == "platform_admin";
                Ok(Self {
                    user_id: user_id.to_string(),
                    role,
                    workspace_id: ws_id,
                    is_admin,
                })
            }
            _ => Err(YntraError::AuthError("Requester user not found or invalid workspace".to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database;

    #[tokio::test]
    async fn test_auth_context_authorize_admin() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // Setup test admin user
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('workspace-auth-1', 'Auth WS', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('user-auth-admin', 'workspace-auth-1', 'admin@auth.io', 'admin')", ()).await.unwrap();

        let auth = AuthContext::authorize(&conn, "user-auth-admin").await.unwrap();
        assert_eq!(auth.user_id, "user-auth-admin");
        assert_eq!(auth.role, "admin");
        assert_eq!(auth.workspace_id, "workspace-auth-1");
        assert!(auth.is_admin);

        // Cleanup
        conn.execute("DELETE FROM users WHERE id = 'user-auth-admin'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'workspace-auth-1'", ()).await.unwrap();
    }

    #[tokio::test]
    async fn test_auth_context_authorize_non_admin() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // Setup test standard user
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('workspace-auth-2', 'Auth WS 2', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('user-auth-normal', 'workspace-auth-2', 'normal@auth.io', 'user')", ()).await.unwrap();

        let auth = AuthContext::authorize(&conn, "user-auth-normal").await.unwrap();
        assert_eq!(auth.user_id, "user-auth-normal");
        assert_eq!(auth.role, "user");
        assert_eq!(auth.workspace_id, "workspace-auth-2");
        assert!(!auth.is_admin);

        // Cleanup
        conn.execute("DELETE FROM users WHERE id = 'user-auth-normal'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'workspace-auth-2'", ()).await.unwrap();
    }

    #[tokio::test]
    async fn test_auth_context_authorize_invalid_user() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        let res = AuthContext::authorize(&conn, "non-existent-user").await;
        assert!(res.is_err());
        if let Err(YntraError::AuthError(msg)) = res {
            assert!(msg.contains("Requester user not found"));
        } else {
            panic!("Expected AuthError");
        }
    }

    #[tokio::test]
    async fn test_auth_role_signature_verification() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // 1. Setup workspace with a creator public key
        let creator_pk = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f2c3c8e2";
        let creator_sk = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings, creator_public_key) VALUES ('workspace-auth-sig', 'Auth Sig WS', '[]', '{}', ?1)", crate::params![creator_pk]).await.unwrap();

        // 2. Generate a valid signature for an admin role
        let valid_sig = crate::infra::crypto::generate_role_signature(creator_sk, "user-auth-sig-admin", "admin", "workspace-auth-sig").unwrap();

        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, role_signature) VALUES ('user-auth-sig-admin', 'workspace-auth-sig', 'admin-sig@auth.io', 'admin', ?1)", crate::params![valid_sig]).await.unwrap();

        let auth = AuthContext::authorize(&conn, "user-auth-sig-admin").await;
        assert!(auth.is_ok());

        // 3. Insert admin user with INVALID signature (tampered locally)
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, role_signature) VALUES ('user-auth-sig-tampered', 'workspace-auth-sig', 'tampered@auth.io', 'admin', 'badsignature')", ()).await.unwrap();

        let auth_fail = AuthContext::authorize(&conn, "user-auth-sig-tampered").await;
        assert!(auth_fail.is_err());
        if let Err(YntraError::AuthError(msg)) = auth_fail {
            assert!(msg.contains("privilege escalation detected"));
        } else {
            panic!("Expected privilege escalation AuthError");
        }

        // Cleanup
        conn.execute("DELETE FROM users WHERE id IN ('user-auth-sig-admin', 'user-auth-sig-tampered')", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'workspace-auth-sig'", ()).await.unwrap();
    }
}
