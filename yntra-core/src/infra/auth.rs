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
