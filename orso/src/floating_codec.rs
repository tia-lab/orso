use anyhow::{anyhow, bail, Result};
use integer_encoding::{VarIntReader, VarIntWriter};
use rayon::prelude::*;
use std::io::Cursor;

#[derive(Clone, Copy, Debug)]
pub enum Codec {
    Lz4,
} // add Zstd later if you want

#[derive(Clone, Debug)]
pub struct FloatingCodec {
    pub codec: Codec,
}

impl Default for FloatingCodec {
    fn default() -> Self {
        Self { codec: Codec::Lz4 }
    }
}

impl FloatingCodec {
    // Default scaling factors for floating-point conversion
    pub const DEFAULT_F64_SCALE: f64 = 1_000_000_000.0; // 9 decimal places
    pub const DEFAULT_F32_SCALE: f32 = 1_000_000.0; // 6 decimal places

    #[inline]
    fn zigzag_i64(i: i64) -> u64 {
        ((i << 1) ^ (i >> 63)) as u64
    }

    #[inline]
    fn unzigzag_i64(u: u64) -> i64 {
        ((u >> 1) as i64) ^ (-((u & 1) as i64))
    }

    #[inline]
    fn zigzag_i32(i: i32) -> u32 {
        ((i << 1) ^ (i >> 31)) as u32
    }

    #[inline]
    fn unzigzag_i32(u: u32) -> i32 {
        ((u >> 1) as i32) ^ (-((u & 1) as i32))
    }

    /// Compress f64 vector by converting to scaled i64
    pub fn compress_f64(&self, data: &Vec<f64>, scale: Option<f64>) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        let scale_factor = scale.unwrap_or(Self::DEFAULT_F64_SCALE);
        let scaled_data: Vec<i64> = data
            .iter()
            .map(|&f| (f * scale_factor).round() as i64)
            .collect();

        // Compress as i64 but with f64 type identifier
        let mut buf = Vec::with_capacity(scaled_data.len() * 2);
        // header: magic + version + len + type
        buf.extend_from_slice(b"ORSO"); // 0..4
        buf.push(1); // 4: version
        buf.push(1); // 5: codec LZ4
        buf.push(4); // 6: type (4 = f64)
        buf.extend_from_slice(&(data.len() as u64).to_le_bytes()); // 7..15

        // Add scale factor to header (8 bytes for f64)
        buf.extend_from_slice(&scale_factor.to_le_bytes()); // 15..23

        // stream varints into a temp vec
        let mut tmp = Vec::with_capacity(scaled_data.len() * 2);
        let mut prev = 0i64;
        for &x in &scaled_data {
            let d = x.wrapping_sub(prev);
            prev = x;
            tmp.write_varint(Self::zigzag_i64(d)).unwrap();
        }

