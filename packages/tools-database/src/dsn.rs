//! Database connection string (DSN) parsing and manipulation
//!
//! This module provides utilities for parsing, validating, and rewriting
//! database connection strings (DSNs) across multiple database types.

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

/// Parsed database connection string information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DSNInfo {
    /// Database protocol/type: "postgres", "mysql", "sqlite", "sqlserver"
    pub protocol: String,

    /// Optional username for authentication
    pub username: Option<String>,

    /// Optional password for authentication
    pub password: Option<String>,

    /// Hostname or file path (for SQLite)
    pub hostname: String,

    /// Optional port number
    pub port: Option<u16>,

    /// Database name or file path
    pub database: String,

    /// Query parameters from DSN (e.g., sslmode=disable)
    pub query_params: HashMap<String, String>,
}

impl DSNInfo {
    /// Reconstruct DSN string from components
    pub fn to_dsn(&self) -> String {
        let mut dsn = format!("{}://", self.protocol);

        // Add auth if present
        if let Some(ref user) = self.username {
            dsn.push_str(user);
            if let Some(ref pass) = self.password {
                dsn.push(':');
                dsn.push_str(pass);
            }
            dsn.push('@');
        }

        // Add host and port
        dsn.push_str(&self.hostname);
        if let Some(port) = self.port {
            dsn.push_str(&format!(":{}", port));
        }

        // Add database
        dsn.push('/');
        dsn.push_str(&self.database);

        // Add query params
        if !self.query_params.is_empty() {
            dsn.push('?');
            let params: Vec<String> = self
                .query_params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            dsn.push_str(&params.join("&"));
        }

        dsn
    }
}

pub fn parse_dsn(dsn: &str) -> Result<DSNInfo> {
    // Validate non-empty
    if dsn.trim().is_empty() {
        bail!("DSN cannot be empty");
    }

    // Extract protocol
    let protocol = dsn
        .split("://")
        .next()
        .context("Invalid DSN: missing protocol separator '://'")?
        .to_lowercase();

    // Normalize protocol aliases
    let protocol = match protocol.as_str() {
        "postgresql" => "postgres",
        "mariadb" => "mysql", // MariaDB uses MySQL protocol
        other => other,
    };

    // Handle SQLite special case
    if protocol == "sqlite" {
        return parse_sqlite_dsn(dsn);
    }

    // Parse standard network DSN
    let url = Url::parse(dsn).context("Failed to parse DSN as URL")?;

    // Extract components
    let username = if !url.username().is_empty() {
        Some(url.username().to_string())
    } else {
        None
    };

    // URL crate automatically handles percent-decoding for passwords
    let password = url.password().map(|p| p.to_string());

    let hostname = url.host_str().context("DSN missing hostname")?.to_string();

    let port = url.port();

    // Extract database from path (remove leading '/')
    let database = url
        .path()
        .strip_prefix('/')
        .unwrap_or(url.path())
        .to_string();

    if database.is_empty() {
        bail!("DSN missing database name");
    }

    // Extract query parameters
    let mut query_params = HashMap::new();
    for (key, value) in url.query_pairs() {
        query_params.insert(key.to_string(), value.to_string());
    }

    Ok(DSNInfo {
        protocol: protocol.to_string(),
        username,
        password,
        hostname,
        port,
        database,
        query_params,
    })
}

fn parse_sqlite_dsn(dsn: &str) -> Result<DSNInfo> {
    // SQLite format: sqlite:///path or sqlite:///:memory:
    let path_part = dsn
        .strip_prefix("sqlite://")
        .context("Invalid SQLite DSN format")?;

    // Handle in-memory database
    if path_part == "/:memory:" {
        return Ok(DSNInfo {
            protocol: "sqlite".to_string(),
            username: None,
            password: None,
            hostname: ":memory:".to_string(),
            port: None,
            database: ":memory:".to_string(),
            query_params: HashMap::new(),
        });
    }

    // Handle file path (strip leading / for absolute paths)
    let file_path = path_part.strip_prefix('/').unwrap_or(path_part);

    Ok(DSNInfo {
        protocol: "sqlite".to_string(),
        username: None,
        password: None,
        hostname: file_path.to_string(),
        port: None,
        database: file_path.to_string(),
        query_params: HashMap::new(),
    })
}

