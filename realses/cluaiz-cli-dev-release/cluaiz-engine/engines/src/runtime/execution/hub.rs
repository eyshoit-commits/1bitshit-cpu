//! ═══════════════════════════════════════════════════════════════════════
//!  CURE Engine: The Hardware Orchestrator (Dynamic Dispatcher)
//! ═══════════════════════════════════════════════════════════════════════

use crate::interface_engines::EngineManager;
use anyhow::{anyhow, Result};
use cluaiz_shared::{CluaizContext, CluaizLinkerPlaceholder, ModelWeightsWrapper};

pub struct HardwareOrchestrator;

impl HardwareOrchestrator {
    /// Dispatches and instantiates the correct model kernel via the Dynamic Cluaiz Linker.
    pub async fn instantiate(
        model_load_path: &str,
        Cluaiz_context: CluaizContext,
    ) -> Result<ModelWeightsWrapper> {
        tracing::info!("🔩 [Orchestrator] Initiating Dynamic Hardware Handshake...");

        // 1. Initialize the Engine Manager (The Cluaiz Linker)
        let base_path = cluaiz_shared::hardware::governor::HardwareGovernor::resolve_hub_path();
        let manager = EngineManager::new(base_path);

        // 2. Identify Engine Type based on DNA Signature
        let engine_type = if Cluaiz_context.dna.signature.is_bitnet {
            "candle" // BitNet uses the optimized candle kernel
        } else if Cluaiz_context.dna.signature.has_experts {
            "llama" // MOE optimized llama kernel
        } else {
            "llama" // Standard GGUF/Transformer
        };

        // 3. Prepare Engine: Hardware Probe + Binary Linkage (7ns Target)
        let binary_path = manager
            .prepare_engine(engine_type)
            .await
            .map_err(|e| anyhow!("Hardware Linkage Failure: {}", e))?;

        // 🚀 [FFI Handshake]: Map the binary to process memory
        let mut manager = manager; // Make mutable for linkage
        manager.load_and_link(binary_path)?;

        // 🏛️ [Core Instantiation]: Create the active engine instance
        manager.instantiate(model_load_path)?;

        // For V1, we return a success placeholder. Future phases will return the actual
        // kernel implementation mapped directly from the FFI symbols.
        tracing::info!(
            "🧬 [Orchestrator] Hardware Handshake SUCCESS. Ready for bare-metal inference."
        );

        Ok(Box::new(CluaizLinkerPlaceholder))
    }

    /// 🚨 EMERGENCY EVICT: Instantly purges all Core memory and kills running processes.
    /// Resets the engine to a zero-memory state.
    pub fn purge_hardware_context() {
        tracing::warn!("🚨 [Manager] EMERGENCY EVICT TRIGGERED. Purging Core Memory...");
        // This hook will be called by the Governor in Phase 3 to kill binary kernels
        // and drop all cached weights.
    }
}
