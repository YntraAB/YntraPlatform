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
        let row: Option<(String, Option<String>)> = conn.query_row(
            "SELECT role, workspace_id FROM users WHERE id = ?1",
            crate::params![user_id],
            |r| Ok((r.get(0)?, r.get(1)?))
        ).await.ok();

        match row {
            Some((role, Some(ws_id))) => {
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
}
