use crate::YntraError;
use serde::Deserialize;

#[derive(Deserialize)]
struct SupabaseUserResponse {
    email: String,
}

async fn get_supabase_config() -> Result<(String, String), YntraError> {
    let mut db_url = None;
    let mut db_key = None;

    // 1. Try retrieving from database
    if let Ok(conn) = crate::database::acquire_connection().await {
        if let Ok(mut stmt) = conn.prepare("SELECT key, value FROM system_settings WHERE key IN ('supabase_url', 'supabase_anon_key')").await {
            if let Ok(mut rows) = stmt.query(()).await {
                while let Ok(Some(row)) = rows.next().await {
                    if let (Ok(key), Ok(val)) = (row.get::<String>(0), row.get::<String>(1)) {
                        if key == "supabase_url" {
                            db_url = Some(val);
                        } else if key == "supabase_anon_key" {
                            db_key = Some(val);
                        }
                    }
                }
            }
        }
    }

    // 2. Try retrieving from environment variables (native target only)
    let env_url = {
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::env::var("SUPABASE_URL").ok()
        }
        #[cfg(target_arch = "wasm32")]
        {
            None
        }
    };

    let env_key = {
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::env::var("SUPABASE_ANON_KEY").ok()
        }
        #[cfg(target_arch = "wasm32")]
        {
            None
        }
    };

    let final_url = db_url
        .or(env_url)
        .ok_or_else(|| YntraError::ValidationError("Supabase URL not configured".to_string()))?;
    let final_key = db_key.or(env_key).ok_or_else(|| {
        YntraError::ValidationError("Supabase Anon Key not configured".to_string())
    })?;

    Ok((final_url, final_key))
}

#[cfg_attr(not(target_arch = "wasm32"), uniffi::export)]
pub async fn get_supabase_user_email(mut token: String) -> Result<String, YntraError> {
    #[cfg(debug_assertions)]
    {
        if token.starts_with("mock_sso_email:") {
            let email = token.trim_start_matches("mock_sso_email:").to_string();
            use zeroize::Zeroize;
            token.zeroize();
            return Ok(email);
        }
    }
    #[cfg(not(debug_assertions))]
    {
        if token.starts_with("mock_sso_email:") {
            use zeroize::Zeroize;
            token.zeroize();
            return Err(YntraError::AuthError(
                "Mock SSO bypass tokens are disabled in release builds".to_string(),
            ));
        }
    }

    let (base_url, apikey) = get_supabase_config().await?;
    let url = format!("{}/auth/v1/user", base_url.trim_end_matches('/'));

    #[cfg(not(target_arch = "wasm32"))]
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| YntraError::NetworkError(e.to_string()))?;
    #[cfg(target_arch = "wasm32")]
    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .header("apikey", apikey)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await;

    use zeroize::Zeroize;
    token.zeroize();

    let res = res.map_err(|e| YntraError::NetworkError(e.to_string()))?;

    if !res.status().is_success() {
        return Err(YntraError::NetworkError(format!(
            "Supabase HTTP error: {}",
            res.status()
        )));
    }

    let user_info: SupabaseUserResponse = res
        .json()
        .await
        .map_err(|e| YntraError::NetworkError(e.to_string()))?;

    Ok(user_info.email)
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn_task<F>(future: F)
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    tokio::spawn(future);
}

#[cfg(target_arch = "wasm32")]
fn spawn_task<F>(future: F)
where
    F: std::future::Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(future);
}

#[uniffi::export]
pub async fn initiate_oauth_login(provider: String, token: String) -> Result<String, YntraError> {
    let conn = crate::database::acquire_connection().await?;
    let session_id = uuid::Uuid::new_v4().to_string();
    let now = crate::infra::time::get_current_time_ms();

    conn.execute(
        "INSERT INTO oauth_auth_sessions (id, provider, token, status, error_message, authenticated_user_id, created_at, updated_at) VALUES (?1, ?2, ?3, 'pending', NULL, NULL, ?4, ?5)",
        crate::params![&session_id, &provider, &token, now, now],
    )
    .await?;

    let session_id_clone = session_id.clone();
    let provider_clone = provider.clone();
    let token_clone = token.clone();

    spawn_task(async move {
        let res = match provider_clone.as_str() {
            "supabase" => get_supabase_user_email(token_clone).await,
            _ => Err(YntraError::ValidationError(
                "Unsupported OAuth provider".to_string(),
            )),
        };

        let now_ms = crate::infra::time::get_current_time_ms();
        if let Ok(conn_task) = crate::database::acquire_connection().await {
            match res {
                Ok(email) => {
                    let user_id: Result<String, _> = conn_task
                        .query_row(
                            "SELECT id FROM users WHERE LOWER(email) = LOWER(?1)",
                            crate::params![&email],
                            |r| r.get(0),
                        )
                        .await;

                    match user_id {
                        Ok(uid) => {
                            let _ = conn_task.execute(
                                "UPDATE oauth_auth_sessions SET status = 'success', authenticated_user_id = ?1, updated_at = ?2 WHERE id = ?3",
                                crate::params![&uid, now_ms, &session_id_clone],
                            ).await;
                        }
                        Err(_) => {
                            let err_msg = format!(
                                "User '{}' authenticated by {} is not registered in this Yntra workspace.",
                                email, provider_clone
                            );
                            let _ = conn_task.execute(
                                "UPDATE oauth_auth_sessions SET status = 'error', error_message = ?1, updated_at = ?2 WHERE id = ?3",
                                crate::params![&err_msg, now_ms, &session_id_clone],
                            ).await;
                        }
                    }
                }
                Err(err) => {
                    let err_msg = err.to_string();
                    let _ = conn_task.execute(
                        "UPDATE oauth_auth_sessions SET status = 'error', error_message = ?1, updated_at = ?2 WHERE id = ?3",
                        crate::params![&err_msg, now_ms, &session_id_clone],
                    ).await;
                }
            }
            crate::infra::observer::notify_observers();
        }
    });

    crate::infra::observer::notify_observers();
    Ok(session_id)
}

