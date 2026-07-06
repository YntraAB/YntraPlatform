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
        db_url = conn.query_row(
            "SELECT value FROM system_settings WHERE key = 'supabase_url'",
            (),
            |r| r.get::<String>(0)
        ).await.ok();
        
        db_key = conn.query_row(
            "SELECT value FROM system_settings WHERE key = 'supabase_anon_key'",
            (),
            |r| r.get::<String>(0)
        ).await.ok();
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
    
    let final_url = db_url.or(env_url).ok_or_else(|| YntraError::ValidationError("Supabase URL not configured".to_string()))?;
    let final_key = db_key.or(env_key).ok_or_else(|| YntraError::ValidationError("Supabase Anon Key not configured".to_string()))?;
    
    Ok((final_url, final_key))
}
 
#[cfg_attr(not(target_arch = "wasm32"), uniffi::export)]
pub async fn get_supabase_user_email(mut token: String) -> Result<String, YntraError> {
    let (base_url, apikey) = get_supabase_config().await?;
    let url = format!("{}/auth/v1/user", base_url.trim_end_matches('/'));

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
        return Err(YntraError::NetworkError(format!("Supabase HTTP error: {}", res.status())));
    }

    let user_info: SupabaseUserResponse = res
        .json()
        .await
        .map_err(|e| YntraError::NetworkError(e.to_string()))?;

    Ok(user_info.email)
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
        conn.execute("DELETE FROM system_settings WHERE key IN ('supabase_url', 'supabase_anon_key')", ()).await.unwrap();
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
        conn.execute("DELETE FROM system_settings WHERE key IN ('supabase_url', 'supabase_anon_key')", ()).await.unwrap();
        unsafe {
            std::env::remove_var("SUPABASE_URL");
            std::env::remove_var("SUPABASE_ANON_KEY");
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_supabase_token_validation_network_error() {
        // Test validating an invalid/expired token with invalid configuration
        // This will result in an HTTP error or connection error since the token/url is invalid
        let res = get_supabase_user_email("invalid_mock_token".to_string()).await;
        assert!(res.is_err());
    }
}

