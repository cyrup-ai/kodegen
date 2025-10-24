//! Configuration validation system

use super::types::CandleChatConfig;

/// Candle configuration validation error
#[derive(Debug, Clone, thiserror::Error)]
pub enum CandleConfigurationValidationError {
    /// Invalid personality configuration detected
    #[error("Invalid personality configuration: {detail}")]
    InvalidPersonality {
        /// Details of the invalid personality configuration
        detail: String,
    },
    /// Invalid behavior configuration detected
    #[error("Invalid behavior configuration: {detail}")]
    InvalidBehavior {
        /// Details of the invalid behavior configuration
        detail: String,
    },
    /// Invalid UI configuration detected
    #[error("Invalid UI configuration: {detail}")]
    InvalidUI {
        /// Details of the invalid UI configuration
        detail: String,
    },

    /// Configuration conflict between settings
    #[error("Configuration conflict: {detail}")]
    Conflict {
        /// Details of the configuration conflict
        detail: String,
    },
    /// Schema validation failed for configuration
    #[error("Schema validation failed: {detail}")]
    SchemaValidation {
        /// Details of the schema validation failure
        detail: String,
    },
    /// Range validation failed for a field
    #[error("Range validation failed: {field} must be between {min} and {max}")]
    RangeValidation {
        /// Field name that failed range validation
        field: String,
        /// Minimum allowed value
        min: f32,
        /// Maximum allowed value
        max: f32,
    },
    /// Required field is missing from configuration
    #[error("Required field missing: {field}")]
    RequiredField {
        /// Name of the missing required field
        field: String,
    },
}

/// Candle configuration validation result
pub type CandleConfigurationValidationResult<T> = Result<T, CandleConfigurationValidationError>;

/// Candle configuration validator trait
pub trait CandleConfigurationValidator {
    /// Validate configuration section
    ///
    /// # Errors
    ///
    /// Returns `CandleConfigurationValidationError` if configuration validation fails
    fn validate(&self, config: &CandleChatConfig) -> CandleConfigurationValidationResult<()>;
    /// Get validator name
    fn name(&self) -> &str;
    /// Get validation priority (lower = higher priority)
    fn priority(&self) -> u8;
}

/// Candle personality configuration validator
pub struct CandlePersonalityValidator;

