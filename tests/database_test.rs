use orso::{Database, DatabaseConfig, TursoMode};

#[tokio::test]
async fn test_database_config_creation() {
    // Test local config
    let local_config = DatabaseConfig::local("test.db");
    assert_eq!(local_config.mode, TursoMode::Local);
    assert_eq!(local_config.local_db_path, "test.db");

    // Test remote config
    let remote_config = DatabaseConfig::remote("http://example.com", "token");
    assert_eq!(remote_config.mode, TursoMode::Remote);
    assert_eq!(remote_config.db_url, "http://example.com");
    assert_eq!(remote_config.db_token, "token");

    // Test sync config
    let sync_config = DatabaseConfig::sync("local.db", "http://example.com", "token");
    assert_eq!(sync_config.mode, TursoMode::Sync);
    assert_eq!(sync_config.local_db_path, "local.db");
    assert_eq!(sync_config.db_url, "http://example.com");
    assert_eq!(sync_config.db_token, "token");

    // Test embed config
    let embed_config = DatabaseConfig::embed("replica.db", "http://example.com", "token");
    assert_eq!(embed_config.mode, TursoMode::Embed);
    assert_eq!(embed_config.local_db_path, "replica.db");
    assert_eq!(embed_config.db_url, "http://example.com");
    assert_eq!(embed_config.db_token, "token");

    println!("All database config creation tests passed");
}

#[tokio::test]
async fn test_database_query_execution() {
    let config = DatabaseConfig::memory();
    let turso = Database::init(config)
        .await
        .expect("Failed to initialize database");

    // Test simple query execution
    let result = turso
        .conn
        .execute(
            "CREATE TABLE IF NOT EXISTS test_query_table (id INTEGER PRIMARY KEY, name TEXT)",
            (),
        )
        .await;
    assert!(result.is_ok());

    // Test inserting data
    let insert_result = turso
        .conn
        .execute(
            "INSERT INTO test_query_table (name) VALUES ('test_name')",
            (),
        )
        .await;
    assert!(insert_result.is_ok());

    // Test querying data
    let rows = turso
        .conn
        .query(
            "SELECT * FROM test_query_table",
            Vec::<libsql::Value>::new(),
        )
        .await;
    assert!(rows.is_ok());

    println!("Database query execution tests passed");
}
