use anyhow::{anyhow, bail, Result};
use integer_encoding::{VarIntReader, VarIntWriter};
use rayon::prelude::*;
use std::io::Cursor;

#[derive(Clone, Copy, Debug)]
pub enum Codec {
    Lz4,
} // add Zstd later if you want

#[derive(Clone, Debug)]
pub struct IntegerCodec {
    pub codec: Codec,
}

impl Default for IntegerCodec {
    fn default() -> Self {
        Self { codec: Codec::Lz4 }
    }
}

impl IntegerCodec {
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

    pub fn compress_i64(&self, data: &Vec<i64>) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        // delta + zigzag → varint
        let mut buf = Vec::with_capacity(data.len() * 2);
        // header: magic + version + len + type
        buf.extend_from_slice(b"ORSO"); // 0..4
        buf.push(1); // 4: version
        buf.push(1); // 5: codec LZ4
        buf.push(0); // 6: type (0 = i64)
        buf.extend_from_slice(&(data.len() as u64).to_le_bytes()); // 7..15

        // stream varints into a temp vec
        let mut tmp = Vec::with_capacity(data.len() * 2);
        let mut prev = 0i64;
        for &x in data {
            let d = x.wrapping_sub(prev);
            prev = x;
            tmp.write_varint(Self::zigzag_i64(d)).unwrap();
        }

