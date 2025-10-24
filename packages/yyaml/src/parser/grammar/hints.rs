//! Production-specific parsing hints and optimizations

use super::productions::Production;

/// Production-specific parsing hints and optimizations
pub struct ProductionHints;

impl ProductionHints {
    /// Get performance hints for production
    #[must_use]
    pub const fn get_hints(production: Production) -> ProductionOptimization {
        match production {
            Production::PlainScalar => ProductionOptimization {
                can_inline: true,
                zero_allocation: true,
                needs_lookahead: true,
                complexity: Complexity::Low,
            },
            Production::FlowSequence | Production::FlowMapping => ProductionOptimization {
                can_inline: false,
                zero_allocation: false,
                needs_lookahead: true,
                complexity: Complexity::Medium,
            },
            Production::BlockSequence | Production::BlockMapping => ProductionOptimization {
                can_inline: false,
                zero_allocation: false,
                needs_lookahead: true,
                complexity: Complexity::High,
            },
            _ => ProductionOptimization {
                can_inline: true,
                zero_allocation: true,
                needs_lookahead: false,
                complexity: Complexity::Low,
            },
        }
    }
}

/// Performance optimization information for productions
#[derive(Debug, Clone, Copy)]
pub struct ProductionOptimization {
    pub can_inline: bool,
    pub zero_allocation: bool,
    pub needs_lookahead: bool,
    pub complexity: Complexity,
}

/// Complexity classification for productions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Complexity {
    Low,
    Medium,
    High,
}
