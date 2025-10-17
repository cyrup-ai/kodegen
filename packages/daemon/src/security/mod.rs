//! Security module for Kodegen daemon
//!
//! This module provides comprehensive security functionality including:
//! - Zero-allocation vulnerability scanning
//! - Lock-free security metrics
//! - SIMD-accelerated pattern matching
//! - CI/CD integration for security validation

pub mod audit;
pub mod shell_executor;

pub use audit::*;
pub use shell_executor::*;
