#[cfg(test)]
mod tests {
    use crate::{self as orso};
    use orso::{
        Database, DatabaseConfig, Filter, FilterOperator, Operator, Orso, Pagination, Sort,
        SortOrder, Value,
    };
    use serde::{Deserialize, Serialize};

    #[derive(Orso, Serialize, Deserialize, Clone, Debug, Default)]
    #[orso_table("test_compressed")]
    struct TestCompressed {
        #[orso_column(primary_key)]
        id: Option<String>,

        #[orso_column(compress)]
        data_points: Vec<i64>,

        name: String,
        age: i32,
    }

    #[derive(Orso, Serialize, Deserialize, Clone, Debug, Default)]
    #[orso_table("test_users")]
    struct TestUser {
        #[orso_column(primary_key)]
        id: Option<String>,

        name: String,

        #[orso_column(unique)]
        email: String,

        age: i32,

        #[orso_column(created_at)]
        created_at: Option<chrono::DateTime<chrono::Utc>>,

        #[orso_column(updated_at)]
        updated_at: Option<chrono::DateTime<chrono::Utc>>,
    }

    #[derive(Orso, Serialize, Deserialize, Clone, Debug, Default)]
    #[orso_table("test_multi_compressed")]
    struct TestUserWithMultipleCompressedFields {
        #[orso_column(primary_key)]
        id: Option<String>,

        #[orso_column(compress)]
        prices: Vec<i64>,

        #[orso_column(compress)]
        volumes: Vec<i64>,

        #[orso_column(compress)]
        trades: Vec<i64>,

        name: String,
        age: i32,

        #[orso_column(created_at)]
        created_at: Option<chrono::DateTime<chrono::Utc>>,

        #[orso_column(updated_at)]
        updated_at: Option<chrono::DateTime<chrono::Utc>>,
    }

    #[tokio::test]
    async fn test_compressed_field_integration() -> Result<(), Box<dyn std::error::Error>> {
        // Create in-memory database
        let config = DatabaseConfig::memory();
        let db = Database::init(config).await?;

        // Create table
        use orso::{migration, Migrations};
        Migrations::init(&db, &[migration!(TestCompressed)]).await?;

        // Create test data
        let test_data = TestCompressed {
            id: None, // Will be auto-generated
            data_points: (0..1000).map(|i| i as i64).collect(),
            name: "Test Data".to_string(),
            age: 25,
        };

        // Insert data
        test_data.insert(&db).await?;

        // Retrieve all data (since we don't know the auto-generated ID)
        let all_records = TestCompressed::find_all(&db).await?;
        assert_eq!(all_records.len(), 1);

        let retrieved = &all_records[0];
        assert_eq!(retrieved.name, "Test Data");
        assert_eq!(retrieved.age, 25);
        assert_eq!(retrieved.data_points.len(), 1000);
        assert_eq!(retrieved.data_points[0], 0);
        assert_eq!(retrieved.data_points[999], 999);

        Ok(())
    }

    #[tokio::test]
    async fn test_compressed_field_filtering() -> Result<(), Box<dyn std::error::Error>> {
        // Create in-memory database
        let config = DatabaseConfig::memory();
        let db = Database::init(config).await?;

        // Create table
        use orso::{migration, Migrations};
        Migrations::init(&db, &[migration!(TestCompressed)]).await?;

        // Create test data
        let test_data1 = TestCompressed {
            id: None,
            data_points: vec![1, 2, 3, 4, 5],
            name: "Test 1".to_string(),
            age: 20,
        };

        let test_data2 = TestCompressed {
            id: None,
            data_points: vec![10, 20, 30, 40, 50],
            name: "Test 2".to_string(),
            age: 30,
        };

        // Insert data
        test_data1.insert(&db).await?;
        test_data2.insert(&db).await?;

        // Filter by name
        let filter = FilterOperator::Single(Filter::new_simple(
            "name",
            Operator::Eq,
            Value::Text("Test 1".to_string()),
        ));
        let filtered_records = TestCompressed::find_where(filter, &db).await?;
        assert_eq!(filtered_records.len(), 1);
        assert_eq!(filtered_records[0].name, "Test 1");
        assert_eq!(filtered_records[0].data_points, vec![1, 2, 3, 4, 5]);

        // Filter by age
        let filter =
            FilterOperator::Single(Filter::new_simple("age", Operator::Gt, Value::Integer(25)));
        let filtered_records = TestCompressed::find_where(filter, &db).await?;
        assert_eq!(filtered_records.len(), 1);
        assert_eq!(filtered_records[0].name, "Test 2");

        Ok(())
    }

