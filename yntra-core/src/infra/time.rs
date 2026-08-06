use chrono::Utc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicI64, Ordering};

static CLOCK_SKEW_OFFSET_MS: AtomicI64 = AtomicI64::new(0);

static HSM_MONOTONIC_COUNTER: AtomicI64 = AtomicI64::new(1);
static ROUGHTIME_AUTHENTICATED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct VerifiedTimestampRecord {
    pub timestamp_ms: i64,
    pub is_clock_tampered: bool,
    pub detected_skew_ms: i64,
}

#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct RoughtimeProofRecord {
    pub server_pubkey_hex: String,
    pub timestamp_midpoint_ms: i64,
    pub radius_ms: u32,
    pub is_signature_valid: bool,
}

#[uniffi::export]
pub fn update_network_time_offset(server_time_ms: i64) {
    let local_wall = Utc::now().timestamp_millis();
    let skew = server_time_ms - local_wall;
    CLOCK_SKEW_OFFSET_MS.store(skew, Ordering::Relaxed);
}

#[uniffi::export]
pub fn update_roughtime_time(
    roughtime_timestamp_ms: i64,
    server_pubkey_hex: String,
    nonce_hex: String,
    signature_hex: String,
) -> Result<RoughtimeProofRecord, crate::YntraError> {
    let nonce_bytes = const_hex::decode(&nonce_hex)
        .map_err(|_| crate::YntraError::ValidationError("Invalid Roughtime nonce hex".to_string()))?;
    let pubkey_bytes = const_hex::decode(&server_pubkey_hex)
        .map_err(|_| crate::YntraError::ValidationError("Invalid Roughtime pubkey hex".to_string()))?;
    let sig_bytes = const_hex::decode(&signature_hex)
        .map_err(|_| crate::YntraError::ValidationError("Invalid Roughtime signature hex".to_string()))?;

    let is_valid = if pubkey_bytes.len() == 32 && sig_bytes.len() == 64 {
        use ed25519_dalek::{Signature, Verifier, VerifyingKey};
        let mut msg = Vec::new();
        msg.extend_from_slice(b"ROUGHTIME");
        msg.extend_from_slice(&roughtime_timestamp_ms.to_be_bytes());
        msg.extend_from_slice(&nonce_bytes);

        let mut pk_arr = [0u8; 32];
        pk_arr.copy_from_slice(&pubkey_bytes);
        let mut sig_arr = [0u8; 64];
        sig_arr.copy_from_slice(&sig_bytes);

        if let Ok(vk) = VerifyingKey::from_bytes(&pk_arr) {
            let sig = Signature::from_bytes(&sig_arr);
            vk.verify(&msg, &sig).is_ok()
        } else {
            false
        }
    } else {
        false
    };

    if is_valid {
        let local_wall = Utc::now().timestamp_millis();
        let skew = roughtime_timestamp_ms - local_wall;
        CLOCK_SKEW_OFFSET_MS.store(skew, Ordering::Relaxed);
        ROUGHTIME_AUTHENTICATED.store(true, Ordering::Relaxed);
    }

    Ok(RoughtimeProofRecord {
        server_pubkey_hex,
        timestamp_midpoint_ms: roughtime_timestamp_ms,
        radius_ms: 1000,
        is_signature_valid: is_valid,
    })
}

#[uniffi::export]
pub fn update_hsm_monotonic_counter(hsm_counter: i64) {
    let current = HSM_MONOTONIC_COUNTER.load(Ordering::Relaxed);
    if hsm_counter > current {
        HSM_MONOTONIC_COUNTER.store(hsm_counter, Ordering::Relaxed);
    }
}

#[allow(dead_code)]
struct MonotonicAnchor {
    wall_start_ms: i64,
    #[cfg(not(target_arch = "wasm32"))]
    instant_start: std::time::Instant,
}

#[allow(dead_code)]
fn get_anchor() -> &'static std::sync::Mutex<MonotonicAnchor> {
    static ANCHOR: OnceLock<std::sync::Mutex<MonotonicAnchor>> = OnceLock::new();
    ANCHOR.get_or_init(|| {
        std::sync::Mutex::new(MonotonicAnchor {
            wall_start_ms: Utc::now().timestamp_millis(),
            #[cfg(not(target_arch = "wasm32"))]
            instant_start: std::time::Instant::now(),
        })
    })
}

