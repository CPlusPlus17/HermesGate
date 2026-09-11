use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;

use crate::{
    api::state::AppState,
    auth::AdminToken,
    db::{self, models::*},
};

pub async fn create_token(
    State(state): State<AppState>,
    _admin: AdminToken,
    Json(payload): Json<CreateTokenRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    if payload.name.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "bad_request", "message": "Token name cannot be empty" })),
        ));
    }

    let conn = state.pool.get().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "db_error", "message": e.to_string() })),
        )
    })?;

    let (token, secret) = db::create_api_token(&conn, payload).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "db_error", "message": e.to_string() })),
        )
    })?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "token": token,
            "secret": secret,
            "message": "Store this secret safely; it will not be shown again."
        })),
    ))
}

pub async fn list_tokens(
    State(state): State<AppState>,
    _admin: AdminToken,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let conn = state.pool.get().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "db_error", "message": e.to_string() })),
        )
    })?;

    let tokens = db::list_api_tokens(&conn).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "db_error", "message": e.to_string() })),
        )
    })?;

    Ok(Json(json!({ "tokens": tokens, "count": tokens.len() })))
}

pub async fn delete_token(
    State(state): State<AppState>,
    _admin: AdminToken,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let conn = state.pool.get().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "db_error", "message": e.to_string() })),
        )
    })?;

    let deleted = db::delete_api_token(&conn, &id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "db_error", "message": e.to_string() })),
        )
    })?;

    if deleted {
        Ok(Json(json!({ "success": true, "message": "Token revoked successfully" })))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "not_found", "message": "Token not found" })),
        ))
    }
}
