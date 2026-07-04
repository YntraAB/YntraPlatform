#[cfg(not(target_arch = "wasm32"))]
pub mod native;

#[cfg(target_arch = "wasm32")]
pub mod wasm {
    use wasm_bindgen::prelude::*;

    pub fn query_wasm(sql: &str) {
        web_sys::console::log_1(&JsValue::from_str(&format!(
            "WASM DB Query (OPFS Web Worker Bridge): {}",
            sql
        )));
    }
}
