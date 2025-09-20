#[cfg(test)]
#[cfg(feature = "sqlite")]
mod tests {
    use crate as orso;
    use orso::{Orso, Database, DatabaseConfig, Migrations, migration};
    use serde::{Deserialize, Serialize};

    #[derive(Orso, Serialize, Deserialize, Clone, Debug, Default)]
    #[orso_table("test_sqlite")]
    struct TestSqlite {
        #[orso_column(primary_key)]
        id: Option<String>,
        
        #[orso_column(compress)]
        data_points: Vec<i64>,
        
        name: String,
        age: i32,
    }

    #[tokio::test]
    async fn test_sqlite_orso_operations() -> Result<(), Box<dyn std::error::Error>> {
        // Create SQLite database using ORSO
        let config = DatabaseConfig::sqlite(":memory:");
        let db = Database::init(config).await?;

        // Create table using ORSO migrations
        Migrations::init(&db, &[migration!(TestSqlite)]).await?;

        // Create test data
        let test_data = TestSqlite {
            id: None, // Will be auto-generated
            data_points: (0..100).map(|i| i as i64).collect(),
            name: "Test SQLite".to_string(),
            age: 25,
        };

        // Insert data using ORSO operations
        test_data.insert(&db).await?;

        // Retrieve all data using ORSO operations
        let all_records = TestSqlite::find_all(&db).await?;
        assert_eq!(all_records.len(), 1);
        
        let retrieved = &all_records[0];
        assert_eq!(retrieved.name, "Test SQLite");
        assert_eq!(retrieved.age, 25);
        assert_eq!(retrieved.data_points.len(), 100);
        assert_eq!(retrieved.data_points[0], 0);
        assert_eq!(retrieved.data_points[99], 99);

        Ok(())
    }

    #[tokio::test]
    async fn test_sqlite_compression() -> Result<(), Box<dyn std::error::Error>> {
        // Create SQLite database
        let config = DatabaseConfig::sqlite(":memory:");
        let db = Database::init(config).await?;

        // Create table
        Migrations::init(&db, &[migration!(TestSqlite)]).await?;

        // Create test data with compressed field
        let test_data = TestSqlite {
            id: None,
            data_points: vec![1, 2, 3, 4, 5],
            name: "Compressed Test".to_string(),
            age: 30,
        };

        // Insert data
        test_data.insert(&db).await?;

        // Retrieve and verify compression works
        let all_records = TestSqlite::find_all(&db).await?;
        assert_eq!(all_records.len(), 1);
        let retrieved = &all_records[0];
        assert_eq!(retrieved.data_points, vec![1, 2, 3, 4, 5]);

        Ok(())
    }
}