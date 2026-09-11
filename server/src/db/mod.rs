pub mod models;

use chrono::Utc;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rand::Rng;
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use std::path::Path;
use tracing::info;

use self::models::*;

pub type DbPool = Pool<SqliteConnectionManager>;

pub fn init_pool<P: AsRef<Path>>(db_path: P) -> Result<DbPool, Box<dyn std::error::Error>> {
    let manager = SqliteConnectionManager::file(db_path)
        .with_init(|c| {
            c.execute_batch("
                PRAGMA journal_mode = WAL;
                PRAGMA synchronous = NORMAL;
                PRAGMA foreign_keys = ON;
            ")
        });
    let pool = Pool::builder().max_size(16).build(manager)?;

    // Run migrations
    let conn = pool.get()?;
    run_migrations(&conn)?;

    Ok(pool)
}

/// Create in-memory pool for tests
pub fn init_memory_pool() -> DbPool {
    let manager = SqliteConnectionManager::memory();
    let pool = Pool::builder().max_size(1).build(manager).unwrap();
    let conn = pool.get().unwrap();
    run_migrations(&conn).unwrap();
    pool
}

fn run_migrations(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS phone_numbers (
            phone_number TEXT PRIMARY KEY,
            label TEXT,
            created_at TEXT NOT NULL,
            last_seen_at TEXT NOT NULL,
            device_id TEXT,
            is_active INTEGER NOT NULL DEFAULT 1
        );

        CREATE TABLE IF NOT EXISTS devices (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            platform TEXT NOT NULL,
            token_hash TEXT UNIQUE NOT NULL,
            created_at TEXT NOT NULL,
            last_seen_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            recipient_number TEXT NOT NULL,
            sender TEXT NOT NULL,
            body TEXT NOT NULL,
            received_at TEXT NOT NULL,
            device_id TEXT,
            sim_slot INTEGER,
            extracted_code TEXT,
            raw_metadata TEXT,
            is_read INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (recipient_number) REFERENCES phone_numbers(phone_number) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_messages_recipient ON messages(recipient_number);
        CREATE INDEX IF NOT EXISTS idx_messages_received ON messages(received_at DESC);
        CREATE INDEX IF NOT EXISTS idx_messages_sender ON messages(sender);
        CREATE INDEX IF NOT EXISTS idx_messages_code ON messages(extracted_code);

        CREATE TABLE IF NOT EXISTS api_tokens (
            id TEXT PRIMARY KEY,
            token_hash TEXT UNIQUE NOT NULL,
            token_prefix TEXT NOT NULL,
            name TEXT NOT NULL,
            allowed_numbers TEXT NOT NULL, -- JSON array of phone numbers or ['*']
            can_read INTEGER NOT NULL DEFAULT 1,
            can_delete INTEGER NOT NULL DEFAULT 0,
            is_admin INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            last_used_at TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_tokens_hash ON api_tokens(token_hash);
    ")?;
    Ok(())
}

pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

mod hex {
    pub fn encode<T: AsRef<[u8]>>(data: T) -> String {
        data.as_ref().iter().map(|b| format!("{:02x}", b)).collect()
    }
}

pub fn generate_token(prefix: &str) -> String {
    let mut rng = rand::thread_rng();
    let random_bytes: [u8; 24] = rng.gen();
    format!("{}_{}", prefix, hex::encode(random_bytes))
}

pub fn ensure_admin_token(pool: &DbPool, custom_admin_token: Option<&str>) -> Result<String, rusqlite::Error> {
    let conn = pool.get().map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    
    // Check if any admin token already exists
    let admin_count: i64 = conn.query_row(
        "SELECT count(*) FROM api_tokens WHERE is_admin = 1",
        [],
        |row| row.get(0),
    )?;

    if admin_count > 0 && custom_admin_token.is_none() {
        return Ok("(existing admin token active)".to_string());
    }

    let raw_token = if let Some(t) = custom_admin_token {
        t.to_string()
    } else {
        generate_token("sms_adm")
    };

    let token_hash = hash_token(&raw_token);
    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let prefix = if raw_token.len() >= 11 { &raw_token[..11] } else { "sms_adm" };

    conn.execute(
        "INSERT OR REPLACE INTO api_tokens (id, token_hash, token_prefix, name, allowed_numbers, can_read, can_delete, is_admin, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 1, 1, 1, ?6)",
        params![
            id,
            token_hash,
            format!("{}...", prefix),
            "System Admin",
            r#"["*"]"#,
            now,
        ],
    )?;

    info!("🔑 Initialized Master Admin Token: {}", raw_token);
    Ok(raw_token)
}

pub fn verify_api_token(conn: &Connection, raw_token: &str) -> Result<Option<ApiToken>, rusqlite::Error> {
    let token_hash = hash_token(raw_token);
    let now = Utc::now().to_rfc3339();

    // Update last_used_at
    conn.execute(
        "UPDATE api_tokens SET last_used_at = ?1 WHERE token_hash = ?2",
        params![now, token_hash],
    )?;

    let mut stmt = conn.prepare(
        "SELECT id, token_hash, token_prefix, name, allowed_numbers, can_read, can_delete, is_admin, created_at, last_used_at
         FROM api_tokens WHERE token_hash = ?1",
    )?;

    let token = stmt.query_row(params![token_hash], |row| {
        let allowed_str: String = row.get(4)?;
        let allowed_numbers: Vec<String> = serde_json::from_str(&allowed_str).unwrap_or_else(|_| vec![]);
        Ok(ApiToken {
            id: row.get(0)?,
            token_hash: row.get(1)?,
            token_prefix: row.get(2)?,
            name: row.get(3)?,
            allowed_numbers,
            can_read: row.get::<_, i32>(5)? != 0,
            can_delete: row.get::<_, i32>(6)? != 0,
            is_admin: row.get::<_, i32>(7)? != 0,
            created_at: row.get(8)?,
            last_used_at: row.get(9)?,
        })
    }).optional()?;

    Ok(token)
}

pub fn verify_device_token(conn: &Connection, raw_token: &str) -> Result<Option<Device>, rusqlite::Error> {
    let token_hash = hash_token(raw_token);
    let now = Utc::now().to_rfc3339();

    conn.execute(
        "UPDATE devices SET last_seen_at = ?1 WHERE token_hash = ?2",
        params![now, token_hash],
    )?;

    let mut stmt = conn.prepare(
        "SELECT id, name, platform, token_hash, created_at, last_seen_at FROM devices WHERE token_hash = ?1",
    )?;

    let device = stmt.query_row(params![token_hash], |row| {
        Ok(Device {
            id: row.get(0)?,
            name: row.get(1)?,
            platform: row.get(2)?,
            token_hash: row.get(3)?,
            created_at: row.get(4)?,
            last_seen_at: row.get(5)?,
        })
    }).optional()?;

    Ok(device)
}

pub fn register_device(
    conn: &Connection,
    name: &str,
    platform: &str,
    custom_token: Option<&str>,
) -> Result<(Device, String), rusqlite::Error> {
    let raw_token = custom_token.map(|s| s.to_string()).unwrap_or_else(|| generate_token("sms_dev"));
    let token_hash = hash_token(&raw_token);
    let id = format!("dev_{}", uuid::Uuid::new_v4().simple());
    let now = Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO devices (id, name, platform, token_hash, created_at, last_seen_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, name, platform, token_hash, now, now],
    )?;

    let device = Device {
        id,
        name: name.to_string(),
        platform: platform.to_string(),
        token_hash,
        created_at: now.clone(),
        last_seen_at: now,
    };

    Ok((device, raw_token))
}

pub fn list_devices(conn: &Connection) -> Result<Vec<Device>, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT id, name, platform, token_hash, created_at, last_seen_at FROM devices ORDER BY last_seen_at DESC")?;
    let rows = stmt.query_map([], |row| {
        Ok(Device {
            id: row.get(0)?,
            name: row.get(1)?,
            platform: row.get(2)?,
            token_hash: row.get(3)?,
            created_at: row.get(4)?,
            last_seen_at: row.get(5)?,
        })
    })?;

    let mut list = Vec::new();
    for r in rows {
        list.push(r?);
    }
    Ok(list)
}

pub fn create_api_token(
    conn: &Connection,
    req: CreateTokenRequest,
) -> Result<(ApiToken, String), rusqlite::Error> {
    let raw_token = if req.is_admin {
        generate_token("sms_adm")
    } else {
        generate_token("sms_tok")
    };
    let token_hash = hash_token(&raw_token);
    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let prefix = format!("{}...", &raw_token[..raw_token.len().min(11)]);
    let allowed_json = serde_json::to_string(&req.allowed_numbers).unwrap_or_else(|_| "[]".to_string());

    conn.execute(
        "INSERT INTO api_tokens (id, token_hash, token_prefix, name, allowed_numbers, can_read, can_delete, is_admin, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            id,
            token_hash,
            prefix,
            req.name,
            allowed_json,
            if req.can_read { 1 } else { 0 },
            if req.can_delete { 1 } else { 0 },
            if req.is_admin { 1 } else { 0 },
            now,
        ],
    )?;

    let token = ApiToken {
        id,
        token_hash,
        token_prefix: prefix,
        name: req.name,
        allowed_numbers: req.allowed_numbers,
        can_read: req.can_read,
        can_delete: req.can_delete,
        is_admin: req.is_admin,
        created_at: now,
        last_used_at: None,
    };

    Ok((token, raw_token))
}

