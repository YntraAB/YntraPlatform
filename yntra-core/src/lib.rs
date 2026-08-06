pub mod database;
pub mod infra;
pub mod models;
pub mod services;

// Re-export error type and observer callback
pub use database::proxy::RemoteSyncCoordinator;
pub use database::sync::*;
pub use database::thin_sync::*;
pub use database::zero_copy::{
    BreakGlassResult, ComplianceMode, DlpInspectionResult, DlpPolicy, EdgeSyncLoop,
    P2PMeshSyncRouter, ZeroCopyAuditStore, ZeroCopyMessageStore, ZeroCopyNoteStore, ZeroCopyStore,
    ZkCryptoTrust, create_peer_note_store, create_peer_store, inspect_payload_dlp_bytes,
};
pub use infra::auth::AuthContext;
pub use infra::errors::*;
pub use infra::observer::*;
pub use models::*;

// Re-export all FFI service functions at the crate root
pub use services::ai_automation::*;
pub use services::audit::*;
pub use services::auth::*;
pub use services::billing::*;
pub use services::blocks::*;
#[cfg(feature = "domain-care")]
pub use services::clients::*;
pub use services::csv_import::*;
pub use services::dashboard::*;
pub use services::directory::*;
pub use services::dynamic_entities::*;
pub use services::event_bus::*;
pub use services::in_app_notifications::*;
pub use services::industry_templates::*;
pub use services::integrations::*;
#[cfg(feature = "domain-jobs")]
pub use services::jobs::*;
pub use services::messages::*;
pub use services::metrics::*;
pub use services::notes::*;
pub use services::presence::*;
pub use services::pre_sync_projection::*;
pub use services::recovery::*;
pub use services::reports::*;
pub use services::role_templates::*;
#[cfg(feature = "domain-school")]
pub use services::school::*;
pub use services::semantic_guardrails::*;
pub use services::storage_guard::*;
pub use services::support::*;
pub use services::teams::*;
pub use services::telemetry::*;
pub use services::time_integrity::*;
pub use services::time_reports::*;
pub use services::todos::*;
pub use services::updater::*;
pub use services::users::*;
pub use infra::kernel::*;
pub use infra::wasm_host::*;
#[cfg(feature = "domain-vehicles")]
pub use services::vehicles::*;
pub use services::wasm_plugins::*;
pub use services::workspaces::*;


pub use services::auth::hardware::{
    PasskeyCredentialInfo, authenticate_with_passkey, delete_passkey_credential, get_user_passkeys,
    register_passkey_credential,
};
pub use services::users::{delete_user_account, export_user_personal_data};

// Support absolute paths inside submodules that import modules re-exported at the root
#[cfg(target_arch = "wasm32")]
pub use database::schema::setup_schema;
pub use infra::crypto::{
    EphemeralSessionToken, SecureStorageProvider, clear_active_session_token, clear_session_key,
    decrypt_field, encrypt_field, is_session_key_set, load_local_workspace_key,
    register_ephemeral_session_token, register_secure_storage_provider,
    revoke_ephemeral_session_token, set_session_key, touch_session_sync_timestamp,
    validate_active_session_token,
};
pub use infra::errors;
pub use infra::observer;

#[cfg(not(target_arch = "wasm32"))]
pub use database::native::set_database_directory;

// Setup UniFFI scaffolding for mobile bindings generation
uniffi::setup_scaffolding!();

#[cfg(not(target_arch = "wasm32"))]
pub mod rusqlite {
    pub use libsql::Error;
    pub use libsql::params_from_iter;

    pub trait ToLibsqlValue {
        fn to_value(&self) -> libsql::Value;
    }

    impl ToLibsqlValue for str {
        fn to_value(&self) -> libsql::Value {
            libsql::Value::Text(self.to_string())
        }
    }

    impl ToLibsqlValue for String {
        fn to_value(&self) -> libsql::Value {
            libsql::Value::Text(self.clone())
        }
    }

    impl ToLibsqlValue for i64 {
        fn to_value(&self) -> libsql::Value {
            libsql::Value::Integer(*self)
        }
    }

    impl ToLibsqlValue for i32 {
        fn to_value(&self) -> libsql::Value {
            libsql::Value::Integer(*self as i64)
        }
    }

    impl ToLibsqlValue for u32 {
        fn to_value(&self) -> libsql::Value {
            libsql::Value::Integer(*self as i64)
        }
    }

