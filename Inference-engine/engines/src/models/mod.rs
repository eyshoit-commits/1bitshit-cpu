//! Public model subsystem for the 1BitShit CPU engine.
//!
//! Llama/GGUF, BitNet and ONNX models share one visible model store. The
//! downloader, cache lookup, purge and runtime loader therefore resolve the
//! same canonical directory.

pub mod entities;
#[path = "fetch_v2.rs"]
pub mod fetch;
pub mod manager;
pub mod registry;
