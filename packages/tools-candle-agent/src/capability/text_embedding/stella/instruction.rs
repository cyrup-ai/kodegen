//! Task-specific instruction formatting for Stella embeddings

/// Valid task types for Stella embeddings
const VALID_TASKS: &[&str] = &[
    "s2p",
    "s2s",
    "search_query",
    "search_document",
    "classification",
    "clustering",
    "retrieval",
];

/// Get the instruction string for a given task (or default)
///
/// Validates the task parameter and logs a warning if invalid.
/// Returns the appropriate instruction text for the task.
fn get_instruction(task: Option<&str>) -> &'static str {
    // Validate task parameter and warn if invalid
    if let Some(t) = task
        && !VALID_TASKS.contains(&t)
    {
        log::warn!(
            "Unknown embedding task '{}'. Using default 's2p'. Valid tasks: {}",
            t,
            VALID_TASKS.join(", ")
        );
    }

    match task {
        Some("s2p") => {
            "Given a web search query, retrieve relevant passages that answer the query."
        }
        Some("s2s") => "Retrieve semantically similar text.",
        Some("search_query") => {
            "Given a web search query, retrieve relevant passages that answer the query."
        } // Map to s2p
        Some("search_document") => {
            "Given a web search query, retrieve relevant passages that answer the query."
        } // Map to s2p
        Some("classification") => "Retrieve semantically similar text.", // Map to s2s
        Some("clustering") => "Retrieve semantically similar text.",     // Map to s2s
        Some("retrieval") => {
            "Given a web search query, retrieve relevant passages that answer the query."
        } // Map to s2p
        _ => "Given a web search query, retrieve relevant passages that answer the query.", // Default to s2p
    }
}

/// Format a single text with task-specific instruction prefix
///
/// Optimized for single-text embeddings - avoids Vec allocation.
/// For batch operations, use `format_with_instruction()` instead.
///
/// # Task Types
/// - `"s2p"`, `"search_query"`, `"search_document"`, or `"retrieval"`: Search query → passage retrieval
///   - Instruction: "Given a web search query, retrieve relevant passages that answer the query."
/// - `"s2s"`, `"classification"`, or `"clustering"`: Semantic similarity
///   - Instruction: "Retrieve semantically similar text."
/// - `None`: Defaults to search query mode (`"s2p"`)
///
/// # Validation
/// Invalid tasks trigger a warning and fall back to default `"s2p"` instruction.
///
/// # Examples
/// ```ignore
/// let formatted = format_single_with_instruction("What is Rust?", Some("search_query"));
/// // Returns: "Instruct: Given a web search query...\nQuery: What is Rust?"
/// ```
#[inline]
pub fn format_single_with_instruction(text: &str, task: Option<&str>) -> String {
    let instruct = get_instruction(task);
    format!("Instruct: {}\nQuery: {}", instruct, text)
}

/// Format multiple texts with task-specific instruction prefix
///
/// For single-text embeddings, prefer `format_single_with_instruction()` to avoid Vec allocation.
///
/// # Task Types
/// - `"s2p"`, `"search_query"`, `"search_document"`, or `"retrieval"`: Search query → passage retrieval
///   - Instruction: "Given a web search query, retrieve relevant passages that answer the query."
/// - `"s2s"`, `"classification"`, or `"clustering"`: Semantic similarity
///   - Instruction: "Retrieve semantically similar text."
/// - `None`: Defaults to search query mode (`"s2p"`)
///
/// # Validation
/// If an invalid task is provided, a warning will be logged and the default `"s2p"` instruction will be used.
///
/// # Examples
/// ```ignore
/// let texts = vec!["What is Rust?", "How does async work?"];
/// let formatted = format_with_instruction(&texts, Some("search_query"));
/// // Returns texts prefixed with search instruction
/// ```
pub fn format_with_instruction(texts: &[&str], task: Option<&str>) -> Vec<String> {
    let instruct = get_instruction(task);
    texts
        .iter()
        .map(|text| format!("Instruct: {}\nQuery: {}", instruct, text))
        .collect()
}

