//! Conditional expression parsing for macro logic

use super::types::CondToken;

/// Runtime value types for condition evaluation
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CondValue {
    /// Numeric value (floating point)
    Number(f64),
    /// String value
    String(String),
    /// Boolean value
    Boolean(bool),
}

impl std::fmt::Display for CondValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CondValue::Number(n) => write!(f, "{n}"),
            CondValue::String(s) => write!(f, "{s}"),
            CondValue::Boolean(b) => write!(f, "{b}"),
        }
    }
}

impl CondValue {
    /// Parse a string into the appropriate value type
    pub(crate) fn parse(s: &str) -> Self {
        // Try boolean literals first
        match s.to_lowercase().as_str() {
            "true" => return CondValue::Boolean(true),
            "false" => return CondValue::Boolean(false),
            _ => {}
        }

        // Try to parse as number
        if let Ok(num) = s.parse::<f64>() {
            return CondValue::Number(num);
        }

        // Default to string
        CondValue::String(s.to_string())
    }

    /// Convert value to boolean for logical operations
    pub(crate) fn as_bool(&self) -> bool {
        match self {
            CondValue::Boolean(b) => *b,
            CondValue::Number(n) => *n != 0.0 && !n.is_nan(),
            CondValue::String(s) => !s.is_empty(),
        }
    }

    /// Test equality with type-aware comparison
    fn equals(&self, other: &Self) -> bool {
        match (self, other) {
            // Both numbers: numeric comparison with epsilon
            (CondValue::Number(a), CondValue::Number(b)) => (a - b).abs() < f64::EPSILON,
            // Both booleans: boolean comparison
            (CondValue::Boolean(a), CondValue::Boolean(b)) => a == b,
            // Both strings: string comparison
            (CondValue::String(a), CondValue::String(b)) => a == b,
            // Mixed types: convert both to string and compare
            _ => self.to_string() == other.to_string(),
        }
    }

    /// Compare values with type checking
    fn compare(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            // Numeric comparison
            (CondValue::Number(a), CondValue::Number(b)) => a.partial_cmp(b),
            // String comparison
            (CondValue::String(a), CondValue::String(b)) => Some(a.cmp(b)),
            // Boolean comparison (false < true)
            (CondValue::Boolean(a), CondValue::Boolean(b)) => Some(a.cmp(b)),
            // Mixed types: cannot compare
            _ => None,
        }
    }
}

/// Parse a string literal with escape sequences
fn parse_string_literal(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut value = String::new();
    while let Some(&ch) = chars.peek() {
        if ch == '"' {
            chars.next();
            break;
        }
        if ch == '\\' {
            // Escape sequence
            chars.next();
            if let Some(&escaped) = chars.peek() {
                match escaped {
                    'n' => value.push('\n'),
                    't' => value.push('\t'),
                    '\\' => value.push('\\'),
                    '"' => value.push('"'),
                    _ => {
                        value.push('\\');
                        value.push(escaped);
                    }
                }
                chars.next();
            }
        } else {
            value.push(ch);
            chars.next();
        }
    }
    value
}

/// Parse an identifier or unquoted value
fn parse_identifier(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut value = String::new();
    while let Some(&ch) = chars.peek() {
        // Stop at whitespace or operator characters
        if ch.is_whitespace() || "()!<>=&|\"".contains(ch) {
            break;
        }
        value.push(ch);
        chars.next();
    }
    value
}

/// Tokenize a condition string into tokens
pub(crate) fn tokenize_condition(input: &str) -> Vec<CondToken> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            // Skip whitespace
            ' ' | '\t' | '\n' | '\r' => {
                chars.next();
            }

            // Parentheses
            '(' => {
                tokens.push(CondToken::LParen);
                chars.next();
            }
            ')' => {
                tokens.push(CondToken::RParen);
                chars.next();
            }

            // NOT operator or inequality
            '!' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(CondToken::Neq);
                } else {
                    tokens.push(CondToken::Not);
                }
            }

            // Equality operator
            '=' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(CondToken::Eq);
                } else {
                    // Single = not supported, treat as ==
                    tokens.push(CondToken::Eq);
                }
            }

            // Less than or less-equal
            '<' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(CondToken::Leq);
                } else {
                    tokens.push(CondToken::Lt);
                }
            }

            // Greater than or greater-equal
            '>' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(CondToken::Geq);
                } else {
                    tokens.push(CondToken::Gt);
                }
            }

            // AND operator
            '&' => {
                chars.next();
                if chars.peek() == Some(&'&') {
                    chars.next();
                    tokens.push(CondToken::And);
                }
                // Single & is invalid, skip
            }

            // OR operator
            '|' => {
                chars.next();
                if chars.peek() == Some(&'|') {
                    chars.next();
                    tokens.push(CondToken::Or);
                }
                // Single | is invalid, skip
            }

            // String literal
            '"' => {
                chars.next(); // consume opening quote
                let string_value = parse_string_literal(&mut chars);
                tokens.push(CondToken::Value(string_value));
            }

            // Identifier or unquoted value
            _ => {
                let identifier = parse_identifier(&mut chars);
                if !identifier.is_empty() {
                    tokens.push(CondToken::Value(identifier));
                }
            }
        }
    }

    tokens
}

