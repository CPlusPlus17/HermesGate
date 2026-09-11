use axum::{extract::State, response::IntoResponse, Json};
use chrono::Utc;
use serde_json::json;

use crate::api::state::AppState;

pub async fn health_check(State(_state): State<AppState>) -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "service": "hermesgate",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": Utc::now().to_rfc3339(),
    }))
}
