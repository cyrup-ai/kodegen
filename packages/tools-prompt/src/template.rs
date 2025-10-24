use super::metadata::{ParameterType, PromptMetadata, PromptTemplate};
use anyhow::{Context, Result};
use gray_matter::engine::YAML;
use gray_matter::{Matter, Pod};
use minijinja::Environment;
use std::collections::HashMap;

/// Parse a .j2.md file into metadata and content
pub fn parse_template(filename: &str, file_content: &str) -> Result<PromptTemplate> {
    // Use gray_matter to split frontmatter and content
    let matter = Matter::<YAML>::new();
    let parsed: gray_matter::ParsedEntity<Pod> = matter
        .parse(file_content)
        .map_err(|e| anyhow::anyhow!("Failed to parse frontmatter: {e}"))?;

    // Extract and deserialize frontmatter
    let metadata: PromptMetadata = parsed
        .data
        .ok_or_else(|| anyhow::anyhow!("No frontmatter found in template"))?
        .deserialize()
        .context("Failed to parse YAML frontmatter")?;

    // Validate metadata
    validate_metadata(&metadata)?;

    // Get content (after frontmatter)
    let content = parsed.content;

    Ok(PromptTemplate {
        filename: filename.to_string(),
        metadata,
        content,
    })
}

/// Validate metadata fields
fn validate_metadata(metadata: &PromptMetadata) -> Result<()> {
    if metadata.title.is_empty() {
        anyhow::bail!("Title cannot be empty");
    }
    if metadata.description.is_empty() {
        anyhow::bail!("Description cannot be empty");
    }
    if metadata.categories.is_empty() {
        anyhow::bail!("At least one category is required");
    }
    if metadata.author.is_empty() {
        anyhow::bail!("Author cannot be empty");
    }
    Ok(())
}

/// Render a template with parameters and environment variables
///
/// # Security Notes
/// - Template size is validated before parsing (max 1MB)
/// - `MiniJinja` has built-in recursion limits (default ~500 levels)
/// - Timeout enforcement (5 seconds) is handled at the tool layer in async context
/// - These protections prevent resource exhaustion from malicious templates
pub fn render_template(
    template: &PromptTemplate,
    parameters: Option<&HashMap<String, serde_json::Value>>,
) -> Result<String> {
    let mut env = Environment::new();

    // Configure environment
    env.set_auto_escape_callback(|_| minijinja::AutoEscape::None);

    // Add template
    env.add_template(&template.filename, &template.content)?;

    // Build context
    let ctx = build_context(template, parameters)?;

    // Render
    let tmpl = env.get_template(&template.filename)?;
    let rendered = tmpl.render(ctx)?;

    Ok(rendered)
}

/// Build template context from parameters
fn build_context(
    template: &PromptTemplate,
    parameters: Option<&HashMap<String, serde_json::Value>>,
) -> Result<minijinja::Value> {
    let params = parameters.cloned().unwrap_or_default();

    // Validate parameters against definitions
    validate_parameters(template, &params)?;

    // Apply defaults for missing optional parameters
    let mut params_with_defaults = apply_defaults(template, params);

    // Add environment variables
    add_env_vars(&mut params_with_defaults);

    Ok(minijinja::Value::from_serialize(&params_with_defaults))
}

/// Add environment variables to context
fn add_env_vars(params: &mut HashMap<String, serde_json::Value>) {
    // Whitelist from security policy (research decision)
    const ALLOWED_ENV_VARS: &[&str] = &["USER", "HOME", "SHELL", "PWD", "EDITOR", "TERM"];

    let safe_env_vars: HashMap<String, String> = std::env::vars()
        .filter(|(key, _)| ALLOWED_ENV_VARS.contains(&key.as_str()))
        .collect();

    // Add as env.* namespace in context
    params.insert("env".to_string(), serde_json::json!(safe_env_vars));
}

/// Validate provided parameters match definitions
fn validate_parameters(
    template: &PromptTemplate,
    params: &HashMap<String, serde_json::Value>,
) -> Result<()> {
    // Check required parameters are present
    for param_def in &template.metadata.parameters {
        if param_def.required && !params.contains_key(&param_def.name) {
            anyhow::bail!(
                "Required parameter '{}' not provided. Description: {}",
                param_def.name,
                param_def.description
            );
        }
    }

    // Validate types for provided parameters
    for param_def in &template.metadata.parameters {
        if let Some(value) = params.get(&param_def.name) {
            validate_parameter_type(param_def, value)?;
        }
    }

    Ok(())
}

/// Validate a parameter value matches its expected type
fn validate_parameter_type(
    param_def: &super::metadata::ParameterDefinition,
    value: &serde_json::Value,
) -> Result<()> {
    let valid = match param_def.param_type {
        ParameterType::String => value.is_string(),
        ParameterType::Number => value.is_number(),
        ParameterType::Boolean => value.is_boolean(),
        ParameterType::Array => value.is_array(),
    };

    if !valid {
        anyhow::bail!(
            "Parameter '{}' has wrong type. Expected {:?}, got {}",
            param_def.name,
            param_def.param_type,
            match value {
                serde_json::Value::Null => "null",
                serde_json::Value::Bool(_) => "boolean",
                serde_json::Value::Number(_) => "number",
                serde_json::Value::String(_) => "string",
                serde_json::Value::Array(_) => "array",
                serde_json::Value::Object(_) => "object",
            }
        );
    }

    Ok(())
}

/// Apply default values for missing optional parameters
fn apply_defaults(
    template: &PromptTemplate,
    mut params: HashMap<String, serde_json::Value>,
) -> HashMap<String, serde_json::Value> {
    for param_def in &template.metadata.parameters {
        if !params.contains_key(&param_def.name)
            && let Some(default) = param_def.default.as_ref()
        {
            params.insert(param_def.name.clone(), default.clone());
        }
    }
    params
}
