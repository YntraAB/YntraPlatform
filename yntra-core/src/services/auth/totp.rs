use totp_rs::{Algorithm, TOTP, Secret};
use chrono::Utc;

#[uniffi::export]
pub fn generate_totp_secret() -> String {
    Secret::generate_secret().to_encoded().to_string()
}

#[uniffi::export]
pub fn verify_user_totp(secret: String, code: String) -> bool {
    let timestamp = Utc::now().timestamp() as u64;
    verify_totp(&secret, &code, timestamp)
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

#[cfg(test)]
mod tests {
    use super::*;
    use totp_rs::{Algorithm, TOTP, Secret};

    #[test]
    fn test_totp_generation_and_verification() {
        let secret_str = generate_totp_secret();
        assert!(!secret_str.is_empty());

        let secret_bytes = Secret::Encoded(secret_str.clone()).to_bytes().unwrap();
        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            30,
            secret_bytes,
        ).unwrap();

        let timestamp = 1700000000;
        let code = totp.generate(timestamp);

        // Verify correct code
        assert!(verify_totp(&secret_str, &code, timestamp));

        // Verify correct code with skew (one interval ahead/behind)
        assert!(verify_totp(&secret_str, &code, timestamp + 25));
        assert!(verify_totp(&secret_str, &code, timestamp - 25));

        // Verify incorrect code
        assert!(!verify_totp(&secret_str, "000000", timestamp));

        // Verify expired code (skew is 1 interval, i.e. 30 seconds, so checking drift [-30s, 0s, +30s])
        assert!(!verify_totp(&secret_str, &code, timestamp + 65));
        assert!(!verify_totp(&secret_str, &code, timestamp - 65));
    }
}

