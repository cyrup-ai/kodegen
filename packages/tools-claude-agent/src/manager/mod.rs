//! Agent session management
//!
//! Provides `AgentManager` for spawning, monitoring, and controlling
//! multiple Claude agent sessions with circular message buffering,
//! working status detection, and automatic cleanup.
//!
//! # Module Structure
//!
//! - `agent_manager` - Core `AgentManager` with public API
//! - `session` - Session state structures
//! - `commands` - Command protocol for agent communication
//! - `background` - Background task spawning
//! - `helpers` - Pure helper functions for message processing

mod agent_manager;
mod background;
mod commands;
mod helpers;
mod session;

pub use agent_manager::{AgentManager, SpawnSessionRequest};
