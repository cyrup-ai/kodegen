//! Prompt template tools metadata

use kodegen_mcp_schema::prompt::{PROMPT_ADD, PROMPT_DELETE, PROMPT_EDIT, PROMPT_GET, AddPromptArgs, DeletePromptArgs, EditPromptArgs, GetPromptArgs};
use crate::stdio::metadata::types::{build_schema, ToolMetadata};

pub fn prompt_tools() -> Vec<ToolMetadata> {
    vec![
        ToolMetadata {
            name: PROMPT_ADD,
            category: "prompt",
            description: "Create a new prompt template. The content must include YAML frontmatter with metadata (title, description, categories, author) followed by the template body.",
            schema: build_schema::<AddPromptArgs>(),
        },
        ToolMetadata {
            name: PROMPT_DELETE,
            category: "prompt",
            description: "Delete a prompt template. Requires confirm=true for safety. This action cannot be undone. Default prompts can be deleted but will be recreated on next run.",
            schema: build_schema::<DeletePromptArgs>(),
        },
        ToolMetadata {
            name: PROMPT_EDIT,
            category: "prompt",
            description: "Edit an existing prompt template. Provide the prompt name and complete new content (including YAML frontmatter). The content is validated before saving.",
            schema: build_schema::<EditPromptArgs>(),
        },
        ToolMetadata {
            name: PROMPT_GET,
            category: "prompt",
            description: "Browse and retrieve prompt templates. Actions: list_categories (show all prompt categories), list_prompts (list all prompts, optionally filtered by category), get_prompt (retrieve specific prompt by name).",
            schema: build_schema::<GetPromptArgs>(),
        },
    ]
}
