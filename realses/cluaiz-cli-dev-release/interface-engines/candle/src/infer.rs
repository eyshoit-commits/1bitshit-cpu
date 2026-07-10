//! Archer-Candle Inference: Streaming generation logic.

use crate::SovereignModel;
use candle_core::{IndexOp, Result as CandleResult, Tensor};
use tokenizers::Tokenizer;

pub struct CandleInference;

impl CandleInference {
    /// Resolves the EOS token ID dynamically from the tokenizer vocabulary.
    fn resolve_eos_token(tokenizer: &Tokenizer) -> u32 {
        let vocab = tokenizer.get_vocab(true);
        for (token_str, id) in &vocab {
            let lower = token_str.to_lowercase();
            if lower.contains("end_of_turn")
                || lower.contains("eos")
                || lower.contains("eot_id")
                || lower == "\x3c|im_end|\x3e"
                || lower == "\x3c/s\x3e"
            {
                tracing::info!("Found dynamic EOS token: {} (ID: {})", token_str, id);
                return *id;
            }
        }
        tracing::warn!("Could not dynamically resolve EOS token. Falling back to ID 2.");
        2
    }

    pub fn generate_stream(
        model: &mut SovereignModel,
        prompt: &str,
        max_tokens: usize,
        tokenizer: &Tokenizer,
        device: &candle_core::Device,
        mut callback: Box<dyn FnMut(String) + Send + 'static>,
    ) -> CandleResult<()> {
        let encoded = tokenizer
            .encode(prompt, true)
            .map_err(|e| candle_core::Error::Msg(format!("Tokenizer error: {e}")))?;
        let tokens = encoded.get_ids();
        let input = Tensor::new(tokens, device)?.unsqueeze(0)?;
        let mut logits = model.forward(&input, 0)?;

        let dynamic_eos = Self::resolve_eos_token(tokenizer);

        for i in 0..max_tokens {
            let next_token = logits
                .i((0, logits.dim(1)? - 1))?
                .argmax(0)?
                .to_scalar::<u32>()?;
            if next_token == dynamic_eos {
                break;
            }
            if let Ok(t) = tokenizer.decode(&[next_token], true) {
                callback(t);
            }
            let next_input = Tensor::new(&[next_token], device)?.unsqueeze(0)?;
            logits = model.forward(&next_input, tokens.len() + i)?;
        }
        Ok(())
    }
}
