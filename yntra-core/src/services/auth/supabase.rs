use crate::YntraError;
use serde::Deserialize;

#[derive(Deserialize)]
struct SupabaseUserResponse {
    email: String,
}

#[cfg_attr(not(target_arch = "wasm32"), uniffi::export)]
pub async fn get_supabase_user_email(token: String) -> Result<String, YntraError> {
    let url = "https://ileffmdueouhbooesjnv.supabase.co/auth/v1/user";
    let apikey = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6ImlsZWZmbWR1ZW91aGJvb2Vzam52Iiwicm9sZSI6ImFub24iLCJpYXQiOjE3NzU0MTkyNzMsImV4cCI6MjA5MDk5NTI3M30.X5Crq8NZH_SDLeRO74f01NpcGrooAJRPj0q82OBISuU";

    let client = reqwest::Client::new();
    let res = client
        .get(url)
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
