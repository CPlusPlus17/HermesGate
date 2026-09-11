use axum::extract::FromRef;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::db::{models::Message, DbPool};

#[derive(Clone)]
pub struct AppState {
    pub pool: DbPool,
    pub tx: broadcast::Sender<Arc<Message>>,
    pub server_url: Arc<String>,
}

impl FromRef<AppState> for DbPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}
