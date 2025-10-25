//! SQL query result limiting to prevent excessive data transfer

use crate::error::DatabaseError;
use crate::sql_parser::extract_first_keyword;
use crate::types::DatabaseType;
use lazy_regex::{Lazy, Regex, lazy_regex};

// Compile-time validated regexes
static LIMIT_REGEX: Lazy<Regex> = lazy_regex!(r"(?i)\bLIMIT\s+(\d+)");
static TOP_REGEX: Lazy<Regex> = lazy_regex!(r"(?i)\bSELECT\s+TOP\s+\(?\d+\)?");
static SELECT_TOP_REPLACE: Lazy<Regex> = lazy_regex!(r"(?i)\bSELECT\s+TOP\s+\(?\d+\)?");
static SELECT_WORD: Lazy<Regex> = lazy_regex!(r"(?i)\bSELECT\b");

/// Apply row limit to SELECT queries only
///
/// For PostgreSQL, MySQL, MariaDB, SQLite: Adds/modifies LIMIT clause
/// For SQL Server: Adds/modifies TOP clause (currently unused - sqlx 0.8 lacks mssql)
///
/// # Behavior
/// - If existing limit is smaller than max_rows, keeps existing limit
/// - If existing limit is larger than max_rows, replaces with max_rows
/// - If no limit exists, adds LIMIT/TOP with max_rows
/// - Non-SELECT queries are returned unchanged
///
/// # Examples
/// ```
/// # use kodegen_tools_database::sql_limiter::apply_row_limit;
/// # use kodegen_tools_database::types::DatabaseType;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let sql = "SELECT * FROM users";
/// let limited = apply_row_limit(sql, 100, DatabaseType::Postgres)?;
/// assert_eq!(limited, "SELECT * FROM users LIMIT 100");
///
/// let sql = "SELECT * FROM users LIMIT 200";
/// let limited = apply_row_limit(sql, 100, DatabaseType::Postgres)?;
/// assert_eq!(limited, "SELECT * FROM users LIMIT 100");
/// # Ok(())
/// # }
/// ```
pub fn apply_row_limit(
    sql: &str,
    max_rows: usize,
    db_type: DatabaseType,
) -> Result<String, DatabaseError> {
    // Only apply to SELECT queries (strip comments first to detect keyword)
    let keyword = extract_first_keyword(sql, db_type)?;
    if keyword != "select" {
        return Ok(sql.to_string());
    }

    match db_type {
        DatabaseType::SqlServer => apply_top_limit(sql, max_rows),
        _ => apply_standard_limit(sql, max_rows),
    }
}

/// Apply LIMIT clause for PostgreSQL, MySQL, MariaDB, SQLite
fn apply_standard_limit(sql: &str, max_rows: usize) -> Result<String, DatabaseError> {
    if let Some(captures) = LIMIT_REGEX.captures(sql) {
        // Existing LIMIT found - use minimum of existing and max_rows
        let existing_limit: usize = captures[1]
            .parse()
            .map_err(|e| DatabaseError::QueryError(format!("Invalid LIMIT value: {}", e)))?;

        let effective_limit = existing_limit.min(max_rows);
        let result = LIMIT_REGEX.replace(sql, format!("LIMIT {}", effective_limit));
        Ok(result.to_string())
    } else {
        // No LIMIT - add one at the end
        let trimmed = sql.trim();
        let has_semicolon = trimmed.ends_with(';');
        let sql_without_semi = if has_semicolon {
            &trimmed[..trimmed.len() - 1]
        } else {
            trimmed
        };

        Ok(format!(
            "{} LIMIT {}{}",
            sql_without_semi,
            max_rows,
            if has_semicolon { ";" } else { "" }
        ))
    }
}

/// Apply TOP clause for SQL Server (currently unused - sqlx 0.8 lacks mssql support)
///
/// SQL Server uses SELECT TOP N instead of LIMIT
/// This code is included for future compatibility when sqlx adds mssql back
fn apply_top_limit(sql: &str, max_rows: usize) -> Result<String, DatabaseError> {
    if TOP_REGEX.is_match(sql) {
        // Replace existing TOP N with TOP max_rows
        let result = SELECT_TOP_REPLACE.replace(sql, format!("SELECT TOP {}", max_rows));
        Ok(result.to_string())
    } else {
        // Add TOP N after SELECT keyword
        let result = SELECT_WORD.replace(sql, format!("SELECT TOP {}", max_rows));
        Ok(result.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adds_limit() {
        let sql = "SELECT * FROM users";
        let result = apply_row_limit(sql, 100, DatabaseType::Postgres);
        assert!(result.is_ok(), "apply_row_limit failed: {:?}", result.err());
        if let Ok(result) = result {
            assert!(result.contains("LIMIT 100"));
        }
    }

    #[test]
    fn test_replaces_larger_limit() {
        let sql = "SELECT * FROM users LIMIT 200";
        let result = apply_row_limit(sql, 100, DatabaseType::Postgres);
        assert!(result.is_ok(), "apply_row_limit failed: {:?}", result.err());
        if let Ok(result) = result {
            assert!(result.contains("LIMIT 100"));
            assert!(!result.contains("LIMIT 200"));
        }
    }

    #[test]
    fn test_keeps_smaller_limit() {
        let sql = "SELECT * FROM users LIMIT 50";
        let result = apply_row_limit(sql, 100, DatabaseType::Postgres);
        assert!(result.is_ok(), "apply_row_limit failed: {:?}", result.err());
        if let Ok(result) = result {
            assert!(result.contains("LIMIT 50"));
        }
    }

    #[test]
    fn test_preserves_semicolon() {
        let sql = "SELECT * FROM users;";
        let result = apply_row_limit(sql, 100, DatabaseType::Postgres);
        assert!(result.is_ok(), "apply_row_limit failed: {:?}", result.err());
        if let Ok(result) = result {
            assert!(result.ends_with(';'));
        }
    }

    #[test]
    fn test_ignores_non_select() {
        let sql = "INSERT INTO users VALUES (1)";
        let result = apply_row_limit(sql, 100, DatabaseType::Postgres);
        assert!(result.is_ok(), "apply_row_limit failed: {:?}", result.err());
        if let Ok(result) = result {
            assert!(!result.contains("LIMIT"));
        }
    }
}
