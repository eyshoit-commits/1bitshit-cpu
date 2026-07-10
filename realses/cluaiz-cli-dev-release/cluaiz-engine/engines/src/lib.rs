//! ═══════════════════════════════════════════════════════════════════════
//!  CURE Engine: Master Archer Unified Core Gateway
//! ═══════════════════════════════════════════════════════════════════════

// 🧬 Archer Cluaiz Core Core

pub mod api;
// pub mod kernel; // Removed in V7 DNA Refactor
pub mod cli;
pub mod hardware;
#[path = "interface-engines/mod.rs"]
pub mod interface_engines;
pub mod memory;
pub mod models;
pub mod neural_foundry;
pub mod runtime;
pub mod sync;
pub mod telemetry;
pub mod utils;

// ─── Master Archer Unified Access ────────────────────────────────────

// pub use kernel::core::CureKernel; // Removed in V7 DNA Refactor

// 2. Hardware & Architecture
pub use hardware::system_control_manager::{
    detect_hardware, has_config, read_config, save_config, update_field,
};
pub use hardware::{
    HardwareDetector, InferenceEngine, InferenceEvent, SiliconTruth, SiliconTruth as HardwareInfo,
};

// 3. Execution & Inference
pub use runtime::execution::loader::GGUFLoader;
pub use runtime::execution::runner::{CluaizMetrics, CluaizRunner};
pub use runtime::execution::sampler::CoreSampler;

// 4. Intelligence & Registry
pub use models::entities::{ChatMessage, ChatRequest, ChatResponse, ChatSession, MessageRole};
pub use models::fetch::{DownloadEvent, ModelDownloader};
pub use models::registry::{CoreRoster, ModelAsset, ModelManifest, ModelRecommendation};

// 5. Routing
pub use api::router::CoreRouter;
pub use cluaiz_shared::BackendType;
