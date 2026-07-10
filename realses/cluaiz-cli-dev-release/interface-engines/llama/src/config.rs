//! 🚀 Sovereign Booster: Dynamic Configuration System
//! This module translates Registry-level capabilities into low-level engine parameters.

use crate::ffi::llama_cpp::{
    llama_context_default_params, llama_model_default_params, LlamaContextParams, LlamaModelParams,
};
use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoosterConfig {
    #[serde(skip_serializing)]
    pub n_gpu_layers: i32,
    pub flash_attn: bool,
    #[serde(skip_serializing)]
    pub use_mmap: bool,
    #[serde(skip_serializing)]
    pub n_ctx: u32,
    #[serde(skip_serializing)]
    pub n_threads: i32,
    pub turbo_quant: String,
    pub dflash: String, // 🏛️ Delta Flash (FlashKDA Support)
    pub speculative_decoding: String,
    pub auto_round: String,
}

impl Default for BoosterConfig {
    fn default() -> Self {
        Self {
            n_gpu_layers: -1,
            flash_attn: true,
            use_mmap: true,
            n_ctx: 0,
            n_threads: -1,
            turbo_quant: "Auto".to_string(),
            dflash: "Auto".to_string(),
            speculative_decoding: "Auto".to_string(),
            auto_round: "Auto".to_string(),
        }
    }
}

impl BoosterConfig {
    /// 🚀 Load the booster configuration from the sovereign system control.
    pub fn load_from_system() -> Self {
        // Default to Industrial Auto standards
        let mut config = Self {
            flash_attn: true,
            use_mmap: true,
            n_gpu_layers: -1, // Full Offload
            n_ctx: 0,         // Auto-detect from model
            n_threads: -1,    // Auto-detect CPU cores
            turbo_quant: "Auto".to_string(),
            dflash: "Auto".to_string(),
            speculative_decoding: "Auto".to_string(),
            auto_round: "Auto".to_string(),
        };

        if let Ok(content) =
            std::fs::read_to_string("C:\\Users\\Aryan\\.cluaiz\\engine\\system_booster.json")
        {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                // Only override if explicitly provided in JSON
                if let Some(fa) = json.get("flash_attn") {
                    config.flash_attn = fa.as_bool().unwrap_or(true);
                }
                if let Some(gl) = json.get("n_gpu_layers") {
                    config.n_gpu_layers = gl.as_i64().unwrap_or(-1) as i32;
                }
                if let Some(df) = json.get("dflash") {
                    config.dflash = df.as_str().unwrap_or("Auto").to_string();
                }
                if let Some(sd) = json.get("speculative_decoding") {
                    config.speculative_decoding = sd.as_str().unwrap_or("Auto").to_string();
                }
                if let Some(ar) = json.get("auto_round") {
                    config.auto_round = ar.as_str().unwrap_or("Auto").to_string();
                }
            }
        }
        config
    }
    /// 🛠️ Transform high-level config into raw model parameters.
    pub fn to_model_params(&self) -> LlamaModelParams {
        let mut params = unsafe { llama_model_default_params() };
        params.n_gpu_layers = self.n_gpu_layers;
        params.use_mmap = self.use_mmap;
        params
    }

    /// 🛠️ Transform high-level config into raw context parameters.
    pub fn to_context_params(&self) -> LlamaContextParams {
        let mut params = unsafe { llama_context_default_params() };
        params.n_ctx = self.n_ctx;
        params.n_threads = self.n_threads;
        params.flash_attn_type = if self.flash_attn { 1 } else { 0 }; // 1 = Enabled
        params
    }
}
