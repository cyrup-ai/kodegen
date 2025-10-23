//! Snippet generation with UTF-8 safety

use imstr::ImString;

/// Generate snippet with zero-allocation string operations and full UTF-8 safety
#[inline]
pub(crate) fn generate_snippet_optimized(text: &str, max_length: usize) -> ImString {
    // Handle edge cases
    if text.is_empty() || max_length == 0 {
        return ImString::new();
    }

    if max_length < 4 {
        // Too short for ellipsis, return what we can
        return ImString::from(text.chars().take(max_length).collect::<String>());
    }

    // Fast path: text fits within limit
    if text.len() <= max_length {
        return ImString::from(text);
    }

    // Find the UTF-8 safe boundary
    let mut boundary = max_length;
    while boundary > 0 && !text.is_char_boundary(boundary) {
        boundary -= 1;
    }

    if boundary == 0 {
        // Pathological case: no valid boundary found
        return ImString::from("...");
    }

    // Get the truncated portion safely
    let truncated = &text[..boundary];

    // Look for sentence endings - expanded set
    const SENTENCE_ENDINGS: &[&str] = &[
        ". ", "! ", "? ", ".\n", "!\n", "?\n", ".\"", "!\"", "?\"", ".'", "!'", "?'", ".)", "!)",
        "?)", ".]", "!]", "?]", "...", "\u{2026}", // Ellipsis patterns
    ];

    // Find best sentence boundary
    let mut best_pos = None;
    let mut best_ending_len = 0;

    for ending in SENTENCE_ENDINGS {
        if let Some(pos) = truncated.rfind(ending) {
            // Ensure we don't cut off the ending itself
            let end_pos = pos + ending.trim_end().len();
            if end_pos <= boundary && (best_pos.is_none() || pos > best_pos.unwrap_or(0)) {
                best_pos = Some(pos);
                best_ending_len = ending.trim_end().len();
            }
        }
    }

    if let Some(pos) = best_pos {
        // Return up to sentence ending
        return ImString::from(&text[..pos + best_ending_len]);
    }

    // Find word boundary - comprehensive set
    const WORD_BOUNDARIES: &[char] = &[
        ' ', '\t', '\n', '\r', // Whitespace
        '.', ',', ';', ':', '!', '?', // Punctuation
        '-', '\u{2013}', '\u{2014}', '_', // Dashes and underscores
        '/', '\\', '|', // Separators
        '(', ')', '[', ']', '{', '}', // Brackets
        '<', '>', // Angle brackets
        '"', '\'', '`', // Quotes
    ];

    // Find the last word boundary
    let mut last_boundary = 0;
    let mut char_indices = truncated.char_indices().peekable();

    while let Some((_idx, ch)) = char_indices.next() {
        if WORD_BOUNDARIES.contains(&ch) {
            // Look ahead to ensure we're not in the middle of a multi-char boundary
            if let Some(&(next_idx, _)) = char_indices.peek() {
                last_boundary = next_idx;
            } else {
                last_boundary = truncated.len();
            }
        }
    }

    if last_boundary > 0 && last_boundary < truncated.len() {
        // Found a good word boundary
        return ImString::from(format!("{}...", &text[..last_boundary]));
    }

    // Last resort: cut at character boundary
    let char_boundary = text
        .char_indices()
        .take_while(|(idx, _)| *idx < max_length - 3)
        .last()
        .map_or(0, |(idx, ch)| idx + ch.len_utf8());

    if char_boundary > 0 {
        ImString::from(format!("{}...", &text[..char_boundary]))
    } else {
        // Extreme edge case: even one character is too long
        ImString::from("...")
    }
}
