//! Footnote marker removal using regex

use regex::Regex;

// Thread-local regex for footnote removal
thread_local! {
    static FOOTNOTE_REGEX: std::cell::RefCell<Option<Regex>> = const { std::cell::RefCell::new(None) };
}

/// Remove footnote markers `[^1]` style (legacy allocating version - prefer _inplace variant)
#[allow(dead_code)] // Library code: allocating version kept for compatibility
pub(crate) fn remove_footnote_markers(text: String) -> String {
    FOOTNOTE_REGEX.with(|re_cell| {
        let mut re_ref = re_cell.borrow_mut();

        // Initialize regex if not already present
        if re_ref.is_none() {
            *re_ref = Regex::new(r"\[\^[^\]]+\]").ok();
        }

        // Use regex if available, otherwise return original text
        match re_ref.as_ref() {
            Some(re) => re.replace_all(&text, "").into_owned(),
            None => text, // Return original if regex compilation failed
        }
    })
}
