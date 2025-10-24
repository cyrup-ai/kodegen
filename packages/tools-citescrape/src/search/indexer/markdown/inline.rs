//! Inline markdown formatting cleanup

/// Clean inline formatting with comprehensive rules (allocating version - UTF-8 safe)
#[inline]
pub(crate) fn clean_inline_formatting(mut text: String) -> String {
    // Early return for empty or very short strings
    if text.len() < 2 {
        return text;
    }

    // Handle inline code first (to preserve content)
    text = process_inline_code(text);

    // Handle links and images
    text = process_links_and_images(text);

    // Remove emphasis markers (order matters: longest first)
    text = remove_emphasis_markers(text);

    // Handle other inline elements
    text = process_other_inline_elements(text);

    text
}

/// Process inline code blocks (legacy allocating version - prefer _inplace variant)
#[allow(dead_code)] // Library code: allocating version kept for compatibility
#[inline]
fn process_inline_code(text: String) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut in_code = false;
    let mut backtick_count = 0;

    while let Some(ch) = chars.next() {
        if ch == '`' {
            let mut count = 1;
            while chars.peek() == Some(&'`') {
                chars.next();
                count += 1;
            }

            if !in_code {
                in_code = true;
                backtick_count = count;
            } else if count == backtick_count {
                in_code = false;
                backtick_count = 0;
                result.push(' '); // Replace with space
            } else {
                // Backticks inside code
                for _ in 0..count {
                    result.push('`');
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Process links and images (legacy allocating version - prefer _inplace variant)
#[allow(dead_code)] // Library code: allocating version kept for compatibility
#[inline]
fn process_links_and_images(text: String) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '!' if chars.peek() == Some(&'[') => {
                // Image - skip the entire construct
                chars.next(); // Skip '['
                if let Some(alt_text) = extract_bracketed_content(&mut chars) {
                    // Optionally include alt text
                    if !alt_text.is_empty() {
                        result.push_str(&alt_text);
                        result.push(' ');
                    }
                }
            }
            '[' => {
                // Link - extract link text
                if let Some(link_text) = extract_bracketed_content(&mut chars) {
                    result.push_str(&link_text);
                    // Skip URL if present
                    if chars.peek() == Some(&'(') {
                        chars.next();
                        skip_parenthetical_content(&mut chars);
                    }
                } else {
                    result.push('[');
                }
            }
            _ => result.push(ch),
        }
    }

    result
}

/// Extract content within brackets, handling nesting (legacy allocating version - prefer _inplace variant)
#[allow(dead_code)] // Library code: allocating version kept for compatibility
#[inline]
fn extract_bracketed_content(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<String> {
    let mut content = String::new();
    let mut depth = 1;
    let mut escaped = false;

    for ch in chars.by_ref() {
        if escaped {
            content.push(ch);
            escaped = false;
            continue;
        }

        match ch {
            '\\' => escaped = true,
            '[' => {
                depth += 1;
                content.push(ch);
            }
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(content);
                }
                content.push(ch);
            }
            _ => content.push(ch),
        }
    }

    None
}

/// Skip content within parentheses (legacy allocating version - prefer _inplace variant)
#[allow(dead_code)] // Library code: allocating version kept for compatibility
#[inline]
fn skip_parenthetical_content(chars: &mut std::iter::Peekable<std::str::Chars>) {
    let mut depth = 1;
    let mut escaped = false;

    for ch in chars.by_ref() {
        if escaped {
            escaped = false;
            continue;
        }

        match ch {
            '\\' => escaped = true,
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return;
                }
            }
            _ => {}
        }
    }
}

/// Remove emphasis markers intelligently (legacy allocating version - prefer _inplace variant)
#[allow(dead_code)] // Library code: allocating version kept for compatibility
#[inline]
fn remove_emphasis_markers(mut text: String) -> String {
    // Order matters: process longest patterns first

    // Bold + italic combinations
    text = text.replace("***", "");
    text = text.replace("___", "");
    text = text.replace("**_", "");
    text = text.replace("__*", "");
    text = text.replace("_**", "");
    text = text.replace("*__", "");

    // Bold
    text = text.replace("**", "");
    text = text.replace("__", "");

    // Italic - be careful not to remove underscores within words
    text = remove_italic_markers(text);

    text
}

/// Remove italic markers while preserving intra-word underscores (legacy allocating version - prefer _inplace variant)
#[allow(dead_code)] // Library code: allocating version kept for compatibility
#[inline]
fn remove_italic_markers(text: String) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '*' => {
                // Check if it's a standalone asterisk
                let prev_is_word = result.chars().last().is_some_and(char::is_alphanumeric);
                let next_is_word = chars.peek().is_some_and(|&c| c.is_alphanumeric());

                if !(prev_is_word && next_is_word) {
                    continue; // Skip the asterisk
                }
                result.push(ch);
            }
            '_' => {
                // Preserve underscores within words
                let prev_is_word = result.chars().last().is_some_and(char::is_alphanumeric);
                let next_is_word = chars.peek().is_some_and(|&c| c.is_alphanumeric());

                if (prev_is_word || next_is_word) && !result.ends_with(' ') {
                    result.push(ch); // Keep underscore
                } else if !prev_is_word && !next_is_word {
                    // Standalone underscore, skip it
                    continue;
                } else {
                    // Edge of word, skip if it's emphasis
                    continue;
                }
            }
            _ => result.push(ch),
        }
    }

    result
}

/// Process other inline elements (legacy allocating version - prefer _inplace variant)
#[allow(dead_code)] // Library code: allocating version kept for compatibility
#[inline]
fn process_other_inline_elements(mut text: String) -> String {
    use super::footnote::remove_footnote_markers;

    // Strikethrough
    text = text.replace("~~", "");

    // Subscript and superscript
    text = text.replace('~', "");
    text = text.replace('^', "");

    // HTML entities (common ones)
    text = text.replace("&nbsp;", " ");
    text = text.replace("&amp;", "&");
    text = text.replace("&lt;", "<");
    text = text.replace("&gt;", ">");
    text = text.replace("&quot;", "\"");
    text = text.replace("&apos;", "'");
    text = text.replace("&#39;", "'");

    // Footnote markers
    text = remove_footnote_markers(text);

    // Keyboard keys
    text = text.replace("<kbd>", "");
    text = text.replace("</kbd>", "");

    // Abbreviations
    text = text.replace("<abbr>", "");
    text = text.replace("</abbr>", "");

    text
}

// ============================================================================
// IN-PLACE VERSIONS (ZERO-ALLOCATION) - Option 3: Hybrid Approach
// ============================================================================