    #[tokio::test]
    async fn test_compressed_field_update() -> Result<(), Box<dyn std::error::Error>> {
        // Create in-memory database
        let config = DatabaseConfig::memory();
        let db = Database::init(config).await?;

        // Create table
        use orso::{migration, Migrations};
        Migrations::init(&db, &[migration!(TestCompressed)]).await?;

        // Create test data
        let test_data = TestCompressed {
            id: None,
            data_points: vec![1, 2, 3],
            name: "Test Update".to_string(),
            age: 25,
        };

        // Insert data
        test_data.insert(&db).await?;

        // Retrieve the record to get its ID
        let all_records = TestCompressed::find_all(&db).await?;
        assert_eq!(all_records.len(), 1);
        let retrieved = all_records.into_iter().next().unwrap();

        // Verify initial data
        assert_eq!(retrieved.data_points, vec![1, 2, 3]);
        assert_eq!(retrieved.name, "Test Update");
        assert_eq!(retrieved.age, 25);

        // Update the data
        let mut updated_record = retrieved;
        updated_record.data_points = vec![10, 20, 30, 40];
        updated_record.name = "Updated Test".to_string();
        updated_record.age = 30;
        updated_record.update(&db).await?;

        // Retrieve updated record
        let updated_records = TestCompressed::find_all(&db).await?;
        assert_eq!(updated_records.len(), 1);
        let updated = &updated_records[0];
        assert_eq!(updated.data_points, vec![10, 20, 30, 40]);
        assert_eq!(updated.name, "Updated Test");
        assert_eq!(updated.age, 30);

        Ok(())
    }

