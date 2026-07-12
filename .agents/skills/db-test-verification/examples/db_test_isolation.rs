// Example of database testing pattern with transactional isolation and WAL mode

#[cfg(test)]
mod tests {
    use yntra_core::infra::errors::YntraError;

    // Helper function to establish a clean isolated database connection for testing
    async fn setup_test_db() -> Result<libsql::Connection, YntraError> {
        let db = libsql::Builder::new_local("file::memory:?cache=shared")
            .build()
            .await
            .map_err(|e| YntraError::DbError(e.to_string()))?;
        
        let conn = db.connect().map_err(|e| YntraError::DbError(e.to_string()))?;

        // 1. Mandatory requirement: Ensure WAL mode is active
        conn.execute("PRAGMA journal_mode = WAL;", ())
            .await
            .map_err(|e| YntraError::DbError(e.to_string()))?;

        // Run migrations
        conn.execute("CREATE TABLE IF NOT EXISTS test_data (id TEXT PRIMARY KEY, val TEXT);", ())
            .await
            .map_err(|e| YntraError::DbError(e.to_string()))?;

        Ok(conn)
    }

    #[tokio::test]
    async fn test_database_isolation() -> Result<(), YntraError> {
        let conn = setup_test_db().await?;

        // 2. Transactional Isolation: Begin Transaction
        conn.execute("BEGIN TRANSACTION;", ()).await.map_err(|e| e.to_string())?;

        // Run database queries
        conn.execute("INSERT INTO test_data (id, val) VALUES ('1', 'isolated-test-val');", ())
            .await
            .map_err(|e| e.to_string())?;

        let mut stmt = conn.prepare("SELECT val FROM test_data WHERE id = ?1").await.map_err(|e| e.to_string())?;
        let mut rows = stmt.query(["1"]).await.map_err(|e| e.to_string())?;
        let row = rows.next().await.map_err(|e| e.to_string())?.unwrap();
        let val: String = row.get(0).map_err(|e| e.to_string())?;
        assert_eq!(val, "isolated-test-val");

        // 3. Rollback changes after assertions to keep clean workspace
        conn.execute("ROLLBACK;", ()).await.map_err(|e| e.to_string())?;

        Ok(())
    }
}
