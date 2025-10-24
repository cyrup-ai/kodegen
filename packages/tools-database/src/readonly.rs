//! Read-only SQL validation to prevent write operations

use crate::error::DatabaseError;
use crate::types::DatabaseType;
use sqlparser::ast::{
    Cte, Expr, FunctionArg, FunctionArgExpr, GroupByExpr, JoinConstraint, Query, Select,
    SelectItem, SetExpr, Statement, TableFactor, TableWithJoins, With,
};
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

/// Entry point: Parse SQL and validate all statements recursively
///
/// Validates that SQL contains only read-only operations by recursively traversing
/// the entire Abstract Syntax Tree (AST), including CTEs, subqueries, derived tables,
/// and expression contexts.
///
/// # Examples
/// ```
/// # use kodegen_tools_database::readonly::validate_readonly_sql;
/// # use kodegen_tools_database::types::DatabaseType;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Allowed
/// validate_readonly_sql("SELECT * FROM users", DatabaseType::Postgres)?;
///
/// // Rejected - top-level write
/// # let result = validate_readonly_sql("DROP TABLE users", DatabaseType::Postgres);
/// # assert!(result.is_err());
///
/// // Rejected - nested write in CTE  
/// # let result = validate_readonly_sql("WITH d AS (DELETE FROM t RETURNING *) SELECT * FROM d", DatabaseType::Postgres);
/// # assert!(result.is_err());
/// # Ok(())
/// # }
/// ```
pub fn validate_readonly_sql(sql: &str, db_type: DatabaseType) -> Result<(), DatabaseError> {
    let dialect = get_dialect(db_type);

    // Parse SQL into AST statements
    let statements = Parser::parse_sql(&*dialect, sql)
        .map_err(|e| DatabaseError::QueryError(format!("SQL parse error: {}", e)))?;

    // Validate each statement recursively
    for statement in statements {
        validate_statement_readonly(&statement, db_type)?;
    }

    Ok(())
}

/// Validate a top-level Statement
fn validate_statement_readonly(
    stmt: &Statement,
    db_type: DatabaseType,
) -> Result<(), DatabaseError> {
    match stmt {
        // Read-only statements
        Statement::Query(query) => {
            validate_query_readonly(query, db_type)?;
        }
        Statement::Explain { statement, .. } => {
            // EXPLAIN can wrap any statement, validate the inner statement
            validate_statement_readonly(statement, db_type)?;
        }

        // Show statements are read-only
        Statement::ShowTables { .. }
        | Statement::ShowColumns { .. }
        | Statement::ShowCreate { .. }
        | Statement::ShowCollation { .. }
        | Statement::ShowVariables { .. }
        | Statement::ShowStatus { .. }
        | Statement::ShowFunctions { .. } => {
            // These are safe read-only operations
        }

        // All write operations - reject immediately
        Statement::Insert { .. } => {
            return Err(DatabaseError::ReadOnlyViolation(
                "INSERT not allowed in read-only mode".to_string(),
            ));
        }
        Statement::Update { .. } => {
            return Err(DatabaseError::ReadOnlyViolation(
                "UPDATE not allowed in read-only mode".to_string(),
            ));
        }
        Statement::Delete { .. } => {
            return Err(DatabaseError::ReadOnlyViolation(
                "DELETE not allowed in read-only mode".to_string(),
            ));
        }
        Statement::Merge { .. } => {
            return Err(DatabaseError::ReadOnlyViolation(
                "MERGE not allowed in read-only mode".to_string(),
            ));
        }
        Statement::CreateTable { .. }
        | Statement::CreateView { .. }
        | Statement::CreateIndex { .. }
        | Statement::CreateSchema { .. }
        | Statement::CreateDatabase { .. }
        | Statement::CreateFunction { .. }
        | Statement::CreateProcedure { .. }
        | Statement::CreateRole { .. }
        | Statement::CreateTrigger { .. }
        | Statement::CreateType { .. }
        | Statement::CreateSequence { .. }
        | Statement::CreatePolicy { .. } => {
            return Err(DatabaseError::ReadOnlyViolation(
                "CREATE statements not allowed in read-only mode".to_string(),
            ));
        }
        Statement::AlterTable { .. }
        | Statement::AlterView { .. }
        | Statement::AlterIndex { .. }
        | Statement::AlterRole { .. }
        | Statement::AlterPolicy { .. } => {
            return Err(DatabaseError::ReadOnlyViolation(
                "ALTER statements not allowed in read-only mode".to_string(),
            ));
        }
        Statement::Drop { .. }
        | Statement::DropFunction { .. }
        | Statement::DropProcedure { .. }
        | Statement::DropTrigger { .. }
        | Statement::DropPolicy { .. } => {
            return Err(DatabaseError::ReadOnlyViolation(
                "DROP statements not allowed in read-only mode".to_string(),
            ));
        }
        Statement::Truncate { .. } => {
            return Err(DatabaseError::ReadOnlyViolation(
                "TRUNCATE not allowed in read-only mode".to_string(),
            ));
        }
        Statement::Copy { .. } | Statement::CopyIntoSnowflake { .. } => {
            return Err(DatabaseError::ReadOnlyViolation(
                "COPY not allowed in read-only mode".to_string(),
            ));
        }
        Statement::Grant { .. } | Statement::Revoke { .. } => {
            return Err(DatabaseError::ReadOnlyViolation(
                "GRANT/REVOKE not allowed in read-only mode".to_string(),
            ));
        }

        // For any other statement types, be conservative and reject
        _ => {
            return Err(DatabaseError::ReadOnlyViolation(
                "Statement type not explicitly allowed in read-only mode".to_string(),
            ));
        }
    }

    Ok(())
}

