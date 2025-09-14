use crate::{Database, FilterOperator, Result};
use chrono::{DateTime, Utc};
use serde::{Serialize, de::DeserializeOwned};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    Text,
    Integer,
    BigInt,
    Numeric,
    Boolean,
    JsonB,
    Timestamp,
}

#[allow(async_fn_in_trait)]
pub trait Orso: Serialize + DeserializeOwned + Send + Sync + Clone {
    fn table_name() -> &'static str;
    fn primary_key_field() -> &'static str {
        "id"
    }
    fn created_at_field() -> Option<&'static str> {
        None
    }
    fn updated_at_field() -> Option<&'static str> {
        None
    }
    fn unique_fields() -> Vec<&'static str> {
        vec![]
    }
    fn has_auto_id() -> bool {
        true
    }
    fn has_timestamps() -> bool {
        true
    }

    fn field_names() -> Vec<&'static str>;
    fn field_types() -> Vec<FieldType>;
    fn field_nullable() -> Vec<bool>;
    fn columns() -> Vec<&'static str>;

    fn get_primary_key(&self) -> Option<String>;
    fn set_primary_key(&mut self, id: String);
    fn get_created_at(&self) -> Option<DateTime<Utc>>;
    fn get_updated_at(&self) -> Option<DateTime<Utc>>;
    fn set_updated_at(&mut self, updated_at: DateTime<Utc>);

    fn migration_sql() -> String;

    fn to_map(&self) -> Result<HashMap<String, crate::Value>>;
    fn from_map(map: HashMap<String, crate::Value>) -> Result<Self>;

    async fn create(&self, db: &Database) -> Result<()> {
        crate::operations::CrudOperations::create(self, db).await
    }
    async fn create_with_table(&self, db: &Database, table_name: &str) -> Result<()> {
        crate::operations::CrudOperations::create_with_table(self, db, table_name).await
    }

    async fn find_by_id(id: &str, db: &Database) -> Result<Option<Self>> {
        crate::operations::CrudOperations::find_by_id::<Self>(id, db).await
    }

    async fn find_by_id_with_table(
        id: &str,
        db: &Database,
        table_name: &str,
    ) -> Result<Option<Self>> {
        crate::operations::CrudOperations::find_by_id_with_table::<Self>(id, db, table_name).await
    }

    async fn find_all(db: &Database) -> Result<Vec<Self>> {
        crate::operations::CrudOperations::find_all::<Self>(db).await
    }

    async fn find_all_with_table(db: &Database, table_name: &str) -> Result<Vec<Self>> {
        crate::operations::CrudOperations::find_all_with_table::<Self>(db, table_name).await
    }

    async fn find_where(filter: FilterOperator, db: &Database) -> Result<Vec<Self>> {
        crate::operations::CrudOperations::find_where::<Self>(filter, db).await
    }

    async fn find_where_with_table(
        filter: FilterOperator,
        db: &Database,
        table_name: &str,
    ) -> Result<Vec<Self>> {
        crate::operations::CrudOperations::find_where_with_table::<Self>(filter, db, table_name)
            .await
    }

    async fn update(&self, db: &Database) -> Result<()> {
        crate::operations::CrudOperations::update(self, db).await
    }

    async fn update_with_table(&self, db: &Database, table_name: &str) -> Result<()> {
        crate::operations::CrudOperations::update_with_table(self, db, table_name).await
    }

    async fn delete(&self, db: &Database) -> Result<bool> {
        crate::operations::CrudOperations::delete(self, db).await
    }

    async fn delete_with_table(&self, db: &Database, table_name: &str) -> Result<bool> {
        crate::operations::CrudOperations::delete_with_table(self, db, table_name).await
    }

    async fn count(db: &Database) -> Result<u64> {
        crate::operations::CrudOperations::count::<Self>(db).await
    }

    async fn count_with_table(db: &Database, table_name: &str) -> Result<u64> {
        crate::operations::CrudOperations::count_with_table::<Self>(db, table_name).await
    }

    // Advanced CRUD operations
    async fn create_or_update(&self, db: &Database) -> Result<()> {
        crate::operations::CrudOperations::create_or_update(self, db).await
    }

    async fn create_or_update_with_table(&self, db: &Database, table_name: &str) -> Result<()> {
        crate::operations::CrudOperations::create_or_update_with_table(self, db, table_name).await
    }

    async fn upsert(&self, db: &Database) -> Result<()> {
        crate::operations::CrudOperations::upsert(self, db).await
    }

    async fn upsert_with_table(&self, db: &Database, table_name: &str) -> Result<()> {
        crate::operations::CrudOperations::upsert_with_table(self, db, table_name).await
    }

    // Batch operations (Turso-optimized with execute_batch)
    async fn batch_create(models: &[Self], db: &Database) -> Result<()> {
        crate::operations::CrudOperations::batch_create(models, db).await
    }

    async fn batch_create_with_table(
        models: &[Self],
        db: &Database,
        table_name: &str,
    ) -> Result<()> {
        crate::operations::CrudOperations::batch_create_with_table(models, db, table_name).await
    }

    async fn batch_update(models: &[Self], db: &Database) -> Result<()> {
        crate::operations::CrudOperations::batch_update(models, db).await
    }

    async fn batch_update_with_table(
        models: &[Self],
        db: &Database,
        table_name: &str,
    ) -> Result<()> {
        crate::operations::CrudOperations::batch_update_with_table(models, db, table_name).await
    }

    async fn batch_delete(ids: &[&str], db: &Database) -> Result<u64> {
        crate::operations::CrudOperations::batch_delete::<Self>(ids, db).await
    }

    async fn batch_delete_with_table(ids: &[&str], db: &Database, table_name: &str) -> Result<u64> {
        crate::operations::CrudOperations::batch_delete_with_table::<Self>(ids, db, table_name)
            .await
    }

    async fn batch_upsert(models: &[Self], db: &Database) -> Result<()> {
        crate::operations::CrudOperations::batch_upsert(models, db).await
    }

    async fn batch_upsert_with_table(
        models: &[Self],
        db: &Database,
        table_name: &str,
    ) -> Result<()> {
        crate::operations::CrudOperations::batch_upsert_with_table(models, db, table_name).await
    }

    // Find operations
    async fn find_one(filter: FilterOperator, db: &Database) -> Result<Option<Self>> {
        crate::operations::CrudOperations::find_one::<Self>(filter, db).await
    }

    async fn find_one_with_table(
        filter: FilterOperator,
        db: &Database,
        table_name: &str,
    ) -> Result<Option<Self>> {
        crate::operations::CrudOperations::find_one_with_table::<Self>(filter, db, table_name).await
    }

    async fn find_paginated(
        pagination: &crate::Pagination,
        db: &Database,
    ) -> Result<crate::PaginatedResult<Self>> {
        crate::operations::CrudOperations::find_paginated::<Self>(pagination, db).await
    }

    async fn find_paginated_with_table(
        pagination: &crate::Pagination,
        db: &Database,
        table_name: &str,
    ) -> Result<crate::PaginatedResult<Self>> {
        crate::operations::CrudOperations::find_paginated_with_table::<Self>(
            pagination, db, table_name,
        )
        .await
    }

    async fn find_where_paginated(
        filter: FilterOperator,
        pagination: &crate::Pagination,
        db: &Database,
    ) -> Result<crate::PaginatedResult<Self>> {
        crate::operations::CrudOperations::find_where_paginated::<Self>(filter, pagination, db)
            .await
    }

    async fn find_where_paginated_with_table(
        filter: FilterOperator,
        pagination: &crate::Pagination,
        db: &Database,
        table_name: &str,
    ) -> Result<crate::PaginatedResult<Self>> {
        crate::operations::CrudOperations::find_where_paginated_with_table::<Self>(
            filter, pagination, db, table_name,
        )
        .await
    }

    // Search operations
    async fn search(
        search_filter: &crate::SearchFilter,
        pagination: Option<&crate::Pagination>,
        db: &Database,
    ) -> Result<crate::PaginatedResult<Self>> {
        crate::operations::CrudOperations::search::<Self>(search_filter, pagination, db).await
    }

    async fn search_with_table(
        search_filter: &crate::SearchFilter,
        pagination: Option<&crate::Pagination>,
        db: &Database,
        table_name: &str,
    ) -> Result<crate::PaginatedResult<Self>> {
        crate::operations::CrudOperations::search_with_table::<Self>(
            search_filter,
            pagination,
            db,
            table_name,
        )
        .await
    }

    // Count operations
    async fn count_where(filter: FilterOperator, db: &Database) -> Result<u64> {
        crate::operations::CrudOperations::count_where::<Self>(filter, db).await
    }

    async fn count_where_with_table(
        filter: FilterOperator,
        db: &Database,
        table_name: &str,
    ) -> Result<u64> {
        crate::operations::CrudOperations::count_where_with_table::<Self>(filter, db, table_name)
            .await
    }

    // Delete operations
    async fn delete_where(filter: FilterOperator, db: &Database) -> Result<u64> {
        crate::operations::CrudOperations::delete_where::<Self>(filter, db).await
    }

    async fn delete_where_with_table(
        filter: FilterOperator,
        db: &Database,
        table_name: &str,
    ) -> Result<u64> {
        crate::operations::CrudOperations::delete_where_with_table::<Self>(filter, db, table_name)
            .await
    }

    // List operations with sorting
    async fn list(
        sort: Option<Vec<crate::Sort>>,
        pagination: Option<&crate::Pagination>,
        db: &Database,
    ) -> Result<crate::PaginatedResult<Self>> {
        crate::operations::CrudOperations::list::<Self>(sort, pagination, db).await
    }

    async fn list_with_table(
        sort: Option<Vec<crate::Sort>>,
        pagination: Option<&crate::Pagination>,
        db: &Database,
        table_name: &str,
    ) -> Result<crate::PaginatedResult<Self>> {
        crate::operations::CrudOperations::list_with_table::<Self>(sort, pagination, db, table_name)
            .await
    }

    async fn list_where(
        filter: FilterOperator,
        sort: Option<Vec<crate::Sort>>,
        pagination: Option<&crate::Pagination>,
        db: &Database,
    ) -> Result<crate::PaginatedResult<Self>> {
        crate::operations::CrudOperations::list_where::<Self>(filter, sort, pagination, db).await
    }

    async fn list_where_with_table(
        filter: FilterOperator,
        sort: Option<Vec<crate::Sort>>,
        pagination: Option<&crate::Pagination>,
        db: &Database,
        table_name: &str,
    ) -> Result<crate::PaginatedResult<Self>> {
        crate::operations::CrudOperations::list_where_with_table::<Self>(
            filter, sort, pagination, db, table_name,
        )
        .await
    }

    // Custom query operations
    async fn query(builder: crate::QueryBuilder, db: &Database) -> Result<Vec<Self>> {
        crate::operations::CrudOperations::query::<Self>(builder, db).await
    }

    async fn query_with_table(builder: crate::QueryBuilder, db: &Database) -> Result<Vec<Self>> {
        crate::operations::CrudOperations::query_with_table::<Self>(builder, db).await
    }

    async fn query_paginated(
        builder: crate::QueryBuilder,
        pagination: &crate::Pagination,
        db: &Database,
    ) -> Result<crate::PaginatedResult<Self>> {
        crate::operations::CrudOperations::query_paginated::<Self>(builder, pagination, db).await
    }

    async fn query_paginated_with_table(
        builder: crate::QueryBuilder,
        pagination: &crate::Pagination,
        db: &Database,
    ) -> Result<crate::PaginatedResult<Self>> {
        crate::operations::CrudOperations::query_paginated_with_table::<Self>(
            builder, pagination, db,
        )
        .await
    }

    // Aggregate operations
    async fn aggregate(
        function: crate::Aggregate,
        column: &str,
        filter: Option<FilterOperator>,
        db: &Database,
    ) -> Result<Option<f64>> {
        crate::operations::CrudOperations::aggregate::<Self>(function, column, filter, db).await
    }

    async fn aggregate_with_table(
        function: crate::Aggregate,
        column: &str,
        filter: Option<FilterOperator>,
        db: &Database,
        table_name: &str,
    ) -> Result<Option<f64>> {
        crate::operations::CrudOperations::aggregate_with_table::<Self>(
            function, column, filter, db, table_name,
        )
        .await
    }

    // Legacy batch operations (for compatibility)
    async fn batch_insert(records: &[Self], db: &Database) -> Result<u64> {
        Self::batch_create(records, db).await?;
        Ok(records.len() as u64)
    }

    // Filter operations
    fn build_filter_operator(filter: &FilterOperator) -> Result<(String, Vec<libsql::Value>)> {
        crate::filters::FilterOperations::build_filter_operator(filter)
    }

    fn build_filter(filter: &crate::Filter) -> Result<(String, Vec<libsql::Value>)> {
        crate::filters::FilterOperations::build_filter(filter)
    }

    // Conversion functions with default implementations
    fn row_to_map(row: &libsql::Row) -> Result<HashMap<String, crate::Value>> {
        crate::operations::CrudOperations::row_to_map(row)
    }

    fn value_to_libsql_value(value: &crate::Value) -> libsql::Value {
        crate::Utils::value_to_libsql_value(value)
    }

    fn libsql_value_to_value(value: &libsql::Value) -> crate::Value {
        crate::Utils::libsql_value_to_value(value)
    }
}
