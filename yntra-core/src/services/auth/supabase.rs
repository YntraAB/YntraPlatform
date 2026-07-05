use crate::YntraError;
use serde::Deserialize;

#[derive(Deserialize)]
struct SupabaseUserResponse {
    email: String,
}

async fn get_supabase_config() -> (String, String) {
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
    
    let final_url = db_url.or(env_url).unwrap_or_else(|| "https://ileffmdueouhbooesjnv.supabase.co".to_string());
    let final_key = db_key.or(env_key).unwrap_or_else(|| "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6ImlsZWZmbWR1ZW91aGJvb2Vzam52Iiwicm9sZSI6ImFub24iLCJpYXQiOjE3NzU0MTkyNzMsImV4cCI6MjA5MDk5NTI3M30.X5Crq8NZH_SDLeRO74f01NpcGrooAJRPj0q82OBISuU".to_string());
    
    (final_url, final_key)
}

#[cfg_attr(not(target_arch = "wasm32"), uniffi::export)]
pub async fn get_supabase_user_email(token: String) -> Result<String, YntraError> {
    let (base_url, apikey) = get_supabase_config().await;
    let url = format!("{}/auth/v1/user", base_url.trim_end_matches('/'));

    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .header("apikey", apikey)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| YntraError::NetworkError(e.to_string()))?;

    if !res.status().is_success() {
        return Err(YntraError::NetworkError(format!("Supabase HTTP error: {}", res.status())));
    }

    let user_info: SupabaseUserResponse = res
        .json()
        .await
        .map_err(|e| YntraError::NetworkError(e.to_string()))?;

    Ok(user_info.email)
}