pub fn list_api_tokens(conn: &Connection) -> Result<Vec<ApiToken>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, token_hash, token_prefix, name, allowed_numbers, can_read, can_delete, is_admin, created_at, last_used_at
         FROM api_tokens ORDER BY created_at DESC",
    )?;

    let rows = stmt.query_map([], |row| {
        let allowed_str: String = row.get(4)?;
        let allowed_numbers: Vec<String> = serde_json::from_str(&allowed_str).unwrap_or_else(|_| vec![]);
        Ok(ApiToken {
            id: row.get(0)?,
            token_hash: row.get(1)?,
            token_prefix: row.get(2)?,
            name: row.get(3)?,
            allowed_numbers,
            can_read: row.get::<_, i32>(5)? != 0,
            can_delete: row.get::<_, i32>(6)? != 0,
            is_admin: row.get::<_, i32>(7)? != 0,
            created_at: row.get(8)?,
            last_used_at: row.get(9)?,
        })
    })?;

    let mut list = Vec::new();
    for r in rows {
        list.push(r?);
    }
    Ok(list)
}

pub fn delete_api_token(conn: &Connection, id: &str) -> Result<bool, rusqlite::Error> {
    let rows = conn.execute("DELETE FROM api_tokens WHERE id = ?1", params![id])?;
    Ok(rows > 0)
}

