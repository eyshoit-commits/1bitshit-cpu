//! ═══════════════════════════════════════════════════════════════════════
//!  CURE Engine: Core Sampler (Cluaiz)
//! ═══════════════════════════════════════════════════════════════════════

use candle_core::{Result, Tensor};
use rand::{distr::Distribution, SeedableRng};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InferenceMode {
    Turbo,
    Classic,
    Auto,
}

#[derive(Debug, Clone)]
pub struct CoreSampler {
    pub temperature: f32,
    pub top_p: f32,
    pub repeat_penalty: f32,
    pub seed: u64,
}

impl CoreSampler {
    pub fn new(seed: u64, temp: f32, top_p: f32, penalty: f32) -> Self {
        Self {
            seed,
            temperature: temp,
            top_p,
            repeat_penalty: penalty,
        }
    }

    /// Dispatch sampling based on InferenceMode.
    pub fn sample(&self, logits: &Tensor, mode: &InferenceMode) -> Result<u32> {
        match mode {
            InferenceMode::Turbo => self.sample_greedy(logits),
            InferenceMode::Classic => self.sample_creative(logits),
            InferenceMode::Auto => {
                // Future: Entropy-based automatic switching.
                // For now, Turbo for maximum potential speed.
                self.sample_greedy(logits)
            }
        }
    }

    /// Pure GPU Greedy Sampling (Zero distribution stall).
    pub fn sample_greedy(&self, logits: &Tensor) -> Result<u32> {
        let argmax = logits.argmax(0)?;
        let scalar = argmax.to_scalar::<u32>()?;
        Ok(scalar)
    }

    /// Creative Sampling (Top-P).
    pub fn sample_creative(&self, logits: &Tensor) -> Result<u32> {
        let logits = (logits / (self.temperature as f64))?;
        let prs = candle_nn::ops::softmax_last_dim(&logits)?;

        let prs: Vec<f32> = prs.to_vec1()?;
        let mut prs_with_index: Vec<(usize, f32)> = prs.into_iter().enumerate().collect();

        // Sort for Top-P
        prs_with_index.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut cumulative_p = 0.0;
        let mut top_p_count = 0;
        for (_, p) in prs_with_index.iter() {
            cumulative_p += p;
            top_p_count += 1;
            if cumulative_p >= self.top_p {
                break;
            }
        }

        let prs_with_index = &prs_with_index[..top_p_count];
        let total_p: f32 = prs_with_index.iter().map(|(_, p)| p).sum();

        let mut rng = rand::rngs::StdRng::seed_from_u64(self.seed);
        let distr = rand::distr::weighted::WeightedIndex::new(
            prs_with_index.iter().map(|(_, p)| p / total_p),
        )
        .map_err(|e| candle_core::Error::Msg(format!("Sampling Error: {}", e)))?;

        Ok(prs_with_index[distr.sample(&mut rng)].0 as u32)
    }
}
