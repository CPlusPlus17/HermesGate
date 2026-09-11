use regex::Regex;
use std::sync::OnceLock;

/// Extracts potential 2FA / OTP verification codes from an SMS message body.
/// Returns the first high-confidence matching code.
pub fn extract_otp(body: &str) -> Option<String> {
    static REGEX_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    let patterns = REGEX_PATTERNS.get_or_init(|| {
        vec![
            // Explicit prefix: "code is 123456", "code: 123456", "code 123-456", "pin is 1234"
            Regex::new(r"(?i)(?:code|verification\s+code|security\s+code|pin|otp|passcode|secret)[\s:=#\-is]+(?:is\s+)?([0-9]{4,8})\b").unwrap(),
            // Hyphenated codes: "123-456"
            Regex::new(r"(?i)(?:code|pin|otp|verification)[\s:=#\-]+([0-9]{3}-[0-9]{3})\b").unwrap(),
            // Google/platform style: "G-123456"
            Regex::new(r"\b([A-Z]-[0-9]{4,8})\b").unwrap(),
            // "Use 123456 to verify" or "enter 123456 to"
            Regex::new(r"(?i)(?:use|enter|type|input)\s+([0-9]{4,8})\s+(?:to|as|for|in)\b").unwrap(),
            // "123456 is your (verification/login) code"
            Regex::new(r"(?i)\b([0-9]{4,8})\s+is\s+your\b").unwrap(),
            // Standalone 4 to 8 digit number surrounded by keywords
            Regex::new(r"(?i)(?:verify|authenticate|login|account|confirmation).{0,30}?\b([0-9]{4,8})\b").unwrap(),
            // Fallback: standalone 6 digit code in a message containing verification keywords
            Regex::new(r"(?i)(?:code|verif|confirm|passcode|token).{0,40}?\b([0-9]{6})\b").unwrap(),
        ]
    });

    for regex in patterns {
        if let Some(captures) = regex.captures(body) {
            if let Some(matched) = captures.get(1) {
                let code = matched.as_str().trim();
                // Ensure code is not a year (like 2024, 2026) unless explicitly captured
                if code.len() == 4 && (code.starts_with("19") || code.starts_with("20")) {
                    // Check if it's explicitly preceded by "code is" or similar
                    if !body.to_lowercase().contains("code") && !body.to_lowercase().contains("pin") {
                        continue;
                    }
                }
                return Some(code.to_string());
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_otp_formats() {
        assert_eq!(
            extract_otp("Your verification code is 492810. Valid for 5 minutes."),
            Some("492810".to_string())
        );
        assert_eq!(
            extract_otp("Use 582914 to verify your Instagram account."),
            Some("582914".to_string())
        );
        assert_eq!(
            extract_otp("Google verification code: G-739102. Do not share."),
            Some("G-739102".to_string())
        );
        assert_eq!(
            extract_otp("839201 is your WhatsApp code."),
            Some("839201".to_string())
        );
        assert_eq!(
            extract_otp("Apple ID Code: 381920. Don't share it with anyone."),
            Some("381920".to_string())
        );
        assert_eq!(
            extract_otp("Your security PIN is 9182"),
            Some("9182".to_string())
        );
        assert_eq!(
            extract_otp("Hello! Your login code: 938-120"),
            Some("938-120".to_string())
        );
    }
}
