use anyhow::{bail, Result};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

/// 🔬 GGUF Prober: Extracts architectural truth directly from binary headers.
/// Zero-dependency implementation to keep the Sovereign Core lightweight.
pub struct GGUFProber;

impl GGUFProber {
    pub fn probe(
        path: &std::path::Path,
    ) -> Result<(HashMap<String, String>, HashMap<String, Vec<usize>>)> {
        let mut file = File::open(path)?;

        // 1. Magic Check (GGUF)
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic)?;
        if &magic != b"GGUF" {
            bail!("Not a valid GGUF binary");
        }

        // 2. Version Check
        let mut version = [0u8; 4];
        file.read_exact(&mut version)?;
        let _version = u32::from_le_bytes(version);

        // 3. Header Counts
        let mut counts = [0u8; 16]; // tensor_count (u64) + metadata_kv_count (u64)
        file.read_exact(&mut counts)?;
        let tensor_count = u64::from_le_bytes(counts[0..8].try_into().unwrap());
        let metadata_kv_count = u64::from_le_bytes(counts[8..16].try_into().unwrap());

        let mut metadata = HashMap::new();
        let mut tensor_infos = HashMap::new();

        // 4. Extract Metadata KVs
        for _ in 0..metadata_kv_count {
            let key = Self::read_string(&mut file)?;
            let value_type = Self::read_u32(&mut file)?;
            let value = Self::read_value(&mut file, value_type)?;
            metadata.insert(key, value);
        }

        // 5. Extract Tensor Infos (Partial for Dimension Truth)
        for _ in 0..tensor_count {
            let name = Self::read_string(&mut file)?;
            let n_dims = Self::read_u32(&mut file)?;
            let mut dims = Vec::new();
            for _ in 0..n_dims {
                dims.push(Self::read_u64(&mut file)? as usize);
            }
            let _dtype = Self::read_u32(&mut file)?;
            let _offset = Self::read_u64(&mut file)?;
            tensor_infos.insert(name, dims);
        }

        Ok((metadata, tensor_infos))
    }

    fn read_string(file: &mut File) -> Result<String> {
        let len = Self::read_u64(file)?;
        let mut buf = vec![0u8; len as usize];
        file.read_exact(&mut buf)?;
        Ok(String::from_utf8_lossy(&buf).to_string())
    }

    fn read_u32(file: &mut File) -> Result<u32> {
        let mut buf = [0u8; 4];
        file.read_exact(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    fn read_u64(file: &mut File) -> Result<u64> {
        let mut buf = [0u8; 8];
        file.read_exact(&mut buf)?;
        Ok(u64::from_le_bytes(buf))
    }

    fn read_value(file: &mut File, value_type: u32) -> Result<String> {
        match value_type {
            0..=11 => {
                // Numeric types
                let mut buf = [0u8; 8]; // Max size for u64/f64
                let size = match value_type {
                    0 | 1 | 6 => 1,
                    2 | 3 | 7 => 2,
                    4 | 5 | 8 | 10 => 4,
                    9 | 11 => 8,
                    _ => 0,
                };
                file.read_exact(&mut buf[..size])?;
                Ok(format!("{:?}", &buf[..size])) // Basic representation
            }
            12 => Ok(if Self::read_u8(file)? == 0 {
                "false".into()
            } else {
                "true".into()
            }),
            13 => Self::read_string(file),
            14 => {
                // Array
                let _item_type = Self::read_u32(file)?;
                let len = Self::read_u64(file)?;
                // Skipping array content for now, just returning length
                file.seek(SeekFrom::Current(8 * len as i64))?; // Rough skip
                Ok(format!("[Array: len={}]", len))
            }
            _ => {
                file.seek(SeekFrom::Current(4))?; // Skip unknown
                Ok("UNKNOWN_TYPE".into())
            }
        }
    }

    fn read_u8(file: &mut File) -> Result<u8> {
        let mut buf = [0u8; 1];
        file.read_exact(&mut buf)?;
        Ok(buf[0])
    }
}
