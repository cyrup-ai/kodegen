//! Helper functions for markdown processing

/// Check if a line is a horizontal rule
#[inline]
pub(crate) fn is_horizontal_rule(line: &str) -> bool {
    let trimmed = line.trim();

    // Must be at least 3 characters
    if trimmed.len() < 3 {
        return false;
    }

    // Check different HR patterns
    let patterns = [
        (trimmed.chars().all(|c| c == '-' || c == ' '), '-'),
        (trimmed.chars().all(|c| c == '*' || c == ' '), '*'),
        (trimmed.chars().all(|c| c == '_' || c == ' '), '_'),
    ];

    for (all_match, marker) in patterns {
        if all_match && trimmed.chars().filter(|&c| c == marker).count() >= 3 {
            return true;
        }
    }

    false
}

/// Check if a tag is an HTML block element
#[inline]
pub(crate) fn is_html_block_tag(tag: &str) -> bool {
    let block_tags = [
        "<div",
        "<p",
        "<h1",
        "<h2",
        "<h3",
        "<h4",
        "<h5",
        "<h6",
        "<blockquote",
        "<pre",
        "<ul",
        "<ol",
        "<li",
        "<table",
        "<thead",
        "<tbody",
        "<tr",
        "<td",
        "<th",
        "<form",
        "<article",
        "<section",
        "<nav",
        "<aside",
        "<header",
        "<footer",
        "<main",
        "<figure",
        "<figcaption",
    ];

    let lower = tag.to_lowercase();
    block_tags.iter().any(|&t| lower.starts_with(t))
}

/// Check if a tag is a closing HTML tag
#[inline]
pub(crate) fn is_closing_html_tag(tag: &str) -> bool {
    tag.trim().starts_with("</")
}

/// Normalize whitespace in the final output
#[inline]
pub(crate) fn normalize_whitespace(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut last_was_space = true; // Start true to trim leading spaces

    for ch in text.chars() {
        if ch.is_whitespace() {
            if !last_was_space {
                result.push(' ');
                last_was_space = true;
            }
        } else {
            result.push(ch);
            last_was_space = false;
        }
    }

    // Trim trailing space
    if result.ends_with(' ') {
        result.pop();
    }

    result
}

/// Remove list markers with proper nesting support (legacy allocating version - prefer _inplace variant)
#[allow(dead_code)] // Library code: allocating version kept for compatibility
#[inline]
pub(crate) fn remove_list_marker(
    line: &str,
    indent_level: usize,
    list_depth: &mut usize,
) -> String {
    let trimmed = line.trim_start();

    // Unordered list markers
    for marker in &["- ", "* ", "+ "] {
        if let Some(content) = trimmed.strip_prefix(marker) {
            *list_depth = (indent_level / 2) + 1;
            return content.to_string();
        }
    }

    // Ordered list markers (with various formats)
    if let Some(dot_pos) = trimmed.find(". ") {
        let prefix = &trimmed[..dot_pos];
        if prefix.chars().all(char::is_numeric) && !prefix.is_empty() && prefix.len() <= 9 {
            *list_depth = (indent_level / 2) + 1;
            return trimmed[dot_pos + 2..].to_string();
        }
    }

    // Parenthetical lists: 1) or a)
    if let Some(paren_pos) = trimmed.find(") ") {
        let prefix = &trimmed[..paren_pos];
        if (prefix.chars().all(char::is_numeric)
            || (prefix.len() == 1 && prefix.chars().next().is_some_and(char::is_alphabetic)))
            && !prefix.is_empty()
        {
            *list_depth = (indent_level / 2) + 1;
            return trimmed[paren_pos + 2..].to_string();
        }
    }

    // Task lists
    for marker in &[
        "- [ ] ", "- [x] ", "- [X] ", "* [ ] ", "* [x] ", "* [X] ", "+ [ ] ", "+ [x] ", "+ [X] ",
    ] {
        if let Some(content) = trimmed.strip_prefix(marker) {
            *list_depth = (indent_level / 2) + 1;
            return content.to_string();
        }
    }

    line.to_string()
}

/// Remove list markers in-place (zero-allocation version)
#[inline]
pub(crate) fn remove_list_marker_inplace(
    buffer: &mut String,
    indent_level: usize,
    list_depth: &mut usize,
) {
    let trimmed_start_len = buffer.trim_start().len();
    if trimmed_start_len == 0 {
        return;
    }

    let leading_space_count = buffer.len() - trimmed_start_len;

    // Helper function to check and remove a prefix
    let mut check_and_remove = |marker: &str| -> bool {
        if buffer[leading_space_count..].starts_with(marker) {
            *list_depth = (indent_level / 2) + 1;
            buffer.drain(..leading_space_count + marker.len());
            return true;
        }
        false
    };

    // Unordered list markers
    for marker in &["- ", "* ", "+ "] {
        if check_and_remove(marker) {
            return;
        }
    }

    // Task lists (check before simple markers)
    for marker in &[
        "- [ ] ", "- [x] ", "- [X] ", "* [ ] ", "* [x] ", "* [X] ", "+ [ ] ", "+ [x] ", "+ [X] ",
    ] {
        if check_and_remove(marker) {
            return;
        }
    }

    // Ordered list markers (with various formats)
    let trimmed = &buffer[leading_space_count..];
    if let Some(dot_pos) = trimmed.find(". ") {
        let prefix = &trimmed[..dot_pos];
        if prefix.chars().all(char::is_numeric) && !prefix.is_empty() && prefix.len() <= 9 {
            *list_depth = (indent_level / 2) + 1;
            buffer.drain(..leading_space_count + dot_pos + 2);
            return;
        }
    }

    // Parenthetical lists: 1) or a)
    if let Some(paren_pos) = trimmed.find(") ") {
        let prefix = &trimmed[..paren_pos];
        if (prefix.chars().all(char::is_numeric)
            || (prefix.len() == 1 && prefix.chars().next().is_some_and(char::is_alphabetic)))
            && !prefix.is_empty()
        {
            *list_depth = (indent_level / 2) + 1;
            buffer.drain(..leading_space_count + paren_pos + 2);
        }
    }
}