/// Recursive descent parser for condition expressions
pub(crate) struct CondParser {
    tokens: Vec<CondToken>,
    pos: usize,
}

impl CondParser {
    /// Create a new parser with the given tokens
    pub(crate) fn new(tokens: Vec<CondToken>) -> Self {
        CondParser { tokens, pos: 0 }
    }

    /// Get the current token without consuming it
    fn current(&self) -> Option<&CondToken> {
        self.tokens.get(self.pos)
    }

    /// Advance to the next token
    fn advance(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    /// Check if current token matches and consume it if so
    fn match_token(&mut self, expected: &CondToken) -> bool {
        if let Some(current) = self.current()
            && std::mem::discriminant(current) == std::mem::discriminant(expected)
        {
            self.advance();
            return true;
        }
        false
    }

    /// Parse OR expression (lowest precedence)
    pub(crate) fn parse_or(&mut self) -> CondValue {
        let mut left = self.parse_and();

        while self.match_token(&CondToken::Or) {
            let right = self.parse_and();
            left = CondValue::Boolean(left.as_bool() || right.as_bool());
        }

        left
    }

    /// Parse AND expression
    fn parse_and(&mut self) -> CondValue {
        let mut left = self.parse_not();

        while self.match_token(&CondToken::And) {
            let right = self.parse_not();
            left = CondValue::Boolean(left.as_bool() && right.as_bool());
        }

        left
    }

    /// Parse NOT expression
    fn parse_not(&mut self) -> CondValue {
        if self.match_token(&CondToken::Not) {
            let value = self.parse_comparison();
            CondValue::Boolean(!value.as_bool())
        } else {
            self.parse_comparison()
        }
    }

    /// Parse comparison expression
    fn parse_comparison(&mut self) -> CondValue {
        let left = self.parse_primary();

        // Check for comparison operator
        if let Some(current) = self.current() {
            match current {
                CondToken::Eq => {
                    self.advance();
                    let right = self.parse_primary();
                    return CondValue::Boolean(left.equals(&right));
                }
                CondToken::Neq => {
                    self.advance();
                    let right = self.parse_primary();
                    return CondValue::Boolean(!left.equals(&right));
                }
                CondToken::Lt => {
                    self.advance();
                    let right = self.parse_primary();
                    if let Some(ordering) = left.compare(&right) {
                        return CondValue::Boolean(ordering == std::cmp::Ordering::Less);
                    }
                    return CondValue::Boolean(false);
                }
                CondToken::Gt => {
                    self.advance();
                    let right = self.parse_primary();
                    if let Some(ordering) = left.compare(&right) {
                        return CondValue::Boolean(ordering == std::cmp::Ordering::Greater);
                    }
                    return CondValue::Boolean(false);
                }
                CondToken::Leq => {
                    self.advance();
                    let right = self.parse_primary();
                    if let Some(ordering) = left.compare(&right) {
                        return CondValue::Boolean(ordering != std::cmp::Ordering::Greater);
                    }
                    return CondValue::Boolean(false);
                }
                CondToken::Geq => {
                    self.advance();
                    let right = self.parse_primary();
                    if let Some(ordering) = left.compare(&right) {
                        return CondValue::Boolean(ordering != std::cmp::Ordering::Less);
                    }
                    return CondValue::Boolean(false);
                }
                _ => {}
            }
        }

        // No comparison operator, return value as-is
        left
    }

    /// Parse primary expression (values and parentheses)
    fn parse_primary(&mut self) -> CondValue {
        // Handle parentheses
        if self.match_token(&CondToken::LParen) {
            let value = self.parse_or(); // Restart at lowest precedence
            self.match_token(&CondToken::RParen); // Consume closing paren (optional)
            return value;
        }

        // Handle values
        if let Some(CondToken::Value(s)) = self.current() {
            let s = s.clone();
            self.advance();
            return CondValue::parse(&s);
        }

        // Error case: unexpected token or end
        CondValue::Boolean(false)
    }
}
