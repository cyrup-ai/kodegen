//! Inline markdown formatting cleanup

/// Clean inline formatting with comprehensive rules (legacy allocating version - prefer _inplace variant)
#[allow(dead_code)] // Library code: allocating version kept for compatibility
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
                let prev_is_word = result.chars().last().is_some_and(|c| c.is_alphanumeric());
                let next_is_word = chars.peek().is_some_and(|&c| c.is_alphanumeric());

                if !(prev_is_word && next_is_word) {
                    continue; // Skip the asterisk
                }
                result.push(ch);
            }
            '_' => {
                // Preserve underscores within words
                let prev_is_word = result.chars().last().is_some_and(|c| c.is_alphanumeric());
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
    text = text.replace("~", "");
    text = text.replace("^", "");

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

/// Clean inline formatting in-place with comprehensive rules (zero-allocation)
#[inline]
pub(crate) fn clean_inline_formatting_inplace(text: &mut String) {
    // Early return for empty or very short strings
    if text.len() < 2 {
        return;
    }

    // Handle inline code first (to preserve content)
    process_inline_code_inplace(text);

    // Handle links and images
    process_links_and_images_inplace(text);

    // Remove emphasis markers (order matters: longest first)
    remove_emphasis_markers_inplace(text);

    // Handle other inline elements
    process_other_inline_elements_inplace(text);
}

/// Process inline code blocks in-place
#[inline]
fn process_inline_code_inplace(text: &mut String) {
    let mut i = 0;
    let mut in_code = false;
    let mut backtick_count = 0;

    while i < text.len() {
        if text[i..].starts_with('`') {
            let mut count = 0;
            let mut j = i;
            while j < text.len() && text[j..].starts_with('`') {
                count += 1;
                j += 1;
            }

            if !in_code {
                in_code = true;
                backtick_count = count;
                // Remove backticks
                text.drain(i..j);
            } else if count == backtick_count {
                in_code = false;
                backtick_count = 0;
                // Replace with space
                text.replace_range(i..j, " ");
                i += 1;
            } else {
                // Keep backticks inside code
                i = j;
            }
        } else {
            i += 1;
        }
    }
}

/// Process links and images in-place
#[inline]
fn process_links_and_images_inplace(text: &mut String) {
    let mut i = 0;

    while i < text.len() {
        // Handle images
        if i + 1 < text.len() && text[i..].starts_with('!') && text[i + 1..].starts_with('[') {
            i += 1; // Skip '!'
            i += 1; // Skip '['

            // Extract alt text
            if let Some(end_bracket) = find_closing_bracket(&text[i..]) {
                let alt_text = text[i..i + end_bracket].to_string();
                let content_end = i + end_bracket + 1;

                // Skip URL if present
                let mut final_pos = content_end;
                if content_end < text.len()
                    && text[content_end..].starts_with('(')
                    && let Some(paren_end) = find_closing_paren(&text[content_end + 1..])
                {
                    final_pos = content_end + paren_end + 2;
                }

                // Replace entire image construct with alt text (or space)
                if !alt_text.is_empty() {
                    text.replace_range(i - 2..final_pos, &alt_text);
                    text.insert(i - 2 + alt_text.len(), ' ');
                    i = i - 2 + alt_text.len() + 1;
                } else {
                    text.drain(i - 2..final_pos);
                }
            } else {
                i += 1;
            }
        }
        // Handle links
        else if text[i..].starts_with('[') {
            i += 1; // Skip '['

            // Extract link text
            if let Some(end_bracket) = find_closing_bracket(&text[i..]) {
                let link_text = text[i..i + end_bracket].to_string();
                let content_end = i + end_bracket + 1;

                // Skip URL if present
                let mut final_pos = content_end;
                if content_end < text.len()
                    && text[content_end..].starts_with('(')
                    && let Some(paren_end) = find_closing_paren(&text[content_end + 1..])
                {
                    final_pos = content_end + paren_end + 2;
                }

                // Replace entire link construct with link text
                text.replace_range(i - 1..final_pos, &link_text);
                i = i - 1 + link_text.len();
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
}

/// Find closing bracket, handling nesting
#[inline]
fn find_closing_bracket(s: &str) -> Option<usize> {
    let mut depth = 1;
    let mut escaped = false;

    for (i, ch) in s.chars().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }

        match ch {
            '\\' => escaped = true,
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }

    None
}

/// Find closing parenthesis, handling nesting
#[inline]
fn find_closing_paren(s: &str) -> Option<usize> {
    let mut depth = 1;
    let mut escaped = false;

    for (i, ch) in s.chars().enumerate() {
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
                    return Some(i);
                }
            }
            _ => {}
        }
    }

    None
}

