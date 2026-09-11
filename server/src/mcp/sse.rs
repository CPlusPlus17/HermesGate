use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    Json,
};
use serde::Deserialize;
use serde_json::json;
use std::{collections::HashMap, convert::Infallible, sync::Arc, time::Duration};
use tokio::sync::{mpsc, Mutex};
use tokio_stream::wrappers::ReceiverStream;
use tracing::info;

use crate::{
    api::state::AppState,
    auth::extract_token_from_headers,
    db::verify_api_token,
    mcp::{handler::handle_mcp_request, protocol::*},
};

#[derive(Clone)]
#[allow(dead_code)]
pub struct McpSseManager {
    // Maps session_id to an mpsc sender for SSE events
    sessions: Arc<Mutex<HashMap<String, mpsc::Sender<String>>>>,
}

impl McpSseManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[derive(Deserialize)]
pub struct SseParams {
    pub token: Option<String>,
}

#[derive(Deserialize)]
pub struct MessageParams {
    pub session_id: Option<String>,
    pub token: Option<String>,
}

/// GET /mcp/sse: MCP SSE endpoint
pub async fn sse_connect(
    State(state): State<AppState>,
    Query(params): Query<SseParams>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let token_str = params
        .token
        .or_else(|| extract_token_from_headers(request.headers()))
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "unauthorized", "message": "Missing ?token= parameter or Authorization header" })),
            )
        })?;

    let conn = state.pool.get().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "db_error", "message": e.to_string() })),
        )
    })?;

    let token = verify_api_token(&conn, &token_str)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "db_error", "message": e.to_string() })),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "unauthorized", "message": "Invalid token" })),
            )
        })?;

    let session_id = uuid::Uuid::new_v4().to_string();
    info!("🤖 MCP SSE client connected: {} (Session: {})", token.name, session_id);

    let (tx, rx) = mpsc::channel::<String>(100);

    // Initial endpoint event according to MCP specification
    let endpoint_uri = format!("/mcp/messages?session_id={}&token={}", session_id, token_str);
    let _ = tx.send(endpoint_uri).await;

    let stream = ReceiverStream::new(rx);
    let sse_stream = tokio_stream::StreamExt::map(stream, move |data| {
        if data.starts_with("/mcp/messages") {
            Ok::<_, Infallible>(Event::default().event("endpoint").data(data))
        } else {
            Ok::<_, Infallible>(Event::default().event("message").data(data))
        }
    });

    Ok(Sse::new(sse_stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)).text("ping"))
        .into_response())
}

/// POST /mcp/messages: Receives JSON-RPC request for MCP
pub async fn receive_message(
    State(state): State<AppState>,
    Query(params): Query<MessageParams>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let token_str = params
        .token
        .or_else(|| extract_token_from_headers(request.headers()))
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "unauthorized", "message": "Missing authentication token" })),
            )
        })?;

    let conn = state.pool.get().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "db_error", "message": e.to_string() })),
        )
    })?;

    let token = verify_api_token(&conn, &token_str)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "db_error", "message": e.to_string() })),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "unauthorized", "message": "Invalid token" })),
            )
        })?;

    let body_bytes = axum::body::to_bytes(request.into_body(), 1024 * 1024)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "bad_request", "message": e.to_string() })),
            )
        })?;

    let rpc_req: JsonRpcRequest = serde_json::from_slice(&body_bytes).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "parse_error", "message": e.to_string() })),
        )
    })?;

    let response = handle_mcp_request(&conn, &token, rpc_req);

    match response {
        Some(resp) => Ok(Json(resp).into_response()),
        None => Ok((StatusCode::ACCEPTED, Json(json!({}))).into_response()),
    }
}
