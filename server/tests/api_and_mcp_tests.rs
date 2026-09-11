use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::broadcast;
use tower::ServiceExt;

use hermesgate::{
    api::{self, state::AppState},
    db::{self, models::*},
    mcp,
};

fn setup_test_app() -> (Router, String, String, String, String) {
    let pool = db::init_memory_pool();
    let conn = pool.get().unwrap();

    // 1. Create Admin Token
    let (_admin_token, raw_admin) = db::create_api_token(
        &conn,
        CreateTokenRequest {
            name: "Master Admin".to_string(),
            allowed_numbers: vec!["*".to_string()],
            can_read: true,
            can_delete: true,
            is_admin: true,
        },
    ).unwrap();

    // 2. Create Device & Token
    let (_device, raw_device_token) = db::register_device(
        &conn,
        "Pixel 8 Test",
        "android",
        None,
    ).unwrap();

    // 3. Create Token 1 (Allowed ONLY +15550000001)
    let (_tok1, raw_tok1) = db::create_api_token(
        &conn,
        CreateTokenRequest {
            name: "Agent 1".to_string(),
            allowed_numbers: vec!["+15550000001".to_string()],
            can_read: true,
            can_delete: false,
            is_admin: false,
        },
    ).unwrap();

    // 4. Create Token 2 (Allowed ONLY +15550000002)
    let (_tok2, raw_tok2) = db::create_api_token(
        &conn,
        CreateTokenRequest {
            name: "Agent 2".to_string(),
            allowed_numbers: vec!["+15550000002".to_string()],
            can_read: true,
            can_delete: false,
            is_admin: false,
        },
    ).unwrap();

    let (tx, _) = broadcast::channel(100);
    let state = AppState {
        pool,
        tx,
        server_url: Arc::new("http://localhost:8080".to_string()),
    };

    let router = Router::new()
        .nest("/api/v1", api::router())
        .nest("/mcp", mcp::router())
        .with_state(state);

    (router, raw_admin, raw_device_token, raw_tok1, raw_tok2)
}

#[tokio::test]
async fn test_sms_forward_and_query_isolation() {
    let (app, admin_token, device_token, tok1, tok2) = setup_test_app();

    // 1. Device forwards SMS for Number 1 (+15550000001)
    let req = Request::builder()
        .uri("/api/v1/sms/forward")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .header("X-Device-Token", &device_token)
        .body(Body::from(
            json!({
                "recipient_number": "+15550000001",
                "sender": "BankOne",
                "body": "Your login code is 829104",
                "sim_slot": 0
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // 2. Device forwards SMS for Number 2 (+15550000002)
    let req = Request::builder()
        .uri("/api/v1/sms/forward")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .header("X-Device-Token", &device_token)
        .body(Body::from(
            json!({
                "recipient_number": "+15550000002",
                "sender": "Google2FA",
                "body": "G-551923 is your verification code",
                "sim_slot": 1
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // 3. Agent 1 queries messages -> MUST ONLY see Number 1
    let req = Request::builder()
        .uri("/api/v1/messages")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", tok1))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    let body_json: Value = serde_json::from_slice(&body_bytes).unwrap();
    let messages = body_json["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["recipient_number"], "+15550000001");
    assert_eq!(messages[0]["extracted_code"], "829104");

    // 4. Agent 1 tries to access Number 2 -> MUST receive 403 Forbidden!
    let req = Request::builder()
        .uri("/api/v1/messages?number=%2B15550000002")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", tok1))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // 5. Agent 1 gets latest OTP -> gets 829104
    let req = Request::builder()
        .uri("/api/v1/messages/latest-otp")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", tok1))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    let body_json: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body_json["otp"]["code"], "829104");

    // 6. Agent 2 gets latest OTP -> gets G-551923
    let req = Request::builder()
        .uri("/api/v1/messages/latest-otp")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", tok2))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    let body_json: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body_json["otp"]["code"], "G-551923");

    // 7. Admin sees both numbers
    let req = Request::builder()
        .uri("/api/v1/messages")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    let body_json: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body_json["messages"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn test_mcp_json_rpc_tools_and_permissions() {
    let (app, _admin, device_token, tok1, _tok2) = setup_test_app();

    // 1. Ingest test SMS
    let req = Request::builder()
        .uri("/api/v1/sms/forward")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .header("X-Device-Token", &device_token)
        .body(Body::from(
            json!({
                "recipient_number": "+15550000001",
                "sender": "SecureCorp",
                "body": "Verification code: 994812",
                "sim_slot": 0
            })
            .to_string(),
        ))
        .unwrap();
    let _ = app.clone().oneshot(req).await.unwrap();

    // 2. MCP Initialize Request
    let req = Request::builder()
        .uri(format!("/mcp/messages?token={}", tok1))
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {}
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    let body_json: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body_json["result"]["protocolVersion"], "2024-11-05");

    // 3. MCP tools/list
    let req = Request::builder()
        .uri(format!("/mcp/messages?token={}", tok1))
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/list",
                "params": {}
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    let body_json: Value = serde_json::from_slice(&body_bytes).unwrap();
    let tools = body_json["result"]["tools"].as_array().unwrap();
    assert!(tools.iter().any(|t| t["name"] == "get_latest_otp"));
    assert!(tools.iter().any(|t| t["name"] == "list_allowed_numbers"));

    // 4. MCP call get_latest_otp for permitted number
    let req = Request::builder()
        .uri(format!("/mcp/messages?token={}", tok1))
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {
                    "name": "get_latest_otp",
                    "arguments": {
                        "phone_number": "+15550000001"
                    }
                }
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    let body_json: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body_json["result"]["isError"], false);
    let text = body_json["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("994812"));

    // 5. MCP call get_latest_otp for UNAUTHORIZED number (+15550000002) -> MUST BE DENIED!
    let req = Request::builder()
        .uri(format!("/mcp/messages?token={}", tok1))
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "tools/call",
                "params": {
                    "name": "get_latest_otp",
                    "arguments": {
                        "phone_number": "+15550000002"
                    }
                }
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    let body_json: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body_json["result"]["isError"], true);
    let text = body_json["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Permission Denied"));
}
