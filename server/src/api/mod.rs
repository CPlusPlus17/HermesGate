pub mod devices;
pub mod health;
pub mod numbers;
pub mod sms;
pub mod state;
pub mod tokens;
pub mod ws;

use axum::{
    routing::{delete, get, post},
    Router,
};

use self::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        // Health
        .route("/health", get(health::health_check))
        // SMS Ingestion & Querying
        .route("/sms/forward", post(sms::forward_sms))
        .route("/messages", get(sms::list_messages))
        .route("/messages/latest-otp", get(sms::get_latest_otp))
        .route("/messages/:id", get(sms::get_message).delete(sms::delete_message))
        // Phone Numbers
        .route("/numbers", get(numbers::list_numbers))
        // Admin Management
        .route("/admin/numbers", post(numbers::upsert_number))
        .route("/admin/tokens", get(tokens::list_tokens).post(tokens::create_token))
        .route("/admin/tokens/:id", delete(tokens::delete_token))
        .route("/admin/devices", get(devices::list_devices).post(devices::create_device))
        // Real-time WebSocket
        .route("/ws", get(ws::ws_handler))
}
