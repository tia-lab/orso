use anyhow::Result;
use orso::{Database, DatabaseConfig, Migrations, migration};
mod models;

#[tokio::test]
async fn test_migrations() -> Result<()> {
    use models::{TestCategory, TestPost, TestUser};
    let config = DatabaseConfig::local("test_migrations.db");
    let db = Database::init(config).await?;

    // Run migrations
    let result = Migrations::init(
        &db,
        &[
            migration!(TestUser),
            migration!(TestPost),
            migration!(TestCategory),
        ],
    )
    .await?;

    assert!(!result.is_empty(), "There should be no pending migrations");

    Ok(())
}
