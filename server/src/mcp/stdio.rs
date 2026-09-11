use std::io::{self, BufRead, Write};


use crate::{
    db::{self, DbPool},
    mcp::{handler::handle_mcp_request, protocol::*},
};

pub async fn run_stdio(pool: DbPool, token_str: &str) -> Result<(), Box<dyn std::error::Error>> {
    let conn = pool.get()?;
    let token = match db::verify_api_token(&conn, token_str)? {
        Some(t) => t,
        None => {
            eprintln!("Error: Invalid token provided for MCP STDIO mode");
            std::process::exit(1);
        }
    };

    eprintln!(
        "🤖 HermesGate MCP server started in STDIO mode for '{}' (Allowed numbers: {:?})",
        token.name, token.allowed_numbers
    );

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break, // EOF or pipe closed
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let rpc_req: JsonRpcRequest = match serde_json::from_str(trimmed) {
            Ok(req) => req,
            Err(e) => {
                let err_resp = JsonRpcResponse::error(
                    None,
                    -32700,
                    format!("Parse error: {}", e),
                    None,
                );
                let out = serde_json::to_string(&err_resp)?;
                writeln!(stdout, "{}", out)?;
                stdout.flush()?;
                continue;
            }
        };

        if let Some(resp) = handle_mcp_request(&conn, &token, rpc_req) {
            let out = serde_json::to_string(&resp)?;
            writeln!(stdout, "{}", out)?;
            stdout.flush()?;
        }
    }

    Ok(())
}
