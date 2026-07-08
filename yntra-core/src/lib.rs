pub mod database;
pub mod infra;
pub mod models;
pub mod services;

// Re-export error type and observer callback
pub use infra::errors::*;
pub use infra::observer::*;
pub use database::sync::*;
pub use infra::auth::AuthContext;
pub use models::*;

// Re-export all FFI service functions at the crate root
pub use services::todos::*;
pub use services::workspaces::*;
pub use services::users::*;
pub use services::teams::*;
pub use services::messages::*;
pub use services::notes::*;
pub use services::time_reports::*;
pub use services::clients::*;
pub use services::blocks::*;
pub use services::reports::*;
pub use services::directory::*;
pub use services::auth::*;
pub use services::jobs::*;
pub use services::audit::*;
pub use services::school::*;
pub use services::role_templates::*;


// Support absolute paths inside submodules that import modules re-exported at the root
pub use infra::errors;
pub use infra::observer;
pub use infra::crypto::{set_session_key, clear_session_key, encrypt_field, decrypt_field, is_session_key_set, load_local_workspace_key};
#[cfg(target_arch = "wasm32")]
pub use database::schema::setup_schema;


// Setup UniFFI scaffolding for mobile bindings generation
uniffi::setup_scaffolding!();

#[cfg(not(target_arch = "wasm32"))]
pub mod rusqlite {
    pub use libsql::params_from_iter;
    pub use libsql::Error;

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
pub async fn init_wasm_db() -> Result<(), YntraError> {
    Ok(())
}

#[cfg(target_arch = "wasm32")]
#[uniffi::export]
pub async fn init_wasm_db() -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    database::setup_schema(&conn).await?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
#[uniffi::export]
pub fn init_tracing() -> Result<(), YntraError> {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        tracing_wasm::set_as_global_default();
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
