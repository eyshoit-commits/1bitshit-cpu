//! Archer-Candle Configuration: Dynamic parameter resolution.

use cluaiz_shared::metadata::dna::StructuralDNA;
use cluaiz_shared::neural_core::config::{NeuralConfig, ResolvedNeuralParams};

pub struct CandleConfig;

impl CandleConfig {
    /// Resolves parameters specifically for the Candle backend.
    pub fn resolve(dna: &StructuralDNA) -> ResolvedNeuralParams {
        let mut params = NeuralConfig::resolve(dna);

        // Custom Candle overrides if any (e.g., sliding window specifics)
        // Sliding Window
        if let Some(sw) = dna
            .dynamic_attributes
            .get("sliding_window")
            .and_then(|v| v.parse::<u64>().ok())
        {
            params.sliding_window = Some(sw as u32);
        }

        params
    }
}
