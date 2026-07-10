use axum::{extract::State, Json};
use serde_json::{json, Value};
use std::sync::Arc;
use crate::state::AppState;
use engines::{HardwareDetector, CoreRoster};
use sysinfo::System;

// ─── GET /models/available ───────────────────────────────────────
pub async fn list_models(State(_state): State<Arc<AppState>>) -> Json<Value> {
    let mut sys = System::new_all();
    sys.refresh_all();
    let ram_gb = sys.total_memory() as f64 / 1_073_741_824.0; // Bytes to GB

    let silicon = HardwareDetector::new().detect();
    let recommendations = CoreRoster::get_recommendations(&silicon, ram_gb);
    
    Json(json!({
        "success": true,
        "system_ram_gb": format!("{:.2}", ram_gb),
        "available_models": recommendations
    }))
}

// ─── GET /hardware ───────────────────────────────────────────────
pub async fn hardware_status(State(_state): State<Arc<AppState>>) -> Json<Value> {
    let stats = HardwareDetector::new().detect();
    Json(json!({
        "success": true,
        "hardware": stats
    }))
}

// ─── POST /models/download ───────────────────────────────────────
pub async fn download_model(State(_state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({
        "success": true,
        "status": "Download feature queued."
    }))
}

// ─── POST /models/load ───────────────────────────────────────────
pub async fn load_model(State(_state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({
        "success": true,
        "status": "Model load feature queued."
    }))
}
