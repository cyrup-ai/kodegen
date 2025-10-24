//! YAML 1.2 indicator classification

/// YAML 1.2 indicator classification
pub struct Indicators;

impl Indicators {
    /// Structure indicators
    pub const STRUCTURE: &'static [char] = &['-', '?', ':', ',', '[', ']', '{', '}'];

    /// Quoted scalar indicators
    pub const QUOTED: &'static [char] = &['\'', '"'];

    /// Block scalar indicators
    pub const BLOCK_SCALAR: &'static [char] = &['|', '>'];

    /// Directive indicators
    pub const DIRECTIVE: &'static [char] = &['%', '!'];

    /// Node property indicators
    pub const NODE_PROPERTY: &'static [char] = &['&', '*'];

    /// Reserved indicators
    pub const RESERVED: &'static [char] = &['@', '`'];

    /// Check if character is a YAML indicator
    #[inline]
    #[must_use]
    pub fn is_indicator(ch: char) -> bool {
        Self::STRUCTURE.contains(&ch)
            || Self::QUOTED.contains(&ch)
            || Self::BLOCK_SCALAR.contains(&ch)
            || Self::DIRECTIVE.contains(&ch)
            || Self::NODE_PROPERTY.contains(&ch)
            || Self::RESERVED.contains(&ch)
    }

    /// Check if character is a flow indicator
    #[inline]
    #[must_use]
    pub const fn is_flow_indicator(ch: char) -> bool {
        matches!(ch, ',' | '[' | ']' | '{' | '}')
    }

    /// Check if character is a block indicator
    #[inline]
    #[must_use]
    pub const fn is_block_indicator(ch: char) -> bool {
        matches!(ch, '-' | '?' | ':')
    }
}
