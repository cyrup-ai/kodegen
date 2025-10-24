//! YAML 1.2 parsing contexts and related types

/// YAML 1.2 parsing contexts for parametric productions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YamlContext {
    BlockIn,  // BLOCK-IN
    BlockOut, // BLOCK-OUT
    BlockKey, // BLOCK-KEY
    FlowIn,   // FLOW-IN
    FlowOut,  // FLOW-OUT
    FlowKey,  // FLOW-KEY
}

/// Block scalar chomping modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChompingMode {
    Strip, // Remove all trailing newlines
    Clip,  // Keep first trailing newline only
    Keep,  // Keep all trailing newlines
}

/// Context information for grammar-driven parsing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseContext {
    /// Root document context
    Document,
    /// Block collection context with indentation level
    BlockIn(usize),
    /// Flow collection context with nesting level
    FlowIn(usize),
    /// Block key context
    BlockKey,
    /// Flow key context
    FlowKey,
    /// Block value context
    BlockValue,
    /// Flow value context
    FlowValue,
}
