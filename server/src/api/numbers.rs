use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    api::state::AppState,
    auth::{AdminToken, AuthenticatedToken},
    db,
};

#[derive(Debug, Deserialize)]
pub struct UpsertNumberRequest {
    pub phone_number: String,
    pub label: Option<String>,
}

pub async fn list_numbers(
    State(state): State<AppState>,
    AuthenticatedToken(token): AuthenticatedToken,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let conn = state.pool.get().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "db_error", "message": e.to_string() })),
        )
    })?;

    let numbers = db::list_phone_numbers(&conn, &token).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "db_error", "message": e.to_string() })),
        )
    })?;

    Ok(Json(json!({
        "phone_numbers": numbers,
        "count": numbers.len(),
    })))
}

pub async fn upsert_number(
    State(state): State<AppState>,
    _admin: AdminToken,
    Json(payload): Json<UpsertNumberRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let number = payload.phone_number.trim();
    if number.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "bad_request", "message": "phone_number cannot be empty" })),
        ));
    }

    let conn = state.pool.get().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "db_error", "message": e.to_string() })),
        )
    })?;

    let res = db::upsert_phone_number(&conn, number, payload.label.as_deref(), None).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "db_error", "message": e.to_string() })),
        )
    })?;

    Ok(Json(json!({ "success": true, "phone_number": res })))
}
