use totp_rs::{Algorithm, TOTP, Secret};
use chrono::Utc;
use zeroize::Zeroize;

#[uniffi::export]
pub fn generate_totp_secret() -> String {
    Secret::generate_secret().to_encoded().to_string()
}

#[uniffi::export]
pub fn verify_user_totp(secret: String, mut code: String) -> bool {
    let timestamp = Utc::now().timestamp() as u64;
    let res = verify_totp(secret, &code, timestamp);
    code.zeroize();
    res
}

fn verify_totp(secret: String, code: &str, timestamp: u64) -> bool {
    let secret_bytes_res = Secret::Encoded(secret).to_bytes();

    let mut secret_bytes = match secret_bytes_res {
        Ok(b) => b,
        Err(_) => return false,
    };

    let totp = match TOTP::new(
        Algorithm::SHA1,
        6,
        1, // Skew: checks drift [-1, 0, 1]
        30,
        secret_bytes.clone(),
    ) {
        Ok(t) => t,
        Err(_) => {
            secret_bytes.zeroize();
            return false;
        }
    };

    secret_bytes.zeroize();
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
        assert!(verify_totp(secret_str.clone(), &code, timestamp));

        // Verify correct code with skew (one interval ahead/behind)
        assert!(verify_totp(secret_str.clone(), &code, timestamp + 25));
        assert!(verify_totp(secret_str.clone(), &code, timestamp - 25));

        // Verify incorrect code
        assert!(!verify_totp(secret_str.clone(), "000000", timestamp));

        // Verify expired code (skew is 1 interval, i.e. 30 seconds, so checking drift [-30s, 0s, +30s])
        assert!(!verify_totp(secret_str.clone(), &code, timestamp + 65));
        assert!(!verify_totp(secret_str.clone(), &code, timestamp - 65));
    }
}

