use anyhow::Result;
use minijinja::Environment;

/// Maximum template size in bytes (1MB)
const MAX_TEMPLATE_SIZE: usize = 1_000_000;

/// Validate `MiniJinja` template syntax
pub fn validate_template_syntax(content: &str) -> Result<()> {
    let mut env = Environment::new();

    // Try to add template - will fail if syntax invalid
    env.add_template("_validation", content)
        .map_err(|e| anyhow::anyhow!("Template syntax error: {e}"))?;

    Ok(())
}

/// Validate complete prompt file (metadata + content)
pub fn validate_prompt_file(content: &str) -> Result<()> {
    // Validate size first (security: prevent resource exhaustion)
    if content.len() > MAX_TEMPLATE_SIZE {
        anyhow::bail!(
            "Template too large ({} bytes). Maximum size is {} bytes (1MB).",
            content.len(),
            MAX_TEMPLATE_SIZE
        );
    }

    // Parse to ensure valid structure
    let template = super::template::parse_template("_validation", content)?;

    // Validate template syntax
    validate_template_syntax(&template.content)?;

    // Additional checks
    validate_no_dangerous_operations(&template.content)?;

    Ok(())
}

/// Check for dangerous template operations
/// Based on security policy (research decision)
fn validate_no_dangerous_operations(content: &str) -> Result<()> {
    // Block include directives (file access)
    if content.contains("{% include") || content.contains("{%- include") {
        anyhow::bail!(
            "Template contains forbidden 'include' directive. \
             File inclusion is not allowed for security reasons."
        );
    }

    // Block extends directives (not needed, potential attack vector)
    if content.contains("{% extends") || content.contains("{%- extends") {
        anyhow::bail!(
            "Template contains forbidden 'extends' directive. \
             Template inheritance is not supported."
        );
    }

    // Block import directives (module loading)
    if content.contains("{% import") || content.contains("{%- import") {
        anyhow::bail!(
            "Template contains forbidden 'import' directive. \
             Module imports are not allowed."
        );
    }

    Ok(())
}
