use crate::YntraError;
use super::DbConnection;

mod tables;
mod migrations;
mod seeds;

pub async fn setup_schema(conn: &DbConnection) -> Result<(), YntraError> {
    // 0. Enable WAL mode
    let _ = conn.execute("PRAGMA journal_mode = WAL", ()).await;

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
    let mut current_version: i32 = conn.query_row("PRAGMA user_version", (), |r| r.get(0)).await.unwrap_or(0);
    if current_version == 0 {
        tables::create_initial_tables(conn).await?;
        seeds::seed_mock_data(conn).await;
        conn.execute("PRAGMA user_version = 1", ()).await?;
        current_version = 1;
    }

    // Run migrations incrementally
    let latest_version = migrations::run_schema_migrations(conn, current_version).await?;
    if latest_version != current_version {
        conn.execute(&format!("PRAGMA user_version = {}", latest_version), ()).await?;
    }

    // 3. Initialize system salt from database settings
    initialize_salt_from_db(conn).await?;

    Ok(())
}

fn obfuscate_salt(salt_hex: &str) -> String {
    let bytes = const_hex::decode(salt_hex).unwrap_or_default();
    let xor_key = b"YntraSaltObfuscationKey2026";
    let obfuscated: Vec<u8> = bytes.iter().enumerate().map(|(i, &b)| b ^ xor_key[i % xor_key.len()]).collect();
    format!("obf:{}", const_hex::encode(obfuscated))
}

fn deobfuscate_salt(obfuscated_str: &str) -> Option<String> {
    if !obfuscated_str.starts_with("obf:") {
        return Some(obfuscated_str.to_string());
    }
    let body = &obfuscated_str[4..];
    let bytes = const_hex::decode(body).ok()?;
    let xor_key = b"YntraSaltObfuscationKey2026";
    let deobfuscated: Vec<u8> = bytes.iter().enumerate().map(|(i, &b)| b ^ xor_key[i % xor_key.len()]).collect();
    Some(const_hex::encode(deobfuscated))
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
        Some(s) => {
            if s.starts_with("obf:") {
                deobfuscate_salt(&s).ok_or_else(|| YntraError::CryptoError("Failed to deobfuscate system salt".to_string()))?
            } else {
                let obf = obfuscate_salt(&s);
                conn.execute(
                    "INSERT OR REPLACE INTO system_settings (key, value) VALUES ('system_salt', ?1)",
                    crate::params![&obf],
                )
                .await
                .map_err(|e| YntraError::DbError(format!("Failed to save obfuscated system salt: {}", e)))?;
                s
            }
        }
        None => {
            let mut random_bytes = [0u8; 32];
            let res = getrandom::fill(&mut random_bytes);
            if res.is_ok() {
                let hex_salt = const_hex::encode(&random_bytes);
                use zeroize::Zeroize;
                random_bytes.zeroize();
                
                let obf = obfuscate_salt(&hex_salt);
                conn.execute(
                    "INSERT OR REPLACE INTO system_settings (key, value) VALUES ('system_salt', ?1)",
                    crate::params![&obf],
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
