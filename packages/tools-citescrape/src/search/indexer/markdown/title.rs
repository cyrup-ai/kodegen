//! Title extraction from markdown content

use imstr::ImString;

/// Extract title from markdown with full edge case handling
#[inline]
pub(crate) fn extract_title_from_markdown_optimized(markdown: &str) -> ImString {
    if markdown.is_empty() {
        return ImString::from("Untitled");
    }

    let mut lines = markdown.lines().take(20); // Check more lines for edge cases
    let mut in_code_block = false;
    let mut in_html_comment = false;
    let mut _potential_setext = None; // Prefix with underscore to silence warning

    while let Some(line) = lines.next() {
        let trimmed = line.trim();

        // Handle code blocks
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code_block = !in_code_block;
            continue;
        }

        if in_code_block {
            continue;
        }

        // Handle HTML comments
        if trimmed.starts_with("<!--") {
            in_html_comment = true;
        }
        if in_html_comment {
            if trimmed.ends_with("-->") {
                in_html_comment = false;
            }
            continue;
        }

        // Skip empty lines
        if trimmed.is_empty() {
            _potential_setext = None;
            continue;
        }

        // ATX headers with various levels
        for level in 1..=6 {
            let prefix = "#".repeat(level) + " ";
            if let Some(header) = trimmed.strip_prefix(&prefix) {
                // Clean inline formatting
                return clean_header_text(header);
            }
        }

        // Check for setext headers (next line)
        if let Some(next_line) = lines.clone().next() {
            let next_trimmed = next_line.trim();
            if next_trimmed.chars().all(|c| c == '=') && next_trimmed.len() >= 3 {
                // H1 setext
                return clean_header_text(trimmed);
            } else if next_trimmed.chars().all(|c| c == '-') && next_trimmed.len() >= 3 {
                // H2 setext
                return clean_header_text(trimmed);
            }
        }

        // Store potential setext header
        if !trimmed.starts_with('>')
            && !trimmed.starts_with('*')
            && !trimmed.starts_with('-')
            && !trimmed.starts_with('+')
            && !trimmed.starts_with(|c: char| c.is_numeric())
        {
            _potential_setext = Some(trimmed);
        }
    }

    // Fallback to first substantial non-empty line
    markdown
        .lines()
        .find(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && 
            !trimmed.starts_with("```") &&
            !trimmed.starts_with("~~~") &&
            !trimmed.starts_with("<!--") &&
            !trimmed.starts_with('>') && // Skip blockquotes
            trimmed.len() > 3 // Skip very short lines
        })
        .map(|line| {
            let cleaned = clean_header_text(line.trim());
            if cleaned.len() > 80 {
                // Smart truncation at word boundary
                let boundary = cleaned.as_str()[..77]
                    .rfind(|c: char| c.is_whitespace() || "-,;:".contains(c))
                    .unwrap_or(77);
                ImString::from(format!("{}...", &cleaned.as_str()[..boundary].trim_end()))
            } else {
                cleaned
            }
        })
        .unwrap_or_else(|| ImString::from("Untitled"))
}

/// Clean inline formatting from header text
#[inline]
pub(crate) fn clean_header_text(text: &str) -> ImString {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut in_link = false;
    let mut in_code = false;
    let mut skip_next = false;

    while let Some(ch) = chars.next() {
        if skip_next {
            skip_next = false;
            continue;
        }

        match ch {
            '\\' => {
                // Escape character - include next character literally
                if let Some(&next_ch) = chars.peek() {
                    chars.next();
                    result.push(next_ch);
                }
            }
            '`' => {
                // Toggle inline code
                in_code = !in_code;
                if !in_code {
                    result.push(' '); // Replace with space
                }
            }
            '[' if !in_code => {
                in_link = true;
            }
            ']' if !in_code && in_link => {
                // Skip the URL part
                if chars.peek() == Some(&'(') {
                    chars.next(); // Skip '('
                    let mut depth = 1;
                    while depth > 0 && chars.peek().is_some() {
                        match chars.next() {
                            Some('(') => depth += 1,
                            Some(')') => depth -= 1,
                            Some('\\') => {
                                chars.next();
                            } // Skip escaped char
                            _ => {}
                        }
                    }
                }
                in_link = false;
            }
            '*' | '_' if !in_code => {
                // Skip emphasis markers
                let emphasis_char = ch;
                let mut count = 1;
                while chars.peek() == Some(&emphasis_char) {
                    chars.next();
                    count += 1;
                }
                // Don't add space for single * or _ (might be in word)
                if count > 1 {
                    result.push(' ');
                }
            }
            '~' if !in_code && chars.peek() == Some(&'~') => {
                // Strikethrough
                chars.next();
            }
            _ => {
                if !in_link || in_code {
                    result.push(ch);
                }
            }
        }
    }

    // Clean up multiple spaces
    ImString::from(result.split_whitespace().collect::<Vec<_>>().join(" "))
}
