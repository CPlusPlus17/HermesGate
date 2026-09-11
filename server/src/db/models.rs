use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoneNumber {
    pub phone_number: String,
    pub label: Option<String>,
    pub created_at: String,
    pub last_seen_at: String,
    pub device_id: Option<String>,
    pub is_active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub platform: String,
    #[serde(skip_serializing)]
    pub token_hash: String,
    pub created_at: String,
    pub last_seen_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub recipient_number: String,
    pub sender: String,
    pub body: String,
    pub received_at: String,
    pub device_id: Option<String>,
    pub sim_slot: Option<i32>,
    pub extracted_code: Option<String>,
    pub raw_metadata: Option<String>,
    pub is_read: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiToken {
    pub id: String,
    #[serde(skip_serializing)]
    pub token_hash: String,
    pub token_prefix: String,
    pub name: String,
    pub allowed_numbers: Vec<String>,
    pub can_read: bool,
    pub can_delete: bool,
    pub is_admin: bool,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

impl ApiToken {
    /// Checks if this token is authorized to access a given phone number.
    pub fn can_access_number(&self, number: &str) -> bool {
        if self.is_admin {
            return true;
        }
        for allowed in &self.allowed_numbers {
            if allowed == "*" || allowed.eq_ignore_ascii_case("ALL") {
                return true;
            }
            // Normalize spaces/dashes if any
            let clean_allowed = allowed.replace([' ', '-', '(', ')'], "");
            let clean_target = number.replace([' ', '-', '(', ')'], "");
            if clean_allowed == clean_target {
                return true;
            }
        }
        false
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtpResult {
    pub code: String,
    pub message_id: String,
    pub recipient_number: String,
    pub sender: String,
    pub body: String,
    pub received_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestSmsRequest {
    pub recipient_number: String,
    pub sender: String,
    pub body: String,
    pub received_at: Option<String>,
    pub device_id: Option<String>,
    pub sim_slot: Option<i32>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTokenRequest {
    pub name: String,
    pub allowed_numbers: Vec<String>,
    #[serde(default = "default_true")]
    pub can_read: bool,
    #[serde(default)]
    pub can_delete: bool,
    #[serde(default)]
    pub is_admin: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTokenResponse {
    pub token: ApiToken,
    pub secret: String, // Full plaintext token returned only on creation
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDeviceRequest {
    pub name: String,
    pub platform: String, // "android", "ios", "api"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDeviceResponse {
    pub device: Device,
    pub token: String, // Full plaintext device token returned only on creation
    pub qr_data: String, // Formatted JSON string ready for mobile QR scan
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessageQueryFilter {
    pub number: Option<String>,
    pub sender: Option<String>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub since: Option<String>,
}