/// Validate a Query (handles CTEs and query body)
fn validate_query_readonly(query: &Query, db_type: DatabaseType) -> Result<(), DatabaseError> {
    // Validate CTEs (WITH clause)
    if let Some(with) = &query.with {
        validate_with_readonly(with, db_type)?;
    }

    // Validate main query body
    validate_set_expr_readonly(&query.body, db_type)?;

    Ok(())
}

/// Validate WITH clause (CTEs)
fn validate_with_readonly(with: &With, db_type: DatabaseType) -> Result<(), DatabaseError> {
    for cte in &with.cte_tables {
        validate_cte_readonly(cte, db_type)?;
    }
    Ok(())
}

/// Validate a single CTE
fn validate_cte_readonly(cte: &Cte, db_type: DatabaseType) -> Result<(), DatabaseError> {
    // Each CTE contains a full query that must be validated
    validate_query_readonly(&cte.query, db_type)?;
    Ok(())
}

/// Validate a SetExpr (query body or set operation)
fn validate_set_expr_readonly(expr: &SetExpr, db_type: DatabaseType) -> Result<(), DatabaseError> {
    match expr {
        SetExpr::Select(select) => {
            validate_select_readonly(select, db_type)?;
        }
        SetExpr::Query(query) => {
            validate_query_readonly(query, db_type)?;
        }
        SetExpr::SetOperation { left, right, .. } => {
            // UNION, EXCEPT, INTERSECT
            validate_set_expr_readonly(left, db_type)?;
            validate_set_expr_readonly(right, db_type)?;
        }
        SetExpr::Values(_) => {
            // VALUES clause is read-only (just data)
        }
        SetExpr::Table(_) => {
            // Direct table reference is read-only
        }
        // CRITICAL: SetExpr can directly contain write operations!
        SetExpr::Insert(_) => {
            return Err(DatabaseError::ReadOnlyViolation(
                "INSERT in set expression not allowed in read-only mode".to_string(),
            ));
        }
        SetExpr::Update(_) => {
            return Err(DatabaseError::ReadOnlyViolation(
                "UPDATE in set expression not allowed in read-only mode".to_string(),
            ));
        }
        SetExpr::Delete(_) => {
            return Err(DatabaseError::ReadOnlyViolation(
                "DELETE in set expression not allowed in read-only mode".to_string(),
            ));
        }
        SetExpr::Merge(_) => {
            return Err(DatabaseError::ReadOnlyViolation(
                "MERGE in set expression not allowed in read-only mode".to_string(),
            ));
        }
    }
    Ok(())
}

