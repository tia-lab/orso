# ORSO

ORSO is a Rust ORM (Object-Relational Mapping) library for working with SQLite and Turso databases. It provides a straightforward way to define database schemas using Rust structs and perform common database operations.

## Features

- **Derive-based schema definition**: Use `#[derive(Orso)]` to automatically generate database schema from Rust structs
- **Multiple database modes**: Support for local SQLite, remote Turso, sync, and embedded modes
- **Automatic schema management**: Generate SQL schema and handle migrations
- **CRUD operations**: insert, read, update, and delete records
- **Batch operations**: Efficient handling of multiple records
- **Query building**: Flexible query construction with filtering and sorting
- **Pagination**: Support for paginated results
- **Foreign key relationships**: Define relationships between tables
- **Type mapping**: Automatic conversion between Rust types and database types
- **Utility operations**: Existence checks, field-based queries, latest/first record finding, and batch ID operations
- **Runtime table selection**: Use `_with_table` methods to work with multiple tables using the same struct

## Installation

Add ORSO to your `Cargo.toml`:

```bash
cargo add orso
```

## Quick Start

### 1. Define Your Model

```rust
use orso::{Orso, orso_table};
use serde::{Deserialize, Serialize};

#[derive(Orso, Serialize, Deserialize, Clone, Default, Debug)]
#[orso_table("users")]
pub struct User {
    pub name: String,
    pub email: String,
    pub age: i32,
}
```

### 2. Initialize Database Connection

```rust
use orso::database::{Database, DatabaseConfig};

// Local SQLite database
let config = DatabaseConfig::local("app.sqlite");
let db = Database::init(config).await?;
```

### 3. Run Migrations

```rust
use orso::{Migrations, migration};

// Create tables automatically with default config
Migrations::init(&db, &[migration!(User)]).await?;

// Or with custom migration config
use orso::MigrationConfig;

let config = MigrationConfig {
    max_backups_per_table: Some(3),     // Keep max 3 migration backups per table
    backup_retention_days: Some(7),     // Delete backups older than 7 days
    backup_suffix: Some("backup".to_string()), // Use "backup" instead of "migration"
};

Migrations::init_with_config(&db, &[migration!(User)], &config).await?;
```

**Custom Table Names**: Override the default table name when you need multiple tables with the same schema:

```rust
// Use default table name from struct
migration!(User)  // Creates "users" table

// Override with custom table name
migration!(User, "users_archive")  // Creates "users_archive" table
migration!(User, "users_backup")   // Creates "users_backup" table
```

**Migration Safety & Backup Management**: ORSO automatically manages migration backups with zero data loss:

- **Zero-loss migrations**: Original data is always backed up before schema changes
- **Smart cleanup**: Automatically removes old migration tables based on count and age
- **Configurable retention**: Control how many backups to keep and for how long
- **Clear naming**: Migration tables use `_migration_` suffix for clarity (e.g., `users_migration_1234567890`)

### 4. Perform CRUD Operations

```rust
// Create
let user = User {
    name: "John Doe".to_string(),
    email: "john@example.com".to_string(),
    age: 30,
};
user.insert(&db).await?;

// Read
let user = User::find_by_id("user-uuid", &db).await?;
let all_users = User::find_all(&db).await?;
let count = User::count(&db).await?;

// Update
if let Some(mut user) = User::find_by_id("user-uuid", &db).await? {
    user.age = 31;
    user.update(&db).await?;
}

// Delete
if let Some(user) = User::find_by_id("user-uuid", &db).await? {
    user.delete(&db).await?;
}
```

### 5. Use New Utility Methods