        // compress varint bytes
        let comp = lz4_flex::block::compress_prepend_size(&tmp);
        buf.extend_from_slice(&comp);
        Ok(buf)
    }

    /// Decompress f64 vector from scaled i64 data
    pub fn decompress_f64(&self, blob: &[u8], scale: Option<f64>) -> Result<Vec<f64>> {
        if blob.is_empty() {
            return Ok(Vec::new());
        }

        if blob.len() < 23 {
            // Minimum header size: 15 (base) + 8 (scale)
            bail!("blob too small");
        }

        if &blob[0..4] != b"ORSO" {
            bail!("bad magic");
        }

        if blob[4] != 1 {
            bail!("bad version");
        }

        if blob[5] != 1 {
            bail!("unsupported codec");
        }

        if blob[6] != 4 {
            bail!("unsupported type, expected f64");
        }

        let n = u64::from_le_bytes(blob[7..15].try_into().unwrap()) as usize;

        // Extract scale factor from blob or use provided
        let scale_factor = if let Some(s) = scale {
            s
        } else {
            f64::from_le_bytes(blob[15..23].try_into().unwrap())
        };

        let packed = lz4_flex::block::decompress_size_prepended(&blob[23..])
            .map_err(|e| anyhow!("lz4 decompress failed: {e}"))?;

        let mut cur = Cursor::new(packed.as_slice());
        let mut out = Vec::with_capacity(n);
        let mut acc = 0i64;
        for _ in 0..n {
            let v: u64 = cur
                .read_varint()
                .map_err(|e| anyhow!("varint decode: {e}"))?;
            let d = Self::unzigzag_i64(v);
            acc = acc.wrapping_add(d);
            out.push(acc);
        }

        // Convert back to f64 using scale factor
        let result: Vec<f64> = out.iter().map(|&i| i as f64 / scale_factor).collect();

        Ok(result)
    }

    /// Compress f32 vector by converting to scaled i32
    pub fn compress_f32(&self, data: &Vec<f32>, scale: Option<f32>) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        let scale_factor = scale.unwrap_or(Self::DEFAULT_F32_SCALE);
        let scaled_data: Vec<i32> = data
            .iter()
            .map(|&f| (f * scale_factor).round() as i32)
            .collect();

        // Compress as i32 but with f32 type identifier
        let mut buf = Vec::with_capacity(scaled_data.len() * 2);
        // header: magic + version + len + type
        buf.extend_from_slice(b"ORSO"); // 0..4
        buf.push(1); // 4: version
        buf.push(1); // 5: codec LZ4
        buf.push(5); // 6: type (5 = f32)
        buf.extend_from_slice(&(data.len() as u64).to_le_bytes()); // 7..15

        // Add scale factor to header (4 bytes for f32)
        buf.extend_from_slice(&scale_factor.to_le_bytes()); // 15..19

        // stream varints into a temp vec
        let mut tmp = Vec::with_capacity(scaled_data.len() * 2);
        let mut prev = 0i32;
        for &x in &scaled_data {
            let d = x.wrapping_sub(prev);
            prev = x;
            tmp.write_varint(Self::zigzag_i32(d)).unwrap();
        }

        // compress varint bytes
        let comp = lz4_flex::block::compress_prepend_size(&tmp);
        buf.extend_from_slice(&comp);
        Ok(buf)
    }

    /// Decompress f32 vector from scaled i32 data
    pub fn decompress_f32(&self, blob: &[u8], scale: Option<f32>) -> Result<Vec<f32>> {
        if blob.is_empty() {
            return Ok(Vec::new());
        }

        if blob.len() < 19 {
            // Minimum header size: 15 (base) + 4 (scale)
            bail!("blob too small");
        }

        if &blob[0..4] != b"ORSO" {
            bail!("bad magic");
        }

        if blob[4] != 1 {
            bail!("bad version");
        }

        if blob[5] != 1 {
            bail!("unsupported codec");
        }

        if blob[6] != 5 {
            bail!("unsupported type, expected f32");
        }

        let n = u64::from_le_bytes(blob[7..15].try_into().unwrap()) as usize;

        // Extract scale factor from blob or use provided
        let scale_factor = if let Some(s) = scale {
            s
        } else {
            f32::from_le_bytes(blob[15..19].try_into().unwrap())
        };

        let packed = lz4_flex::block::decompress_size_prepended(&blob[19..])
            .map_err(|e| anyhow!("lz4 decompress failed: {e}"))?;

        let mut cur = Cursor::new(packed.as_slice());
        let mut out = Vec::with_capacity(n);
        let mut acc = 0i32;
        for _ in 0..n {
            let v: u32 = cur
                .read_varint()
                .map_err(|e| anyhow!("varint decode: {e}"))?;
            let d = Self::unzigzag_i32(v);
            acc = acc.wrapping_add(d);
            out.push(acc);
        }

        // Convert back to f32 using scale factor
        let result: Vec<f32> = out.iter().map(|&i| i as f32 / scale_factor).collect();

        Ok(result)
    }

    /// Compress multiple f64 arrays
    pub fn compress_many_f64(
        &self,
        arrays: &[Vec<f64>],
        scales: Option<Vec<f64>>,
    ) -> Result<Vec<Vec<u8>>> {
        let default_scale = Self::DEFAULT_F64_SCALE;
        if let Some(scale_vec) = scales {
            arrays
                .par_iter()
                .zip(scale_vec.par_iter())
                .map(|(a, &s)| self.compress_f64(a, Some(s)))
                .collect()
        } else {
            arrays
                .par_iter()
                .map(|a| self.compress_f64(a, Some(default_scale)))
                .collect()
        }
    }

    /// Decompress multiple f64 arrays
    pub fn decompress_many_f64(
        &self,
        blobs: &[Vec<u8>],
        scales: Option<Vec<f64>>,
    ) -> Result<Vec<Vec<f64>>> {
        let default_scale = Self::DEFAULT_F64_SCALE;
        if let Some(scale_vec) = scales {
            blobs
                .par_iter()
                .zip(scale_vec.par_iter())
                .map(|(b, &s)| self.decompress_f64(b, Some(s)))
                .collect()
        } else {
            blobs
                .par_iter()
                .map(|b| self.decompress_f64(b, Some(default_scale)))
                .collect()
        }
    }

    /// Compress multiple f32 arrays
    pub fn compress_many_f32(
        &self,
        arrays: &[Vec<f32>],
        scales: Option<Vec<f32>>,
    ) -> Result<Vec<Vec<u8>>> {
        let default_scale = Self::DEFAULT_F32_SCALE;
        if let Some(scale_vec) = scales {
            arrays
                .par_iter()
                .zip(scale_vec.par_iter())
                .map(|(a, &s)| self.compress_f32(a, Some(s)))
                .collect()
        } else {
            arrays
                .par_iter()
                .map(|a| self.compress_f32(a, Some(default_scale)))
                .collect()
        }
    }

    /// Decompress multiple f32 arrays
    pub fn decompress_many_f32(
        &self,
        blobs: &[Vec<u8>],
        scales: Option<Vec<f32>>,
    ) -> Result<Vec<Vec<f32>>> {
        let default_scale = Self::DEFAULT_F32_SCALE;
        if let Some(scale_vec) = scales {
            blobs
                .par_iter()
                .zip(scale_vec.par_iter())
                .map(|(b, &s)| self.decompress_f32(b, Some(s)))
                .collect()
        } else {
            blobs
                .par_iter()
                .map(|b| self.decompress_f32(b, Some(default_scale)))
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{rngs::StdRng, Rng, SeedableRng};

    #[test]
    fn roundtrip_f64() -> Result<()> {
        let c = FloatingCodec::default();
        let v: Vec<f64> = (0..10_000).map(|i| i as f64 * 0.001).collect();
        let blob = c.compress_f64(&v, None)?;
        let back = c.decompress_f64(&blob, None)?;
        for (original, decompressed) in v.iter().zip(back.iter()) {
            assert!(
                (original - decompressed).abs() < 1e-12,
                "Values differ: {:?} vs {:?}",
                v,
                back
            );
        }
        Ok(())
    }

    #[test]
    fn roundtrip_f32() -> Result<()> {
        let c = FloatingCodec::default();
        let v: Vec<f32> = (0..10_000).map(|i| i as f32 * 0.001).collect();
        let blob = c.compress_f32(&v, None)?;
        let back = c.decompress_f32(&blob, None)?;
        for (original, decompressed) in v.iter().zip(back.iter()) {
            assert!(
                (original - decompressed).abs() < 1e-6,
                "Values differ: {:?} vs {:?}",
                v,
                back
            );
        }
        Ok(())
    }

    #[test]
    fn roundtrip_parallel_f64() -> Result<()> {
        let c = FloatingCodec::default();
        let arrays: Vec<Vec<f64>> = (0..64)
            .map(|k| (0..8192).map(|i| (i as f64 + k as f64) * 0.001).collect())
            .collect();
        let blobs = c.compress_many_f64(&arrays, None)?;
        let back = c.decompress_many_f64(&blobs, None)?;

        // For floating point, we need to use approximate equality with a very small tolerance
        for (original_array, decompressed_array) in arrays.iter().zip(back.iter()) {
            for (original, decompressed) in original_array.iter().zip(decompressed_array.iter()) {
                assert!(
                    (original - decompressed).abs() < 1e-12,
                    "Values differ: {} vs {}",
                    original,
                    decompressed
                );
            }
        }
        Ok(())
    }

    #[test]
    fn roundtrip_parallel_f32() -> Result<()> {
        let c = FloatingCodec::default();
        let arrays: Vec<Vec<f32>> = (0..64)
            .map(|k| (0..8192).map(|i| (i as f32 + k as f32) * 0.001).collect())
            .collect();
        let blobs = c.compress_many_f32(&arrays, None)?;
        let back = c.decompress_many_f32(&blobs, None)?;

        // For floating point, we need to use approximate equality with a small tolerance
        for (original_array, decompressed_array) in arrays.iter().zip(back.iter()) {
            for (original, decompressed) in original_array.iter().zip(decompressed_array.iter()) {
                assert!(
                    (original - decompressed).abs() < 1e-6,
                    "Values differ: {} vs {}",
                    original,
                    decompressed
                );
            }
        }
        Ok(())
    }

    #[test]
    fn randomish_f64_ok() -> Result<()> {
        let mut rng = StdRng::seed_from_u64(42);
        let v: Vec<f64> = (0..50_000).map(|_| rng.gen::<f64>() * 1000.0).collect();
        let c = FloatingCodec::default();
        let blob = c.compress_f64(&v, None)?;
        let back = c.decompress_f64(&blob, None)?;
        // For floating point, we need to use approximate equality
        for (original, decompressed) in v.iter().zip(back.iter()) {
            assert!(
                (original - decompressed).abs() < 1e-9,
                "Values differ: {} vs {}",
                original,
                decompressed
            );
        }
        Ok(())
    }

    #[test]
    fn randomish_f32_ok() -> Result<()> {
        let mut rng = StdRng::seed_from_u64(42);
        let v: Vec<f32> = (0..50_000).map(|_| rng.gen::<f32>() * 1000.0).collect();
        let c = FloatingCodec::default();
        let blob = c.compress_f32(&v, None)?;
        let back = c.decompress_f32(&blob, None)?;
        // For floating point, we need to use approximate equality
        for (original, decompressed) in v.iter().zip(back.iter()) {
            assert!(
                (original - decompressed).abs() < 1e-4,
                "Values differ: {} vs {}",
                original,
                decompressed
            );
        }
        Ok(())
    }

    #[test]
    fn report_metrics_ema_like_sizes_f64() -> Result<()> {
        use std::time::Instant;

        // helper: deterministic EMA-like series for u64
        fn ema_like_f64(len: usize) -> Vec<f64> {
            let mut out = Vec::with_capacity(len);
            // start around 117_000
            let mut ema: f64 = 117_100.0;
            let alpha = 2.0 / (9.0 + 1.0); // like EMA(9)
                                           // deterministic "price" signal: slow trend + small oscillations
            for i in 0..len {
                let t = i as f64;
                let price = 117_000.0
                    + 0.05 * t                              // tiny trend
                    + (t / 37.0).sin() * 30.0               // slow sine wiggle
                    + ((t / 5.0).sin() * 3.0).floor(); // tiny step noise
                ema = alpha * price + (1.0 - alpha) * ema;
                out.push(ema);
            }
            out
        }

        let codec = FloatingCodec::default();

        for &n in &[100usize, 1_000usize, 100_000usize] {
            let data = ema_like_f64(n);

            // compress
            let t0 = Instant::now();
            let blob = codec.compress_f64(&data, None)?;
            let comp_ms = t0.elapsed().as_secs_f64() * 1000.0;

            // decompress
            let t1 = Instant::now();
            let back = codec.decompress_f64(&blob, None)?;
            let decomp_ms = t1.elapsed().as_secs_f64() * 1000.0;

            // For floating point, we need to use approximate equality
            for (original, decompressed) in data.iter().zip(back.iter()) {
                assert!(
                    (original - decompressed).abs() < 1e-9,
                    "Values differ: {} vs {}",
                    original,
                    decompressed
                );
            }

            let raw_bytes = data.len() * 8;
            let comp_bytes = blob.len();
            let ratio = (raw_bytes as f64) / (comp_bytes.max(1) as f64);

            eprintln!(
                "f64 n={:<7} raw={:<10}B  comp={:<10}B  ratio={:>5.2}x  compress={:>6.3} ms  decompress={:>6.3} ms",
                n, raw_bytes, comp_bytes, ratio, comp_ms, decomp_ms
            );
        }

        Ok(())
    }

    #[test]
    fn report_metrics_ema_like_sizes_f32() -> Result<()> {
        use std::time::Instant;

        // helper: deterministic EMA-like series for f32
        fn ema_like_f32(len: usize) -> Vec<f32> {
            let mut out = Vec::with_capacity(len);
            // start around 117_000
            let mut ema: f32 = 117_100.0;
            let alpha = 2.0 / (9.0 + 1.0); // like EMA(9)
                                           // deterministic "price" signal: slow trend + small oscillations
            for i in 0..len {
                let t = i as f32;
                let price = 117_000.0
                + 0.05 * t                              // tiny trend
                + (t / 37.0).sin() * 30.0               // slow sine wiggle
                + ((t / 5.0).sin() * 3.0).floor(); // tiny step noise
                ema = alpha * price + (1.0 - alpha) * ema;
                out.push(ema);
            }
            out
        }

        let codec = FloatingCodec::default();
        
        // Use a smaller scale factor to avoid overflow
        let scale_factor = Some(1000.0); // Reduce from 1_000_000.0 to 1000.0

        for &n in &[100usize, 1_000usize, 100_000usize] {
            let data = ema_like_f32(n);

            // compress
            let t0 = Instant::now();
            let blob = codec.compress_f32(&data, scale_factor)?;
            let comp_ms = t0.elapsed().as_secs_f64() * 1000.0;

            // decompress
            let t1 = Instant::now();
            let back = codec.decompress_f32(&blob, scale_factor)?;
            let decomp_ms = t1.elapsed().as_secs_f64() * 1000.0;

            // For floating point, we need to use approximate equality
            for (original, decompressed) in data.iter().zip(back.iter()) {
                assert!(
                    (original - decompressed).abs() < 1e-1, // Increased tolerance for f32 with smaller scale
                    "Values differ: {} vs {}",
                    original,
                    decompressed
                );
            }

            let raw_bytes = data.len() * 4;
            let comp_bytes = blob.len();
            let ratio = (raw_bytes as f64) / (comp_bytes.max(1) as f64);

            eprintln!(
                "f32 n={:<7} raw={:<10}B  comp={:<10}B  ratio={:>5.2}x  compress={:>6.3} ms  decompress={:>6.3} ms",
                n, raw_bytes, comp_bytes, ratio, comp_ms, decomp_ms
            );
        }

        Ok(())
    }
}
