//! Grammar validation and decision utilities

use super::context_types::{YamlContext, ParseContext};
use super::parametric_context::ParametricContext;
use super::productions::Production;
use super::productions::{ParseError, ParseErrorKind};
use crate::lexer::{Position, TokenKind};

/// Grammar validation and decision utilities
pub struct Grammar;

impl Grammar {
    /// Check if token can start a document
    #[inline]
    #[must_use]
    pub const fn can_start_document(token: &TokenKind) -> bool {
        matches!(
            token,
            TokenKind::DocumentStart
                | TokenKind::YamlDirective { .. }
                | TokenKind::TagDirective { .. }
                | TokenKind::ReservedDirective { .. }
                | TokenKind::Scalar { .. }
                | TokenKind::FlowSequenceStart
                | TokenKind::FlowMappingStart
                | TokenKind::BlockEntry
                | TokenKind::Anchor(_)
                | TokenKind::Alias(_)
                | TokenKind::Tag { .. }
        )
    }

    /// Check if token can start a node
    #[inline]
    #[must_use]
    pub const fn can_start_node(token: &TokenKind) -> bool {
        matches!(
            token,
            TokenKind::Scalar { .. }
                | TokenKind::FlowSequenceStart
                | TokenKind::FlowMappingStart
                | TokenKind::BlockEntry
                | TokenKind::Anchor(_)
                | TokenKind::Alias(_)
                | TokenKind::Tag { .. }
        )
    }

    /// Check if token can start a flow collection
    #[inline]
    #[must_use]
    pub const fn can_start_flow_collection(token: &TokenKind) -> bool {
        matches!(
            token,
            TokenKind::FlowSequenceStart | TokenKind::FlowMappingStart
        )
    }

    /// Check if token can start a block collection
    #[inline]
    #[must_use]
    pub const fn can_start_block_collection(token: &TokenKind) -> bool {
        matches!(token, TokenKind::BlockEntry | TokenKind::Key)
    }

    /// Check if token is a scalar
    #[inline]
    #[must_use]
    pub const fn is_scalar(token: &TokenKind) -> bool {
        matches!(token, TokenKind::Scalar { .. })
    }

    /// Check if token is a property (anchor or tag)
    #[inline]
    #[must_use]
    pub const fn is_property(token: &TokenKind) -> bool {
        matches!(token, TokenKind::Anchor(_) | TokenKind::Tag { .. })
    }

    /// Check if token can continue a plain scalar in given context
    #[must_use]
    pub const fn can_continue_plain_scalar(token: &TokenKind, context: &ParseContext) -> bool {
        match context {
            ParseContext::FlowIn(_) | ParseContext::FlowKey | ParseContext::FlowValue => {
                // In flow context, plain scalars are more restricted
                match token {
                    TokenKind::FlowEntry
                    | TokenKind::FlowSequenceEnd
                    | TokenKind::FlowMappingEnd
                    | TokenKind::Value => false,
                    TokenKind::Scalar { .. } => true,
                    _ => false,
                }
            }
            ParseContext::BlockIn(_) | ParseContext::BlockKey | ParseContext::BlockValue => {
                // In block context, plain scalars can continue until certain indicators
                match token {
                    TokenKind::Value
                    | TokenKind::BlockEntry
                    | TokenKind::Comment(_)
                    | TokenKind::DocumentStart
                    | TokenKind::DocumentEnd => false,
                    TokenKind::Scalar { .. } => true,
                    _ => false,
                }
            }
            ParseContext::Document => {
                // At document level, very permissive
                !matches!(
                    token,
                    TokenKind::DocumentStart | TokenKind::DocumentEnd | TokenKind::StreamEnd
                )
            }
        }
    }

    /// Determine if a mapping key-value pair is implicit (no explicit value indicator)
    #[must_use]
    pub const fn is_implicit_mapping_value(context: &ParseContext) -> bool {
        matches!(context, ParseContext::BlockIn(_))
    }

    /// Check if indentation is valid for block context
    #[inline]
    #[must_use]
    pub const fn is_valid_block_indentation(
        current_indent: usize,
        context_indent: usize,
        strict: bool,
    ) -> bool {
        if strict {
            current_indent > context_indent
        } else {
            current_indent >= context_indent
        }
    }

