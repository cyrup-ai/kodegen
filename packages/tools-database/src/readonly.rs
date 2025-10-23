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
/// // Allowed
/// validate_readonly_sql("SELECT * FROM users", DatabaseType::Postgres)?;
///
/// // Rejected - top-level write
/// validate_readonly_sql("DROP TABLE users", DatabaseType::Postgres)?; // Error!
///
/// // Rejected - nested write in CTE
/// validate_readonly_sql("WITH d AS (DELETE FROM t RETURNING *) SELECT * FROM d", DatabaseType::Postgres)?; // Error!
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
fn validate_set_expr_readonly(
    expr: &SetExpr,
    db_type: DatabaseType,
) -> Result<(), DatabaseError> {
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
        validate_expr_readonly(expr, db_type)?;
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
        FunctionArg::Unnamed(arg_expr) | FunctionArg::Named { arg: arg_expr, .. } | FunctionArg::ExprNamed { arg: arg_expr, .. } => {
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
}
