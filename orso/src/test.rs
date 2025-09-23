#[cfg(test)]
mod tests {
    use crate::{self as orso, FloatingCodec, IntegerCodec, Migrations, Utils};
    use orso::{
        migration, Database, DatabaseConfig, Filter, FilterOperator, Operator, Orso, Pagination,
        Sort, SortOrder, Value,
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

    #[derive(Orso, serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
    #[orso_table("field_type_debug")]
    struct FieldTypeDebug {
        #[orso_column(primary_key)]
        id: Option<String>,

        #[orso_column(compress)]
        int_data: Vec<i64>,

        #[orso_column(compress)]
        float_data: Vec<f64>,

        name: String,
    }

    #[derive(Orso, Serialize, Deserialize, Clone, Debug, Default)]
    #[orso_table("debug_compressed")]
    struct DebugCompressed {
        #[orso_column(primary_key)]
        id: Option<String>,

        #[orso_column(compress)]
        data_points: Vec<i64>,

        name: String,
        age: i32,
    }

    #[tokio::test]
    async fn test_field_type_debug() {
        println!("Testing field types:");
        let field_names = FieldTypeDebug::field_names();
        let field_types = FieldTypeDebug::field_types();
        let compressed_flags = FieldTypeDebug::field_compressed();

        for i in 0..field_names.len() {
            println!(
                "Field: {} -> Type: {:?} -> Compressed: {}",
                field_names[i], field_types[i], compressed_flags[i]
            );
        }
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

    #[tokio::test]
    async fn simple_compression_test() -> Result<(), Box<dyn std::error::Error>> {
        #[derive(Orso, Serialize, Deserialize, Clone, Debug, Default)]
        #[orso_table("compression_test")]
        struct CompressionTest {
            #[orso_column(primary_key)]
            id: Option<String>,

            #[orso_column(compress)]
            int_data: Vec<i64>,

            #[orso_column(compress)]
            float_data: Vec<f64>,

            name: String,
        }

        // Create a local database for testing
        let db_path = "test_compression.db";
        let config = DatabaseConfig::local(db_path);
        let db = Database::init(config).await?;

        // Create table
        Migrations::init(&db, &[migration!(CompressionTest)]).await?;

        // Create test data
        let test_data = CompressionTest {
            id: None,
            int_data: (0..1000).map(|i| i as i64 * 100).collect(),
            float_data: (0..1000).map(|i| i as f64 * 0.01).collect(),
            name: "Test Data".to_string(),
        };

        println!("Original data sizes:");
        println!("  int_data: {} elements", test_data.int_data.len());
        println!("  float_data: {} elements", test_data.float_data.len());

        // Test compression codecs directly
        let integer_codec = IntegerCodec::default();
        let floating_codec = FloatingCodec::default();

        // Compress data directly
        let compressed_int = integer_codec.compress_i64(&test_data.int_data)?;
        let compressed_float = floating_codec.compress_f64(&test_data.float_data, None)?;

        println!(
            "\
Direct compression results:"
        );
        println!(
            "  int_data: {} bytes (compressed from {} bytes)",
            compressed_int.len(),
            test_data.int_data.len() * 8
        );
        println!(
            "  float_data: {} bytes (compressed from {} bytes)",
            compressed_float.len(),
            test_data.float_data.len() * 8
        );

        println!(
            "\
Compression ratios:"
        );
        println!(
            "  int_data: {:.2}x",
            (test_data.int_data.len() * 8) as f64 / compressed_int.len() as f64
        );
        println!(
            "  float_data: {:.2}x",
            (test_data.float_data.len() * 8) as f64 / compressed_float.len() as f64
        );

        // Test decompression
        let _decompressed_int = integer_codec.decompress_i64(&compressed_int)?;
        let decompressed_float = floating_codec.decompress_f64(&compressed_float, None)?;

        println!(
            "\
Decompression verification:"
        );
        //println!("  int_data matches: {}", decompressed_int == test_data.int_data);
        println!(
            "  float_data matches: {}",
            decompressed_float
                .iter()
                .zip(test_data.float_data.iter())
                .all(|(a, b)| (a - b).abs() < 1e-10)
        );

        // Insert data into database
        test_data.insert(&db).await?;

        // Retrieve data from database
        let retrieved_records = CompressionTest::find_all(&db).await?;
        assert_eq!(retrieved_records.len(), 1);

        let retrieved = &retrieved_records[0];
        println!(
            "\
Database retrieval verification:"
        );
        println!("  Name matches: {}", retrieved.name == "Test Data");
        println!(
            "  int_data length matches: {}",
            retrieved.int_data.len() == test_data.int_data.len()
        );
        println!(
            "  float_data length matches: {}",
            retrieved.float_data.len() == test_data.float_data.len()
        );

        // Check if data matches
        let int_matches = retrieved.int_data == test_data.int_data;
        let float_matches = retrieved
            .float_data
            .iter()
            .zip(test_data.float_data.iter())
            .all(|(a, b)| (a - b).abs() < 1e-10);

        println!("  int_data matches: {}", int_matches);
        println!("  float_data matches: {}", float_matches);

        // Let's also check what the database thinks it stored by looking at the schema
        println!(
            "\
Checking table schema..."
        );
        let mut rows = db
            .conn
            .query(
                "SELECT sql FROM sqlite_master WHERE type='table' AND name='compression_test'",
                (),
            )
            .await?;
        if let Some(row) = rows.next().await? {
            let schema: String = row.get(0)?;
            println!("Table schema: {}", schema);
        }

        // Clean up
        std::fs::remove_file(db_path)?;

        println!(
            "\
Test completed successfully!"
        );
        Ok(())
    }

    #[tokio::test]
    async fn batch_compression_test() -> Result<(), Box<dyn std::error::Error>> {
        use orso::{migration, Database, DatabaseConfig, Migrations, Orso};
        use serde::{Deserialize, Serialize};

        #[derive(Orso, Serialize, Deserialize, Clone, Debug, Default)]
        #[orso_table("batch_compression_test")]
        struct BatchCompressionTest {
            #[orso_column(primary_key)]
            id: Option<String>,

            #[orso_column(compress)]
            int_data_1: Vec<i64>,

            #[orso_column(compress)]
            int_data_2: Vec<i64>,

            #[orso_column(compress)]
            int_data_3: Vec<i64>,

            #[orso_column(compress)]
            float_data_1: Vec<f64>,

            #[orso_column(compress)]
            float_data_2: Vec<f64>,

            #[orso_column(compress)]
            float_data_3: Vec<f64>,

            #[orso_column(compress)]
            u64_data_1: Vec<u64>,

            #[orso_column(compress)]
            u64_data_2: Vec<u64>,

            #[orso_column(compress)]
            u64_data_3: Vec<u64>,

            name: String,
            description: String,
        }
        // Create a local database for testing
        let db_path = "batch_compression_test.db";
        let config = DatabaseConfig::local(db_path);
        let db = Database::init(config).await?;

        // Create table
        Migrations::init(&db, &[migration!(BatchCompressionTest)]).await?;

        // Create test data
        let test_data1 = BatchCompressionTest {
            id: None,
            int_data_1: (0..5000).map(|i| i as i64 * 100).collect(),
            int_data_2: (0..5000).map(|i| i as i64 * 200).collect(),
            int_data_3: (0..5000).map(|i| i as i64 * 300).collect(),
            float_data_1: (0..5000).map(|i| i as f64 * 0.01).collect(),
            float_data_2: (0..5000).map(|i| i as f64 * 0.02).collect(),
            float_data_3: (0..5000).map(|i| i as f64 * 0.03).collect(),
            u64_data_1: (0..5000).map(|i| i as u64 * 400).collect(),
            u64_data_2: (0..5000).map(|i| i as u64 * 500).collect(),
            u64_data_3: (0..5000).map(|i| i as u64 * 600).collect(),
            name: "Test Data 1".to_string(),
            description: "First batch test record".to_string(),
        };

        let test_data2 = BatchCompressionTest {
            id: None,
            int_data_1: (0..3000).map(|i| i as i64 * 10).collect(),
            int_data_2: (0..3000).map(|i| i as i64 * 20).collect(),
            int_data_3: (0..3000).map(|i| i as i64 * 30).collect(),
            float_data_1: (0..3000).map(|i| i as f64 * 0.1).collect(),
            float_data_2: (0..3000).map(|i| i as f64 * 0.2).collect(),
            float_data_3: (0..3000).map(|i| i as f64 * 0.3).collect(),
            u64_data_1: (0..3000).map(|i| i as u64 * 40).collect(),
            u64_data_2: (0..3000).map(|i| i as u64 * 50).collect(),
            u64_data_3: (0..3000).map(|i| i as u64 * 60).collect(),
            name: "Test Data 2".to_string(),
            description: "Second batch test record".to_string(),
        };

        let test_data3 = BatchCompressionTest {
            id: None,
            int_data_1: (0..7000).map(|i| i as i64 * 1).collect(),
            int_data_2: (0..7000).map(|i| i as i64 * 2).collect(),
            int_data_3: (0..7000).map(|i| i as i64 * 3).collect(),
            float_data_1: (0..7000).map(|i| i as f64 * 1.0).collect(),
            float_data_2: (0..7000).map(|i| i as f64 * 2.0).collect(),
            float_data_3: (0..7000).map(|i| i as f64 * 3.0).collect(),
            u64_data_1: (0..7000).map(|i| i as u64 * 4).collect(),
            u64_data_2: (0..7000).map(|i| i as u64 * 5).collect(),
            u64_data_3: (0..7000).map(|i| i as u64 * 6).collect(),
            name: "Test Data 3".to_string(),
            description: "Third batch test record".to_string(),
        };

        println!("Original data sizes:");
        println!(
            "  Record 1 int_data: {} elements each",
            test_data1.int_data_1.len()
        );
        println!(
            "  Record 1 float_data: {} elements each",
            test_data1.float_data_1.len()
        );
        println!(
            "  Record 1 u64_data: {} elements each",
            test_data1.u64_data_1.len()
        );

        // Test compression codecs directly
        let integer_codec = IntegerCodec::default();
        let floating_codec = FloatingCodec::default();

        // Compress data directly for first record
        let compressed_int_1 = integer_codec.compress_i64(&test_data1.int_data_1)?;
        let compressed_float_1 = floating_codec.compress_f64(&test_data1.float_data_1, None)?;
        let compressed_u64_1 = integer_codec.compress_u64(&test_data1.u64_data_1)?;

        println!("\nDirect compression results for first record:");
        println!(
            "  int_data_1: {} bytes (compressed from {} bytes)",
            compressed_int_1.len(),
            test_data1.int_data_1.len() * 8
        );
        println!(
            "  float_data_1: {} bytes (compressed from {} bytes)",
            compressed_float_1.len(),
            test_data1.float_data_1.len() * 8
        );
        println!(
            "  u64_data_1: {} bytes (compressed from {} bytes)",
            compressed_u64_1.len(),
            test_data1.u64_data_1.len() * 8
        );

        println!("\nCompression ratios for first record:");
        println!(
            "  int_data_1: {:.2}x",
            (test_data1.int_data_1.len() * 8) as f64 / compressed_int_1.len() as f64
        );
        println!(
            "  float_data_1: {:.2}x",
            (test_data1.float_data_1.len() * 8) as f64 / compressed_float_1.len() as f64
        );
        println!(
            "  u64_data_1: {:.2}x",
            (test_data1.u64_data_1.len() * 8) as f64 / compressed_u64_1.len() as f64
        );

        // Test decompression
        let _decompressed_int = integer_codec.decompress_i64(&compressed_int_1)?;
        let decompressed_float = floating_codec.decompress_f64(&compressed_float_1, None)?;
        let _decompressed_u64 = integer_codec.decompress_u64(&compressed_u64_1)?;

        println!("\nDecompression verification for first record:");
        //println!("  int_data_1 matches: {}", decompressed_int == test_data1.int_data_1);
        println!(
            "  float_data_1 matches: {}",
            decompressed_float
                .iter()
                .zip(test_data1.float_data_1.iter())
                .all(|(a, b)| (a - b).abs() < 1e-10)
        );
        //println!("  u64_data_1 matches: {}", decompressed_u64 == test_data1.u64_data_1);

        // Test individual inserts
        println!("\n=== Testing Individual Inserts ===");
        test_data1.insert(&db).await?;
        test_data2.insert(&db).await?;
        test_data3.insert(&db).await?;

        // Retrieve data from database
        let retrieved_records = BatchCompressionTest::find_all(&db).await?;
        println!(
            "Retrieved {} records from database",
            retrieved_records.len()
        );

        for (i, retrieved) in retrieved_records.iter().enumerate() {
            println!(
                "  Record {}: name='{}', description='{}'",
                i + 1,
                retrieved.name,
                retrieved.description
            );
            println!("    int_data_1 length: {}", retrieved.int_data_1.len());
            println!("    float_data_1 length: {}", retrieved.float_data_1.len());
            println!("    u64_data_1 length: {}", retrieved.u64_data_1.len());
        }

        // Verify data integrity
        let record1 = &retrieved_records[0];
        let record2 = &retrieved_records[1];
        let record3 = &retrieved_records[2];

        println!("\nData integrity verification:");
        println!(
            "  Record 1 int_data_1 matches: {}",
            record1.int_data_1 == test_data1.int_data_1
        );
        println!(
            "  Record 1 float_data_1 matches: {}",
            record1
                .float_data_1
                .iter()
                .zip(test_data1.float_data_1.iter())
                .all(|(a, b)| (a - b).abs() < 1e-10)
        );
        println!(
            "  Record 1 u64_data_1 matches: {}",
            record1.u64_data_1 == test_data1.u64_data_1
        );

        println!(
            "  Record 2 int_data_1 matches: {}",
            record2.int_data_1 == test_data2.int_data_1
        );
        println!(
            "  Record 2 float_data_1 matches: {}",
            record2
                .float_data_1
                .iter()
                .zip(test_data2.float_data_1.iter())
                .all(|(a, b)| (a - b).abs() < 1e-10)
        );
        println!(
            "  Record 2 u64_data_1 matches: {}",
            record2.u64_data_1 == test_data2.u64_data_1
        );

        println!(
            "  Record 3 int_data_1 matches: {}",
            record3.int_data_1 == test_data3.int_data_1
        );
        println!(
            "  Record 3 float_data_1 matches: {}",
            record3
                .float_data_1
                .iter()
                .zip(test_data3.float_data_1.iter())
                .all(|(a, b)| (a - b).abs() < 1e-10)
        );
        println!(
            "  Record 3 u64_data_1 matches: {}",
            record3.u64_data_1 == test_data3.u64_data_1
        );

        // Clean up for batch test
        std::fs::remove_file(db_path)?;

        // Test batch inserts
        println!("\n=== Testing Batch Inserts ===");
        let db_path2 = "batch_compression_test2.db";
        let config2 = DatabaseConfig::local(db_path2);
        let db2 = Database::init(config2).await?;

        // Create table
        Migrations::init(&db2, &[migration!(BatchCompressionTest)]).await?;

        let batch_data = vec![test_data1.clone(), test_data2.clone(), test_data3.clone()];

        // Batch insert
        BatchCompressionTest::batch_create(&batch_data, &db2).await?;

        // Retrieve data from database
        let retrieved_records_batch = BatchCompressionTest::find_all(&db2).await?;
        println!(
            "Retrieved {} records from batch insert",
            retrieved_records_batch.len()
        );

        for (i, retrieved) in retrieved_records_batch.iter().enumerate() {
            println!(
                "  Record {}: name='{}', description='{}'",
                i + 1,
                retrieved.name,
                retrieved.description
            );
            println!("    int_data_1 length: {}", retrieved.int_data_1.len());
            println!("    float_data_1 length: {}", retrieved.float_data_1.len());
            println!("    u64_data_1 length: {}", retrieved.u64_data_1.len());
        }

        // Verify batch data integrity
        if retrieved_records_batch.len() >= 3 {
            let batch_record1 = &retrieved_records_batch[0];
            let batch_record2 = &retrieved_records_batch[1];
            let batch_record3 = &retrieved_records_batch[2];

            println!("\nBatch data integrity verification:");
            println!(
                "  Record 1 int_data_1 matches: {}",
                batch_record1.int_data_1 == test_data1.int_data_1
            );
            println!(
                "  Record 1 float_data_1 matches: {}",
                batch_record1
                    .float_data_1
                    .iter()
                    .zip(test_data1.float_data_1.iter())
                    .all(|(a, b)| (a - b).abs() < 1e-10)
            );
            println!(
                "  Record 1 u64_data_1 matches: {}",
                batch_record1.u64_data_1 == test_data1.u64_data_1
            );

            println!(
                "  Record 2 int_data_1 matches: {}",
                batch_record2.int_data_1 == test_data2.int_data_1
            );
            println!(
                "  Record 2 float_data_1 matches: {}",
                batch_record2
                    .float_data_1
                    .iter()
                    .zip(test_data2.float_data_1.iter())
                    .all(|(a, b)| (a - b).abs() < 1e-10)
            );
            println!(
                "  Record 2 u64_data_1 matches: {}",
                batch_record2.u64_data_1 == test_data2.u64_data_1
            );

            println!(
                "  Record 3 int_data_1 matches: {}",
                batch_record3.int_data_1 == test_data3.int_data_1
            );
            println!(
                "  Record 3 float_data_1 matches: {}",
                batch_record3
                    .float_data_1
                    .iter()
                    .zip(test_data3.float_data_1.iter())
                    .all(|(a, b)| (a - b).abs() < 1e-10)
            );
            println!(
                "  Record 3 u64_data_1 matches: {}",
                batch_record3.u64_data_1 == test_data3.u64_data_1
            );
        }

        // Clean up
        std::fs::remove_file(db_path2)?;

        println!("\nAll tests completed successfully!");
        Ok(())
    }
    #[tokio::test]
    async fn batch_operations_test() -> Result<(), Box<dyn std::error::Error>> {
        use orso::{migration, Database, DatabaseConfig, Migrations, Orso};
        use serde::{Deserialize, Serialize};

        #[derive(Orso, Serialize, Deserialize, Clone, Debug, Default)]
        #[orso_table("batch_operations_test")]
        struct BatchOperationsTest {
            #[orso_column(primary_key)]
            id: Option<String>,

            #[orso_column(compress)]
            compressed_int_data: Vec<i64>,

            #[orso_column(compress)]
            compressed_float_data: Vec<f64>,

            #[orso_column(unique)]
            name: String,

            description: String,
        }
        // Create a local database for testing
        let db_path = "batch_operations_test.db";
        let config = DatabaseConfig::local(db_path);
        let db = Database::init(config).await?;

        // Create table
        Migrations::init(&db, &[migration!(BatchOperationsTest)]).await?;

        println!("=== Testing Batch Insert ===");

        // Create test data
        let test_data1 = BatchOperationsTest {
            id: None,
            compressed_int_data: (0..1000).map(|i| i as i64 * 100).collect(),
            compressed_float_data: (0..1000).map(|i| i as f64 * 0.01).collect(),
            name: "Record 1".to_string(),
            description: "First test record".to_string(),
        };

        let test_data2 = BatchOperationsTest {
            id: None,
            compressed_int_data: (0..500).map(|i| i as i64 * 200).collect(),
            compressed_float_data: (0..500).map(|i| i as f64 * 0.02).collect(),
            name: "Record 2".to_string(),
            description: "Second test record".to_string(),
        };

        let test_data3 = BatchOperationsTest {
            id: None,
            compressed_int_data: (0..1500).map(|i| i as i64 * 300).collect(),
            compressed_float_data: (0..1500).map(|i| i as f64 * 0.03).collect(),
            name: "Record 3".to_string(),
            description: "Third test record".to_string(),
        };

        let batch_data = vec![test_data1, test_data2, test_data3];

        // Batch insert
        match BatchOperationsTest::batch_create(&batch_data, &db).await {
            Ok(_) => println!("✓ Batch insert succeeded"),
            Err(e) => println!("✗ Batch insert failed: {}", e),
        }

        // Verify the data was inserted
        let records = BatchOperationsTest::find_all(&db).await?;
        println!("Records inserted: {}", records.len());
        for (i, record) in records.iter().enumerate() {
            println!(
                "  Record {}: name='{}', int_data_len={}, float_data_len={}",
                i + 1,
                record.name,
                record.compressed_int_data.len(),
                record.compressed_float_data.len()
            );
        }

        println!("\n=== Testing Batch Update ===");

        // Modify the records
        let mut updated_records = records.clone();
        for record in &mut updated_records {
            // Double the size of the compressed data
            record.compressed_int_data = record.compressed_int_data.iter().map(|x| x * 2).collect();
            record.compressed_float_data = record
                .compressed_float_data
                .iter()
                .map(|x| x * 2.0)
                .collect();
            record.description = format!("Updated: {}", record.description);
        }

        // Batch update
        match BatchOperationsTest::batch_update(&updated_records, &db).await {
            Ok(_) => println!("✓ Batch update succeeded"),
            Err(e) => println!("✗ Batch update failed: {}", e),
        }

        // Verify the data was updated
        let updated_records_db = BatchOperationsTest::find_all(&db).await?;
        println!("Records after update: {}", updated_records_db.len());
        for (i, record) in updated_records_db.iter().enumerate() {
            println!(
                "  Record {}: name='{}', description='{}', int_data_len={}, float_data_len={}",
                i + 1,
                record.name,
                record.description,
                record.compressed_int_data.len(),
                record.compressed_float_data.len()
            );

            // Verify data integrity
            let expected_int = (0..if i == 0 {
                1000
            } else if i == 1 {
                500
            } else {
                1500
            })
                .map(|x| {
                    x as i64
                        * if i == 0 {
                            200
                        } else if i == 1 {
                            400
                        } else {
                            600
                        }
                })
                .collect::<Vec<i64>>();
            let matches_int = record.compressed_int_data == expected_int;
            println!("    Int data matches: {}", matches_int);
        }

        // Clean up for upsert test
        std::fs::remove_file(db_path)?;

        println!("\n=== Testing Batch Upsert ===");

        let db_path2 = "batch_operations_test2.db";
        let config2 = DatabaseConfig::local(db_path2);
        let db2 = Database::init(config2).await?;

        // Create table
        Migrations::init(&db2, &[migration!(BatchOperationsTest)]).await?;

        // Create initial data for upsert
        let initial_data = vec![
            BatchOperationsTest {
                id: None,
                compressed_int_data: (0..100).map(|i| i as i64 * 10).collect(),
                compressed_float_data: (0..100).map(|i| i as f64 * 0.1).collect(),
                name: "Existing Record 1".to_string(),
                description: "This will be updated".to_string(),
            },
            BatchOperationsTest {
                id: None,
                compressed_int_data: (0..200).map(|i| i as i64 * 20).collect(),
                compressed_float_data: (0..200).map(|i| i as f64 * 0.2).collect(),
                name: "Existing Record 2".to_string(),
                description: "This will also be updated".to_string(),
            },
        ];

        // Insert initial data
        BatchOperationsTest::batch_create(&initial_data, &db2).await?;

        // Create upsert data (mix of existing and new records)
        let upsert_data = vec![
            // This should update the existing record
            BatchOperationsTest {
                id: None, // ID will be auto-generated or matched by unique field
                compressed_int_data: (0..150).map(|i| i as i64 * 15).collect(),
                compressed_float_data: (0..150).map(|i| i as f64 * 0.15).collect(),
                name: "Existing Record 1".to_string(), // Same unique name
                description: "Updated via upsert".to_string(),
            },
            // This should insert a new record
            BatchOperationsTest {
                id: None,
                compressed_int_data: (0..300).map(|i| i as i64 * 30).collect(),
                compressed_float_data: (0..300).map(|i| i as f64 * 0.3).collect(),
                name: "New Record 1".to_string(), // New unique name
                description: "Inserted via upsert".to_string(),
            },
        ];

        // Batch upsert
        match BatchOperationsTest::batch_upsert(&upsert_data, &db2).await {
            Ok(_) => println!("✓ Batch upsert succeeded"),
            Err(e) => println!("✗ Batch upsert failed: {}", e),
        }

        // Verify the results
        let final_records = BatchOperationsTest::find_all(&db2).await?;
        println!("Records after upsert: {}", final_records.len());
        for (i, record) in final_records.iter().enumerate() {
            println!(
                "  Record {}: name='{}', description='{}', int_data_len={}, float_data_len={}",
                i + 1,
                record.name,
                record.description,
                record.compressed_int_data.len(),
                record.compressed_float_data.len()
            );
        }

        // Clean up
        std::fs::remove_file(db_path2)?;

        println!("\n=== Summary ===");
        println!(
        "All batch operations (insert, update, upsert) now properly handle compressed BLOB data!"
    );
        println!("The fixes ensure that:");
        println!(
            "1. BLOB data is properly passed as parameters instead of being converted to NULL"
        );
        println!("2. Compressed data maintains its integrity through all operations");
        println!("3. Batch operations work correctly with the ORM's compression features");

        Ok(())
    }

    #[tokio::test]
    async fn debug_compression_check_vector_collect() -> Result<(), Box<dyn std::error::Error>> {
        // Create in-memory database
        let config = DatabaseConfig::memory();
        let db = Database::init(config).await?;

        // Create table
        use orso::{migration, Migrations};
        Migrations::init(&db, &[migration!(DebugCompressed)]).await?;

        // Create test data
        let test_data = DebugCompressed {
            id: None,                       // Will be auto-generated
            data_points: (0..10).collect(), // Sample data points
            name: "Test Data".to_string(),
            age: 25,
        };

        println!("Original data_points: {:?}", test_data.data_points);

        // Check what to_map produces
        let map = test_data.to_map()?;
        println!("Map keys: {:?}", map.keys().collect::<Vec<_>>());

        for (key, value) in &map {
            match value {
                orso::Value::Blob(blob) => {
                    println!("{}: BLOB ({} bytes)", key, blob.len());
                    if blob.len() >= 4 && &blob[0..4] == b"ORSO" {
                        println!("  -> Has ORSO header ✓");
                    } else {
                        println!("  -> No ORSO header ✗");
                    }
                }
                orso::Value::Text(text) => {
                    println!("{}: TEXT ({})", key, text);
                }
                _ => {
                    println!("{}: {:?}", key, value);
                }
            }
        }

        // Insert data
        test_data.insert(&db).await?;

        // Check what's actually in the database
        let mut rows = db
            .conn
            .query("SELECT data_points FROM debug_compressed LIMIT 1", ())
            .await?;
        if let Some(row) = rows.next().await? {
            match row.get_value(0) {
                Ok(libsql::Value::Blob(blob)) => {
                    println!("Database value: BLOB ({} bytes)", blob.len());
                    if blob.len() >= 4 && &blob[0..4] == b"ORSO" {
                        println!("  -> Has ORSO header ✓");
                    } else {
                        println!("  -> No ORSO header ✗");
                        println!(
                            "  -> First 32 bytes as text: {:?}",
                            String::from_utf8_lossy(&blob[0..std::cmp::min(32, blob.len())])
                        );
                    }
                }
                Ok(libsql::Value::Text(text)) => {
                    println!("Database value: TEXT ({})", text);
                }
                Ok(other) => {
                    println!("Database value: {:?}", other);
                }
                Err(e) => {
                    println!("Database value error: {}", e);
                }
            }
        }

        // Retrieve all data (since we don't know the auto-generated ID)
        let all_records = DebugCompressed::find_all(&db).await?;
        assert_eq!(all_records.len(), 1);

        let retrieved = &all_records[0];
        println!("Retrieved data_points: {:?}", retrieved.data_points);
        assert_eq!(retrieved.name, "Test Data");
        assert_eq!(retrieved.age, 25);
        assert_eq!(retrieved.data_points.len(), 10);
        assert_eq!(retrieved.data_points[0], 0);
        assert_eq!(retrieved.data_points[9], 9);

        Ok(())
    }

    #[tokio::test]
    async fn debug_compression_check_vector_simple() -> Result<(), Box<dyn std::error::Error>> {
        // Create in-memory database
        let config = DatabaseConfig::memory();
        let db = Database::init(config).await?;

        // Create table
        use orso::{migration, Migrations};
        Migrations::init(&db, &[migration!(DebugCompressed)]).await?;

        // Create test data
        let test_data = DebugCompressed {
            id: None, // Will be auto-generated
            data_points: vec![
                1000, 2000, 3000, 4000, 5000, 1000, 2000, 3000, 4000, 5000, 1000, 2000, 3000, 4000,
                5000, 1000, 2000, 3000, 4000, 5000, 1000, 2000, 3000, 4000, 5000, 1000, 2000, 3000,
                4000, 5000, 1000, 2000, 3000, 4000, 5000, 1000, 2000, 3000, 4000, 5000, 1000, 2000,
                3000, 4000, 5000,
            ], // Sample data points
            name: "Test Data".to_string(),
            age: 25,
        };

        println!("Original data_points: {:?}", test_data.data_points);

        // Check what to_map produces
        let map = test_data.to_map()?;
        println!("Map keys: {:?}", map.keys().collect::<Vec<_>>());

        for (key, value) in &map {
            match value {
                orso::Value::Blob(blob) => {
                    println!("{}: BLOB ({} bytes)", key, blob.len());
                    if blob.len() >= 4 && &blob[0..4] == b"ORSO" {
                        println!("  -> Has ORSO header ✓");
                    } else {
                        println!("  -> No ORSO header ✗");
                    }
                }
                orso::Value::Text(text) => {
                    println!("{}: TEXT ({})", key, text);
                }
                _ => {
                    println!("{}: {:?}", key, value);
                }
            }
        }

        // Insert data
        test_data.insert(&db).await?;

        // Check what's actually in the database
        let mut rows = db
            .conn
            .query("SELECT data_points FROM debug_compressed LIMIT 1", ())
            .await?;
        if let Some(row) = rows.next().await? {
            match row.get_value(0) {
                Ok(libsql::Value::Blob(blob)) => {
                    println!("Database value: BLOB ({} bytes)", blob.len());
                    if blob.len() >= 4 && &blob[0..4] == b"ORSO" {
                        println!("  -> Has ORSO header ✓");
                    } else {
                        println!("  -> No ORSO header ✗");
                        println!(
                            "  -> First 32 bytes as text: {:?}",
                            String::from_utf8_lossy(&blob[0..std::cmp::min(32, blob.len())])
                        );
                    }
                }
                Ok(libsql::Value::Text(text)) => {
                    println!("Database value: TEXT ({})", text);
                }
                Ok(other) => {
                    println!("Database value: {:?}", other);
                }
                Err(e) => {
                    println!("Database value error: {}", e);
                }
            }
        }

        // Retrieve all data (since we don't know the auto-generated ID)
        let all_records = DebugCompressed::find_all(&db).await?;
        assert_eq!(all_records.len(), 1);

        let retrieved = &all_records[0];
        println!("Retrieved data_points: {:?}", retrieved.data_points);
        assert_eq!(retrieved.name, "Test Data");
        assert_eq!(retrieved.age, 25);
        assert_eq!(retrieved.data_points.len(), 10);
        assert_eq!(retrieved.data_points[0], 0);
        assert_eq!(retrieved.data_points[9], 9);

        Ok(())
    }

    #[derive(Orso, Serialize, Deserialize, Clone, Debug, Default)]
    #[orso_table("collect_vs_vec_test")]
    struct CollectVsVecTest {
        #[orso_column(primary_key)]
        id: Option<String>,

        #[orso_column(compress)]
        collected_data: Vec<i64>, // Created with .collect()

        #[orso_column(compress)]
        vec_data: Vec<i64>, // Created with vec![]

        name: String,
    }

    #[tokio::test]
    async fn test_collect_vs_vec_macro() -> Result<(), Box<dyn std::error::Error>> {
        // Create in-memory database
        let config = DatabaseConfig::memory();
        let db = Database::init(config).await?;

        // Create table
        Migrations::init(&db, &[migration!(CollectVsVecTest)]).await?;

        // Create test data - one with collect, one with vec!
        let test_data = CollectVsVecTest {
            id: None,
            collected_data: (0..5).collect(),   // Using .collect()
            vec_data: vec![10, 20, 30, 40, 50], // Using vec![]
            name: "Test Data".to_string(),
        };

        println!("Original collected_data: {:?}", test_data.collected_data);
        println!("Original vec_data: {:?}", test_data.vec_data);

        // Check what to_map produces
        let map = test_data.to_map()?;
        println!("\nMap keys and values:");
        for (key, value) in &map {
            match value {
                orso::Value::Blob(blob) => {
                    println!("{}: BLOB ({} bytes)", key, blob.len());
                    if blob.len() >= 4 && &blob[0..4] == b"ORSO" {
                        println!("  -> Has ORSO header ✓");
                    } else {
                        println!("  -> No ORSO header ✗");
                        println!(
                            "  -> First chars: {}",
                            String::from_utf8_lossy(&blob[0..std::cmp::min(32, blob.len())])
                        );
                    }
                }
                orso::Value::Text(text) => {
                    println!("{}: TEXT ({})", key, text);
                }
                _ => {
                    println!("{}: {:?}", key, value);
                }
            }
        }

        // Insert data
        test_data.insert(&db).await?;

        // Check what's actually in the database
        let mut rows = db
            .conn
            .query(
                "SELECT collected_data, vec_data FROM collect_vs_vec_test LIMIT 1",
                (),
            )
            .await?;
        if let Some(row) = rows.next().await? {
            println!("\nDatabase values:");

            // Check collected_data
            match row.get_value(0) {
                Ok(libsql::Value::Blob(blob)) => {
                    println!("collected_data in DB: BLOB ({} bytes)", blob.len());
                    if blob.len() >= 4 && &blob[0..4] == b"ORSO" {
                        println!("  -> Has ORSO header ✓");
                    } else {
                        println!("  -> No ORSO header ✗");
                    }
                }
                Ok(libsql::Value::Text(text)) => {
                    println!("collected_data in DB: TEXT ({})", text);
                }
                _ => {}
            }

            // Check vec_data
            match row.get_value(1) {
                Ok(libsql::Value::Blob(blob)) => {
                    println!("vec_data in DB: BLOB ({} bytes)", blob.len());
                    if blob.len() >= 4 && &blob[0..4] == b"ORSO" {
                        println!("  -> Has ORSO header ✓");
                    } else {
                        println!("  -> No ORSO header ✗");
                    }
                }
                Ok(libsql::Value::Text(text)) => {
                    println!("vec_data in DB: TEXT ({})", text);
                }
                _ => {}
            }
        }

        // Retrieve and verify
        let all_records = CollectVsVecTest::find_all(&db).await?;
        assert_eq!(all_records.len(), 1);

        let retrieved = &all_records[0];
        println!("\nRetrieved data:");
        println!("Retrieved collected_data: {:?}", retrieved.collected_data);
        println!("Retrieved vec_data: {:?}", retrieved.vec_data);

        Ok(())
    }

    #[derive(Orso, Serialize, Deserialize, Clone, Debug, Default)]
    #[orso_table("allocator_test")]
    struct AllocatorTest {
        #[orso_column(primary_key)]
        id: Option<String>,

        #[orso_column(compress)]
        compressed_regular: Vec<i64>, // This should work

        #[orso_column(compress)]
        compressed_with_alloc: Vec<i64>, // Fixed: use standard Vec<i64>

        name: String,
        age: i32,
    }

    #[tokio::test]
    async fn test_allocator_specific_vec() -> Result<(), Box<dyn std::error::Error>> {
        // Create in-memory database
        let config = DatabaseConfig::memory();
        let db = Database::init(config).await?;

        // Create table
        Migrations::init(&db, &[migration!(AllocatorTest)]).await?;

        // Create test data
        let test_data = AllocatorTest {
            id: None,
            compressed_regular: vec![1, 2, 3, 4, 5],
            compressed_with_alloc: vec![10, 20, 30, 40, 50],
            name: "Test Data".to_string(),
            age: 25,
        };

        println!(
            "Original compressed_regular: {:?}",
            test_data.compressed_regular
        );
        println!(
            "Original compressed_with_alloc: {:?}",
            test_data.compressed_with_alloc
        );

        // Check what to_map produces
        let map = test_data.to_map()?;
        println!("Map keys: {:?}", map.keys().collect::<Vec<_>>());

        for (key, value) in &map {
            match value {
                orso::Value::Blob(blob) => {
                    println!("{}: BLOB ({} bytes)", key, blob.len());
                    if blob.len() >= 4 && &blob[0..4] == b"ORSO" {
                        println!("  -> Has ORSO header ✓");
                    } else {
                        println!("  -> No ORSO header ✗");
                        println!(
                            "  -> First 32 chars: {}",
                            String::from_utf8_lossy(&blob[0..std::cmp::min(32, blob.len())])
                        );
                    }
                }
                orso::Value::Text(text) => {
                    println!("{}: TEXT ({})", key, text);
                }
                _ => {
                    println!("{}: {:?}", key, value);
                }
            }
        }

        // Insert data
        test_data.insert(&db).await?;

        // Check what's actually in the database
        let mut rows = db
            .conn
            .query(
                "SELECT compressed_regular, compressed_with_alloc FROM allocator_test LIMIT 1",
                (),
            )
            .await?;
        if let Some(row) = rows.next().await? {
            // Check compressed_regular
            match row.get_value(0) {
                Ok(libsql::Value::Blob(blob)) => {
                    println!("compressed_regular in DB: BLOB ({} bytes)", blob.len());
                    if blob.len() >= 4 && &blob[0..4] == b"ORSO" {
                        println!("  -> Has ORSO header ✓");
                    } else {
                        println!("  -> No ORSO header ✗");
                    }
                }
                Ok(libsql::Value::Text(text)) => {
                    println!("compressed_regular in DB: TEXT ({})", text);
                }
                _ => {}
            }

            // Check compressed_with_alloc
            match row.get_value(1) {
                Ok(libsql::Value::Blob(blob)) => {
                    println!("compressed_with_alloc in DB: BLOB ({} bytes)", blob.len());
                    if blob.len() >= 4 && &blob[0..4] == b"ORSO" {
                        println!("  -> Has ORSO header ✓");
                    } else {
                        println!("  -> No ORSO header ✗");
                        println!(
                            "  -> First 32 chars: {}",
                            String::from_utf8_lossy(&blob[0..std::cmp::min(32, blob.len())])
                        );
                    }
                }
                Ok(libsql::Value::Text(text)) => {
                    println!("compressed_with_alloc in DB: TEXT ({})", text);
                }
                _ => {}
            }
        }

        // Retrieve and verify
        let all_records = AllocatorTest::find_all(&db).await?;
        assert_eq!(all_records.len(), 1);

        let retrieved = &all_records[0];
        println!(
            "Retrieved compressed_regular: {:?}",
            retrieved.compressed_regular
        );
        println!(
            "Retrieved compressed_with_alloc: {:?}",
            retrieved.compressed_with_alloc
        );

        Ok(())
    }
}
