use chrono::{DateTime, Utc};
use orso::{Database, DatabaseConfig, Migrations, Orso, migration, orso_column, orso_table};
use serde::{Deserialize, Serialize};

#[derive(Orso, Serialize, Deserialize, Default, Debug, Clone)]
#[orso_table("test_users")]
pub struct TestUser {
    #[orso_column(primary_key)]
    pub id: Option<String>,
    pub name: String,
    pub email: String,
    pub age: i32,
    #[orso_column(unique)]
    pub username: String,
    #[orso_column(created_at)]
    pub created_at: Option<DateTime<Utc>>,
    #[orso_column(updated_at)]
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Orso, Serialize, Deserialize, Clone, Debug, Default)]
#[orso_table("test_posts")]
pub struct TestPost {
    #[orso_column(primary_key)]
    pub id: Option<String>,
    pub title: String,
    pub content: String,
    #[orso_column(ref = "test_users")]
    pub user_id: String,
    #[orso_column(created_at)]
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Orso, Serialize, Deserialize, Clone, Debug, Default)]
#[orso_table("test_categories")]
pub struct TestCategory {
    #[orso_column(primary_key)]
    pub id: Option<String>,
    pub name: String,
    #[orso_column(created_at)]
    pub created_at: Option<DateTime<Utc>>,
}
#[tokio::test]
async fn test_model_derivation_and_migration() {
    // Create an in-memory database for testing
    let config = DatabaseConfig::memory();
    let db = Database::init(config)
        .await
        .expect("Failed to initialize database");

    println!("Database initialized successfully");

    // Test that we can run migrations
    let result = Migrations::init(
        &db,
        &[
            migration!(TestUser),
            migration!(TestPost),
            migration!(TestCategory),
        ],
    )
    .await;

    assert!(result.is_ok(), "Migrations should succeed");
    println!("Migrations completed successfully");

    // Test that the model has the expected trait methods
    assert_eq!(TestUser::table_name(), "test_users");
    assert_eq!(TestUser::primary_key_field(), "id");

    println!("Model trait methods work correctly");
}

#[tokio::test]
async fn test_model_schema_generation() {
    // Test that we can generate migration SQL
    let sql = TestUser::migration_sql();

    assert!(sql.contains("CREATE TABLE IF NOT EXISTS test_users"));
    assert!(sql.contains("id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16))))"));
    assert!(sql.contains("name TEXT NOT NULL"));
    assert!(sql.contains("email TEXT NOT NULL"));
    assert!(sql.contains("age INTEGER NOT NULL"));
    assert!(sql.contains("username TEXT NOT NULL UNIQUE"));

    println!("Generated SQL schema:n{}", sql);
}