        // compress varint bytes
        let comp = lz4_flex::block::compress_prepend_size(&tmp);
        buf.extend_from_slice(&comp);
        Ok(buf)
    }

    pub fn decompress_i64(&self, blob: &[u8]) -> Result<Vec<i64>> {
        if blob.is_empty() {
            return Ok(Vec::new());
        }
        if blob.len() < 15 {
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
        if blob[6] != 0 {
            bail!("unsupported type, expected i64");
        }
        let n = u64::from_le_bytes(blob[7..15].try_into().unwrap()) as usize;

        let packed = lz4_flex::block::decompress_size_prepended(&blob[15..])
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
        Ok(out)
    }

    pub fn compress_u64(&self, data: &Vec<u64>) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        // delta + varint (no zigzag needed for unsigned)
        let mut buf = Vec::with_capacity(data.len() * 2);
        // header: magic + version + len + type
        buf.extend_from_slice(b"ORSO"); // 0..4
        buf.push(1); // 4: version
        buf.push(1); // 5: codec LZ4
        buf.push(1); // 6: type (1 = u64)
        buf.extend_from_slice(&(data.len() as u64).to_le_bytes()); // 7..15

        // stream varints into a temp vec
        let mut tmp = Vec::with_capacity(data.len() * 2);
        let mut prev = 0u64;
        for &x in data {
            let d = x.wrapping_sub(prev);
            prev = x;
            tmp.write_varint(d).unwrap();
        }

        // compress varint bytes
        let comp = lz4_flex::block::compress_prepend_size(&tmp);
        buf.extend_from_slice(&comp);
        Ok(buf)
    }

    pub fn decompress_u64(&self, blob: &[u8]) -> Result<Vec<u64>> {
        if blob.is_empty() {
            return Ok(Vec::new());
        }
        if blob.len() < 15 {
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
        if blob[6] != 1 {
            bail!("unsupported type, expected u64");
        }
        let n = u64::from_le_bytes(blob[7..15].try_into().unwrap()) as usize;

        let packed = lz4_flex::block::decompress_size_prepended(&blob[15..])
            .map_err(|e| anyhow!("lz4 decompress failed: {e}"))?;

        let mut cur = Cursor::new(packed.as_slice());
        let mut out = Vec::with_capacity(n);
        let mut acc = 0u64;
        for _ in 0..n {
            let v: u64 = cur
                .read_varint()
                .map_err(|e| anyhow!("varint decode: {e}"))?;
            acc = acc.wrapping_add(v);
            out.push(acc);
        }
        Ok(out)
    }

    pub fn compress_i32(&self, data: &Vec<i32>) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        // delta + zigzag → varint (similar to i64 but with i32)
        let mut buf = Vec::with_capacity(data.len() * 2);
        // header: magic + version + len + type
        buf.extend_from_slice(b"ORSO"); // 0..4
        buf.push(1); // 4: version
        buf.push(1); // 5: codec LZ4
        buf.push(2); // 6: type (2 = i32)
        buf.extend_from_slice(&(data.len() as u64).to_le_bytes()); // 7..15

        // stream varints into a temp vec
        let mut tmp = Vec::with_capacity(data.len() * 2);
        let mut prev = 0i32;
        for &x in data {
            let d = x.wrapping_sub(prev);
            prev = x;
            tmp.write_varint(Self::zigzag_i32(d)).unwrap();
        }

        // compress varint bytes
        let comp = lz4_flex::block::compress_prepend_size(&tmp);
        buf.extend_from_slice(&comp);
        Ok(buf)
    }

    pub fn decompress_i32(&self, blob: &[u8]) -> Result<Vec<i32>> {
        if blob.is_empty() {
            return Ok(Vec::new());
        }
        if blob.len() < 15 {
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
        if blob[6] != 2 {
            bail!("unsupported type, expected i32");
        }
        let n = u64::from_le_bytes(blob[7..15].try_into().unwrap()) as usize;

        let packed = lz4_flex::block::decompress_size_prepended(&blob[15..])
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
        Ok(out)
    }

    pub fn compress_u32(&self, data: &Vec<u32>) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        // delta + varint (no zigzag needed for unsigned)
        let mut buf = Vec::with_capacity(data.len() * 2);
        // header: magic + version + len + type
        buf.extend_from_slice(b"ORSO"); // 0..4
        buf.push(1); // 4: version
        buf.push(1); // 5: codec LZ4
        buf.push(3); // 6: type (3 = u32)
        buf.extend_from_slice(&(data.len() as u64).to_le_bytes()); // 7..15

        // stream varints into a temp vec
        let mut tmp = Vec::with_capacity(data.len() * 2);
        let mut prev = 0u32;
        for &x in data {
            let d = x.wrapping_sub(prev);
            prev = x;
            tmp.write_varint(d).unwrap();
        }

        // compress varint bytes
        let comp = lz4_flex::block::compress_prepend_size(&tmp);
        buf.extend_from_slice(&comp);
        Ok(buf)
    }

    pub fn decompress_u32(&self, blob: &[u8]) -> Result<Vec<u32>> {
        if blob.is_empty() {
            return Ok(Vec::new());
        }
        if blob.len() < 15 {
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
        if blob[6] != 3 {
            bail!("unsupported type, expected u32");
        }
        let n = u64::from_le_bytes(blob[7..15].try_into().unwrap()) as usize;

        let packed = lz4_flex::block::decompress_size_prepended(&blob[15..])
            .map_err(|e| anyhow!("lz4 decompress failed: {e}"))?;

        let mut cur = Cursor::new(packed.as_slice());
        let mut out = Vec::with_capacity(n);
        let mut acc = 0u32;
        for _ in 0..n {
            let v: u32 = cur
                .read_varint()
                .map_err(|e| anyhow!("varint decode: {e}"))?;
            acc = acc.wrapping_add(v);
            out.push(acc);
        }
        Ok(out)
    }

    pub fn compress_many_i64(&self, arrays: &[Vec<i64>]) -> Result<Vec<Vec<u8>>> {
        arrays.par_iter().map(|a| self.compress_i64(a)).collect()
    }

    pub fn decompress_many_i64(&self, blobs: &[Vec<u8>]) -> Result<Vec<Vec<i64>>> {
        blobs.par_iter().map(|b| self.decompress_i64(b)).collect()
    }

    pub fn compress_many_u64(&self, arrays: &[Vec<u64>]) -> Result<Vec<Vec<u8>>> {
        arrays.par_iter().map(|a| self.compress_u64(a)).collect()
    }

    pub fn decompress_many_u64(&self, blobs: &[Vec<u8>]) -> Result<Vec<Vec<u64>>> {
        blobs.par_iter().map(|b| self.decompress_u64(b)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{rngs::StdRng, Rng, SeedableRng};

    #[test]
    fn roundtrip_i64() -> Result<()> {
        let c = IntegerCodec::default();
        let v: Vec<i64> = (0..10_000).map(|i| i as i64).collect();
        let blob = c.compress_i64(&v)?;
        let back = c.decompress_i64(&blob)?;
        assert_eq!(v, back);
        Ok(())
    }

    #[test]
    fn roundtrip_u64() -> Result<()> {
        let c = IntegerCodec::default();
        let v: Vec<u64> = (0..10_000).map(|i| i as u64).collect();
        let blob = c.compress_u64(&v)?;
        let back = c.decompress_u64(&blob)?;
        assert_eq!(v, back);
        Ok(())
    }

    #[test]
    fn roundtrip_i32() -> Result<()> {
        let c = IntegerCodec::default();
        let v: Vec<i32> = (0..10_000).map(|i| i as i32).collect();
        let blob = c.compress_i32(&v)?;
        let back = c.decompress_i32(&blob)?;
        assert_eq!(v, back);
        Ok(())
    }

    #[test]
    fn roundtrip_u32() -> Result<()> {
        let c = IntegerCodec::default();
        let v: Vec<u32> = (0..10_000).map(|i| i as u32).collect();
        let blob = c.compress_u32(&v)?;
        let back = c.decompress_u32(&blob)?;
        assert_eq!(v, back);
        Ok(())
    }

    #[test]
    fn roundtrip_parallel_i64() -> Result<()> {
        let c = IntegerCodec::default();
        let arrays: Vec<Vec<i64>> = (0..64)
            .map(|k| (0..8192).map(|i| (i as i64) + k).collect())
            .collect();
        let blobs = c.compress_many_i64(&arrays)?;
        let back = c.decompress_many_i64(&blobs)?;
        assert_eq!(arrays, back);
        Ok(())
    }

    #[test]
    fn roundtrip_parallel_u64() -> Result<()> {
        let c = IntegerCodec::default();
        let arrays: Vec<Vec<u64>> = (0..64)
            .map(|k| (0..8192).map(|i| (i as u64) + k).collect())
            .collect();
        let blobs = c.compress_many_u64(&arrays)?;
        let back = c.decompress_many_u64(&blobs)?;
        assert_eq!(arrays, back);
        Ok(())
    }

    #[test]
    fn randomish_i64_ok() -> Result<()> {
        let mut rng = StdRng::seed_from_u64(42);
        let v: Vec<i64> = (0..50_000).map(|_| rng.r#gen::<i64>() >> 3).collect();
        let c = IntegerCodec::default();
        let blob = c.compress_i64(&v)?;
        let back = c.decompress_i64(&blob)?;
        assert_eq!(v, back);
        Ok(())
    }

    #[test]
    fn randomish_u64_ok() -> Result<()> {
        let mut rng = StdRng::seed_from_u64(42);
        let v: Vec<u64> = (0..50_000).map(|_| (rng.r#gen::<i64>() >> 3) as u64).collect();
        let c = IntegerCodec::default();
        let blob = c.compress_u64(&v)?;
        let back = c.decompress_u64(&blob)?;
        assert_eq!(v, back);
        Ok(())
    }

    #[test]
    fn report_metrics_ema_like_sizes() -> Result<()> {
        use std::time::Instant;

        // helper: deterministic EMA-like series (smooth with small variations),
        // scaled to i64 by 1e6 (so we mimic f64 EMA values).
        fn ema_like_i64(len: usize) -> Vec<i64> {
            let mut out = Vec::with_capacity(len);
            // start around 117_000.xxx (scaled by 1e6)
            let mut ema: f64 = 117_100.0;
            let alpha = 2.0 / (9.0 + 1.0); // like EMA(9)
                                           // deterministic "price" signal: slow trend + small oscillations
            for i in 0..len {
                let t = i as f64;
                let price = 117_000.0
                + 0.05 * t                              // tiny trend
                + (t / 37.0).sin() * 30.0              // slow sine wiggle
                + ((t / 5.0).sin() * 3.0).floor(); // tiny step noise
                ema = alpha * price + (1.0 - alpha) * ema;
                let scaled = (ema * 1_000_000.0).round() as i64;
                out.push(scaled);
            }
            out
        }

        let codec = IntegerCodec::default(); // LZ4 path from your implementation

        for &n in &[100usize, 1_000usize, 100_000usize] {
            let data = ema_like_i64(n);

            // compress
            let t0 = Instant::now();
            let blob = codec.compress_i64(&data)?;
            let comp_ms = t0.elapsed().as_secs_f64() * 1000.0;

            // decompress
            let t1 = Instant::now();
            let back = codec.decompress_i64(&blob)?;
            let decomp_ms = t1.elapsed().as_secs_f64() * 1000.0;

            assert_eq!(data, back, "round-trip failed for n={}", n);

            let raw_bytes = data.len() * 8;
            let comp_bytes = blob.len();
            let ratio = (raw_bytes as f64) / (comp_bytes.max(1) as f64);

            eprintln!(
                "i64 n={:<7} raw={:<10}B  comp={:<10}B  ratio={:>5.2}x  compress={:>6.3} ms  decompress={:>6.3} ms",
                n, raw_bytes, comp_bytes, ratio, comp_ms, decomp_ms
            );
        }

        Ok(())
    }

    #[test]
    fn report_metrics_ema_like_sizes_u64() -> Result<()> {
        use std::time::Instant;

        // helper: deterministic EMA-like series for u64
        fn ema_like_u64(len: usize) -> Vec<u64> {
            let mut out = Vec::with_capacity(len);
            // start around 117_000
            let mut ema: f64 = 117_100.0;
            let alpha = 2.0 / (9.0 + 1.0); // like EMA(9)
                                           // deterministic "price" signal: slow trend + small oscillations
            for i in 0..len {
                let t = i as f64;
                let price = 117_000.0
                + 0.05 * t                              // tiny trend
                + (t / 37.0).sin() * 30.0              // slow sine wiggle
                + ((t / 5.0).sin() * 3.0).floor(); // tiny step noise
                ema = alpha * price + (1.0 - alpha) * ema;
                let scaled = (ema * 1_000_000.0).round() as u64;
                out.push(scaled);
            }
            out
        }

        let codec = IntegerCodec::default();

        for &n in &[100usize, 1_000usize, 100_000usize] {
            let data = ema_like_u64(n);

            // compress
            let t0 = Instant::now();
            let blob = codec.compress_u64(&data)?;
            let comp_ms = t0.elapsed().as_secs_f64() * 1000.0;

            // decompress
            let t1 = Instant::now();
            let back = codec.decompress_u64(&blob)?;
            let decomp_ms = t1.elapsed().as_secs_f64() * 1000.0;

            assert_eq!(data, back, "round-trip failed for n={}", n);

            let raw_bytes = data.len() * 8;
            let comp_bytes = blob.len();
            let ratio = (raw_bytes as f64) / (comp_bytes.max(1) as f64);

            eprintln!(
                "u64 n={:<7} raw={:<10}B  comp={:<10}B  ratio={:>5.2}x  compress={:>6.3} ms  decompress={:>6.3} ms",
                n, raw_bytes, comp_bytes, ratio, comp_ms, decomp_ms
            );
        }

        Ok(())
    }

    #[test]
    fn report_metrics_ema_like_sizes_i32() -> Result<()> {
        use std::time::Instant;

        // helper: deterministic EMA-like series for i32
        fn ema_like_i32(len: usize) -> Vec<i32> {
            let mut out = Vec::with_capacity(len);
            // start around 117_000
            let mut ema: f64 = 117_100.0;
            let alpha = 2.0 / (9.0 + 1.0); // like EMA(9)
                                           // deterministic "price" signal: slow trend + small oscillations
            for i in 0..len {
                let t = i as f64;
                let price = 117_000.0
                + 0.05 * t                              // tiny trend
                + (t / 37.0).sin() * 30.0              // slow sine wiggle
                + ((t / 5.0).sin() * 3.0).floor(); // tiny step noise
                ema = alpha * price + (1.0 - alpha) * ema;
                let scaled = (ema * 1_000.0).round() as i32;
                out.push(scaled);
            }
            out
        }

        let codec = IntegerCodec::default();

        for &n in &[100usize, 1_000usize, 100_000usize] {
            let data = ema_like_i32(n);

            // compress
            let t0 = Instant::now();
            let blob = codec.compress_i32(&data)?;
            let comp_ms = t0.elapsed().as_secs_f64() * 1000.0;

            // decompress
            let t1 = Instant::now();
            let back = codec.decompress_i32(&blob)?;
            let decomp_ms = t1.elapsed().as_secs_f64() * 1000.0;

            assert_eq!(data, back, "round-trip failed for n={}", n);

            let raw_bytes = data.len() * 4;
            let comp_bytes = blob.len();
            let ratio = (raw_bytes as f64) / (comp_bytes.max(1) as f64);

            eprintln!(
                "i32 n={:<7} raw={:<10}B  comp={:<10}B  ratio={:>5.2}x  compress={:>6.3} ms  decompress={:>6.3} ms",
                n, raw_bytes, comp_bytes, ratio, comp_ms, decomp_ms
            );
        }

        Ok(())
    }

    #[test]
    fn report_metrics_ema_like_sizes_u32() -> Result<()> {
        use std::time::Instant;

        // helper: deterministic EMA-like series for u32
        fn ema_like_u32(len: usize) -> Vec<u32> {
            let mut out = Vec::with_capacity(len);
            // start around 117_000
            let mut ema: f64 = 117_100.0;
            let alpha = 2.0 / (9.0 + 1.0); // like EMA(9)
                                           // deterministic "price" signal: slow trend + small oscillations
            for i in 0..len {
                let t = i as f64;
                let price = 117_000.0
                + 0.05 * t                              // tiny trend
                + (t / 37.0).sin() * 30.0              // slow sine wiggle
                + ((t / 5.0).sin() * 3.0).floor(); // tiny step noise
                ema = alpha * price + (1.0 - alpha) * ema;
                let scaled = (ema * 1_000.0).round() as u32;
                out.push(scaled);
            }
            out
        }

        let codec = IntegerCodec::default();

        for &n in &[100usize, 1_000usize, 100_000usize] {
            let data = ema_like_u32(n);

            // compress
            let t0 = Instant::now();
            let blob = codec.compress_u32(&data)?;
            let comp_ms = t0.elapsed().as_secs_f64() * 1000.0;

            // decompress
            let t1 = Instant::now();
            let back = codec.decompress_u32(&blob)?;
            let decomp_ms = t1.elapsed().as_secs_f64() * 1000.0;

            assert_eq!(data, back, "round-trip failed for n={}", n);

            let raw_bytes = data.len() * 4;
            let comp_bytes = blob.len();
            let ratio = (raw_bytes as f64) / (comp_bytes.max(1) as f64);

            eprintln!(
                "u32 n={:<7} raw={:<10}B  comp={:<10}B  ratio={:>5.2}x  compress={:>6.3} ms  decompress={:>6.3} ms",
                n, raw_bytes, comp_bytes, ratio, comp_ms, decomp_ms
            );
        }

        Ok(())
    }
}