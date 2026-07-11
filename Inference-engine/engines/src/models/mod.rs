//! Public model subsystem for the 1BitShit CPU engine.
//!
//! Keep this module entry point committed. `engines/src/lib.rs` exposes it via
//! `pub mod models;`, so a missing file makes the entire workspace fail before
//! any runtime or model-download code can be tested.

pub mod entities;
pub mod fetch;
pub mod manager;
pub mod registry;