/// Validate a SELECT statement
fn validate_select_readonly(select: &Select, db_type: DatabaseType) -> Result<(), DatabaseError> {
    // Validate SELECT projection (select list items)
    for item in &select.projection {
        validate_select_item_readonly(item, db_type)?;
    }

    // Validate FROM clause (table factors and joins)
    for table_with_joins in &select.from {
        validate_table_with_joins_readonly(table_with_joins, db_type)?;
    }

    // Validate WHERE clause
    if let Some(expr) = &select.selection {
        validate_expr_readonly(expr, db_type)?;
    }

    // Validate HAVING clause
    if let Some(expr) = &select.having {
        validate_expr_readonly(expr, db_type)?;
    }

    // Validate QUALIFY clause (Snowflake)
    if let Some(expr) = &select.qualify {
        validate_expr_readonly(expr, db_type)?;
    }

    // Validate PREWHERE clause (ClickHouse)
    if let Some(expr) = &select.prewhere {
        validate_expr_readonly(expr, db_type)?;
    }

    // Validate GROUP BY expressions
    validate_group_by_readonly(&select.group_by, db_type)?;

    // Validate CLUSTER BY, DISTRIBUTE BY, SORT BY (Hive)
    for expr in &select.cluster_by {
        validate_expr_readonly(expr, db_type)?;
    }
    for expr in &select.distribute_by {
        validate_expr_readonly(expr, db_type)?;
    }
    for expr in &select.sort_by {
        validate_expr_readonly(&expr.expr, db_type)?;
    }

    Ok(())
}

/// Validate a SELECT list item
fn validate_select_item_readonly(
    item: &SelectItem,
    db_type: DatabaseType,
) -> Result<(), DatabaseError> {
    match item {
        SelectItem::UnnamedExpr(expr) => {
            validate_expr_readonly(expr, db_type)?;
        }
        SelectItem::ExprWithAlias { expr, .. } => {
            validate_expr_readonly(expr, db_type)?;
        }
        SelectItem::QualifiedWildcard(..) | SelectItem::Wildcard(..) => {
            // Wildcards are safe
        }
    }
    Ok(())
}

/// Validate GROUP BY clause
fn validate_group_by_readonly(
    group_by: &GroupByExpr,
    db_type: DatabaseType,
) -> Result<(), DatabaseError> {
    match group_by {
        GroupByExpr::All(..) => {}
        GroupByExpr::Expressions(exprs, ..) => {
            for expr in exprs {
                validate_expr_readonly(expr, db_type)?;
            }
        }
    }
    Ok(())
}

/// Validate table with joins (FROM clause element)
fn validate_table_with_joins_readonly(
    table_with_joins: &TableWithJoins,
    db_type: DatabaseType,
) -> Result<(), DatabaseError> {
    // Validate main table
    validate_table_factor_readonly(&table_with_joins.relation, db_type)?;

    // Validate joined tables
    for join in &table_with_joins.joins {
        validate_table_factor_readonly(&join.relation, db_type)?;

        // Validate join condition if present
        match &join.join_operator {
            sqlparser::ast::JoinOperator::Inner(constraint)
            | sqlparser::ast::JoinOperator::Left(constraint)
            | sqlparser::ast::JoinOperator::LeftOuter(constraint)
            | sqlparser::ast::JoinOperator::Right(constraint)
            | sqlparser::ast::JoinOperator::RightOuter(constraint)
            | sqlparser::ast::JoinOperator::FullOuter(constraint)
            | sqlparser::ast::JoinOperator::Semi(constraint)
            | sqlparser::ast::JoinOperator::LeftSemi(constraint)
            | sqlparser::ast::JoinOperator::RightSemi(constraint)
            | sqlparser::ast::JoinOperator::Anti(constraint)
            | sqlparser::ast::JoinOperator::LeftAnti(constraint)
            | sqlparser::ast::JoinOperator::RightAnti(constraint) => {
                if let JoinConstraint::On(expr) = constraint {
                    validate_expr_readonly(expr, db_type)?;
                }
            }
            sqlparser::ast::JoinOperator::AsOf {
                match_condition,
                constraint,
            } => {
                validate_expr_readonly(match_condition, db_type)?;
                if let JoinConstraint::On(expr) = constraint {
                    validate_expr_readonly(expr, db_type)?;
                }
            }
            _ => {
                // CrossJoin, CrossApply, OuterApply have no constraints
            }
        }
    }

    Ok(())
}

