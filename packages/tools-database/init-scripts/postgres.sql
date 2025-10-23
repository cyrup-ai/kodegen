-- PostgreSQL-specific schema
CREATE TABLE departments (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) UNIQUE NOT NULL,
    budget DECIMAL(10,2)
);

CREATE TABLE employees (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    email VARCHAR(100) UNIQUE NOT NULL,
    department_id INTEGER REFERENCES departments(id),
    salary DECIMAL(10,2),
    active BOOLEAN DEFAULT true
);

CREATE INDEX idx_employees_name ON employees(name);

-- PostgreSQL stored procedure
CREATE OR REPLACE FUNCTION get_department_employee_count(dept_id INTEGER)
RETURNS INTEGER AS $$
BEGIN
    RETURN (SELECT COUNT(*) FROM employees WHERE department_id = dept_id AND active = true);
END;
$$ LANGUAGE plpgsql;