#[uniffi::export]
pub async fn get_oauth_login_status(
    session_id: String,
) -> Result<Option<crate::OauthAuthSession>, YntraError> {
    let conn = crate::database::acquire_connection().await?;
    let mut stmt = conn
        .prepare("SELECT id, provider, token, status, error_message, authenticated_user_id, created_at, updated_at FROM oauth_auth_sessions WHERE id = ?1")
        .await?;

    let mut rows = stmt.query(crate::params![&session_id]).await?;
    if let Some(row) = rows.next().await? {
        Ok(Some(crate::OauthAuthSession {
            id: row.get(0)?,
            provider: row.get(1)?,
            token: row.get(2)?,
            status: row.get(3)?,
            error_message: row.get(4)?,
            authenticated_user_id: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        }))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_supabase_config_resolution() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = crate::database::acquire_connection().await.unwrap();

        // 1. Clear database config & env variables
        conn.execute(
            "DELETE FROM system_settings WHERE key IN ('supabase_url', 'supabase_anon_key')",
            (),
        )
        .await
        .unwrap();
        unsafe {
            std::env::remove_var("SUPABASE_URL");
            std::env::remove_var("SUPABASE_ANON_KEY");
        }

        // Should return error
        let res = get_supabase_config().await;
        assert!(res.is_err());
        if let Err(YntraError::ValidationError(msg)) = res {
            assert!(msg.contains("Supabase URL not configured"));
        } else {
            panic!("Expected ValidationError");
        }

        // 2. Set Env variables
        unsafe {
            std::env::set_var("SUPABASE_URL", "https://env-url.supabase.co");
            std::env::set_var("SUPABASE_ANON_KEY", "env-key");
        }
        let (url, key) = get_supabase_config().await.unwrap();
        assert_eq!(url, "https://env-url.supabase.co");
        assert_eq!(key, "env-key");

        // 3. Set Database settings (should override/precede env vars)
        conn.execute("INSERT OR REPLACE INTO system_settings (key, value) VALUES ('supabase_url', 'https://db-url.supabase.co')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO system_settings (key, value) VALUES ('supabase_anon_key', 'db-key')", ()).await.unwrap();
        let (url, key) = get_supabase_config().await.unwrap();
        assert_eq!(url, "https://db-url.supabase.co");
        assert_eq!(key, "db-key");

        // Clean up
        conn.execute(
            "DELETE FROM system_settings WHERE key IN ('supabase_url', 'supabase_anon_key')",
            (),
        )
        .await
        .unwrap();
        unsafe {
            std::env::remove_var("SUPABASE_URL");
            std::env::remove_var("SUPABASE_ANON_KEY");
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_supabase_token_validation_network_error() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        // Test validating an invalid/expired token with invalid configuration
        // This will result in an HTTP error or connection error since the token/url is invalid
        let res = get_supabase_user_email("invalid_mock_token".to_string()).await;
        assert!(res.is_err());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_oauth_login_flow() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let _conn = crate::database::acquire_connection().await.unwrap();

        // 1. Initiate login
        let session_id = initiate_oauth_login("supabase".to_string(), "mock_token".to_string())
            .await
            .unwrap();
        assert!(!session_id.is_empty());

        // 2. Verify it's created as pending
        let session = get_oauth_login_status(session_id.clone())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(session.status, "pending");

        // 3. Wait for background task to resolve configuration & fail (since Supabase URL is not set)
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // 4. Verify session state transitioned to error
        let session_after = get_oauth_login_status(session_id.clone())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(session_after.status, "error");
        assert!(session_after.error_message.is_some());
    }
}