```rust
use orso::{filter, filter_op, Value};

// Existence checks (very efficient - returns bool without fetching data)
let has_users = User::exists(&db).await?;
let has_adults = User::exists_filter(
    filter_op!(filter!("age", orso::Operator::Ge, 18)),
    &db
).await?;

// Find by any field
let johns = User::find_by_field("name", Value::Text("John".to_string()), &db).await?;
let gmail_users = User::find_by_field("email", Value::Text("gmail.com".to_string()), &db).await?;

// Find latest/first records with filters
let filter = filter_op!(filter!("age", orso::Operator::Gt, 25));
let latest_adult = User::find_latest_filter(filter.clone(), &db).await?;
let first_adult = User::find_first_filter(filter, &db).await?;

// Find latest/first by specific field
let latest_john = User::find_latest_by_field("name", Value::Text("John".to_string()), &db).await?;

// Batch operations for performance
let user_ids = vec!["id1", "id2", "id3"];
let users = User::find_by_ids(&user_ids, &db).await?;

let ages = vec![Value::Integer(25), Value::Integer(30), Value::Integer(35)];
let specific_ages = User::find_by_field_in("age", &ages, &db).await?;

println!("Found {} users with specific ages", specific_ages.len());
```

## Custom Table Operations (`_with_table` methods)

All CRUD operations have `_with_table` variants that allow you to specify a custom table name at runtime, enabling one struct to work with multiple tables:

```rust
// Create in custom table
user.insert_with_table(&db, "users_archive").await?;

// Read from custom table
let user = User::find_by_id_with_table("user-uuid", &db, "users_archive").await?;
let all_users = User::find_all_with_table(&db, "users_backup").await?;

// Update in custom table
user.update_with_table(&db, "users_temp").await?;

// Delete from custom table
user.delete_with_table(&db, "users_old").await?;

// Count in custom table
let count = User::count_with_table(&db, "users_archive").await?;
```

### Complete Example: Multiple Tables from One Struct

Here's a practical example of using one struct to insert and manage multiple tables:

```rust
use orso::{Orso, Database, DatabaseConfig, Migrations, migration};
use serde::{Deserialize, Serialize};

#[derive(Orso, Serialize, Deserialize, Clone, Default, Debug)]
#[orso_table("indicators")]
pub struct IndicatorsData {
    pub symbol: String,
    pub price: f64,
    pub volume: i64,
    pub rsi: f64,
    pub macd: f64,
    pub timestamp: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = DatabaseConfig::local("trading.db");
    let db = Database::init(config).await?;

    // Create 3 different tables from the same struct
    Migrations::init(&db, &[
        migration!(IndicatorsData, "indicators_1h"),  // 1-hour timeframe
        migration!(IndicatorsData, "indicators_4h"),  // 4-hour timeframe
        migration!(IndicatorsData, "indicators_1d"),  // 1-day timeframe
    ]).await?;

    // Sample data
    let btc_1h = IndicatorsData {
        symbol: "BTC".to_string(),
        price: 45000.0,
        volume: 1000000,
        rsi: 65.5,
        macd: 120.0,
        timestamp: "2024-01-15T10:00:00Z".to_string(),
    };

    let btc_4h = IndicatorsData {
        symbol: "BTC".to_string(),
        price: 44800.0,
        volume: 4000000,
        rsi: 62.3,
        macd: 95.0,
        timestamp: "2024-01-15T08:00:00Z".to_string(),
    };

    let btc_1d = IndicatorsData {
        symbol: "BTC".to_string(),
        price: 44500.0,
        volume: 24000000,
        rsi: 58.7,
        macd: 75.0,
        timestamp: "2024-01-15T00:00:00Z".to_string(),
    };

    // Insert data into different tables using the same struct
    btc_1h.insert_with_table(&db, "indicators_1h").await?;
    btc_4h.insert_with_table(&db, "indicators_4h").await?;
    btc_1d.insert_with_table(&db, "indicators_1d").await?;

    // Query data from different timeframes
    let hourly_data = IndicatorsData::find_all_with_table(&db, "indicators_1h").await?;
    let four_hour_data = IndicatorsData::find_all_with_table(&db, "indicators_4h").await?;
    let daily_data = IndicatorsData::find_all_with_table(&db, "indicators_1d").await?;

    println!("1-hour indicators: {} records", hourly_data.len());
    println!("4-hour indicators: {} records", four_hour_data.len());
    println!("Daily indicators: {} records", daily_data.len());

    // Use filtering on specific tables
    use orso::{filter, filter_op};
    let filter = filter_op!(filter!("rsi", crate::Operator::Gt, 60.0));

    let overbought_1h = IndicatorsData::find_where_with_table(filter.clone(), &db, "indicators_1h").await?;
    let overbought_4h = IndicatorsData::find_where_with_table(filter.clone(), &db, "indicators_4h").await?;
    let overbought_1d = IndicatorsData::find_where_with_table(filter, &db, "indicators_1d").await?;

    println!("Overbought conditions:");
    println!("  1H: {} symbols", overbought_1h.len());
    println!("  4H: {} symbols", overbought_4h.len());
    println!("  1D: {} symbols", overbought_1d.len());

    // Batch operations on specific tables
    let more_data = vec![/* ... more IndicatorsData instances ... */];
    IndicatorsData::batch_insert_with_table(&more_data, &db, "indicators_1h").await?;

    Ok(())
}
```

