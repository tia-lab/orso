use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Attribute, Data, DeriveInput, Fields, Lit, parse_macro_input, punctuated::Punctuated,
    token::Comma,
};

#[proc_macro_attribute]
pub fn orso_column(_args: TokenStream, input: TokenStream) -> TokenStream {
    input
}

// orso_table attribute (passthrough - only used for table naming)
#[proc_macro_attribute]
pub fn orso_table(_args: TokenStream, input: TokenStream) -> TokenStream {
    input
}

// Derive macro for Orso trait
#[proc_macro_derive(Orso, attributes(orso_table, orso_column))]
pub fn derive_orso(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    // Extract table name from attributes or use default
    let table_name =
        extract_orso_table_name(&input.attrs).unwrap_or_else(|| name.to_string().to_lowercase());

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // Extract field metadata
    let (
        field_names,
        column_definitions,
        mathilde_field_types,
        nullable_flags,
        primary_key_field,
        created_at_field,
        updated_at_field,
        unique_fields,
    ) = if let Data::Struct(data) = &input.data {
        if let Fields::Named(fields) = &data.fields {
            extract_field_metadata_original(&fields.named)
        } else {
            (vec![], vec![], vec![], vec![], None, None, None, vec![])
        }
    } else {
        (vec![], vec![], vec![], vec![], None, None, None, vec![])
    };

    // Generate dynamic getters based on actual fields found
    let primary_key_getter = if let Some(ref pk_field) = primary_key_field {
        quote! {
            match &self.#pk_field {
                Some(pk) => Some(pk.to_string()),
                None => None,
            }
        }
    } else {
        quote! { None }
    };

    let primary_key_setter = if let Some(ref pk_field) = primary_key_field {
        quote! {
            if let Ok(parsed_id) = id.parse() {
                self.#pk_field = Some(parsed_id);
            }
        }
    } else {
        quote! { /* No primary key field found */ }
    };

    let created_at_getter = if let Some(ref ca_field) = created_at_field {
        quote! { self.#ca_field }
    } else {
        quote! { None }
    };

    let updated_at_getter = if let Some(ref ua_field) = updated_at_field {
        quote! { self.#ua_field }
    } else {
        quote! { None }
    };

    let updated_at_setter = if let Some(ref ua_field) = updated_at_field {
        quote! { self.#ua_field = Some(updated_at); }
    } else {
        quote! { /* No updated_at field found */ }
    };

    // Generate field name constants
    let primary_key_field_name = if let Some(ref pk_field) = primary_key_field {
        quote! { stringify!(#pk_field) }
    } else {
        quote! { "id" }
    };

    let created_at_field_name = if let Some(ref ca_field) = created_at_field {
        quote! { Some(stringify!(#ca_field)) }
    } else {
        quote! { None }
    };

    let updated_at_field_name = if let Some(ref ua_field) = updated_at_field {
        quote! { Some(stringify!(#ua_field)) }
    } else {
        quote! { None }
    };

    // Generate unique fields list
    let unique_field_names: Vec<proc_macro2::TokenStream> = unique_fields
        .iter()
        .map(|field| quote! { stringify!(#field) })
        .collect();

    // Generate only the trait implementation
    let expanded = quote! {
        impl #impl_generics orso::Orso for #name #ty_generics #where_clause {
            fn table_name() -> &'static str {
                #table_name
            }

            fn primary_key_field() -> &'static str {
                #primary_key_field_name
            }

            fn created_at_field() -> Option<&'static str> {
                #created_at_field_name
            }

            fn updated_at_field() -> Option<&'static str> {
                #updated_at_field_name
            }

            fn unique_fields() -> Vec<&'static str> {
                vec![#(#unique_field_names),*]
            }

            fn get_primary_key(&self) -> Option<String> {
                #primary_key_getter
            }

            fn set_primary_key(&mut self, id: String) {
                #primary_key_setter
            }

            fn get_created_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
                #created_at_getter
            }

            fn get_updated_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
                #updated_at_getter
            }

            fn set_updated_at(&mut self, updated_at: chrono::DateTime<chrono::Utc>) {
                #updated_at_setter
            }

            fn field_names() -> Vec<&'static str> {
                vec![#(#field_names),*]
            }

            fn field_types() -> Vec<orso::FieldType> {
                vec![#(#mathilde_field_types),*]
            }

            fn field_nullable() -> Vec<bool> {
                vec![#(#nullable_flags),*]
            }

            fn columns() -> Vec<&'static str> {
                vec![#(#field_names),*]
            }

            fn migration_sql() -> String {
                // Only generate columns for actual struct fields
                let columns: Vec<String> = vec![#(#column_definitions),*];

                format!(
                    "CREATE TABLE IF NOT EXISTS {} (\n    {}\n)",
                    Self::table_name(),
                    columns.join(",\n    ")
                )
            }

            fn to_map(&self) -> orso::Result<std::collections::HashMap<String, orso::Value>> {
                use serde_json;
                let json = serde_json::to_value(self)?;
                let map: std::collections::HashMap<String, serde_json::Value> =
                    serde_json::from_value(json)?;

                let mut result = std::collections::HashMap::new();

                // Get field names for auto-generated fields
                let pk_field = Self::primary_key_field();
                let created_field = Self::created_at_field();
                let updated_field = Self::updated_at_field();

                for (k, v) in map {
                    // Skip auto-generated fields when they are null - let SQLite use DEFAULT values
                    let should_skip = matches!(v, serde_json::Value::Null) && (
                        k == pk_field ||
                        (created_field.is_some() && k == created_field.unwrap()) ||
                        (updated_field.is_some() && k == updated_field.unwrap())
                    );

                    if should_skip {
                        continue;
                    }

                    let value = match v {
                        serde_json::Value::Null => orso::Value::Null,
                        serde_json::Value::Bool(b) => orso::Value::Boolean(b),
                        serde_json::Value::Number(n) => {
                            if let Some(i) = n.as_i64() {
                                orso::Value::Integer(i)
                            } else if let Some(f) = n.as_f64() {
                                orso::Value::Real(f)
                            } else {
                                orso::Value::Text(n.to_string())
                            }
                        }
                        serde_json::Value::String(s) => orso::Value::Text(s),
                        serde_json::Value::Array(_) => orso::Value::Text(serde_json::to_string(&v)?),
                        serde_json::Value::Object(_) => orso::Value::Text(serde_json::to_string(&v)?),
                    };
                    result.insert(k, value);
                }
                Ok(result)
            }

            fn from_map(mut map: std::collections::HashMap<String, orso::Value>) -> orso::Result<Self> {
                use serde_json;
                let mut json_map = serde_json::Map::new();

                // Get field metadata for type-aware conversion
                let field_names = Self::field_names();
                let field_types = Self::field_types();

                for (k, v) in &map {
                    // Don't skip any fields when deserializing FROM database - we want all values

                    let json_value = match v {
                        orso::Value::Null => serde_json::Value::Null,
                        orso::Value::Boolean(b) => serde_json::Value::Bool(*b),
                        orso::Value::Integer(i) => {
                            // Check if this field should be a boolean based on field type
                            if let Some(pos) = field_names.iter().position(|&name| name == k) {
                                if matches!(field_types.get(pos), Some(orso::FieldType::Boolean)) {
                                    // This is a boolean field, convert 0/1 to bool
                                    serde_json::Value::Bool(*i != 0)
                                } else {
                                    serde_json::Value::Number(serde_json::Number::from(*i))
                                }
                            } else {
                                serde_json::Value::Number(serde_json::Number::from(*i))
                            }
                        },
                        orso::Value::Real(f) => {
                            if let Some(n) = serde_json::Number::from_f64(*f) {
                                serde_json::Value::Number(n)
                            } else {
                                serde_json::Value::String(f.to_string())
                            }
                        }
                        orso::Value::Text(s) => {
                            // Check if this might be a SQLite datetime that needs conversion
                            if s.len() == 19 && s.chars().nth(4) == Some('-') && s.chars().nth(7) == Some('-') && s.chars().nth(10) == Some(' ') {
                                // This looks like SQLite datetime format: "2025-09-13 10:50:43"
                                // Convert to RFC3339 format: "2025-09-13T10:50:43Z"
                                let rfc3339_format = s.replace(' ', "T") + "Z";
                                serde_json::Value::String(rfc3339_format)
                            } else {
                                serde_json::Value::String(s.clone())
                            }
                        },
                        orso::Value::Blob(b) => {
                            serde_json::Value::Array(
                                b.iter()
                                .map(|byte| serde_json::Value::Number(serde_json::Number::from(*byte)))
                                .collect()
                            )
                        }
                    };
                    json_map.insert(k.clone(), json_value);
                }

                let json_value = serde_json::Value::Object(json_map);

                match serde_json::from_value(json_value) {
                    Ok(result) => Ok(result),
                    Err(e) => Err(orso::Error::Serialization(e.to_string()))
                }
            }


            // Utility methods
            fn row_to_map(row: &libsql::Row) -> orso::Result<std::collections::HashMap<String, orso::Value>> {
                let mut map = std::collections::HashMap::new();
                for i in 0..row.column_count() {
                    if let Some(column_name) = row.column_name(i) {
                        let value = row.get_value(i).unwrap_or(libsql::Value::Null);
                        map.insert(column_name.to_string(), Self::libsql_value_to_value(&value));
                    }
                }
                Ok(map)
            }

            fn value_to_libsql_value(value: &orso::Value) -> libsql::Value {
                match value {
                    orso::Value::Null => libsql::Value::Null,
                    orso::Value::Integer(i) => libsql::Value::Integer(*i),
                    orso::Value::Real(f) => libsql::Value::Real(*f),
                    orso::Value::Text(s) => libsql::Value::Text(s.clone()),
                    orso::Value::Blob(b) => libsql::Value::Blob(b.clone()),
                    orso::Value::Boolean(b) => libsql::Value::Integer(if *b { 1 } else { 0 }),
                }
            }

            fn libsql_value_to_value(value: &libsql::Value) -> orso::Value {
                match value {
                    libsql::Value::Null => orso::Value::Null,
                    libsql::Value::Integer(i) => {
                        // SQLite stores booleans as integers 0/1
                        // Check if this might be a boolean value
                        if *i == 0 || *i == 1 {
                            // This could be a boolean, but we don't have type context here
                            // For now, keep as integer and let from_map handle the conversion
                            orso::Value::Integer(*i)
                        } else {
                            orso::Value::Integer(*i)
                        }
                    },
                    libsql::Value::Real(f) => orso::Value::Real(*f),
                    libsql::Value::Text(s) => orso::Value::Text(s.clone()),
                    libsql::Value::Blob(b) => orso::Value::Blob(b.clone()),
                }
            }
        }
    };

    TokenStream::from(expanded)
}

// Parse field-level column definition with inline REFERENCES for maximum Turso compatibility
fn parse_field_column_definition(field: &syn::Field) -> String {
    let field_name = field.ident.as_ref().unwrap().to_string();

    // Check for orso_column attributes
    for attr in &field.attrs {
        if attr.path().is_ident("orso_column") {
            return parse_orso_column_attr(attr, &field_name, &field.ty);
        }
    }

    // Default column definition based on field type
    map_rust_type_to_sql_column(&field.ty, &field_name)
}

// Parse orso_column attribute with support for foreign keys
fn parse_orso_column_attr(
    attr: &syn::Attribute,
    field_name: &str,
    field_type: &syn::Type,
) -> String {
    let mut column_type = None;
    let mut is_foreign_key = false;
    let mut foreign_table = None;
    let mut unique = false;
    let mut primary_key = false;

    let mut is_created_at = false;
    let mut is_updated_at = false;

    let _ = attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("ref") {
            is_foreign_key = true;
            if let Ok(value) = meta.value() {
                let lit: Lit = value.parse()?;
                if let Lit::Str(lit_str) = lit {
                    foreign_table = Some(lit_str.value());
                }
            }
        } else if meta.path.is_ident("type") {
            if let Ok(value) = meta.value() {
                let lit: Lit = value.parse()?;
                if let Lit::Str(lit_str) = lit {
                    column_type = Some(lit_str.value());
                }
            }
        } else if meta.path.is_ident("unique") {
            unique = true;
        } else if meta.path.is_ident("primary_key") {
            primary_key = true;
        } else if meta.path.is_ident("created_at") {
            is_created_at = true;
        } else if meta.path.is_ident("updated_at") {
            is_updated_at = true;
        }
        Ok(())
    });

    // Generate column definition
    let base_type = if is_foreign_key {
        "TEXT".to_string() // Foreign keys are always TEXT (UUID)
    } else {
        column_type.unwrap_or_else(|| map_rust_type_to_sql_type(field_type))
    };

    let mut column_def = format!("{} {}", field_name, base_type);

    if primary_key {
        column_def.push_str(" PRIMARY KEY");
        // Add default for primary key if it's TEXT type
        if base_type == "TEXT" {
            column_def.push_str(" DEFAULT (lower(hex(randomblob(16))))");
        }
    }
    // Add NOT NULL for non-Option types (except primary keys which are already handled)
    if !is_option_type(field_type) && !primary_key {
        column_def.push_str(" NOT NULL");
    }
    if unique {
        column_def.push_str(" UNIQUE");
    }
    if let Some(ref_table) = foreign_table {
        column_def.push_str(&format!(" REFERENCES {}(id)", ref_table));
    }

    // Add defaults for timestamp columns
    if is_created_at || is_updated_at {
        column_def.push_str(" DEFAULT (datetime('now'))");
    }

    column_def
}

// Map Rust types to SQL column definitions
fn map_rust_type_to_sql_column(rust_type: &syn::Type, field_name: &str) -> String {
    let sql_type = map_rust_type_to_sql_type(rust_type);
    let mut column_def = format!("{} {}", field_name, sql_type);

    // Add NOT NULL for non-Option types
    if !is_option_type(rust_type) {
        column_def.push_str(" NOT NULL");
    }

    column_def
}

// Map Rust types to SQL types
fn map_rust_type_to_sql_type(rust_type: &syn::Type) -> String {
    if let syn::Type::Path(type_path) = rust_type {
        if let Some(segment) = type_path.path.segments.last() {
            let type_name = segment.ident.to_string();
            return match type_name.as_str() {
                "String" => "TEXT".to_string(),
                "i64" | "i32" | "i16" | "i8" => "INTEGER".to_string(),
                "u64" | "u32" | "u16" | "u8" => "INTEGER".to_string(),
                "f64" | "f32" => "REAL".to_string(),
                "bool" => "INTEGER".to_string(), // SQLite stores booleans as integers
                "Option" => {
                    // Handle Option<T> types
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() {
                            return map_rust_type_to_sql_type(inner_type);
                        }
                    }
                    "TEXT".to_string()
                }
                _ => "TEXT".to_string(),
            };
        }
    }
    "TEXT".to_string()
}

// Map field types to FieldType enum
fn map_field_type(rust_type: &syn::Type, _field: &syn::Field) -> proc_macro2::TokenStream {
    if let syn::Type::Path(type_path) = rust_type {
        if let Some(segment) = type_path.path.segments.last() {
            let type_name = segment.ident.to_string();
            return match type_name.as_str() {
                "String" => quote! { orso::FieldType::Text },
                "i64" => quote! { orso::FieldType::BigInt },
                "i32" | "i16" | "i8" => quote! { orso::FieldType::Integer },
                "u64" => quote! { orso::FieldType::BigInt },
                "u32" | "u16" | "u8" => quote! { orso::FieldType::Integer },
                "f64" | "f32" => quote! { orso::FieldType::Numeric },
                "bool" => quote! { orso::FieldType::Boolean },
                "Option" => {
                    // Handle Option<T> types - get the inner type
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() {
                            return map_field_type(inner_type, _field);
                        }
                    }
                    quote! { orso::FieldType::Text }
                }
                _ => quote! { orso::FieldType::Text },
            };
        }
    }
    quote! { orso::FieldType::Text }
}

// Check if a type is Option<T>
fn is_option_type(rust_type: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = rust_type {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}

// Extract field metadata from all struct fields
fn extract_field_metadata_original(
    fields: &Punctuated<syn::Field, Comma>,
) -> (
    Vec<proc_macro2::TokenStream>,
    Vec<proc_macro2::TokenStream>,
    Vec<proc_macro2::TokenStream>,
    Vec<bool>,
    Option<proc_macro2::Ident>,
    Option<proc_macro2::Ident>,
    Option<proc_macro2::Ident>,
    Vec<proc_macro2::Ident>,
) {
    let mut field_names = Vec::new();
    let mut column_defs = Vec::new();
    let mut field_types = Vec::new();
    let mut nullable_flags = Vec::new();
    let mut primary_key_field: Option<proc_macro2::Ident> = None;
    let mut created_at_field: Option<proc_macro2::Ident> = None;
    let mut updated_at_field: Option<proc_macro2::Ident> = None;
    let mut unique_fields = Vec::new();

    for field in fields {
        if let Some(field_name) = &field.ident {
            // Check for special attributes
            let mut is_primary_key = false;
            let mut is_created_at = false;
            let mut is_updated_at = false;
            let mut is_unique = false;

            for attr in &field.attrs {
                if attr.path().is_ident("orso_column") {
                    let _ = attr.parse_nested_meta(|meta| {
                        if meta.path.is_ident("primary_key") {
                            is_primary_key = true;
                            primary_key_field = Some(field_name.clone());
                        } else if meta.path.is_ident("created_at") {
                            is_created_at = true;
                            created_at_field = Some(field_name.clone());
                        } else if meta.path.is_ident("updated_at") {
                            is_updated_at = true;
                            updated_at_field = Some(field_name.clone());
                        } else if meta.path.is_ident("unique") {
                            is_unique = true;
                        }
                        Ok(())
                    });
                }
            }

            if is_unique {
                unique_fields.push(field_name.clone());
            }

            // Process ALL fields - no skipping based on field names

            let field_name_token = quote! { stringify!(#field_name) };
            field_names.push(field_name_token);

            // Parse column attributes for foreign key references (inline REFERENCES)
            let column_def = parse_field_column_definition(field);
            column_defs.push(quote! { #column_def.to_string() });

            // Enhanced type mapping based on field type and attributes
            let field_type = map_field_type(&field.ty, field);
            field_types.push(field_type);

            // Check if field is Option<T> (nullable)
            let is_nullable = is_option_type(&field.ty);
            nullable_flags.push(is_nullable);
        }
    }

    (
        field_names,
        column_defs,
        field_types,
        nullable_flags,
        primary_key_field,
        created_at_field,
        updated_at_field,
        unique_fields,
    )
}

// Extract table name from struct attributes
fn extract_orso_table_name(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("orso_table") {
            if let Ok(Lit::Str(lit_str)) = attr.parse_args::<Lit>() {
                return Some(lit_str.value());
            }
        }
    }
    None
}
