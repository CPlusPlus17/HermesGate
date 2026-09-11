use rusqlite::Connection;
use serde_json::{json, Value};
use tracing::info;

use crate::{
    db::{self, models::*},
    mcp::protocol::*,
};

pub fn get_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "list_allowed_numbers".to_string(),
            description: "Lists the phone numbers this AI assistant is authorized to access, along with labels and received message counts.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "get_latest_otp".to_string(),
            description: "Retrieves the most recent One-Time Password (OTP), 2FA verification code, or login PIN received on an authorized phone number within a given time window.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "phone_number": {
                        "type": "string",
                        "description": "Optional phone number (e.g. +15551234567). If omitted and you have access to only one number, it will be automatically selected."
                    },
                    "max_age_minutes": {
                        "type": "integer",
                        "description": "Maximum age of the SMS message in minutes. Defaults to 10 minutes."
                    }
                },
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "get_latest_messages".to_string(),
            description: "Retrieves the most recent incoming SMS messages for authorized phone numbers.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "phone_number": {
                        "type": "string",
                        "description": "Optional phone number filter. Must be an authorized number."
                    },
                    "sender": {
                        "type": "string",
                        "description": "Optional filter by sender name or number (e.g. 'Google', 'WhatsApp', '+1800...')."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of messages to return (default 10, max 100)."
                    }
                },
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "search_messages".to_string(),
            description: "Searches through received SMS messages by keyword in the message body or sender.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search keyword or text to find in SMS messages."
                    },
                    "phone_number": {
                        "type": "string",
                        "description": "Optional phone number filter (must be authorized)."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of messages to return (default 10)."
                    }
                },
                "required": ["query"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "get_message_details".to_string(),
            description: "Fetches full details of a specific SMS message by its unique message ID.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "message_id": {
                        "type": "string",
                        "description": "The unique message UUID."
                    }
                },
                "required": ["message_id"],
                "additionalProperties": false
            }),
        },
    ]
}

pub fn execute_tool(
    conn: &Connection,
    token: &ApiToken,
    name: &str,
    args: Value,
) -> CallToolResult {
    info!("🤖 MCP Tool Call: {} by token '{}'", name, token.name);

    match name {
        "list_allowed_numbers" => {
            match db::list_phone_numbers(conn, token) {
                Ok(numbers) => {
                    let formatted = json!({
                        "count": numbers.len(),
                        "allowed_numbers": numbers,
                    });
                    CallToolResult::text(serde_json::to_string_pretty(&formatted).unwrap())
                }
                Err(e) => CallToolResult::error(format!("Database error: {}", e)),
            }
        }
        "get_latest_otp" => {
            let phone_number = args.get("phone_number").and_then(|v| v.as_str());
            let max_age = args
                .get("max_age_minutes")
                .and_then(|v| v.as_i64())
                .unwrap_or(10);

            // If phone_number is specified, check permission
            if let Some(num) = phone_number {
                if !token.can_access_number(num) {
                    return CallToolResult::error(format!(
                        "Permission Denied: Token '{}' is not authorized to access phone number {}.",
                        token.name, num
                    ));
                }
            }

            match db::get_latest_otp(conn, phone_number, token, max_age) {
                Ok(Some(otp)) => {
                    let result = json!({
                        "found": true,
                        "otp_code": otp.code,
                        "recipient_number": otp.recipient_number,
                        "sender": otp.sender,
                        "received_at": otp.received_at,
                        "full_message": otp.body,
                    });
                    CallToolResult::text(serde_json::to_string_pretty(&result).unwrap())
                }
                Ok(None) => {
                    CallToolResult::text(json!({
                        "found": false,
                        "message": format!("No OTP verification code found within the last {} minutes for authorized numbers.", max_age)
                    }).to_string())
                }
                Err(e) => CallToolResult::error(format!("Database error: {}", e)),
            }
        }
        "get_latest_messages" => {
            let phone_number = args.get("phone_number").and_then(|v| v.as_str()).map(|s| s.to_string());
            let sender = args.get("sender").and_then(|v| v.as_str()).map(|s| s.to_string());
            let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(10);

            if let Some(ref num) = phone_number {
                if !token.can_access_number(num) {
                    return CallToolResult::error(format!(
                        "Permission Denied: Token '{}' is not authorized to access phone number {}.",
                        token.name, num
                    ));
                }
            }

            let filter = MessageQueryFilter {
                number: phone_number,
                sender,
                search: None,
                limit: Some(limit),
                offset: None,
                since: None,
            };

            match db::query_messages(conn, &filter, token) {
                Ok(messages) => {
                    let count = messages.len();
                    let result = json!({
                        "count": count,
                        "messages": messages,
                    });
                    CallToolResult::text(serde_json::to_string_pretty(&result).unwrap())
                }
                Err(e) => CallToolResult::error(format!("Database error: {}", e)),
            }
        }
        "search_messages" => {
            let query = match args.get("query").and_then(|v| v.as_str()) {
                Some(q) => q.to_string(),
                None => return CallToolResult::error("Missing required parameter 'query'"),
            };
            let phone_number = args.get("phone_number").and_then(|v| v.as_str()).map(|s| s.to_string());
            let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(10);

            if let Some(ref num) = phone_number {
                if !token.can_access_number(num) {
                    return CallToolResult::error(format!(
                        "Permission Denied: Token '{}' is not authorized to access phone number {}.",
                        token.name, num
                    ));
                }
            }

            let filter = MessageQueryFilter {
                number: phone_number,
                sender: None,
                search: Some(query),
                limit: Some(limit),
                offset: None,
                since: None,
            };

            match db::query_messages(conn, &filter, token) {
                Ok(messages) => {
                    let count = messages.len();
                    let result = json!({
                        "count": count,
                        "messages": messages,
                    });
                    CallToolResult::text(serde_json::to_string_pretty(&result).unwrap())
                }
                Err(e) => CallToolResult::error(format!("Database error: {}", e)),
            }
        }
        "get_message_details" => {
            let message_id = match args.get("message_id").and_then(|v| v.as_str()) {
                Some(id) => id,
                None => return CallToolResult::error("Missing required parameter 'message_id'"),
            };

            match db::get_message_by_id(conn, message_id, token) {
                Ok(Some(msg)) => {
                    CallToolResult::text(serde_json::to_string_pretty(&msg).unwrap())
                }
                Ok(None) => {
                    CallToolResult::error(format!("Message '{}' not found or you are not authorized to view it.", message_id))
                }
                Err(e) => CallToolResult::error(format!("Database error: {}", e)),
            }
        }
        _ => CallToolResult::error(format!("Unknown tool: {}", name)),
    }
}
