#[cfg(test)]
mod migration_detection_tests {
    use crate as orso;
    use orso::{Orso, Database, DatabaseConfig, Migrations, migration};
    use serde::{Deserialize, Serialize};

    #[derive(Orso, Serialize, Deserialize, Clone, Debug, Default)]
    #[orso_table("migration_test")]
    struct MigrationTest {
        #[orso_column(primary_key)]
        id: Option<String>,
        name: String,
        age: i32,
    }

    #[tokio::test]
    async fn test_migration_no_change_detection() -> Result<(), Box<dyn std::error::Error>> {
        // Create in-memory database
        let config = DatabaseConfig::memory();
        let db = Database::init(config).await?;

        // Run initial migration
        let results1 = Migrations::init(&db, &[migration!(MigrationTest)]).await?;
        println!("First migration results: {:?}", results1);

        // Run migration again - should detect no changes
        let results2 = Migrations::init(&db, &[migration!(MigrationTest)]).await?;
        println!("Second migration results: {:?}", results2);

        // Should be no migration actions since no schema changed
        assert!(results2.is_empty() || results2.iter().all(|r| matches!(r.action, orso::migrations::MigrationAction::SchemaMatched)));

        Ok(())
    }
}