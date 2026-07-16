use chrono::Utc;
use totp_rs::{Algorithm, Secret, TOTP};
use zeroize::Zeroize;

#[uniffi::export]
pub fn generate_totp_secret() -> String {
    Secret::generate_secret().to_encoded().to_string()
}

#[uniffi::export]
pub fn verify_user_totp(mut secret: String, mut code: String) -> bool {
    let timestamp = Utc::now().timestamp() as u64;
    let res = verify_totp(secret.clone(), &code, timestamp);
    secret.zeroize();
    code.zeroize();
    res
}

static LAST_VERIFIED_STEPS: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<String, Vec<u64>>>,
> = std::sync::OnceLock::new();

fn verify_totp(secret: String, code: &str, timestamp: u64) -> bool {
    let secret_bytes_res = Secret::Encoded(secret.clone()).to_bytes();

    let mut secret_bytes = match secret_bytes_res {
        Ok(b) => b,
        Err(_) => return false,
    };

    let totp_res = TOTP::new(
        Algorithm::SHA1,
        6,
        1, // Skew: checks drift [-1, 0, 1]
        30,
        secret_bytes.clone(),
    );

    secret_bytes.zeroize();

    let totp = match totp_res {
        Ok(t) => t,
        Err(_) => return false,
    };

    let current_step = timestamp / 30;
    let mut verified_step = None;
    for step_offset in &[-1i64, 0, 1] {
        let step = (current_step as i64 + step_offset) as u64;
        let expected_code = totp.generate(step * 30);
        if expected_code == code {
            verified_step = Some(step);
            break;
        }
    }

    if let Some(step) = verified_step {
        static EPHEMERAL_SALT: std::sync::OnceLock<[u8; 32]> = std::sync::OnceLock::new();
        let salt = EPHEMERAL_SALT.get_or_init(|| {
            let mut s = [0u8; 32];
            let _ = getrandom::fill(&mut s);
            s
        });

        let secret_hash = {
            use sha2::Digest;
            let mut hasher = sha2::Sha256::new();
            hasher.update(secret.as_bytes());
            hasher.update(salt);
            const_hex::encode(hasher.finalize())
        };

        let mut cache = LAST_VERIFIED_STEPS
            .get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
            .lock()
            .unwrap();

        let entry = cache.entry(secret_hash).or_insert_with(Vec::new);

        // Remove any expired steps outside of the valid skew window (current_step - 1)
        let min_valid_step = current_step.saturating_sub(1);
        entry.retain(|&s| s >= min_valid_step);

        // Check if this step has already been verified
        if entry.contains(&step) {
            return false;
        }

        entry.push(step);
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use totp_rs::{Algorithm, Secret, TOTP};

    #[test]
    fn test_totp_generation_and_verification() {
        let secret_str = generate_totp_secret();
        assert!(!secret_str.is_empty());

        let secret_bytes = Secret::Encoded(secret_str.clone()).to_bytes().unwrap();
        let totp = TOTP::new(Algorithm::SHA1, 6, 1, 30, secret_bytes).unwrap();

        let timestamp = 1700000000;
        let code = totp.generate(timestamp);

        // Verify correct code succeeds the first time
        assert!(verify_totp(secret_str.clone(), &code, timestamp));

        // Replay of the same code on the same/adjacent time steps fails
        assert!(!verify_totp(secret_str.clone(), &code, timestamp));
        assert!(!verify_totp(secret_str.clone(), &code, timestamp + 5));

        // Verify a new code from a subsequent step works
        let next_timestamp = timestamp + 30;
        let next_code = totp.generate(next_timestamp);
        assert!(verify_totp(secret_str.clone(), &next_code, next_timestamp));

        // Verify incorrect code fails
        assert!(!verify_totp(secret_str.clone(), "000000", timestamp));
    }

    #[test]
    fn test_totp_sliding_window_skew_replay() {
        let secret_str = generate_totp_secret();
        let secret_bytes = Secret::Encoded(secret_str.clone()).to_bytes().unwrap();
        let totp = TOTP::new(Algorithm::SHA1, 6, 1, 30, secret_bytes).unwrap();

        let timestamp = 1700000000;
        let current_step = timestamp / 30;

        let code_prev = totp.generate((current_step - 1) * 30);
        let code_curr = totp.generate(current_step * 30);
        let code_next = totp.generate((current_step + 1) * 30);

        // 1. Verify code_curr for current step succeeds
        assert!(verify_totp(secret_str.clone(), &code_curr, timestamp));

        // 2. Replay of same code_curr fails
        assert!(!verify_totp(secret_str.clone(), &code_curr, timestamp));

        // 3. Verify code_next (skew +1) succeeds
        assert!(verify_totp(secret_str.clone(), &code_next, timestamp));

        // 4. Replay of code_next fails
        assert!(!verify_totp(secret_str.clone(), &code_next, timestamp));

        // 5. Verify code_prev (skew -1) succeeds
        assert!(verify_totp(secret_str.clone(), &code_prev, timestamp));

        // 6. Replay of code_prev fails
        assert!(!verify_totp(secret_str.clone(), &code_prev, timestamp));

        // 7. Verify an old expired step (skew -2) fails
        let code_expired = totp.generate((current_step - 2) * 30);
        assert!(!verify_totp(secret_str.clone(), &code_expired, timestamp));
    }
}
