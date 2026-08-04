use chrono::Utc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicI64, Ordering};

static CLOCK_SKEW_OFFSET_MS: AtomicI64 = AtomicI64::new(0);

#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct VerifiedTimestampRecord {
    pub timestamp_ms: i64,
    pub is_clock_tampered: bool,
    pub detected_skew_ms: i64,
}

#[uniffi::export]
pub fn update_network_time_offset(server_time_ms: i64) {
    let local_wall = Utc::now().timestamp_millis();
    let skew = server_time_ms - local_wall;
    CLOCK_SKEW_OFFSET_MS.store(skew, Ordering::Relaxed);
}

struct MonotonicAnchor {
    wall_start_ms: i64,
    #[cfg(not(target_arch = "wasm32"))]
    instant_start: std::time::Instant,
}

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
        (drift > 300_000, drift)
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
}