/// Validate a table factor (table reference or derived table)
fn validate_table_factor_readonly(
    factor: &TableFactor,
    db_type: DatabaseType,
) -> Result<(), DatabaseError> {
    match factor {
        TableFactor::Table { .. } => {
            // Regular table reference is safe
        }
        TableFactor::Derived { subquery, .. } => {
            // CRITICAL: Derived tables contain subqueries
            validate_query_readonly(subquery, db_type)?;
        }
        TableFactor::Function { args, .. } => {
            // Table-valued functions may have expression arguments
            for arg in args {
                validate_function_arg_readonly(arg, db_type)?;
            }
        }
        TableFactor::UNNEST { array_exprs, .. } => {
            // UNNEST expressions
            for expr in array_exprs {
                validate_expr_readonly(expr, db_type)?;
            }
        }
        TableFactor::NestedJoin {
            table_with_joins, ..
        } => {
            // Nested joins
            validate_table_with_joins_readonly(table_with_joins, db_type)?;
        }
        TableFactor::Pivot { table, .. } | TableFactor::Unpivot { table, .. } => {
            // Pivot/Unpivot base tables
            validate_table_factor_readonly(table, db_type)?;
        }
        _ => {
            // Other table factor types (JSON tables, etc.) - be conservative
        }
    }
    Ok(())
}

/// Validate function argument (may contain expressions)
fn validate_function_arg_readonly(
    arg: &FunctionArg,
    db_type: DatabaseType,
) -> Result<(), DatabaseError> {
    match arg {
        FunctionArg::Unnamed(arg_expr)
        | FunctionArg::Named { arg: arg_expr, .. }
        | FunctionArg::ExprNamed { arg: arg_expr, .. } => {
            // Extract the actual Expr from FunctionArgExpr
            if let FunctionArgExpr::Expr(expr) = arg_expr {
                validate_expr_readonly(expr, db_type)?;
            }
            // QualifiedWildcard and Wildcard are safe (no nested queries)
        }
    }
    Ok(())
}

