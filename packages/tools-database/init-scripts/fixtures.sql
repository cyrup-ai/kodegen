-- Universal fixture data compatible with ALL database types
-- Uses TRUE/FALSE for boolean compatibility (works everywhere)

-- Departments (5 records)
INSERT INTO departments (name, budget) VALUES
('Engineering', 2500000.00),
('Sales', 1800000.00),
('Marketing', 950000.00),
('Operations', 1200000.00),
('Human Resources', 650000.00);

-- Employees (15 records)
INSERT INTO employees (name, email, department_id, salary, hire_date, active) VALUES
('Alice Johnson', 'alice.johnson@company.com', 1, 125000.00, '2020-01-15', TRUE),
('Bob Smith', 'bob.smith@company.com', 1, 110000.00, '2020-03-22', TRUE),
('Carol White', 'carol.white@company.com', 1, 135000.00, '2019-07-10', TRUE),
('David Brown', 'david.brown@company.com', 2, 95000.00, '2021-02-01', TRUE),
('Emma Davis', 'emma.davis@company.com', 2, 102000.00, '2020-11-15', TRUE),
('Frank Miller', 'frank.miller@company.com', 2, 88000.00, '2022-01-10', FALSE),
('Grace Wilson', 'grace.wilson@company.com', 3, 78000.00, '2021-05-20', TRUE),
('Henry Moore', 'henry.moore@company.com', 3, 82000.00, '2021-08-12', TRUE),
('Ivy Taylor', 'ivy.taylor@company.com', 4, 92000.00, '2020-09-05', TRUE),
('Jack Anderson', 'jack.anderson@company.com', 4, 87000.00, '2021-12-01', TRUE),
('Kelly Thomas', 'kelly.thomas@company.com', 4, 95000.00, '2019-04-18', TRUE),
('Liam Jackson', 'liam.jackson@company.com', 5, 72000.00, '2022-03-15', TRUE),
('Mia Harris', 'mia.harris@company.com', 5, 76000.00, '2021-10-22', TRUE),
('Noah Martin', 'noah.martin@company.com', 1, 142000.00, '2018-06-01', TRUE),
('Olivia Garcia', 'olivia.garcia@company.com', 2, 115000.00, '2019-09-30', TRUE);

-- Projects (8 records)
INSERT INTO projects (name, department_id, start_date, end_date, status) VALUES
('Platform Redesign', 1, '2024-01-01', '2024-12-31', 'active'),
('Mobile App Launch', 1, '2024-03-15', NULL, 'active'),
('Q1 Sales Campaign', 2, '2024-01-01', '2024-03-31', 'completed'),
('Q2 Sales Campaign', 2, '2024-04-01', '2024-06-30', 'active'),
('Brand Refresh', 3, '2024-02-01', '2024-08-31', 'active'),
('Process Automation', 4, '2023-10-01', '2024-04-30', 'active'),
('Onboarding System', 5, '2024-01-15', '2024-05-15', 'active'),
('Legacy System Migration', 1, '2023-06-01', '2023-12-31', 'completed');

-- Employee-Project Assignments (20 records)
INSERT INTO employee_projects (employee_id, project_id, role) VALUES
(1, 1, 'Tech Lead'),
(2, 1, 'Backend Developer'),
(3, 2, 'Project Manager'),
(1, 2, 'Architect'),
(14, 1, 'Senior Engineer'),
(4, 3, 'Sales Lead'),
(5, 3, 'Account Manager'),
(15, 4, 'Sales Director'),
(4, 4, 'Regional Manager'),
(7, 5, 'Marketing Manager'),
(8, 5, 'Designer'),
(9, 6, 'Operations Lead'),
(10, 6, 'Process Analyst'),
(11, 6, 'Systems Engineer'),
(12, 7, 'HR Manager'),
(13, 7, 'Training Specialist'),
(2, 8, 'Backend Developer'),
(3, 8, 'Migration Lead'),
(14, 8, 'Database Specialist'),
(1, 6, 'Technical Advisor');

-- Audit Log (10 records)
INSERT INTO audit_log (table_name, record_id, action, changed_by) VALUES
('employees', 1, 'INSERT', 'system'),
('employees', 2, 'INSERT', 'system'),
('departments', 1, 'INSERT', 'system'),
('projects', 1, 'INSERT', 'admin'),
('employees', 6, 'UPDATE', 'hr_manager'),
('employee_projects', 1, 'INSERT', 'pm_alice'),
('projects', 3, 'UPDATE', 'sales_lead'),
('employees', 14, 'UPDATE', 'admin'),
('projects', 8, 'UPDATE', 'tech_lead'),
('departments', 3, 'UPDATE', 'finance');
