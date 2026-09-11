use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;
use std::sync::Arc;
use tracing::info;

use crate::{
    api::state::AppState,
    auth::{AuthenticatedToken, DeviceOrAdmin},
    db::{self, models::*},
    otp::extract_otp,
};

/// Ingestion endpoint: receives forwarded SMS from Android / iOS / Test
pub async fn forward_sms(
    State(state): State<AppState>,
    device_auth: DeviceOrAdmin,
    Json(mut payload): Json<IngestSmsRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // If sent by a registered device and payload didn't specify device_id, fill it in
    if payload.device_id.is_none() {
        if let Some(ref dev) = device_auth.device {
            payload.device_id = Some(dev.id.clone());
        }
    }

    // Clean phone number (strip whitespace)
    payload.recipient_number = payload.recipient_number.trim().to_string();
    payload.sender = payload.sender.trim().to_string();

    if payload.recipient_number.is_empty() || payload.body.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "bad_request", "message": "recipient_number and body cannot be empty" })),
        ));
    }

    // Extract OTP if present
    let otp_code = extract_otp(&payload.body);

    info!(
        "📲 Received SMS for {} from {}: {} (Extracted OTP: {:?})",
        payload.recipient_number,
        payload.sender,
        if payload.body.len() > 30 { &payload.body[..30] } else { &payload.body },
        otp_code
    );

    let conn = state.pool.get().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "db_error", "message": e.to_string() })),
        )
    })?;

    let message = db::insert_message(&conn, payload, otp_code.clone()).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "db_error", "message": e.to_string() })),
        )
    })?;

    let arc_msg = Arc::new(message.clone());
    // Broadcast to active WebSocket clients
    let _ = state.tx.send(arc_msg);

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "message_id": message.id,
            "recipient_number": message.recipient_number,
            "extracted_code": message.extracted_code,
            "received_at": message.received_at,
        })),
    ))
}

/// Query messages filtered by token permissions
pub async fn list_messages(
    State(state): State<AppState>,
    AuthenticatedToken(token): AuthenticatedToken,
    Query(filter): Query<MessageQueryFilter>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // If specific number requested, check permission explicitly
    if let Some(ref num) = filter.number {
        if !token.can_access_number(num) {
            return Err((
                StatusCode::FORBIDDEN,
                Json(json!({
                    "error": "forbidden",
                    "message": format!("Token is not authorized to access number {}", num)
                })),
            ));
        }
    }

    let conn = state.pool.get().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "db_error", "message": e.to_string() })),
        )
    })?;

    let messages = db::query_messages(&conn, &filter, &token).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "db_error", "message": e.to_string() })),
        )
    })?;

    let count = messages.len();
    Ok(Json(json!({
        "messages": messages,
        "count": count,
    })))
}

/// Get the latest OTP code received for an authorized number
pub async fn get_latest_otp(
    State(state): State<AppState>,
    AuthenticatedToken(token): AuthenticatedToken,
    Query(params): Query<serde_json::Value>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let number = params.get("number").and_then(|v| v.as_str());
    let max_age = params
        .get("max_age_minutes")
        .and_then(|v| v.as_i64())
        .unwrap_or(10);

    if let Some(num) = number {
        if !token.can_access_number(num) {
            return Err((
                StatusCode::FORBIDDEN,
                Json(json!({
                    "error": "forbidden",
                    "message": format!("Token is not authorized to access number {}", num)
                })),
            ));
        }
    }

    let conn = state.pool.get().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "db_error", "message": e.to_string() })),
        )
    })?;

    let otp_result = db::get_latest_otp(&conn, number, &token, max_age).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "db_error", "message": e.to_string() })),
        )
    })?;

    match otp_result {
        Some(otp) => Ok(Json(json!({
            "found": true,
            "otp": otp,
        }))),
        None => Ok(Json(json!({
            "found": false,
            "message": "No recent OTP found within the specified time window for authorized numbers",
        }))),
    }
}

/// Get single message by ID
pub async fn get_message(
    State(state): State<AppState>,
    AuthenticatedToken(token): AuthenticatedToken,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let conn = state.pool.get().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "db_error", "message": e.to_string() })),
        )
    })?;

    let message = db::get_message_by_id(&conn, &id, &token).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "db_error", "message": e.to_string() })),
        )
    })?;

    match message {
        Some(msg) => Ok(Json(json!({ "message": msg }))),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "not_found", "message": "Message not found or unauthorized" })),
        )),
    }
}

/// Delete message by ID
pub async fn delete_message(
    State(state): State<AppState>,
    AuthenticatedToken(token): AuthenticatedToken,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    if !token.can_delete && !token.is_admin {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "forbidden", "message": "Token lacks delete permission" })),
        ));
    }

    let conn = state.pool.get().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "db_error", "message": e.to_string() })),
        )
    })?;

    let deleted = db::delete_message(&conn, &id, &token).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "db_error", "message": e.to_string() })),
        )
    })?;

    if deleted {
        Ok(Json(json!({ "success": true, "message": "Message deleted" })))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "not_found", "message": "Message not found or unauthorized" })),
        ))
    }
}
