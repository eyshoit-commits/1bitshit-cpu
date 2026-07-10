use crate::state::AppState;
use axum::{
    extract::{Path, State},
    response::Json,
};
use chrono::Utc;
use engines::models::entities::{ChatMessage, ChatRequest, ChatResponse, ChatSession, MessageRole};
use serde_json::{json, Value};
use std::sync::Arc;

// ─── POST /chat — The Main Event ─────────────────────────────────────
pub async fn chat(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ChatRequest>,
) -> Json<ChatResponse> {
    // Zero-IPC routing: Directly send prompt through the Dispatcher
    let dispatch_result = state.dispatcher.dispatch_prompt(&request.message).await;

    let content = match dispatch_result {
        Ok(res) => res,
        Err(e) => format!("Error processing prompt: {}", e),
    };

    let response = ChatResponse {
        id: format!("resp-{}", Utc::now().timestamp()),
        session_id: request.session_id.clone(),
        message: content,
        role: MessageRole::Assistant,
        timestamp: Utc::now(),
        model: "Sovereign-Internal".to_string(), // In production, this would be the loaded model ID
        tokens_used: 0,
    };

    Json(response)
}

// ─── GET /history/:session_id ────────────────────────────────────────
pub async fn get_history(
    State(_state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
) -> Json<Value> {
    // TODO: Connect history to the EmbeddedManager instead of the removed Kernel
    Json(json!({
        "success": false,
        "error": format!("History retrieval pending EmbeddedManager integration for session '{}'", session_id)
    }))
}

// ─── GET /sessions ───────────────────────────────────────────────────
pub async fn get_sessions(State(_state): State<Arc<AppState>>) -> Json<Value> {
    // TODO: Connect sessions to the EmbeddedManager
    Json(json!({
        "success": true,
        "count": 0,
        "sessions": []
    }))
}
