use crate::YntraError;
use super::DbConnection;

mod tables;
mod migrations;
mod seeds;

pub async fn setup_schema(conn: &DbConnection) -> Result<(), YntraError> {
    tables::create_initial_tables(conn).await?;
    migrations::run_schema_migrations(conn).await?;
    seeds::seed_mock_data(conn).await;
    Ok(())
}
