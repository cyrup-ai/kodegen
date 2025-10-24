//! YAML parser with state machine architecture
//!
//! This module provides YAML 1.2 parsing with a single state machine implementation.

pub mod ast;
pub mod character_productions;
pub mod flow;
pub mod grammar;
pub mod indentation;
pub mod loader;
pub mod state_machine;
pub mod structural_productions;

pub use ast::*;
pub use character_productions::CharacterProductions;
pub use flow::FlowProductions;
pub use grammar::{ChompingMode, ParametricContext, YamlContext};
pub use loader::YamlLoader;
pub use state_machine::{State, StateMachine};