/// Validate DSN format and return database type
pub fn validate_dsn(dsn: &str) -> Result<String> {
    // Parse to validate structure
    let info = parse_dsn(dsn)?;

    // Check protocol is supported
    let supported = ["postgres", "mysql", "sqlite", "sqlserver"];
    if !supported.contains(&info.protocol.as_str()) {
        bail!(
            "Unsupported database type '{}'. Supported: {}",
            info.protocol,
            supported.join(", ")
        );
    }

    // Validate port range if present
    if let Some(port) = info.port
        && port == 0
    {
        bail!("Invalid port number: {}. Must be 1-65535", port);
    }

    // SQLite-specific validation
    if info.protocol == "sqlite" {
        if info.hostname == ":memory:" {
            return Ok("sqlite".to_string());
        }

        // Check if path looks reasonable (not validating existence)
        if info.hostname.is_empty() {
            bail!("SQLite DSN missing file path");
        }
    } else {
        // Network database validation
        if info.hostname.is_empty() {
            bail!("DSN missing hostname");
        }
        if info.database.is_empty() {
            bail!("DSN missing database name");
        }
    }

    Ok(info.protocol)
}

/// Rewrite DSN to connect through SSH tunnel on localhost
///
/// Takes original DSN pointing to remote host and rewrites it to
/// connect to localhost:tunnel_port, preserving all other components.
///
/// # Example
/// ```
/// let original = "postgres://user:pass@remote.db.com:5432/mydb?sslmode=require";
/// let rewritten = rewrite_dsn_for_tunnel(original, 54321)?;
/// // Result: "postgres://user:pass@127.0.0.1:54321/mydb?sslmode=require"
/// ```
pub fn rewrite_dsn_for_tunnel(dsn: &str, tunnel_port: u16) -> Result<String> {
    let mut info = parse_dsn(dsn).context("Failed to parse DSN for tunnel rewriting")?;

    // SQLite doesn't support tunneling (no network connection)
    if info.protocol == "sqlite" {
        bail!("Cannot create SSH tunnel for SQLite (file-based database)");
    }

    // Rewrite hostname and port to tunnel endpoint
    info.hostname = "127.0.0.1".to_string();
    info.port = Some(tunnel_port);

    // Reconstruct DSN with new connection details
    Ok(info.to_dsn())
}

/// Extract hostname from DSN
pub fn extract_host(dsn: &str) -> Result<String> {
    let info = parse_dsn(dsn)?;

    if info.protocol == "sqlite" {
        bail!("SQLite databases do not have a network host");
    }

    Ok(info.hostname)
}

/// Extract port from DSN, using database-specific defaults
pub fn extract_port(dsn: &str) -> Result<u16> {
    let info = parse_dsn(dsn)?;

    if info.protocol == "sqlite" {
        bail!("SQLite databases do not have a network port");
    }

    // Return explicit port or database-specific default
    Ok(info.port.unwrap_or_else(|| default_port(&info.protocol)))
}

/// Get default port for database type
fn default_port(protocol: &str) -> u16 {
    match protocol {
        "postgres" => 5432,
        "mysql" => 3306,
        "sqlserver" => 1433,
        _ => 5432, // Fallback to postgres default
    }
}

/// Extract database name from DSN
pub fn extract_database(dsn: &str) -> Result<String> {
    let info = parse_dsn(dsn)?;
    Ok(info.database)
}

/// Detect database type from DSN protocol
///
/// Returns normalized database type: "postgres", "mysql", "sqlite", "sqlserver"
pub fn detect_database_type(dsn: &str) -> Result<String> {
    let protocol = dsn
        .split("://")
        .next()
        .context("Invalid DSN: missing protocol separator")?
        .to_lowercase();

    // Map protocol to database type, handling aliases
    let db_type = match protocol.as_str() {
        "postgres" | "postgresql" => "postgres",
        "mysql" | "mariadb" => "mysql",
        "sqlite" | "sqlite3" => "sqlite",
        "sqlserver" | "mssql" => "sqlserver",
        unknown => bail!("Unknown database protocol: {}", unknown),
    };

    Ok(db_type.to_string())
}
