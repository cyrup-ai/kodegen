//! Markdown processing and heading normalization functionality.
//!
//! Processes markdown content by normalizing headings and handling various markdown formats.

/// Pre-built heading prefixes to avoid repeated string allocations
const HEADING_PREFIXES: [&str; 6] = ["# ", "## ", "### ", "#### ", "##### ", "###### "];

/// Code fence state to track fence type and character count
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CodeFence {
    char: char,         // '`' or '~'
    count: usize,       // Number of characters in the fence
    line_number: usize, // Line number where the fence opened
}

/// Detect code fence marker at the start of a line
/// Returns Some((char, count)) if the line starts with 3+ backticks or tildes
fn detect_code_fence(line: &str) -> Option<(char, usize)> {
    let trimmed = line.trim_start();

    // Check for backticks
    if trimmed.starts_with('`') {
        let count = trimmed.chars().take_while(|&c| c == '`').count();
        if count >= 3 {
            return Some(('`', count));
        }
    }

    // Check for tildes
    if trimmed.starts_with('~') {
        let count = trimmed.chars().take_while(|&c| c == '~').count();
        if count >= 3 {
            return Some(('~', count));
        }
    }

    None
}

/// Detect if a line looks like code based on common code patterns
/// Used for heuristic recovery of unclosed code fences
fn looks_like_code(line: &str) -> bool {
    let trimmed = line.trim();

    // Existing checks
    if trimmed.ends_with(';')
        || trimmed.ends_with('{')
        || trimmed.ends_with('}')
        || trimmed.contains("return ")
        || trimmed.contains("function ")
        || trimmed.contains("def ")
        || trimmed.starts_with("import ")
        || trimmed.starts_with("from ")
    {
        return true;
    }

    // NEW: Detect function/method calls with parentheses
    if trimmed.contains('(') && trimmed.contains(')') {
        return true;
    }

    // NEW: Detect variable assignments and operators
    if trimmed.contains(" = ")
        || trimmed.contains("const ")
        || trimmed.contains("let ")
        || trimmed.contains("var ")
    {
        return true;
    }

    // NEW: Detect array/object access
    if trimmed.contains('[') || trimmed.contains(']') {
        return true;
    }

    // Detect C-style comments only (avoid confusion with markdown headings)
    if trimmed.starts_with("//") {
        return true;
    }

    // NEW: Indented lines are likely code continuation
    if line.starts_with("    ") || line.starts_with('\t') {
        return true;
    }

    false
}

