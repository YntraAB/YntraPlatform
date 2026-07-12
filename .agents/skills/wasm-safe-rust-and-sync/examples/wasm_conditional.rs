// Example of conditional compilation patterns for WebAssembly and Native platforms

// 1. Native-only block
#[cfg(not(target_arch = "wasm32"))]
pub fn perform_storage_setup(db_path: &str) -> Result<(), String> {
    // Native-only logic: filesystem access, raw sockets, multi-threading
    std::fs::create_dir_all(db_path).map_err(|e| e.to_string())?;
    println!("Database directory created at native filesystem: {}", db_path);
    Ok(())
}

// 2. WASM-only block
#[cfg(target_arch = "wasm32")]
pub fn perform_storage_setup(_db_path: &str) -> Result<(), String> {
    // WASM-only logic: web sys bindings, js_sys, wasm_bindgen futures
    // Browser environment storage (OPFS) setup is handled via JS Worker Bridge
    Ok(())
}

// 3. Wrapping native futures to implement Send on WebAssembly target
pub struct SendFuture<F> {
    inner: F,
}

impl<F> SendFuture<F> {
    pub fn new(inner: F) -> Self {
        Self { inner }
    }
}

// Safety: WebAssembly is single-threaded in the browser environment,
// so implementing Send is safe and satisfies cross-compilation trait bounds.
unsafe impl<F> Send for SendFuture<F> {}

impl<F: std::future::Future> std::future::Future for SendFuture<F> {
    type Output = F::Output;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let mut_self = unsafe { self.get_unchecked_mut() };
        let inner = unsafe { std::pin::Pin::new_unchecked(&mut mut_self.inner) };
        inner.poll(cx)
    }
}
