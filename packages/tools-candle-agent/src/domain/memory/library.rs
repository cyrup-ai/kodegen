//! Library type for memory system - EXACT API from ARCHITECTURE.md
//!
//! A library represents a named and isolated memory namespace that can be used
//! by one or many agents. This is a pure domain entity containing only the
//! data structure - service logic is implemented in the cyrup package.

/// Library type for memory namespace isolation
///
/// Libraries provide named, isolated memory namespaces that can be shared
/// between multiple agents while maintaining context separation.
///
/// # Examples
///
/// ```rust
/// // Create a library from ARCHITECTURE.md
/// let library = Library::named("obsidian_vault");
///
/// // Use in agent role builder (service logic in cyrup package)
/// FluentAi::agent_role("rusty-squire")
///     .memory(Library::named("obsidian_vault"))
///     .into_agent();
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Library {
    name: String}

impl Library {
    /// Create a named library - EXACT syntax: Library::named("obsidian_vault")
    ///
    /// # Arguments
    /// * `name` - The library name, used as memory namespace identifier
    ///
    /// # Returns
    /// New library instance
    ///
    /// # Examples
    /// ```rust
    /// let library = Library::named("obsidian_vault");
    /// ```
    pub fn named(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    /// Get the library name
    ///
    /// # Returns
    /// Reference to the library name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the namespace identifier for this library
    ///
    /// # Returns
    /// Formatted namespace string for memory system isolation
    pub fn namespace(&self) -> String {
        format!("lib_{}", self.name)
    }

    /// Validate library name for namespace safety
    ///
    /// # Arguments
    /// * `name` - Library name to validate
    ///
    /// # Returns
    /// Result indicating if name is valid for use as a memory namespace
    pub fn validate_name(name: &str) -> Result<(), String> {
        if name.is_empty() {
            return Err("Library name cannot be empty".to_string());
        }

        if name.len() > 100 {
            return Err("Library name cannot exceed 100 characters".to_string());
        }

        // Check for invalid characters that could cause namespace issues
        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(
                "Library name can only contain alphanumeric characters, underscores, and hyphens"
                    .to_string(),
            );
        }

        Ok(())
    }

    /// Check if this library name is valid
    ///
    /// # Returns
    /// True if the library name is valid for namespace usage
    pub fn is_valid(&self) -> bool {
        Self::validate_name(&self.name).is_ok()
    }
}

impl std::fmt::Display for Library {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Library({})", self.name)
    }
}
