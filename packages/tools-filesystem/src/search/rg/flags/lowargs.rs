/*!
Provides the definition of low level arguments from CLI flags.

NOTE: This module contains comprehensive ripgrep compatibility types.
Many enum variants and struct fields are unused but maintained for
full ripgrep API compatibility.
*/

use std::{ffi::OsString, path::PathBuf};

use {
    bstr::BString,
    grep::printer::{HyperlinkFormat, UserColorSpec},
};

/// A collection of "low level" arguments.
///
/// The "low level" here is meant to constrain this type to be as close to the
/// actual CLI flags and arguments as possible. Namely, other than some
/// convenience types to help validate flag values and deal with overrides
/// between flags, these low level arguments do not contain any higher level
/// abstractions.
///
/// Another self-imposed constraint is that populating low level arguments
/// should not require anything other than validating what the user has
/// provided. For example, low level arguments should not contain a
/// `HyperlinkConfig`, since in order to get a full configuration, one needs to
/// discover the hostname of the current system (which might require running a
/// binary or a syscall).
///
/// Low level arguments are populated by the parser directly via the `update`
/// method on the corresponding implementation of the `Flag` trait.
///
/// NOTE: Many fields are unused but kept for ripgrep compatibility.
#[allow(dead_code)]
#[derive(Debug, Default)]
pub(crate) struct LowArgs {
    // Essential arguments.
    pub(crate) mode: Mode,
    pub(crate) positional: Vec<OsString>,
    pub(crate) patterns: Vec<PatternSource>,
    // Everything else, sorted lexicographically.
    pub(crate) binary: BinaryMode,
    pub(crate) boundary: Option<BoundaryMode>,
    pub(crate) buffer: BufferMode,
    pub(crate) byte_offset: bool,
    pub(crate) case: CaseMode,
    pub(crate) color: ColorChoice,
    pub(crate) colors: Vec<UserColorSpec>,
    pub(crate) column: Option<bool>,
    pub(crate) context: ContextMode,
    pub(crate) context_separator: ContextSeparator,
    pub(crate) crlf: bool,
    pub(crate) dfa_size_limit: Option<usize>,
    pub(crate) encoding: EncodingMode,
    pub(crate) engine: Engine,
    pub(crate) field_context_separator: FieldContextSeparator,
    pub(crate) field_match_separator: FieldMatchSeparator,
    pub(crate) fixed_strings: bool,
    pub(crate) follow: bool,
    pub(crate) glob_case_insensitive: bool,
    pub(crate) globs: Vec<String>,
    pub(crate) heading: Option<bool>,
    pub(crate) hidden: bool,
    pub(crate) hostname_bin: Option<PathBuf>,
    pub(crate) hyperlink_format: HyperlinkFormat,
    pub(crate) iglobs: Vec<String>,
    pub(crate) ignore_file: Vec<PathBuf>,
    pub(crate) ignore_file_case_insensitive: bool,
    pub(crate) include_zero: bool,
    pub(crate) invert_match: bool,
    pub(crate) line_number: Option<bool>,
    pub(crate) max_columns: Option<u64>,
    pub(crate) max_columns_preview: bool,
    pub(crate) max_count: Option<u64>,
    pub(crate) max_depth: Option<usize>,
    pub(crate) max_filesize: Option<u64>,
    pub(crate) mmap: MmapMode,
    pub(crate) multiline: bool,
    pub(crate) multiline_dotall: bool,
    pub(crate) no_config: bool,
    pub(crate) no_ignore_dot: bool,
    pub(crate) no_ignore_exclude: bool,
    pub(crate) no_ignore_files: bool,
    pub(crate) no_ignore_global: bool,
    pub(crate) no_ignore_messages: bool,
    pub(crate) no_ignore_parent: bool,
    pub(crate) no_ignore_vcs: bool,
    pub(crate) no_messages: bool,
    pub(crate) no_require_git: bool,
    pub(crate) no_unicode: bool,
    pub(crate) null: bool,
    pub(crate) null_data: bool,
    pub(crate) one_file_system: bool,
    pub(crate) only_matching: bool,
    pub(crate) path_separator: Option<u8>,
    pub(crate) pre: Option<PathBuf>,
    pub(crate) pre_glob: Vec<String>,
    pub(crate) quiet: bool,
    pub(crate) regex_size_limit: Option<usize>,
    pub(crate) replace: Option<BString>,
    pub(crate) search_zip: bool,
    pub(crate) stats: bool,
    pub(crate) stop_on_nonmatch: bool,
    pub(crate) threads: Option<usize>,
    pub(crate) trim: bool,
    pub(crate) type_changes: Vec<TypeChange>,
    pub(crate) unrestricted: usize,
    pub(crate) vimgrep: bool,
    pub(crate) with_filename: Option<bool>,
}

