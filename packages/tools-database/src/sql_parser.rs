//! SQL parsing utilities for statement splitting, comment stripping, and keyword extraction
//!
//! Uses sqlparser crate for proper SQL parsing with validation.

use crate::error::DatabaseError;
use crate::types::DatabaseType;
use sqlparser::dialect::{Dialect, MsSqlDialect, MySqlDialect, PostgreSqlDialect, SQLiteDialect};
use sqlparser::parser::Parser;

/// Get appropriate SQL dialect for the database type
fn get_dialect(db_type: DatabaseType) -> Box<dyn Dialect> {
    match db_type {
        DatabaseType::Postgres => Box::new(PostgreSqlDialect {}),
        DatabaseType::MySQL | DatabaseType::MariaDB => Box::new(MySqlDialect {}),
        DatabaseType::SQLite => Box::new(SQLiteDialect {}),
        DatabaseType::SqlServer => Box::new(MsSqlDialect {}),
    }
}

/// Split multi-statement SQL by semicolons, respecting string literals
///
/// Uses sqlparser crate for proper SQL parsing with validation.
/// Detects unterminated string literals and returns an error.
///
/// # Examples
/// ```
/// let sql = "SELECT 1; INSERT INTO t VALUES ('a;b'); SELECT 2;";
/// let stmts = split_sql_statements(sql, DatabaseType::Postgres)?;
/// assert_eq!(stmts.len(), 3);
/// assert_eq!(stmts[1], "INSERT INTO t VALUES ('a;b')");
/// ```
///
/// # Errors
/// Returns `DatabaseError::QueryError` if:
/// - SQL contains unterminated string literals
/// - SQL has invalid syntax that prevents parsing
pub fn split_sql_statements(sql: &str, db_type: DatabaseType) -> Result<Vec<String>, DatabaseError> {
    let dialect = get_dialect(db_type);
    
    Parser::parse_sql(&*dialect, sql)
        .map(|stmts| stmts.iter().map(|s| s.to_string()).collect())
        .map_err(|e| DatabaseError::QueryError(format!("SQL parse error: {}", e)))
}

/// Strip SQL comments (single-line and multi-line), respecting string literals
///
/// Handles both SQL standard doubled-quote escaping (`''`, `""`) and MySQL-style
/// backslash escaping (`\'`, `\"`, `\\`) based on database type.
///
/// # Examples
/// ```
/// let sql = "SELECT * FROM users -- get all\n/* WHERE active */";
/// let cleaned = strip_comments(sql, DatabaseType::Postgres);
/// assert_eq!(cleaned, "SELECT * FROM users \n");
/// ```
pub fn strip_comments(sql: &str, db_type: DatabaseType) -> String {
    let mut result = String::new();
    let mut chars = sql.chars().peekable();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    
    let backslash_escapes = matches!(db_type, DatabaseType::MySQL | DatabaseType::MariaDB);

    while let Some(ch) = chars.next() {
        match ch {
            // Handle backslash escapes for MySQL/MariaDB
            '\\' if backslash_escapes && (in_single_quote || in_double_quote) => {
                result.push(ch);
                if let Some(next_ch) = chars.next() {
                    result.push(next_ch);
                }
            }
            '\'' if !in_double_quote => {
                result.push(ch);
                if chars.peek() == Some(&'\'') {
                    result.push(chars.next().unwrap());
                } else {
                    in_single_quote = !in_single_quote;
                }
            }
            '"' if !in_single_quote => {
                result.push(ch);
                if chars.peek() == Some(&'"') {
                    result.push(chars.next().unwrap());
                } else {
                    in_double_quote = !in_double_quote;
                }
            }
            '-' if !in_single_quote && !in_double_quote && chars.peek() == Some(&'-') => {
                // Single-line comment: -- to end of line
                chars.next(); // consume second dash
                for c in chars.by_ref() {
                    if c == '\n' {
                        result.push(c); // keep the newline for structure
                        break;
                    }
                }
            }
            '/' if !in_single_quote && !in_double_quote && chars.peek() == Some(&'*') => {
                // Multi-line comment: /* ... */
                chars.next(); // consume *
                let mut prev = '/';
                for c in chars.by_ref() {
                    if prev == '*' && c == '/' {
                        break; // end of comment
                    }
                    prev = c;
                }
            }
            _ => result.push(ch),
        }
    }

    result
}

/// Extract first SQL keyword from statement (after stripping comments)
///
/// # Examples
/// ```
/// let sql = "  SELECT * FROM users";
/// assert_eq!(extract_first_keyword(sql, DatabaseType::Postgres)?, "select");
///
/// let sql = "-- comment\nINSERT INTO logs";
/// assert_eq!(extract_first_keyword(sql, DatabaseType::Postgres)?, "insert");
/// ```
pub fn extract_first_keyword(sql: &str, db_type: DatabaseType) -> Result<String, DatabaseError> {
    let cleaned = strip_comments(sql, db_type);
    let trimmed = cleaned.trim();

    if trimmed.is_empty() {
        return Err(DatabaseError::QueryError(
            "Empty SQL statement after stripping comments".to_string(),
        ));
    }

    // Extract first word and convert to lowercase
    let keyword = trimmed
        .split_whitespace()
        .next()
        .ok_or_else(|| DatabaseError::QueryError("No SQL keyword found".to_string()))?
        .to_lowercase();

    Ok(keyword)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_respects_string_literals() {
        let sql = "SELECT 1; INSERT INTO t VALUES ('a;b;c'); SELECT 2;";
        let stmts = split_sql_statements(sql, DatabaseType::Postgres).unwrap();
        assert_eq!(stmts.len(), 3);
        assert!(stmts[1].contains("'a;b;c'"));
    }

    #[test]
    fn test_strip_preserves_strings() {
        let sql = "SELECT '-- not a comment' FROM t";
        let cleaned = strip_comments(sql, DatabaseType::Postgres);
        assert!(cleaned.contains("-- not a comment"));
    }

    #[test]
    fn test_unterminated_single_quote() {
        let sql = "INSERT INTO t VALUES ('test);";
        let result = split_sql_statements(sql, DatabaseType::Postgres);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("parse error"));
    }

    #[test]
    fn test_unterminated_double_quote() {
        let sql = "SELECT \"column FROM table;";
        let result = split_sql_statements(sql, DatabaseType::Postgres);
        assert!(result.is_err());
    }

    #[test]
    fn test_terminated_strings_ok() {
        let sql = "SELECT 'test'; INSERT INTO t VALUES ('data');";
        let result = split_sql_statements(sql, DatabaseType::Postgres);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn test_escaped_quotes_ok() {
        let sql = "SELECT 'can''t'; SELECT \"quote\"\"test\";";
        let result = split_sql_statements(sql, DatabaseType::Postgres);
        assert!(result.is_ok());
    }
}
