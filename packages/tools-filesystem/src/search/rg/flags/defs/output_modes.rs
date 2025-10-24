//! OutputModes category flags.

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

/// -c/--count
#[derive(Debug)]
struct Count;

impl Flag for Count {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'c')
    }
    fn name_long(&self) -> &'static str {
        "count"
    }
    fn doc_category(&self) -> Category {
        Category::OutputModes
    }
    fn doc_short(&self) -> &'static str {
        r"Show count of matching lines for each file."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag suppresses normal output and shows the number of lines that match
the given patterns for each file searched. Each file containing a match has
its path and count printed on each line. Note that unless \flag{multiline} is
enabled and the pattern(s) given can match over multiple lines, this reports
the number of lines that match and not the total number of matches. When
multiline mode is enabled and the pattern(s) given can match over multiple
lines, \flag{count} is equivalent to \flag{count-matches}.
.sp
If only one file is given to ripgrep, then only the count is printed if there
is a match. The \flag{with-filename} flag can be used to force printing the
file path in this case. If you need a count to be printed regardless of whether
there is a match, then use \flag{include-zero}.
.sp
Note that it is possible for this flag to have results inconsistent with
the output of \flag{files-with-matches}. Notably, by default, ripgrep tries
to avoid searching files with binary data. With this flag, ripgrep needs to
search the entire content of files, which may include binary data. But with
\flag{files-with-matches}, ripgrep can stop as soon as a match is observed,
which may come well before any binary data. To avoid this inconsistency without
disabling binary detection, use the \flag{binary} flag.
.sp
This overrides the \flag{count-matches} flag. Note that when \flag{count}
is combined with \flag{only-matching}, then ripgrep behaves as if
\flag{count-matches} was given.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--count can only be enabled");
        args.mode.update(Mode::Search(SearchMode::Count));
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_count() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(Mode::Search(SearchMode::Standard), args.mode);

    let args = parse_low_raw(["--count"]).expect("Test parsing should succeed");
    assert_eq!(Mode::Search(SearchMode::Count), args.mode);

    let args = parse_low_raw(["-c"]).expect("Test parsing should succeed");
    assert_eq!(Mode::Search(SearchMode::Count), args.mode);

    let args = parse_low_raw(["--count-matches", "--count"]).expect("Test parsing should succeed");
    assert_eq!(Mode::Search(SearchMode::Count), args.mode);

    let args = parse_low_raw(["--count-matches", "-c"]).expect("Test parsing should succeed");
    assert_eq!(Mode::Search(SearchMode::Count), args.mode);
}

/// --count-matches
#[derive(Debug)]

/// --count-matches
#[derive(Debug)]
struct CountMatches;

impl Flag for CountMatches {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "count-matches"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        None
    }
    fn doc_category(&self) -> Category {
        Category::OutputModes
    }
    fn doc_short(&self) -> &'static str {
        r"Show count of every match for each file."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag suppresses normal output and shows the number of individual matches
of the given patterns for each file searched. Each file containing matches has
its path and match count printed on each line. Note that this reports the total
number of individual matches and not the number of lines that match.
.sp
If only one file is given to ripgrep, then only the count is printed if there
is a match. The \flag{with-filename} flag can be used to force printing the
file path in this case.
.sp
This overrides the \flag{count} flag. Note that when \flag{count} is combined
with \flag{only-matching}, then ripgrep behaves as if \flag{count-matches} was
given.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--count-matches can only be enabled");
        args.mode.update(Mode::Search(SearchMode::CountMatches));
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_count_matches() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(Mode::Search(SearchMode::Standard), args.mode);

    let args = parse_low_raw(["--count-matches"]).expect("Test parsing should succeed");
    assert_eq!(Mode::Search(SearchMode::CountMatches), args.mode);

    let args = parse_low_raw(["--count", "--count-matches"]).expect("Test parsing should succeed");
    assert_eq!(Mode::Search(SearchMode::CountMatches), args.mode);

    let args = parse_low_raw(["-c", "--count-matches"]).expect("Test parsing should succeed");
    assert_eq!(Mode::Search(SearchMode::CountMatches), args.mode);
}

/// --crlf
#[derive(Debug)]

/// -F/--fixed-strings
#[derive(Debug)]

/// --json
#[derive(Debug)]
struct JSON;

impl Flag for JSON {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "json"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-json")
    }
    fn doc_category(&self) -> Category {
        Category::OutputModes
    }
    fn doc_short(&self) -> &'static str {
        r"Show search results in a JSON Lines format."
    }
    fn doc_long(&self) -> &'static str {
        r"
Enable printing results in a JSON Lines format.
.sp
When this flag is provided, ripgrep will emit a sequence of messages, each
encoded as a JSON object, where there are five different message types:
.sp
.TP 12
\fBbegin\fP
A message that indicates a file is being searched and contains at least one
match.
.TP 12
\fBend\fP
A message the indicates a file is done being searched. This message also
include summary statistics about the search for a particular file.
.TP 12
\fBmatch\fP
A message that indicates a match was found. This includes the text and offsets
of the match.
.TP 12
\fBcontext\fP
A message that indicates a contextual line was found. This includes the text of
the line, along with any match information if the search was inverted.
.TP 12
\fBsummary\fP
The final message emitted by ripgrep that contains summary statistics about the
search across all files.
.PP
Since file paths or the contents of files are not guaranteed to be valid
UTF-8 and JSON itself must be representable by a Unicode encoding, ripgrep
will emit all data elements as objects with one of two keys: \fBtext\fP or
\fBbytes\fP. \fBtext\fP is a normal JSON string when the data is valid UTF-8
while \fBbytes\fP is the base64 encoded contents of the data.
.sp
The JSON Lines format is only supported for showing search results. It cannot
be used with other flags that emit other types of output, such as \flag{files},
\flag{files-with-matches}, \flag{files-without-match}, \flag{count} or
\flag{count-matches}. ripgrep will report an error if any of the aforementioned
flags are used in concert with \flag{json}.
.sp
Other flags that control aspects of the standard output such as
\flag{only-matching}, \flag{heading}, \flag{replace}, \flag{max-columns}, etc.,
have no effect when \flag{json} is set. However, enabling JSON output will
always implicitly and unconditionally enable \flag{stats}.
.sp
A more complete description of the JSON format used can be found here:
\fIhttps://docs.rs/grep-printer/*/grep_printer/struct.JSON.html\fP.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        if v.unwrap_switch() {
            args.mode.update(Mode::Search(SearchMode::Json));
        } else if matches!(args.mode, Mode::Search(SearchMode::Json)) {
            // --no-json only reverts to the default mode if the mode is
            // JSON, otherwise it's a no-op.
            args.mode.update(Mode::Search(SearchMode::Standard));
        }
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_json() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(Mode::Search(SearchMode::Standard), args.mode);

    let args = parse_low_raw(["--json"]).expect("Test parsing should succeed");
    assert_eq!(Mode::Search(SearchMode::Json), args.mode);

    let args = parse_low_raw(["--json", "--no-json"]).expect("Test parsing should succeed");
    assert_eq!(Mode::Search(SearchMode::Standard), args.mode);

    let args = parse_low_raw(["--json", "--files", "--no-json"]).expect("Test parsing should succeed");
    assert_eq!(Mode::Files, args.mode);
}
