use axum::{
    extract::State,
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

pub async fn create_device(
    State(state): State<AppState>,
    _admin: AdminToken,
    Json(payload): Json<CreateDeviceRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    if payload.name.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "bad_request", "message": "Device name cannot be empty" })),
        ));
    }

    let conn = state.pool.get().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "db_error", "message": e.to_string() })),
        )
    })?;

    let (device, token) = db::register_device(&conn, &payload.name, &payload.platform, None).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "db_error", "message": e.to_string() })),
        )
    })?;

    // Create QR-code data payload for easy scanning in the Android app
    let qr_data = json!({
        "server_url": *state.server_url,
        "device_id": device.id,
        "device_token": token,
        "device_name": device.name,
    }).to_string();

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "device": device,
            "token": token,
            "qr_data": qr_data,
        })),
    ))
}

pub async fn list_devices(
    State(state): State<AppState>,
    _admin: AdminToken,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let conn = state.pool.get().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "db_error", "message": e.to_string() })),
        )
    })?;

    let devices = db::list_devices(&conn).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "db_error", "message": e.to_string() })),
        )
    })?;

    Ok(Json(json!({ "devices": devices, "count": devices.len() })))
}
