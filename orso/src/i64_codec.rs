use anyhow::{anyhow, bail, Result};
use integer_encoding::{VarIntReader, VarIntWriter};
use rayon::prelude::*;
use std::io::Cursor;

#[derive(Clone, Copy, Debug)]
pub enum Codec {
    Lz4,
} // add Zstd later if you want

#[derive(Clone, Debug)]
pub struct I64Codec {
    pub codec: Codec,
}
impl Default for I64Codec {
    fn default() -> Self {
        Self { codec: Codec::Lz4 }
    }
}
impl I64Codec {
    #[inline]
    fn zigzag(i: i64) -> u64 {
        ((i << 1) ^ (i >> 63)) as u64
    }
    #[inline]
    fn unzigzag(u: u64) -> i64 {
        ((u >> 1) as i64) ^ (-((u & 1) as i64))
    }

    pub fn compress(&self, data: &Vec<i64>) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        // delta + zigzag → varint
        let mut buf = Vec::with_capacity(data.len() * 2);
        // header: magic + version + len
        buf.extend_from_slice(b"ORSO"); // 0..4
        buf.push(1); // 4: version
        buf.push(1); // 5: codec LZ4
        buf.extend_from_slice(&(data.len() as u64).to_le_bytes()); // 6..14

        // stream varints into a temp vec
        let mut tmp = Vec::with_capacity(data.len() * 2);
        let mut prev = 0i64;
        for &x in data {
            let d = x.wrapping_sub(prev);
            prev = x;
            tmp.write_varint(Self::zigzag(d)).unwrap();
        }

        // compress varint bytes
        let comp = lz4_flex::block::compress_prepend_size(&tmp);
        buf.extend_from_slice(&comp);
        Ok(buf)
    }

    pub fn decompress(&self, blob: &[u8]) -> Result<Vec<i64>> {
        if blob.is_empty() {
            return Ok(Vec::new());
        }
        if blob.len() < 14 {
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
        let n = u64::from_le_bytes(blob[6..14].try_into().unwrap()) as usize;

        let packed = lz4_flex::block::decompress_size_prepended(&blob[14..])
            .map_err(|e| anyhow!("lz4 decompress failed: {e}"))?;

        let mut cur = Cursor::new(packed.as_slice());
        let mut out = Vec::with_capacity(n);
        let mut acc = 0i64;
        for _ in 0..n {
            let v: u64 = cur
                .read_varint()
                .map_err(|e| anyhow!("varint decode: {e}"))?;
            let d = Self::unzigzag(v);
            acc = acc.wrapping_add(d);
            out.push(acc);
        }
        Ok(out)
    }

    pub fn compress_many(&self, arrays: &[Vec<i64>]) -> Result<Vec<Vec<u8>>> {
        arrays.par_iter().map(|a| self.compress(a)).collect()
    }
    pub fn decompress_many(&self, blobs: &[Vec<u8>]) -> Result<Vec<Vec<i64>>> {
        blobs.par_iter().map(|b| self.decompress(b)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{rngs::StdRng, Rng, SeedableRng};

    #[test]
    fn roundtrip_basic() -> Result<()> {
        let c = I64Codec::default();
        let v: Vec<i64> = (0..10_000).map(|i| i as i64).collect();
        let blob = c.compress(&v)?;
        let back = c.decompress(&blob)?;
        assert_eq!(v, back);
        Ok(())
    }

    #[test]
    fn roundtrip_parallel() -> Result<()> {
        let c = I64Codec::default();
        let arrays: Vec<Vec<i64>> = (0..64)
            .map(|k| (0..8192).map(|i| (i as i64) + k).collect())
            .collect();
        let blobs = c.compress_many(&arrays)?;
        let back = c.decompress_many(&blobs)?;
        assert_eq!(arrays, back);
        Ok(())
    }

    #[test]
    fn randomish_ok() -> Result<()> {
        let mut rng = StdRng::seed_from_u64(42);
        let v: Vec<i64> = (0..50_000).map(|_| rng.r#gen::<i64>() >> 3).collect();
        let c = I64Codec::default();
        let blob = c.compress(&v)?;
        let back = c.decompress(&blob)?;
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

        let codec = I64Codec::default(); // LZ4 path from your implementation

        for &n in &[100usize, 1_000usize, 100_000usize] {
            let data = ema_like_i64(n);

            // compress
            let t0 = Instant::now();
            let blob = codec.compress(&data)?;
            let comp_ms = t0.elapsed().as_secs_f64() * 1000.0;

            // decompress
            let t1 = Instant::now();
            let back = codec.decompress(&blob)?;
            let decomp_ms = t1.elapsed().as_secs_f64() * 1000.0;

            assert_eq!(data, back, "round-trip failed for n={}", n);

            let raw_bytes = data.len() * 8;
            let comp_bytes = blob.len();
            let ratio = (raw_bytes as f64) / (comp_bytes.max(1) as f64);

            eprintln!(
                "n={:<7} raw={:<10}B  comp={:<10}B  ratio={:>5.2}x  compress={:>6.3} ms  decompress={:>6.3} ms",
                n, raw_bytes, comp_bytes, ratio, comp_ms, decomp_ms
            );
        }

        Ok(())
    }
}