    /// Determine production rule for current parsing state
    pub fn determine_production(
        token: &TokenKind,
        context: &ParseContext,
        lookahead: Option<&TokenKind>,
    ) -> Result<Production, ParseError> {
        match context {
            ParseContext::Document => match token {
                TokenKind::DocumentStart => Ok(Production::ExplicitDocument),
                TokenKind::YamlDirective { .. }
                | TokenKind::TagDirective { .. }
                | TokenKind::ReservedDirective { .. } => Ok(Production::DirectiveDocument),
                _ if Self::can_start_node(token) => Ok(Production::BareDocument),
                _ => Err(ParseError::new(
                    ParseErrorKind::UnexpectedToken,
                    Position::start(),
                    format!("unexpected token in document context: {token:?}"),
                )),
            },

            ParseContext::FlowIn(_) => {
                match token {
                    TokenKind::FlowSequenceStart => Ok(Production::FlowSequence),
                    TokenKind::FlowMappingStart => Ok(Production::FlowMapping),
                    TokenKind::Scalar { .. } => {
                        // Check if this could be a flow mapping key
                        if matches!(lookahead, Some(TokenKind::Value)) {
                            Ok(Production::FlowPair)
                        } else {
                            Ok(Production::PlainScalar)
                        }
                    }
                    TokenKind::Anchor(_) | TokenKind::Tag { .. } => Ok(Production::Properties),
                    TokenKind::Alias(_) => Ok(Production::Node),
                    _ => Ok(Production::FlowNode),
                }
            }

            ParseContext::BlockIn(_) => {
                match token {
                    TokenKind::BlockEntry => Ok(Production::BlockSequence),
                    TokenKind::Key => Ok(Production::BlockMapping),
                    TokenKind::Scalar { .. } => {
                        // Check if this could be a block mapping key
                        if matches!(lookahead, Some(TokenKind::Value)) {
                            Ok(Production::BlockPair)
                        } else {
                            Ok(Production::PlainScalar)
                        }
                    }
                    TokenKind::Anchor(_) | TokenKind::Tag { .. } => Ok(Production::Properties),
                    TokenKind::Alias(_) => Ok(Production::Node),
                    _ => Ok(Production::BlockNode),
                }
            }

            ParseContext::BlockKey | ParseContext::FlowKey => match token {
                TokenKind::Scalar { .. } => Ok(Production::PlainScalar),
                TokenKind::FlowSequenceStart => Ok(Production::FlowSequence),
                TokenKind::FlowMappingStart => Ok(Production::FlowMapping),
                TokenKind::Anchor(_) | TokenKind::Tag { .. } => Ok(Production::Properties),
                TokenKind::Alias(_) => Ok(Production::Node),
                _ => Err(ParseError::new(
                    ParseErrorKind::UnexpectedToken,
                    Position::start(),
                    format!("unexpected token in key context: {token:?}"),
                )),
            },

            ParseContext::BlockValue | ParseContext::FlowValue => match token {
                TokenKind::Scalar { .. } => Ok(Production::PlainScalar),
                TokenKind::FlowSequenceStart => Ok(Production::FlowSequence),
                TokenKind::FlowMappingStart => Ok(Production::FlowMapping),
                TokenKind::BlockEntry => Ok(Production::BlockSequence),
                TokenKind::Anchor(_) | TokenKind::Tag { .. } => Ok(Production::Properties),
                TokenKind::Alias(_) => Ok(Production::Node),
                _ => Ok(Production::Node),
            },
        }
    }

