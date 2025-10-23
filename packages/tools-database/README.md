# Database Tools

Docker-based example demonstrating all 7 database tools across 4 database types.

## Quick Start

```bash
# 1. Start Docker containers
cd packages/tools-database
docker-compose up -d

# 2. Wait for health checks (20-30 seconds)
docker-compose ps  # All should show "healthy"

# 3. Run example
cd ../mcp-client
cargo run --example database

# 4. Stop containers
cd ../tools-database
docker-compose down
```

## Tools Demonstrated

1. **list_schemas** - List database schemas/databases
2. **list_tables** - List tables in a schema
3. **get_table_schema** - Get table column information
4. **get_table_indexes** - Get table indexes
5. **get_stored_procedures** - List stored procedures (PostgreSQL/MySQL only)
6. **execute_sql** - Execute SQL queries (SELECT, INSERT, UPDATE, DELETE)
7. **get_pool_stats** - Get connection pool health metrics

### Database Types

- **PostgreSQL 17** (port 5432) - SERIAL, BOOLEAN, plpgsql functions
- **MySQL 9.1** (port 3306) - AUTO_INCREMENT, TINYINT(1), MySQL functions
- **MariaDB 11.6** (port 3307) - AUTO_INCREMENT, TINYINT(1), MariaDB functions
- **SQLite** (file-based) - AUTOINCREMENT, INTEGER for boolean, no stored procedures

## Schema Overview

Universal schema with 2 tables modeling employee management:

**departments** (3 records):
- id (primary key)
- name (unique, varchar)
- budget (decimal)

**employees** (6 records):
- id (primary key)
- name (varchar)
- email (unique, varchar)
- department_id (foreign key to departments)
- salary (decimal)
- active (boolean/tinyint)
- INDEX on name

**Stored Procedures** (PostgreSQL/MySQL/MariaDB only):
- `get_department_employee_count(dept_id)` - Count active employees in department


## Connection Strings

```bash
# PostgreSQL
postgres://testuser:testpass@localhost:5432/testdb

# MySQL
mysql://testuser:testpass@localhost:3306/testdb

# MariaDB (different port to avoid conflict)
mysql://testuser:testpass@localhost:3307/testdb

# SQLite (file-based)
sqlite:///tmp/kodegen_test.db
```

## Example Output

```
Starting database tools example
Connected to server

======================================================================
 Testing POSTGRES
======================================================================

[1/8] Testing list_schemas...
✅ list_schemas completed

[2/8] Testing list_tables...
✅ list_tables completed

[3/8] Testing get_table_schema on 'employees' table...
✅ get_table_schema completed

[4/8] Testing get_table_indexes on 'employees' table...
✅ get_table_indexes completed

[5/8] Testing execute_sql with SELECT...
✅ execute_sql (SELECT) completed

[6/8] Testing execute_sql with JOIN...
✅ execute_sql (JOIN) completed

[7/8] Testing get_pool_stats...
✅ get_pool_stats completed

[8/8] Testing get_stored_procedures...
✅ get_stored_procedures completed

✅ All tests passed for postgres

... (similar output for MySQL, MariaDB, SQLite)

======================================================================
 ALL TESTS COMPLETE
======================================================================
✅ Successfully demonstrated all 7 database tools across 4 database types
```

## Troubleshooting

### Containers won't start

```bash
# Check if ports are already in use
lsof -i :5432
lsof -i :3306
lsof -i :3307

# View container logs
docker-compose logs postgres
docker-compose logs mysql
docker-compose logs mariadb
```

### Schema not loading

```bash
# Recreate containers with fresh data
docker-compose down -v  # -v removes volumes
docker-compose up -d
```

### Example fails to connect

```bash
# Verify containers are healthy
docker-compose ps

# Test connections manually
docker exec -it kodegen-test-postgres psql -U testuser -d testdb -c "SELECT COUNT(*) FROM employees;"
# Expected: 6

docker exec -it kodegen-test-mysql mysql -u testuser -ptestpass testdb -e "SELECT COUNT(*) FROM employees;"
# Expected: 6
```

## Architecture

### Tool Implementations

All 7 database tools are implemented in `packages/tools-database/src/tools/`:
- `execute_sql.rs` - SQL query execution with transaction support (575 lines)
- `list_schemas.rs` - Schema/database listing (178 lines)
- `list_tables.rs` - Table listing within schemas (227 lines)
- `get_table_schema.rs` - Column introspection
- `get_table_indexes.rs` - Index introspection
- `get_stored_procedures.rs` - Stored procedure listing
- `get_pool_stats.rs` - Connection pool health metrics (119 lines)

### Example Pattern

The example follows the established pattern from [filesystem.rs](../packages/mcp-client/examples/filesystem.rs):
1. Connect to server with specific tool category (`Database`)
2. Create logging client for JSONL output
3. Call each tool with appropriate arguments
4. Handle errors gracefully
5. Close connection cleanly

### Docker Infrastructure

- `docker-compose.yml` - Container orchestration with health checks
- `init-scripts/postgres.sql` - PostgreSQL-specific schema
- `init-scripts/mysql.sql` - MySQL-specific schema
- `init-scripts/mariadb.sql` - MariaDB-specific schema
- `init-scripts/fixtures.sql` - Universal INSERT statements

## Reference Implementation

See [tmp/dbhub](../tmp/dbhub) for reference patterns:
- Integration test base: [src/connectors/__tests__/shared/integration-test-base.ts](../tmp/dbhub/src/connectors/__tests__/shared/integration-test-base.ts)
- Test patterns: connection → schemas → tables → SQL execution → stored procedures
