use crate::YntraError;
use super::DbConnection;

mod tables;
mod migrations;
mod seeds;

pub async fn setup_schema(conn: &DbConnection) -> Result<(), YntraError> {
    let current_version: i32 = conn.query_row("PRAGMA user_version", (), |r| r.get(0)).await.unwrap_or(0);
    if current_version == 0 {
        tables::create_initial_tables(conn).await?;
        migrations::run_schema_migrations(conn).await?;
        seeds::seed_mock_data(conn).await;
        conn.execute("PRAGMA user_version = 1", ()).await?;
    }
    Ok(())
}
