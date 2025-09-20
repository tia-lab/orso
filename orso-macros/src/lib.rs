use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, punctuated::Punctuated, token::Comma, Attribute, Data, DeriveInput, Fields,
    Lit,
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
        compressed_fields, // New compression flags
    ) = if let Data::Struct(data) = &input.data {
        if let Fields::Named(fields) = &data.fields {
            extract_field_metadata_original(&fields.named)
        } else {
            (
                vec![],
                vec![],
                vec![],
                vec![],
                None,
                None,
                None,
                vec![],
                vec![],
            )
        }
    } else {
        (
            vec![],
            vec![],
            vec![],
            vec![],
            None,
            None,
            None,
            vec![],
            vec![],
        )
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

    // Generate compressed fields list
    let compressed_field_flags: Vec<proc_macro2::TokenStream> = compressed_fields
        .iter()
        .map(|&is_compressed| quote! { #is_compressed })
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

            fn field_compressed() -> Vec<bool> {
                vec![#(#compressed_field_flags),*]
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

                // Get compression information
                let field_names = Self::field_names();
                let compressed_flags = Self::field_compressed();

                // Group compressed fields by type for batch processing
                let mut compressed_i64_fields: std::collections::HashMap<String, Vec<i64>> = std::collections::HashMap::new();
                let mut compressed_u64_fields: std::collections::HashMap<String, Vec<u64>> = std::collections::HashMap::new();
                let mut compressed_i32_fields: std::collections::HashMap<String, Vec<i32>> = std::collections::HashMap::new();
                let mut compressed_u32_fields: std::collections::HashMap<String, Vec<u32>> = std::collections::HashMap::new();

                // First pass: collect compressed fields by type
                for (k, v) in &map {
                    // Skip auto-generated fields when they are null - let SQLite use DEFAULT values
                    let should_skip = matches!(v, serde_json::Value::Null) && (
                        *k == pk_field ||
                        (created_field.is_some() && *k == created_field.unwrap()) ||
                        (updated_field.is_some() && *k == updated_field.unwrap())
                    );

                    if should_skip {
                        continue;
                    }

                    // Check if this field should be compressed
                    let is_compressed = field_names.iter().position(|&name| name == *k)
                        .and_then(|pos| compressed_flags.get(pos).copied())
                        .unwrap_or(false);

                    if is_compressed {
                        // Handle compressed fields by collecting them for batch processing
                        match v {
                            serde_json::Value::Array(arr) => {
                                // Determine the element type of the array and collect accordingly
                                // For now, we'll try to convert to the most common integer types
                                let i64_result: Result<Vec<i64>, _> = arr.iter().map(|val| {
                                    match val {
                                        serde_json::Value::Number(n) => {
                                            n.as_i64().ok_or_else(|| "Invalid i64 value".to_string())
                                        }
                                        _ => Err("Non-numeric value in array".to_string()),
                                    }
                                }).collect();
                                
                                if let Ok(vec) = i64_result {
                                    compressed_i64_fields.insert(k.clone(), vec);
                                    continue; // Skip normal processing for this field
                                }
                                
                                // Try u64
                                let u64_result: Result<Vec<u64>, _> = arr.iter().map(|val| {
                                    match val {
                                        serde_json::Value::Number(n) => {
                                            n.as_u64().ok_or_else(|| "Invalid u64 value".to_string())
                                        }
                                        _ => Err("Non-numeric value in array".to_string()),
                                    }
                                }).collect();
                                
                                if let Ok(vec) = u64_result {
                                    compressed_u64_fields.insert(k.clone(), vec);
                                    continue; // Skip normal processing for this field
                                }
                                
                                // Try i32
                                let i32_result: Result<Vec<i32>, _> = arr.iter().map(|val| {
                                    match val {
                                        serde_json::Value::Number(n) => {
                                            n.as_i64().and_then(|i| i32::try_from(i).ok()).ok_or_else(|| "Invalid i32 value".to_string())
                                        }
                                        _ => Err("Non-numeric value in array".to_string()),
                                    }
                                }).collect();
                                
                                if let Ok(vec) = i32_result {
                                    compressed_i32_fields.insert(k.clone(), vec);
                                    continue; // Skip normal processing for this field
                                }
                                
                                // Try u32
                                let u32_result: Result<Vec<u32>, _> = arr.iter().map(|val| {
                                    match val {
                                        serde_json::Value::Number(n) => {
                                            n.as_u64().and_then(|u| u32::try_from(u).ok()).ok_or_else(|| "Invalid u32 value".to_string())
                                        }
                                        _ => Err("Non-numeric value in array".to_string()),
                                    }
                                }).collect();
                                
                                if let Ok(vec) = u32_result {
                                    compressed_u32_fields.insert(k.clone(), vec);
                                    continue; // Skip normal processing for this field
                                }
                            }
                            _ => {} // Fall through to normal processing
                        }
                    }
                }

                // Batch process compressed fields by type
                // Process i64 fields
                if !compressed_i64_fields.is_empty() {
                    let codec = orso::IntegerCodec::default();
                    if compressed_i64_fields.len() == 1 {
                        // Single field - process individually
                        let (field_name, vec) = compressed_i64_fields.into_iter().next().unwrap();
                        match codec.compress_i64(&vec) {
                            Ok(compressed) => {
                                result.insert(field_name, orso::Value::Blob(compressed));
                            }
                            Err(_) => {
                                // Fallback to JSON string
                                if let Some(original_value) = map.get(&field_name) {
                                    result.insert(field_name, orso::Value::Text(serde_json::to_string(original_value)?));
                                }
                            }
                        }
                    } else {
                        // Multiple fields - process in batch
                        let field_names: Vec<String> = compressed_i64_fields.keys().cloned().collect();
                        let arrays: Vec<Vec<i64>> = compressed_i64_fields.values().cloned().collect();
                        
                        match codec.compress_many_i64(&arrays) {
                            Ok(compressed_blobs) => {
                                for (field_name, blob) in field_names.into_iter().zip(compressed_blobs.into_iter()) {
                                    result.insert(field_name, orso::Value::Blob(blob));
                                }
                            }
                            Err(_) => {
                                // Fallback to individual compression
                                for (field_name, vec) in compressed_i64_fields {
                                    match codec.compress_i64(&vec) {
                                        Ok(compressed) => {
                                            result.insert(field_name, orso::Value::Blob(compressed));
                                        }
                                        Err(_) => {
                                            // Ultimate fallback to JSON string
                                            if let Some(original_value) = map.get(&field_name) {
                                                result.insert(field_name, orso::Value::Text(serde_json::to_string(original_value)?));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Process u64 fields
                if !compressed_u64_fields.is_empty() {
                    let codec = orso::IntegerCodec::default();
                    if compressed_u64_fields.len() == 1 {
                        // Single field - process individually
                        let (field_name, vec) = compressed_u64_fields.into_iter().next().unwrap();
                        match codec.compress_u64(&vec) {
                            Ok(compressed) => {
                                result.insert(field_name, orso::Value::Blob(compressed));
                            }
                            Err(_) => {
                                // Fallback to JSON string
                                if let Some(original_value) = map.get(&field_name) {
                                    result.insert(field_name, orso::Value::Text(serde_json::to_string(original_value)?));
                                }
                            }
                        }
                    } else {
                        // Multiple fields - process in batch
                        let field_names: Vec<String> = compressed_u64_fields.keys().cloned().collect();
                        let arrays: Vec<Vec<u64>> = compressed_u64_fields.values().cloned().collect();
                        
                        match codec.compress_many_u64(&arrays) {
                            Ok(compressed_blobs) => {
                                for (field_name, blob) in field_names.into_iter().zip(compressed_blobs.into_iter()) {
                                    result.insert(field_name, orso::Value::Blob(blob));
                                }
                            }
                            Err(_) => {
                                // Fallback to individual compression
                                for (field_name, vec) in compressed_u64_fields {
                                    match codec.compress_u64(&vec) {
                                        Ok(compressed) => {
                                            result.insert(field_name, orso::Value::Blob(compressed));
                                        }
                                        Err(_) => {
                                            // Ultimate fallback to JSON string
                                            if let Some(original_value) = map.get(&field_name) {
                                                result.insert(field_name, orso::Value::Text(serde_json::to_string(original_value)?));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Process i32 fields (compress as i64 for storage efficiency)
                if !compressed_i32_fields.is_empty() {
                    let codec = orso::IntegerCodec::default();
                    if compressed_i32_fields.len() == 1 {
                        // Single field - process individually
                        let (field_name, vec) = compressed_i32_fields.into_iter().next().unwrap();
                        let i64_vec: Vec<i64> = vec.into_iter().map(|x| x as i64).collect();
                        match codec.compress_i64(&i64_vec) {
                            Ok(compressed) => {
                                result.insert(field_name, orso::Value::Blob(compressed));
                            }
                            Err(_) => {
                                // Fallback to JSON string
                                if let Some(original_value) = map.get(&field_name) {
                                    result.insert(field_name, orso::Value::Text(serde_json::to_string(original_value)?));
                                }
                            }
                        }
                    } else {
                        // Multiple fields - process in batch
                        let field_names: Vec<String> = compressed_i32_fields.keys().cloned().collect();
                        let arrays: Vec<Vec<i64>> = compressed_i32_fields.values().map(|vec| vec.iter().map(|&x| x as i64).collect()).collect();
                        
                        match codec.compress_many_i64(&arrays) {
                            Ok(compressed_blobs) => {
                                for (field_name, blob) in field_names.into_iter().zip(compressed_blobs.into_iter()) {
                                    result.insert(field_name, orso::Value::Blob(blob));
                                }
                            }
                            Err(_) => {
                                // Fallback to individual compression
                                for (field_name, vec) in compressed_i32_fields {
                                    let i64_vec: Vec<i64> = vec.into_iter().map(|x| x as i64).collect();
                                    match codec.compress_i64(&i64_vec) {
                                        Ok(compressed) => {
                                            result.insert(field_name, orso::Value::Blob(compressed));
                                        }
                                        Err(_) => {
                                            // Ultimate fallback to JSON string
                                            if let Some(original_value) = map.get(&field_name) {
                                                result.insert(field_name, orso::Value::Text(serde_json::to_string(original_value)?));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Process u32 fields (compress as u64 for storage efficiency)
                if !compressed_u32_fields.is_empty() {
                    let codec = orso::IntegerCodec::default();
                    if compressed_u32_fields.len() == 1 {
                        // Single field - process individually
                        let (field_name, vec) = compressed_u32_fields.into_iter().next().unwrap();
                        let u64_vec: Vec<u64> = vec.into_iter().map(|x| x as u64).collect();
                        match codec.compress_u64(&u64_vec) {
                            Ok(compressed) => {
                                result.insert(field_name, orso::Value::Blob(compressed));
                            }
                            Err(_) => {
                                // Fallback to JSON string
                                if let Some(original_value) = map.get(&field_name) {
                                    result.insert(field_name, orso::Value::Text(serde_json::to_string(original_value)?));
                                }
                            }
                        }
                    } else {
                        // Multiple fields - process in batch
                        let field_names: Vec<String> = compressed_u32_fields.keys().cloned().collect();
                        let arrays: Vec<Vec<u64>> = compressed_u32_fields.values().map(|vec| vec.iter().map(|&x| x as u64).collect()).collect();
                        
                        match codec.compress_many_u64(&arrays) {
                            Ok(compressed_blobs) => {
                                for (field_name, blob) in field_names.into_iter().zip(compressed_blobs.into_iter()) {
                                    result.insert(field_name, orso::Value::Blob(blob));
                                }
                            }
                            Err(_) => {
                                // Fallback to individual compression
                                for (field_name, vec) in compressed_u32_fields {
                                    let u64_vec: Vec<u64> = vec.into_iter().map(|x| x as u64).collect();
                                    match codec.compress_u64(&u64_vec) {
                                        Ok(compressed) => {
                                            result.insert(field_name, orso::Value::Blob(compressed));
                                        }
                                        Err(_) => {
                                            // Ultimate fallback to JSON string
                                            if let Some(original_value) = map.get(&field_name) {
                                                result.insert(field_name, orso::Value::Text(serde_json::to_string(original_value)?));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Second pass: process non-compressed fields and any fields that fell through
                for (k, v) in map {
                    // Skip fields that were already processed as compressed
                    if result.contains_key(&k) {
                        continue;
                    }

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
                let compressed_flags = Self::field_compressed();

                // Group compressed fields by type for batch processing
                let mut compressed_i64_blobs: std::collections::HashMap<String, Vec<u8>> = std::collections::HashMap::new();
                let mut compressed_u64_blobs: std::collections::HashMap<String, Vec<u8>> = std::collections::HashMap::new();
                let mut compressed_i32_blobs: std::collections::HashMap<String, Vec<u8>> = std::collections::HashMap::new();
                let mut compressed_u32_blobs: std::collections::HashMap<String, Vec<u8>> = std::collections::HashMap::new();

                // First pass: collect compressed fields by type
                for (k, v) in &map {
                    // Check if this field should be decompressed
                    let is_compressed = field_names.iter().position(|&name| name == *k)
                        .and_then(|pos| compressed_flags.get(pos).copied())
                        .unwrap_or(false);

                    if is_compressed {
                        match v {
                            orso::Value::Blob(blob) => {
                                // For now, we'll assume i64 for all compressed fields
                                // In a more sophisticated implementation, we might store type info in the blob header
                                compressed_i64_blobs.insert(k.clone(), blob.clone());
                            }
                            _ => {
                                // Non-blob compressed fields - handle individually
                                let json_value = match v {
                                    orso::Value::Text(s) => {
                                        // Try to parse as JSON array
                                        match serde_json::from_str(s) {
                                            Ok(val) => val,
                                            Err(_) => serde_json::Value::String(s.clone()),
                                        }
                                    }
                                    orso::Value::Null => serde_json::Value::Null,
                                    orso::Value::Boolean(b) => serde_json::Value::Bool(*b),
                                    orso::Value::Integer(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
                                    orso::Value::Real(f) => {
                                        if let Some(n) = serde_json::Number::from_f64(*f) {
                                            serde_json::Value::Number(n)
                                        } else {
                                            serde_json::Value::String(f.to_string())
                                        }
                                    }
                                    orso::Value::Blob(blob) => {
                                        // This shouldn't happen for compressed fields that are already blobs
                                        serde_json::Value::Array(
                                            blob.iter()
                                            .map(|byte| serde_json::Value::Number(serde_json::Number::from(*byte)))
                                            .collect()
                                        )
                                    }
                                };
                                json_map.insert(k.clone(), json_value);
                            }
                        }
                    }
                }

                // Batch process compressed fields by type
                // Process i64 fields
                if !compressed_i64_blobs.is_empty() {
                    let codec = orso::IntegerCodec::default();
                    if compressed_i64_blobs.len() == 1 {
                        // Single field - process individually
                        let (field_name, blob) = compressed_i64_blobs.into_iter().next().unwrap();
                        match codec.decompress_i64(&blob) {
                            Ok(vec) => {
                                // Convert Vec<i64> to serde_json::Value::Array
                                let json_array = serde_json::Value::Array(
                                    vec.into_iter().map(|i| serde_json::Value::Number(serde_json::Number::from(i))).collect()
                                );
                                json_map.insert(field_name, json_array);
                            }
                            Err(e) => {
                                // If decompression fails, return the raw data as a string
                                let error_msg = format!("Failed to decompress: {:?}", blob);
                                json_map.insert(field_name, serde_json::Value::String(error_msg));
                            }
                        }
                    } else {
                        // Multiple fields - process in batch
                        let field_names: Vec<String> = compressed_i64_blobs.keys().cloned().collect();
                        let blobs: Vec<Vec<u8>> = compressed_i64_blobs.values().cloned().collect();
                        
                        match codec.decompress_many_i64(&blobs) {
                            Ok(arrays) => {
                                for (field_name, vec) in field_names.into_iter().zip(arrays.into_iter()) {
                                    // Convert Vec<i64> to serde_json::Value::Array
                                    let json_array = serde_json::Value::Array(
                                        vec.into_iter().map(|i| serde_json::Value::Number(serde_json::Number::from(i))).collect()
                                    );
                                    json_map.insert(field_name, json_array);
                                }
                            }
                            Err(_) => {
                                // Fallback to individual decompression
                                for (field_name, blob) in compressed_i64_blobs {
                                    match codec.decompress_i64(&blob) {
                                        Ok(vec) => {
                                            // Convert Vec<i64> to serde_json::Value::Array
                                            let json_array = serde_json::Value::Array(
                                                vec.into_iter().map(|i| serde_json::Value::Number(serde_json::Number::from(i))).collect()
                                            );
                                            json_map.insert(field_name, json_array);
                                        }
                                        Err(_) => {
                                            // Ultimate fallback to raw blob data as string
                                            let error_msg = format!("Failed to decompress blob for field: {}", field_name);
                                            json_map.insert(field_name, serde_json::Value::String(error_msg));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Process u64 fields (currently we don't distinguish u64 from i64 in decompression)
                if !compressed_u64_blobs.is_empty() {
                    let codec = orso::IntegerCodec::default();
                    if compressed_u64_blobs.len() == 1 {
                        // Single field - process individually
                        let (field_name, blob) = compressed_u64_blobs.into_iter().next().unwrap();
                        match codec.decompress_u64(&blob) {
                            Ok(vec) => {
                                // Convert Vec<u64> to serde_json::Value::Array
                                let json_array = serde_json::Value::Array(
                                    vec.into_iter().map(|i| serde_json::Value::Number(serde_json::Number::from(i))).collect()
                                );
                                json_map.insert(field_name, json_array);
                            }
                            Err(e) => {
                                // If decompression fails, return the raw data as a string
                                let error_msg = format!("Failed to decompress: {:?}", blob);
                                json_map.insert(field_name, serde_json::Value::String(error_msg));
                            }
                        }
                    } else {
                        // Multiple fields - process in batch
                        let field_names: Vec<String> = compressed_u64_blobs.keys().cloned().collect();
                        let blobs: Vec<Vec<u8>> = compressed_u64_blobs.values().cloned().collect();
                        
                        match codec.decompress_many_u64(&blobs) {
                            Ok(arrays) => {
                                for (field_name, vec) in field_names.into_iter().zip(arrays.into_iter()) {
                                    // Convert Vec<u64> to serde_json::Value::Array
                                    let json_array = serde_json::Value::Array(
                                        vec.into_iter().map(|i| serde_json::Value::Number(serde_json::Number::from(i))).collect()
                                    );
                                    json_map.insert(field_name, json_array);
                                }
                            }
                            Err(_) => {
                                // Fallback to individual decompression
                                for (field_name, blob) in compressed_u64_blobs {
                                    match codec.decompress_u64(&blob) {
                                        Ok(vec) => {
                                            // Convert Vec<u64> to serde_json::Value::Array
                                            let json_array = serde_json::Value::Array(
                                                vec.into_iter().map(|i| serde_json::Value::Number(serde_json::Number::from(i))).collect()
                                            );
                                            json_map.insert(field_name, json_array);
                                        }
                                        Err(_) => {
                                            // Ultimate fallback to raw blob data as string
                                            let error_msg = format!("Failed to decompress blob for field: {}", field_name);
                                            json_map.insert(field_name, serde_json::Value::String(error_msg));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Process i32 fields (convert from i64 back to i32)
                if !compressed_i32_blobs.is_empty() {
                    let codec = orso::IntegerCodec::default();
                    if compressed_i32_blobs.len() == 1 {
                        // Single field - process individually
                        let (field_name, blob) = compressed_i32_blobs.into_iter().next().unwrap();
                        match codec.decompress_i64(&blob) {
                            Ok(vec) => {
                                // Convert Vec<i64> to Vec<i32> and then to serde_json::Value::Array
                                let json_array = serde_json::Value::Array(
                                    vec.into_iter().map(|i| i32::try_from(i).unwrap_or(i as i32))
                                    .map(|i| serde_json::Value::Number(serde_json::Number::from(i))).collect()
                                );
                                json_map.insert(field_name, json_array);
                            }
                            Err(e) => {
                                // If decompression fails, return the raw data as a string
                                let error_msg = format!("Failed to decompress: {:?}", blob);
                                json_map.insert(field_name, serde_json::Value::String(error_msg));
                            }
                        }
                    } else {
                        // Multiple fields - process in batch
                        let field_names: Vec<String> = compressed_i32_blobs.keys().cloned().collect();
                        let blobs: Vec<Vec<u8>> = compressed_i32_blobs.values().cloned().collect();
                        
                        match codec.decompress_many_i64(&blobs) {
                            Ok(arrays) => {
                                for (field_name, vec) in field_names.into_iter().zip(arrays.into_iter()) {
                                    // Convert Vec<i64> to Vec<i32> and then to serde_json::Value::Array
                                    let json_array = serde_json::Value::Array(
                                        vec.into_iter().map(|i| i32::try_from(i).unwrap_or(i as i32))
                                        .map(|i| serde_json::Value::Number(serde_json::Number::from(i))).collect()
                                    );
                                    json_map.insert(field_name, json_array);
                                }
                            }
                            Err(_) => {
                                // Fallback to individual decompression
                                for (field_name, blob) in compressed_i32_blobs {
                                    match codec.decompress_i64(&blob) {
                                        Ok(vec) => {
                                            // Convert Vec<i64> to Vec<i32> and then to serde_json::Value::Array
                                            let json_array = serde_json::Value::Array(
                                                vec.into_iter().map(|i| i32::try_from(i).unwrap_or(i as i32))
                                                .map(|i| serde_json::Value::Number(serde_json::Number::from(i))).collect()
                                            );
                                            json_map.insert(field_name, json_array);
                                        }
                                        Err(_) => {
                                            // Ultimate fallback to raw blob data as string
                                            let error_msg = format!("Failed to decompress blob for field: {}", field_name);
                                            json_map.insert(field_name, serde_json::Value::String(error_msg));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Process u32 fields (convert from u64 back to u32)
                if !compressed_u32_blobs.is_empty() {
                    let codec = orso::IntegerCodec::default();
                    if compressed_u32_blobs.len() == 1 {
                        // Single field - process individually
                        let (field_name, blob) = compressed_u32_blobs.into_iter().next().unwrap();
                        match codec.decompress_u64(&blob) {
                            Ok(vec) => {
                                // Convert Vec<u64> to Vec<u32> and then to serde_json::Value::Array
                                let json_array = serde_json::Value::Array(
                                    vec.into_iter().map(|i| u32::try_from(i).unwrap_or(i as u32))
                                    .map(|i| serde_json::Value::Number(serde_json::Number::from(i))).collect()
                                );
                                json_map.insert(field_name, json_array);
                            }
                            Err(e) => {
                                // If decompression fails, return the raw data as a string
                                let error_msg = format!("Failed to decompress: {:?}", blob);
                                json_map.insert(field_name, serde_json::Value::String(error_msg));
                            }
                        }
                    } else {
                        // Multiple fields - process in batch
                        let field_names: Vec<String> = compressed_u32_blobs.keys().cloned().collect();
                        let blobs: Vec<Vec<u8>> = compressed_u32_blobs.values().cloned().collect();
                        
                        match codec.decompress_many_u64(&blobs) {
                            Ok(arrays) => {
                                for (field_name, vec) in field_names.into_iter().zip(arrays.into_iter()) {
                                    // Convert Vec<u64> to Vec<u32> and then to serde_json::Value::Array
                                    let json_array = serde_json::Value::Array(
                                        vec.into_iter().map(|i| u32::try_from(i).unwrap_or(i as u32))
                                        .map(|i| serde_json::Value::Number(serde_json::Number::from(i))).collect()
                                    );
                                    json_map.insert(field_name, json_array);
                                }
                            }
                            Err(_) => {
                                // Fallback to individual decompression
                                for (field_name, blob) in compressed_u32_blobs {
                                    match codec.decompress_u64(&blob) {
                                        Ok(vec) => {
                                            // Convert Vec<u64> to Vec<u32> and then to serde_json::Value::Array
                                            let json_array = serde_json::Value::Array(
                                                vec.into_iter().map(|i| u32::try_from(i).unwrap_or(i as u32))
                                                .map(|i| serde_json::Value::Number(serde_json::Number::from(i))).collect()
                                            );
                                            json_map.insert(field_name, json_array);
                                        }
                                        Err(_) => {
                                            // Ultimate fallback to raw blob data as string
                                            let error_msg = format!("Failed to decompress blob for field: {}", field_name);
                                            json_map.insert(field_name, serde_json::Value::String(error_msg));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Process non-compressed fields and any fields that fell through
                for (k, v) in &map {
                    // Skip fields that were already processed as compressed
                    if json_map.contains_key(k) {
                        continue;
                    }

                    let json_value = match v {
                        orso::Value::Null => serde_json::Value::Null,
                        orso::Value::Boolean(b) => serde_json::Value::Bool(*b),
                        orso::Value::Integer(i) => {
                            // Check if this field should be a boolean based on field type
                            if let Some(pos) = field_names.iter().position(|&name| name == *k) {
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

// Parse orso_column attribute with support for foreign keys and compression
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
    let mut is_compressed = false;

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
        } else if meta.path.is_ident("compress") {
            is_compressed = true;
        }
        Ok(())
    });

    // Generate column definition
    // For compressed fields, we always use BLOB type
    let base_type = if is_compressed {
        "BLOB".to_string()
    } else if is_foreign_key {
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
        column_def.push_str(" DEFAULT (strftime('%Y-%m-%dT%H:%M:%S.000Z', 'now'))");
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
    Vec<bool>, // Compression flags
) {
    let mut field_names = Vec::new();
    let mut column_defs = Vec::new();
    let mut field_types = Vec::new();
    let mut nullable_flags = Vec::new();
    let mut primary_key_field: Option<proc_macro2::Ident> = None;
    let mut created_at_field: Option<proc_macro2::Ident> = None;
    let mut updated_at_field: Option<proc_macro2::Ident> = None;
    let mut unique_fields = Vec::new();
    let mut compressed_fields = Vec::new(); // New vector for compression flags

    for field in fields {
        if let Some(field_name) = &field.ident {
            // Check for special attributes
            let mut is_primary_key = false;
            let mut is_created_at = false;
            let mut is_updated_at = false;
            let mut is_unique = false;
            let mut is_compressed = false; // Track compression

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
                        } else if meta.path.is_ident("compress") {
                            is_compressed = true;
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

            // Store compression flag
            compressed_fields.push(is_compressed);
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
        compressed_fields, // Return compression flags
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
