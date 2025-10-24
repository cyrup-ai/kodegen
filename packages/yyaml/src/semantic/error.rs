//! Semantic analysis error types and handling
//!
//! Provides comprehensive error types for semantic analysis with precise location
//! tracking and detailed error messages for debugging and user feedback.

use crate::lexer::Position;

/// Semantic analysis errors with precise location tracking
#[derive(Debug, Clone, PartialEq)]
pub enum SemanticError {
    UnresolvedAlias {
        alias_name: String,
        position: Position,
    },
    CircularReference {
        alias_name: String,
        path: String,
        position: Position,
    },
    DuplicateAnchor {
        anchor_name: String,
        first_position: Position,
        duplicate_position: Position,
    },
    InvalidTagHandle {
        handle: String,
        position: Position,
    },
    UnknownTag {
        tag: String,
        position: Position,
    },
    TagResolutionFailed {
        tag: String,
        reason: String,
        position: Position,
    },
    ValidationDepthExceeded {
        max_depth: usize,
        current_depth: usize,
        position: Position,
    },
    UnknownTagHandle {
        handle: String,
        position: Position,
    },
    CustomTagResolutionFailed {
        tag: String,
        error: String,
        position: Position,
    },
    UnknownCustomTag {
        tag: String,
        position: Position,
    },
    InvalidDocumentStructure {
        reason: String,
        position: Position,
    },
    TypeMismatch {
        expected: String,
        actual: String,
        position: Position,
    },
    ValueValidationFailed {
        value: String,
        constraint: String,
        position: Position,
    },
    ReferenceTrackingError {
        reason: String,
        position: Position,
    },
    AnchorRegistrationFailed {
        anchor_name: String,
        reason: String,
        position: Position,
    },
    ValidationError {
        message: String,
        position: Position,
    },
    InternalError {
        message: String,
        position: Position,
    },
    ConflictingAnchor {
        anchor_name: String,
        first_position: Position,
        second_position: Position,
    },
    ValidationFailure {
        message: String,
        rule: String,
        position: Position,
    },
    MaxDepthExceeded {
        max_depth: usize,
        current_path: Vec<String>,
    },
}

impl SemanticError {
    /// Get the position associated with this error
    #[must_use] 
    pub fn position(&self) -> Position {
        match self {
            Self::UnresolvedAlias { position, .. } => *position,
            Self::CircularReference { position, .. } => *position,
            Self::DuplicateAnchor {
                duplicate_position, ..
            } => *duplicate_position,
            Self::InvalidTagHandle { position, .. } => *position,
            Self::UnknownTag { position, .. } => *position,
            Self::TagResolutionFailed { position, .. } => *position,
            Self::ValidationDepthExceeded { position, .. } => *position,
            Self::UnknownTagHandle { position, .. } => *position,
            Self::CustomTagResolutionFailed { position, .. } => *position,
            Self::UnknownCustomTag { position, .. } => *position,
            Self::InvalidDocumentStructure { position, .. } => *position,
            Self::TypeMismatch { position, .. } => *position,
            Self::ValueValidationFailed { position, .. } => *position,
            Self::ReferenceTrackingError { position, .. } => *position,
            Self::AnchorRegistrationFailed { position, .. } => *position,
            Self::ValidationError { position, .. } => *position,
            Self::InternalError { position, .. } => *position,
            Self::ConflictingAnchor { first_position, .. } => *first_position,
            Self::ValidationFailure { position, .. } => *position,
            Self::MaxDepthExceeded { .. } => Position::default(),
        }
    }

    /// Get human-readable error message
    #[must_use] 
    pub fn message(&self) -> String {
        match self {
            Self::UnresolvedAlias { alias_name, .. } => {
                format!("Unresolved alias reference: '{alias_name}'")
            }
            Self::CircularReference {
                alias_name, path, ..
            } => {
                format!("Circular reference detected for alias '{alias_name}' at path '{path}'")
            }
            Self::DuplicateAnchor {
                anchor_name,
                first_position,
                ..
            } => {
                format!(
                    "Duplicate anchor '{}' (first defined at line {}, column {})",
                    anchor_name, first_position.line, first_position.column
                )
            }
            Self::InvalidTagHandle { handle, .. } => {
                format!("Invalid tag handle: '{handle}'")
            }
            Self::UnknownTag { tag, .. } => {
                format!("Unknown tag: '{tag}'")
            }
            Self::TagResolutionFailed { tag, reason, .. } => {
                format!("Failed to resolve tag '{tag}': {reason}")
            }
            Self::ValidationDepthExceeded {
                max_depth,
                current_depth,
                ..
            } => {
                format!("Validation depth exceeded: {current_depth} > {max_depth} (maximum)")
            }
            Self::UnknownTagHandle { handle, .. } => {
                format!("Unknown tag handle: '{handle}'")
            }
            Self::CustomTagResolutionFailed { tag, error, .. } => {
                format!("Custom tag resolution failed for '{tag}': {error}")
            }
            Self::UnknownCustomTag { tag, .. } => {
                format!("Unknown custom tag: '{tag}'")
            }
            Self::InvalidDocumentStructure { reason, .. } => {
                format!("Invalid document structure: {reason}")
            }
            Self::TypeMismatch {
                expected, actual, ..
            } => {
                format!("Type mismatch: expected '{expected}', found '{actual}'")
            }
            Self::ValueValidationFailed {
                value, constraint, ..
            } => {
                format!("Value '{value}' failed validation: {constraint}")
            }
            Self::ReferenceTrackingError { reason, .. } => {
                format!("Reference tracking error: {reason}")
            }
            Self::AnchorRegistrationFailed {
                anchor_name,
                reason,
                ..
            } => {
                format!("Failed to register anchor '{anchor_name}': {reason}")
            }
            Self::ValidationError { message, .. } => {
                format!("Validation error: {message}")
            }
            Self::InternalError { message, .. } => {
                format!("Internal error: {message}")
            }
            Self::ConflictingAnchor { anchor_name, .. } => {
                format!("Conflicting anchor definition: '{anchor_name}'")
            }
            Self::ValidationFailure { message, rule, .. } => {
                format!("Validation rule '{rule}' failed: {message}")
            }
            Self::MaxDepthExceeded {
                max_depth,
                current_path,
            } => {
                format!("Maximum depth {max_depth} exceeded at path: {current_path:?}")
            }
        }
    }

