//! ═══════════════════════════════════════════════════════════════════════
//!  CURE Storage — The Embedded Engine
//! ═══════════════════════════════════════════════════════════════════════
//!  Contains native ultra-lightweight database connections.
//! ═══════════════════════════════════════════════════════════════════════

pub mod models;
pub mod manager;

pub use models::{EmbeddedKind, EmbeddedStatus};
pub use manager::EmbeddedManager;