pub fn upsert_phone_number(
    conn: &Connection,
    phone_number: &str,
    label: Option<&str>,
    device_id: Option<&str>,
) -> Result<PhoneNumber, rusqlite::Error> {
    let now = Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO phone_numbers (phone_number, label, created_at, last_seen_at, device_id, is_active)
         VALUES (?1, ?2, ?3, ?4, ?5, 1)
         ON CONFLICT(phone_number) DO UPDATE SET
            last_seen_at = excluded.last_seen_at,
            label = COALESCE(excluded.label, phone_numbers.label),
            device_id = COALESCE(excluded.device_id, phone_numbers.device_id),
            is_active = 1",
        params![phone_number, label, now, now, device_id],
    )?;

    let mut stmt = conn.prepare("SELECT phone_number, label, created_at, last_seen_at, device_id, is_active FROM phone_numbers WHERE phone_number = ?1")?;
    let number = stmt.query_row(params![phone_number], |row| {
        Ok(PhoneNumber {
            phone_number: row.get(0)?,
            label: row.get(1)?,
            created_at: row.get(2)?,
            last_seen_at: row.get(3)?,
            device_id: row.get(4)?,
            is_active: row.get::<_, i32>(5)? != 0,
            message_count: None,
        })
    })?;

    Ok(number)
}

