pub mod handler;
pub mod protocol;
pub mod resources;
pub mod sse;
pub mod stdio;
pub mod tools;

use axum::{
    routing::{get, post},
    Router,
};

use crate::api::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/sse", get(sse::sse_connect))
        .route("/messages", post(sse::receive_message))
}
