//! Code execution tool definitions with zero allocation patterns

use serde::{Deserialize, Serialize};

use super::errors::ValidationError;
use super::language::CodeLanguage;
use super::limits::ResourceLimits;
use super::request::CodeExecutionRequest;
use super::validation::ValidationConfig;

/// Code execution tool for secure multi-language code execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeExecutionTool {
    /// Tool name (owned string allocated once)
    pub name: String,
    /// Programming language for execution
    pub language: CodeLanguage,
    /// Tool description (owned string allocated once)
    pub description: String,
    /// Example usage demonstrating the tool (owned string allocated once)
    pub example_usage: String,
    /// Security validation settings
    pub validation_config: ValidationConfig,
    /// Resource limits for execution
    pub resource_limits: ResourceLimits,
    /// Tool version for compatibility tracking
    pub version: String,
    /// Whether tool is experimental
    pub experimental: bool,
}

impl CodeExecutionTool {
    /// Create a new code execution tool with essential fields
    #[inline]
    pub fn new(
        name: impl Into<String>,
        language: CodeLanguage,
        description: impl Into<String>,
        example_usage: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            language,
            description: description.into(),
            example_usage: example_usage.into(),
            validation_config: ValidationConfig::default(),
            resource_limits: ResourceLimits::default(),
            version: "1.0.0".to_string(),
            experimental: false,
        }
    }

    /// Python analysis tool constructor for data analysis and calculations
    #[inline]
    #[must_use]
    pub fn python_analysis() -> Self {
        Self::new(
            "python_analysis",
            CodeLanguage::Python,
            "Execute Python code for data analysis, calculations, and scientific computing",
            r#"import pandas as pd
import numpy as np

# Load and analyze data
df = pd.read_csv('data.csv')
print(df.describe())
print(f"Average: {df['column'].mean()}")"#,
        )
        .with_validation_config(ValidationConfig::python_secure())
        .with_resource_limits(ResourceLimits::analysis_workload())
    }

    /// JavaScript processing tool constructor for JSON processing and API calls
    #[inline]
    #[must_use]
    pub fn javascript_processing() -> Self {
        Self::new(
            "javascript_processing",
            CodeLanguage::JavaScript,
            "Execute JavaScript code for JSON processing, API calls, and data transformation",
            r#"const data = {
    users: [
        { name: "Alice", age: 30 },
        { name: "Bob", age: 25 }
    ]
};

const averageAge = data.users.reduce((sum, user) => sum + user.age, 0) / data.users.length;
console.log(`Average age: ${averageAge}`);"#,
        )
        .with_validation_config(ValidationConfig::javascript_secure())
        .with_resource_limits(ResourceLimits::processing_workload())
    }

    /// Rust computation tool constructor for performance-critical calculations
    #[inline]
    #[must_use]
    pub fn rust_computation() -> Self {
        Self::new(
            "rust_computation",
            CodeLanguage::Rust,
            "Execute Rust code for performance-critical calculations and system programming",
            r#"fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn main() {
    let result = fibonacci(20);
    println!("Fibonacci(20) = {}", result);
}"#,
        )
        .with_validation_config(ValidationConfig::rust_secure())
        .with_resource_limits(ResourceLimits::computation_workload())
    }

    /// Go processing tool constructor for concurrent processing tasks
    #[inline]
    #[must_use]
    pub fn go_processing() -> Self {
        Self::new(
            "go_processing",
            CodeLanguage::Go,
            "Execute Go code for concurrent processing, networking, and system tasks",
            r#"package main

import (
    "fmt"
    "sync"
    "time"
)

func worker(id int, wg *sync.WaitGroup) {
    defer wg.Done()
    fmt.Printf("Worker %d starting\n", id)
    time.Sleep(time.Second)
    fmt.Printf("Worker %d done\n", id)
}

func main() {
    var wg sync.WaitGroup
    for i := 1; i <= 3; i++ {
        wg.Add(1)
        go worker(i, &wg)
    }
    wg.Wait()
    fmt.Println("All workers completed")
}"#,
        )
        .with_validation_config(ValidationConfig::go_secure())
        .with_resource_limits(ResourceLimits::processing_workload())
    }

    /// Bash automation tool constructor for system automation and file operations
    #[inline]
    #[must_use]
    pub fn bash_automation() -> Self {
        Self::new(
            "bash_automation",
            CodeLanguage::Bash,
            "Execute Bash scripts for system automation, file operations, and command orchestration",
            r#"#!/bin/bash

# Process files in directory
for file in *.txt; do
    if [ -f "$file" ]; then
        echo "Processing: $file"
        wc -l "$file"
    fi
done

echo "File processing complete""#,
        )
        .with_validation_config(ValidationConfig::bash_secure())
        .with_resource_limits(ResourceLimits::automation_workload())
    }

    /// Set validation configuration - builder pattern for fluent API
    #[inline]
    #[must_use]
    pub fn with_validation_config(mut self, config: ValidationConfig) -> Self {
        self.validation_config = config;
        self
    }

    /// Set resource limits - builder pattern for fluent API
    #[inline]
    #[must_use]
    pub fn with_resource_limits(mut self, limits: ResourceLimits) -> Self {
        self.resource_limits = limits;
        self
    }

    /// Set version - builder pattern for fluent API
    #[inline]
    #[must_use]
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Mark as experimental - builder pattern for fluent API
    #[inline]
    #[must_use]
    pub fn experimental(mut self) -> Self {
        self.experimental = true;
        self
    }

    /// Validate code against tool's security configuration
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` if code contains prohibited patterns or violates security rules
    #[inline]
    pub fn validate_code(&self, code: &str) -> Result<(), ValidationError> {
        self.validation_config.validate_code(code, &self.language)
    }

    /// Check if code execution would exceed resource limits
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` if resource limits would be exceeded
    #[inline]
    pub fn check_resource_limits(
        &self,
        request: &CodeExecutionRequest,
    ) -> Result<(), ValidationError> {
        self.resource_limits.validate_request(request)
    }
}
