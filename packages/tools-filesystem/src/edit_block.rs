use crate::validate_path;
use chrono::Utc;
use kodegen_mcp_schema::filesystem::{EditBlockArgs, EditBlockPromptArgs};
use kodegen_mcp_tool::Tool;
use kodegen_mcp_tool::error::McpError;
use kodegen_utils::char_analysis::CharCodeData;
use kodegen_utils::char_diff::CharDiff;
use kodegen_utils::edit_log::{EditBlockLogEntry, EditBlockResult, get_edit_logger};
use kodegen_utils::fuzzy_logger::{FuzzySearchLogEntry, get_logger};
use kodegen_utils::fuzzy_search::{get_similarity_ratio, recursive_fuzzy_index_of_with_defaults};
use kodegen_utils::line_endings::{detect_line_ending, normalize_line_endings};
use kodegen_utils::suggestions::{EditFailureReason, Suggestion, SuggestionContext};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::{Value, json};
use std::time::Instant;
use tokio::fs;

// ============================================================================
// TOOL STRUCT
// ============================================================================

#[derive(Clone)]
pub struct EditBlockTool {
    config_manager: kodegen_tools_config::ConfigManager,
}

impl EditBlockTool {
    #[must_use]
    pub fn new(config_manager: kodegen_tools_config::ConfigManager) -> Self {
        Self { config_manager }
    }
}

// ============================================================================
// TOOL IMPLEMENTATION
// ============================================================================

impl Tool for EditBlockTool {
    type Args = EditBlockArgs;
    type PromptArgs = EditBlockPromptArgs;