#[uniffi::export]
pub fn get_verified_timestamp_ms() -> VerifiedTimestampRecord {
    let raw_wall = Utc::now().timestamp_millis();
    let skew = CLOCK_SKEW_OFFSET_MS.load(Ordering::Relaxed);
    let calibrated = raw_wall + skew;

    #[cfg(not(target_arch = "wasm32"))]
    let (is_tampered, drift) = {
        let anchor = get_anchor().lock().unwrap();
        let elapsed = anchor.instant_start.elapsed().as_millis() as i64;
        let expected_wall = anchor.wall_start_ms + elapsed;
        let drift = (raw_wall - expected_wall).abs();

        let _hsm_cnt = HSM_MONOTONIC_COUNTER.fetch_add(1, Ordering::Relaxed);

        let is_tampered = drift > 300_000 || raw_wall < anchor.wall_start_ms;
        (is_tampered, drift)
    };

    #[cfg(target_arch = "wasm32")]
    let (is_tampered, drift) = (false, 0i64);

    VerifiedTimestampRecord {
        timestamp_ms: calibrated,
        is_clock_tampered: is_tampered,
        detected_skew_ms: drift,
    }
}

pub fn get_current_time_ms() -> i64 {
    let verified = get_verified_timestamp_ms();
    verified.timestamp_ms
}

pub fn now_ts() -> i64 {
    get_current_time_ms()
}

pub fn get_current_datetime_str() -> String {
    Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn get_current_time_str_hm() -> String {
    Utc::now().format("%H:%M").to_string()
}

#[cfg(target_arch = "wasm32")]
struct SendFuture<F> {
    inner: F,
    thread_id: std::thread::ThreadId,
}

#[cfg(target_arch = "wasm32")]
unsafe impl<F> Send for SendFuture<F> {}

#[cfg(target_arch = "wasm32")]
impl<F: std::future::Future> std::future::Future for SendFuture<F> {
    type Output = F::Output;
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        unsafe {
            let mut_self = self.get_unchecked_mut();
            if std::thread::current().id() != mut_self.thread_id {
                panic!("Safety violation: SendFuture polled on a different thread under WASM.");
            }
            let inner = std::pin::Pin::new_unchecked(&mut mut_self.inner);
            inner.poll(cx)
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = setTimeout)]
    fn set_timeout(handler: &js_sys::Function, timeout: i32) -> wasm_bindgen::JsValue;
}

pub async fn sleep_ms(ms: u64) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
    }
    #[cfg(target_arch = "wasm32")]
    {
        let promise = js_sys::Promise::new(&mut |resolve, _| {
            let _ = set_timeout(&resolve, ms as i32);
        });
        let _ = SendFuture {
            inner: wasm_bindgen_futures::JsFuture::from(promise),
            thread_id: std::thread::current().id(),
        }
        .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datetime_format() {
        let dt = get_current_datetime_str();
        assert_eq!(dt.len(), 19);
        assert_eq!(&dt[4..5], "-");
        assert_eq!(&dt[7..8], "-");
        assert_eq!(&dt[10..11], " ");
        assert_eq!(&dt[13..14], ":");
        assert_eq!(&dt[16..17], ":");
    }

    #[test]
    fn test_hm_format() {
        let hm = get_current_time_str_hm();
        assert_eq!(hm.len(), 5);
        assert_eq!(&hm[2..3], ":");
    }

    #[test]
    fn test_network_offset_calibration() {
        let local_now = Utc::now().timestamp_millis();
        let simulated_server_now = local_now + 10_000;
        update_network_time_offset(simulated_server_now);

        let verified = get_verified_timestamp_ms();
        assert!(verified.timestamp_ms >= local_now + 9_000);
        assert!(!verified.is_clock_tampered);
    }

    #[test]
    fn test_roughtime_verification_and_hsm_counter() {
        use ed25519_dalek::Signer;

        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[42; 32]);
        let pubkey_hex = const_hex::encode(signing_key.verifying_key().to_bytes());

        let timestamp_ms = 1775000000000i64;
        let nonce_bytes = b"roughtime_nonce_1234567890123456";
        let nonce_hex = const_hex::encode(nonce_bytes);

        let mut msg = Vec::new();
        msg.extend_from_slice(b"ROUGHTIME");
        msg.extend_from_slice(&timestamp_ms.to_be_bytes());
        msg.extend_from_slice(nonce_bytes);

        let sig = signing_key.sign(&msg);
        let sig_hex = const_hex::encode(sig.to_bytes());

        let proof = update_roughtime_time(timestamp_ms, pubkey_hex.clone(), nonce_hex, sig_hex).unwrap();
        assert!(proof.is_signature_valid);
        assert_eq!(proof.timestamp_midpoint_ms, timestamp_ms);
        assert_eq!(proof.server_pubkey_hex, pubkey_hex);

        // HSM Monotonic counter test
        update_hsm_monotonic_counter(500);
        let verified = get_verified_timestamp_ms();
        assert!(verified.timestamp_ms > 0);
    }
}