    /// Validate scalar style is appropriate for context
    pub fn validate_scalar_style(
        style: crate::lexer::ScalarStyle,
        context: &ParseContext,
        value: &str,
    ) -> Result<(), ParseError> {
        use crate::lexer::ScalarStyle;

        match style {
            ScalarStyle::Plain => {
                // Plain scalars have context-dependent restrictions
                match context {
                    ParseContext::FlowIn(_) | ParseContext::FlowKey | ParseContext::FlowValue => {
                        // Check for flow indicators in plain scalar
                        if value.contains(&[',', '[', ']', '{', '}'][..]) {
                            return Err(ParseError::new(
                                ParseErrorKind::UnexpectedToken,
                                Position::start(),
                                "plain scalar contains flow indicators in flow context",
                            ));
                        }
                    }
                    _ => {
                        // Block context restrictions
                        if value.starts_with(&['-', '?', ':', '|', '>', '@', '`'][..]) {
                            return Err(ParseError::new(
                                ParseErrorKind::UnexpectedToken,
                                Position::start(),
                                "plain scalar starts with reserved indicator",
                            ));
                        }
                    }
                }
            }
            ScalarStyle::SingleQuoted | ScalarStyle::DoubleQuoted => {
                // Quoted scalars are generally safe
            }
            ScalarStyle::Literal | ScalarStyle::Folded => {
                // Block scalars only allowed in block context
                match context {
                    ParseContext::FlowIn(_) | ParseContext::FlowKey | ParseContext::FlowValue => {
                        return Err(ParseError::new(
                            ParseErrorKind::UnexpectedToken,
                            Position::start(),
                            "block scalar not allowed in flow context",
                        ));
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    /// Check if context allows simple keys
    #[inline]
    #[must_use]
    pub const fn allows_simple_keys(context: &ParseContext) -> bool {
        matches!(context, ParseContext::FlowIn(_) | ParseContext::BlockIn(_))
    }

    /// Check if context requires explicit key indicators
    #[inline]
    #[must_use]
    pub const fn requires_explicit_key(context: &ParseContext) -> bool {
        matches!(context, ParseContext::FlowIn(_))
    }

    /// Determine next context after parsing a production
    #[must_use]
    pub fn next_context(
        current: &ParseContext,
        production: Production,
        indent: Option<usize>,
    ) -> ParseContext {
        match (current, production) {
            (ParseContext::Document, Production::ExplicitDocument) => ParseContext::BlockIn(0),
            (ParseContext::Document, Production::BareDocument) => ParseContext::BlockIn(0),

            (ParseContext::BlockIn(level), Production::FlowSequence) => {
                ParseContext::FlowIn(*level + 1)
            }
            (ParseContext::BlockIn(level), Production::FlowMapping) => {
                ParseContext::FlowIn(*level + 1)
            }
            (ParseContext::BlockIn(_), Production::BlockSequence) => {
                ParseContext::BlockIn(indent.unwrap_or(0))
            }
            (ParseContext::BlockIn(_), Production::BlockMapping) => {
                ParseContext::BlockIn(indent.unwrap_or(0))
            }

            (ParseContext::FlowIn(level), Production::FlowSequence) => {
                ParseContext::FlowIn(*level + 1)
            }
            (ParseContext::FlowIn(level), Production::FlowMapping) => {
                ParseContext::FlowIn(*level + 1)
            }

            (ParseContext::BlockIn(_level), Production::BlockKey) => ParseContext::BlockKey,
            (ParseContext::BlockIn(_level), Production::BlockValue) => ParseContext::BlockValue,
            (ParseContext::FlowIn(_level), Production::FlowPair) => ParseContext::FlowKey,

            // Default: maintain current context
            _ => current.clone(),
        }
    }

    /// Check if production is valid in current parametric context
    #[must_use]
    pub fn is_valid_parametric_production(
        production: &Production,
        context: &ParametricContext,
    ) -> bool {
        production.matches(context.current_indent(), context.current_context)
    }

    /// Get applicable productions for current parametric context
    #[must_use]
    pub fn applicable_parametric_productions(context: &ParametricContext) -> Vec<Production> {
        let mut productions = Vec::new();
        let indent = context.current_indent();

        match context.current_context {
            YamlContext::BlockOut | YamlContext::BlockIn => {
                productions.push(Production::LBlockSequence(indent));
                productions.push(Production::LBlockMapping(indent));
                productions.push(Production::SIndent(indent));
                productions.push(Production::SBlockLinePrefix(indent));
            }
            YamlContext::FlowOut | YamlContext::FlowIn => {
                productions.push(Production::CFlowSequence(indent, context.current_context));
                productions.push(Production::CFlowMapping(indent, context.current_context));
                productions.push(Production::SFlowLinePrefix(indent));
            }
            YamlContext::BlockKey | YamlContext::FlowKey => {
                productions.push(Production::NSPlainFirst(context.current_context));
                productions.push(Production::NSPlainSafe(context.current_context));
            }
        }

        productions
    }

    /// Convert between parametric context and existing parse context for compatibility
    #[must_use]
    pub fn parametric_to_parse_context(parametric: &ParametricContext) -> ParseContext {
        parametric.to_parse_context()
    }

    /// Determine next parametric context after parsing a production
    #[must_use]
    pub fn next_parametric_context(
        current: &ParametricContext,
        production: Production,
        _new_indent: Option<i32>,
    ) -> YamlContext {
        match (current.current_context, production) {
            (YamlContext::BlockOut, Production::Document) => YamlContext::BlockIn,
            (YamlContext::BlockIn, Production::LBlockSequence(_)) => YamlContext::BlockIn,
            (YamlContext::BlockIn, Production::LBlockMapping(_)) => YamlContext::BlockIn,
            (YamlContext::BlockIn, Production::CFlowSequence(_, _)) => YamlContext::FlowIn,
            (YamlContext::BlockIn, Production::CFlowMapping(_, _)) => YamlContext::FlowIn,
            (YamlContext::FlowIn, Production::CFlowSequence(_, _)) => YamlContext::FlowIn,
            (YamlContext::FlowIn, Production::CFlowMapping(_, _)) => YamlContext::FlowIn,
            (YamlContext::BlockIn, Production::BlockKey) => YamlContext::BlockKey,
            (YamlContext::BlockKey, Production::BlockValue) => YamlContext::BlockIn,
            (YamlContext::FlowIn, Production::FlowPair) => YamlContext::FlowKey,
            (YamlContext::FlowKey, Production::FlowEntry) => YamlContext::FlowIn,
            // Default: maintain current context
            _ => current.current_context,
        }
    }
}