    fn name() -> &'static str {
        "edit_block"
    }

    fn description() -> &'static str {
        "Apply surgical text replacements to files. Takes old_string and new_string, and performs \
         exact string replacement. By default replaces one occurrence. To replace multiple, set \
         expected_replacements. Returns error if old_string not found, or warning if actual count \
         doesn't match expected. Automatically validates paths."
    }

    fn read_only() -> bool {
        false
    }

    fn destructive() -> bool {
        true // Modifies file content
    }

    fn idempotent() -> bool {
        false // Each replacement changes content
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        let start_time = Instant::now(); // START TIMER

        // Validate inputs
        if args.old_string.is_empty() {
            return Err(McpError::InvalidArguments(
                "Empty search strings are not allowed. Please provide a non-empty string to search for.".to_string()
            ));
        }

        if args.old_string == args.new_string {
            return Err(McpError::InvalidArguments(
                "old_string and new_string are identical. No changes would be made.".to_string(),
            ));
        }

        let valid_path = validate_path(&args.file_path, &self.config_manager).await?;

        // Get file extension for response
        let extension = valid_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_string();

        // Read file
        let content = fs::read_to_string(&valid_path).await?;

        // Detect file's line ending style
        let file_line_ending = detect_line_ending(&content);

        // Normalize search string to match file's line endings
        let normalized_old_string = normalize_line_endings(&args.old_string, file_line_ending);

        // Check line limits and generate warning if exceeded
        let line_limit = self.config_manager.get_file_write_line_limit();
        let search_lines = normalized_old_string.lines().count().max(1);
        let replace_lines = args.new_string.lines().count().max(1);
        let max_lines = search_lines.max(replace_lines);

        let warning = if max_lines > line_limit {
            let problem_text = if search_lines > replace_lines {
                "search text"
            } else {
                "replacement text"
            };
            format!(
                "\n\nWARNING: The {problem_text} has {max_lines} lines (maximum: {line_limit}).\n\n\
                 RECOMMENDATION: For large search/replace operations, consider breaking them \
                 into smaller chunks with fewer lines."
            )
        } else {
            String::new()
        };

        // Count occurrences using normalized search
        let occurrence_count = content.matches(&normalized_old_string).count();

        // Handle no exact matches - try fuzzy search
        if occurrence_count == 0 {
            // Measure fuzzy search performance
            let start = std::time::Instant::now();

            // Attempt fuzzy match
            let fuzzy_result = recursive_fuzzy_index_of_with_defaults(&content, &args.old_string);

            // Calculate elapsed time in milliseconds
            let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

            // Calculate similarity using standard function
            let similarity = get_similarity_ratio(&fuzzy_result.value, &args.old_string);

            // Get configurable threshold from config
            let threshold = self.config_manager.get_fuzzy_search_threshold();

            // Get execution time for logging
            let execution_time = start_time.elapsed().as_secs_f64() * 1000.0;

            // Log fuzzy search attempt (FIRE-AND-FORGET, NEVER BLOCKS!)
            let log_entry = EditBlockLogEntry {
                timestamp: Utc::now(),
                search_text: args.old_string.clone(),
                found_text: Some(fuzzy_result.value.clone()),
                similarity: Some(similarity),
                execution_time_ms: execution_time,
                exact_match_count: 0,
                expected_replacements: args.expected_replacements,
                fuzzy_threshold: threshold,
                below_threshold: similarity < threshold,
                diff: None,
                search_length: args.old_string.len(),
                found_length: Some(fuzzy_result.value.len()),
                file_extension: extension.clone(),
                character_codes: None,
                unique_character_count: None,
                diff_length: None,
                result: if similarity >= threshold {
                    EditBlockResult::FuzzyMatchAccepted
                } else {
                    EditBlockResult::FuzzyMatchRejected
                },
            };

            get_edit_logger().log(log_entry);

            if similarity >= threshold {
                // Found similar text - show character diff
                let diff = CharDiff::new(&args.old_string, &fuzzy_result.value);
                let diff_display = diff.format();
                let is_whitespace_only = diff.is_whitespace_only();

                // Calculate line number where match was found
                let line_number = content[..fuzzy_result.start].matches('\n').count() + 1;

                // Log the fuzzy match attempt
                let logger = get_logger().await;
                let fuzzy_log_entry = FuzzySearchLogEntry {
                    timestamp: Utc::now(),
                    search_text: args.old_string.clone(),
                    found_text: fuzzy_result.value.clone(),
                    similarity,
                    execution_time_ms: elapsed_ms,
                    exact_match_count: 0,
                    expected_replacements: args.expected_replacements,
                    fuzzy_threshold: threshold,
                    below_threshold: false,
                    diff: diff_display.clone(),
                    search_length: args.old_string.len(),
                    found_length: fuzzy_result.value.len(),
                    file_extension: extension.clone(),
                };

                let _ = logger.log(&fuzzy_log_entry).await; // Ignore log errors
                let log_path = Some(logger.log_path().to_path_buf());
                drop(logger); // Release lock

                // Build suggestion context
                let context = SuggestionContext {
                    file_path: args.file_path.clone(),
                    search_string: args.old_string.clone(),
                    line_number: Some(line_number),
                    log_path,
                    execution_time_ms: Some(elapsed_ms),
                };

                // Build user-facing suggestion
                let suggestion = Suggestion::for_failure(
                    &EditFailureReason::FuzzyMatchAboveThreshold {
                        similarity,
                        is_whitespace_only,
                    },
                    &context,
                );

                // Perform comprehensive character analysis
                let char_data = CharCodeData::analyze(&args.old_string, &fuzzy_result.value);

                // Build complete error message
                let mut error_msg = suggestion.message.clone();
                error_msg.push_str("\n\nCharacter-level differences:\n");
                error_msg.push_str(&diff_display);

                if is_whitespace_only {
                    error_msg.push_str("\n\nNote: Difference is whitespace only.");
                }

                // Add comprehensive character analysis
                error_msg.push_str("\n\n");
                error_msg.push_str(&char_data.format_detailed_report());

                error_msg.push_str(&suggestion.format());

                return Err(McpError::InvalidArguments(error_msg));
            }

            // Calculate line number where match was found
            let line_number = content[..fuzzy_result.start].matches('\n').count() + 1;

            // Log the fuzzy match attempt (below threshold)
            let diff = CharDiff::new(&args.old_string, &fuzzy_result.value);
            let diff_display = diff.format();

            let logger = get_logger().await;
            let fuzzy_log_entry = FuzzySearchLogEntry {
                timestamp: Utc::now(),
                search_text: args.old_string.clone(),
                found_text: fuzzy_result.value.clone(),
                similarity,
                execution_time_ms: elapsed_ms,
                exact_match_count: 0,
                expected_replacements: args.expected_replacements,
                fuzzy_threshold: threshold,
                below_threshold: true,
                diff: diff_display,
                search_length: args.old_string.len(),
                found_length: fuzzy_result.value.len(),
                file_extension: extension.clone(),
            };

            let _ = logger.log(&fuzzy_log_entry).await; // Ignore log errors
            let log_path = Some(logger.log_path().to_path_buf());
            drop(logger); // Release lock

            // No good fuzzy match found - below threshold
            let context = SuggestionContext {
                file_path: args.file_path.clone(),
                search_string: args.old_string.clone(),
                line_number: Some(line_number),
                log_path,
                execution_time_ms: Some(elapsed_ms),
            };

            let suggestion = Suggestion::for_failure(
                &EditFailureReason::FuzzyMatchBelowThreshold {
                    similarity,
                    threshold,
                    found_text: fuzzy_result.value.clone(),
                },
                &context,
            );

            // Build complete error message with BOTH message and suggestions
            let mut error_msg = String::new();
            error_msg.push_str(&suggestion.message);
            error_msg.push_str(&suggestion.format());

            return Err(McpError::InvalidArguments(error_msg));
        }

        // Perform replacement using normalized strings
        let normalized_new_string = normalize_line_endings(&args.new_string, file_line_ending);
        let new_content = content.replace(&normalized_old_string, &normalized_new_string);

        fs::write(&valid_path, &new_content).await?;

        // Build response based on match status
        let execution_time = start_time.elapsed().as_secs_f64() * 1000.0;

        // Log successful exact match (FIRE-AND-FORGET, NEVER BLOCKS!)
        let log_entry = EditBlockLogEntry {
            timestamp: Utc::now(),
            search_text: args.old_string.clone(),
            found_text: Some(args.old_string.clone()),
            similarity: Some(1.0), // Exact match
            execution_time_ms: execution_time,
            exact_match_count: occurrence_count,
            expected_replacements: args.expected_replacements,
            fuzzy_threshold: self.config_manager.get_fuzzy_search_threshold(),
            below_threshold: false,
            diff: None,
            search_length: args.old_string.len(),
            found_length: Some(args.old_string.len()),
            file_extension: extension.clone(),
            character_codes: None,
            unique_character_count: None,
            diff_length: None,
            result: EditBlockResult::ExactMatch,
        };

        get_edit_logger().log(log_entry);

        if occurrence_count == args.expected_replacements {
            // Exact match - success
            Ok(json!({
                "success": true,
                "message": format!(
                    "Successfully replaced {} occurrence(s) of the target string in {}{}",
                    args.expected_replacements,
                    args.file_path,
                    warning
                ),
                "replacements_made": occurrence_count,
                "expected": args.expected_replacements,
                "file_extension": extension,
                "old_string_length": args.old_string.len(),
                "new_string_length": args.new_string.len()
            }))
        } else {
            // Mismatch - success with warning and suggestions
            let context = SuggestionContext {
                file_path: args.file_path.clone(),
                search_string: args.old_string.clone(),
                line_number: None,
                log_path: None,
                execution_time_ms: None,
            };

            let suggestion = Suggestion::for_failure(
                &EditFailureReason::UnexpectedCount {
                    expected: args.expected_replacements,
                    found: occurrence_count,
                },
                &context,
            );

            // Return success with warning and suggestions
            Ok(json!({
                "success": true,
                "warning": format!(
                    "{}\nAll {} occurrence(s) were replaced.{}{}",
                    suggestion.message,
                    occurrence_count,
                    suggestion.format(),
                    warning
                ),
                "replacements_made": occurrence_count,
                "expected": args.expected_replacements,
                "file_extension": extension,
                "old_string_length": args.old_string.len(),
                "new_string_length": args.new_string.len()
            }))
        }
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I use edit_block to make precise changes to files?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The edit_block tool performs surgical text replacements:\n\n\
                     1. Single replacement (default):\n\
                        edit_block({\n\
                          \"file_path\": \"file.txt\",\n\
                          \"old_string\": \"original text\",\n\
                          \"new_string\": \"updated text\"\n\
                        })\n\n\
                     2. Multiple replacements:\n\
                        edit_block({\n\
                          \"file_path\": \"file.txt\",\n\
                          \"old_string\": \"foo\",\n\
                          \"new_string\": \"bar\",\n\
                          \"expected_replacements\": 5\n\
                        })\n\n\
                     Best practices:\n\
                     - Include enough context to make old_string unique\n\
                     - Use expected_replacements to catch unexpected matches\n\
                     - Tool warns if actual count differs from expected\n\
                     - Empty search strings are rejected\n\
                     - Replaces ALL occurrences if expected_replacements matches\n\n\
                     The tool automatically:\n\
                     - Validates file exists and is readable\n\
                     - Performs exact string matching (not regex)\n\
                     - Preserves file encoding\n\
                     - Returns detailed success/warning messages",
                ),
            },
        ])
    }
}
