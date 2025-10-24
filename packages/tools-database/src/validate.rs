//! Identifier validation for SQL injection prevention

use crate::error::DatabaseError;

/// Validate SQLite identifier for safe use in PRAGMA commands
///
/// SQLite PRAGMA commands do NOT support parameterized queries, requiring
/// direct string interpolation. This function validates identifiers to prevent
/// SQL injection attacks.
///
/// ## Validation Rules
///
/// - **Length**: 1-64 characters (reasonable limit for identifiers)
/// - **Characters**: Only alphanumeric and underscore `[a-zA-Z0-9_]`
/// - **Start character**: Must be letter or underscore (not digit)
/// - **Keywords**: Cannot be SQL keywords (SELECT, DROP, etc.)
///
/// ## Why These Rules?
///
/// These rules are intentionally **more restrictive** than SQLite's actual
/// identifier syntax. This defense-in-depth approach ensures safety even if
/// future code changes introduce new attack vectors.
///
/// ## Example
///
/// ```rust
/// use kodegen_tools_database::validate::validate_sqlite_identifier;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
///
/// // Valid identifiers
/// validate_sqlite_identifier("users")?;
/// validate_sqlite_identifier("user_accounts")?;
/// validate_sqlite_identifier("table_123")?;
/// validate_sqlite_identifier("_private")?;
///
/// // Invalid identifiers (SQL injection attempts)
/// # assert!(validate_sqlite_identifier("users; DROP TABLE users").is_err());
/// # assert!(validate_sqlite_identifier("users)").is_err());
/// # assert!(validate_sqlite_identifier("users'").is_err());
/// # assert!(validate_sqlite_identifier("users--").is_err());
///
/// // Invalid identifiers (rule violations)
/// # assert!(validate_sqlite_identifier("").is_err());
/// # assert!(validate_sqlite_identifier("123table").is_err());
/// # assert!(validate_sqlite_identifier("SELECT").is_err());
/// # Ok(())
/// # }
/// ```
pub fn validate_sqlite_identifier(name: &str) -> Result<(), DatabaseError> {
    // Rule 1: Check empty
    if name.is_empty() {
        return Err(DatabaseError::QueryError(
            "Identifier cannot be empty".to_string(),
        ));
    }

    // Rule 2: Check length (64 chars is reasonable limit)
    if name.len() > 64 {
        return Err(DatabaseError::QueryError(format!(
            "Identifier too long: {} characters (max 64)",
            name.len()
        )));
    }

    // Rule 3: Check characters - only alphanumeric and underscore
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(DatabaseError::QueryError(format!(
            "Invalid identifier: '{}'. Only alphanumeric and underscore allowed",
            name
        )));
    }

    // Rule 4: Check doesn't start with digit
    if let Some(first_char) = name.chars().next()
        && first_char.is_ascii_digit()
    {
        return Err(DatabaseError::QueryError(format!(
            "Identifier cannot start with digit: '{}'",
            name
        )));
    }

    // Rule 5: Check not a SQL keyword (defense-in-depth)
    // Keywords that could be exploited or cause confusion
    let keywords = [
        "SELECT", "INSERT", "UPDATE", "DELETE", "DROP", "CREATE", "ALTER", "TABLE", "INDEX",
        "VIEW", "TRIGGER", "PRAGMA", "ATTACH", "DETACH", "BEGIN", "COMMIT", "ROLLBACK", "VACUUM",
        "ANALYZE",
    ];

    if keywords.contains(&name.to_uppercase().as_str()) {
        return Err(DatabaseError::QueryError(format!(
            "Identifier cannot be SQL keyword: '{}'",
            name
        )));
    }

    Ok(())
}