pub fn list_phone_numbers(
    conn: &Connection,
    token: &ApiToken,
) -> Result<Vec<PhoneNumber>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT p.phone_number, p.label, p.created_at, p.last_seen_at, p.device_id, p.is_active,
                (SELECT COUNT(*) FROM messages m WHERE m.recipient_number = p.phone_number) AS msg_count
         FROM phone_numbers p
         ORDER BY p.last_seen_at DESC",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(PhoneNumber {
            phone_number: row.get(0)?,
            label: row.get(1)?,
            created_at: row.get(2)?,
            last_seen_at: row.get(3)?,
            device_id: row.get(4)?,
            is_active: row.get::<_, i32>(5)? != 0,
            message_count: Some(row.get(6)?),
        })
    })?;

    let mut list = Vec::new();
    for r in rows {
        let num = r?;
        // Filter by token permissions!
        if token.can_access_number(&num.phone_number) {
            list.push(num);
        }
    }
    Ok(list)
}

pub fn insert_message(
    conn: &Connection,
    req: IngestSmsRequest,
    extracted_code: Option<String>,
) -> Result<Message, rusqlite::Error> {
    // Ensure phone number exists in phone_numbers table
    upsert_phone_number(conn, &req.recipient_number, None, req.device_id.as_deref())?;

    let id = uuid::Uuid::new_v4().to_string();
    let received_at = req.received_at.unwrap_or_else(|| Utc::now().to_rfc3339());
    let raw_meta = req.metadata.map(|v| v.to_string());

    conn.execute(
        "INSERT INTO messages (id, recipient_number, sender, body, received_at, device_id, sim_slot, extracted_code, raw_metadata, is_read)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0)",
        params![
            id,
            req.recipient_number,
            req.sender,
            req.body,
            received_at,
            req.device_id,
            req.sim_slot,
            extracted_code,
            raw_meta,
        ],
    )?;

    let msg = Message {
        id,
        recipient_number: req.recipient_number,
        sender: req.sender,
        body: req.body,
        received_at,
        device_id: req.device_id,
        sim_slot: req.sim_slot,
        extracted_code,
        raw_metadata: raw_meta,
        is_read: false,
    };

    Ok(msg)
}

pub fn query_messages(
    conn: &Connection,
    filter: &MessageQueryFilter,
    token: &ApiToken,
) -> Result<Vec<Message>, rusqlite::Error> {
    // If specific number requested, ensure authorization
    if let Some(ref num) = filter.number {
        if !token.can_access_number(num) {
            return Ok(Vec::new()); // Unauthorized for this specific number
        }
    }

    let mut sql = String::from(
        "SELECT id, recipient_number, sender, body, received_at, device_id, sim_slot, extracted_code, raw_metadata, is_read
         FROM messages WHERE 1=1"
    );
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    // 1. Phone number permission restriction
    if !token.is_admin && !token.allowed_numbers.iter().any(|n| n == "*" || n.eq_ignore_ascii_case("ALL")) {
        if token.allowed_numbers.is_empty() {
            return Ok(Vec::new()); // No numbers allowed
        }
        let placeholders: Vec<String> = (0..token.allowed_numbers.len()).map(|_| "?".to_string()).collect();
        sql.push_str(&format!(" AND recipient_number IN ({})", placeholders.join(",")));
        for num in &token.allowed_numbers {
            params_vec.push(Box::new(num.clone()));
        }
    }

    // 2. Query filters
    if let Some(ref num) = filter.number {
        sql.push_str(" AND recipient_number = ?");
        params_vec.push(Box::new(num.clone()));
    }

    if let Some(ref snd) = filter.sender {
        sql.push_str(" AND sender LIKE ?");
        params_vec.push(Box::new(format!("%{}%", snd)));
    }

    if let Some(ref s) = filter.search {
        sql.push_str(" AND (body LIKE ? OR sender LIKE ?)");
        params_vec.push(Box::new(format!("%{}%", s)));
        params_vec.push(Box::new(format!("%{}%", s)));
    }

    if let Some(ref since) = filter.since {
        sql.push_str(" AND received_at >= ?");
        params_vec.push(Box::new(since.clone()));
    }

    sql.push_str(" ORDER BY received_at DESC");

    let limit = filter.limit.unwrap_or(50).clamp(1, 500);
    sql.push_str(" LIMIT ?");
    params_vec.push(Box::new(limit));

    if let Some(offset) = filter.offset {
        sql.push_str(" OFFSET ?");
        params_vec.push(Box::new(offset));
    }

    let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(&params_refs[..], |row| {
        Ok(Message {
            id: row.get(0)?,
            recipient_number: row.get(1)?,
            sender: row.get(2)?,
            body: row.get(3)?,
            received_at: row.get(4)?,
            device_id: row.get(5)?,
            sim_slot: row.get(6)?,
            extracted_code: row.get(7)?,
            raw_metadata: row.get(8)?,
            is_read: row.get::<_, i32>(9)? != 0,
        })
    })?;

    let mut list = Vec::new();
    for r in rows {
        list.push(r?);
    }
    Ok(list)
}

