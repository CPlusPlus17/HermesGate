use rusqlite::Connection;


use crate::{
    db::{self, models::*},
    mcp::protocol::*,
};

pub fn get_resource_definitions(conn: &Connection, token: &ApiToken) -> Vec<ResourceDefinition> {
    let mut resources = vec![ResourceDefinition {
        uri: "sms://numbers".to_string(),
        name: "Authorized Phone Numbers".to_string(),
        description: Some("List of phone numbers authorized for this assistant".to_string()),
        mime_type: Some("application/json".to_string()),
    }];

    if let Ok(numbers) = db::list_phone_numbers(conn, token) {
        for num in numbers {
            resources.push(ResourceDefinition {
                uri: format!("sms://messages/{}", num.phone_number),
                name: format!("SMS Feed: {}", num.label.as_deref().unwrap_or(&num.phone_number)),
                description: Some(format!("Recent SMS received on {}", num.phone_number)),
                mime_type: Some("application/json".to_string()),
            });
        }
    }

    resources
}

pub fn read_resource(
    conn: &Connection,
    token: &ApiToken,
    uri: &str,
) -> Result<ResourceContent, String> {
    if uri == "sms://numbers" {
        let numbers = db::list_phone_numbers(conn, token).map_err(|e| e.to_string())?;
        return Ok(ResourceContent {
            uri: uri.to_string(),
            mime_type: Some("application/json".to_string()),
            text: serde_json::to_string_pretty(&numbers).unwrap(),
        });
    }

    if let Some(number) = uri.strip_prefix("sms://messages/") {
        if !token.can_access_number(number) {
            return Err(format!("Permission Denied: Unauthorized for number {}", number));
        }

        let filter = MessageQueryFilter {
            number: Some(number.to_string()),
            limit: Some(20),
            ..Default::default()
        };

        let messages = db::query_messages(conn, &filter, token).map_err(|e| e.to_string())?;
        return Ok(ResourceContent {
            uri: uri.to_string(),
            mime_type: Some("application/json".to_string()),
            text: serde_json::to_string_pretty(&messages).unwrap(),
        });
    }

    Err(format!("Resource not found: {}", uri))
}
