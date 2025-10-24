use std::error::Error;
use std::fmt;

/// The scanning position for error reporting.
#[derive(Clone, Copy, PartialEq, Debug, Eq)]
pub struct Marker {
    pub index: usize,
    pub line: usize,
    pub col: usize,
}

impl Default for Marker {
    fn default() -> Self {
        Self {
            index: 0,
            line: 1,
            col: 0,
        }
    }
}

/// The parse error used by the scanner/parser if something goes wrong.
#[derive(Clone, Debug)]
pub struct ScanError {
    pub mark: Marker,
    pub info: String,
}

impl ScanError {
    #[must_use]
    pub fn new(mark: Marker, info: &str) -> Self {
        Self {
            mark,
            info: info.to_owned(),
        }
    }
}

impl fmt::Display for ScanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at line {} col {}",
            self.info,
            self.mark.line,
            self.mark.col + 1
        )
    }
}

impl Error for ScanError {}