/// The overall mode that ripgrep should operate in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Mode {
    Search(SearchMode),
    Files,
}

impl Default for Mode {
    fn default() -> Mode {
        Mode::Search(SearchMode::Standard)
    }
}

/// The kind of search that ripgrep is going to perform.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SearchMode {
    Standard,
    Count,
    CountMatches,
    Json,
}

/// Indicates how ripgrep should treat binary data.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum BinaryMode {
    /// Automatically determine the binary mode to use. Essentially, when
    /// a file is searched explicitly, then it will be searched using the
    /// `SearchAndSuppress` strategy. Otherwise, it will be searched in a way
    /// that attempts to skip binary files as much as possible. That is, once
    /// a file is classified as binary, searching will immediately stop.
    Auto,
    /// Search files even when they have binary data, but if a match is found,
    /// suppress it and emit a warning.
    ///
    /// In this mode, `NUL` bytes are replaced with line terminators. This is
    /// a heuristic meant to reduce heap memory usage, since true binary data
    /// isn't line oriented. If one attempts to treat such data as line
    /// oriented, then one may wind up with impractically large lines. For
    /// example, many binary files contain very long runs of NUL bytes.
    SearchAndSuppress,
    /// Treat all files as if they were plain text. There's no skipping and no
    /// replacement of `NUL` bytes with line terminators.
    AsText,
}

#[allow(clippy::derivable_impls)]
impl Default for BinaryMode {
    fn default() -> BinaryMode {
        BinaryMode::Auto
    }
}

/// Indicates what kind of boundary mode to use (line or word).
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum BoundaryMode {
    /// Only allow matches when surrounded by line bounaries.
    Line,
    /// Only allow matches when surrounded by word bounaries.
    Word,
}

/// Indicates the buffer mode that ripgrep should use when printing output.
///
/// The default is `Auto`.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum BufferMode {
    /// Select the buffer mode automatically based on whether stdout is
    /// connected to a tty.
    Auto,
}

#[allow(clippy::derivable_impls)]
impl Default for BufferMode {
    fn default() -> BufferMode {
        BufferMode::Auto
    }
}

/// Indicates the case mode for how to interpret all patterns given to ripgrep.
///
/// The default is `Sensitive`.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum CaseMode {
    /// Patterns are matched case sensitively. i.e., `a` does not match `A`.
    Sensitive,
    /// Patterns are matched case insensitively. i.e., `a` does match `A`.
    Insensitive,
    /// Patterns are automatically matched case insensitively only when they
    /// consist of all lowercase literal characters. For example, the pattern
    /// `a` will match `A` but `A` will not match `a`.
    Smart,
}

#[allow(clippy::derivable_impls)]
impl Default for CaseMode {
    fn default() -> CaseMode {
        CaseMode::Sensitive
    }
}

/// Indicates whether ripgrep should include color/hyperlinks in its output.
///
/// The default is `Auto`.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum ColorChoice {
    /// Color and hyperlinks will never be used.
    Never,
    /// Color and hyperlinks will be used only when stdout is connected to a
    /// tty.
    Auto,
}

#[allow(clippy::derivable_impls)]
impl Default for ColorChoice {
    fn default() -> ColorChoice {
        ColorChoice::Auto
    }
}

/// Indicates the line context options ripgrep should use for output.
///
/// The default is no context at all.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum ContextMode {
    /// Only show a certain number of lines before and after each match.
    Limited(ContextModeLimited),
}

impl Default for ContextMode {
    fn default() -> ContextMode {
        ContextMode::Limited(ContextModeLimited::default())
    }
}

impl ContextMode {
    /// Set the "before" context.
    pub(crate) fn set_before(&mut self, lines: usize) {
        match *self {
            ContextMode::Limited(ContextModeLimited { ref mut before, .. }) => {
                *before = Some(lines)
            }
        }
    }

    /// Set the "after" context.
    pub(crate) fn set_after(&mut self, lines: usize) {
        match *self {
            ContextMode::Limited(ContextModeLimited { ref mut after, .. }) => *after = Some(lines),
        }
    }

    /// Set the "both" context.
    pub(crate) fn set_both(&mut self, lines: usize) {
        match *self {
            ContextMode::Limited(ContextModeLimited { ref mut both, .. }) => *both = Some(lines),
        }
    }
}

