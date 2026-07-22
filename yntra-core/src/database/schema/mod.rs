use super::DbConnection;
use crate::YntraError;

mod migrations;
mod seeds;
mod tables;

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

    // Ensure client pepper is loaded from database into the global static cache
    let db_pepper: Option<String> = conn
        .query_row(
            "SELECT value FROM system_settings WHERE key = 'client_pepper'",
            (),
            |r| Ok(r.get(0)?),
        )
        .await
        .ok();

    if let Some(pepper) = db_pepper {
        let _ = crate::infra::crypto::set_database_pepper(pepper);
    } else {
        // Generate new pepper and store it in database
        let mut rand_bytes = [0u8; 32];
        if getrandom::fill(&mut rand_bytes).is_ok() {
            let new_pepper = const_hex::encode(&rand_bytes);
            if conn.execute(
                "INSERT INTO system_settings (key, value) VALUES ('client_pepper', ?1)",
                crate::params![&new_pepper],
            ).await.is_ok() {
                let _ = crate::infra::crypto::set_database_pepper(new_pepper);
            }
        }
    }

    // 2. Ensure all baseline tables exist (idempotent CREATE TABLE IF NOT EXISTS)
    tables::create_initial_tables(conn).await?;

    let current_version: i32 = conn
        .query_row("PRAGMA user_version", (), |r| r.get(0))
        .await
        .unwrap_or(0);

    // Run migrations incrementally
    let latest_version = migrations::run_schema_migrations(conn, current_version).await?;
    if latest_version != current_version {
        conn.execute(&format!("PRAGMA user_version = {}", latest_version), ())
            .await?;
    }

    // Seed mock data if not already seeded
    seeds::seed_mock_data(conn).await?;

    // 3. Initialize system salt from database settings
    initialize_salt_from_db(conn).await?;

    // In debug mode, automatically clear creator_public_key for workspace-1 to recover from previous runs
    #[cfg(debug_assertions)]
    {
        let _ = conn
            .execute(
                "UPDATE workspaces SET creator_public_key = NULL WHERE id = 'workspace-1'",
                (),
            )
            .await;
    }

    Ok(())
}

fn obfuscate_salt(salt_hex: &str) -> String {
    use zeroize::Zeroize;
    let mut bytes = const_hex::decode(salt_hex).unwrap_or_default();
    let xor_key = b"YntraSaltObfuscationKey2026";
    let mut obfuscated: Vec<u8> = bytes
        .iter()
        .enumerate()
        .map(|(i, &b)| b ^ xor_key[i % xor_key.len()])
        .collect();
    let result = format!("obf:{}", const_hex::encode(&obfuscated));
    bytes.zeroize();
    obfuscated.zeroize();
    result
}

fn deobfuscate_salt(obfuscated_str: &str) -> Option<String> {
    if !obfuscated_str.starts_with("obf:") {
        return Some(obfuscated_str.to_string());
    }
    use zeroize::Zeroize;
    let body = &obfuscated_str[4..];
    let mut bytes = const_hex::decode(body).ok()?;
    let xor_key = b"YntraSaltObfuscationKey2026";
    let mut deobfuscated: Vec<u8> = bytes
        .iter()
        .enumerate()
        .map(|(i, &b)| b ^ xor_key[i % xor_key.len()])
        .collect();
    let result = const_hex::encode(&deobfuscated);
    bytes.zeroize();
    deobfuscated.zeroize();
    Some(result)
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
                deobfuscate_salt(&s).ok_or_else(|| {
                    YntraError::CryptoError("Failed to deobfuscate system salt".to_string())
                })?
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
                return Err(YntraError::CryptoError(format!(
                    "Failed to generate random salt: {:?}",
                    res.err()
                )));
            }
        }
    };

    crate::infra::crypto::initialize_system_salt(salt);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_salt_obfuscation_roundtrip() {
        let original_salt = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let obf = obfuscate_salt(original_salt);
        assert!(obf.starts_with("obf:"));
        let deobf = deobfuscate_salt(&obf).unwrap();
        assert_eq!(deobf, original_salt);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_initialize_salt_lifecycle() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = crate::database::acquire_connection().await.unwrap();

        // Ensure database settings starts clean of system_salt
        conn.execute("DELETE FROM system_settings WHERE key = 'system_salt'", ())
            .await
            .unwrap();

        // 1. Fresh initialization (generates random salt)
        initialize_salt_from_db(&conn).await.unwrap();

        // Get value from settings
        let stored: String = conn
            .query_row(
                "SELECT value FROM system_settings WHERE key = 'system_salt'",
                (),
                |r| r.get(0),
            )
            .await
            .unwrap();
        assert!(stored.starts_with("obf:"));

        // 2. Subsequent load
        initialize_salt_from_db(&conn).await.unwrap();
    }
}