/// Process markdown headings to normalize heading levels and handle different markdown styles
pub fn process_markdown_headings(markdown: &str) -> String {
    let lines: Vec<&str> = markdown.lines().collect();

    // Process all lines
    let mut processed_lines = Vec::new();
    let mut fence_stack: Option<CodeFence> = None;

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim_start();

        // Track code blocks to avoid processing headings inside them
        if let Some((fence_char, fence_count)) = detect_code_fence(trimmed) {
            if let Some(ref current_fence) = fence_stack {
                // We're inside a code block - check if this closes it
                if fence_char == current_fence.char && fence_count >= current_fence.count {
                    // This closes the current fence
                    fence_stack = None;
                }
            } else {
                // This opens a new code fence
                fence_stack = Some(CodeFence {
                    char: fence_char,
                    count: fence_count,
                    line_number: i,
                });
            }
            processed_lines.push(line.to_string());
            i += 1;
            continue;
        }

        if fence_stack.is_none() {
            // Check if this is a setext-style heading (text followed by === or ---)
            if i + 1 < lines.len() {
                let next_line = lines[i + 1];
                let next_trimmed = next_line.trim();

                if !line.trim().is_empty()
                    && (next_trimmed.chars().all(|c| c == '=')
                        || next_trimmed.chars().all(|c| c == '-'))
                    && !next_trimmed.is_empty()
                {
                    // This is a setext heading - convert to ATX style
                    let level = if next_trimmed.chars().all(|c| c == '=') {
                        1
                    } else {
                        2
                    };
                    let normalized_level = normalize_heading_level(level);
                    let new_heading =
                        format!("{}{}", HEADING_PREFIXES[normalized_level - 1], line.trim());
                    processed_lines.push(new_heading);
                    i += 2; // Skip both the heading line and the underline
                    continue;
                }
            }

            // Check for ATX-style headings
            if let Some(heading) = extract_heading_level(trimmed) {
                let (level, content) = heading;
                let normalized_level = normalize_heading_level(level);
                let new_heading = format!("{}{}", HEADING_PREFIXES[normalized_level - 1], content);
                processed_lines.push(new_heading);
            } else {
                processed_lines.push(line.to_string());
            }
        } else {
            // Inside a code fence - preserve the line as-is
            processed_lines.push(line.to_string());
        }

        i += 1;
    }

    // Auto-close fence if still open (best-effort recovery)
    if let Some(fence) = fence_stack {
        tracing::warn!(
            "Unclosed code fence starting at line {} (char: '{}', count: {}), attempting recovery",
            fence.line_number,
            fence.char,
            fence.count
        );

        // Strategy: Look backwards from end to find last code-like line
        let mut last_code_idx = processed_lines.len().saturating_sub(1);
        for (idx, line) in processed_lines.iter().enumerate().rev() {
            if looks_like_code(line) {
                last_code_idx = idx;
                break;
            }
        }

        // Insert closing fence after last code line
        let closing = fence.char.to_string().repeat(fence.count);
        processed_lines.insert(last_code_idx + 1, closing);

        tracing::info!(
            "Auto-closed fence at line {} (after last code-like content)",
            last_code_idx + 1
        );
    }

    processed_lines.join("\n")
}

/// Extract heading level and content from a markdown line
///
/// Removes optional closing hashes from ATX headings per `CommonMark` spec.
/// For example: `## Title ##` becomes `## Title`
#[must_use]
pub fn extract_heading_level(line: &str) -> Option<(usize, &str)> {
    // Match markdown headings (# to ######)
    if line.starts_with('#') {
        let level = line.chars().take_while(|&c| c == '#').count();
        if level > 0 && level <= 6 {
            let content = line[level..].trim_start();

            // Remove optional closing hashes (must be preceded by whitespace per CommonMark)
            // Strategy: scan from right, skip hashes, then skip whitespace
            // If we skipped both, we have a valid closing sequence

            let bytes = content.as_bytes();
            let mut end = bytes.len();
            let mut hash_start = end;

            // Skip trailing hashes
            while hash_start > 0 && bytes[hash_start - 1] == b'#' {
                hash_start -= 1;
            }

            // If we found trailing hashes, check for preceding whitespace
            if hash_start < end {
                let mut ws_start = hash_start;
                // Skip whitespace before the hashes
                while ws_start > 0 && bytes[ws_start - 1].is_ascii_whitespace() {
                    ws_start -= 1;
                }

                // If there was whitespace between content and hashes, remove both
                // OR if all of content is hashes (hash_start == 0), return empty
                if ws_start < hash_start || hash_start == 0 {
                    end = ws_start;
                }
            }

            return Some((level, &content[..end]));
        }
    }
    None
}

