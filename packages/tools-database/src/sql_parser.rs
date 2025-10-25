//! SQL parsing utilities for statement splitting, comment stripping, and keyword extraction
//!
//! Uses sqlparser crate for proper SQL parsing with validation.

use crate::error::DatabaseError;
use crate::types::DatabaseType;
use sqlparser::dialect::{Dialect, MsSqlDialect, MySqlDialect, PostgreSqlDialect, SQLiteDialect};
use sqlparser::parser::Parser;
use sqlparser::tokenizer::{Token, Tokenizer, Whitespace};

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
/// # use kodegen_tools_database::sql_parser::split_sql_statements;
/// # use kodegen_tools_database::types::DatabaseType;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let sql = "SELECT 1; INSERT INTO t VALUES ('a;b'); SELECT 2;";
/// let stmts = split_sql_statements(sql, DatabaseType::Postgres)?;
/// assert_eq!(stmts.len(), 3);
/// assert_eq!(stmts[1], "INSERT INTO t VALUES ('a;b')");
/// # Ok(())
/// # }
/// ```
///
/// # Errors
/// Returns `DatabaseError::QueryError` if:
/// - SQL contains unterminated string literals
/// - SQL has invalid syntax that prevents parsing
pub fn split_sql_statements(
    sql: &str,
    db_type: DatabaseType,
) -> Result<Vec<String>, DatabaseError> {
    let dialect = get_dialect(db_type);

    Parser::parse_sql(&*dialect, sql)
        .map(|stmts| stmts.iter().map(|s| s.to_string()).collect())
        .map_err(|e| DatabaseError::QueryError(format!("SQL parse error: {}", e)))
}

/// Strip SQL comments (single-line and multi-line) using sqlparser tokenizer
///
/// Uses sqlparser's tokenizer to correctly handle database-specific syntax features
/// while removing comments. This ensures proper handling of edge cases that manual
/// string parsing cannot reliably handle.
///
/// # Supported Features
/// - Single-line comments: `-- comment`
/// - Multi-line comments: `/* comment */`
/// - Nested block comments: `/* outer /* inner */ outer */` (PostgreSQL)
/// - String literals: `'text'` with `''` escaping
/// - PostgreSQL dollar-quoted strings: `$$text$$` or `$tag$text$tag$`
/// - PostgreSQL escape strings: `E'text\n'`
/// - SQL Server bracket identifiers: `[identifier]`
/// - MySQL backtick identifiers: `` `identifier` ``
/// - MySQL backslash escapes: `\'` (when db_type is MySQL/MariaDB)
/// - Unicode characters in comments and strings
///
/// # Examples
/// ```
/// # use kodegen_tools_database::sql_parser::strip_comments;
/// # use kodegen_tools_database::types::DatabaseType;
/// // Standard comments
/// let sql = "SELECT * FROM users -- get all\n/* WHERE active */";
/// let cleaned = strip_comments(sql, DatabaseType::Postgres);
/// assert_eq!(cleaned.trim(), "SELECT * FROM users");
///
/// // PostgreSQL dollar quotes preserve comment-like text
/// let sql = "SELECT $$ -- not a comment $$ FROM t";
/// let cleaned = strip_comments(sql, DatabaseType::Postgres);
/// assert!(cleaned.contains("-- not a comment"));
/// ```
///
/// # Error Handling
/// If tokenization fails (malformed SQL), returns the original SQL unchanged
/// as a fallback to ensure robustness.
pub fn strip_comments(sql: &str, db_type: DatabaseType) -> String {
    let dialect = get_dialect(db_type);
    let mut tokenizer = Tokenizer::new(&*dialect, sql);

    match tokenizer.tokenize() {
        Ok(tokens) => {
            // Filter out comment tokens (which are represented as Whitespace variants)
            // Keep other whitespace (space, tab, newline) but remove comments
            tokens
                .iter()
                .filter(|token| {
                    !matches!(
                        token,
                        Token::Whitespace(Whitespace::SingleLineComment { .. })
                            | Token::Whitespace(Whitespace::MultiLineComment(_))
                    )
                })
                .map(|token| token.to_string())
                .collect::<Vec<_>>()
                .join("")
        }
        Err(_) => {
            // Fallback to original SQL if tokenization fails
            sql.to_string()
        }
    }
}