    /// Create an unresolved alias error
    #[inline]
    #[must_use] 
    pub const fn unresolved_alias(alias_name: String, position: Position) -> Self {
        Self::UnresolvedAlias {
            alias_name,
            position,
        }
    }

    /// Create a circular reference error
    #[inline]
    #[must_use] 
    pub const fn circular_reference(alias_name: String, path: String, position: Position) -> Self {
        Self::CircularReference {
            alias_name,
            path,
            position,
        }
    }

    /// Create a duplicate anchor error
    #[inline]
    #[must_use] 
    pub const fn duplicate_anchor(
        anchor_name: String,
        first_position: Position,
        duplicate_position: Position,
    ) -> Self {
        Self::DuplicateAnchor {
            anchor_name,
            first_position,
            duplicate_position,
        }
    }

    /// Create an invalid tag handle error
    #[inline]
    #[must_use] 
    pub const fn invalid_tag_handle(handle: String, position: Position) -> Self {
        Self::InvalidTagHandle { handle, position }
    }

    /// Create an unknown tag error
    #[inline]
    #[must_use] 
    pub const fn unknown_tag(tag: String, position: Position) -> Self {
        Self::UnknownTag { tag, position }
    }

    /// Create a tag resolution failed error
    #[inline]
    #[must_use] 
    pub const fn tag_resolution_failed(tag: String, reason: String, position: Position) -> Self {
        Self::TagResolutionFailed {
            tag,
            reason,
            position,
        }
    }

    /// Create a validation depth exceeded error
    #[inline]
    #[must_use] 
    pub const fn validation_depth_exceeded(
        max_depth: usize,
        current_depth: usize,
        position: Position,
    ) -> Self {
        Self::ValidationDepthExceeded {
            max_depth,
            current_depth,
            position,
        }
    }

    /// Create an unknown tag handle error
    #[inline]
    #[must_use] 
    pub const fn unknown_tag_handle(handle: String, position: Position) -> Self {
        Self::UnknownTagHandle { handle, position }
    }

    /// Create a custom tag resolution failed error
    #[inline]
    #[must_use] 
    pub const fn custom_tag_resolution_failed(tag: String, error: String, position: Position) -> Self {
        Self::CustomTagResolutionFailed {
            tag,
            error,
            position,
        }
    }

    /// Create an unknown custom tag error
    #[inline]
    #[must_use] 
    pub const fn unknown_custom_tag(tag: String, position: Position) -> Self {
        Self::UnknownCustomTag { tag, position }
    }

    /// Create an invalid document structure error
    #[inline]
    #[must_use] 
    pub const fn invalid_document_structure(reason: String, position: Position) -> Self {
        Self::InvalidDocumentStructure { reason, position }
    }

    /// Create a type mismatch error
    #[inline]
    #[must_use] 
    pub const fn type_mismatch(expected: String, actual: String, position: Position) -> Self {
        Self::TypeMismatch {
            expected,
            actual,
            position,
        }
    }

    /// Create a value validation failed error
    #[inline]
    #[must_use] 
    pub const fn value_validation_failed(value: String, constraint: String, position: Position) -> Self {
        Self::ValueValidationFailed {
            value,
            constraint,
            position,
        }
    }

    /// Create a reference tracking error
    #[inline]
    #[must_use] 
    pub const fn reference_tracking_error(reason: String, position: Position) -> Self {
        Self::ReferenceTrackingError { reason, position }
    }

    /// Create an anchor registration failed error
    #[inline]
    #[must_use] 
    pub const fn anchor_registration_failed(
        anchor_name: String,
        reason: String,
        position: Position,
    ) -> Self {
        Self::AnchorRegistrationFailed {
            anchor_name,
            reason,
            position,
        }
    }

    /// Create a validation error
    #[inline]
    #[must_use] 
    pub const fn validation_error(message: String, position: Position) -> Self {
        Self::ValidationError { message, position }
    }
}

impl std::fmt::Display for SemanticError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for SemanticError {}

/// Error type aliases for commonly used semantic errors
pub type SemanticResult<T> = Result<T, SemanticError>;

/// Macro for creating semantic errors with position context
#[macro_export]
macro_rules! semantic_error {
    (unresolved_alias, $name:expr, $pos:expr) => {
        $crate::semantic::error::SemanticError::unresolved_alias($name.to_string(), $pos)
    };
    (circular_reference, $name:expr, $path:expr, $pos:expr) => {
        $crate::semantic::error::SemanticError::circular_reference(
            $name.to_string(),
            $path.to_string(),
            $pos,
        )
    };
    (duplicate_anchor, $name:expr, $first:expr, $dup:expr) => {
        $crate::semantic::error::SemanticError::duplicate_anchor($name.to_string(), $first, $dup)
    };
    (validation_error, $msg:expr, $pos:expr) => {
        $crate::semantic::error::SemanticError::validation_error($msg.to_string(), $pos)
    };
}
