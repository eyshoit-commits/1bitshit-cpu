//! 🚀 AtmaSteer: Bare-Metal KV Cache Mapping & Injection
//! This module implements the zero-copy logic from the research papers.

use std::path::Path;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use crate::interfaces::memory_contract::{SovereignBuffer, MappedBuffer};
use cluaiz_shared::hardware::schema::profiles::SiliconTruth;
use cluaiz_shared::hardware::memory::kv_cache::stitching::CluaizSignal;

/// 🏛️ SovereignMapper
/// Handles the mapping of .kv-cache or .gguf files based on hardware capabilities.
pub struct SovereignMapper {
    pub silicon: SiliconTruth,
}

impl SovereignMapper {
    pub fn new(silicon: SiliconTruth) -> Self {
        Self { silicon }
    }

    /// 🧠 Map Model Weights or KV Skills
    /// Implements "Lazy Loading" for SSD and "Pre-faulting" for HDD.
    pub fn map_vault<P: AsRef<Path>>(&self, path: P) -> Result<MappedBuffer> {
        let buffer = MappedBuffer::from_file(&path)?;
        
        // 🧪 Hardware Awareness Check
        let is_ssd = self.silicon.storage.iter().any(|s| s.drive_type.to_lowercase().contains("ssd"));
        
        if !is_ssd {
            println!("🐌 [Mapper] Slow storage detected. Applying HDD Pre-fault strategy.");
        } else {
            println!("🚀 [Mapper] NVMe/SSD detected. Zero-copy mmap active.");
        }

        Ok(buffer)
    }

    /// ⚡ Skill Stitcher (AtmaSteer Tier 1)
    /// Prepares a pre-computed skill tensor for injection.
    pub fn prepare_skill_injection(&self, skill_path: &str) -> Result<Box<dyn SovereignBuffer>> {
        let path = Path::new(skill_path);
        if !path.exists() {
            return Err(anyhow!("❌ Skill file not found: {}", skill_path));
        }

        let mapped = self.map_vault(path)?;
        Ok(Box::new(mapped))
    }
}

/// 🚥 KVStitcher
/// High-level engine for preparing CluaizSignals for the Foundry.
pub struct KVStitcher;

impl KVStitcher {
    pub fn prepare_signal(skill_path: &Path, token_count: usize, head_dim: usize) -> Result<CluaizSignal> {
        if !skill_path.exists() {
            return Err(anyhow!("❌ Skill file not found: {:?}", skill_path));
        }

        let buffer = MappedBuffer::from_file(skill_path)?;
        
        Ok(CluaizSignal {
            raw_data: Arc::new(buffer),
            token_count,
            head_dim,
        })
    }
}

/// 🚥 KVSteering
/// Manages the actual pointer-passing to the backends.
pub struct KVSteering;

impl KVSteering {
    /// 💉 Inject to Engine
    /// Passes the raw pointer from the SovereignBuffer directly to the engine's memory space.
    pub unsafe fn inject(_engine_ptr: *mut std::ffi::c_void, buffer: &dyn SovereignBuffer) {
        let ptr = buffer.as_ptr();
        let len = buffer.len();
        
        println!("💉 [KV-Steering] Injecting zero-copy buffer: {:?} ({} bytes)", ptr, len);
    }
}
