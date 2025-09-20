# ORSO - Rust ORM for SQLite and Turso

ORSO is a Rust ORM (Object-Relational Mapping) library for working with SQLite and Turso databases. It provides a straightforward way to define database schemas using Rust structs and perform common database operations with zero-loss migrations.

## Project Overview

- **Name**: ORSO (ORM for turSO)
- **Type**: Rust library crate
- **Description**: A Rust ORM for SQLite and Turso with zero-loss migrations
- **Main Technologies**: Rust, libSQL, SQLite, Turso
- **Architecture**: 
  - Workspace with two crates: `orso` (main library) and `orso-macros` (procedural macros)
  - Procedural derive macros for automatic schema generation
  - Multiple database connection modes (local, remote, sync, embedded)
  - Zero-loss migration system with automatic backup management

## Key Features

- **Derive-based schema definition**: Use `#[derive(Orso)]` to automatically generate database schema from Rust structs
- **Multiple database modes**: Support for local SQLite, remote Turso, sync, and embedded modes
- **Automatic schema management**: Generate SQL schema and handle migrations with zero data loss
- **CRUD operations**: insert, read, update, and delete records
- **Batch operations**: Efficient handling of multiple records
- **Query building**: Flexible query construction with filtering and sorting
- **Pagination**: Support for paginated results
- **Foreign key relationships**: Define relationships between tables
- **Type mapping**: Automatic conversion between Rust types and database types
- **Utility operations**: Existence checks, field-based queries, latest/first record finding, and batch ID operations
- **Runtime table selection**: Use `_with_table` methods to work with multiple tables using the same struct

## Building and Running

### Prerequisites

- Rust toolchain (cargo, rustc)
- For Turso integration: Turso database URL and authentication token

### Building

```bash
# Build the entire workspace
cargo build

# Build with optimizations
cargo build --release
```

### Running Tests

```bash
# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p orso
cargo test -p orso-macros
```

### Publishing

```bash
# Publish both crates in the workspace
cargo publish
```

## Development Conventions

### Code Structure

- **orso/**: Main library implementation
  - `src/database.rs`: Database connection and configuration
  - `src/migrations.rs`: Migration system with zero-loss schema changes
  - `src/operations.rs`: CRUD operations and query building
  - `src/filters.rs`: Filter and search functionality
  - `src/query.rs`: Query builder implementation
  - `src/pagination.rs`: Pagination support
  - `src/macros.rs`: Convenience macros
  - `src/traits.rs`: Core traits (Orso trait)
  - `src/types.rs`: Value types and field types
  - `src/error.rs`: Error handling
  - `src/utils.rs`: Utility functions
- **orso-macros/**: Procedural macros for derive functionality
  - Implements `#[derive(Orso)]` macro
  - Handles `#[orso_table]` and `#[orso_column]` attributes

### Coding Standards

- Follow Rust community best practices
- Use `rustfmt` for code formatting
- Include documentation for public APIs
- Write tests for new functionality
- Use tracing for logging

### Schema Definition

1. Define models using Rust structs
2. Use `#[derive(Orso)]` to enable ORM functionality
3. Use `#[orso_table("table_name")]` to specify table names
4. Use `#[orso_column(...)]` for column attributes:
   - `primary_key`: Mark field as primary key
   - `unique`: Add unique constraint
   - `ref = "table_name"`: Define foreign key reference
   - `created_at`: Auto-managed creation timestamp
   - `updated_at`: Auto-managed update timestamp

### Migration System

- ORSO provides automatic zero-loss migrations
- Migration backups are automatically managed with configurable retention
- Use `Migrations::init()` or `Migrations::init_with_config()` to run migrations
- Migration configuration options:
  - `max_backups_per_table`: Maximum number of migration backups to keep
  - `backup_retention_days`: Delete backups older than this many days
  - `backup_suffix`: Suffix used for migration table names

### Database Operations

- Use model methods for common operations:
  - `insert()`, `update()`, `delete()`: Basic CRUD
  - `find_by_id()`, `find_all()`, `find_where()`: Data retrieval
  - `batch_insert()`, `batch_update()`, `batch_delete()`: Batch operations
  - `find_latest()`, `find_latest_filter()`: Find latest records
  - `exists()`, `exists_filter()`: Check for record existence
  - `_with_table` variants for custom table operations

## Useful Commands

```bash
# Format code
cargo fmt

# Run linter
cargo clippy

# Run tests
cargo test

# Build documentation
cargo doc --open

# Check for unused dependencies
cargo udeps
```