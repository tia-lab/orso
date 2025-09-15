use crate::{
    Aggregate, Database, Error, FilterOperator, PaginatedResult, Pagination, QueryBuilder, Result,
    SearchFilter, Sort, SortOrder, Utils,
};
use std::collections::HashMap;
use tracing::{debug, info, trace, warn};

/// CRUD operations for database models
pub struct CrudOperations;

impl CrudOperations {
    /// Insert a new record in the database
    pub async fn insert<T>(model: &T, db: &Database) -> Result<()>
    where
        T: crate::Orso,
    {
        Self::insert_with_table(model, db, T::table_name()).await
    }
    /// Insert a new record in the database
    pub async fn insert_with_table<T>(model: &T, db: &Database, table_name: &str) -> Result<()>
    where
        T: crate::Orso,
    {
        let map = model.to_map()?;
        let columns: Vec<String> = map.keys().cloned().collect();
        let values: Vec<String> = map.keys().map(|_| "?".to_string()).collect();

        let sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table_name,
            columns.join(", "),
            values.join(", ")
        );

        debug!(sql = %sql, "Executing SQL");

        let params: Vec<libsql::Value> =
            map.values().map(|v| T::value_to_libsql_value(v)).collect();

        db.conn.execute(&sql, params).await?;

        debug!(table = table_name, "Successfully created record");
        Ok(())
    }

    /// Insert or update a record based on whether it has a primary key
    pub async fn insert_or_update<T>(model: &T, db: &Database) -> Result<()>
    where
        T: crate::Orso,
    {
        Self::insert_or_update_with_table(model, db, T::table_name()).await
    }

    pub async fn insert_or_update_with_table<T>(
        model: &T,
        db: &Database,
        table_name: &str,
    ) -> Result<()>
    where
        T: crate::Orso,
    {
        if let Some(id) = model.get_primary_key() {
            // Check if record exists
            match Self::find_by_id_with_table::<T>(&id, db, table_name).await? {
                Some(_) => {
                    // Record exists, update it
                    Self::update_with_table(model, db, table_name).await
                }
                None => {
                    // Record doesn't exist, insert it
                    warn!(table = table_name, id = %id, "Record with ID not found, creating new record");
                    Self::insert_with_table(model, db, table_name).await
                }
            }
        } else {
            // No primary key, insert new record
            trace!(
                table = table_name,
                "Creating new record (no primary key provided)"
            );
            Self::insert_with_table(model, db, table_name).await
        }
    }

    /// Insert or update a record based on unique constraints
    pub async fn upsert<T>(model: &T, db: &Database) -> Result<()>
    where
        T: crate::Orso,
    {
        Self::upsert_with_table(model, db, T::table_name()).await
    }

    pub async fn upsert_with_table<T>(model: &T, db: &Database, table_name: &str) -> Result<()>
    where
        T: crate::Orso,
    {
        let unique_columns: Vec<&str> = T::unique_fields();
        if unique_columns.is_empty() {
            return Err(Error::Validation(
                "No unique columns defined with orso_column(unique) for upsert".to_string(),
            ));
        }

        let map = model.to_map()?;

        // Build WHERE clause for unique columns
        let mut where_conditions = Vec::new();
        let mut where_params = Vec::new();

        for column in &unique_columns {
            if let Some(value) = map.get(*column) {
                where_conditions.push(format!("{column} = ?"));
                where_params.push(T::value_to_libsql_value(value));
            }
        }

        if where_conditions.is_empty() {
            return Err(Error::Validation(
                "No valid unique column values found for upsert".to_string(),
            ));
        }

        let where_clause = where_conditions.join(" AND ");
        let sql = format!(
            "SELECT * FROM {} WHERE {} LIMIT 1",
            table_name, where_clause
        );

        info!(table = table_name, "Checking for existing record");
        debug!(sql = %sql, "Executing upsert query");

        let mut rows = db.conn.query(&sql, where_params).await?;

        if let Some(row) = rows.next().await? {
            // Record exists, update it
            let _row_map = T::row_to_map(&row)?;
            info!(table = table_name, "Found existing record, updating");
            Self::update_with_table(model, db, table_name).await
        } else {
            // Record doesn't exist, insert it
            info!(
                table = table_name,
                "No existing record found, creating new one"
            );
            Self::insert_with_table(model, db, table_name).await
        }
    }

    /// Insert multiple records using Turso batch operations for optimal performance
    pub async fn batch_create<T>(models: &[T], db: &Database) -> Result<()>
    where
        T: crate::Orso,
    {
        Self::batch_insert_with_table(models, db, T::table_name()).await
    }

    pub async fn batch_insert_with_table<T>(
        models: &[T],
        db: &Database,
        table_name: &str,
    ) -> Result<()>
    where
        T: crate::Orso,
    {
        if models.is_empty() {
            return Ok(());
        }

        let mut stmts = Vec::new();
        stmts.push("BEGIN".to_string());

        for model in models {
            let map = model.to_map()?;
            let columns: Vec<String> = map.keys().cloned().collect();
            let values: Vec<String> = map
                .values()
                .map(|v| match v {
                    crate::Value::Text(s) => format!("'{}'", s.replace("'", "''")),
                    crate::Value::Integer(i) => i.to_string(),
                    crate::Value::Real(f) => f.to_string(),
                    crate::Value::Boolean(b) => if *b { "1" } else { "0" }.to_string(),
                    crate::Value::Null => "NULL".to_string(),
                    crate::Value::Blob(_) => "NULL".to_string(), // Simplified for batch
                })
                .collect();

            let sql = format!(
                "INSERT INTO {} ({}) VALUES ({})",
                table_name,
                columns.join(", "),
                values.join(", ")
            );
            stmts.push(sql);
        }

        stmts.push("COMMIT".to_string());
        let batch_sql = stmts.join(";");

        // For batch operations, just execute the batch - no need to return models
        db.conn.execute_batch(&batch_sql).await?;
        Ok(())
    }

    /// Find a record by its primary key
    pub async fn find_by_id<T>(id: &str, db: &Database) -> Result<Option<T>>
    where
        T: crate::Orso,
    {
        Self::find_by_id_with_table(id, db, T::table_name()).await
    }

    pub async fn find_by_id_with_table<T>(
        id: &str,
        db: &Database,
        table_name: &str,
    ) -> Result<Option<T>>
    where
        T: crate::Orso,
    {
        let sql = format!(
            "SELECT * FROM {} WHERE {} = ? LIMIT 1",
            table_name,
            T::primary_key_field() // Use dynamic primary key field name
        );

        debug!(table =table_name, id = %id, "Finding record by ID");
        debug!(sql = %sql, "Executing find query");

        let mut rows = db
            .conn
            .query(&sql, vec![libsql::Value::Text(id.to_string())])
            .await?;

        if let Some(row) = rows.next().await? {
            let map = T::row_to_map(&row)?;
            debug!(table =table_name, id = %id, "Found record");
            Ok(Some(T::from_map(map)?))
        } else {
            debug!(table =table_name, id = %id, "No record found");
            Ok(None)
        }
    }

    /// Find a single record by a specific condition
    pub async fn find_one<T>(filter: FilterOperator, db: &Database) -> Result<Option<T>>
    where
        T: crate::Orso,
    {
        Self::find_one_with_table(filter, db, T::table_name()).await
    }

    pub async fn find_one_with_table<T>(
        filter: FilterOperator,
        db: &Database,
        table_name: &str,
    ) -> Result<Option<T>>
    where
        T: crate::Orso,
    {
        let builder = QueryBuilder::new(table_name)._where(filter).limit(1);

        let results = builder.execute::<T>(db).await?;
        Ok(results.into_iter().next())
    }

    /// Find all records
    pub async fn find_all<T>(db: &Database) -> Result<Vec<T>>
    where
        T: crate::Orso,
    {
        Self::find_all_with_table(db, T::table_name()).await
    }

    pub async fn find_all_with_table<T>(db: &Database, table_name: &str) -> Result<Vec<T>>
    where
        T: crate::Orso,
    {
        let builder = QueryBuilder::new(table_name);
        builder.execute::<T>(db).await
    }

    /// Find records with a filter
    pub async fn find_where<T>(filter: FilterOperator, db: &Database) -> Result<Vec<T>>
    where
        T: crate::Orso,
    {
        Self::find_where_with_table(filter, db, T::table_name()).await
    }

    pub async fn find_where_with_table<T>(
        filter: FilterOperator,
        db: &Database,
        table_name: &str,
    ) -> Result<Vec<T>>
    where
        T: crate::Orso,
    {
        let builder = QueryBuilder::new(table_name)._where(filter);
        builder.execute::<T>(db).await
    }

    pub async fn find_latest<T>(db: &Database) -> Result<Option<T>>
    where
        T: crate::Orso,
    {
        Self::find_latest_with_table(db, T::table_name()).await
    }

    pub async fn find_latest_with_table<T>(db: &Database, table_name: &str) -> Result<Option<T>>
    where
        T: crate::Orso,
    {
        let created_at_field = T::created_at_field().unwrap_or("created_at");
        let sort = Sort::new(created_at_field, SortOrder::Desc);
        let builder = QueryBuilder::new(table_name).order_by(sort).limit(1);

        let results = builder.execute::<T>(db).await?;
        Ok(results.into_iter().next())
    }

    /// Find latest record matching filter
    pub async fn find_latest_filter<T>(filter: FilterOperator, db: &Database) -> Result<Option<T>>
    where
        T: crate::Orso,
    {
        Self::find_latest_filter_with_table(filter, db, T::table_name()).await
    }

    pub async fn find_latest_filter_with_table<T>(
        filter: FilterOperator,
        db: &Database,
        table_name: &str,
    ) -> Result<Option<T>>
    where
        T: crate::Orso,
    {
        let created_at_field = T::created_at_field().unwrap_or("created_at");
        let sort = Sort::new(created_at_field, SortOrder::Desc);
        let builder = QueryBuilder::new(table_name)
            ._where(filter)
            .order_by(sort)
            .limit(1);
        let results = builder.execute::<T>(db).await?;
        Ok(results.into_iter().next())
    }

    /// Find first record matching filter (oldest)
    pub async fn find_first_filter<T>(filter: FilterOperator, db: &Database) -> Result<Option<T>>
    where
        T: crate::Orso,
    {
        Self::find_first_filter_with_table(filter, db, T::table_name()).await
    }

    pub async fn find_first_filter_with_table<T>(
        filter: FilterOperator,
        db: &Database,
        table_name: &str,
    ) -> Result<Option<T>>
    where
        T: crate::Orso,
    {
        let created_at_field = T::created_at_field().unwrap_or("created_at");
        let sort = Sort::new(created_at_field, SortOrder::Asc);
        let builder = QueryBuilder::new(table_name)
            ._where(filter)
            .order_by(sort)
            .limit(1);
        let results = builder.execute::<T>(db).await?;
        Ok(results.into_iter().next())
    }

    /// Check if any record exists
    pub async fn exists<T>(db: &Database) -> Result<bool>
    where
        T: crate::Orso,
    {
        Self::exists_with_table::<T>(db, T::table_name()).await
    }

    pub async fn exists_with_table<T>(db: &Database, table_name: &str) -> Result<bool>
    where
        T: crate::Orso,
    {
        let builder = QueryBuilder::new(table_name).limit(1);
        let count = builder.execute_count(db).await?;
        Ok(count > 0)
    }

    /// Check if any record exists matching filter
    pub async fn exists_filter<T>(filter: FilterOperator, db: &Database) -> Result<bool>
    where
        T: crate::Orso,
    {
        Self::exists_filter_with_table::<T>(filter, db, T::table_name()).await
    }

    pub async fn exists_filter_with_table<T>(
        filter: FilterOperator,
        db: &Database,
        table_name: &str,
    ) -> Result<bool>
    where
        T: crate::Orso,
    {
        let builder = QueryBuilder::new(table_name)._where(filter).limit(1);
        let count = builder.execute_count(db).await?;
        Ok(count > 0)
    }

    /// Find by any field value
    pub async fn find_by_field<T>(field: &str, value: crate::Value, db: &Database) -> Result<Vec<T>>
    where
        T: crate::Orso,
    {
        Self::find_by_field_with_table(field, value, db, T::table_name()).await
    }

    pub async fn find_by_field_with_table<T>(
        field: &str,
        value: crate::Value,
        db: &Database,
        table_name: &str,
    ) -> Result<Vec<T>>
    where
        T: crate::Orso,
    {
        let filter =
            FilterOperator::Single(crate::Filter::new_simple(field, crate::Operator::Eq, value));
        let builder = QueryBuilder::new(table_name)._where(filter);
        builder.execute::<T>(db).await
    }

    /// Find latest record by field value
    pub async fn find_latest_by_field<T>(
        field: &str,
        value: crate::Value,
        db: &Database,
    ) -> Result<Option<T>>
    where
        T: crate::Orso,
    {
        Self::find_latest_by_field_with_table(field, value, db, T::table_name()).await
    }

    pub async fn find_latest_by_field_with_table<T>(
        field: &str,
        value: crate::Value,
        db: &Database,
        table_name: &str,
    ) -> Result<Option<T>>
    where
        T: crate::Orso,
    {
        let filter =
            FilterOperator::Single(crate::Filter::new_simple(field, crate::Operator::Eq, value));
        let created_at_field = T::created_at_field().unwrap_or("created_at");
        let sort = Sort::new(created_at_field, SortOrder::Desc);
        let builder = QueryBuilder::new(table_name)
            ._where(filter)
            .order_by(sort)
            .limit(1);
        let results = builder.execute::<T>(db).await?;
        Ok(results.into_iter().next())
    }

    /// Find first record by field value (oldest)
    pub async fn find_first_by_field<T>(
        field: &str,
        value: crate::Value,
        db: &Database,
    ) -> Result<Option<T>>
    where
        T: crate::Orso,
    {
        Self::find_first_by_field_with_table(field, value, db, T::table_name()).await
    }

    pub async fn find_first_by_field_with_table<T>(
        field: &str,
        value: crate::Value,
        db: &Database,
        table_name: &str,
    ) -> Result<Option<T>>
    where
        T: crate::Orso,
    {
        let filter =
            FilterOperator::Single(crate::Filter::new_simple(field, crate::Operator::Eq, value));
        let created_at_field = T::created_at_field().unwrap_or("created_at");
        let sort = Sort::new(created_at_field, SortOrder::Asc);
        let builder = QueryBuilder::new(table_name)
            ._where(filter)
            .order_by(sort)
            .limit(1);
        let results = builder.execute::<T>(db).await?;
        Ok(results.into_iter().next())
    }

    /// Find multiple records by IDs (batch operation)
    pub async fn find_by_ids<T>(ids: &[&str], db: &Database) -> Result<Vec<T>>
    where
        T: crate::Orso,
    {
        Self::find_by_ids_with_table(ids, db, T::table_name()).await
    }

    pub async fn find_by_ids_with_table<T>(
        ids: &[&str],
        db: &Database,
        table_name: &str,
    ) -> Result<Vec<T>>
    where
        T: crate::Orso,
    {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let id_values: Vec<crate::Value> = ids
            .iter()
            .map(|id| crate::Value::Text(id.to_string()))
            .collect();
        let pk_field = T::primary_key_field();
        let filter = FilterOperator::Single(crate::Filter::in_values(pk_field, id_values));
        let builder = QueryBuilder::new(table_name)._where(filter);
        builder.execute::<T>(db).await
    }

    /// Find records by multiple values for same field (IN clause)
    pub async fn find_by_field_in<T>(
        field: &str,
        values: &[crate::Value],
        db: &Database,
    ) -> Result<Vec<T>>
    where
        T: crate::Orso,
    {
        Self::find_by_field_in_with_table(field, values, db, T::table_name()).await
    }

    pub async fn find_by_field_in_with_table<T>(
        field: &str,
        values: &[crate::Value],
        db: &Database,
        table_name: &str,
    ) -> Result<Vec<T>>
    where
        T: crate::Orso,
    {
        if values.is_empty() {
            return Ok(Vec::new());
        }

        let filter = FilterOperator::Single(crate::Filter::in_values(field, values.to_vec()));
        let builder = QueryBuilder::new(table_name)._where(filter);
        builder.execute::<T>(db).await
    }

    /// Find records with pagination
    pub async fn find_paginated<T>(
        pagination: &Pagination,
        db: &Database,
    ) -> Result<PaginatedResult<T>>
    where
        T: crate::Orso,
    {
        Self::find_paginated_with_table(pagination, db, T::table_name()).await
    }

    pub async fn find_paginated_with_table<T>(
        pagination: &Pagination,
        db: &Database,
        table_name: &str,
    ) -> Result<PaginatedResult<T>>
    where
        T: crate::Orso,
    {
        let builder = QueryBuilder::new(table_name);
        builder.execute_paginated::<T>(db, pagination).await
    }

    /// Find records with filter and pagination
    pub async fn find_where_paginated<T>(
        filter: FilterOperator,
        pagination: &Pagination,
        db: &Database,
    ) -> Result<PaginatedResult<T>>
    where
        T: crate::Orso,
    {
        Self::find_where_paginated_with_table(filter, pagination, db, T::table_name()).await
    }

    pub async fn find_where_paginated_with_table<T>(
        filter: FilterOperator,
        pagination: &Pagination,
        db: &Database,
        table_name: &str,
    ) -> Result<PaginatedResult<T>>
    where
        T: crate::Orso,
    {
        let builder = QueryBuilder::new(table_name)._where(filter);
        builder.execute_paginated::<T>(db, pagination).await
    }

    /// Search records with text search
    pub async fn search<T>(
        search_filter: &SearchFilter,
        pagination: Option<&Pagination>,
        db: &Database,
    ) -> Result<PaginatedResult<T>>
    where
        T: crate::Orso,
    {
        Self::search_with_table(search_filter, pagination, db, T::table_name()).await
    }

    pub async fn search_with_table<T>(
        search_filter: &SearchFilter,
        pagination: Option<&Pagination>,
        db: &Database,
        table_name: &str,
    ) -> Result<PaginatedResult<T>>
    where
        T: crate::Orso,
    {
        let filter = search_filter.to_filter_operator();
        let pagination = pagination.unwrap_or(&Pagination::default()).clone();

        Self::find_where_paginated_with_table::<T>(filter, &pagination, db, table_name).await
    }

    /// Count all records
    pub async fn count<T>(db: &Database) -> Result<u64>
    where
        T: crate::Orso,
    {
        Self::count_with_table::<T>(db, T::table_name()).await
    }

    pub async fn count_with_table<T>(db: &Database, table_name: &str) -> Result<u64>
    where
        T: crate::Orso,
    {
        let sql = format!("SELECT COUNT(*) FROM {}", table_name);
        let mut rows = db.conn.query(&sql, vec![libsql::Value::Null; 0]).await?;

        if let Some(row) = rows.next().await? {
            row.get_value(0)
                .ok()
                .and_then(|v| match v {
                    libsql::Value::Integer(i) => Some(i as u64),
                    _ => None,
                })
                .ok_or_else(|| Error::Query("Failed to get count".to_string()))
        } else {
            Err(Error::Query("No count result".to_string()))
        }
    }

    /// Count records with a filter
    pub async fn count_where<T>(filter: FilterOperator, db: &Database) -> Result<u64>
    where
        T: crate::Orso,
    {
        Self::count_where_with_table::<T>(filter, db, T::table_name()).await
    }

    pub async fn count_where_with_table<T>(
        filter: FilterOperator,
        db: &Database,
        table_name: &str,
    ) -> Result<u64>
    where
        T: crate::Orso,
    {
        let builder = QueryBuilder::new(table_name)._where(filter);

        let (sql, params) = builder.build_count()?;
        let mut rows = db.conn.query(&sql, params).await?;

        if let Some(row) = rows.next().await? {
            row.get_value(0)
                .ok()
                .and_then(|v| match v {
                    libsql::Value::Integer(i) => Some(i as u64),
                    _ => None,
                })
                .ok_or_else(|| Error::Query("Failed to get count".to_string()))
        } else {
            Err(Error::Query("No count result".to_string()))
        }
    }

    /// Update a record
    pub async fn update<T>(model: &T, db: &Database) -> Result<()>
    where
        T: crate::Orso,
    {
        Self::update_with_table(model, db, T::table_name()).await
    }

    pub async fn update_with_table<T>(model: &T, db: &Database, table_name: &str) -> Result<()>
    where
        T: crate::Orso,
    {
        let id = model.get_primary_key().ok_or_else(|| {
            Error::Validation("Cannot update record without primary key".to_string())
        })?;

        let map = model.to_map()?;
        let pk_field = T::primary_key_field();
        let updated_at_field = T::updated_at_field();

        let mut set_clauses = Vec::new();
        for k in map.keys() {
            if k != pk_field {
                // For updated_at fields, use database function instead of model value
                if updated_at_field.is_some() && k == updated_at_field.unwrap() {
                    set_clauses.push(format!("{k} = datetime('now')"));
                } else {
                    set_clauses.push(format!("{k} = ?"));
                }
            }
        }

        let sql = format!(
            "UPDATE {} SET {} WHERE {} = ?",
            table_name,
            set_clauses.join(", "),
            pk_field
        );

        info!(table = table_name, id = %id, "Updating record");
        debug!(sql = %sql, "Executing update query");

        let mut params: Vec<libsql::Value> = map
            .iter()
            .filter(|(k, _)| {
                k != &pk_field && !(updated_at_field.is_some() && k == &updated_at_field.unwrap())
            })
            .map(|(_, v)| T::value_to_libsql_value(v))
            .collect();
        params.push(libsql::Value::Text(id.clone()));

        db.conn.execute(&sql, params).await?;

        info!(table = table_name, id = %id, "Successfully updated record");
        Ok(())
    }

    /// Update multiple records using Turso batch operations
    pub async fn batch_update<T>(models: &[T], db: &Database) -> Result<()>
    where
        T: crate::Orso,
    {
        Self::batch_update_with_table(models, db, T::table_name()).await
    }

    pub async fn batch_update_with_table<T>(
        models: &[T],
        db: &Database,
        table_name: &str,
    ) -> Result<()>
    where
        T: crate::Orso,
    {
        if models.is_empty() {
            return Ok(());
        }

        let mut stmts = Vec::new();
        stmts.push("BEGIN".to_string());

        for model in models {
            let id = model.get_primary_key().ok_or_else(|| {
                Error::Validation("Cannot batch update record without primary key".to_string())
            })?;

            let map = model.to_map()?;
            let pk_field = T::primary_key_field();
            let updated_at_field = T::updated_at_field();
            let set_clauses: Vec<String> = map
                .keys()
                .filter(|&k| k != pk_field)
                .map(|k| {
                    // For updated_at fields, use database function instead of model value
                    if updated_at_field.is_some() && k == updated_at_field.unwrap() {
                        format!("{} = datetime('now')", k)
                    } else {
                        let value = map.get(k).unwrap();
                        let sql_value = match value {
                            crate::Value::Text(s) => format!("'{}'", s.replace("'", "''")),
                            crate::Value::Integer(i) => i.to_string(),
                            crate::Value::Real(f) => f.to_string(),
                            crate::Value::Boolean(b) => if *b { "1" } else { "0" }.to_string(),
                            crate::Value::Null => "NULL".to_string(),
                            crate::Value::Blob(_) => "NULL".to_string(),
                        };
                        format!("{} = {}", k, sql_value)
                    }
                })
                .collect();

            let sql = format!(
                "UPDATE {} SET {} WHERE {} = '{}'",
                table_name,
                set_clauses.join(", "),
                pk_field,
                id.replace("'", "''")
            );
            stmts.push(sql);
        }

        stmts.push("COMMIT".to_string());
        let batch_sql = stmts.join(";");

        db.conn.execute_batch(&batch_sql).await?;
        Ok(())
    }

    /// Delete a record
    pub async fn delete<T>(model: &T, db: &Database) -> Result<bool>
    where
        T: crate::Orso,
    {
        Self::delete_with_table(model, db, T::table_name()).await
    }

    pub async fn delete_with_table<T>(model: &T, db: &Database, table_name: &str) -> Result<bool>
    where
        T: crate::Orso,
    {
        let id = model.get_primary_key().ok_or_else(|| {
            Error::Validation("Cannot delete record without primary key".to_string())
        })?;

        let sql = format!(
            "DELETE FROM {} WHERE {} = ?",
            table_name,
            T::primary_key_field()
        );

        info!(table = table_name, id = %id, "Deleting record");
        debug!(sql = %sql, "Executing delete query");

        db.conn.execute(&sql, vec![libsql::Value::Text(id)]).await?;
        info!(table = table_name, "Successfully deleted record");
        Ok(true)
    }

    /// Delete multiple records using Turso batch operations
    pub async fn batch_delete<T>(ids: &[&str], db: &Database) -> Result<u64>
    where
        T: crate::Orso,
    {
        Self::batch_delete_with_table::<T>(ids, db, T::table_name()).await
    }

    pub async fn batch_delete_with_table<T>(
        ids: &[&str],
        db: &Database,
        table_name: &str,
    ) -> Result<u64>
    where
        T: crate::Orso,
    {
        if ids.is_empty() {
            return Ok(0);
        }

        let mut stmts = Vec::new();
        stmts.push("BEGIN".to_string());

        let pk_field = T::primary_key_field();
        for id in ids {
            let sql = format!(
                "DELETE FROM {} WHERE {} = '{}'",
                table_name,
                pk_field,
                id.replace("'", "''")
            );
            stmts.push(sql);
        }

        stmts.push("COMMIT".to_string());
        let batch_sql = stmts.join(";");

        db.conn.execute_batch(&batch_sql).await?;
        Ok(ids.len() as u64)
    }

    /// Upsert multiple records using Turso batch operations with automatically detected unique columns
    pub async fn batch_upsert<T>(models: &[T], db: &Database) -> Result<()>
    where
        T: crate::Orso,
    {
        Self::batch_upsert_with_table(models, db, T::table_name()).await
    }

    pub async fn batch_upsert_with_table<T>(
        models: &[T],
        db: &Database,
        table_name: &str,
    ) -> Result<()>
    where
        T: crate::Orso,
    {
        if models.is_empty() {
            return Ok(());
        }

        let unique_columns: Vec<&str> = T::unique_fields();
        if unique_columns.is_empty() {
            return Err(Error::Validation(
                "No unique columns defined with orso_column(unique) for batch upsert".to_string(),
            ));
        }

        let mut stmts = Vec::new();
        stmts.push("BEGIN".to_string());

        for model in models {
            let map = model.to_map()?;

            // Build conflict columns for ON CONFLICT clause
            let conflict_columns = unique_columns.join(", ");

            let columns: Vec<String> = map.keys().cloned().collect();
            let values: Vec<String> = map
                .values()
                .map(|v| match v {
                    crate::Value::Text(s) => format!("'{}'", s.replace("'", "''")),
                    crate::Value::Integer(i) => i.to_string(),
                    crate::Value::Real(f) => f.to_string(),
                    crate::Value::Boolean(b) => if *b { "1" } else { "0" }.to_string(),
                    crate::Value::Null => "NULL".to_string(),
                    crate::Value::Blob(_) => "NULL".to_string(),
                })
                .collect();

            // Build UPDATE SET clause for conflict resolution
            let updated_at_field = T::updated_at_field();
            let update_sets: Vec<String> = columns
                .iter()
                .filter(|col| !unique_columns.contains(&col.as_str())) // Don't update unique columns
                .map(|col| {
                    // For updated_at fields, use database function instead of excluded value
                    if updated_at_field.is_some() && col == updated_at_field.unwrap() {
                        format!("{} = datetime('now')", col)
                    } else {
                        format!("{} = excluded.{}", col, col)
                    }
                })
                .collect();

            let sql = if update_sets.is_empty() {
                // If no columns to update, just ignore conflicts
                format!(
                    "INSERT OR IGNORE INTO {} ({}) VALUES ({})",
                    table_name,
                    columns.join(", "),
                    values.join(", ")
                )
            } else {
                // Use INSERT ... ON CONFLICT DO UPDATE for proper upsert
                format!(
                    "INSERT INTO {} ({}) VALUES ({}) ON CONFLICT ({}) DO UPDATE SET {}",
                    table_name,
                    columns.join(", "),
                    values.join(", "),
                    conflict_columns,
                    update_sets.join(", ")
                )
            };

            stmts.push(sql);
        }

        stmts.push("COMMIT".to_string());
        let batch_sql = stmts.join(";");

        db.conn.execute_batch(&batch_sql).await?;
        Ok(())
    }

    /// Delete records with a filter
    pub async fn delete_where<T>(filter: FilterOperator, db: &Database) -> Result<u64>
    where
        T: crate::Orso,
    {
        Self::delete_where_with_table::<T>(filter, db, T::table_name()).await
    }

    pub async fn delete_where_with_table<T>(
        filter: FilterOperator,
        db: &Database,
        table_name: &str,
    ) -> Result<u64>
    where
        T: crate::Orso,
    {
        let builder = QueryBuilder::new(table_name)._where(filter);

        let (sql, params) = builder.build()?;
        let delete_sql = sql.replace("SELECT *", "DELETE");
        db.conn.execute(&delete_sql, params).await?;

        // Note: SQLite doesn't return the number of affected rows directly
        // This is a simplified implementation
        Ok(1)
    }

    /// List records with optional sorting and pagination
    pub async fn list<T>(
        sort: Option<Vec<Sort>>,
        pagination: Option<&Pagination>,
        db: &Database,
    ) -> Result<PaginatedResult<T>>
    where
        T: crate::Orso,
    {
        Self::list_with_table(sort, pagination, db, T::table_name()).await
    }

    pub async fn list_with_table<T>(
        sort: Option<Vec<Sort>>,
        pagination: Option<&Pagination>,
        db: &Database,
        table_name: &str,
    ) -> Result<PaginatedResult<T>>
    where
        T: crate::Orso,
    {
        let mut builder = QueryBuilder::new(table_name);

        if let Some(sorts) = sort {
            builder = builder.order_by_multiple(sorts);
        }

        let pagination = pagination.unwrap_or(&Pagination::default()).clone();
        builder.execute_paginated::<T>(db, &pagination).await
    }

    /// List records with filter, sorting, and pagination
    pub async fn list_where<T>(
        filter: FilterOperator,
        sort: Option<Vec<Sort>>,
        pagination: Option<&Pagination>,
        db: &Database,
    ) -> Result<PaginatedResult<T>>
    where
        T: crate::Orso,
    {
        Self::list_where_with_table(filter, sort, pagination, db, T::table_name()).await
    }

    pub async fn list_where_with_table<T>(
        filter: FilterOperator,
        sort: Option<Vec<Sort>>,
        pagination: Option<&Pagination>,
        db: &Database,
        table_name: &str,
    ) -> Result<PaginatedResult<T>>
    where
        T: crate::Orso,
    {
        let mut builder = QueryBuilder::new(table_name)._where(filter);

        if let Some(sorts) = sort {
            builder = builder.order_by_multiple(sorts);
        }

        let pagination = pagination.unwrap_or(&Pagination::default()).clone();
        builder.execute_paginated::<T>(db, &pagination).await
    }

    /// Execute a custom query
    pub async fn query<T>(builder: QueryBuilder, db: &Database) -> Result<Vec<T>>
    where
        T: crate::Orso,
    {
        builder.execute::<T>(db).await
    }

    /// Execute a custom query with table override
    pub async fn query_with_table<T>(builder: QueryBuilder, db: &Database) -> Result<Vec<T>>
    where
        T: crate::Orso,
    {
        // Note: QueryBuilder already accepts custom table names in constructor
        // This method is provided for API consistency
        builder.execute::<T>(db).await
    }

    /// Execute a custom query with pagination
    pub async fn query_paginated<T>(
        builder: QueryBuilder,
        pagination: &Pagination,
        db: &Database,
    ) -> Result<PaginatedResult<T>>
    where
        T: crate::Orso,
    {
        builder.execute_paginated::<T>(db, pagination).await
    }

    /// Execute a custom query with pagination and table override
    pub async fn query_paginated_with_table<T>(
        builder: QueryBuilder,
        pagination: &Pagination,
        db: &Database,
    ) -> Result<PaginatedResult<T>>
    where
        T: crate::Orso,
    {
        // Note: QueryBuilder already accepts custom table names in constructor
        // This method is provided for API consistency
        builder.execute_paginated::<T>(db, pagination).await
    }

    /// Get aggregate value
    pub async fn aggregate<T>(
        function: Aggregate,
        column: &str,
        filter: Option<FilterOperator>,
        db: &Database,
    ) -> Result<Option<f64>>
    where
        T: crate::Orso,
    {
        Self::aggregate_with_table::<T>(function, column, filter, db, T::table_name()).await
    }

    pub async fn aggregate_with_table<T>(
        function: Aggregate,
        column: &str,
        filter: Option<FilterOperator>,
        db: &Database,
        table_name: &str,
    ) -> Result<Option<f64>>
    where
        T: crate::Orso,
    {
        let mut builder = QueryBuilder::new(table_name).aggregate(function, column, None::<String>);

        if let Some(filter) = filter {
            builder = builder._where(filter);
        }

        let (sql, params) = builder.build()?;
        let mut rows = db.conn.query(&sql, params).await?;

        if let Some(row) = rows.next().await? {
            let value = row
                .get_value(0)
                .ok()
                .and_then(|v| match v {
                    libsql::Value::Integer(i) => Some(i as f64),
                    libsql::Value::Real(f) => Some(f),
                    _ => None,
                })
                .ok_or_else(|| Error::Query("Failed to get aggregate value".to_string()))?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    /// Convert a database row to a HashMap
    pub fn row_to_map(row: &libsql::Row) -> Result<HashMap<String, crate::Value>> {
        let mut map = HashMap::new();
        for i in 0..row.column_count() {
            if let Some(column_name) = row.column_name(i) {
                let value = row.get_value(i).unwrap_or(libsql::Value::Null);
                map.insert(
                    column_name.to_string(),
                    Utils::libsql_value_to_value(&value),
                );
            }
        }
        Ok(map)
    }
}
