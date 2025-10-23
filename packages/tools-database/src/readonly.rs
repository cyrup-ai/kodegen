//! Read-only SQL validation to prevent write operations

use crate::error::DatabaseError;
use crate::sql_parser::{extract_first_keyword, split_sql_statements};
use crate::types::DatabaseType;

/// Get allowed keywords for read-only queries based on database type
///
/// Keywords are based on common read-only operations per database.
/// Reference: https://github.com/cyrup-ai/kodegen/blob/main/tmp/dbhub/src/utils/allowed-keywords.ts
fn get_allowed_keywords(db_type: DatabaseType) -> &'static [&'static str] {
    match db_type {
        DatabaseType::Postgres => &[
            "select", "with", // CTEs are read-only
            "explain", "analyze", // Query analysis
            "show",    // Show settings/tables
        ],
        DatabaseType::MySQL | DatabaseType::MariaDB => &[
            "select", "with", "explain", "analyze", "show", "describe", // Describe tables
            "desc",     // Short form of describe
        ],
        DatabaseType::SQLite => &[
            "select", "with", "explain", "analyze", "pragma", // SQLite metadata queries
        ],
        DatabaseType::SqlServer => &[
            "select", "with", "explain", "showplan", // SQL Server query plans
        ],
    }
}

/// Validate that SQL contains only read-only operations
///
/// Checks ALL statements in multi-statement SQL. If any statement
/// contains a disallowed keyword, the entire validation fails.
///
/// # Examples
/// ```
/// // Allowed
/// validate_readonly_sql("SELECT * FROM users", DatabaseType::Postgres)?;
///
/// // Rejected
/// validate_readonly_sql("DROP TABLE users", DatabaseType::Postgres)?; // Error!
/// ```
pub fn validate_readonly_sql(sql: &str, db_type: DatabaseType) -> Result<(), DatabaseError> {
    let statements = split_sql_statements(sql, db_type)?;
    let allowed = get_allowed_keywords(db_type);

    for statement in statements {
        let keyword = extract_first_keyword(&statement, db_type)?;

        if !allowed.contains(&keyword.as_str()) {
            return Err(DatabaseError::ReadOnlyViolation(format!(
                "Keyword '{}' is not allowed in read-only mode for {}. Allowed keywords: {}",
                keyword,
                db_type,
                allowed.join(", ")
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allows_select() {
        assert!(validate_readonly_sql("SELECT 1", DatabaseType::Postgres).is_ok());
    }

    #[test]
    fn test_rejects_drop() {
        assert!(validate_readonly_sql("DROP TABLE t", DatabaseType::Postgres).is_err());
    }

    #[test]
    fn test_rejects_insert() {
        assert!(validate_readonly_sql("INSERT INTO t VALUES (1)", DatabaseType::Postgres).is_err());
    }

    #[test]
    fn test_validates_all_statements() {
        // First statement is fine, second is not
        let sql = "SELECT 1; DELETE FROM users";
        assert!(validate_readonly_sql(sql, DatabaseType::Postgres).is_err());
    }
}
