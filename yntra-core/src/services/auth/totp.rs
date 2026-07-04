use totp_rs::{Algorithm, TOTP, Secret};

#[uniffi::export]
pub fn generate_totp_secret() -> String {
    Secret::generate_secret().to_encoded().to_string()
}

#[uniffi::export]
pub fn verify_user_totp(secret: String, code: String) -> bool {
    let timestamp = get_current_timestamp();
    verify_totp(&secret, &code, timestamp)
}

#[cfg(not(target_arch = "wasm32"))]
fn get_current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(target_arch = "wasm32")]
fn get_current_timestamp() -> u64 {
    (js_sys::Date::now() / 1000.0) as u64
}

fn verify_totp(secret: &str, code: &str, timestamp: u64) -> bool {
    let secret_bytes = match Secret::Encoded(secret.to_string()).to_bytes() {
        Ok(b) => b,
        Err(_) => return false,
    };

    let totp = match TOTP::new(
        Algorithm::SHA1,
        6,
        1, // Skew: checks drift [-1, 0, 1]
        30,
        secret_bytes,
    ) {
        Ok(t) => t,
        Err(_) => return false,
    };

    totp.check(code, timestamp)
}
