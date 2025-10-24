//! Security validation configuration and logic

use serde::{Deserialize, Serialize};

use super::errors::ValidationError;
use super::language::CodeLanguage;

/// Security validation configuration for code execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Maximum code size in bytes
    pub max_code_size_bytes: usize,
    /// Prohibited patterns in code
    pub prohibited_patterns: Vec<String>,
    /// Allowed imports/modules
    pub allowed_imports: Vec<String>,
    /// Prohibited imports/modules  
    pub prohibited_imports: Vec<String>,
    /// Enable syntax validation
    pub syntax_validation: bool,
    /// Enable security scanning
    pub security_scanning: bool,
    /// Maximum nesting depth for code structures
    pub max_nesting_depth: u32,
    /// Maximum number of loops allowed
    pub max_loops: u32,
    /// Maximum function call depth
    pub max_call_depth: u32,
}

impl Default for ValidationConfig {
    /// Create default validation configuration
    #[inline]
    fn default() -> Self {
        Self {
            max_code_size_bytes: 64 * 1024, // 64KB
            prohibited_patterns: vec![
                "eval(".to_string(),
                "exec(".to_string(),
                "__import__".to_string(),
                "subprocess".to_string(),
                "os.system".to_string(),
            ],
            allowed_imports: Vec::new(),
            prohibited_imports: vec![
                "subprocess".to_string(),
                "os".to_string(),
                "sys".to_string(),
                "socket".to_string(),
                "urllib".to_string(),
                "requests".to_string(),
            ],
            syntax_validation: true,
            security_scanning: true,
            max_nesting_depth: 10,
            max_loops: 100,
            max_call_depth: 50,
        }
    }
}

impl ValidationConfig {
    /// Create secure Python validation configuration
    #[inline]
    #[must_use]
    pub fn python_secure() -> Self {
        Self {
            max_code_size_bytes: 128 * 1024, // 128KB
            prohibited_patterns: vec![
                "eval(".to_string(),
                "exec(".to_string(),
                "__import__".to_string(),
                "subprocess".to_string(),
                "os.system".to_string(),
                "open(".to_string(),
                "file(".to_string(),
                "input(".to_string(),
                "raw_input(".to_string(),
            ],
            allowed_imports: vec![
                "math".to_string(),
                "random".to_string(),
                "datetime".to_string(),
                "json".to_string(),
                "pandas".to_string(),
                "numpy".to_string(),
                "matplotlib".to_string(),
                "seaborn".to_string(),
            ],
            prohibited_imports: vec![
                "subprocess".to_string(),
                "os".to_string(),
                "sys".to_string(),
                "socket".to_string(),
                "urllib".to_string(),
                "requests".to_string(),
                "pickle".to_string(),
                "ctypes".to_string(),
            ],
            syntax_validation: true,
            security_scanning: true,
            max_nesting_depth: 8,
            max_loops: 50,
            max_call_depth: 30,
        }
    }

    /// Create secure JavaScript validation configuration
    #[inline]
    #[must_use]
    pub fn javascript_secure() -> Self {
        Self {
            max_code_size_bytes: 64 * 1024, // 64KB
            prohibited_patterns: vec![
                "eval(".to_string(),
                "Function(".to_string(),
                "require(".to_string(),
                "import(".to_string(),
                "fetch(".to_string(),
                "XMLHttpRequest".to_string(),
                "localStorage".to_string(),
                "sessionStorage".to_string(),
                "document".to_string(),
                "window".to_string(),
            ],
            allowed_imports: vec![
                "Math".to_string(),
                "Date".to_string(),
                "JSON".to_string(),
                "Array".to_string(),
                "Object".to_string(),
                "String".to_string(),
                "Number".to_string(),
            ],
            prohibited_imports: vec![
                "fs".to_string(),
                "http".to_string(),
                "https".to_string(),
                "net".to_string(),
                "child_process".to_string(),
                "cluster".to_string(),
                "crypto".to_string(),
            ],
            syntax_validation: true,
            security_scanning: true,
            max_nesting_depth: 6,
            max_loops: 30,
            max_call_depth: 20,
        }
    }

    /// Create secure Rust validation configuration
    #[inline]
    #[must_use]
    pub fn rust_secure() -> Self {
        Self {
            max_code_size_bytes: 256 * 1024, // 256KB
            prohibited_patterns: vec![
                "unsafe".to_string(),
                "std::process".to_string(),
                "std::fs".to_string(),
                "std::net".to_string(),
                "libc::".to_string(),
                "ptr::".to_string(),
                "mem::transmute".to_string(),
            ],
            allowed_imports: vec![
                "std::collections".to_string(),
                "std::iter".to_string(),
                "std::ops".to_string(),
                "std::fmt".to_string(),
                "std::str".to_string(),
                "std::vec".to_string(),
            ],
            prohibited_imports: vec![
                "std::process".to_string(),
                "std::fs".to_string(),
                "std::net".to_string(),
                "std::os".to_string(),
                "std::thread".to_string(),
                "std::sync".to_string(),
            ],
            syntax_validation: true,
            security_scanning: true,
            max_nesting_depth: 12,
            max_loops: 100,
            max_call_depth: 40,
        }
    }

