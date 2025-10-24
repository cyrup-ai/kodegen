//! Parser configuration

/// Template parser configuration
#[derive(Debug, Clone)]
pub struct ParserConfig {
    /// Maximum nesting depth for template structures
    pub max_depth: usize,
    /// Maximum number of tokens to parse
    pub max_tokens: usize,
    /// Whether to allow complex expressions in templates
    pub allow_expressions: bool,
    /// Whether to allow function calls in templates
    pub allow_functions: bool,
    /// Whether to require all variables to be explicitly defined
    pub strict_variables: bool,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            max_depth: 32,
            max_tokens: 10000,
            allow_expressions: true,
            allow_functions: true,
            strict_variables: false,
        }
    }
}
