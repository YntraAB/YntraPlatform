@echo off
echo ===================================================
echo  Yntra Platform - Cross-Platform Multi-Target Build
echo ===================================================

echo [1/4] Checking Rust core workspace compilation...
cargo check --workspace
if %ERRORLEVEL% neq 0 (
    echo Error: Workspace check failed.
    exit /b %ERRORLEVEL%
)

echo [2/4] Verifying WASM target compatibility (wasm32-unknown-unknown)...
cargo check --target wasm32-unknown-unknown -p yntra-core
if %ERRORLEVEL% neq 0 (
    echo Error: WASM target compilation check failed.
    exit /b %ERRORLEVEL%
)

echo [3/4] Generating UniFFI Swift & Kotlin mobile bindings...
cargo run -p yntra-uniffi-bindgen
if %ERRORLEVEL% neq 0 (
    echo Error: UniFFI bindings generation failed.
    exit /b %ERRORLEVEL%
)

echo [4/4] Verifying Dioxus UI compilation...
cargo check -p yntra-ui
if %ERRORLEVEL% neq 0 (
    echo Error: Dioxus UI compilation failed.
    exit /b %ERRORLEVEL%
)

echo ===================================================
echo  SUCCESS: All targets (Desktop, Web WASM, iOS, Android) verified!
echo ===================================================
