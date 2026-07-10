use std::path::Path;
use anyhow::{Result, anyhow};
use tokenizers::Tokenizer;
use crate::models::registry::Provisioner;
use crate::runtime::execution::hub::HardwareOrchestrator as CoreHub;
use cluaiz_shared::{ModelWeightsWrapper, CluaizContext, StructuralDNA, TemplateManager};
use cluaiz_shared::utils::GGUFProber;

/// GGUFLoader: Lightweight orchestrator for quantized Core models.
pub struct GGUFLoader;

impl GGUFLoader {
    pub async fn load_model(path: &Path, hf_repo: &str) -> Result<(ModelWeightsWrapper, Tokenizer, Option<u32>)> {
        // 1. Detect Architecture via Native Prober (Zero Framework Bloat)
        let (metadata, _tensor_infos) = GGUFProber::probe(path)
            .map_err(|e| anyhow!("Native Probe Failure: {}", e))?;
        
        let arch = metadata.get("general.architecture")
            .ok_or_else(|| anyhow!("Registry Alert: Architecture metadata missing in GGUF file."))?;
        
        // [Cluaiz CLEAN]: Replaced tracing::info with println for editor stability
        println!("🔍 Autonomous Discovery: Probed architecture '{}' via Native Prober", arch);

        // 2. Extract Special Tokens (Resilient Handshake)
        let bos_token_id = metadata.get("tokenizer.ggml.bos_token_id")
            .and_then(|v| v.parse::<u32>().ok());

        // 3. Identify Metadata Assets (Structural DNA)
        let model_dir = path.parent().ok_or_else(|| anyhow!("Invalid model path structure."))?;
        let dna_path = model_dir.join("structural_dna.json");
        let architectural_dna = if dna_path.exists() {
             StructuralDNA::load(&dna_path)
                .map_err(|load_err| anyhow!("Cluaiz Boot Failure: DNA corrupt. Detail: {}", load_err))?
        } else {
             StructuralDNA::default()
        };

        let tokenizer_path = Provisioner::ensure_assets(model_dir, hf_repo, None, "tokenizer.json").await?;
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| anyhow!("Core Hardware Error: Failed to load tokenizer: {}", e))?;

        // 🧬 Cluaiz ACTIVATION: Dynamic Context Bootstrapping
        let Cluaiz_context = CluaizContext::boot(
            architectural_dna,
            TemplateManager::default()
        );
 
        // 4. Delegate Instantiation to the Core Hub (Universal DNA Dispatch)
        let model = CoreHub::instantiate(path.to_string_lossy().as_ref(), Cluaiz_context).await?;
  
        Ok((model, tokenizer, bos_token_id))
    }
}
