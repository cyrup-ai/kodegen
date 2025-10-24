//! Document structure validation and semantic correctness checking
//!
//! This module provides comprehensive document validation including structure
//! validation, constraint checking, and semantic correctness according to YAML 1.2.

pub mod analyzer;
pub mod constraints;
pub mod context;
pub mod fixes;
pub mod metrics;
pub mod rules;
pub mod validator;
pub mod warnings;

pub use analyzer::{PatternDetector, PatternImpact, PatternMatch, StructureAnalyzer};
pub use constraints::{ConstraintChecker, ConstraintRule, StructureConstraints, TypeConstraints};
pub use context::ValidationContext;
pub use fixes::{FixImpact, FixType, ValidationFix};
pub use metrics::{ComplexityMetrics, OptimizationDifficulty, OptimizationHint, OptimizationType};
pub use rules::{ValidationRule, ValidationRuleSet};
pub use validator::{DocumentValidator, ValidationStatistics};
pub use warnings::{ValidationWarning, ValidationWarningContext, WarningSeverity, WarningType};
