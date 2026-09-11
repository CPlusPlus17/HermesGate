use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::{request::Parts, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::db::{models::*, DbPool};

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthErrorResponse {
    pub error: String,
    pub message: String,
}

#[derive(Debug)]
pub enum AuthError {
    MissingToken,
    InvalidToken,
    Forbidden(String),
    DatabaseError(String),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, err_type, msg) = match self {
            AuthError::MissingToken => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "Missing Authorization header (Bearer token) or ?token= query parameter".to_string(),
            ),
            AuthError::InvalidToken => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "Invalid or expired token".to_string(),
            ),
            AuthError::Forbidden(ref reason) => (
                StatusCode::FORBIDDEN,
                "forbidden",
                reason.clone(),
            ),
            AuthError::DatabaseError(ref reason) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                reason.clone(),
            ),
        };

        let body = Json(AuthErrorResponse {
            error: err_type.to_string(),
            message: msg,
        });

        (status, body).into_response()
    }
}

pub fn extract_token_from_headers(headers: &HeaderMap) -> Option<String> {
    if let Some(auth_header) = headers.get(axum::http::header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                return Some(token.trim().to_string());
            }
        }
    }

    if let Some(tok_header) = headers.get("x-api-token") {
        if let Ok(tok_str) = tok_header.to_str() {
            return Some(tok_str.trim().to_string());
        }
    }

    if let Some(tok_header) = headers.get("x-device-token") {
        if let Ok(tok_str) = tok_header.to_str() {
            return Some(tok_str.trim().to_string());
        }
    }

    None
}

/// Extracts Bearer token or ?token= query from request parts
pub fn extract_token_str(parts: &Parts) -> Option<String> {
    if let Some(tok) = extract_token_from_headers(&parts.headers) {
        return Some(tok);
    }

    // Query param ?token=...
    if let Some(query) = parts.uri.query() {
        if let Ok(params) = serde_urlencoded::from_str::<HashMap<String, String>>(query) {
            if let Some(token) = params.get("token") {
                return Some(token.clone());
            }
        }
    }

    None
}

/// Extractor for any valid API token (MCP or REST)
pub struct AuthenticatedToken(pub ApiToken);

#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedToken
where
    DbPool: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let pool = DbPool::from_ref(state);
        let token_str = extract_token_str(parts).ok_or(AuthError::MissingToken)?;

        let conn = pool.get().map_err(|e| AuthError::DatabaseError(e.to_string()))?;
        let token = crate::db::verify_api_token(&conn, &token_str)
            .map_err(|e| AuthError::DatabaseError(e.to_string()))?
            .ok_or(AuthError::InvalidToken)?;

        if !token.can_read && !token.is_admin {
            return Err(AuthError::Forbidden("Token lacks read permission".to_string()));
        }

        Ok(AuthenticatedToken(token))
    }
}

/// Extractor for Admin-only operations
pub struct AdminToken(pub ApiToken);

#[async_trait]
impl<S> FromRequestParts<S> for AdminToken
where
    DbPool: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let AuthenticatedToken(token) = AuthenticatedToken::from_request_parts(parts, state).await?;
        if !token.is_admin {
            return Err(AuthError::Forbidden("Administrator privileges required".to_string()));
        }
        Ok(AdminToken(token))
    }
}

/// Extractor for mobile device forwarding
pub struct DeviceOrAdmin {
    pub device: Option<Device>,
    pub admin: Option<ApiToken>,
}

#[async_trait]
impl<S> FromRequestParts<S> for DeviceOrAdmin
where
    DbPool: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let pool = DbPool::from_ref(state);
        let token_str = extract_token_str(parts).ok_or(AuthError::MissingToken)?;

        let conn = pool.get().map_err(|e| AuthError::DatabaseError(e.to_string()))?;

        // Check if it's a valid device token
        if let Ok(Some(device)) = crate::db::verify_device_token(&conn, &token_str) {
            return Ok(DeviceOrAdmin {
                device: Some(device),
                admin: None,
            });
        }

        // Check if it's a valid API/Admin token
        if let Ok(Some(token)) = crate::db::verify_api_token(&conn, &token_str) {
            if token.is_admin {
                return Ok(DeviceOrAdmin {
                    device: None,
                    admin: Some(token),
                });
            }
        }

        Err(AuthError::InvalidToken)
    }
}