impl CandleConfigurationValidator for CandlePersonalityValidator {
    fn validate(&self, config: &CandleChatConfig) -> CandleConfigurationValidationResult<()> {
        let personality = &config.personality;

        // Validate creativity range
        if !(0.0..=1.0).contains(&personality.creativity) {
            return Err(CandleConfigurationValidationError::RangeValidation {
                field: String::from("creativity"),
                min: 0.0,
                max: 1.0,
            });
        }

        // Validate formality range
        if !(0.0..=1.0).contains(&personality.formality) {
            return Err(CandleConfigurationValidationError::RangeValidation {
                field: String::from("formality"),
                min: 0.0,
                max: 1.0,
            });
        }

        // Validate humor range
        if !(0.0..=1.0).contains(&personality.humor) {
            return Err(CandleConfigurationValidationError::RangeValidation {
                field: String::from("humor"),
                min: 0.0,
                max: 1.0,
            });
        }

        // Validate empathy range
        if !(0.0..=1.0).contains(&personality.empathy) {
            return Err(CandleConfigurationValidationError::RangeValidation {
                field: String::from("empathy"),
                min: 0.0,
                max: 1.0,
            });
        }

        // Validate expertise level
        let valid_expertise = ["beginner", "intermediate", "advanced", "expert"];
        if !valid_expertise.contains(&personality.expertise_level.as_ref()) {
            return Err(CandleConfigurationValidationError::InvalidPersonality {
                detail: String::from("Invalid expertise level"),
            });
        }

        // Validate tone
        let valid_tones = ["formal", "casual", "friendly", "professional", "neutral"];
        if !valid_tones.contains(&personality.tone.as_ref()) {
            return Err(CandleConfigurationValidationError::InvalidPersonality {
                detail: String::from("Invalid tone"),
            });
        }

        // Validate verbosity
        let valid_verbosity = ["concise", "balanced", "detailed"];
        if !valid_verbosity.contains(&personality.verbosity.as_ref()) {
            return Err(CandleConfigurationValidationError::InvalidPersonality {
                detail: String::from("Invalid verbosity level"),
            });
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "personality"
    }

    fn priority(&self) -> u8 {
        1
    }
}

/// Candle behavior configuration validator
pub struct CandleBehaviorValidator;

impl CandleConfigurationValidator for CandleBehaviorValidator {
    fn validate(&self, config: &CandleChatConfig) -> CandleConfigurationValidationResult<()> {
        let behavior = &config.behavior;

        // Validate proactivity range
        if !(0.0..=1.0).contains(&behavior.proactivity) {
            return Err(CandleConfigurationValidationError::RangeValidation {
                field: String::from("proactivity"),
                min: 0.0,
                max: 1.0,
            });
        }

        // Validate question frequency range
        if !(0.0..=1.0).contains(&behavior.question_frequency) {
            return Err(CandleConfigurationValidationError::RangeValidation {
                field: String::from("question_frequency"),
                min: 0.0,
                max: 1.0,
            });
        }

        // Validate conversation flow
        let valid_flows = ["natural", "structured", "adaptive", "guided"];
        if !valid_flows.contains(&behavior.conversation_flow.as_ref()) {
            return Err(CandleConfigurationValidationError::InvalidBehavior {
                detail: String::from("Invalid conversation flow"),
            });
        }

        // Validate follow-up behavior
        let valid_followups = ["contextual", "consistent", "adaptive", "minimal"];
        if !valid_followups.contains(&behavior.follow_up_behavior.as_ref()) {
            return Err(CandleConfigurationValidationError::InvalidBehavior {
                detail: String::from("Invalid follow-up behavior"),
            });
        }

        // Validate error handling
        let valid_error_handling = ["graceful", "verbose", "silent", "strict"];
        if !valid_error_handling.contains(&behavior.error_handling.as_ref()) {
            return Err(CandleConfigurationValidationError::InvalidBehavior {
                detail: String::from("Invalid error handling approach"),
            });
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "behavior"
    }

    fn priority(&self) -> u8 {
        2
    }
}

/// Candle UI configuration validator
pub struct CandleUIValidator;

impl CandleConfigurationValidator for CandleUIValidator {
    fn validate(&self, config: &CandleChatConfig) -> CandleConfigurationValidationResult<()> {
        let ui = &config.ui;

        // Validate theme
        let valid_themes = ["light", "dark", "auto", "system", "custom"];
        if !valid_themes.contains(&ui.theme.as_ref()) {
            return Err(CandleConfigurationValidationError::InvalidUI {
                detail: String::from("Invalid theme"),
            });
        }

        // Validate layout
        let valid_layouts = ["standard", "compact", "wide", "mobile", "adaptive"];
        if !valid_layouts.contains(&ui.layout.as_ref()) {
            return Err(CandleConfigurationValidationError::InvalidUI {
                detail: String::from("Invalid layout"),
            });
        }

        // Validate color scheme
        let valid_color_schemes = ["adaptive", "high_contrast", "colorblind", "custom"];
        if !valid_color_schemes.contains(&ui.color_scheme.as_ref()) {
            return Err(CandleConfigurationValidationError::InvalidUI {
                detail: String::from("Invalid color scheme"),
            });
        }

        // Validate display density
        let valid_densities = ["compact", "comfortable", "spacious"];
        if !valid_densities.contains(&ui.display_density.as_ref()) {
            return Err(CandleConfigurationValidationError::InvalidUI {
                detail: String::from("Invalid display density"),
            });
        }

        // Validate animations
        let valid_animations = ["none", "minimal", "smooth", "rich"];
        if !valid_animations.contains(&ui.animations.as_ref()) {
            return Err(CandleConfigurationValidationError::InvalidUI {
                detail: String::from("Invalid animation setting"),
            });
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "ui"
    }

    fn priority(&self) -> u8 {
        3
    }
}