/// A context mode for a finite number of lines.
///
/// Namely, this indicates that a specific number of lines (possibly zero)
/// should be shown before and/or after each matching line.
///
/// Note that there is a subtle difference between `Some(0)` and `None`. In the
/// former case, it happens when `0` is given explicitly, where as `None` is
/// the default value and occurs when no value is specified.
///
/// `both` is only set by the -C/--context flag. The reason why we don't just
/// set before = after = --context is because the before and after context
/// settings always take precedent over the -C/--context setting, regardless of
/// order. Thus, we need to keep track of them separately.
#[derive(Debug, Default, Eq, PartialEq)]
pub(crate) struct ContextModeLimited {
    before: Option<usize>,
    after: Option<usize>,
    both: Option<usize>,
}

impl ContextModeLimited {
    /// Returns the specific number of contextual lines that should be shown
    /// around each match. This takes proper precedent into account, i.e.,
    /// that `before` and `after` both partially override `both` in all cases.
    ///
    /// By default, this returns `(0, 0)`.
    pub(crate) fn get(&self) -> (usize, usize) {
        let (mut before, mut after) = self.both.map_or((0, 0), |lines| (lines, lines));
        // --before and --after always override --context, regardless
        // of where they appear relative to each other.
        if let Some(lines) = self.before {
            before = lines;
        }
        if let Some(lines) = self.after {
            after = lines;
        }
        (before, after)
    }
}

/// Represents the separator to use between non-contiguous sections of
/// contextual lines.
///
/// The default is `--`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ContextSeparator(Option<BString>);

impl Default for ContextSeparator {
    fn default() -> ContextSeparator {
        ContextSeparator(Some(BString::from("--")))
    }
}

impl ContextSeparator {
    /// Return the raw bytes of this separator.
    ///
    /// If context separators were disabled, then this returns `None`.
    ///
    /// Note that this may return a `Some` variant with zero bytes.
    pub(crate) fn into_bytes(self) -> Option<Vec<u8>> {
        self.0.map(std::convert::Into::into)
    }
}

/// The encoding mode the searcher will use.
///
/// The default is `Auto`.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum EncodingMode {
    /// Use only BOM sniffing to auto-detect an encoding.
    Auto,
    /// Use an explicit encoding forcefully, but let BOM sniffing override it.
    Some(grep::searcher::Encoding),
    /// Use no explicit encoding and disable all BOM sniffing. This will
    /// always result in searching the raw bytes, regardless of their
    /// true encoding.
    Disabled,
}

#[allow(clippy::derivable_impls)]
impl Default for EncodingMode {
    fn default() -> EncodingMode {
        EncodingMode::Auto
    }
}

/// The regex engine to use.
///
/// The default is `Default`.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Engine {
    /// Uses the default regex engine: Rust's `regex` crate.
    ///
    /// (Well, technically it uses `regex-automata`, but `regex-automata` is
    /// the implementation of the `regex` crate.)
    Default,
    /// Dynamically select the right engine to use.
    ///
    /// This works by trying to use the default engine, and if the pattern does
    /// not compile, it switches over to the PCRE2 engine if it's available.
    Auto,
    /// Uses the PCRE2 regex engine if it's available.
    PCRE2,
}

#[allow(clippy::derivable_impls)]
impl Default for Engine {
    fn default() -> Engine {
        Engine::Default
    }
}

/// The field context separator to use to between metadata for each contextual
/// line.
///
/// The default is `-`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FieldContextSeparator(BString);

impl Default for FieldContextSeparator {
    fn default() -> FieldContextSeparator {
        FieldContextSeparator(BString::from("-"))
    }
}

impl FieldContextSeparator {}

/// The field match separator to use to between metadata for each matching
/// line.
///
/// The default is `:`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FieldMatchSeparator(BString);

impl Default for FieldMatchSeparator {
    fn default() -> FieldMatchSeparator {
        FieldMatchSeparator(BString::from(":"))
    }
}

impl FieldMatchSeparator {}

/// Indicates when to use memory maps.
///
/// The default is `Auto`.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum MmapMode {
    /// This instructs ripgrep to use heuristics for selecting when to and not
    /// to use memory maps for searching.
    Auto,
}

#[allow(clippy::derivable_impls)]
impl Default for MmapMode {
    fn default() -> MmapMode {
        MmapMode::Auto
    }
}

/// Represents a source of patterns that ripgrep should search for.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum PatternSource {
    /// Comes from the `-e/--regexp` flag.
    Regexp(String),
}

/// A single instance of a selection of one of ripgrep's file types.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum TypeChange {
    /// Select the given type for filtering.
    Select { name: String },
    /// Select the given type for filtering but negate it.
    Negate { name: String },
}