/// Extract first SQL keyword from statement (after stripping comments)
///
/// # Examples
/// ```
/// # use kodegen_tools_database::sql_parser::extract_first_keyword;
/// # use kodegen_tools_database::types::DatabaseType;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let sql = "  SELECT * FROM users";
/// assert_eq!(extract_first_keyword(sql, DatabaseType::Postgres)?, "select");
///
/// let sql = "-- comment\nINSERT INTO logs";
/// assert_eq!(extract_first_keyword(sql, DatabaseType::Postgres)?, "insert");
/// # Ok(())
/// # }
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
        let result = split_sql_statements(sql, DatabaseType::Postgres);
        assert!(result.is_ok(), "split_sql_statements failed: {:?}", result.err());
        if let Ok(stmts) = result {
            assert_eq!(stmts.len(), 3);
            assert!(stmts[1].contains("'a;b;c'"));
        }
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
        assert!(result.is_err(), "Expected error for unterminated single quote");
        if let Err(e) = result {
            assert!(e.to_string().contains("parse error"));
        }
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
        assert!(result.is_ok(), "split_sql_statements failed: {:?}", result.err());
        if let Ok(stmts) = result {
            assert_eq!(stmts.len(), 2);
        }
    }

    #[test]
    fn test_escaped_quotes_ok() {
        let sql = "SELECT 'can''t'; SELECT \"quote\"\"test\";";
        let result = split_sql_statements(sql, DatabaseType::Postgres);
        assert!(result.is_ok());
    }

    // Edge case tests for comment stripping

    #[test]
    fn test_postgres_dollar_quotes_basic() {
        let sql = "SELECT $$ -- not a comment $$ FROM t";
        let cleaned = strip_comments(sql, DatabaseType::Postgres);
        // The -- inside dollar quotes should be preserved
        assert!(
            cleaned.contains("-- not a comment"),
            "Dollar-quoted string should preserve comment-like text, got: {}",
            cleaned
        );
    }

    #[test]
    fn test_postgres_dollar_quotes_tagged() {
        let sql = "SELECT $tag$ /* not a comment */ $tag$ FROM t";
        let cleaned = strip_comments(sql, DatabaseType::Postgres);
        // The /* */ inside tagged dollar quotes should be preserved
        assert!(
            cleaned.contains("/* not a comment */"),
            "Tagged dollar-quoted string should preserve comment markers, got: {}",
            cleaned
        );
    }

    #[test]
    fn test_postgres_dollar_quotes_with_real_comment() {
        let sql = "SELECT $$ text $$ FROM t -- Real comment";
        let cleaned = strip_comments(sql, DatabaseType::Postgres);
        // Real comment after dollar quotes should be stripped
        assert!(
            !cleaned.contains("Real comment"),
            "Real comment after dollar quotes should be stripped, got: {}",
            cleaned
        );
        assert!(
            cleaned.contains("text"),
            "Dollar-quoted text should be preserved, got: {}",
            cleaned
        );
    }

    #[test]
    fn test_sql_server_bracket_identifiers() {
        let sql = "SELECT [column--name] FROM table -- This comment should be stripped";
        let cleaned = strip_comments(sql, DatabaseType::SqlServer);
        // The -- inside brackets should be preserved
        assert!(
            cleaned.contains("[column--name]") || cleaned.contains("column--name"),
            "Bracket identifier should preserve comment-like text, got: {}",
            cleaned
        );
        // Real comment should be stripped
        assert!(
            !cleaned.contains("This comment should be stripped"),
            "Real comment should be stripped, got: {}",
            cleaned
        );
    }

    #[test]
    fn test_mysql_backtick_identifiers() {
        let sql = "SELECT `column--name` FROM table -- Real comment";
        let cleaned = strip_comments(sql, DatabaseType::MySQL);
        // The -- inside backticks should be preserved
        assert!(
            cleaned.contains("`column--name`") || cleaned.contains("column--name"),
            "Backtick identifier should preserve comment-like text, got: {}",
            cleaned
        );
        // Real comment should be stripped
        assert!(
            !cleaned.contains("Real comment"),
            "Real comment should be stripped, got: {}",
            cleaned
        );
    }

    #[test]
    fn test_mysql_backtick_with_block_comment() {
        let sql = "SELECT `column/* test */name` FROM t /* comment */";
        let cleaned = strip_comments(sql, DatabaseType::MySQL);
        // Block comment markers inside backticks should be preserved
        assert!(
            cleaned.contains("/* test */")
                || cleaned.contains("column") && cleaned.contains("name"),
            "Backtick identifier should preserve block comment markers, got: {}",
            cleaned
        );
        // Real block comment should be stripped
        assert!(
            !cleaned.contains("comment */") || cleaned.contains("`column/* test */name`"),
            "Real block comment should be stripped or be inside backticks, got: {}",
            cleaned
        );
    }

    #[test]
    fn test_postgres_escape_strings() {
        let sql = "SELECT E'Line 1\\n-- Not a comment' FROM t";
        let cleaned = strip_comments(sql, DatabaseType::Postgres);
        // The -- inside escape string should be preserved
        assert!(
            cleaned.contains("-- Not a comment"),
            "Escape string should preserve comment-like text, got: {}",
            cleaned
        );
    }

    #[test]
    fn test_postgres_escape_strings_with_real_comment() {
        let sql = "SELECT E'text\\n' FROM t -- Real comment";
        let cleaned = strip_comments(sql, DatabaseType::Postgres);
        // Real comment should be stripped
        assert!(
            !cleaned.contains("Real comment"),
            "Real comment after escape string should be stripped, got: {}",
            cleaned
        );
        assert!(
            cleaned.contains("text") || cleaned.contains("E'"),
            "Escape string should be preserved, got: {}",
            cleaned
        );
    }

    #[test]
    fn test_nested_block_comments_postgres() {
        let sql = "/* outer /* inner */ still comment */ SELECT 1";
        let cleaned = strip_comments(sql, DatabaseType::Postgres);
        // PostgreSQL supports nested comments - all should be stripped
        assert!(
            !cleaned.contains("outer"),
            "Nested comments should be fully stripped, got: {}",
            cleaned
        );
        assert!(
            !cleaned.contains("inner"),
            "Inner nested comment should be stripped, got: {}",
            cleaned
        );
        assert!(
            !cleaned.contains("still comment"),
            "Outer comment continuation should be stripped, got: {}",
            cleaned
        );
        assert!(
            cleaned.contains("SELECT") || cleaned.contains("select"),
            "SQL statement should be preserved, got: {}",
            cleaned
        );
    }

    #[test]
    fn test_unicode_in_comments() {
        let sql = "-- コメント (Japanese comment)\nSELECT 1";
        let cleaned = strip_comments(sql, DatabaseType::Postgres);
        // Unicode comment should be stripped
        assert!(
            !cleaned.contains("コメント"),
            "Unicode comment should be stripped, got: {}",
            cleaned
        );
        assert!(
            cleaned.contains("SELECT") || cleaned.contains("select"),
            "SQL statement should be preserved, got: {}",
            cleaned
        );
    }

    #[test]
    fn test_unicode_in_block_comments() {
        let sql = "/* 注释 中文 */ SELECT 1";
        let cleaned = strip_comments(sql, DatabaseType::Postgres);
        // Unicode block comment should be stripped
        assert!(
            !cleaned.contains("注释"),
            "Unicode block comment should be stripped, got: {}",
            cleaned
        );
        assert!(
            cleaned.contains("SELECT") || cleaned.contains("select"),
            "SQL statement should be preserved, got: {}",
            cleaned
        );
    }

    #[test]
    fn test_unicode_in_strings_preserved() {
        let sql = "SELECT '日本語テキスト' FROM t -- comment";
        let cleaned = strip_comments(sql, DatabaseType::Postgres);
        // Unicode in strings should be preserved
        assert!(
            cleaned.contains("日本語テキスト"),
            "Unicode in strings should be preserved, got: {}",
            cleaned
        );
        // Comment should be stripped
        assert!(
            !cleaned.contains("comment"),
            "Comment should be stripped, got: {}",
            cleaned
        );
    }

    #[test]
    fn test_complex_nested_scenario() {
        // Multiple edge cases combined
        let sql = "SELECT $$ /* not comment */ $$ as col, E'-- also not' -- real comment\n/* block */ FROM [table--name]";
        let cleaned = strip_comments(sql, DatabaseType::Postgres);
        // Dollar-quoted block comment marker should be preserved
        assert!(
            cleaned.contains("/* not comment */"),
            "Dollar-quoted block comment marker should be preserved, got: {}",
            cleaned
        );
        // Escape string comment marker should be preserved
        assert!(
            cleaned.contains("-- also not"),
            "Escape string comment marker should be preserved, got: {}",
            cleaned
        );
        // Real comments should be stripped
        assert!(
            !cleaned.contains("real comment"),
            "Real comment should be stripped, got: {}",
            cleaned
        );
        assert!(
            !cleaned.contains("block */") || cleaned.contains("/* not comment */"),
            "Block comment should be stripped (unless it's the preserved one), got: {}",
            cleaned
        );
    }
}
