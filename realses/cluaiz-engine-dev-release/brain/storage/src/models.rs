//! ═══════════════════════════════════════════════════════════════════════
//!  CURE Storage: Embedded Data Vault Models
//! ═══════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};

/// The type of embedded database engine
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EmbeddedKind {
    SurrealDB,
    LanceDB,
    DuckDB,
    NativeFS,
}

impl std::fmt::Display for EmbeddedKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmbeddedKind::SurrealDB => write!(f, "SurrealDB (Docs+Graph)"),
            EmbeddedKind::LanceDB   => write!(f, "LanceDB (Vector)"),
            EmbeddedKind::DuckDB    => write!(f, "DuckDB (Analytics)"),
            EmbeddedKind::NativeFS  => write!(f, "Native FS (Blobs)"),
        }
    }
}

/// Status of the embedded database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedStatus {
    pub kind: EmbeddedKind,
    pub status: String,      // "online", "initializing", "error"
    pub memory_footprint_mb: u32, 
    pub storage_path: String,
    pub is_active: bool,
}
