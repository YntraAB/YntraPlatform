#[cfg(not(target_arch = "wasm32"))]
pub mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::{acquire_connection, DbConnection, Statement, Row, Rows};

#[cfg(target_arch = "wasm32")]
pub mod wasm;
#[cfg(target_arch = "wasm32")]
pub use wasm::{acquire_connection, DbConnection, Statement, Row, Rows};

pub mod schema;
pub use schema::setup_schema;

#[cfg(not(target_arch = "wasm32"))]
pub static DB_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());