### Available `_with_table` Methods

All standard operations have `_with_table` variants:

**CRUD Operations:**

- `insert_with_table(&self, db, table_name)`
- `find_by_id_with_table(id, db, table_name)`
- `find_all_with_table(db, table_name)`
- `find_where_with_table(filter, db, table_name)`
- `update_with_table(&self, db, table_name)`
- `delete_with_table(&self, db, table_name)`

**Advanced Operations:**

- `insert_or_update_with_table(&self, db, table_name)`
- `upsert_with_table(&self, db, table_name)`
- `count_with_table(db, table_name)`
- `count_where_with_table(filter, db, table_name)`

**Batch Operations:**

- `batch_insert_with_table(models, db, table_name)`
- `batch_update_with_table(models, db, table_name)`
- `batch_delete_with_table(ids, db, table_name)`
- `batch_upsert_with_table(models, db, table_name)`

**Query Operations:**

- `find_one_with_table(filter, db, table_name)`
- `find_latest_with_table(db, table_name)`
- `find_paginated_with_table(pagination, db, table_name)`
- `find_where_paginated_with_table(filter, pagination, db, table_name)`
- `search_with_table(search_filter, pagination, db, table_name)`
- `list_with_table(sort, pagination, db, table_name)`
- `list_where_with_table(filter, sort, pagination, db, table_name)`
- `delete_where_with_table(filter, db, table_name)`
- `aggregate_with_table(function, column, filter, db, table_name)`

**Utility Operations (New!):**

- `exists_with_table(db, table_name)` - Check if any records exist
- `exists_filter_with_table(filter, db, table_name)` - Check if filtered records exist
- `find_latest_filter_with_table(filter, db, table_name)` - Find latest record matching filter
- `find_first_filter_with_table(filter, db, table_name)` - Find oldest record matching filter
- `find_by_field_with_table(field, value, db, table_name)` - Find records by any field
- `find_latest_by_field_with_table(field, value, db, table_name)` - Find latest record by field
- `find_first_by_field_with_table(field, value, db, table_name)` - Find oldest record by field
- `find_by_ids_with_table(ids, db, table_name)` - Batch find by multiple IDs
- `find_by_field_in_with_table(field, values, db, table_name)` - Find by multiple field values

## Utility Operations in Action

These new utility methods make common database patterns much simpler:

### Before vs After

**Finding Latest Record by Field (Your Use Case):**

```rust
// Before: Manual filter construction
let filter = filter_op!(filter!("pair", orso::Operator::Eq, "BTCUSDT"));
let record = TableIndicatorsRegime::find_latest_filter_with_table(filter, &db, &table_name).await?;

// After: Direct field query
let record = TableIndicatorsRegime::find_latest_by_field_with_table(
    "pair",
    Value::Text("BTCUSDT".to_string()),
    &db,
    &table_name
).await?;
```