pub fn get_latest_otp(
    conn: &Connection,
    requested_number: Option<&str>,
    token: &ApiToken,
    max_age_minutes: i64,
) -> Result<Option<OtpResult>, rusqlite::Error> {
    if let Some(num) = requested_number {
        if !token.can_access_number(num) {
            return Ok(None);
        }
    }

    let cutoff = (Utc::now() - chrono::Duration::minutes(max_age_minutes)).to_rfc3339();

    let mut sql = String::from(
        "SELECT id, recipient_number, sender, body, received_at, extracted_code
         FROM messages
         WHERE extracted_code IS NOT NULL AND received_at >= ?"
    );
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(cutoff)];

    if !token.is_admin && !token.allowed_numbers.iter().any(|n| n == "*" || n.eq_ignore_ascii_case("ALL")) {
        if token.allowed_numbers.is_empty() {
            return Ok(None);
        }
        let placeholders: Vec<String> = (0..token.allowed_numbers.len()).map(|_| "?".to_string()).collect();
        sql.push_str(&format!(" AND recipient_number IN ({})", placeholders.join(",")));
        for num in &token.allowed_numbers {
            params_vec.push(Box::new(num.clone()));
        }
    }

    if let Some(num) = requested_number {
        sql.push_str(" AND recipient_number = ?");
        params_vec.push(Box::new(num.to_string()));
    }

    sql.push_str(" ORDER BY received_at DESC LIMIT 1");

    let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
    let mut stmt = conn.prepare(&sql)?;

    let result = stmt.query_row(&params_refs[..], |row| {
        Ok(OtpResult {
            message_id: row.get(0)?,
            recipient_number: row.get(1)?,
            sender: row.get(2)?,
            body: row.get(3)?,
            received_at: row.get(4)?,
            code: row.get(5)?,
        })
    }).optional()?;

    Ok(result)
}

pub fn get_message_by_id(
    conn: &Connection,
    id: &str,
    token: &ApiToken,
) -> Result<Option<Message>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, recipient_number, sender, body, received_at, device_id, sim_slot, extracted_code, raw_metadata, is_read
         FROM messages WHERE id = ?1",
    )?;

    let msg = stmt.query_row(params![id], |row| {
        Ok(Message {
            id: row.get(0)?,
            recipient_number: row.get(1)?,
            sender: row.get(2)?,
            body: row.get(3)?,
            received_at: row.get(4)?,
            device_id: row.get(5)?,
            sim_slot: row.get(6)?,
            extracted_code: row.get(7)?,
            raw_metadata: row.get(8)?,
            is_read: row.get::<_, i32>(9)? != 0,
        })
    }).optional()?;

    if let Some(ref m) = msg {
        if !token.can_access_number(&m.recipient_number) {
            return Ok(None);
        }
    }

    Ok(msg)
}

