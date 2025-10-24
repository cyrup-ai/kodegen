//! Logging category flags.

use std::{path::PathBuf, sync::LazyLock};
use {anyhow::Context as AnyhowContext, bstr::ByteVec};

use crate::search::rg::flags::{
    Category, Flag, FlagValue,
    lowargs::{
        BinaryMode, BoundaryMode, BufferMode, CaseMode, ColorChoice,
        ContextMode, EncodingMode, Engine,
        LowArgs, MmapMode, Mode, PatternSource, SearchMode, TypeChange,
    },
};

#[cfg(test)]
use crate::search::rg::flags::parse::parse_low_raw;

use super::{CompletionType, convert};

/// --debug
#[derive(Debug)]
struct Debug;

impl Flag for Debug {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "debug"
    }
    fn doc_category(&self) -> Category {
        Category::Logging
    }
    fn doc_short(&self) -> &'static str {
        r"Show debug messages."
    }
    fn doc_long(&self) -> &'static str {
        r"
Show debug messages. Please use this when filing a bug report.
.sp
The \flag{debug} flag is generally useful for figuring out why ripgrep skipped
searching a particular file. The debug messages should mention all files
skipped and why they were skipped.
.sp
To get even more debug output, use the \flag{trace} flag, which implies
\flag{debug} along with additional trace data.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--debug can only be enabled");
        args.logging = Some(LoggingMode::Debug);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_debug() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.logging);

    let args = parse_low_raw(["--debug"]).expect("Test parsing should succeed");
    assert_eq!(Some(LoggingMode::Debug), args.logging);

    let args = parse_low_raw(["--trace", "--debug"]).expect("Test parsing should succeed");
    assert_eq!(Some(LoggingMode::Debug), args.logging);
}

/// --dfa-size-limit
#[derive(Debug)]

/// --no-ignore-messages
#[derive(Debug)]
struct NoIgnoreMessages;

impl Flag for NoIgnoreMessages {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "no-ignore-messages"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("ignore-messages")
    }
    fn doc_category(&self) -> Category {
        Category::Logging
    }
    fn doc_short(&self) -> &'static str {
        r"Suppress gitignore parse error messages."
    }
    fn doc_long(&self) -> &'static str {
        r"
When this flag is enabled, all error messages related to parsing ignore files
are suppressed. By default, error messages are printed to stderr. In cases
where these errors are expected, this flag can be used to avoid seeing the
noise produced by the messages.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.no_ignore_messages = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_no_ignore_messages() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.no_ignore_messages);

    let args = parse_low_raw(["--no-ignore-messages"]).expect("Test parsing should succeed");
    assert_eq!(true, args.no_ignore_messages);

    let args =
        parse_low_raw(["--no-ignore-messages", "--ignore-messages"]).expect("Test parsing should succeed");
    assert_eq!(false, args.no_ignore_messages);
}

/// --no-ignore-parent
#[derive(Debug)]

/// --no-messages
#[derive(Debug)]
struct NoMessages;

impl Flag for NoMessages {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "no-messages"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("messages")
    }
    fn doc_category(&self) -> Category {
        Category::Logging
    }
    fn doc_short(&self) -> &'static str {
        r"Suppress some error messages."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag suppresses some error messages. Specifically, messages related to
the failed opening and reading of files. Error messages related to the syntax
of the pattern are still shown.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.no_messages = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_no_messages() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.no_messages);

    let args = parse_low_raw(["--no-messages"]).expect("Test parsing should succeed");
    assert_eq!(true, args.no_messages);

    let args = parse_low_raw(["--no-messages", "--messages"]).expect("Test parsing should succeed");
    assert_eq!(false, args.no_messages);
}

/// --no-pcre2-unicode
#[derive(Debug)]

/// --stats
#[derive(Debug)]
struct Stats;

impl Flag for Stats {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "stats"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-stats")
    }
    fn doc_category(&self) -> Category {
        Category::Logging
    }
    fn doc_short(&self) -> &'static str {
        r"Print statistics about the search."
    }
    fn doc_long(&self) -> &'static str {
        r"
When enabled, ripgrep will print aggregate statistics about the search. When
this flag is present, ripgrep will print at least the following stats to
stdout at the end of the search: number of matched lines, number of files with
matches, number of files searched, and the time taken for the entire search to
complete.
.sp
This set of aggregate statistics may expand over time.
.sp
This flag is always and implicitly enabled when \flag{json} is used.
.sp
Note that this flag has no effect if \flag{files}, \flag{files-with-matches} or
\flag{files-without-match} is passed.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.stats = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_stats() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.stats);

    let args = parse_low_raw(["--stats"]).expect("Test parsing should succeed");
    assert_eq!(true, args.stats);

    let args = parse_low_raw(["--stats", "--no-stats"]).expect("Test parsing should succeed");
    assert_eq!(false, args.stats);
}

/// --stop-on-nonmatch
#[derive(Debug)]

/// --trace
#[derive(Debug)]
struct Trace;

impl Flag for Trace {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "trace"
    }
    fn doc_category(&self) -> Category {
        Category::Logging
    }
    fn doc_short(&self) -> &'static str {
        r"Show trace messages."
    }
    fn doc_long(&self) -> &'static str {
        r"
Show trace messages. This shows even more detail than the \flag{debug}
flag. Generally, one should only use this if \flag{debug} doesn't emit the
information you're looking for.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--trace can only be enabled");
        args.logging = Some(LoggingMode::Trace);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_trace() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.logging);

    let args = parse_low_raw(["--trace"]).expect("Test parsing should succeed");
    assert_eq!(Some(LoggingMode::Trace), args.logging);

    let args = parse_low_raw(["--debug", "--trace"]).expect("Test parsing should succeed");
    assert_eq!(Some(LoggingMode::Trace), args.logging);
}
