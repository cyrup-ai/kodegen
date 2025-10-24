//! Chat configuration builder implementations
//!
//! All chat configuration construction logic and builder patterns.

use std::sync::Arc;

use crate::domain::chat::config::{
    BehaviorConfig, ChatConfig, IntegrationConfig, PersonalityConfig, UIConfig,
};

/// Configuration builder for ergonomic configuration creation
pub struct ConfigurationBuilder {
    config: ChatConfig,
}

impl ConfigurationBuilder {
    /// Create a new configuration builder
    pub fn new() -> Self {
        Self {
            config: ChatConfig::default(),
        }
    }

    /// Set personality configuration
    pub fn personality(mut self, personality: PersonalityConfig) -> Self {
        self.config.personality = personality;
        self
    }

    /// Set behavior configuration
    pub fn behavior(mut self, behavior: BehaviorConfig) -> Self {
        self.config.behavior = behavior;
        self
    }

    /// Set UI configuration
    pub fn ui(mut self, ui: UIConfig) -> Self {
        self.config.ui = ui;
        self
    }

    /// Set integration configuration
    pub fn integration(mut self, integration: IntegrationConfig) -> Self {
        self.config.integration = integration;
        self
    }

    /// Build the configuration
    pub fn build(self) -> ChatConfig {
        self.config
    }
}

impl Default for ConfigurationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Personality configuration builder
pub struct PersonalityConfigBuilder {
    config: PersonalityConfig,
}

impl PersonalityConfigBuilder {
    /// Create a new personality configuration builder
    pub fn new() -> Self {
        Self {
            config: PersonalityConfig::default(),
        }
    }

    /// Set tone
    pub fn tone(mut self, tone: impl Into<Arc<str>>) -> Self {
        self.config.tone = tone.into();
        self
    }

    /// Set creativity level
    pub fn creativity(mut self, creativity: f64) -> Self {
        self.config.creativity = creativity.clamp(0.0, 1.0);
        self
    }

    /// Set formality level
    pub fn formality(mut self, formality: f64) -> Self {
        self.config.formality = formality.clamp(0.0, 1.0);
        self
    }

    /// Set expertise level
    pub fn expertise(mut self, expertise: impl Into<Arc<str>>) -> Self {
        self.config.expertise_level = expertise.into();
        self
    }

    /// Add personality trait
    pub fn trait_name(mut self, trait_name: impl Into<Arc<str>>) -> Self {
        self.config.traits.push(trait_name.into());
        self
    }

    /// Set response style
    pub fn response_style(mut self, style: impl Into<Arc<str>>) -> Self {
        self.config.response_style = style.into();
        self
    }

    /// Set humor level
    pub fn humor(mut self, humor: f64) -> Self {
        self.config.humor = humor.clamp(0.0, 1.0);
        self
    }

    /// Set empathy level
    pub fn empathy(mut self, empathy: f64) -> Self {
        self.config.empathy = empathy.clamp(0.0, 1.0);
        self
    }

    /// Set verbosity level
    pub fn verbosity(mut self, verbosity: impl Into<Arc<str>>) -> Self {
        self.config.verbosity = verbosity.into();
        self
    }

    /// Build the personality configuration
    pub fn build(self) -> PersonalityConfig {
        self.config
    }
}

impl Default for PersonalityConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}