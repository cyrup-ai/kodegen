//! YAML 1.2 grammar rules and production definitions
//!
//! This module provides comprehensive grammar rules, production definitions,
//! and parsing utilities for YAML 1.2 specification compliance.

use crate::lexer::Position;

use super::context_types::{YamlContext, ChompingMode};

/// Parse error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub position: Position,
    pub message: String,
}

impl ParseError {
    pub fn new(kind: ParseErrorKind, position: Position, message: impl Into<String>) -> Self {
        Self {
            kind,
            position,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseErrorKind {
    LexicalError,
    UnexpectedToken,
    ExpectedToken,
    UnexpectedEndOfInput,
    RecursionLimitExceeded,
    UnexpectedState,
    InternalError,
}

/// YAML 1.2 grammar rules and production definitions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Production {
    // Keep existing non-parametric productions
    Stream,
    Document,
    ExplicitDocument,
    ImplicitDocument,
    DirectiveDocument,
    BareDocument,

    // Node productions
    Node,
    FlowNode,
    BlockNode,

    // Collection productions
    FlowSequence,
    FlowMapping,
    BlockSequence,
    BlockMapping,

    // Scalar productions
    PlainScalar,
    QuotedScalar,
    SingleQuotedScalar,
    DoubleQuotedScalar,
    LiteralScalar,
    FoldedScalar,

    // Property productions
    Properties,
    Tag,
    Anchor,

    // Flow productions
    FlowPair,
    FlowEntry,

    // Block productions
    BlockEntry,
    BlockPair,
    BlockKey,
    BlockValue,

    // Indicator productions
    Comment,
    Directive,
    Reserved,

    // Comment productions [75-79]
    CNbCommentText, // c-nb-comment-text
    BComment,       // b-comment
    SBComment,      // s-b-comment
    LComment,       // l-comment
    SLComments,     // s-l-comments

    // ADD parametric productions (grouped by category):

    // Indentation productions [YAML 1.2 spec productions 63-74]
    SIndent(i32),   // s-indent(n)
    SIndentLt(i32), // s-indent(<n)
    SIndentLe(i32), // s-indent(≤n)

    // Line prefix productions [76-79, 74]
    SLinePrefix(i32, YamlContext), // s-line-prefix(n,c)
    SBlockLinePrefix(i32),     // s-block-line-prefix(n)
    SFlowLinePrefix(i32),      // s-flow-line-prefix(n)
    SFlowFolded(i32),         // s-flow-folded(n)

    // Separation productions [80-81, 66]
    SSeparate(i32, YamlContext), // s-separate(n,c)
    SSeparateLines(i32),     // s-separate-lines(n)
    SSeparateInLine,          // s-separate-in-line

    // Empty productions [70-73, 72]
    LEmpty(i32, YamlContext),    // l-empty(n,c)
    BLTrimmed(i32, YamlContext), // b-l-trimmed(n,c)
    BLFolded(i32, YamlContext),  // b-l-folded(n,c)
    BAsSpace,                    // b-as-space

    // Block scalar productions [162-182]
    // Headers
    CIndentationIndicator,       // c-indentation-indicator
    CChompingIndicator,          // c-chomping-indicator
    BChompedLast,                // b-chomped-last
    LChompedEmpty(i32),          // l-chomped-empty(n)

    // Chomping
    LStripEmpty(i32),            // l-strip-empty(n)
    LKeepEmpty(i32),             // l-keep-empty(n)
    LTrailComments(i32),         // l-trail-comments(n)

    // Literal content
    LNbLiteralText(i32),         // l-nb-literal-text(n)
    BNbLiteralNext(i32),         // b-nb-literal-next(n)
    LLiteralContent(i32, ChompingMode), // l-literal-content(n,t)

    // Folded content
    SNbFoldedText(i32),          // s-nb-folded-text(n)
    LNbFoldedLines(i32),         // l-nb-folded-lines(n)
    SNbSpacedText(i32),          // s-nb-spaced-text(n)
    BLSpaced(i32),               // b-l-spaced(n)
    LNbSpacedLines(i32),         // l-nb-spaced-lines(n)
    LNbSameLines(i32),           // l-nb-same-lines(n)
    LNbDiffLines(i32),           // l-nb-diff-lines(n)
    LFoldedContent(i32, ChompingMode), // l-folded-content(n,t)

    CLBlockScalar(i32, ChompingMode), // c-l-block-scalar(n,t)
    CLLiteral(i32),                   // c-l+literal(n)
    CLFolded(i32),                    // c-l+folded(n)

    // Double-quoted scalar productions [105-121]
    // Foundation
    EScalar,                    // e-scalar
    ENode,                      // e-node
    NbDoubleChar,               // nb-double-char
    NsDoubleChar,               // ns-double-char
    CDoubleQuoted(i32, YamlContext), // c-double-quoted(n,c)
    
    // Line handling
    NbDoubleOneLine,             // nb-double-one-line
    SDoubleEscaped(i32),        // s-double-escaped(n)
    SDoubleBreak(i32),          // s-double-break(n)
    NbNsDoubleInLine,            // nb-ns-double-in-line
    SDoubleNextLine(i32),        // s-double-next-line(n)
    NbDoubleMultiLine(i32),     // nb-double-multi-line(n)

    // Single-quoted scalar productions [117-125]
    CQuotedQuote,               // c-quoted-quote
    NbSingleChar,               // nb-single-char
    NsSingleChar,               // ns-single-char
    CSingleQuoted(i32, YamlContext), // c-single-quoted(n,c)
    NbSingleOneLine,             // nb-single-one-line
    NbNsSingleInLine,            // nb-ns-single-in-line
    SSingleNextLine(i32),        // s-single-next-line(n)
    NbSingleMultiLine(i32),     // nb-single-multi-line(n)

    // Flow scalar productions [126-135]
    NSPlainFirst(YamlContext), // ns-plain-first(c)
    NSPlainSafe(YamlContext),  // ns-plain-safe(c)
    NSPlainSafeOut,            // ns-plain-safe-out
    NSPlainSafeIn,             // ns-plain-safe-in
    NSPlainChar(YamlContext),  // ns-plain-char(c)
    NSPlainOneLine(YamlContext), // ns-plain-one-line(c)
    NSPlainMultiLine(i32, YamlContext), // ns-plain-multi-line(n,c)
    NbNsPlainInLine(YamlContext), // nb-ns-plain-in-line(c)
    SNsPlainNextLine(i32, YamlContext), // s-ns-plain-next-line(n,c)

    // Flow collection productions [136-161]
    // Foundation
    InFlow(YamlContext),                // in-flow(c)
    NSSFlowMapEntries(i32, YamlContext), // ns-s-flow-map-entries(n,c)
    NSFlowMapEntry(i32, YamlContext),   // ns-flow-map-entry(n,c)
    
    // Explicit entries
    NSFlowMapExplicitEntry(i32, YamlContext), // ns-flow-map-explicit-entry(n,c)
    NSFlowMapImplicitEntry(i32, YamlContext), // ns-flow-map-implicit-entry(n,c)
    NSFlowMapYamlKeyEntry(i32, YamlContext),  // ns-flow-map-yaml-key-entry(n,c)
    CNsFlowMapEmptyKeyEntry(i32, YamlContext), // c-ns-flow-map-empty-key-entry(n,c)
    
    // Values & pairs
    CNsFlowMapSeparateValue(i32, YamlContext), // c-ns-flow-map-separate-value(n,c)
    CNsFlowMapJsonKeyEntry(i32, YamlContext),  // c-ns-flow-map-json-key-entry(n,c)
    CNsFlowMapAdjacentValue(i32, YamlContext), // c-ns-flow-map-adjacent-value(n,c)
    NSFlowPairEntry(i32, YamlContext),         // ns-flow-pair-entry(n,c)
    NSFlowPairYamlKeyEntry(i32, YamlContext),  // ns-flow-pair-yaml-key-entry(n,c)
    CNsFlowPairJsonKeyEntry(i32, YamlContext), // c-ns-flow-pair-json-key-entry(n,c)
    
    // Content & nodes
    NSSImplicitYamlKey(YamlContext),     // ns-s-implicit-yaml-key(c)
    CSImplicitJsonKey(YamlContext),      // c-s-implicit-json-key(c)
    NSFlowYamlContent(i32, YamlContext), // ns-flow-yaml-content(n,c)
    CFlowJsonContent(i32, YamlContext),  // c-flow-json-content(n,c)
    NSFlowContent(i32, YamlContext),     // ns-flow-content(n,c)
    NSFlowYamlNode(i32, YamlContext),    // ns-flow-yaml-node(n,c)
    CFlowJsonNode(i32, YamlContext),     // c-flow-json-node(n,c)
    
    CFlowSequence(i32, YamlContext), // c-flow-sequence(n,c)
    CFlowMapping(i32, YamlContext),  // c-flow-mapping(n,c)
    NSFlowSeqEntry(i32, YamlContext), // ns-flow-seq-entry(n,c)
    NSSFlowSeqEntries(i32, YamlContext), // ns-s-flow-seq-entries(n,c)
    NSFlowNode(i32, YamlContext), // ns-flow-node(n,c)
    NSFlowPair(i32, YamlContext),    // ns-flow-pair(n,c)

    // Block collection productions [183-201]
    LBlockSequence(i32),    // l+block-sequence(n)
    LBlockMapping(i32),     // l+block-mapping(n)
    NSLBlockMapEntry(i32),  // ns-l-block-map-entry(n)
    NSLCompactMapping(i32), // ns-l-compact-mapping(n)

    // Additional block collection productions
    CLBlockMapExplicitEntry(i32), // c-l-block-map-explicit-entry(n)
    CLBlockMapImplicitEntry(i32), // c-l-block-map-implicit-entry(n)
    NSLBlockMapExplicitValue(i32), // ns-l-block-map-explicit-value(n)

    // Block sequence entries
    CLBlockSeqEntry(i32),              // c-l-block-seq-entry(n)
    SLBlockIndented(i32, YamlContext), // s-l+block-indented(n,c)
    NSLCompactSequence(i32),           // ns-l-compact-sequence(n)

    // Block map explicit entries
    CLBlockMapExplicitKey(i32),     // c-l-block-map-explicit-key(n)
    LBlockMapExplicitValue(i32),    // l-block-map-explicit-value(n)

    // Block map implicit entries
    NSLBlockMapImplicitEntry(i32),  // ns-l-block-map-implicit-entry(n)
    NSSBlockMapImplicitKey,         // ns-s-block-map-implicit-key
    CLBlockMapImplicitValue(i32),   // c-l-block-map-implicit-value(n)

    // Block nodes & content
    SLBlockNode(i32, YamlContext),    // s-l+block-node(n,c)
    SLFlowInBlock(i32),               // s-l+flow-in-block(n)
    SLBlockInBlock(i32, YamlContext), // s-l+block-in-block(n,c)
    SLBlockScalar(i32, YamlContext),  // s-l+block-scalar(n,c)
    SLBlockCollection(i32, YamlContext), // s-l+block-collection(n,c)

    // Document productions
    LDocumentPrefix, // l-document-prefix
    CDirectivesEnd, // c-directives-end
    CDocumentEnd, // c-document-end
    LDocumentSuffix, // l-document-suffix
    CForbidden, // c-forbidden
    LBareDocument, // l-bare-document
    LExplicitDocument, // l-explicit-document
    LDirectiveDocument, // l-directive-document
    LAnyDocument, // l-any-document
    LYamlStream, // l-yaml-stream

    // Directive productions [82-95]
    // Directive structure
    LDirective,              // l-directive
    NSDirectiveName,         // ns-directive-name
    NSDirectiveParameter,    // ns-directive-parameter
    
    // YAML directives
    NSYamlDirective,         // ns-yaml-directive
    NSYamlVersion,           // ns-yaml-version
    
    // TAG directives
    NSTagDirective,          // ns-tag-directive
    CTagHandle,              // c-tag-handle
    CPrimaryTagHandle,       // c-primary-tag-handle
    CSecondaryTagHandle,     // c-secondary-tag-handle
    CNamedTagHandle,         // c-named-tag-handle
    NSTagPrefix,             // ns-tag-prefix
    CNsLocalTagPrefix,       // c-ns-local-tag-prefix
    NSGlobalTagPrefix,       // ns-global-tag-prefix
    
    NSReservedDirective,     // ns-reserved-directive

    // Additional block scalar
    CBBlockHeader(i32, ChompingMode), // c-b-block-header(m,t)
}

impl Production {
    /// Check if this production matches with given parameters
    #[must_use]
    pub fn matches(&self, indent: i32, context: YamlContext) -> bool {
        match self {
            Self::SIndent(n) => indent == *n,
            Self::SIndentLt(n) => indent < *n,
            Self::SIndentLe(n) => indent <= *n,
            Self::SLinePrefix(n, c) => indent == *n && context == *c,
            Self::SSeparate(n, c) => indent == *n && context == *c,
            Self::LBlockSequence(n) => indent >= *n,
            Self::LBlockMapping(n) => indent >= *n,
            Self::NSPlainFirst(c)
            | Self::NSPlainSafe(c)
            | Self::NSPlainChar(c)
            | Self::NSPlainOneLine(c) => context == *c,
            Self::NSPlainMultiLine(n, c) => indent >= *n && context == *c,
            Self::CLBlockMapExplicitEntry(n) => indent >= *n,
            Self::CLBlockMapImplicitEntry(n) => indent >= *n,
            Self::NSLBlockMapExplicitValue(n) => indent >= *n,
            Self::SFlowFolded(n) => indent >= *n,
            Self::CFlowSequence(n, c) | Self::CFlowMapping(n, c) => {
                indent >= *n && context == *c
            }
            Self::NSFlowSeqEntry(n, c)
            | Self::NSSFlowSeqEntries(n, c)
            | Self::NSFlowNode(n, c)
            | Self::NSFlowPair(n, c) => indent >= *n && context == *c,
            Self::CDoubleQuoted(n, c) => indent >= *n && context == *c,
            Self::SDoubleEscaped(n) => indent >= *n,
            Self::SDoubleBreak(n) => indent >= *n,
            Self::SDoubleNextLine(n) => indent >= *n,
            Self::NbDoubleMultiLine(n) => indent >= *n,
            Self::LChompedEmpty(n) => indent >= *n,
            Self::LStripEmpty(n) => indent >= *n,
            Self::LKeepEmpty(n) => indent >= *n,
            Self::LTrailComments(n) => indent >= *n,
            Self::LNbLiteralText(n) => indent >= *n,
            Self::BNbLiteralNext(n) => indent >= *n,
            Self::LLiteralContent(n, _) => indent >= *n,
            Self::SNbFoldedText(n) => indent >= *n,
            Self::LNbFoldedLines(n) => indent >= *n,
            Self::SNbSpacedText(n) => indent >= *n,
            Self::BLSpaced(n) => indent >= *n,
            Self::LNbSpacedLines(n) => indent >= *n,
            Self::LNbSameLines(n) => indent >= *n,
            Self::LNbDiffLines(n) => indent >= *n,
            Self::LFoldedContent(n, _) => indent >= *n,
            Self::CSingleQuoted(n, c) => indent >= *n && context == *c,
            Self::SSingleNextLine(n) => indent >= *n,
            Self::NbSingleMultiLine(n) => indent >= *n,
            Self::NbNsPlainInLine(c) => context == *c,
            Self::SNsPlainNextLine(n, c) => indent >= *n && context == *c,
            Self::InFlow(c) => context == *c,
            Self::NSSFlowMapEntries(n, c)
            | Self::NSFlowMapEntry(n, c)
            | Self::NSFlowMapExplicitEntry(n, c)
            | Self::NSFlowMapImplicitEntry(n, c)
            | Self::NSFlowMapYamlKeyEntry(n, c)
            | Self::CNsFlowMapEmptyKeyEntry(n, c)
            | Self::CNsFlowMapSeparateValue(n, c)
            | Self::CNsFlowMapJsonKeyEntry(n, c)
            | Self::CNsFlowMapAdjacentValue(n, c)
            | Self::NSFlowPairEntry(n, c)
            | Self::NSFlowPairYamlKeyEntry(n, c)
            | Self::CNsFlowPairJsonKeyEntry(n, c)
            | Self::NSFlowYamlContent(n, c)
            | Self::CFlowJsonContent(n, c)
            | Self::NSFlowContent(n, c)
            | Self::NSFlowYamlNode(n, c)
            | Self::CFlowJsonNode(n, c) => indent >= *n && context == *c,
            Self::NSSImplicitYamlKey(c)
            | Self::CSImplicitJsonKey(c) => context == *c,
            Self::CLBlockSeqEntry(n) => indent >= *n,
            Self::SLBlockIndented(n, c) => indent >= *n && context == *c,
            Self::NSLCompactSequence(n) => indent >= *n,
            Self::CLBlockMapExplicitKey(n) => indent >= *n,
            Self::LBlockMapExplicitValue(n) => indent >= *n,
            Self::NSLBlockMapImplicitEntry(n) => indent >= *n,
            Self::CLBlockMapImplicitValue(n) => indent >= *n,
            Self::SLBlockNode(n, c) => indent >= *n && context == *c,
            Self::SLFlowInBlock(n) => indent >= *n,
            Self::SLBlockInBlock(n, c) => indent >= *n && context == *c,
            Self::SLBlockScalar(n, c) => indent >= *n && context == *c,
            Self::SLBlockCollection(n, c) => indent >= *n && context == *c,
            // Non-parametric productions always match
            _ => true,
        }
    }

    /// Get the minimum indentation required by this production
    #[must_use]
    pub fn min_indent(&self) -> Option<i32> {
        match self {
            Self::SIndent(n) => Some(*n),
            Self::SIndentLt(_n) => Some(0), // Any indent less than n
            Self::SIndentLe(_n) => Some(0), // Any indent <= n
            Self::SBlockLinePrefix(n) => Some(*n),
            Self::LBlockSequence(n) => Some(n + 1), // Entries at n+1
            Self::LBlockMapping(n) => Some(n + 1),  // Keys at n+1
            Self::CLBlockMapExplicitEntry(n) => Some(*n),
            Self::CLBlockMapImplicitEntry(n) => Some(*n),
            Self::NSLBlockMapExplicitValue(n) => Some(*n),
            Self::SFlowFolded(n) => Some(*n),
            Self::CDoubleQuoted(n, _) => Some(*n),
            Self::SDoubleEscaped(n) => Some(*n),
            Self::SDoubleBreak(n) => Some(*n),
            Self::SDoubleNextLine(n) => Some(*n),
            Self::NbDoubleMultiLine(n) => Some(*n),
            Self::CSingleQuoted(n, _) => Some(*n),
            Self::SSingleNextLine(n) => Some(*n),
            Self::NbSingleMultiLine(n) => Some(*n),
            Self::SNsPlainNextLine(n, _) => Some(*n),
            Self::LChompedEmpty(n) => Some(*n),
            Self::LStripEmpty(n) => Some(*n),
            Self::LKeepEmpty(n) => Some(*n),
            Self::LTrailComments(n) => Some(*n),
            Self::LNbLiteralText(n) => Some(*n),
            Self::BNbLiteralNext(n) => Some(*n),
            Self::LLiteralContent(n, _) => Some(*n),
            Self::SNbFoldedText(n) => Some(*n),
            Self::LNbFoldedLines(n) => Some(*n),
            Self::SNbSpacedText(n) => Some(*n),
            Self::BLSpaced(n) => Some(*n),
            Self::LNbSpacedLines(n) => Some(*n),
            Self::LNbSameLines(n) => Some(*n),
            Self::LNbDiffLines(n) => Some(*n),
            Self::LFoldedContent(n, _) => Some(*n),
            Self::NSSFlowMapEntries(n, _)
            | Self::NSFlowMapEntry(n, _)
            | Self::NSFlowMapExplicitEntry(n, _)
            | Self::NSFlowMapImplicitEntry(n, _)
            | Self::NSFlowMapYamlKeyEntry(n, _)
            | Self::CNsFlowMapEmptyKeyEntry(n, _)
            | Self::CNsFlowMapSeparateValue(n, _)
            | Self::CNsFlowMapJsonKeyEntry(n, _)
            | Self::CNsFlowMapAdjacentValue(n, _)
            | Self::NSFlowPairEntry(n, _)
            | Self::NSFlowPairYamlKeyEntry(n, _)
            | Self::CNsFlowPairJsonKeyEntry(n, _)
            | Self::NSFlowYamlContent(n, _)
            | Self::CFlowJsonContent(n, _)
            | Self::NSFlowContent(n, _)
            | Self::NSFlowYamlNode(n, _)
            | Self::CFlowJsonNode(n, _) => Some(*n),
            Self::CLBlockSeqEntry(n) => Some(*n),
            Self::SLBlockIndented(n, _) => Some(*n),
            Self::NSLCompactSequence(n) => Some(*n),
            Self::CLBlockMapExplicitKey(n) => Some(*n),
            Self::LBlockMapExplicitValue(n) => Some(*n),
            Self::NSLBlockMapImplicitEntry(n) => Some(*n),
            Self::CLBlockMapImplicitValue(n) => Some(*n),
            Self::SLBlockNode(n, _) => Some(*n),
            Self::SLFlowInBlock(n) => Some(*n),
            Self::SLBlockInBlock(n, _) => Some(*n),
            Self::SLBlockScalar(n, _) => Some(*n),
            Self::SLBlockCollection(n, _) => Some(*n),
            _ => None,
        }
    }

    /// Check if production requires specific context
    #[must_use]
    pub const fn required_context(&self) -> Option<YamlContext> {
        match self {
            Self::SLinePrefix(_, c)
            | Self::SSeparate(_, c)
            | Self::LEmpty(_, c)
            | Self::BLTrimmed(_, c)
            | Self::BLFolded(_, c) => Some(*c),
            Self::NSPlainFirst(c)
            | Self::NSPlainSafe(c)
            | Self::NSPlainChar(c)
            | Self::NSPlainOneLine(c) => Some(*c),
            Self::NSPlainMultiLine(_, c) => Some(*c),
            Self::CFlowSequence(_, c)
            | Self::CFlowMapping(_, c)
            | Self::NSFlowSeqEntry(_, c)
            | Self::NSSFlowSeqEntries(_, c)
            | Self::NSFlowNode(_, c)
            | Self::NSFlowPair(_, c)
            | Self::CDoubleQuoted(_, c) => Some(*c),
            Self::NbNsPlainInLine(c)
            | Self::SNsPlainNextLine(_, c)
            | Self::CSingleQuoted(_, c) => Some(*c),
            Self::InFlow(c)
            | Self::NSSFlowMapEntries(_, c)
            | Self::NSFlowMapEntry(_, c)
            | Self::NSFlowMapExplicitEntry(_, c)
            | Self::NSFlowMapImplicitEntry(_, c)
            | Self::NSFlowMapYamlKeyEntry(_, c)
            | Self::CNsFlowMapEmptyKeyEntry(_, c)
            | Self::CNsFlowMapSeparateValue(_, c)
            | Self::CNsFlowMapJsonKeyEntry(_, c)
            | Self::CNsFlowMapAdjacentValue(_, c)
            | Self::NSFlowPairEntry(_, c)
            | Self::NSFlowPairYamlKeyEntry(_, c)
            | Self::CNsFlowPairJsonKeyEntry(_, c)
            | Self::NSSImplicitYamlKey(c)
            | Self::CSImplicitJsonKey(c)
            | Self::NSFlowYamlContent(_, c)
            | Self::CFlowJsonContent(_, c)
            | Self::NSFlowContent(_, c)
            | Self::NSFlowYamlNode(_, c)
            | Self::CFlowJsonNode(_, c) => Some(*c),
            Self::SLBlockIndented(_, c)
            | Self::SLBlockNode(_, c)
            | Self::SLBlockInBlock(_, c)
            | Self::SLBlockScalar(_, c)
            | Self::SLBlockCollection(_, c) => Some(*c),
            _ => None,
        }
    }
}
