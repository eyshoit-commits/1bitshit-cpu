use axum::http::Method;
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

use crate::handlers::{chat, models, system};
use crate::state::AppState;

pub fn build(state: Arc<AppState>) -> Router {
    // ── CORS — Allow any origin (Desktop, Mobile, Web can all call) ──
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any);

    Router::new()
        .route("/", get(system::root))
        .route("/health", get(system::health_check))
        .route("/info", get(system::system_info))
        .route("/status/embedded", get(system::embedded_status))
        .route("/chat", post(chat::chat))
        .route("/history/{session_id}", get(chat::get_history))
        .route("/history", get(chat::get_sessions))
        // ── Models API ──
        .route("/models/available", get(models::list_models))
        .route("/hardware", get(models::hardware_status))
        .route("/models/download", post(models::download_model))
        .route("/models/load", post(models::load_model))
        .layer(cors)
        .with_state(state)
}
