use crate::YntraError;
use super::DbConnection;

mod tables;
mod migrations;
mod seeds;

pub async fn setup_schema(conn: &DbConnection) -> Result<(), YntraError> {
    // 1. Ensure system_settings table exists
    conn.execute(
        "CREATE TABLE IF NOT EXISTS system_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
        (),
    )
    .await
    .map_err(|e| YntraError::DbError(e.to_string()))?;

    // 2. Set up initial tables and migrations if version is 0
    let current_version: i32 = conn.query_row("PRAGMA user_version", (), |r| r.get(0)).await.unwrap_or(0);
    if current_version == 0 {
        tables::create_initial_tables(conn).await?;
        migrations::run_schema_migrations(conn).await?;
        seeds::seed_mock_data(conn).await;
        conn.execute("PRAGMA user_version = 1", ()).await?;
    }

    // 3. Initialize system salt from database settings
    initialize_salt_from_db(conn).await?;

    Ok(())
}

async fn initialize_salt_from_db(conn: &DbConnection) -> Result<(), YntraError> {
    let existing_salt: Option<String> = conn
        .query_row(
            "SELECT value FROM system_settings WHERE key = 'system_salt'",
            (),
            |r| r.get(0),
        )
        .await
        .ok();

    let salt = match existing_salt {
        Some(s) => s,
        None => {
            let mut random_bytes = [0u8; 32];
            let res = getrandom::fill(&mut random_bytes);
            if res.is_ok() {
                let hex_salt = const_hex::encode(&random_bytes);
                use zeroize::Zeroize;
                random_bytes.zeroize();
                
                conn.execute(
                    "INSERT OR REPLACE INTO system_settings (key, value) VALUES ('system_salt', ?1)",
                    crate::params![&hex_salt],
                )
                .await
                .map_err(|e| YntraError::DbError(format!("Failed to save system salt: {}", e)))?;
                
                hex_salt
            } else {
                use zeroize::Zeroize;
                random_bytes.zeroize();
                return Err(YntraError::CryptoError(format!("Failed to generate random salt: {:?}", res.err())));
            }
        }
    };

    crate::infra::crypto::initialize_system_salt(salt);
    Ok(())
}
