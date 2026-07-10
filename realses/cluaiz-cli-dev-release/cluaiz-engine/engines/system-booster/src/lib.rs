//! ═══════════════════════════════════════════════════════════════════════
//!  CURE External Crate: System Booster (Bare Metal Isolator)
//! ═══════════════════════════════════════════════════════════════════════

pub mod dflash;
pub mod manager;
pub mod os_tuning;
pub mod speculative;

pub mod system_booster;
pub mod telemetry;

// 🏛️ Reusing the Unified Architecture from archer-shared
pub use cluaiz_shared::hardware::governor::HardwareGovernor;
pub use cluaiz_shared::hardware::schema::booster::{BoosterControl, FeatureState};
pub use system_booster::SystemBooster;
