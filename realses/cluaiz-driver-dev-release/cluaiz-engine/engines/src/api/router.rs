//! router.rs: The Core Dispatcher.
//! Routes prompts to the appropriate backend based on model architecture.

use std::path::PathBuf;
use crate::utils::healer::AutoHealer;
use cluaiz_shared::{UnifiedBackend, BackendType, CluaizContext, StructuralDNA, TemplateManager, ModelWeightsWrapper};
use crate::runtime::execution::hub::HardwareOrchestrator;
use candle_core::Device;

pub enum Backend {
    Empty(DummyBackend),
    Cluaiz(ModelWeightsWrapper),
}

impl UnifiedBackend for Backend {
    fn generate(&mut self, prompt: &str, max_tokens: usize) -> Result<String, String> {
        match self {
            Self::Empty(b) => b.generate(prompt, max_tokens),
            Self::Cluaiz(b) => b.generate(prompt, max_tokens),
        }
    }
    fn prefill(&mut self, prompt: &str) -> anyhow::Result<()> {
        match self {
            Self::Empty(b) => b.prefill(prompt),
            Self::Cluaiz(b) => b.prefill(prompt),
        }
    }

    fn evaluate_tps(&self) -> f64 {
        match self {
            Self::Empty(b) => b.evaluate_tps(),
            Self::Cluaiz(b) => b.evaluate_tps(),
        }
    }
}

impl cluaiz_shared::CluaizInference for Backend {
    fn forward_raw(&mut self, inputs: &[u32], pos: usize) -> anyhow::Result<Vec<f32>> {
        match self {
            Self::Cluaiz(b) => b.forward_raw(inputs, pos),
            Self::Empty(_) => Err(anyhow::anyhow!("Empty backend")),
        }
    }

    fn generate_stream(
        &mut self,
        prompt: &str,
        max_tokens: usize,
        tokenizer: &tokenizers::Tokenizer,
        callback: Box<dyn FnMut(String) + Send + 'static>,
    ) -> anyhow::Result<()> {
        match self {
            Self::Cluaiz(b) => b.generate_stream(prompt, max_tokens, tokenizer, callback),
            Self::Empty(_) => Err(anyhow::anyhow!("Empty backend")),
        }
    }
}

pub struct CoreRouter {
    pub active_backend: Backend,
    pub tokenizer: Option<tokenizers::Tokenizer>,
    pub foundry: crate::neural_foundry::CoreFoundry,
}

impl Default for CoreRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl CoreRouter {
    pub fn new() -> Self {
        Self { 
            active_backend: Backend::Empty(DummyBackend),
            tokenizer: None,
            foundry: crate::neural_foundry::CoreFoundry::new(),
        }
    }

    pub async fn load_model(path: PathBuf, runtime: BackendType, _device: &Device) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            let repo_id = path.file_stem().map(|s| s.to_string_lossy()).unwrap_or_default();
            let _ = AutoHealer::heal_missing_tokenizer(&repo_id, parent).await;
        }

        // [Cluaiz ALIGNMENT]: Bootstrapping context with local DNA and Templates
        let mut dna = StructuralDNA::default();
        if let Some(parent) = path.parent() {
            let dna_path = parent.join("structural_dna.json");
            if dna_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&dna_path) {
                    if let Ok(loaded_dna) = serde_json::from_str::<StructuralDNA>(&content) {
                        dna = loaded_dna;
                        println!("🧬 [Router] Neural DNA synchronized from local manifest.");
                    }
                }
            }
        }
        dna.preferred_runtime = Some(runtime);
        
        let context = CluaizContext::boot(
            dna,
            TemplateManager::default(),
        );

        // 🚀 THE Cluaiz HANDSHAKE: Dispatching to the Dynamic Linker
        println!("🧬 [Router] Dispatching to HardwareOrchestrator for dynamic linkage...");
        let engine = HardwareOrchestrator::instantiate(&path.to_string_lossy(), context)
            .await
            .map_err(|e| format!("Cluaiz Handshake Failure: {}", e))?;

        let (tokenizer, t_error) = if let Some(p) = path.parent() {
            let t_path = p.join("tokenizer.json");
            if t_path.exists() {
                match tokenizers::Tokenizer::from_file(&t_path) {
                    Ok(t) => (Some(t), None),
                    Err(e) => (None, Some(format!("Tokenizer found but failed to parse: {}", e))),
                }
            } else {
                (None, Some(format!("tokenizer.json missing at {:?}", t_path)))
            }
        } else {
            (None, Some("Invalid model path parent.".to_string()))
        };

        if let Some(err) = t_error {
            println!("🗣️ [Router] Voice initialization fail: {}", err);
        }

        let mut foundry = crate::neural_foundry::CoreFoundry::new();
        // Load skills from a standard location (this could be configurable)
        foundry.initialize("skills");

        Ok(Self { 
            active_backend: Backend::Cluaiz(engine), 
            tokenizer,
            foundry 
        })
    }

    pub fn generate(&mut self, prompt: &str, max_tokens: usize) -> Result<String, String> {
        self.active_backend.generate(prompt, max_tokens)
    }

    pub fn generate_stream(
        &mut self,
        prompt: &str,
        max_tokens: usize,
        callback: Box<dyn FnMut(String) + Send + 'static>,
    ) -> Result<(), String> {
        // 🧪 Cluaiz HANDSHAKE: Check for skills before generation
        let rt = tokio::runtime::Handle::current();
        let intent_result = rt.block_on(self.foundry.process_intent(prompt))
            .map_err(|e| format!("Skill Discovery Error: {}", e))?;

        match &mut self.active_backend {
            Backend::Cluaiz(b) => {
                // If Core signals (skill souls) were identified, inject them into the kernel
                if !intent_result.signals.is_empty() {
                    println!("💉 [Router] Injecting {} Core signals into active backend...", intent_result.signals.len());
                    b.inject_signals(intent_result.signals).map_err(|e| format!("Signal Injection Failure: {}", e))?;
                }

                if let Some(ref tokenizer) = self.tokenizer {
                    b.generate_stream(prompt, max_tokens, tokenizer, callback)
                        .map_err(|e| e.to_string())
                } else {
                    Err("Tokenizer not loaded.".to_string())
                }
            },
            Backend::Empty(_) => Err("Core weights not loaded. Please select a model with @ or wait for the Auto-Pilot handshake to complete.".to_string()),
        }
    }
}

pub struct DummyBackend;
impl cluaiz_shared::UnifiedBackend for DummyBackend {
    fn generate(&mut self, _prompt: &str, _max_tokens: usize) -> Result<String, String> {
        Err("Core weights not loaded.".to_string())
    }
    fn prefill(&mut self, _prompt: &str) -> anyhow::Result<()> { Ok(()) }
    fn evaluate_tps(&self) -> f64 { 0.0 }
}

impl cluaiz_shared::CluaizInference for DummyBackend {
    fn forward_raw(&mut self, _inputs: &[u32], _pos: usize) -> anyhow::Result<Vec<f32>> {
        Err(anyhow::anyhow!("Dummy backend"))
    }
    fn generate_stream(
        &mut self,
        _prompt: &str,
        _max_tokens: usize,
        _tokenizer: &tokenizers::Tokenizer,
        _callback: Box<dyn FnMut(String) + Send + 'static>,
    ) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("Dummy backend"))
    }
}