pub fn delete_message(
    conn: &Connection,
    id: &str,
    token: &ApiToken,
) -> Result<bool, rusqlite::Error> {
    // Check permission first
    let msg = get_message_by_id(conn, id, token)?;
    if msg.is_none() {
        return Ok(false);
    }
    if !token.can_delete && !token.is_admin {
        return Ok(false);
    }

    let count = conn.execute("DELETE FROM messages WHERE id = ?1", params![id])?;
    Ok(count > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_permissions_and_query_isolation() {
        let pool = init_memory_pool();
        let conn = pool.get().unwrap();

        // Register two phone numbers
        upsert_phone_number(&conn, "+15551111111", Some("Number 1"), None).unwrap();
        upsert_phone_number(&conn, "+15552222222", Some("Number 2"), None).unwrap();

        // Create Token A (only +15551111111)
        let (token_a, raw_token_a) = create_api_token(
            &conn,
            CreateTokenRequest {
                name: "Agent A".to_string(),
                allowed_numbers: vec!["+15551111111".to_string()],
                can_read: true,
                can_delete: false,
                is_admin: false,
            },
        ).unwrap();

        // Create Token B (only +15552222222)
        let (token_b, _raw_token_b) = create_api_token(
            &conn,
            CreateTokenRequest {
                name: "Agent B".to_string(),
                allowed_numbers: vec!["+15552222222".to_string()],
                can_read: true,
                can_delete: false,
                is_admin: false,
            },
        ).unwrap();

        // Create Token Admin (wildcard "*")
        let (token_admin, _raw_admin) = create_api_token(
            &conn,
            CreateTokenRequest {
                name: "Admin".to_string(),
                allowed_numbers: vec!["*".to_string()],
                can_read: true,
                can_delete: true,
                is_admin: true,
            },
        ).unwrap();

        // Verify token lookup by raw token
        let verified = verify_api_token(&conn, &raw_token_a).unwrap().unwrap();
        assert_eq!(verified.id, token_a.id);
        assert!(verified.can_access_number("+15551111111"));
        assert!(!verified.can_access_number("+15552222222"));

        // Insert SMS for Number 1 with OTP
        insert_message(
            &conn,
            IngestSmsRequest {
                recipient_number: "+15551111111".to_string(),
                sender: "BankSecure".to_string(),
                body: "Your security code is 781920".to_string(),
                received_at: None,
                device_id: Some("dev_1".to_string()),
                sim_slot: Some(0),
                metadata: None,
            },
            Some("781920".to_string()),
        ).unwrap();

        // Insert SMS for Number 2 with OTP
        insert_message(
            &conn,
            IngestSmsRequest {
                recipient_number: "+15552222222".to_string(),
                sender: "GovAuth".to_string(),
                body: "Use 645123 for two-factor login".to_string(),
                received_at: None,
                device_id: Some("dev_2".to_string()),
                sim_slot: Some(0),
                metadata: None,
            },
            Some("645123".to_string()),
        ).unwrap();

        // Token A queries messages -> MUST ONLY SEE Number 1 messages!
        let msgs_a = query_messages(&conn, &MessageQueryFilter::default(), &token_a).unwrap();
        assert_eq!(msgs_a.len(), 1);
        assert_eq!(msgs_a[0].recipient_number, "+15551111111");
        assert_eq!(msgs_a[0].extracted_code, Some("781920".to_string()));

        // Token B queries messages -> MUST ONLY SEE Number 2 messages!
        let msgs_b = query_messages(&conn, &MessageQueryFilter::default(), &token_b).unwrap();
        assert_eq!(msgs_b.len(), 1);
        assert_eq!(msgs_b[0].recipient_number, "+15552222222");
        assert_eq!(msgs_b[0].extracted_code, Some("645123".to_string()));

        // Token A asks for latest OTP without specifying number -> Gets Number 1 OTP
        let otp_a = get_latest_otp(&conn, None, &token_a, 10).unwrap().unwrap();
        assert_eq!(otp_a.code, "781920");

        // Token A tries to ask for Number 2's OTP -> MUST BE DENIED / None!
        let otp_unauth = get_latest_otp(&conn, Some("+15552222222"), &token_a, 10).unwrap();
        assert!(otp_unauth.is_none(), "Token A must NOT be able to fetch OTP for Number 2");

        // Token Admin sees both messages
        let msgs_admin = query_messages(&conn, &MessageQueryFilter::default(), &token_admin).unwrap();
        assert_eq!(msgs_admin.len(), 2);

        // List phone numbers for Token A
        let numbers_a = list_phone_numbers(&conn, &token_a).unwrap();
        assert_eq!(numbers_a.len(), 1);
        assert_eq!(numbers_a[0].phone_number, "+15551111111");
    }
}
