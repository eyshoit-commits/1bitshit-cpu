use crate::state::AppState;
use axum::{extract::State, response::Json};
use serde_json::{json, Value};
use std::sync::Arc;

// ─── Root ────────────────────────────────────────────────────────────
pub async fn root() -> Json<Value> {
    Json(json!({
        "engine": "CURE — Cluaiz Universal Runtime Engine",
        "version": env!("CARGO_PKG_VERSION"),
        "gateway": "http://localhost:8000",
        "endpoints": {
            "GET  /":                 "This welcome message",
            "GET  /health":           "Engine health check",
            "GET  /info":             "System information & pillars",
            "POST /chat":             "Send message → get AI response",
            "GET  /history/:session": "Chat history for a session",
            "GET  /sessions":         "List all chat sessions",
            "GET  /status/sidecars":  "Database sidecar status",
            "GET  /hardware":         "Detect system RAM/CPU to suggest models",
            "POST /models/download":  "Download .gguf from Hugging Face",
            "POST /models/load":      "Load a downloaded .gguf file"
        },
        "philosophy": "Nothing Need. Just CURE."
    }))
}

// ─── Health Check ────────────────────────────────────────────────────
pub async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "alive",
        "engine": "CURE — Cluaiz Universal Runtime Engine",
        "version": env!("CARGO_PKG_VERSION"),
        "message": "🚀 CURE is alive! All systems operational."
    }))
}

// ─── System Info ─────────────────────────────────────────────────────
pub async fn system_info() -> Json<Value> {
    Json(json!({
        "engine": "CURE",
        "full_name": "Cluaiz Universal Runtime Engine",
        "version": env!("CARGO_PKG_VERSION"),
        "pillars": {
            "api": "Gateway — HTTP server on port 8000 (this!)",
            "kernel": "Brain — Decision-making & orchestration",
            "storage": "Sidecars — 5 Official DB engines (Mongo, Neo4j, ClickHouse, Qdrant, MinIO)",
            "engines": "Muscles — C++ model inference via llama.cpp FFI"
        },
        "philosophy": "Nothing Need. Just CURE.",
        "banned": ["Python", "Docker", "Ollama", "npm", "pip"]
    }))
}

// ─── GET /status/embedded ────────────────────────────────────────────
pub async fn embedded_status(State(state): State<Arc<AppState>>) -> Json<Value> {
    let statuses = state.embedded.health_check_all().await;
    Json(json!({
        "success": true,
        "sidecars": statuses
    }))
}