/// Remove emphasis markers in-place
#[inline]
fn remove_emphasis_markers_inplace(text: &mut String) {
    // Order matters: process longest patterns first

    // Bold + italic combinations (3 characters)
    replace_all_inplace(text, "***", "");
    replace_all_inplace(text, "___", "");
    replace_all_inplace(text, "**_", "");
    replace_all_inplace(text, "__*", "");
    replace_all_inplace(text, "_**", "");
    replace_all_inplace(text, "*__", "");

    // Bold (2 characters)
    replace_all_inplace(text, "**", "");
    replace_all_inplace(text, "__", "");

    // Italic - be careful not to remove underscores within words
    remove_italic_markers_inplace(text);
}

/// Remove italic markers while preserving intra-word underscores (in-place)
#[inline]
fn remove_italic_markers_inplace(text: &mut String) {
    let mut i = 0;

    while i < text.len() {
        let ch = text.chars().nth(i);

        match ch {
            Some('*') => {
                // Check if it's a standalone asterisk
                let prev_char = if i > 0 { text.chars().nth(i - 1) } else { None };
                let next_char = text.chars().nth(i + 1);

                let prev_is_word = prev_char.is_some_and(|c| c.is_alphanumeric());
                let next_is_word = next_char.is_some_and(|c| c.is_alphanumeric());

                if !(prev_is_word && next_is_word) {
                    text.remove(i); // Skip the asterisk
                } else {
                    i += 1;
                }
            }
            Some('_') => {
                // Preserve underscores within words
                let prev_char = if i > 0 { text.chars().nth(i - 1) } else { None };
                let next_char = text.chars().nth(i + 1);

                let prev_is_word = prev_char.is_some_and(|c| c.is_alphanumeric());
                let next_is_word = next_char.is_some_and(|c| c.is_alphanumeric());
                let prev_is_space = prev_char.is_some_and(|c| c == ' ');

                if (prev_is_word || next_is_word) && !prev_is_space {
                    i += 1; // Keep underscore
                } else if !prev_is_word && !next_is_word {
                    text.remove(i); // Standalone underscore, skip it
                } else {
                    text.remove(i); // Edge of word, skip if it's emphasis
                }
            }
            Some(_) => i += 1,
            None => break,
        }
    }
}

/// Process other inline elements in-place
#[inline]
fn process_other_inline_elements_inplace(text: &mut String) {
    use super::footnote::remove_footnote_markers_inplace;

    // Strikethrough
    replace_all_inplace(text, "~~", "");

    // Subscript and superscript
    replace_all_inplace(text, "~", "");
    replace_all_inplace(text, "^", "");

    // HTML entities (common ones)
    replace_all_inplace(text, "&nbsp;", " ");
    replace_all_inplace(text, "&amp;", "&");
    replace_all_inplace(text, "&lt;", "<");
    replace_all_inplace(text, "&gt;", ">");
    replace_all_inplace(text, "&quot;", "\"");
    replace_all_inplace(text, "&apos;", "'");
    replace_all_inplace(text, "&#39;", "'");

    // Footnote markers
    remove_footnote_markers_inplace(text);

    // Keyboard keys
    replace_all_inplace(text, "<kbd>", "");
    replace_all_inplace(text, "</kbd>", "");

    // Abbreviations
    replace_all_inplace(text, "<abbr>", "");
    replace_all_inplace(text, "</abbr>", "");
}

/// Replace all occurrences of a pattern in-place
#[inline]
fn replace_all_inplace(text: &mut String, pattern: &str, replacement: &str) {
    if pattern.is_empty() {
        return;
    }

    let mut start = 0;
    while let Some(pos) = text[start..].find(pattern) {
        let absolute_pos = start + pos;
        text.replace_range(absolute_pos..absolute_pos + pattern.len(), replacement);
        start = absolute_pos + replacement.len();
    }
}
