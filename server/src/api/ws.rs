use axum::{
    extract::{
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;

use tracing::{debug, info};

use crate::{
    api::state::AppState,
    db::{models::ApiToken, verify_api_token},
};

#[derive(Deserialize)]
pub struct WsQuery {
    pub token: Option<String>,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(query): Query<WsQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let raw_token = query.token.ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "unauthorized", "message": "Missing ?token= query parameter" })),
        )
    })?;

    let conn = state.pool.get().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "db_error", "message": e.to_string() })),
        )
    })?;

    let token = verify_api_token(&conn, &raw_token)
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

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, token)))
}

async fn handle_socket(socket: WebSocket, state: AppState, token: ApiToken) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.tx.subscribe();

    info!("🔌 WebSocket connected for client: {} (Admin: {})", token.name, token.is_admin);

    // Send connected welcome message
    let welcome = json!({
        "type": "connected",
        "message": "Connected to real-time SMS stream",
        "token_name": token.name,
        "is_admin": token.is_admin,
    });
    if sender.send(WsMessage::Text(welcome.to_string())).await.is_err() {
        return;
    }

    // Task 1: Forward incoming messages to the websocket client (subject to per-number permissions)
    let send_task = async move {
        while let Ok(msg) = rx.recv().await {
            // Check if this token is allowed to see this number!
            if token.can_access_number(&msg.recipient_number) {
                let event = json!({
                    "type": "new_sms",
                    "message": *msg,
                });
                if sender.send(WsMessage::Text(event.to_string())).await.is_err() {
                    break;
                }
            }
        }
    };

    // Task 2: Keep alive / listen to client pings
    let recv_task = async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                WsMessage::Ping(_) => {
                    // Axum automatically replies to standard Ping with Pong
                }
                WsMessage::Text(t) => {
                    debug!("Received WS text from client: {}", t);
                }
                WsMessage::Close(_) => {
                    break;
                }
                _ => {}
            }
        }
    };

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    info!("🔌 WebSocket disconnected");
}