**Checking if Data Exists:**

```rust
// Before: Fetch and check length
let users = User::find_all(&db).await?;
let exists = !users.is_empty();

// After: Efficient existence check
let exists = User::exists(&db).await?;
```

**Batch Finding by IDs:**

```rust
// Before: Multiple individual queries
let mut users = Vec::new();
for id in ["id1", "id2", "id3"] {
    if let Some(user) = User::find_by_id(id, &db).await? {
        users.push(user);
    }
}

// After: Single batch query
let users = User::find_by_ids(&["id1", "id2", "id3"], &db).await?;
```

### Real-World Use Cases

**Financial Data Processing:**

```rust
// Check if we have today's data
let today_filter = filter_op!(filter!("date", orso::Operator::Eq, today));
let has_todays_data = PriceData::exists_filter(&today_filter, &db).await?;

// Get latest price for each symbol
let symbols = vec!["BTCUSDT", "ETHUSDT", "ADAUSDT"];
for symbol in symbols {
    let latest_price = PriceData::find_latest_by_field(
        "symbol",
        Value::Text(symbol.to_string()),
        &db
    ).await?;
    println!("{}: {:?}", symbol, latest_price);
}
```

**User Management:**

```rust
// Find all users from specific domains
let domains = vec![
    Value::Text("gmail.com".to_string()),
    Value::Text("company.com".to_string())
];
let users = User::find_by_field_in("email_domain", &domains, &db).await?;

// Check if any admin users exist
let admin_filter = filter_op!(filter!("role", orso::Operator::Eq, "admin"));
let has_admins = User::exists_filter(&admin_filter, &db).await?;
```

## Database Connection Modes

ORSO supports different database connection modes:

```rust
use orso::database::{Database, DatabaseConfig};

// Local SQLite file
let local_config = DatabaseConfig::local("local.db");

// Remote Turso database
let remote_config = DatabaseConfig::remote(
    "libsql://your-database.turso.io",
    "your-auth-token"
);

// Local database with sync to Turso
let sync_config = DatabaseConfig::sync(
    "local.db",
    "libsql://your-database.turso.io",
    "your-auth-token"
);

// Embedded replica with remote sync
let embed_config = DatabaseConfig::embed(
    "replica.db",
    "libsql://your-database.turso.io",
    "your-auth-token"
);

let db = Database::init(config).await?;
```

## Schema Definition

Define your database schema using Rust structs with the `Orso` derive macro:

### Basic Fields

```rust
#[derive(Orso, Serialize, Deserialize, Clone, Default, Debug)]
#[orso_table("products")]
pub struct Product {
    pub name: String,
    pub price: f64,
    pub in_stock: bool,
    pub description: Option<String>, // Nullable field
}
```

### Column Attributes

```rust
#[derive(Orso, Serialize, Deserialize, Clone, Default, Debug)]
#[orso_table("users")]
pub struct User {
    #[orso_column(primary_key)]
    pub id: String, // Primary key

    #[orso_column(unique)]
    pub email: String, // Unique constraint

    #[orso_column(ref = "categories")]
    pub category_id: String, // Foreign key reference

    #[orso_column(created_at)]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>, // Auto-managed timestamp

    #[orso_column(updated_at)]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>, // Auto-managed timestamp
}
```

## Migrations

ORSO provides automatic zero-loss migrations with smart backup management:

### Basic Migration Setup

```rust
use orso::{Migrations, migration};

// Initialize multiple tables with default settings
Migrations::init(&db, &[
    migration!(User),
    migration!(Product),
    migration!(Order)
]).await?;
```

### Advanced Migration Configuration