    impl ToLibsqlValue for u64 {
        fn to_value(&self) -> libsql::Value {
            libsql::Value::Integer(*self as i64)
        }
    }

    impl ToLibsqlValue for f64 {
        fn to_value(&self) -> libsql::Value {
            libsql::Value::Real(*self)
        }
    }

    impl ToLibsqlValue for bool {
        fn to_value(&self) -> libsql::Value {
            libsql::Value::Integer(if *self { 1 } else { 0 })
        }
    }

    impl ToLibsqlValue for Vec<u8> {
        fn to_value(&self) -> libsql::Value {
            libsql::Value::Blob(self.clone())
        }
    }

    impl ToLibsqlValue for [u8] {
        fn to_value(&self) -> libsql::Value {
            libsql::Value::Blob(self.to_vec())
        }
    }

    impl<T: ToLibsqlValue> ToLibsqlValue for Option<T> {
        fn to_value(&self) -> libsql::Value {
            match self {
                Some(v) => v.to_value(),
                None => libsql::Value::Null,
            }
        }
    }

    impl<T: ToLibsqlValue + ?Sized> ToLibsqlValue for &T {
        fn to_value(&self) -> libsql::Value {
            T::to_value(self)
        }
    }

    pub use crate::params;
}

#[cfg(not(target_arch = "wasm32"))]
#[macro_export]
macro_rules! params {
    ($($value:expr),* $(,)?) => {{
        #[allow(unused_imports)]
        use $crate::rusqlite::ToLibsqlValue;
        vec![$($value.to_value()),*]
    }};
}

#[cfg(not(target_arch = "wasm32"))]
#[macro_export]
macro_rules! named_params {
    ($($name:expr => $value:expr),* $(,)?) => {{
        #[allow(unused_imports)]
        use $crate::rusqlite::ToLibsqlValue;
        libsql::params::Params::Named(vec![
            $( ($name.to_string(), $value.to_value()) ),*
        ])
    }};
}

#[cfg(target_arch = "wasm32")]
pub mod rusqlite {
    pub use crate::database::wasm::params_from_iter;
    pub type Error = crate::YntraError;

    pub trait ToWasmValue {
        fn to_value(&self) -> serde_json::Value;
    }

    impl ToWasmValue for str {
        fn to_value(&self) -> serde_json::Value {
            serde_json::Value::String(self.to_string())
        }
    }

    impl ToWasmValue for String {
        fn to_value(&self) -> serde_json::Value {
            serde_json::Value::String(self.clone())
        }
    }

    impl ToWasmValue for i64 {
        fn to_value(&self) -> serde_json::Value {
            serde_json::Value::Number(serde_json::value::Number::from(*self))
        }
    }

    impl ToWasmValue for i32 {
        fn to_value(&self) -> serde_json::Value {
            serde_json::Value::Number(serde_json::value::Number::from(*self))
        }
    }

    impl ToWasmValue for u32 {
        fn to_value(&self) -> serde_json::Value {
            serde_json::Value::Number(serde_json::value::Number::from(*self))
        }
    }

    impl ToWasmValue for u64 {
        fn to_value(&self) -> serde_json::Value {
            serde_json::Value::Number(serde_json::value::Number::from(*self))
        }
    }

    impl ToWasmValue for usize {
        fn to_value(&self) -> serde_json::Value {
            serde_json::Value::Number(serde_json::value::Number::from(*self as u64))
        }
    }

    impl ToWasmValue for f64 {
        fn to_value(&self) -> serde_json::Value {
            if let Some(n) = serde_json::value::Number::from_f64(*self) {
                serde_json::Value::Number(n)
            } else {
                serde_json::Value::Null
            }
        }
    }

    impl ToWasmValue for bool {
        fn to_value(&self) -> serde_json::Value {
            serde_json::Value::Bool(*self)
        }
    }

    impl ToWasmValue for Vec<u8> {
        fn to_value(&self) -> serde_json::Value {
            serde_json::Value::String(const_hex::encode(self))
        }
    }

    impl ToWasmValue for [u8] {
        fn to_value(&self) -> serde_json::Value {
            serde_json::Value::String(const_hex::encode(self))
        }
    }

    impl<T: ToWasmValue> ToWasmValue for Option<T> {
        fn to_value(&self) -> serde_json::Value {
            match self {
                Some(v) => v.to_value(),
                None => serde_json::Value::Null,
            }
        }
    }

    impl<T: ToWasmValue + ?Sized> ToWasmValue for &T {
        fn to_value(&self) -> serde_json::Value {
            T::to_value(self)
        }
    }

