//! Archer-Candle Loader: Weight initialization and DNA-driven GGUF parsing.

use crate::SovereignModel;
use anyhow::Result;
use candle_core::quantized::gguf_file::{Content, Value};
use candle_core::Device;
use cluaiz_shared::metadata::dna::StructuralDNA;
use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;

pub struct CandleLoader;

impl CandleLoader {
    pub fn load(
        _path: &PathBuf,
        content: Content,
        file: &mut File,
        device: &Device,
        dna: Option<StructuralDNA>,
    ) -> Result<SovereignModel> {
        let mut dna_ref = dna.ok_or_else(|| anyhow::anyhow!("DNA required for Sovereign V1.0"))?;

        // 🛡️ [TRANSLATOR]: Convert Candle GGUF types to framework-agnostic DNA types
        // [FIXED]: Iterating over references to avoid moving 'content'
        let mut metadata_simple = HashMap::new();
        for (k, v) in &content.metadata {
            let val_str = match v {
                Value::String(s) => s.clone(),
                Value::U8(u) => u.to_string(),
                Value::U16(u) => u.to_string(),
                Value::U32(u) => u.to_string(),
                Value::F32(f) => f.to_string(),
                Value::I8(i) => i.to_string(),
                Value::I16(i) => i.to_string(),
                Value::I32(i) => i.to_string(),
                _ => "unsupported".to_string(),
            };
            metadata_simple.insert(k.clone(), val_str);
        }

        let mut tensor_simple = HashMap::new();
        for (k, v) in &content.tensor_infos {
            tensor_simple.insert(k.clone(), v.shape.dims().to_vec());
        }

        // 🏁 [Truth Protocol] Sync with binary metadata
        dna_ref.sync_with_metadata(&metadata_simple, &tensor_simple);

        if dna_ref.signature.is_bitnet {
            println!("🚀 [Kernel] Routine: 1-bit Neural Logic — Dispatching Variant 1.");
            let weights = candle_transformers::models::quantized_llama::ModelWeights::from_gguf(
                content, file, device,
            )?;
            Ok(SovereignModel::Variant1(weights))
        } else if dna_ref.signature.is_heterogeneous {
            println!("🚀 [Kernel] Routine: Heterogeneous Block Logic — Dispatching Variant 2.");
            let weights = candle_transformers::models::quantized_gemma3::ModelWeights::from_gguf(
                content, file, device,
            )?;
            Ok(SovereignModel::Variant2(weights))
        } else {
            println!("🚀 [Kernel] Routine: Uniform GQA Logic — Dispatching Variant 1.");
            let weights = candle_transformers::models::quantized_llama::ModelWeights::from_gguf(
                content, file, device,
            )?;
            Ok(SovereignModel::Variant1(weights))
        }
    }
}