```rust
use orso::{Migrations, migration, MigrationConfig};

// Custom migration configuration
let config = MigrationConfig {
    max_backups_per_table: Some(5),      // Keep max 5 migration tables per original table
    backup_retention_days: Some(30),     // Delete migration tables older than 30 days
    backup_suffix: Some("migration".to_string()), // Suffix for migration table names
};

// Apply custom config to migrations
Migrations::init_with_config(&db, &[
    migration!(User),
    migration!(Product, "products_v2"),  // Custom table name
], &config).await?;
```

### Migration Configuration Options

| Option                  | Default       | Description                                                           |
| ----------------------- | ------------- | --------------------------------------------------------------------- |
| `max_backups_per_table` | `5`           | Maximum number of migration backup tables to keep per original table  |
| `backup_retention_days` | `30`          | Delete migration tables older than this many days                     |
| `backup_suffix`         | `"migration"` | Suffix used in migration table names (e.g., `table_migration_123456`) |

### Zero-Loss Migration Process

When ORSO detects schema changes, it automatically:

1. **Analyzes** the current vs expected schema
2. **Creates** a temporary table with the new schema
3. **Migrates** all data from the original table (preserving row order)
4. **Renames** the original table to `{table}_migration_{timestamp}`
5. **Renames** the temporary table to the original name
6. **Cleans up** old migration tables based on your retention policy
7. **Verifies** migration success

### Migration Table Examples

```sql
-- Original table
users

-- After migration (backup created)
users                    -- New schema
users_migration_1234567890  -- Backup with original data

-- After multiple migrations (with cleanup)
users                    -- Current table
users_migration_1234567890  -- Recent backup
users_migration_1234567891  -- Most recent backup
-- older backups automatically cleaned up
```

## Querying Data

### Basic Queries

```rust
// Find all records
let users = User::find_all(&db).await?;

// Find by ID
let user = User::find_by_id("user-id", &db).await?;

// Find with filters
use orso::{filter, filter_op};

let filter = filter_op!(filter!("age", crate::Operator::Eq, 25));
let users = User::find_where(filter, &db).await?;
```

### Complex Filtering

```rust
use orso::{filter, filter_op};

// AND conditions
let and_filter = filter_op!(and,
    filter!("age", crate::Operator::Ge, 18),
    filter!("email", crate::Operator::Like, "%@company.com")
);

// OR conditions
let or_filter = filter_op!(or,
    filter!("role", crate::Operator::Eq, "admin"),
    filter!("role", crate::Operator::Eq, "moderator")
);

// NOT conditions
let not_filter = filter_op!(not, filter!("status", crate::Operator::Eq, "inactive"));

let users = User::find_where(and_filter, &db).await?;
```

### Query Builder

```rust
use orso::{query, filter, filter_op, sort};

// Basic query with sorting and limits
let results = query!("users")
    .select(vec!["name", "email"])
    ._where(filter_op!(filter!("age", crate::Operator::Ge, 18)))
    .order_by(sort!("name", asc))
    .limit(10)
    .execute::<User>(&db)
    .await?;

// Aggregation queries
let count = query!("users")
    .select_count()
    .execute_count(&db)
    .await?;

// Group by queries
let results = query!("users")
    .select(vec!["department", "COUNT(*) as employee_count"])
    .group_by(vec!["department"])
    .execute::<EmployeeCount>(&db)
    .await?;
```

## Batch Operations

For better performance with multiple records:

```rust
// Batch insert
let users = vec![user1, user2, user3];
User::batch_create(&users, &db).await?;

// Batch update
User::batch_update(&users, &db).await?;

// Batch delete
let ids = vec!["id1", "id2", "id3"];
User::batch_delete(&ids, &db).await?;
```

## Pagination

ORSO provides built-in pagination support:

```rust
use orso::{pagination, sort, query, filter};

// Offset-based pagination
let pagination = pagination!(1, 20); // Page 1, 20 items per page
let results = User::find_paginated(&pagination, &db).await?;

// Paginated queries with filtering
let filter = filter!("active", crate::Operator::Eq, true);
let results = User::find_where_paginated(filter, &pagination, &db).await?;

// Using query builder with pagination
let results = query!("users")
    .order_by(sort!("name", asc))
    .execute_paginated::<User>(&db, &pagination)
    .await?;
```

