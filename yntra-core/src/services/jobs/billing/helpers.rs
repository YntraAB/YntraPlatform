use crate::database;
use crate::infra::errors::YntraError;

pub fn base64_encode(input: &[u8]) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((input.len() + 2) / 3 * 4);
    let mut chunks = input.chunks_exact(3);
    while let Some(chunk) = chunks.next() {
        let b = ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | (chunk[2] as u32);
        result.push(CHARSET[((b >> 18) & 63) as usize] as char);
        result.push(CHARSET[((b >> 12) & 63) as usize] as char);
        result.push(CHARSET[((b >> 6) & 63) as usize] as char);
        result.push(CHARSET[(b & 63) as usize] as char);
    }
    let remainder = chunks.remainder();
    if remainder.len() == 1 {
        let b = (remainder[0] as u32) << 16;
        result.push(CHARSET[((b >> 18) & 63) as usize] as char);
        result.push(CHARSET[((b >> 12) & 63) as usize] as char);
        result.push('=');
        result.push('=');
    } else if remainder.len() == 2 {
        let b = ((remainder[0] as u32) << 16) | ((remainder[1] as u32) << 8);
        result.push(CHARSET[((b >> 18) & 63) as usize] as char);
        result.push(CHARSET[((b >> 12) & 63) as usize] as char);
        result.push(CHARSET[((b >> 6) & 63) as usize] as char);
        result.push('=');
    }
    result
}

pub fn create_http_client() -> Result<reqwest::Client, YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| YntraError::NetworkError(e.to_string()))
    }
    #[cfg(target_arch = "wasm32")]
    {
        Ok(reqwest::Client::new())
    }
}

pub async fn get_config_val(
    key: &str,
    _env_var: &str,
    workspace_settings: &serde_json::Value,
) -> Option<String> {
    if let Some(val) = workspace_settings.get(key).and_then(|v| v.as_str()) {
        return Some(val.to_string());
    }

    if let Ok(conn) = database::acquire_connection().await {
        let val_res: Result<String, _> = conn
            .query_row(
                "SELECT value FROM system_settings WHERE key = ?1",
                crate::params![key],
                |r| r.get(0),
            )
            .await;
        if let Ok(val) = val_res {
            return Some(val);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Ok(val) = std::env::var(_env_var) {
            return Some(val);
        }
    }

    None
}
