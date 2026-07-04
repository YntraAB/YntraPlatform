pub mod browser;
pub mod loopback;

use tokio::sync::mpsc;
use yntra_core::DatabaseObserver;

pub struct DioxusDbObserver {
    pub tx: mpsc::UnboundedSender<()>,
}

impl DatabaseObserver for DioxusDbObserver {
    fn on_database_changed(&self) {
        let _ = self.tx.send(());
    }
}

#[cfg(target_arch = "wasm32")]
pub async fn sleep_ms(ms: u32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        if let Some(window) = web_sys::window() {
            window
                .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms as i32)
                .unwrap();
        }
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn sleep_ms(ms: u32) {
    tokio::time::sleep(std::time::Duration::from_millis(ms as u64)).await;
}

pub async fn get_supabase_user_email(token: &str) -> Result<String, String> {
    yntra_core::get_supabase_user_email(token.to_string())
        .await
        .map_err(|e| e.to_string())
}
