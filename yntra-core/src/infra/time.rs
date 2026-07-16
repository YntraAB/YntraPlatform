use chrono::Utc;

pub fn get_current_time_ms() -> i64 {
    Utc::now().timestamp_millis()
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
        }.await;
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
}