## Convenience Macros

ORSO provides several convenience macros for common operations:

```rust
use orso::{filter, filter_op, sort, pagination, query, search};

// Filter creation
let eq_filter = filter!("age", crate::Operator::Eq, 25);
let gt_filter = filter!("age", crate::Operator::Gt, 18);
let in_filter = filter!("status", in, vec!["active", "pending"]);
let between_filter = filter!("age", between, 18, 65);
let null_filter = filter!("email", is_null);
let not_null_filter = filter!("email", is_not_null);

// Sorting
let sort_asc = sort!("name", asc);
let sort_desc = sort!("created_at", desc);
let sort_default = sort!("name"); // defaults to ascending

// Pagination
let pagination = pagination!(1, 20); // page 1, 20 items per page
let default_pagination = pagination!(1); // page 1, 20 items per page (default)

// Query building
let query_builder = query!("users");

// Filter operations (combining filters)
let and_filter = filter_op!(and, eq_filter, gt_filter);
let or_filter = filter_op!(or, eq_filter, in_filter);
let not_filter = filter_op!(not, null_filter);
let single_filter = filter_op!(eq_filter); // single filter

// Search filters
let search_filter = search!("john", "name", "email");
```

## Supported Operators

ORSO provides various operators for filtering:

```rust
use orso::Operator;

// Equality operators
Operator::Eq      // Equal (=)
Operator::Ne      // Not equal (!=)
Operator::Lt      // Less than (<)
Operator::Le      // Less than or equal (<=)
Operator::Gt      // Greater than (>)
Operator::Ge      // Greater than or equal (>=)

// Pattern matching
Operator::Like    // LIKE
Operator::NotLike // NOT LIKE

// Set operators
Operator::In      // IN
Operator::NotIn   // NOT IN

// Null checks
Operator::IsNull    // IS NULL
Operator::IsNotNull // IS NOT NULL

// Range operators
Operator::Between    // BETWEEN
Operator::NotBetween // NOT BETWEEN
```

## Error Handling

ORSO provides comprehensive error handling:

```rust
use orso::{Error, Result};

match User::find_by_id("user-id", &db).await {
    Ok(Some(user)) => println!("Found user: {}", user.name),
    Ok(None) => println!("User not found"),
    Err(Error::DatabaseError(msg)) => eprintln!("Database error: {}", msg),
    Err(Error::Connection(err)) => eprintln!("Connection error: {}", err),
    Err(e) => eprintln!("Other error: {}", e),
}
```

## Supported Data Types

ORSO maps Rust types to SQLite types:

| Rust Type                 | SQLite Type             |
| ------------------------- | ----------------------- |
| `String`                  | TEXT                    |
| `i8`, `i16`, `i32`, `i64` | INTEGER                 |
| `u8`, `u16`, `u32`, `u64` | INTEGER                 |
| `f32`, `f64`              | REAL                    |
| `bool`                    | INTEGER (0/1)           |
| `Option<T>`               | Depends on T (nullable) |
| `Vec<u8>`                 | BLOB                    |
| `chrono::DateTime<Utc>`   | TEXT                    |

## Generated Schema

ORSO automatically generates SQL schema:

```sql
-- For a User struct with automatic fields
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    name TEXT NOT NULL,
    email TEXT NOT NULL,
    age INTEGER NOT NULL,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
)
```

## Dependencies

ORSO depends on several key crates:

- `libsql` - The SQLite/Turso database driver
- `serde` - Serialization framework
- `chrono` - Date and time handling
- `uuid` - UUID generation
- `tokio` - Async runtime
- `anyhow` - Error handling

## Limitations

- Schema changes require running migrations
- Complex relationships may need manual implementation
- Advanced SQL features may require raw queries
- Foreign key values should be retrieved from database operations
