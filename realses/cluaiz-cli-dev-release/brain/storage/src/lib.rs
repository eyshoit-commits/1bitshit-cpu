//! ═══════════════════════════════════════════════════════════════════════
//!  CURE Storage — The Embedded Engine
//! ═══════════════════════════════════════════════════════════════════════
//!  Contains native ultra-lightweight database connections.
//! ═══════════════════════════════════════════════════════════════════════

pub mod manager;
pub mod models;

pub use manager::EmbeddedManager;
pub use models::{EmbeddedKind, EmbeddedStatus};
