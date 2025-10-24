//! Ripgrep JSON Lines output parsing
//!
//! Parses JSON Lines format from `grep::printer::JSON` and converts
//! to our `SearchResult` type.
//!
//! NOTE: Many fields are unused but required for serde deserialization
//! of ripgrep's JSON output format.

use crate::search::types::{SearchResult, SearchResultType};
use serde::Deserialize;

/// Root message type from ripgrep JSON output
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[allow(dead_code)]
pub(crate) enum RipgrepMessage {
    #[serde(rename = "match")]
    Match { data: MatchData },
    #[serde(rename = "context")]
    Context { data: ContextData },
    #[serde(rename = "begin")]
    Begin { data: BeginData },
    #[serde(rename = "end")]
    End { data: EndData },
}

#[derive(Debug, Deserialize)]
pub(crate) struct MatchData {
    pub path: Option<PathData>,
    pub line_number: Option<u64>,
    pub submatches: Vec<SubMatch>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ContextData {
    pub path: Option<PathData>,
    pub lines: Lines,
    pub line_number: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PathData {
    pub text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Lines {
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SubMatch {
    pub r#match: MatchText,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MatchText {
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct BeginData {}

#[derive(Debug, Deserialize)]
pub(crate) struct EndData {}

/// Parse JSON Lines buffer from ripgrep into `SearchResult` entries
pub(crate) fn parse_json_buffer(buffer: &[u8]) -> anyhow::Result<Vec<SearchResult>> {
    let mut results = Vec::new();

    // JSON Lines format: one JSON object per line
    for line in buffer.split(|&b| b == b'\n') {
        if line.is_empty() {
            continue;
        }

        let msg: RipgrepMessage = serde_json::from_slice(line)?;

        match msg {
            RipgrepMessage::Match { data } => {
                let file = data
                    .path
                    .and_then(|p| p.text)
                    .unwrap_or_else(|| "<stdin>".to_string());

                results.push(SearchResult {
                    file,
                    line: data.line_number.map(|n| n as u32),
                    r#match: data.submatches.first().map(|sm| sm.r#match.text.clone()),
                    r#type: SearchResultType::Content,
                    is_context: false,
                    is_binary: None,
                    binary_suppressed: None,
                    modified: None,
                    accessed: None,
                    created: None,
                });
            }
            RipgrepMessage::Context { data } => {
                let file = data
                    .path
                    .and_then(|p| p.text)
                    .unwrap_or_else(|| "<stdin>".to_string());

                results.push(SearchResult {
                    file,
                    line: data.line_number.map(|n| n as u32),
                    r#match: Some(data.lines.text),
                    r#type: SearchResultType::Content,
                    is_context: true,
                    is_binary: None,
                    binary_suppressed: None,
                    modified: None,
                    accessed: None,
                    created: None,
                });
            }
            RipgrepMessage::Begin { .. } | RipgrepMessage::End { .. } => {
                // Ignore begin/end markers
            }
        }
    }

    Ok(results)
}