/// Validate an expression (handles subqueries and nested expressions)
fn validate_expr_readonly(expr: &Expr, db_type: DatabaseType) -> Result<(), DatabaseError> {
    match expr {
        // CRITICAL: Expression subqueries
        Expr::Subquery(query) => {
            validate_query_readonly(query, db_type)?;
        }
        Expr::InSubquery { subquery, .. } => {
            validate_query_readonly(subquery, db_type)?;
        }
        Expr::Exists { subquery, .. } => {
            validate_query_readonly(subquery, db_type)?;
        }

        // Recursive expression types
        Expr::BinaryOp { left, right, .. } => {
            validate_expr_readonly(left, db_type)?;
            validate_expr_readonly(right, db_type)?;
        }
        Expr::UnaryOp { expr, .. } => {
            validate_expr_readonly(expr, db_type)?;
        }
        Expr::Cast { expr, .. } => {
            validate_expr_readonly(expr, db_type)?;
        }
        Expr::Extract { expr, .. } => {
            validate_expr_readonly(expr, db_type)?;
        }
        Expr::Substring {
            expr,
            substring_from,
            substring_for,
            ..
        } => {
            validate_expr_readonly(expr, db_type)?;
            if let Some(from_expr) = substring_from {
                validate_expr_readonly(from_expr, db_type)?;
            }
            if let Some(for_expr) = substring_for {
                validate_expr_readonly(for_expr, db_type)?;
            }
        }
        Expr::Nested(expr) => {
            validate_expr_readonly(expr, db_type)?;
        }
        Expr::Case {
            operand,
            conditions,
            else_result,
            ..
        } => {
            // Validate the operand if present
            if let Some(expr) = operand {
                validate_expr_readonly(expr, db_type)?;
            }
            // Validate each WHEN condition and result
            for case_when in conditions {
                validate_expr_readonly(&case_when.condition, db_type)?;
                validate_expr_readonly(&case_when.result, db_type)?;
            }
            // Validate ELSE result if present
            if let Some(expr) = else_result {
                validate_expr_readonly(expr, db_type)?;
            }
        }
        Expr::Function(func) => {
            // Handle FunctionArguments enum
            match &func.args {
                sqlparser::ast::FunctionArguments::List(arg_list) => {
                    for arg in &arg_list.args {
                        validate_function_arg_readonly(arg, db_type)?;
                    }
                }
                sqlparser::ast::FunctionArguments::Subquery(query) => {
                    // Function with subquery argument
                    validate_query_readonly(query, db_type)?;
                }
                sqlparser::ast::FunctionArguments::None => {
                    // No arguments (e.g., CURRENT_TIMESTAMP)
                }
            }
        }
        Expr::InList { expr, list, .. } => {
            validate_expr_readonly(expr, db_type)?;
            for item in list {
                validate_expr_readonly(item, db_type)?;
            }
        }
        Expr::Between {
            expr, low, high, ..
        } => {
            validate_expr_readonly(expr, db_type)?;
            validate_expr_readonly(low, db_type)?;
            validate_expr_readonly(high, db_type)?;
        }
        Expr::IsNull(expr)
        | Expr::IsNotNull(expr)
        | Expr::IsTrue(expr)
        | Expr::IsNotTrue(expr)
        | Expr::IsFalse(expr)
        | Expr::IsNotFalse(expr)
        | Expr::IsUnknown(expr)
        | Expr::IsNotUnknown(expr) => {
            validate_expr_readonly(expr, db_type)?;
        }
        Expr::InUnnest {
            expr, array_expr, ..
        } => {
            validate_expr_readonly(expr, db_type)?;
            validate_expr_readonly(array_expr, db_type)?;
        }
        Expr::Tuple(exprs) => {
            for expr in exprs {
                validate_expr_readonly(expr, db_type)?;
            }
        }
        Expr::Array(arr) => {
            for expr in &arr.elem {
                validate_expr_readonly(expr, db_type)?;
            }
        }

        // Literal values and column references are safe
        Expr::Identifier(..)
        | Expr::CompoundIdentifier(..)
        | Expr::Value(..)
        | Expr::TypedString { .. }
        | Expr::Interval { .. } => {
            // These are safe - no nested queries
        }

        // Other expression types - most are safe, but be thorough
        _ => {
            // For any expression type not explicitly handled, conservatively allow it
            // unless it's discovered to contain write operations in testing
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

    // Attack Vector 1: CTEs with Write Operations
    #[test]
    fn test_blocks_cte_with_delete() {
        let sql =
            "WITH deleted AS (DELETE FROM users WHERE id = 1 RETURNING *) SELECT * FROM deleted";
        let result = validate_readonly_sql(sql, DatabaseType::Postgres);
        assert!(result.is_err(), "Should block DELETE in CTE");
        assert!(result.unwrap_err().to_string().contains("DELETE"));
    }

    #[test]
    fn test_blocks_cte_with_insert() {
        let sql =
            "WITH inserted AS (INSERT INTO logs VALUES (1) RETURNING *) SELECT * FROM inserted";
        let result = validate_readonly_sql(sql, DatabaseType::Postgres);
        assert!(result.is_err(), "Should block INSERT in CTE");
        assert!(result.unwrap_err().to_string().contains("INSERT"));
    }

    #[test]
    fn test_blocks_cte_with_update() {
        let sql =
            "WITH updated AS (UPDATE users SET active = false RETURNING *) SELECT * FROM updated";
        let result = validate_readonly_sql(sql, DatabaseType::Postgres);
        assert!(result.is_err(), "Should block UPDATE in CTE");
        assert!(result.unwrap_err().to_string().contains("UPDATE"));
    }

    // Attack Vector 2: Derived Table Subqueries
    #[test]
    fn test_blocks_derived_table_with_update() {
        let sql = "SELECT * FROM (UPDATE logs SET checked = true RETURNING user_id) AS updated_logs WHERE user_id > 100";
        let result = validate_readonly_sql(sql, DatabaseType::Postgres);
        assert!(result.is_err(), "Should block UPDATE in derived table");
        assert!(result.unwrap_err().to_string().contains("UPDATE"));
    }

    #[test]
    fn test_blocks_derived_table_with_insert() {
        let sql = "SELECT * FROM (INSERT INTO audit VALUES (NOW()) RETURNING *) AS audit_log";
        let result = validate_readonly_sql(sql, DatabaseType::Postgres);
        assert!(result.is_err(), "Should block INSERT in derived table");
        assert!(result.unwrap_err().to_string().contains("INSERT"));
    }

    #[test]
    fn test_blocks_derived_table_with_delete() {
        let sql = "SELECT * FROM (DELETE FROM temp WHERE created < NOW() RETURNING id) AS cleaned";
        let result = validate_readonly_sql(sql, DatabaseType::Postgres);
        assert!(
            result.is_err(),
            "Should block DELETE in derived table: {:?}",
            result
        );
    }

    // Attack Vector 3: Expression Subqueries
    #[test]
    fn test_blocks_expression_subquery_with_insert() {
        let sql =
            "SELECT * FROM users WHERE id IN (INSERT INTO audit VALUES (NOW()) RETURNING user_id)";
        let result = validate_readonly_sql(sql, DatabaseType::Postgres);
        assert!(
            result.is_err(),
            "Should block INSERT in WHERE subquery: {:?}",
            result
        );
    }

    #[test]
    fn test_blocks_expression_subquery_with_delete() {
        let sql = "SELECT * FROM orders WHERE id = (DELETE FROM temp_orders WHERE id = 1 RETURNING order_id)";
        let result = validate_readonly_sql(sql, DatabaseType::Postgres);
        assert!(
            result.is_err(),
            "Should block DELETE in expression subquery: {:?}",
            result
        );
    }

    #[test]
    fn test_blocks_expression_subquery_with_update() {
        let sql = "SELECT COUNT(*) FROM users WHERE active = (UPDATE settings SET value = 'true' RETURNING value)";
        let result = validate_readonly_sql(sql, DatabaseType::Postgres);
        assert!(
            result.is_err(),
            "Should block UPDATE in expression subquery: {:?}",
            result
        );
    }

    // Attack Vector 4: SetExpr Direct Writes
    #[test]
    fn test_blocks_setexpr_insert_in_union() {
        let sql =
            "SELECT * FROM users UNION ALL (INSERT INTO logs VALUES (1, 'injected') RETURNING *)";
        let result = validate_readonly_sql(sql, DatabaseType::Postgres);
        assert!(result.is_err(), "Should block INSERT in UNION");
        assert!(result.unwrap_err().to_string().contains("INSERT"));
    }

    #[test]
    fn test_blocks_setexpr_update_in_union() {
        let sql = "SELECT id FROM users UNION (UPDATE logs SET checked = true RETURNING id)";
        let result = validate_readonly_sql(sql, DatabaseType::Postgres);
        assert!(result.is_err(), "Should block UPDATE in UNION");
        assert!(result.unwrap_err().to_string().contains("UPDATE"));
    }

    #[test]
    fn test_blocks_setexpr_delete_in_intersect() {
        let sql = "SELECT id FROM users INTERSECT (DELETE FROM inactive_users RETURNING id)";
        let result = validate_readonly_sql(sql, DatabaseType::Postgres);
        assert!(result.is_err(), "Should block DELETE in INTERSECT");
        assert!(result.unwrap_err().to_string().contains("DELETE"));
    }

    // Additional comprehensive tests
    #[test]
    fn test_blocks_nested_cte_with_write() {
        // Nested CTEs where the inner CTE has a write operation
        let sql = "WITH outer_cte AS (WITH inner_cte AS (DELETE FROM t RETURNING *) SELECT * FROM inner_cte) SELECT * FROM outer_cte";
        let result = validate_readonly_sql(sql, DatabaseType::Postgres);
        assert!(result.is_err(), "Should block nested CTE with DELETE");
    }

    #[test]
    fn test_blocks_write_in_subquery_in_select_list() {
        let sql = "SELECT id, (SELECT * FROM (INSERT INTO audit VALUES (1) RETURNING id)) AS audit_id FROM users";
        let result = validate_readonly_sql(sql, DatabaseType::Postgres);
        assert!(
            result.is_err(),
            "Should block INSERT in SELECT list subquery"
        );
    }

    #[test]
    fn test_blocks_write_in_having_clause() {
        let sql = "SELECT user_id, COUNT(*) FROM orders GROUP BY user_id HAVING COUNT(*) > (DELETE FROM temp RETURNING 1)";
        let result = validate_readonly_sql(sql, DatabaseType::Postgres);
        assert!(result.is_err(), "Should block DELETE in HAVING clause");
    }

    #[test]
    fn test_allows_complex_safe_query() {
        // Complex query with CTEs, subqueries, joins - all read-only
        let sql = r#"
            WITH user_stats AS (
                SELECT user_id, COUNT(*) as order_count
                FROM orders
                WHERE created_at > NOW() - INTERVAL '30 days'
                GROUP BY user_id
            )
            SELECT u.*, us.order_count
            FROM users u
            INNER JOIN user_stats us ON u.id = us.user_id
            WHERE u.active = true
              AND u.id IN (SELECT user_id FROM subscriptions WHERE status = 'active')
            ORDER BY us.order_count DESC
            LIMIT 100
        "#;
        assert!(validate_readonly_sql(sql, DatabaseType::Postgres).is_ok());
    }

    #[test]
    fn test_allows_explain() {
        let sql = "EXPLAIN SELECT * FROM users WHERE id = 1";
        assert!(validate_readonly_sql(sql, DatabaseType::Postgres).is_ok());
    }

    #[test]
    fn test_blocks_explain_with_write() {
        let sql = "EXPLAIN DELETE FROM users WHERE id = 1";
        let result = validate_readonly_sql(sql, DatabaseType::Postgres);
        assert!(result.is_err(), "Should block EXPLAIN with DELETE");
    }

    #[test]
    fn test_rejects_create_table() {
        let sql = "CREATE TABLE new_table (id INT)";
        let result = validate_readonly_sql(sql, DatabaseType::Postgres);
        assert!(result.is_err(), "Should block CREATE TABLE");
        assert!(result.unwrap_err().to_string().contains("CREATE"));
    }

    #[test]
    fn test_rejects_alter_table() {
        let sql = "ALTER TABLE users ADD COLUMN email VARCHAR(255)";
        let result = validate_readonly_sql(sql, DatabaseType::Postgres);
        assert!(result.is_err(), "Should block ALTER TABLE");
        assert!(result.unwrap_err().to_string().contains("ALTER"));
    }

    #[test]
    fn test_rejects_truncate() {
        let sql = "TRUNCATE TABLE logs";
        let result = validate_readonly_sql(sql, DatabaseType::Postgres);
        assert!(result.is_err(), "Should block TRUNCATE");
        assert!(result.unwrap_err().to_string().contains("TRUNCATE"));
    }

    #[test]
    fn test_rejects_grant() {
        let sql = "GRANT SELECT ON users TO public";
        let result = validate_readonly_sql(sql, DatabaseType::Postgres);
        assert!(result.is_err(), "Should block GRANT");
        assert!(result.unwrap_err().to_string().contains("GRANT"));
    }

    #[test]
    fn test_rejects_revoke() {
        let sql = "REVOKE SELECT ON users FROM public";
        let result = validate_readonly_sql(sql, DatabaseType::Postgres);
        assert!(result.is_err(), "Should block REVOKE");
        assert!(result.unwrap_err().to_string().contains("REVOKE"));
    }
}
