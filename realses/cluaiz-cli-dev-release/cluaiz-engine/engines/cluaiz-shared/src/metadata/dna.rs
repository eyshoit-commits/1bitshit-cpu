use crate::backend::signature::{BackendType, KernelSignature};
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Structural DNA Synchronization (The Root Genome) ──────────────────────
#[derive(Debug, Clone, Deserialize, Serialize, Archive, RkyvSerialize, RkyvDeserialize)]
#[archive(check_bytes)]
pub struct StructuralDNA {
    pub model_identity: String,
    pub layer_count: Option<usize>,
    pub attention_head_count: Option<usize>,
    pub attention_head_count_kv: Option<usize>,
    pub attention_head_dim: Option<usize>,
    pub hidden_size: Option<usize>,
    pub intermediate_size: Option<usize>,
    pub attention_dimensionality_truth: Option<usize>,
    pub signature: KernelSignature,
    pub preferred_runtime: Option<BackendType>,
    pub heterogeneous_map: Option<HashMap<String, usize>>,
    pub max_context_length: Option<usize>,
    pub dynamic_attributes: HashMap<String, String>,
}

impl Default for StructuralDNA {
    fn default() -> Self {
        Self {
            model_identity: "unknown".into(),
            layer_count: None,
            attention_head_count: None,
            attention_head_count_kv: None,
            attention_head_dim: None,
            hidden_size: None,
            intermediate_size: None,
            attention_dimensionality_truth: None,
            signature: KernelSignature::default(),
            preferred_runtime: None,
            heterogeneous_map: None,
            max_context_length: None, // Must be truth-grounded
            dynamic_attributes: HashMap::new(),
        }
    }
}

impl StructuralDNA {
    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read DNA: {e}"))?;
        serde_json::from_str(&content).map_err(|e| format!("DNA Syntax Error: {e}"))
    }

    pub fn load_archived(path: &std::path::Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("Failed to read Binary DNA: {e}"))?;
        let archived = unsafe { rkyv::archived_root::<StructuralDNA>(&bytes) };
        let deserialized: StructuralDNA = archived.deserialize(&mut rkyv::Infallible).unwrap();
        Ok(deserialized)
    }

    /// Truth Protocol: Synchronizes DNA fields with actual binary metadata.
    pub fn sync_with_metadata(
        &mut self,
        metadata: &HashMap<String, String>,
        _tensor_infos: &HashMap<String, Vec<usize>>,
    ) {
        // [SOVEREIGN CLEAN]: Switched to println for better editor compatibility
        println!("🧬 [DNA] Initiating Multi-Layer Truth Protocol...");

        for (key, value) in metadata {
            if key.ends_with(".embedding_length") || key.ends_with(".hidden_size") {
                if let Ok(v) = value.parse::<usize>() {
                    self.hidden_size = Some(v);
                }
            } else if key.ends_with(".block_count") || key.ends_with(".layer_count") {
                if let Ok(v) = value.parse::<usize>() {
                    self.layer_count = Some(v);
                }
            } else if key.ends_with(".attention.head_count")
                || key.ends_with(".num_attention_heads")
            {
                if let Ok(v) = value.parse::<usize>() {
                    self.attention_head_count = Some(v);
                }
            } else if key.ends_with(".attention.head_count_kv")
                || key.ends_with(".num_key_value_heads")
            {
                if let Ok(v) = value.parse::<usize>() {
                    self.attention_head_count_kv = Some(v);
                }
            } else if key.ends_with(".feed_forward_length") || key.ends_with(".intermediate_size") {
                if let Ok(v) = value.parse::<usize>() {
                    self.intermediate_size = Some(v);
                }
            } else if key.contains("context_length") || key.contains("max_position_embeddings") {
                if let Ok(v) = value.parse::<usize>() {
                    self.max_context_length = Some(v);
                }
            } else if key == "general.architecture" {
                self.model_identity = value.clone();
            }
        }
    }

    pub fn sync_to_archive(&self, target_path: &std::path::Path) -> Result<(), String> {
        let bytes = rkyv::to_bytes::<StructuralDNA, 1024>(self)
            .map_err(|e| format!("Archive Failed: {e}"))?;
        std::fs::write(target_path, bytes).map_err(|e| format!("Disk Write Failed: {e}"))?;
        println!("✅ [DNA] Sovereign Archive Created: {:?}", target_path);
        Ok(())
    }

    /// 🛠️ Parser: Converts manifest context strings (e.g., "8k", "128k") to usize.
    pub fn parse_context_string(ctx_str: &str) -> usize {
        let normalized = ctx_str.to_lowercase();
        if normalized.ends_with('k') {
            let num = normalized
                .trim_end_matches('k')
                .parse::<usize>()
                .unwrap_or(4);
            num * 1024
        } else if normalized.ends_with('m') {
            let num = normalized
                .trim_end_matches('m')
                .parse::<usize>()
                .unwrap_or(1);
            num * 1024 * 1024
        } else {
            normalized.parse::<usize>().unwrap_or(4096)
        }
    }

    /// 🛠️ Skeleton Factory: Creates a primed DNA backbone from manifest data.
    pub fn create_skeleton(
        id: String,
        has_vision: bool,
        expert_count: Option<usize>,
        bit_depth: f64,
        context_window: &str,
    ) -> Self {
        let mut signature = KernelSignature::default();
        signature.is_multimodal = has_vision;
        if expert_count.is_some() {
            signature.has_experts = true;
        }

        let mut preferred_runtime = Some(BackendType::RuntimeA); // Default: Candle
        if bit_depth < 2.0 {
            signature.is_bitnet = true;
            preferred_runtime = Some(BackendType::RuntimeB); // BitNet -> Llama.cpp
        }

        Self {
            model_identity: id,
            signature,
            preferred_runtime,
            max_context_length: Some(Self::parse_context_string(context_window)),
            ..Default::default()
        }
    }
}
