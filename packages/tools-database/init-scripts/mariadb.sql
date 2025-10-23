-- MariaDB schema (nearly identical to MySQL)
CREATE TABLE departments (
    id INT AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(100) UNIQUE NOT NULL,
    budget DECIMAL(10,2)
);

CREATE TABLE employees (
    id INT AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    email VARCHAR(100) UNIQUE NOT NULL,
    department_id INT,
    salary DECIMAL(10,2),
    active TINYINT(1) DEFAULT 1,
    FOREIGN KEY (department_id) REFERENCES departments(id),
    INDEX idx_employees_name (name)
);

-- MariaDB stored procedure
DELIMITER $$
CREATE FUNCTION get_department_employee_count(dept_id INT)
RETURNS INT
DETERMINISTIC
BEGIN
    RETURN (SELECT COUNT(*) FROM employees WHERE department_id = dept_id AND active = 1);
END$$
DELIMITER ;
