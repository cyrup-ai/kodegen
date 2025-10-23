//! SQL parsing utilities for statement splitting, comment stripping, and keyword extraction
//!
//! Uses sqlparser crate for proper SQL parsing with validation.

use crate::error::DatabaseError;
use crate::types::DatabaseType;
use sqlparser::dialect::{Dialect, GenericDialect, MySqlDialect, PostgreSqlDialect, SQLiteDialect};
use sqlparser::parser::Parser;

/// Split multi-statement SQL by semicolons, respecting string literals
///
/// Handles both SQL standard doubled-quote escaping (`''`, `""`) and MySQL-style
/// backslash escaping (`\'`, `\"`, `\\`) based on database type.
///
/// # Examples
/// ```
/// let sql = "SELECT 1; INSERT INTO t VALUES ('a;b'); SELECT 2;";
/// let stmts = split_sql_statements(sql, DatabaseType::Postgres);
/// assert_eq!(stmts.len(), 3);
/// assert_eq!(stmts[1], "INSERT INTO t VALUES ('a;b')");
/// ```
pub fn split_sql_statements(sql: &str, db_type: DatabaseType) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut chars = sql.chars().peekable();
    
    // Determine if backslash escapes are enabled
    let backslash_escapes = matches!(db_type, DatabaseType::MySQL | DatabaseType::MariaDB);

    while let Some(ch) = chars.next() {
        match ch {
            // Handle backslash escapes for MySQL/MariaDB
            '\\' if backslash_escapes && (in_single_quote || in_double_quote) => {
                current.push(ch);
                // Consume next character (the escaped character)
                if let Some(next_ch) = chars.next() {
                    current.push(next_ch);
                }
            }
            '\'' if !in_double_quote => {
                current.push(ch);
                // Handle escaped quotes (doubled quotes: '' in SQL)
                if chars.peek() == Some(&'\'') {
                    current.push(chars.next().unwrap());
                } else {
                    in_single_quote = !in_single_quote;
                }
            }
            '"' if !in_single_quote => {
                current.push(ch);
                // Handle escaped quotes (doubled quotes: "" in SQL)
                if chars.peek() == Some(&'"') {
                    current.push(chars.next().unwrap());
                } else {
                    in_double_quote = !in_double_quote;
                }
            }
            ';' if !in_single_quote && !in_double_quote => {
                // Semicolon outside quotes - split here
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    statements.push(trimmed.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    // Don't forget the last statement (may not end with semicolon)
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        statements.push(trimmed.to_string());
    }

    statements
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
        let stmts = split_sql_statements(sql, DatabaseType::Postgres);
        assert_eq!(stmts.len(), 3);
        assert!(stmts[1].contains("'a;b;c'"));
    }

    #[test]
    fn test_strip_preserves_strings() {
        let sql = "SELECT '-- not a comment' FROM t";
        let cleaned = strip_comments(sql, DatabaseType::Postgres);
        assert!(cleaned.contains("-- not a comment"));
    }
}
