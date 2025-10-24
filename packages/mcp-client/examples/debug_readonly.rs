use kodegen_tools_database::readonly::validate_readonly_sql;
use kodegen_tools_database::types::DatabaseType;

fn main() {
    // Test 1: Derived table with DELETE
    let sql1 = "SELECT * FROM (DELETE FROM temp WHERE created < NOW() RETURNING id) AS cleaned";
    println!("Test 1: Derived table with DELETE");
    println!("SQL: {}", sql1);
    match validate_readonly_sql(sql1, DatabaseType::Postgres) {
        Ok(_) => println!("❌ UNEXPECTEDLY ALLOWED\n"),
        Err(e) => println!("✓ Blocked with error: {}\n", e),
    }

    // Test 2: Expression subquery with INSERT
    let sql2 =
        "SELECT * FROM users WHERE id IN (INSERT INTO audit VALUES (NOW()) RETURNING user_id)";
    println!("Test 2: Expression subquery with INSERT");
    println!("SQL: {}", sql2);
    match validate_readonly_sql(sql2, DatabaseType::Postgres) {
        Ok(_) => println!("❌ UNEXPECTEDLY ALLOWED\n"),
        Err(e) => println!("✓ Blocked with error: {}\n", e),
    }

    // Test 3: Expression subquery with DELETE
    let sql3 =
        "SELECT * FROM orders WHERE id = (DELETE FROM temp_orders WHERE id = 1 RETURNING order_id)";
    println!("Test 3: Expression subquery with DELETE (scalar)");
    println!("SQL: {}", sql3);
    match validate_readonly_sql(sql3, DatabaseType::Postgres) {
        Ok(_) => println!("❌ UNEXPECTEDLY ALLOWED\n"),
        Err(e) => println!("✓ Blocked with error: {}\n", e),
    }

    // Test 4: Expression subquery with UPDATE
    let sql4 = "SELECT COUNT(*) FROM users WHERE active = (UPDATE settings SET value = 'true' RETURNING value)";
    println!("Test 4: Expression subquery with UPDATE (scalar)");
    println!("SQL: {}", sql4);
    match validate_readonly_sql(sql4, DatabaseType::Postgres) {
        Ok(_) => println!("❌ UNEXPECTEDLY ALLOWED\n"),
        Err(e) => println!("✓ Blocked with error: {}\n", e),
    }
}
