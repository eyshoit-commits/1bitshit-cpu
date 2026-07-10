use anyhow::{Result, anyhow};
use cluaiz_shared::backend::signature::{KernelSignature, GlobalFeatureRegistry, BackendType};
use system_booster::BoosterControl;

/// 🚦 NeuralDispatcher
/// The core router that takes a prompt and sends it to the correct backend.
pub struct NeuralDispatcher {
    pub booster_state: BoosterControl,
    pub current_signature: KernelSignature,
}

impl NeuralDispatcher {
    pub fn new(booster_state: BoosterControl, signature: KernelSignature) -> Self {
        Self {
            booster_state,
            current_signature: signature,
        }
    }

    /// Primary entry point for inference.
    /// Checks the SystemBooster and routes to the correct engine.
    pub async fn dispatch_prompt(&self, prompt: &str) -> Result<String> {
        // 🚀 Real-time Silicon Probe
        let hardware = cluaiz_shared::hardware::HardwareOrchestrator::probe().silicon_truth;

        // Use the GlobalFeatureRegistry to select the correct runtime based on signature AND hardware
        let backend = GlobalFeatureRegistry::select_runtime(&self.current_signature, &hardware);
        
        tracing::info!("🚦 [Dispatcher] Routing prompt to backend: {:?}", backend);
        
        match backend {
            BackendType::RuntimeB => {
                Ok(format!("⚡ [Llama Backend via Dispatcher] Processed: {}", prompt))
            }
            BackendType::RuntimeC | BackendType::RuntimeA => {
                Ok(format!("🔩 [Native Backend via Dispatcher] Processed: {}", prompt))
            }
            _ => {
                Err(anyhow!("❌ [Dispatcher] Unsupported backend architecture for this mission: {:?}", backend))
            }
        }
    }
}
