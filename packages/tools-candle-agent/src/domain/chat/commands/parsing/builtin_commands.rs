//! Built-in command definitions and registration

use std::collections::HashMap;

use super::super::types::{CommandInfo, ParameterInfo, ParameterType, StabilityLevel};

/// Register all built-in commands
#[allow(clippy::too_many_lines)]
pub(super) fn register_builtin_commands(
    commands: &mut HashMap<String, CommandInfo>,
    aliases: &mut HashMap<String, String>,
) {
    // Helper function to register a command with its aliases
    let register = |commands: &mut HashMap<String, CommandInfo>,
                    aliases: &mut HashMap<String, String>,
                    info: CommandInfo| {
        let name = info.name.clone();

        // Register aliases
        for alias in &info.aliases {
            aliases.insert(alias.clone(), name.clone());
        }

        // Register main command
        commands.insert(name, info);
    };

    // Help command
    register(
        commands,
        aliases,
        CommandInfo {
            name: "help".to_string(),
            description: "Show help information".to_string(),
            usage: "/help [command] [--extended]".to_string(),
            parameters: vec![
                ParameterInfo {
                    name: "command".to_string(),
                    description: "Optional command to get help for".to_string(),
                    parameter_type: ParameterType::String,
                    required: false,
                    default_value: None,
                    min_value: None,
                    max_value: None,
                    pattern: None,
                    examples: vec!["help".to_string(), "clear".to_string()],
                },
                ParameterInfo {
                    name: "extended".to_string(),
                    description: "Show extended help".to_string(),
                    parameter_type: ParameterType::Boolean,
                    required: false,
                    default_value: Some("false".to_string()),
                    min_value: None,
                    max_value: None,
                    pattern: None,
                    examples: vec!["true".to_string(), "false".to_string()],
                },
            ],
            aliases: vec!["h".to_string(), "?".to_string()],
            category: "General".to_string(),
            examples: vec![
                "/help".to_string(),
                "/help config".to_string(),
                "/help --extended".to_string(),
            ],
            version: "1.0.0".to_string(),
            author: Some("Fluent AI Team".to_string()),
            tags: vec!["help".to_string(), "documentation".to_string()],
            required_permissions: vec![],
            deprecated: false,
            deprecation_message: None,
            experimental: false,
            stability: StabilityLevel::Stable,
        },
    );

    // Clear command
    register(
        commands,
        aliases,
        CommandInfo {
            name: "clear".to_string(),
            description: "Clear chat history".to_string(),
            usage: "/clear [--confirm] [--keep-last N]".to_string(),
            parameters: vec![
                ParameterInfo {
                    name: "confirm".to_string(),
                    description: "Confirm the action".to_string(),
                    parameter_type: ParameterType::Boolean,
                    required: false,
                    default_value: Some("false".to_string()),
                    min_value: None,
                    max_value: None,
                    pattern: None,
                    examples: vec!["true".to_string(), "false".to_string()],
                },
                ParameterInfo {
                    name: "keep-last".to_string(),
                    description: "Keep last N messages".to_string(),
                    parameter_type: ParameterType::Integer,
                    required: false,
                    default_value: None,
                    min_value: Some(1.0),
                    max_value: Some(1000.0),
                    pattern: None,
                    examples: vec!["10".to_string(), "50".to_string()],
                },
            ],
            aliases: vec!["cls".to_string(), "reset".to_string()],
            category: "History".to_string(),
            examples: vec![
                "/clear".to_string(),
                "/clear --confirm".to_string(),
                "/clear --keep-last 10".to_string(),
            ],
            version: "1.0.0".to_string(),
            author: Some("Fluent AI Team".to_string()),
            tags: vec!["history".to_string(), "cleanup".to_string()],
            required_permissions: vec![],
            deprecated: false,
            deprecation_message: None,
            experimental: false,
            stability: StabilityLevel::Stable,
        },
    );

    // Export command
    register(
        commands,
        aliases,
        CommandInfo {
            name: "export".to_string(),
            description: "Export conversation".to_string(),
            usage: "/export --format FORMAT [--output FILE] [--include-metadata]".to_string(),
            parameters: vec![
                ParameterInfo {
                    name: "format".to_string(),
                    description: "Export format (json, markdown, pdf, html)".to_string(),
                    parameter_type: ParameterType::Enum {
                        values: vec![
                            "json".to_string(),
                            "markdown".to_string(),
                            "pdf".to_string(),
                            "html".to_string(),
                        ],
                    },
                    required: true,
                    default_value: None,
                    min_value: None,
                    max_value: None,
                    pattern: None,
                    examples: vec![],
                },
                ParameterInfo {
                    name: "output".to_string(),
                    description: "Output file path".to_string(),
                    parameter_type: ParameterType::Path,
                    required: false,
                    default_value: None,
                    min_value: None,
                    max_value: None,
                    pattern: None,
                    examples: vec![],
                },
                ParameterInfo {
                    name: "include-metadata".to_string(),
                    description: "Include metadata in export".to_string(),
                    parameter_type: ParameterType::Boolean,
                    required: false,
                    default_value: Some("true".to_string()),
                    min_value: None,
                    max_value: None,
                    pattern: None,
                    examples: vec!["true".to_string(), "false".to_string()],
                },
            ],
            aliases: vec!["save".to_string()],
            category: "Export".to_string(),
            examples: vec![
                "/export --format json".to_string(),
                "/export --format markdown --output chat.md".to_string(),
                "/export --format pdf --include-metadata".to_string(),
            ],
            version: "1.0.0".to_string(),
            author: Some("Fluent AI Team".to_string()),
            tags: vec![
                "export".to_string(),
                "save".to_string(),
                "backup".to_string(),
            ],
            required_permissions: vec![],
            deprecated: false,
            deprecation_message: None,
            experimental: false,
            stability: StabilityLevel::Stable,
        },
    );

    // Config command
    register(
        commands,
        aliases,
        CommandInfo {
            name: "config".to_string(),
            description: "Modify configuration".to_string(),
            usage: "/config [KEY] [VALUE] [--show] [--reset]".to_string(),
            parameters: vec![
                ParameterInfo {
                    name: "key".to_string(),
                    description: "Configuration key".to_string(),
                    parameter_type: ParameterType::String,
                    required: false,
                    default_value: None,
                    min_value: None,
                    max_value: None,
                    pattern: None,
                    examples: vec![],
                },
                ParameterInfo {
                    name: "value".to_string(),
                    description: "Configuration value".to_string(),
                    parameter_type: ParameterType::String,
                    required: false,
                    default_value: None,
                    min_value: None,
                    max_value: None,
                    pattern: None,
                    examples: vec![],
                },
                ParameterInfo {
                    name: "show".to_string(),
                    description: "Show current configuration".to_string(),
                    parameter_type: ParameterType::Boolean,
                    required: false,
                    default_value: Some("false".to_string()),
                    min_value: None,
                    max_value: None,
                    pattern: None,
                    examples: vec!["true".to_string(), "false".to_string()],
                },
                ParameterInfo {
                    name: "reset".to_string(),
                    description: "Reset to defaults".to_string(),
                    parameter_type: ParameterType::Boolean,
                    required: false,
                    default_value: Some("false".to_string()),
                    min_value: None,
                    max_value: None,
                    pattern: None,
                    examples: vec!["true".to_string(), "false".to_string()],
                },
            ],
            aliases: vec!["cfg".to_string(), "settings".to_string()],
            category: "Configuration".to_string(),
            examples: vec![
                "/config --show".to_string(),
                "/config theme dark".to_string(),
                "/config --reset".to_string(),
            ],
            version: "1.0.0".to_string(),
            author: Some("Fluent AI Team".to_string()),
            tags: vec!["configuration".to_string(), "settings".to_string()],
            required_permissions: vec![],
            deprecated: false,
            deprecation_message: None,
            experimental: false,
            stability: StabilityLevel::Stable,
        },
    );

    // Search command
    register(
        commands,
        aliases,
        CommandInfo {
            name: "search".to_string(),
            description: "Search chat history".to_string(),
            usage: "/search QUERY [--scope SCOPE] [--limit N] [--include-context]".to_string(),
            parameters: vec![
                ParameterInfo {
                    name: "query".to_string(),
                    description: "Search query".to_string(),
                    parameter_type: ParameterType::String,
                    required: true,
                    default_value: None,
                    min_value: None,
                    max_value: None,
                    pattern: None,
                    examples: vec![],
                },
                ParameterInfo {
                    name: "scope".to_string(),
                    description: "Search scope (all, current, recent)".to_string(),
                    parameter_type: ParameterType::Enum {
                        values: vec![
                            "all".to_string(),
                            "current".to_string(),
                            "recent".to_string(),
                            "bookmarked".to_string(),
                        ],
                    },
                    required: false,
                    default_value: Some("all".to_string()),
                    min_value: None,
                    max_value: None,
                    pattern: None,
                    examples: vec![],
                },
                ParameterInfo {
                    name: "limit".to_string(),
                    description: "Maximum results to return".to_string(),
                    parameter_type: ParameterType::Integer,
                    required: false,
                    default_value: Some("10".to_string()),
                    min_value: Some(1.0),
                    max_value: Some(100.0),
                    pattern: None,
                    examples: vec![],
                },
                ParameterInfo {
                    name: "include-context".to_string(),
                    description: "Include surrounding context".to_string(),
                    parameter_type: ParameterType::Boolean,
                    required: false,
                    default_value: Some("false".to_string()),
                    min_value: None,
                    max_value: None,
                    pattern: None,
                    examples: vec!["true".to_string(), "false".to_string()],
                },
            ],
            aliases: vec!["find".to_string(), "query".to_string()],
            category: "Search".to_string(),
            examples: vec![
                "/search error".to_string(),
                "/search \"API call\" --scope recent".to_string(),
                "/search bug --limit 5 --include-context".to_string(),
            ],
            version: "1.0.0".to_string(),
            author: Some("Fluent AI Team".to_string()),
            tags: vec![
                "search".to_string(),
                "find".to_string(),
                "query".to_string(),
            ],
            required_permissions: vec![],
            deprecated: false,
            deprecation_message: None,
            experimental: false,
            stability: StabilityLevel::Stable,
        },
    );
}