/// Normalize heading level to ensure it's within valid range (1-6)
#[must_use]
pub fn normalize_heading_level(level: usize) -> usize {
    // Ensure headings are in the range 1-6
    level.clamp(1, 6)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_heading_level_with_closing_hashes() {
        // Normal case with closing hashes
        let result = extract_heading_level("## Title ##");
        assert_eq!(result, Some((2, "Title")));

        // H1 with closing hashes
        let result = extract_heading_level("# Main Heading #");
        assert_eq!(result, Some((1, "Main Heading")));

        // H3 with closing hashes
        let result = extract_heading_level("### Section ###");
        assert_eq!(result, Some((3, "Section")));
    }

    #[test]
    fn test_extract_heading_level_unmatched_closing_hashes() {
        // More closing hashes than opening (valid per CommonMark)
        let result = extract_heading_level("## Title #####");
        assert_eq!(result, Some((2, "Title")));

        // Fewer closing hashes than opening
        let result = extract_heading_level("#### Title ##");
        assert_eq!(result, Some((4, "Title")));
    }

    #[test]
    fn test_extract_heading_level_hash_in_content() {
        // Content ends with hash (e.g., C#)
        let result = extract_heading_level("## C# ##");
        assert_eq!(result, Some((2, "C#")));

        // Multiple hashes in content
        let result = extract_heading_level("## Use C# and F# ##");
        assert_eq!(result, Some((2, "Use C# and F#")));

        // Hash not at end
        let result = extract_heading_level("## Title#content ##");
        assert_eq!(result, Some((2, "Title#content")));
    }

    #[test]
    fn test_extract_heading_level_no_closing_hashes() {
        // Normal heading without closing hashes
        let result = extract_heading_level("## Title");
        assert_eq!(result, Some((2, "Title")));

        // With extra spaces
        let result = extract_heading_level("###  Spaced Title  ");
        assert_eq!(result, Some((3, "Spaced Title  ")));
    }

    #[test]
    fn test_extract_heading_level_edge_cases() {
        // Single character content
        let result = extract_heading_level("## A ##");
        assert_eq!(result, Some((2, "A")));

        // Empty content (just hashes)
        // Should return empty content, not None
        // Actually, with our implementation, rfind returns None for all-hash/whitespace
        let result = extract_heading_level("## ##");
        assert_eq!(result, Some((2, "")));

        // Maximum heading level
        let result = extract_heading_level("###### Level 6 ######");
        assert_eq!(result, Some((6, "Level 6")));
    }

    #[test]
    fn test_extract_heading_level_no_space_before_closing() {
        // Per CommonMark, closing hashes MUST be preceded by whitespace
        // Without space, the trailing ## is part of the content
        let result = extract_heading_level("##Title##");
        assert_eq!(result, Some((2, "Title##")));
    }

    #[test]
    fn test_extract_heading_level_invalid_headings() {
        // Not a heading
        let result = extract_heading_level("Regular text");
        assert_eq!(result, None);

        // Too many hashes (> 6)
        let result = extract_heading_level("####### Not a heading");
        assert_eq!(result, None);

        // Empty string
        let result = extract_heading_level("");
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_heading_level_whitespace_variations() {
        // Multiple spaces before closing hashes
        let result = extract_heading_level("## Title     ##");
        assert_eq!(result, Some((2, "Title")));

        // Tab before closing hashes
        let result = extract_heading_level("## Title\t##");
        assert_eq!(result, Some((2, "Title")));

        // Mixed whitespace
        let result = extract_heading_level("##  Title  \t ##");
        assert_eq!(result, Some((2, "Title")));
    }

    #[test]
    fn test_process_markdown_preserves_content_before_h1() {
        // Test that content before the first H1 is preserved (bug fix for HTML_CLEANER_15)
        let markdown = "This is an introduction paragraph.\n\nSome important information here.\n\n# First Heading\n\nArticle content...";
        let result = process_markdown_headings(markdown);

        // Verify introduction is preserved
        assert!(result.contains("This is an introduction paragraph."));
        assert!(result.contains("Some important information here."));
        assert!(result.contains("# First Heading"));
        assert!(result.contains("Article content..."));
    }

    #[test]
    fn test_process_markdown_with_no_h1() {
        // Test that all content is preserved when there's no H1
        let markdown = "Some content\n\n## Subheading\n\nMore content";
        let result = process_markdown_headings(markdown);

        assert!(result.contains("Some content"));
        assert!(result.contains("## Subheading"));
        assert!(result.contains("More content"));
    }

    #[test]
    fn test_process_markdown_with_setext_h1() {
        // Test that content before setext-style H1 is preserved
        let markdown = "Introduction text\n\nFirst Heading\n=============\n\nContent after heading";
        let result = process_markdown_headings(markdown);

        assert!(result.contains("Introduction text"));
        assert!(result.contains("# First Heading")); // Should be converted to ATX style
        assert!(result.contains("Content after heading"));
    }

    #[test]
    fn test_tilde_code_fence_tracking() {
        // Test that headings inside tilde code fences are not processed
        let markdown = "# Real Heading\n\n~~~\n# This is not a heading, it's code\n## Also not a heading\n~~~\n\n# Another Real Heading";
        let result = process_markdown_headings(markdown);

        // Real headings should be normalized
        assert!(result.contains("# Real Heading"));
        assert!(result.contains("# Another Real Heading"));

        // Headings inside tilde fence should remain unchanged
        assert!(result.contains("# This is not a heading, it's code"));
        assert!(result.contains("## Also not a heading"));

        // The fence markers should be preserved
        assert!(result.contains("~~~"));
    }

    #[test]
    fn test_mixed_backtick_and_tilde_fences() {
        // Test that backtick and tilde fences work independently
        let markdown = "# Heading\n\n```\n## Not a heading (backticks)\n```\n\n~~~\n### Not a heading (tildes)\n~~~\n\n## Real Heading 2";
        let result = process_markdown_headings(markdown);

        // Real headings should be present
        assert!(result.contains("# Heading"));
        assert!(result.contains("## Real Heading 2"));

        // Code inside fences should be unchanged
        assert!(result.contains("## Not a heading (backticks)"));
        assert!(result.contains("### Not a heading (tildes)"));
    }

    #[test]
    fn test_tilde_fence_does_not_close_backtick_fence() {
        // Test that tilde fence inside backtick fence is treated as content
        let markdown = "```\n~~~\n# Not a heading\n~~~\n```\n\n# Real Heading";
        let result = process_markdown_headings(markdown);

        // The heading inside should remain unchanged because it's inside backtick fence
        assert!(result.contains("# Not a heading"));
        assert!(result.contains("# Real Heading"));

        // Both fence types should be preserved
        assert!(result.contains("```"));
        assert!(result.contains("~~~"));
    }

    #[test]
    fn test_backtick_fence_does_not_close_tilde_fence() {
        // Test that backtick fence inside tilde fence is treated as content
        let markdown = "~~~\n```\n## Not a heading\n```\n~~~\n\n## Real Heading";
        let result = process_markdown_headings(markdown);

        // The heading inside should remain unchanged because it's inside tilde fence
        assert!(result.contains("## Not a heading"));
        assert!(result.contains("## Real Heading"));

        // Both fence types should be preserved
        assert!(result.contains("```"));
        assert!(result.contains("~~~"));
    }

    #[test]
    fn test_unbalanced_code_fence_with_proper_tracking() {
        // Test the bug fix for HTML_CLEANER_17: unbalanced code fence should not break processing
        let markdown = "# Real Heading 1\n\nSome content\n\n```\nThis code block is never closed\n\n## Heading 2 - Should NOT be processed (inside fence)\n\n### Heading 3 - Should NOT be processed (inside fence)\n\nMore content";
        let result = process_markdown_headings(markdown);

        // First heading should be normalized
        assert!(result.contains("# Real Heading 1"));

        // Content should be preserved
        assert!(result.contains("Some content"));
        assert!(result.contains("This code block is never closed"));
        assert!(result.contains("More content"));

        // Headings inside unclosed fence should remain unchanged (not processed)
        assert!(result.contains("## Heading 2 - Should NOT be processed (inside fence)"));
        assert!(result.contains("### Heading 3 - Should NOT be processed (inside fence)"));

        // Opening fence should be preserved
        assert!(result.contains("```"));
    }

    #[test]
    fn test_fence_count_matching() {
        // Test that closing fence must have at least as many characters as opening fence
        let markdown = "# Heading 1\n\n`````\nCode block with 5 backticks\n```\nThis does NOT close the fence (only 3 backticks)\n## Not a heading\n```\nStill not closed\n`````\nThis closes it (5 backticks)\n\n## Real Heading 2";
        let result = process_markdown_headings(markdown);

        // Real headings should be normalized
        assert!(result.contains("# Heading 1"));
        assert!(result.contains("## Real Heading 2"));

        // Heading inside the 5-backtick fence should not be processed
        assert!(result.contains("## Not a heading"));

        // All fence markers should be preserved
        assert!(result.contains("`````"));
    }

    #[test]
    fn test_different_fence_types_do_not_match() {
        // Test that backticks don't close tilde fences and vice versa
        let markdown = "# Heading\n\n~~~\nOpened with tildes\n```\nThis does not close it (different char)\n## Not a heading\n```\n~~~\nNow it's closed\n\n## Real Heading";
        let result = process_markdown_headings(markdown);

        // Real headings should be normalized
        assert!(result.contains("# Heading"));
        assert!(result.contains("## Real Heading"));

        // Heading inside tilde fence should not be processed
        assert!(result.contains("## Not a heading"));
    }

    #[test]
    fn test_unclosed_fence_recovery() {
        let markdown = r#"
# Introduction

```python
def example():
    print("Hello")
# Missing closing fence

## Section 2

Real content.
"#;

        let result = process_markdown_headings(markdown);

        // Should auto-close fence (check for closing fence added)
        let fence_count = result.matches("```").count();
        assert!(
            fence_count >= 2,
            "Should have both opening and auto-closed fence"
        );

        // NEW: Verify print statement stays inside fence
        let lines: Vec<&str> = result.lines().collect();
        let opening_idx = lines
            .iter()
            .position(|l| l.contains("```python"))
            .expect("Test operation should succeed");
        let closing_idx = lines
            .iter()
            .rposition(|l| l.trim() == "```")
            .expect("Test operation should succeed");
        let print_idx = lines
            .iter()
            .position(|l| l.contains("print("))
            .expect("Test operation should succeed");

        assert!(
            print_idx > opening_idx && print_idx < closing_idx,
            "print() line should be between fence markers, but fence closes at line {closing_idx} and print is at line {print_idx}"
        );

        // Should process subsequent headings
        assert!(result.contains("## Section 2"));

        // Content should be preserved
        assert!(result.contains("Real content."));
    }

    #[test]
    fn test_unclosed_fence_at_end() {
        let markdown = "# Title\n\n```python\ncode\n";
        let result = process_markdown_headings(markdown);

        // Should auto-close fence at end
        let fence_count = result.matches("```").count();
        assert_eq!(
            fence_count, 2,
            "Should have both opening and auto-closed fence"
        );

        // Title should be preserved
        assert!(result.contains("# Title"));
    }

    #[test]
    fn test_unclosed_fence_detects_various_code_patterns() {
        // Test that fence closes after different code patterns
        let test_cases = vec![
            ("```js\nconsole.log('test')\n\n## Heading", "function call"),
            ("```python\nx = 5\n\n## Heading", "assignment"),
            ("```js\narr[0] = 1\n\n## Heading", "array access"),
            ("```\n    indented line\n\n## Heading", "indented code"),
            ("```python\n# code comment\n\n## Heading", "comment"),
        ];

        for (markdown, description) in test_cases {
            let result = process_markdown_headings(markdown);
            let fence_count = result.matches("```").count();
            assert_eq!(
                fence_count, 2,
                "Should auto-close fence for {description} pattern"
            );
        }
    }

    #[test]
    fn test_unclosed_fence_with_heading_after_code() {
        let markdown = r"
```python
def foo():
    return 42

# Next Section

This is prose text.
";

        let result = process_markdown_headings(markdown);

        // Should auto-close fence
        let fence_count = result.matches("```").count();
        assert_eq!(
            fence_count, 2,
            "Should have both opening and auto-closed fence"
        );

        // Verify heading is NOT inside the fence
        let lines: Vec<&str> = result.lines().collect();
        let closing_idx = lines
            .iter()
            .rposition(|l| l.trim() == "```")
            .expect("Test operation should succeed");
        let heading_idx = lines
            .iter()
            .position(|l| l.contains("# Next Section"))
            .expect("Test operation should succeed");

        assert!(
            heading_idx > closing_idx,
            "Heading should be AFTER closing fence (outside fence), but fence closes at line {closing_idx} and heading is at line {heading_idx}"
        );
    }
}