    pub use crate::params;
}

#[cfg(target_arch = "wasm32")]
#[macro_export]
macro_rules! params {
    ($($value:expr),* $(,)?) => {{
        #[allow(unused_imports)]
        use $crate::rusqlite::ToWasmValue;
        vec![$($value.to_value()),*]
    }};
}

#[cfg(target_arch = "wasm32")]
#[macro_export]
macro_rules! named_params {
    ($($name:expr => $value:expr),* $(,)?) => {{
        #[allow(unused_imports)]
        use $crate::rusqlite::ToWasmValue;
        let mut map = serde_json::Map::new();
        $(
            map.insert($name.to_string(), $value.to_value());
        )*
        serde_json::Value::Object(map)
    }};
}

#[cfg(not(target_arch = "wasm32"))]
#[uniffi::export]
pub async fn init_wasm_db() -> Result<(), YntraError> {
    database::native::init_database_async().await?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub async fn wait_for_js_bridge() {
    if let Some(window) = web_sys::window() {
        let sql_key = wasm_bindgen::JsValue::from_str("yntra_execute_sql");
        let load_key = wasm_bindgen::JsValue::from_str("yntra_load_store_bin");
        let save_key = wasm_bindgen::JsValue::from_str("yntra_save_store_bin");
        let mut attempts = 0;
        loop {
            let sql_ready = js_sys::Reflect::has(&window, &sql_key).unwrap_or(false);
            let load_ready = js_sys::Reflect::has(&window, &load_key).unwrap_or(false);
            let save_ready = js_sys::Reflect::has(&window, &save_key).unwrap_or(false);
            if sql_ready && load_ready && save_ready {
                break;
            }
            attempts += 1;
            if attempts > 500 {
                tracing::warn!("Timeout waiting for JS bridge bindings (5 seconds exceeded).");
                break;
            }
            crate::infra::time::sleep_ms(10).await;
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[uniffi::export]
pub async fn init_wasm_db() -> Result<(), YntraError> {
    wait_for_js_bridge().await;
    let conn = database::acquire_connection().await?;
    database::setup_schema(&conn).await?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
#[uniffi::export]
pub fn init_tracing() -> Result<(), YntraError> {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        if !tracing::dispatcher::has_been_set() {
            tracing_wasm::set_as_global_default();
        }
    });
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
#[uniffi::export]
pub fn init_tracing() -> Result<(), YntraError> {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();
    });
    Ok(())
}

#[uniffi::export]
pub async fn load_workspace_zero_copy_stores(workspace_id: String) -> Result<(), YntraError> {
    services::todos::load_todos_from_opfs_internal(&workspace_id).await?;
    services::messages::load_messages_from_opfs_internal(&workspace_id).await?;
    services::notes::load_notes_from_opfs_internal(&workspace_id).await?;
    services::audit::load_audits_from_opfs_internal(&workspace_id).await?;
    Ok(())
}

// ============================================================================
// UniFFI Feature Fallback Wrappers for Mobile SDK Linkage Protection
// ============================================================================

#[cfg(not(feature = "domain-care"))]
#[uniffi::export]
pub async fn get_care_clients(_requester_user_id: String, _workspace_id: String) -> Result<Vec<models::ClientProfile>, YntraError> {
    Err(YntraError::ModuleDisabledError("Care domain feature ('domain-care') is disabled in this binary build.".to_string()))
}

#[cfg(not(feature = "domain-school"))]
#[uniffi::export]
pub async fn get_school_students(_requester_user_id: String, _workspace_id: String) -> Result<Vec<models::StudentProfile>, YntraError> {
    Err(YntraError::ModuleDisabledError("School domain feature ('domain-school') is disabled in this binary build.".to_string()))
}

#[cfg(not(feature = "domain-jobs"))]
#[uniffi::export]
pub async fn get_job_assignments(_requester_user_id: String, _workspace_id: String) -> Result<Vec<models::JobTicket>, YntraError> {
    Err(YntraError::ModuleDisabledError("Jobs domain feature ('domain-jobs') is disabled in this binary build.".to_string()))
}

#[cfg(not(feature = "domain-vehicles"))]
#[uniffi::export]
pub async fn get_vehicle_fleet(_requester_user_id: String, _workspace_id: String) -> Result<Vec<models::MoveVehicle>, YntraError> {
    Err(YntraError::ModuleDisabledError("Vehicles domain feature ('domain-vehicles') is disabled in this binary build.".to_string()))
}
