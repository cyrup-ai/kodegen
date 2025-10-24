//! Complete YAML 1.2 grammar implementation
//!
//! This module provides comprehensive grammar rules, production definitions,
//! and parsing utilities for YAML 1.2 specification compliance.

// Re-export all grammar components
pub use self::{
    context_stack::ContextStack,
    context_types::{ChompingMode, ParseContext, YamlContext},
    grammar_impl::Grammar,
    hints::{Complexity, ProductionHints, ProductionOptimization},
    indicators::Indicators,
    parametric_context::ParametricContext,
    productions::{ParseError, ParseErrorKind, Production},
};

// Submodules
mod context_stack;
mod context_types;
mod grammar_impl;
mod hints;
mod indicators;
mod parametric_context;
mod productions;
