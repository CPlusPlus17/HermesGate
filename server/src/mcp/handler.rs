use rusqlite::Connection;
use serde_json::{json, Value};

use crate::{
    db::models::ApiToken,
    mcp::{protocol::*, resources, tools},
};

pub fn handle_mcp_request(
    conn: &Connection,
    token: &ApiToken,
    req: JsonRpcRequest,
) -> Option<JsonRpcResponse> {
    let id = req.id;

    match req.method.as_str() {
        "initialize" => {
            let result = InitializeResult {
                protocol_version: "2024-11-05".to_string(),
                capabilities: ServerCapabilities {
                    tools: Some(json!({
                        "listChanged": false
                    })),
                    resources: Some(json!({
                        "subscribe": false,
                        "listChanged": false
                    })),
                },
                server_info: ServerInfo {
                    name: "hermesgate-mcp".to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                },
            };
            Some(JsonRpcResponse::success(id, serde_json::to_value(result).unwrap()))
        }
        "notifications/initialized" => {
            // Client acknowledgment, no response needed
            None
        }
        "ping" => {
            Some(JsonRpcResponse::success(id, json!({})))
        }
        "tools/list" => {
            let tools_list = tools::get_tool_definitions();
            Some(JsonRpcResponse::success(id, json!({ "tools": tools_list })))
        }
        "tools/call" => {
            let params = req.params.unwrap_or(Value::Null);
            let tool_name = match params.get("name").and_then(|v| v.as_str()) {
                Some(name) => name,
                None => {
                    return Some(JsonRpcResponse::error(
                        id,
                        -32602,
                        "Missing tool name".to_string(),
                        None,
                    ));
                }
            };
            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
            let call_result = tools::execute_tool(conn, token, tool_name, arguments);
            Some(JsonRpcResponse::success(
                id,
                serde_json::to_value(call_result).unwrap(),
            ))
        }
        "resources/list" => {
            let resources_list = resources::get_resource_definitions(conn, token);
            Some(JsonRpcResponse::success(id, json!({ "resources": resources_list })))
        }
        "resources/read" => {
            let params = req.params.unwrap_or(Value::Null);
            let uri = match params.get("uri").and_then(|v| v.as_str()) {
                Some(u) => u,
                None => {
                    return Some(JsonRpcResponse::error(
                        id,
                        -32602,
                        "Missing resource uri".to_string(),
                        None,
                    ));
                }
            };

            match resources::read_resource(conn, token, uri) {
                Ok(content) => {
                    Some(JsonRpcResponse::success(id, json!({ "contents": [content] })))
                }
                Err(err_msg) => {
                    Some(JsonRpcResponse::error(id, -32000, err_msg, None))
                }
            }
        }
        _ => Some(JsonRpcResponse::error(
            id,
            -32601,
            format!("Method not found: {}", req.method),
            None,
        )),
    }
}
