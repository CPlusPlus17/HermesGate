use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(name = "hermesgate", about = "HermesGate: Secure Multi-Device SMS Gateway with MCP, REST, and Web UI")]
pub struct Config {
    /// Host address to bind
    #[arg(long, default_value = "0.0.0.0", env = "HOST")]
    pub host: String,

    /// Port to listen on
    #[arg(short, long, default_value_t = 8080, env = "PORT")]
    pub port: u16,

    /// SQLite database file path
    #[arg(long, default_value = "hermesgate.db", env = "DATABASE_PATH")]
    pub db_path: String,

    /// Pre-configured master admin token
    #[arg(long, env = "ADMIN_TOKEN")]
    pub admin_token: Option<String>,

    /// Run in Model Context Protocol (MCP) STDIO mode
    #[arg(long)]
    pub mcp_stdio: bool,

    /// API/MCP Bearer Token (required when running in STDIO mode)
    #[arg(long, env = "SMS_TOKEN")]
    pub token: Option<String>,
}
