use orso::{
    migration, Database, DatabaseConfig, Migrations, Orso, Value, IntegerCodec, FloatingCodec
};
use serde::{Deserialize, Serialize};

#[derive(Orso, Serialize, Deserialize, Clone, Debug, Default)]
#[orso_table("compression_test")]
struct CompressionTest {
    #[orso_column(primary_key)]
    id: Option<String>,
    
    #[orso_column(compress)]
    int_data: Vec<i64>,
    
    #[orso_column(compress)]
    float_data: Vec<f64>,
    
    #[orso_column(compress)]
    u64_data: Vec<u64>,
    
    name: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a local database for testing
    let db_path = "test_compression.db";
    let config = DatabaseConfig::local(db_path);
    let db = Database::init(config).await?;
    
    // Create table
    Migrations::init(&db, &[migration!(CompressionTest)]).await?;
    
    // Create test data
    let test_data = CompressionTest {
        id: None,
        int_data: (0..10000).map(|i| i as i64 * 100).collect(),
        float_data: (0..10000).map(|i| i as f64 * 0.01).collect(),
        u64_data: (0..10000).map(|i| i as u64 * 200).collect(),
        name: "Test Data".to_string(),
    };
    
    println!("Original data sizes:");
    println!("  int_data: {} elements", test_data.int_data.len());
    println!("  float_data: {} elements", test_data.float_data.len());
    println!("  u64_data: {} elements", test_data.u64_data.len());
    
    // Test compression codecs directly
    let integer_codec = IntegerCodec::default();
    let floating_codec = FloatingCodec::default();
    
    // Compress data directly
    let compressed_int = integer_codec.compress_i64(&test_data.int_data)?;
    let compressed_float = floating_codec.compress_f64(&test_data.float_data, None)?;
    let compressed_u64 = integer_codec.compress_u64(&test_data.u64_data)?;
    
    println!("\
Direct compression results:");
    println!("  int_data: {} bytes (compressed from {} bytes)", compressed_int.len(), test_data.int_data.len() * 8);
    println!("  float_data: {} bytes (compressed from {} bytes)", compressed_float.len(), test_data.float_data.len() * 8);
    println!("  u64_data: {} bytes (compressed from {} bytes)", compressed_u64.len(), test_data.u64_data.len() * 8);
    
    println!("\
Compression ratios:");
    println!("  int_data: {:.2}x", (test_data.int_data.len() * 8) as f64 / compressed_int.len() as f64);
    println!("  float_data: {:.2}x", (test_data.float_data.len() * 8) as f64 / compressed_float.len() as f64);
    println!("  u64_data: {:.2}x", (test_data.u64_data.len() * 8) as f64 / compressed_u64.len() as f64);
    
    // Test decompression
    let decompressed_int = integer_codec.decompress_i64(&compressed_int)?;
    let decompressed_float = floating_codec.decompress_f64(&compressed_float, None)?;
    let decompressed_u64 = integer_codec.decompress_u64(&compressed_u64)?;
    
    println!("\
Decompression verification:");
    println!("  int_data matches: {}", decompressed_int == test_data.int_data);
    println!("  float_data matches: {}", decompressed_float.iter().zip(test_data.float_data.iter()).all(|(a, b)| (a - b).abs() < 1e-10));
    println!("  u64_data matches: {}", decompressed_u64 == test_data.u64_data);
    
    // Insert data into database
    test_data.insert(&db).await?;
    
    // Retrieve data from database
    let retrieved_records = CompressionTest::find_all(&db).await?;
    assert_eq!(retrieved_records.len(), 1);
    
    let retrieved = &retrieved_records[0];
    println!("\
Database retrieval verification:");
    println!("  Name matches: {}", retrieved.name == "Test Data");
    println!("  int_data length matches: {}", retrieved.int_data.len() == test_data.int_data.len());
    println!("  float_data length matches: {}", retrieved.float_data.len() == test_data.float_data.len());
    println!("  u64_data length matches: {}", retrieved.u64_data.len() == test_data.u64_data.len());
    
    // Check if data matches
    let int_matches = retrieved.int_data == test_data.int_data;
    let float_matches = retrieved.float_data.iter().zip(test_data.float_data.iter()).all(|(a, b)| (a - b).abs() < 1e-10);
    let u64_matches = retrieved.u64_data == test_data.u64_data;
    
    println!("  int_data matches: {}", int_matches);
    println!("  float_data matches: {}", float_matches);
    println!("  u64_data matches: {}", u64_matches);
    
    // Check the actual database to see what's stored
    println!("\
Checking database storage...");
    
    // Get the raw data from the database
    let mut rows = db.conn.query("SELECT int_data, float_data, u64_data FROM compression_test LIMIT 1", ()).await?;
    if let Some(row) = rows.next().await? {
        let int_blob: Vec<u8> = row.get(0)?;
        let float_blob: Vec<u8> = row.get(1)?;
        let u64_blob: Vec<u8> = row.get(2)?;
        
        println!("Raw database BLOB sizes:");
        println!("  int_data BLOB: {} bytes", int_blob.len());
        println!("  float_data BLOB: {} bytes", float_blob.len());
        println!("  u64_data BLOB: {} bytes", u64_blob.len());
        
        // Check if BLOBs have ORSO header
        if int_blob.len() >= 4 && &int_blob[0..4] == b"ORSO" {
            println!("  int_data BLOB has ORSO header ✓");
        } else {
            println!("  int_data BLOB does NOT have ORSO header ✗");
            println!("  First 16 bytes as hex: {}", hex::encode(&int_blob[0..std::cmp::min(16, int_blob.len())]));
        }
        
        if float_blob.len() >= 4 && &float_blob[0..4] == b"ORSO" {
            println!("  float_data BLOB has ORSO header ✓");
        } else {
            println!("  float_data BLOB does NOT have ORSO header ✗");
            println!("  First 16 bytes as hex: {}", hex::encode(&float_blob[0..std::cmp::min(16, float_blob.len())]));
        }
        
        if u64_blob.len() >= 4 && &u64_blob[0..4] == b"ORSO" {
            println!("  u64_data BLOB has ORSO header ✓");
        } else {
            println!("  u64_data BLOB does NOT have ORSO header ✗");
            println!("  First 16 bytes as hex: {}", hex::encode(&u64_blob[0..std::cmp::min(16, u64_blob.len())]));
        }
    }
    
    // Clean up
    std::fs::remove_file(db_path)?;
    
    println!("\
Test completed successfully!");
    Ok(())
}