    #[tokio::test]
    async fn test_compressed_field_delete() -> Result<(), Box<dyn std::error::Error>> {
        // Create in-memory database
        let config = DatabaseConfig::memory();
        let db = Database::init(config).await?;

        // Create table
        use orso::{migration, Migrations};
        Migrations::init(&db, &[migration!(TestCompressed)]).await?;

        // Create test data
        let test_data = TestCompressed {
            id: None,
            data_points: vec![1, 2, 3],
            name: "Test Delete".to_string(),
            age: 25,
        };

        // Insert data
        test_data.insert(&db).await?;

        // Verify record exists
        let all_records = TestCompressed::find_all(&db).await?;
        assert_eq!(all_records.len(), 1);

        // Delete the record
        let record = &all_records[0];
        record.delete(&db).await?;

        // Verify record is deleted
        let all_records = TestCompressed::find_all(&db).await?;
        assert_eq!(all_records.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_compressed_fields_same_type() -> Result<(), Box<dyn std::error::Error>> {
        // Create in-memory database
        let config = DatabaseConfig::memory();
        let db = Database::init(config).await?;

        // Create table
        use orso::{migration, Migrations};
        Migrations::init(&db, &[migration!(TestUserWithMultipleCompressedFields)]).await?;

        // Create test data with multiple compressed fields of the same type
        let test_data = TestUserWithMultipleCompressedFields {
            id: None,
            prices: (0..1000).map(|i| i as i64 * 100).collect(),
            volumes: (0..1000).map(|i| i as i64 * 50).collect(),
            trades: (0..1000).map(|i| i as i64 * 25).collect(),
            name: "Multi Compressed User".to_string(),
            age: 30,
            created_at: None,
            updated_at: None,
        };

        // Insert data
        test_data.insert(&db).await?;

        // Retrieve data
        let all_records = TestUserWithMultipleCompressedFields::find_all(&db).await?;
        assert_eq!(all_records.len(), 1);

        let retrieved = &all_records[0];
        assert_eq!(retrieved.name, "Multi Compressed User");
        assert_eq!(retrieved.prices.len(), 1000);
        assert_eq!(retrieved.volumes.len(), 1000);
        assert_eq!(retrieved.trades.len(), 1000);
        assert_eq!(retrieved.prices[0], 0);
        assert_eq!(retrieved.prices[999], 99900);
        assert_eq!(retrieved.volumes[0], 0);
        assert_eq!(retrieved.volumes[999], 49950);
        assert_eq!(retrieved.trades[0], 0);
        assert_eq!(retrieved.trades[999], 24975);

        Ok(())
    }

    // Basic CRUD operations tests
    #[tokio::test]
    async fn test_basic_crud_operations() -> Result<(), Box<dyn std::error::Error>> {
        // Create in-memory database
        let config = DatabaseConfig::memory();
        let db = Database::init(config).await?;

        // Create table
        use orso::{migration, Migrations};
        Migrations::init(&db, &[migration!(TestUser)]).await?;

        // Create test user
        let user = TestUser {
            id: None,
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            age: 30,
            created_at: None,
            updated_at: None,
        };

        // Insert user
        user.insert(&db).await?;

        // Verify user was created with an ID
        let all_users = TestUser::find_all(&db).await?;
        assert_eq!(all_users.len(), 1);
        let created_user = &all_users[0];
        assert!(created_user.id.is_some());
        assert_eq!(created_user.name, "John Doe");
        assert_eq!(created_user.email, "john@example.com");
        assert_eq!(created_user.age, 30);
        assert!(created_user.created_at.is_some());

        // Find user by ID
        let user_id = created_user.id.as_ref().unwrap();
        let found_user = TestUser::find_by_id(user_id, &db).await?;
        assert!(found_user.is_some());
        let found_user = found_user.unwrap();
        assert_eq!(found_user.name, "John Doe");

        // Update user
        let mut updated_user = found_user;
        updated_user.name = "Jane Doe".to_string();
        updated_user.age = 35;
        updated_user.update(&db).await?;

        // Verify update
        let updated_users = TestUser::find_all(&db).await?;
        assert_eq!(updated_users.len(), 1);
        let updated_user = &updated_users[0];
        assert_eq!(updated_user.name, "Jane Doe");
        assert_eq!(updated_user.age, 35);
        assert!(updated_user.updated_at.is_some());

        // Delete user
        updated_user.delete(&db).await?;

        // Verify deletion
        let remaining_users = TestUser::find_all(&db).await?;
        assert_eq!(remaining_users.len(), 0);

        Ok(())
    }

    // Filtering and querying tests
    #[tokio::test]
    async fn test_filtering_and_querying() -> Result<(), Box<dyn std::error::Error>> {
        // Create in-memory database
        let config = DatabaseConfig::memory();
        let db = Database::init(config).await?;

        // Create table
        use orso::{migration, Migrations};
        Migrations::init(&db, &[migration!(TestUser)]).await?;

        // Create test users
        let users = vec![
            TestUser {
                id: None,
                name: "Alice".to_string(),
                email: "alice@example.com".to_string(),
                age: 25,
                created_at: None,
                updated_at: None,
            },
            TestUser {
                id: None,
                name: "Bob".to_string(),
                email: "bob@example.com".to_string(),
                age: 30,
                created_at: None,
                updated_at: None,
            },
            TestUser {
                id: None,
                name: "Charlie".to_string(),
                email: "charlie@example.com".to_string(),
                age: 35,
                created_at: None,
                updated_at: None,
            },
        ];

        // Insert users
        for user in users {
            user.insert(&db).await?;
        }

        // Test find_where with simple filter
        let filter =
            FilterOperator::Single(Filter::new_simple("age", Operator::Gt, Value::Integer(25)));
        let filtered_users = TestUser::find_where(filter, &db).await?;
        assert_eq!(filtered_users.len(), 2);
        assert!(filtered_users.iter().all(|u| u.age > 25));

        // Test find_where with multiple conditions (AND)
        let filter1 = Filter::new_simple("age", Operator::Gt, Value::Integer(25));
        let filter2 = Filter::new_simple("name", Operator::Like, Value::Text("%o%".to_string()));
        let combined_filter = FilterOperator::And(vec![
            FilterOperator::Single(filter1),
            FilterOperator::Single(filter2),
        ]);
        let filtered_users = TestUser::find_where(combined_filter, &db).await?;
        assert_eq!(filtered_users.len(), 1);
        assert_eq!(filtered_users[0].name, "Bob");

        // Test sorting
        let sort = Sort::new("age", SortOrder::Asc);
        let sorted_users = TestUser::list(Some(vec![sort]), None, &db).await?;
        assert_eq!(sorted_users.data.len(), 3);
        assert_eq!(sorted_users.data[0].age, 25);
        assert_eq!(sorted_users.data[1].age, 30);
        assert_eq!(sorted_users.data[2].age, 35);

        // Test pagination
        let pagination = Pagination::new(1, 2); // Page 1, 2 items per page
        let paginated_users = TestUser::find_paginated(&pagination, &db).await?;
        assert_eq!(paginated_users.data.len(), 2);
        assert_eq!(paginated_users.pagination.total, Some(3));

        Ok(())
    }

    // Unique constraint tests
    #[tokio::test]
    async fn test_unique_constraints() -> Result<(), Box<dyn std::error::Error>> {
        // Create in-memory database
        let config = DatabaseConfig::memory();
        let db = Database::init(config).await?;

        // Create table
        use orso::{migration, Migrations};
        Migrations::init(&db, &[migration!(TestUser)]).await?;

        // Create first user
        let user1 = TestUser {
            id: None,
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            age: 30,
            created_at: None,
            updated_at: None,
        };

        user1.insert(&db).await?;

        // Try to create another user with the same email (should fail)
        let user2 = TestUser {
            id: None,
            name: "Jane Doe".to_string(),
            email: "john@example.com".to_string(), // Same email
            age: 25,
            created_at: None,
            updated_at: None,
        };

        let result = user2.insert(&db).await;
        assert!(result.is_err());

        Ok(())
    }

    // Batch operations tests
    #[tokio::test]
    async fn test_batch_operations() -> Result<(), Box<dyn std::error::Error>> {
        // Create in-memory database
        let config = DatabaseConfig::memory();
        let db = Database::init(config).await?;

        // Create table
        use orso::{migration, Migrations};
        Migrations::init(&db, &[migration!(TestUser)]).await?;

        // Create multiple users
        let users = vec![
            TestUser {
                id: None,
                name: "User 1".to_string(),
                email: "user1@example.com".to_string(),
                age: 20,
                created_at: None,
                updated_at: None,
            },
            TestUser {
                id: None,
                name: "User 2".to_string(),
                email: "user2@example.com".to_string(),
                age: 25,
                created_at: None,
                updated_at: None,
            },
            TestUser {
                id: None,
                name: "User 3".to_string(),
                email: "user3@example.com".to_string(),
                age: 30,
                created_at: None,
                updated_at: None,
            },
        ];

        // Batch insert
        TestUser::batch_create(&users, &db).await?;

        // Verify all users were inserted
        let all_users = TestUser::find_all(&db).await?;
        assert_eq!(all_users.len(), 3);

        // Test batch delete
        let user_ids: Vec<&str> = all_users
            .iter()
            .filter_map(|u| u.id.as_ref())
            .map(|id| id.as_str())
            .collect();

        let deleted_count = TestUser::batch_delete(&user_ids, &db).await?;
        assert_eq!(deleted_count, 3);

        // Verify all users were deleted
        let remaining_users = TestUser::find_all(&db).await?;
        assert_eq!(remaining_users.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_migration_no_change_detection() -> Result<(), Box<dyn std::error::Error>> {
        use crate as orso;
        use orso::{migration, Database, DatabaseConfig, Migrations, Orso};
        use serde::{Deserialize, Serialize};
        #[derive(Orso, Serialize, Deserialize, Clone, Debug, Default)]
        #[orso_table("migration_test")]
        struct MigrationTest {
            #[orso_column(primary_key)]
            id: Option<String>,
            name: String,
            age: i32,
        }
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
        assert!(
            results2.is_empty()
                || results2
                    .iter()
                    .all(|r| matches!(r.action, orso::migrations::MigrationAction::SchemaMatched))
        );

        Ok(())
    }

    // Migration detection tests
    #[tokio::test]
    async fn test_migration_constraint_detection() -> Result<(), Box<dyn std::error::Error>> {
        // Create in-memory database
        let config = DatabaseConfig::memory();
        let db = Database::init(config).await?;

        // First, create a table without unique constraints
        #[derive(Orso, Serialize, Deserialize, Clone, Debug, Default)]
        #[orso_table("migration_test")]
        struct MigrationTestInitial {
            #[orso_column(primary_key)]
            id: Option<String>,
            name: String,
            email: String, // No unique constraint initially
            age: i32,
        }

        // Run initial migration
        use orso::{migration, Migrations};
        Migrations::init(&db, &[migration!(MigrationTestInitial)]).await?;

        // Now, create a new version with a unique constraint
        #[derive(Orso, Serialize, Deserialize, Clone, Debug, Default)]
        #[orso_table("migration_test")]
        struct MigrationTestWithUnique {
            #[orso_column(primary_key)]
            id: Option<String>,
            name: String,
            #[orso_column(unique)] // Added unique constraint
            email: String,
            age: i32,
        }

        // Run migration again - this should detect the constraint change
        let results = Migrations::init(&db, &[migration!(MigrationTestWithUnique)]).await?;

        // The migration should have detected changes and performed a migration
        assert!(!results.is_empty());
        match &results[0].action {
            orso::migrations::MigrationAction::DataMigrated { .. } => {
                // Migration was performed as expected
            }
            _ => {
                panic!("Expected DataMigrated action, got {:?}", results[0].action);
            }
        }

        // Test that the unique constraint is now enforced
        let user1 = MigrationTestWithUnique {
            id: None,
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            age: 30,
        };

        user1.insert(&db).await?;

        // Try to insert another user with the same email (should fail)
        let user2 = MigrationTestWithUnique {
            id: None,
            name: "Jane Doe".to_string(),
            email: "john@example.com".to_string(), // Same email
            age: 25,
        };

        let result = user2.insert(&db).await;
        assert!(
            result.is_err(),
            "Unique constraint should be enforced after migration"
        );

        Ok(())
    }

    // Migration compression detection tests
    #[tokio::test]
    async fn test_migration_compression_detection() -> Result<(), Box<dyn std::error::Error>> {
        // Create in-memory database
        let config = DatabaseConfig::memory();
        let db = Database::init(config).await?;

        // First, create a table without compression
        #[derive(Orso, Serialize, Deserialize, Clone, Debug, Default)]
        #[orso_table("compression_migration_test")]
        struct CompressionTestInitial {
            #[orso_column(primary_key)]
            id: Option<String>,
            name: String,
            data_points: Vec<i64>, // No compression initially
            age: i32,
        }

        // Run initial migration
        use orso::{migration, Migrations};
        Migrations::init(&db, &[migration!(CompressionTestInitial)]).await?;

        // Insert some test data
        let initial_data = CompressionTestInitial {
            id: None,
            name: "Test User".to_string(),
            data_points: (0..100).map(|i| i as i64).collect(),
            age: 25,
        };

        initial_data.insert(&db).await?;

        // Now, create a new version with compression
        #[derive(Orso, Serialize, Deserialize, Clone, Debug, Default)]
        #[orso_table("compression_migration_test")]
        struct CompressionTestWithCompression {
            #[orso_column(primary_key)]
            id: Option<String>,
            name: String,
            #[orso_column(compress)] // Added compression
            data_points: Vec<i64>,
            age: i32,
        }

        // Run migration again - this should detect the compression change
        let results = Migrations::init(&db, &[migration!(CompressionTestWithCompression)]).await?;

        // The migration should have detected changes and performed a migration
        assert!(!results.is_empty());
        match &results[0].action {
            orso::migrations::MigrationAction::DataMigrated { .. } => {
                // Migration was performed as expected
            }
            _ => {
                panic!("Expected DataMigrated action, got {:?}", results[0].action);
            }
        }

        // Verify that we can still retrieve the data correctly
        let all_records = CompressionTestWithCompression::find_all(&db).await?;
        assert_eq!(all_records.len(), 1);
        assert_eq!(all_records[0].data_points.len(), 100);
        assert_eq!(all_records[0].data_points[0], 0);
        assert_eq!(all_records[0].data_points[99], 99);

        Ok(())
    }
}

#[cfg(test)]
mod id_generation_tests {
    use crate as orso;
    use orso::{migration, Database, DatabaseConfig, Migrations, Orso, Utils};
    use serde::{Deserialize, Serialize};

    #[derive(Orso, Serialize, Deserialize, Clone, Debug, Default)]
    #[orso_table("id_generation_test")]
    struct IdGenerationTest {
        #[orso_column(primary_key)]
        id: Option<String>,
        name: String,
        age: i32,
    }

    #[tokio::test]
    async fn test_id_auto_generation() -> Result<(), Box<dyn std::error::Error>> {
        // Create in-memory database
        let config = DatabaseConfig::memory();
        let db = Database::init(config).await?;

        // Create table
        Migrations::init(&db, &[migration!(IdGenerationTest)]).await?;

        // Create record with None ID (should auto-generate)
        let record = IdGenerationTest {
            id: None, // This should be auto-generated by the database
            name: "Test User".to_string(),
            age: 25,
        };

        // Insert record
        record.insert(&db).await?;

        // Retrieve all records
        let all_records = IdGenerationTest::find_all(&db).await?;
        assert_eq!(all_records.len(), 1);

        let retrieved = &all_records[0];
        assert!(retrieved.id.is_some(), "ID should be auto-generated");
        assert!(
            !retrieved.id.as_ref().unwrap().is_empty(),
            "ID should not be empty"
        );
        assert_eq!(retrieved.name, "Test User");
        assert_eq!(retrieved.age, 25);

        Ok(())
    }

    #[tokio::test]
    async fn test_id_generation_debug() -> Result<(), Box<dyn std::error::Error>> {
        // Create in-memory database
        let config = DatabaseConfig::memory();
        let db = Database::init(config).await?;

        // Create table
        Migrations::init(&db, &[migration!(IdGenerationTest)]).await?;

        // Let's check the table schema to see what DEFAULT is set
        let schema_sql =
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='id_generation_test'";
        let mut rows = db.conn.query(schema_sql, ()).await?;

        if let Some(row) = rows.next().await? {
            let schema: String = row.get(0)?;
            println!("Table schema: {}", schema);
        }

        // Create record with None ID
        let record = IdGenerationTest {
            id: None,
            name: "Debug Test".to_string(),
            age: 30,
        };

        // Insert record
        record.insert(&db).await?;

        // Check what was actually inserted
        let all_records = IdGenerationTest::find_all(&db).await?;
        println!("Records found: {}", all_records.len());

        for record in &all_records {
            println!("Record ID: {:?}", record.id);
            println!("Record name: {}", record.name);
            println!("Record age: {}", record.age);
        }

        assert_eq!(all_records.len(), 1);
        let retrieved = &all_records[0];
        assert!(retrieved.id.is_some(), "ID should be auto-generated");

        Ok(())
    }

    #[test]
    fn test_utils_generate_id() {
        let id1 = Utils::generate_id();
        let id2 = Utils::generate_id();

        // Both should be valid UUIDs
        assert!(!id1.is_empty());
        assert!(!id2.is_empty());

        // Should be different (very high probability)
        assert_ne!(id1, id2);

        // Should contain hyphens in correct positions for UUID format
        assert!(id1.contains('-'));
        assert_eq!(id1.len(), 36); // Standard UUID length

        // Should parse as valid UUID
        let uuid1 = uuid::Uuid::parse_str(&id1);
        assert!(uuid1.is_ok());
    }

    #[test]
    fn test_utils_current_timestamp() {
        let timestamp = Utils::current_timestamp();

        // Should not be empty
        assert!(!timestamp.is_empty());

        // Should contain T and end with Z
        assert!(timestamp.contains('T'));
        assert!(timestamp.ends_with('Z'));

        // Should parse back correctly
        let parsed = Utils::parse_timestamp(&timestamp);
        assert!(parsed.is_ok());
    }

    #[test]
    fn test_utils_parse_timestamp() {
        // Test valid timestamp
        let valid_timestamp = "2025-09-20T13:12:26.845448Z";
        let parsed = Utils::parse_timestamp(valid_timestamp);
        assert!(parsed.is_ok());

        // Test invalid timestamp
        let invalid_timestamp = "invalid-timestamp";
        let parsed = Utils::parse_timestamp(invalid_timestamp);
        assert!(parsed.is_err());
    }
}