    /// Create secure Go validation configuration
    #[inline]
    #[must_use]
    pub fn go_secure() -> Self {
        Self {
            max_code_size_bytes: 128 * 1024, // 128KB
            prohibited_patterns: vec![
                "os.Exec".to_string(),
                "os.Start".to_string(),
                "syscall.".to_string(),
                "unsafe.".to_string(),
                "reflect.".to_string(),
                "net/http".to_string(),
                "net/url".to_string(),
            ],
            allowed_imports: vec![
                "fmt".to_string(),
                "math".to_string(),
                "sort".to_string(),
                "strings".to_string(),
                "strconv".to_string(),
                "time".to_string(),
                "sync".to_string(),
            ],
            prohibited_imports: vec![
                "os".to_string(),
                "os/exec".to_string(),
                "net".to_string(),
                "net/http".to_string(),
                "syscall".to_string(),
                "unsafe".to_string(),
                "plugin".to_string(),
            ],
            syntax_validation: true,
            security_scanning: true,
            max_nesting_depth: 10,
            max_loops: 75,
            max_call_depth: 35,
        }
    }

    /// Create secure Bash validation configuration
    #[inline]
    #[must_use]
    pub fn bash_secure() -> Self {
        Self {
            max_code_size_bytes: 32 * 1024, // 32KB
            prohibited_patterns: vec![
                "rm -rf".to_string(),
                "chmod".to_string(),
                "chown".to_string(),
                "sudo".to_string(),
                "su ".to_string(),
                "passwd".to_string(),
                "curl".to_string(),
                "wget".to_string(),
                "nc ".to_string(),
                "netcat".to_string(),
                "> /dev".to_string(),
                "dd if=".to_string(),
                "mkfs".to_string(),
                "fdisk".to_string(),
            ],
            allowed_imports: Vec::new(),
            prohibited_imports: Vec::new(),
            syntax_validation: true,
            security_scanning: true,
            max_nesting_depth: 5,
            max_loops: 20,
            max_call_depth: 15,
        }
    }

    /// Validate code against this configuration
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` if:
    /// - Code size exceeds `max_code_size_bytes`
    /// - Security scanning is enabled and prohibited patterns are found
    /// - Language-specific validation fails
    #[inline]
    pub fn validate_code(
        &self,
        code: &str,
        language: &CodeLanguage,
    ) -> Result<(), ValidationError> {
        // Check code size
        if code.len() > self.max_code_size_bytes {
            return Err(ValidationError::code_size_exceeded(
                code.len(),
                self.max_code_size_bytes,
            ));
        }

        // Check prohibited patterns
        if self.security_scanning {
            let mut found_patterns = Vec::new();
            for pattern in &self.prohibited_patterns {
                if code.contains(pattern) {
                    found_patterns.push(pattern.clone());
                }
            }
            if !found_patterns.is_empty() {
                return Err(ValidationError::prohibited_patterns(found_patterns));
            }
        }

        // Language-specific validation
        match language {
            CodeLanguage::Python => self.validate_python_code(code),
            CodeLanguage::JavaScript => Self::validate_javascript_code(code),
            CodeLanguage::Rust => Self::validate_rust_code(code),
            CodeLanguage::Go => Self::validate_go_code(code),
            CodeLanguage::Bash => Self::validate_bash_code(code),
        }
    }

    /// Validate Python-specific patterns
    #[inline]
    fn validate_python_code(&self, code: &str) -> Result<(), ValidationError> {
        // Check for prohibited imports
        for line in code.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("import ") || trimmed.starts_with("from ") {
                for prohibited in &self.prohibited_imports {
                    if trimmed.contains(prohibited) {
                        return Err(ValidationError::security_failed(format!(
                            "Prohibited import: {prohibited}"
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    /// Validate JavaScript-specific patterns
    #[inline]
    fn validate_javascript_code(code: &str) -> Result<(), ValidationError> {
        // Check for Node.js requires
        if code.contains("require(") {
            return Err(ValidationError::security_failed(
                "Node.js require() is not allowed",
            ));
        }
        Ok(())
    }

    /// Validate Rust-specific patterns
    #[inline]
    fn validate_rust_code(code: &str) -> Result<(), ValidationError> {
        // Check for unsafe blocks
        if code.contains("unsafe") {
            return Err(ValidationError::security_failed(
                "Unsafe Rust code is not allowed",
            ));
        }
        Ok(())
    }

    /// Validate Go-specific patterns
    #[inline]
    fn validate_go_code(code: &str) -> Result<(), ValidationError> {
        // Check for dangerous syscalls
        if code.contains("syscall.") {
            return Err(ValidationError::security_failed(
                "Direct syscalls are not allowed",
            ));
        }
        Ok(())
    }

    /// Validate Bash-specific patterns
    #[inline]
    fn validate_bash_code(code: &str) -> Result<(), ValidationError> {
        // Check for dangerous file operations
        if code.contains("rm -rf /") {
            return Err(ValidationError::security_failed(
                "Destructive file operations are not allowed",
            ));
        }
        Ok(())
    }
}
