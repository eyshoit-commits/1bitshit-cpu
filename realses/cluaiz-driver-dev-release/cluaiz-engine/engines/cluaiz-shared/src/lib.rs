//! archer-shared: Common traits and types for the CURE Engine ecosystem.

/// 🏛️ CluaizDNA: Modular Lifecycle Management for the Neural Ecosystem
pub struct CluaizDNA;
impl CluaizDNA {
    pub const CLI: &'static str = "dev-release";
    pub const ENGINE: &'static str = "dev-release";
    pub const KERNEL: &'static str = "dev-release";
    pub const DRIVER: &'static str = "dev-release";
}

pub mod hardware;
pub mod metadata;
pub mod neural;
pub mod prompting;
pub mod backend;
pub mod neural_core;
pub mod utils;

// ── Business Logic (Unified from shared) ──
pub mod profile;
pub mod auth;
pub mod onboarding;
pub mod Chat;

pub use hardware::{governor::*, telemetry::*};
pub use metadata::dna::*;
pub use prompting::templater::*;
pub use backend::{context::*, traits::*, signature::*};
pub use neural_core::NeuralResult;

/// 🏛️ CluaizLinkerPlaceholder: Used during Phase 1 to verify the Dynamic Linker Handshake.
pub struct CluaizLinkerPlaceholder;

// Ensure placeholder is Send + Sync for the Orchestrator's type requirements
unsafe impl Send for CluaizLinkerPlaceholder {}
unsafe impl Sync for CluaizLinkerPlaceholder {}

impl crate::backend::traits::UnifiedBackend for CluaizLinkerPlaceholder {
    fn generate(&mut self, _prompt: &str, _max_tokens: usize) -> std::result::Result<String, String> {
        Err("✅ SOVEREIGN LINKER: Phase 1 Handshake Success. Real inference pending Phase 2.".to_string())
    }
    fn prefill(&mut self, _prompt: &str) -> anyhow::Result<()> { Ok(()) }
    fn evaluate_tps(&self) -> f64 { 0.0 }
}

impl crate::backend::traits::CluaizInference for CluaizLinkerPlaceholder {
    fn forward_raw(&mut self, _input_ids: &[u32], _pos: usize) -> anyhow::Result<Vec<f32>> {
        Err(anyhow::anyhow!("Handshake Placeholder"))
    }
    fn generate_stream(
        &mut self,
        _prompt: &str,
        _max_tokens: usize,
        _tokenizer: &tokenizers::Tokenizer,
        _callback: Box<dyn FnMut(String) + Send + 'static>,
    ) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("✅ SOVEREIGN LINKER: Handshake Complete. Ready for Phase 2."))
    }
}
