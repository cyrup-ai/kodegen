//! Analysis context and configuration management for semantic processing
//!
//! Provides context tracking, configuration management, and state coordination
//! for semantic analysis operations with zero-allocation design.

use crate::lexer::Position;
use std::borrow::Cow;
use std::collections::HashMap;

/// Context for semantic analysis operations
#[derive(Debug, Clone)]
pub struct AnalysisContext<'input> {
    pub(crate) current_document_index: usize,
    pub(crate) processing_phase: ProcessingPhase,
    tag_prefixes: HashMap<Cow<'input, str>, Cow<'input, str>>,
    yaml_version: Option<(u32, u32)>,
    strict_mode: bool,
    cycle_detection_enabled: bool,
    current_position: Position,
}

/// Semantic processing phases for coordinated analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessingPhase {
    AnchorCollection,
    TagResolution,
    AliasResolution,
    DocumentValidation,
    FinalValidation,
}

impl<'input> AnalysisContext<'input> {
    /// Create new analysis context with default settings
    #[must_use] 
    pub fn new() -> Self {
        Self {
            current_document_index: 0,
            processing_phase: ProcessingPhase::AnchorCollection,
            tag_prefixes: Self::default_tag_prefixes(),
            yaml_version: None,
            strict_mode: false,
            cycle_detection_enabled: true,
            current_position: Position::default(),
        }
    }

    /// Create analysis context from semantic configuration
    #[must_use] 
    pub fn from_config(config: &SemanticConfig<'input>) -> Self {
        Self {
            current_document_index: 0,
            processing_phase: ProcessingPhase::AnchorCollection,
            tag_prefixes: Self::default_tag_prefixes(),
            yaml_version: config.yaml_version,
            strict_mode: config.strict_mode,
            cycle_detection_enabled: config.cycle_detection_enabled,
            current_position: Position::default(),
        }
    }

    /// Get default YAML 1.2 tag prefixes
    fn default_tag_prefixes() -> HashMap<Cow<'input, str>, Cow<'input, str>> {
        let mut prefixes = HashMap::with_capacity(2);
        prefixes.insert(Cow::Borrowed("!"), Cow::Borrowed("!"));
        prefixes.insert(Cow::Borrowed("!!"), Cow::Borrowed("tag:yaml.org,2002:"));
        prefixes
    }

    /// Check if in strict validation mode
    #[inline]
    #[must_use] 
    pub const fn is_strict(&self) -> bool {
        self.strict_mode
    }

    /// Check if cycle detection is enabled
    #[inline]
    #[must_use] 
    pub const fn cycle_detection_enabled(&self) -> bool {
        self.cycle_detection_enabled
    }

    /// Get YAML version
    #[inline]
    #[must_use] 
    pub const fn yaml_version(&self) -> Option<(u32, u32)> {
        self.yaml_version
    }

    /// Look up tag prefix
    #[must_use] 
    pub fn resolve_tag_prefix(&self, handle: &str) -> Option<&Cow<'input, str>> {
        self.tag_prefixes.get(handle)
    }

    /// Get tag handle prefix (alias for resolve_tag_prefix for compatibility)
    #[inline]
    #[must_use] 
    pub fn get_tag_handle(&self, handle: &str) -> Option<&Cow<'input, str>> {
        self.resolve_tag_prefix(handle)
    }

    /// Get current position in the document
    #[inline]
    #[must_use] 
    pub const fn current_position(&self) -> Position {
        self.current_position
    }

    /// Update current position during parsing
    #[inline]
    pub const fn set_position(&mut self, position: Position) {
        self.current_position = position;
    }

    /// Add custom tag prefix
    pub fn add_tag_prefix(&mut self, handle: Cow<'input, str>, prefix: Cow<'input, str>) {
        self.tag_prefixes.insert(handle, prefix);
    }

    /// Set YAML version for validation
    pub const fn set_yaml_version(&mut self, major: u32, minor: u32) {
        self.yaml_version = Some((major, minor));
    }

    /// Enable strict validation mode
    pub const fn enable_strict_mode(&mut self) {
        self.strict_mode = true;
    }

    /// Disable cycle detection (for performance in simple cases)
    pub const fn disable_cycle_detection(&mut self) {
        self.cycle_detection_enabled = false;
    }

    /// Get current processing phase
    #[inline]
    #[must_use] 
    pub const fn processing_phase(&self) -> ProcessingPhase {
        self.processing_phase
    }

    /// Get current document index
    #[inline]
    #[must_use] 
    pub const fn current_document_index(&self) -> usize {
        self.current_document_index
    }

    /// Register anchor metadata for owned processing (no-op for now)
    #[inline]
    pub fn register_anchor_metadata(&mut self, _anchor_name: String, _position: Position) {
        // For owned document processing, we collect metadata but don't store references
        // This is a placeholder method that can be expanded later if needed
    }

    /// Register tag metadata for owned processing (no-op for now)
    #[inline]
    pub fn register_tag_metadata(
        &mut self,
        _handle: Option<String>,
        _suffix: String,
        _position: Position,
    ) {
        // For owned document processing, we collect metadata but don't store references
        // This is a placeholder method that can be expanded later if needed
    }
}

impl<'input> Default for AnalysisContext<'input> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for semantic analysis
#[derive(Debug, Clone)]
pub struct SemanticConfig<'input> {
    pub strict_mode: bool,
    pub cycle_detection_enabled: bool,
    pub yaml_version: Option<(u32, u32)>,
    pub custom_tag_prefixes: HashMap<Cow<'input, str>, Cow<'input, str>>,
}

impl<'input> Default for SemanticConfig<'input> {
    fn default() -> Self {
        Self {
            strict_mode: false,
            cycle_detection_enabled: true,
            yaml_version: None,
            custom_tag_prefixes: HashMap::new(),
        }
    }
}

impl<'input> SemanticConfig<'input> {
    /// Create new configuration with strict mode enabled
    #[must_use] 
    pub fn strict() -> Self {
        Self {
            strict_mode: true,
            ..Self::default()
        }
    }

    /// Create configuration with cycle detection disabled for performance
    #[must_use] 
    pub fn fast() -> Self {
        Self {
            cycle_detection_enabled: false,
            ..Self::default()
        }
    }

    /// Set YAML version for validation
    #[must_use] 
    pub const fn with_yaml_version(mut self, major: u32, minor: u32) -> Self {
        self.yaml_version = Some((major, minor));
        self
    }

    /// Add custom tag prefix
    #[must_use] 
    pub fn with_tag_prefix(mut self, handle: Cow<'input, str>, prefix: Cow<'input, str>) -> Self {
        self.custom_tag_prefixes.insert(handle, prefix);
        self
    }

    /// Enable strict mode
    #[must_use] 
    pub const fn with_strict_mode(mut self) -> Self {
        self.strict_mode = true;
        self
    }

    /// Disable cycle detection
    #[must_use] 
    pub const fn without_cycle_detection(mut self) -> Self {
        self.cycle_detection_enabled = false;
        self
    }
}
