pub mod database;
pub mod infra;
pub mod models;
pub mod services;

// Re-export error type and observer callback
pub use infra::errors::*;
pub use infra::observer::*;
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
#[cfg(target_arch = "wasm32")]
pub use infra::wasm_store;

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

    impl ToLibsqlValue for &str {
        fn to_value(&self) -> libsql::Value {
            libsql::Value::Text(self.to_string())
        }
    }

    impl ToLibsqlValue for &String {
        fn to_value(&self) -> libsql::Value {
            libsql::Value::Text((*self).clone())
        }
    }

    impl ToLibsqlValue for &i64 {
        fn to_value(&self) -> libsql::Value {
            libsql::Value::Integer(**self)
        }
    }

    impl ToLibsqlValue for &i32 {
        fn to_value(&self) -> libsql::Value {
            libsql::Value::Integer(**self as i64)
        }
    }

    impl ToLibsqlValue for &f64 {
        fn to_value(&self) -> libsql::Value {
            libsql::Value::Real(**self)
        }
    }

    impl ToLibsqlValue for &bool {
        fn to_value(&self) -> libsql::Value {
            libsql::Value::Integer(if **self { 1 } else { 0 })
        }
    }

    impl<T: ToLibsqlValue> ToLibsqlValue for &Option<T> {
        fn to_value(&self) -> libsql::Value {
            match self.as_ref() {
                Some(v) => v.to_value(),
                None => libsql::Value::Null,
            }
        }
    }

    pub use crate::params;
}

#[cfg(not(target_arch = "wasm32"))]
#[macro_export]
macro_rules! params {
    ($($value:expr),* $(,)?) => {{
        use $crate::rusqlite::ToLibsqlValue;
        vec![$($value.to_value()),*]
    }};
}
