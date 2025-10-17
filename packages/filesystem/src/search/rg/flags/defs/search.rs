//! Search category flags.

use std::{path::PathBuf, sync::LazyLock};
use {anyhow::Context as AnyhowContext, bstr::ByteVec};

use crate::search::rg::flags::{
    Category, Flag, FlagValue,
    lowargs::{
        BinaryMode, BoundaryMode, BufferMode, CaseMode, ColorChoice,
        ContextMode, EncodingMode, EngineChoice,
        LowArgs, MmapMode, Mode, PatternSource, SearchMode, SortMode,
        SortModeKind, TypeChange,
    },
};

#[cfg(test)]
use crate::search::rg::flags::parse::parse_low_raw;

use super::{CompletionType, convert};

/// --auto-hybrid-regex
#[derive(Debug)]
struct AutoHybridRegex;

impl Flag for AutoHybridRegex {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "auto-hybrid-regex"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-auto-hybrid-regex")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        "(DEPRECATED) Use PCRE2 if appropriate."
    }
    fn doc_long(&self) -> &'static str {
        r"
DEPRECATED. Use \flag{engine} instead.
.sp
When this flag is used, ripgrep will dynamically choose between supported regex
engines depending on the features used in a pattern. When ripgrep chooses a
regex engine, it applies that choice for every regex provided to ripgrep (e.g.,
via multiple \flag{regexp} or \flag{file} flags).
.sp
As an example of how this flag might behave, ripgrep will attempt to use
its default finite automata based regex engine whenever the pattern can be
successfully compiled with that regex engine. If PCRE2 is enabled and if the
pattern given could not be compiled with the default regex engine, then PCRE2
will be automatically used for searching. If PCRE2 isn't available, then this
flag has no effect because there is only one regex engine to choose from.
.sp
In the future, ripgrep may adjust its heuristics for how it decides which
regex engine to use. In general, the heuristics will be limited to a static
analysis of the patterns, and not to any specific runtime behavior observed
while searching files.
.sp
The primary downside of using this flag is that it may not always be obvious
which regex engine ripgrep uses, and thus, the match semantics or performance
profile of ripgrep may subtly and unexpectedly change. However, in many cases,
all regex engines will agree on what constitutes a match and it can be nice
to transparently support more advanced regex features like look-around and
backreferences without explicitly needing to enable them.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let mode = if v.unwrap_switch() {
            EngineChoice::Auto
        } else {
            EngineChoice::Default
        };
        args.engine = mode;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_auto_hybrid_regex() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(EngineChoice::Default, args.engine);

    let args = parse_low_raw(["--auto-hybrid-regex"]).unwrap();
    assert_eq!(EngineChoice::Auto, args.engine);

    let args =
        parse_low_raw(["--auto-hybrid-regex", "--no-auto-hybrid-regex"])
            .unwrap();
    assert_eq!(EngineChoice::Default, args.engine);

    let args =
        parse_low_raw(["--no-auto-hybrid-regex", "--auto-hybrid-regex"])
            .unwrap();
    assert_eq!(EngineChoice::Auto, args.engine);

    let args = parse_low_raw(["--auto-hybrid-regex", "-P"]).unwrap();
    assert_eq!(EngineChoice::PCRE2, args.engine);

    let args = parse_low_raw(["-P", "--auto-hybrid-regex"]).unwrap();
    assert_eq!(EngineChoice::Auto, args.engine);

    let args =
        parse_low_raw(["--engine=auto", "--auto-hybrid-regex"]).unwrap();
    assert_eq!(EngineChoice::Auto, args.engine);

    let args =
        parse_low_raw(["--engine=default", "--auto-hybrid-regex"]).unwrap();
    assert_eq!(EngineChoice::Auto, args.engine);

    let args =
        parse_low_raw(["--auto-hybrid-regex", "--engine=default"]).unwrap();
    assert_eq!(EngineChoice::Default, args.engine);
}

/// -B/--before-context
#[derive(Debug)]

/// -s/--case-sensitive
#[derive(Debug)]
struct CaseSensitive;

impl Flag for CaseSensitive {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b's')
    }
    fn name_long(&self) -> &'static str {
        "case-sensitive"
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Search case sensitively (default)."
    }
    fn doc_long(&self) -> &'static str {
        r"
Execute the search case sensitively. This is the default mode.
.sp
This is a global option that applies to all patterns given to ripgrep.
Individual patterns can still be matched case insensitively by using inline
regex flags. For example, \fB(?i)abc\fP will match \fBabc\fP case insensitively
even when this flag is used.
.sp
This flag overrides the \flag{ignore-case} and \flag{smart-case} flags.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "flag has no negation");
        args.case = CaseMode::Sensitive;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_case_sensitive() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(CaseMode::Sensitive, args.case);

    let args = parse_low_raw(["--case-sensitive"]).unwrap();
    assert_eq!(CaseMode::Sensitive, args.case);

    let args = parse_low_raw(["-s"]).unwrap();
    assert_eq!(CaseMode::Sensitive, args.case);
}

/// --color
#[derive(Debug)]

/// --crlf
#[derive(Debug)]
struct Crlf;

impl Flag for Crlf {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "crlf"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-crlf")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Use CRLF line terminators (nice for Windows)."
    }
    fn doc_long(&self) -> &'static str {
        r"
When enabled, ripgrep will treat CRLF (\fB\\r\\n\fP) as a line terminator
instead of just \fB\\n\fP.
.sp
Principally, this permits the line anchor assertions \fB^\fP and \fB$\fP in
regex patterns to treat CRLF, CR or LF as line terminators instead of just LF.
Note that they will never match between a CR and a LF. CRLF is treated as one
single line terminator.
.sp
When using the default regex engine, CRLF support can also be enabled inside
the pattern with the \fBR\fP flag. For example, \fB(?R:$)\fP will match just
before either CR or LF, but never between CR and LF.
.sp
This flag overrides \flag{null-data}.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.crlf = v.unwrap_switch();
        if args.crlf {
            args.null_data = false;
        }
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_crlf() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.crlf);

    let args = parse_low_raw(["--crlf"]).unwrap();
    assert_eq!(true, args.crlf);
    assert_eq!(false, args.null_data);

    let args = parse_low_raw(["--crlf", "--null-data"]).unwrap();
    assert_eq!(false, args.crlf);
    assert_eq!(true, args.null_data);

    let args = parse_low_raw(["--null-data", "--crlf"]).unwrap();
    assert_eq!(true, args.crlf);
    assert_eq!(false, args.null_data);

    let args = parse_low_raw(["--null-data", "--no-crlf"]).unwrap();
    assert_eq!(false, args.crlf);
    assert_eq!(true, args.null_data);

    let args = parse_low_raw(["--null-data", "--crlf", "--no-crlf"]).unwrap();
    assert_eq!(false, args.crlf);
    assert_eq!(false, args.null_data);
}

/// --debug
#[derive(Debug)]

/// --dfa-size-limit
#[derive(Debug)]
struct DfaSizeLimit;

impl Flag for DfaSizeLimit {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "dfa-size-limit"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("NUM+SUFFIX?")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"The upper size limit of the regex DFA."
    }
    fn doc_long(&self) -> &'static str {
        r"
The upper size limit of the regex DFA. The default limit is something generous
for any single pattern or for many smallish patterns. This should only be
changed on very large regex inputs where the (slower) fallback regex engine may
otherwise be used if the limit is reached.
.sp
The input format accepts suffixes of \fBK\fP, \fBM\fP or \fBG\fP which
correspond to kilobytes, megabytes and gigabytes, respectively. If no suffix is
provided the input is treated as bytes.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let v = v.unwrap_value();
        args.dfa_size_limit = Some(convert::human_readable_usize(&v)?);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_dfa_size_limit() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(None, args.dfa_size_limit);

    #[cfg(target_pointer_width = "64")]
    {
        let args = parse_low_raw(["--dfa-size-limit", "9G"]).unwrap();
        assert_eq!(Some(9 * (1 << 30)), args.dfa_size_limit);

        let args = parse_low_raw(["--dfa-size-limit=9G"]).unwrap();
        assert_eq!(Some(9 * (1 << 30)), args.dfa_size_limit);

        let args =
            parse_low_raw(["--dfa-size-limit=9G", "--dfa-size-limit=0"])
                .unwrap();
        assert_eq!(Some(0), args.dfa_size_limit);
    }

    let args = parse_low_raw(["--dfa-size-limit=0K"]).unwrap();
    assert_eq!(Some(0), args.dfa_size_limit);

    let args = parse_low_raw(["--dfa-size-limit=0M"]).unwrap();
    assert_eq!(Some(0), args.dfa_size_limit);

    let args = parse_low_raw(["--dfa-size-limit=0G"]).unwrap();
    assert_eq!(Some(0), args.dfa_size_limit);

    let result = parse_low_raw(["--dfa-size-limit", "9999999999999999999999"]);
    assert!(result.is_err(), "{result:?}");

    let result = parse_low_raw(["--dfa-size-limit", "9999999999999999G"]);
    assert!(result.is_err(), "{result:?}");
}

/// -E/--encoding
#[derive(Debug)]

/// -E/--encoding
#[derive(Debug)]
struct Encoding;

impl Flag for Encoding {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'E')
    }
    fn name_long(&self) -> &'static str {
        "encoding"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-encoding")
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("ENCODING")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Specify the text encoding of files to search."
    }
    fn doc_long(&self) -> &'static str {
        r"
Specify the text encoding that ripgrep will use on all files searched. The
default value is \fBauto\fP, which will cause ripgrep to do a best effort
automatic detection of encoding on a per-file basis. Automatic detection in
this case only applies to files that begin with a UTF-8 or UTF-16 byte-order
mark (BOM). No other automatic detection is performed. One can also specify
\fBnone\fP which will then completely disable BOM sniffing and always result
in searching the raw bytes, including a BOM if it's present, regardless of its
encoding.
.sp
Other supported values can be found in the list of labels here:
\fIhttps://encoding.spec.whatwg.org/#concept-encoding-get\fP.
.sp
For more details on encoding and how ripgrep deals with it, see \fBGUIDE.md\fP.
.sp
The encoding detection that ripgrep uses can be reverted to its automatic mode
via the \flag-negate{encoding} flag.
"
    }
    fn completion_type(&self) -> CompletionType {
        CompletionType::Encoding
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let value = match v {
            FlagValue::Value(v) => v,
            FlagValue::Switch(true) => {
                unreachable!("--encoding must accept a value")
            }
            FlagValue::Switch(false) => {
                args.encoding = EncodingMode::Auto;
                return Ok(());
            }
        };
        let label = convert::str(&value)?;
        args.encoding = match label {
            "auto" => EncodingMode::Auto,
            "none" => EncodingMode::Disabled,
            _ => EncodingMode::Some(grep::searcher::Encoding::new(label)?),
        };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_encoding() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(EncodingMode::Auto, args.encoding);

    let args = parse_low_raw(["--encoding", "auto"]).unwrap();
    assert_eq!(EncodingMode::Auto, args.encoding);

    let args = parse_low_raw(["--encoding", "none"]).unwrap();
    assert_eq!(EncodingMode::Disabled, args.encoding);

    let args = parse_low_raw(["--encoding=none"]).unwrap();
    assert_eq!(EncodingMode::Disabled, args.encoding);

    let args = parse_low_raw(["-E", "none"]).unwrap();
    assert_eq!(EncodingMode::Disabled, args.encoding);

    let args = parse_low_raw(["-Enone"]).unwrap();
    assert_eq!(EncodingMode::Disabled, args.encoding);

    let args = parse_low_raw(["-E", "none", "--no-encoding"]).unwrap();
    assert_eq!(EncodingMode::Auto, args.encoding);

    let args = parse_low_raw(["--no-encoding", "-E", "none"]).unwrap();
    assert_eq!(EncodingMode::Disabled, args.encoding);

    let args = parse_low_raw(["-E", "utf-16"]).unwrap();
    let enc = grep::searcher::Encoding::new("utf-16").unwrap();
    assert_eq!(EncodingMode::Some(enc), args.encoding);

    let args = parse_low_raw(["-E", "utf-16", "--no-encoding"]).unwrap();
    assert_eq!(EncodingMode::Auto, args.encoding);

    let result = parse_low_raw(["-E", "foo"]);
    assert!(result.is_err(), "{result:?}");
}

/// --engine
#[derive(Debug)]

/// --engine
#[derive(Debug)]
struct Engine;

impl Flag for Engine {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "engine"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("ENGINE")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Specify which regex engine to use."
    }
    fn doc_long(&self) -> &'static str {
        r"
Specify which regular expression engine to use. When you choose a regex engine,
it applies that choice for every regex provided to ripgrep (e.g., via multiple
\flag{regexp} or \flag{file} flags).
.sp
Accepted values are \fBdefault\fP, \fBpcre2\fP, or \fBauto\fP.
.sp
The default value is \fBdefault\fP, which is usually the fastest and should be
good for most use cases. The \fBpcre2\fP engine is generally useful when you
want to use features such as look-around or backreferences. \fBauto\fP will
dynamically choose between supported regex engines depending on the features
used in a pattern on a best effort basis.
.sp
Note that the \fBpcre2\fP engine is an optional ripgrep feature. If PCRE2
wasn't included in your build of ripgrep, then using this flag will result in
ripgrep printing an error message and exiting.
.sp
This overrides previous uses of the \flag{pcre2} and \flag{auto-hybrid-regex}
flags.
"
    }
    fn doc_choices(&self) -> &'static [&'static str] {
        &["default", "pcre2", "auto"]
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let v = v.unwrap_value();
        let string = convert::str(&v)?;
        args.engine = match string {
            "default" => EngineChoice::Default,
            "pcre2" => EngineChoice::PCRE2,
            "auto" => EngineChoice::Auto,
            _ => anyhow::bail!("unrecognized regex engine '{string}'"),
        };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_engine() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(EngineChoice::Default, args.engine);

    let args = parse_low_raw(["--engine", "pcre2"]).unwrap();
    assert_eq!(EngineChoice::PCRE2, args.engine);

    let args = parse_low_raw(["--engine=pcre2"]).unwrap();
    assert_eq!(EngineChoice::PCRE2, args.engine);

    let args =
        parse_low_raw(["--auto-hybrid-regex", "--engine=pcre2"]).unwrap();
    assert_eq!(EngineChoice::PCRE2, args.engine);

    let args =
        parse_low_raw(["--engine=pcre2", "--auto-hybrid-regex"]).unwrap();
    assert_eq!(EngineChoice::Auto, args.engine);

    let args =
        parse_low_raw(["--auto-hybrid-regex", "--engine=auto"]).unwrap();
    assert_eq!(EngineChoice::Auto, args.engine);

    let args =
        parse_low_raw(["--auto-hybrid-regex", "--engine=default"]).unwrap();
    assert_eq!(EngineChoice::Default, args.engine);

    let args =
        parse_low_raw(["--engine=pcre2", "--no-auto-hybrid-regex"]).unwrap();
    assert_eq!(EngineChoice::Default, args.engine);
}

/// --field-context-separator
#[derive(Debug)]

/// -F/--fixed-strings
#[derive(Debug)]
struct FixedStrings;

impl Flag for FixedStrings {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'F')
    }
    fn name_long(&self) -> &'static str {
        "fixed-strings"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-fixed-strings")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Treat all patterns as literals."
    }
    fn doc_long(&self) -> &'static str {
        r"
Treat all patterns as literals instead of as regular expressions. When this
flag is used, special regular expression meta characters such as \fB.(){}*+\fP
should not need be escaped.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.fixed_strings = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_fixed_strings() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.fixed_strings);

    let args = parse_low_raw(["--fixed-strings"]).unwrap();
    assert_eq!(true, args.fixed_strings);

    let args = parse_low_raw(["-F"]).unwrap();
    assert_eq!(true, args.fixed_strings);

    let args = parse_low_raw(["-F", "--no-fixed-strings"]).unwrap();
    assert_eq!(false, args.fixed_strings);

    let args = parse_low_raw(["--no-fixed-strings", "-F"]).unwrap();
    assert_eq!(true, args.fixed_strings);
}

/// -L/--follow
#[derive(Debug)]

/// -i/--ignore-case
#[derive(Debug)]
struct IgnoreCase;

impl Flag for IgnoreCase {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'i')
    }
    fn name_long(&self) -> &'static str {
        "ignore-case"
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Case insensitive search."
    }
    fn doc_long(&self) -> &'static str {
        r#"
When this flag is provided, all patterns will be searched case insensitively.
The case insensitivity rules used by ripgrep's default regex engine conform to
Unicode's "simple" case folding rules.
.sp
This is a global option that applies to all patterns given to ripgrep.
Individual patterns can still be matched case sensitively by using
inline regex flags. For example, \fB(?\-i)abc\fP will match \fBabc\fP
case sensitively even when this flag is used.
.sp
This flag overrides \flag{case-sensitive} and \flag{smart-case}.
"#
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "flag has no negation");
        args.case = CaseMode::Insensitive;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_ignore_case() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(CaseMode::Sensitive, args.case);

    let args = parse_low_raw(["--ignore-case"]).unwrap();
    assert_eq!(CaseMode::Insensitive, args.case);

    let args = parse_low_raw(["-i"]).unwrap();
    assert_eq!(CaseMode::Insensitive, args.case);

    let args = parse_low_raw(["-i", "-s"]).unwrap();
    assert_eq!(CaseMode::Sensitive, args.case);

    let args = parse_low_raw(["-s", "-i"]).unwrap();
    assert_eq!(CaseMode::Insensitive, args.case);
}

/// --ignore-file
#[derive(Debug)]

/// -v/--invert-match
#[derive(Debug)]
struct InvertMatch;

impl Flag for InvertMatch {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'v')
    }
    fn name_long(&self) -> &'static str {
        "invert-match"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-invert-match")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Invert matching."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag inverts matching. That is, instead of printing lines that match,
ripgrep will print lines that don't match.
.sp
Note that this only inverts line-by-line matching. For example, combining this
flag with \flag{files-with-matches} will emit files that contain any lines
that do not match the patterns given. That's not the same as, for example,
\flag{files-without-match}, which will emit files that do not contain any
matching lines.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.invert_match = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_invert_match() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.invert_match);

    let args = parse_low_raw(["--invert-match"]).unwrap();
    assert_eq!(true, args.invert_match);

    let args = parse_low_raw(["-v"]).unwrap();
    assert_eq!(true, args.invert_match);

    let args = parse_low_raw(["-v", "--no-invert-match"]).unwrap();
    assert_eq!(false, args.invert_match);
}

/// --json
#[derive(Debug)]

/// -x/--line-regexp
#[derive(Debug)]
struct LineRegexp;

impl Flag for LineRegexp {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'x')
    }
    fn name_long(&self) -> &'static str {
        "line-regexp"
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Show matches surrounded by line boundaries."
    }
    fn doc_long(&self) -> &'static str {
        r"
When enabled, ripgrep will only show matches surrounded by line boundaries.
This is equivalent to surrounding every pattern with \fB^\fP and \fB$\fP. In
other words, this only prints lines where the entire line participates in a
match.
.sp
This overrides the \flag{word-regexp} flag.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--line-regexp has no negation");
        args.boundary = Some(BoundaryMode::Line);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_line_regexp() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(None, args.boundary);

    let args = parse_low_raw(["--line-regexp"]).unwrap();
    assert_eq!(Some(BoundaryMode::Line), args.boundary);

    let args = parse_low_raw(["-x"]).unwrap();
    assert_eq!(Some(BoundaryMode::Line), args.boundary);
}

/// -M/--max-columns
#[derive(Debug)]

/// -m/--max-count
#[derive(Debug)]
struct MaxCount;

impl Flag for MaxCount {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'm')
    }
    fn name_long(&self) -> &'static str {
        "max-count"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("NUM")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Limit the number of matching lines."
    }
    fn doc_long(&self) -> &'static str {
        r"
Limit the number of matching lines per file searched to \fINUM\fP.
.sp
When \flag{multiline} is used, a single match that spans multiple lines is only
counted once for the purposes of this limit. Multiple matches in a single line
are counted only once, as they would be in non-multiline mode.
.sp
When combined with \flag{after-context} or \flag{context}, it's possible for
more matches than the maximum to be printed if contextual lines contain a
match.
.sp
Note that \fB0\fP is a legal value but not likely to be useful. When used,
ripgrep won't search anything.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.max_count = Some(convert::u64(&v.unwrap_value())?);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_max_count() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(None, args.max_count);

    let args = parse_low_raw(["--max-count", "5"]).unwrap();
    assert_eq!(Some(5), args.max_count);

    let args = parse_low_raw(["-m", "5"]).unwrap();
    assert_eq!(Some(5), args.max_count);

    let args = parse_low_raw(["-m", "5", "--max-count=10"]).unwrap();
    assert_eq!(Some(10), args.max_count);
    let args = parse_low_raw(["-m0"]).unwrap();
    assert_eq!(Some(0), args.max_count);
}

/// --max-depth
#[derive(Debug)]

/// --mmap
#[derive(Debug)]
struct Mmap;

impl Flag for Mmap {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "mmap"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-mmap")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Search with memory maps when possible."
    }
    fn doc_long(&self) -> &'static str {
        r"
When enabled, ripgrep will search using memory maps when possible. This is
enabled by default when ripgrep thinks it will be faster.
.sp
Memory map searching cannot be used in all circumstances. For example, when
searching virtual files or streams likes \fBstdin\fP. In such cases, memory
maps will not be used even when this flag is enabled.
.sp
Note that ripgrep may abort unexpectedly when memory maps are used if it
searches a file that is simultaneously truncated. Users can opt out of this
possibility by disabling memory maps.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.mmap = if v.unwrap_switch() {
            MmapMode::AlwaysTryMmap
        } else {
            MmapMode::Never
        };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_mmap() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(MmapMode::Auto, args.mmap);

    let args = parse_low_raw(["--mmap"]).unwrap();
    assert_eq!(MmapMode::AlwaysTryMmap, args.mmap);

    let args = parse_low_raw(["--no-mmap"]).unwrap();
    assert_eq!(MmapMode::Never, args.mmap);

    let args = parse_low_raw(["--mmap", "--no-mmap"]).unwrap();
    assert_eq!(MmapMode::Never, args.mmap);

    let args = parse_low_raw(["--no-mmap", "--mmap"]).unwrap();
    assert_eq!(MmapMode::AlwaysTryMmap, args.mmap);
}

/// -U/--multiline
#[derive(Debug)]

/// -U/--multiline
#[derive(Debug)]
struct Multiline;

impl Flag for Multiline {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'U')
    }
    fn name_long(&self) -> &'static str {
        "multiline"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-multiline")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Enable searching across multiple lines."
    }
    fn doc_long(&self) -> &'static str {
        r#"
This flag enable searching across multiple lines.
.sp
When multiline mode is enabled, ripgrep will lift the restriction that a
match cannot include a line terminator. For example, when multiline mode
is not enabled (the default), then the regex \fB\\p{any}\fP will match any
Unicode codepoint other than \fB\\n\fP. Similarly, the regex \fB\\n\fP is
explicitly forbidden, and if you try to use it, ripgrep will return an error.
However, when multiline mode is enabled, \fB\\p{any}\fP will match any Unicode
codepoint, including \fB\\n\fP, and regexes like \fB\\n\fP are permitted.
.sp
An important caveat is that multiline mode does not change the match semantics
of \fB.\fP. Namely, in most regex matchers, a \fB.\fP will by default match any
character other than \fB\\n\fP, and this is true in ripgrep as well. In order
to make \fB.\fP match \fB\\n\fP, you must enable the "dot all" flag inside the
regex. For example, both \fB(?s).\fP and \fB(?s:.)\fP have the same semantics,
where \fB.\fP will match any character, including \fB\\n\fP. Alternatively, the
\flag{multiline-dotall} flag may be passed to make the "dot all" behavior the
default. This flag only applies when multiline search is enabled.
.sp
There is no limit on the number of the lines that a single match can span.
.sp
\fBWARNING\fP: Because of how the underlying regex engine works, multiline
searches may be slower than normal line-oriented searches, and they may also
use more memory. In particular, when multiline mode is enabled, ripgrep
requires that each file it searches is laid out contiguously in memory (either
by reading it onto the heap or by memory-mapping it). Things that cannot be
memory-mapped (such as \fBstdin\fP) will be consumed until EOF before searching
can begin. In general, ripgrep will only do these things when necessary.
Specifically, if the \flag{multiline} flag is provided but the regex does
not contain patterns that would match \fB\\n\fP characters, then ripgrep
will automatically avoid reading each file into memory before searching it.
Nevertheless, if you only care about matches spanning at most one line, then it
is always better to disable multiline mode.
.sp
This overrides the \flag{stop-on-nonmatch} flag.
"#
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.multiline = v.unwrap_switch();
        if args.multiline {
            args.stop_on_nonmatch = false;
        }
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_multiline() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.multiline);

    let args = parse_low_raw(["--multiline"]).unwrap();
    assert_eq!(true, args.multiline);

    let args = parse_low_raw(["-U"]).unwrap();
    assert_eq!(true, args.multiline);

    let args = parse_low_raw(["-U", "--no-multiline"]).unwrap();
    assert_eq!(false, args.multiline);
}

/// --multiline-dotall
#[derive(Debug)]

/// --multiline-dotall
#[derive(Debug)]
struct MultilineDotall;

impl Flag for MultilineDotall {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "multiline-dotall"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-multiline-dotall")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Make '.' match line terminators."
    }
    fn doc_long(&self) -> &'static str {
        r#"
This flag enables "dot all" mode in all regex patterns. This causes \fB.\fP to
match line terminators when multiline searching is enabled. This flag has no
effect if multiline searching isn't enabled with the \flag{multiline} flag.
.sp
Normally, a \fB.\fP will match any character except line terminators. While
this behavior typically isn't relevant for line-oriented matching (since
matches can span at most one line), this can be useful when searching with the
\flag{multiline} flag. By default, multiline mode runs without "dot all" mode
enabled.
.sp
This flag is generally intended to be used in an alias or your ripgrep config
file if you prefer "dot all" semantics by default. Note that regardless of
whether this flag is used, "dot all" semantics can still be controlled via
inline flags in the regex pattern itself, e.g., \fB(?s:.)\fP always enables
"dot all" whereas \fB(?-s:.)\fP always disables "dot all". Moreover, you
can use character classes like \fB\\p{any}\fP to match any Unicode codepoint
regardless of whether "dot all" mode is enabled or not.
"#
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.multiline_dotall = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_multiline_dotall() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.multiline_dotall);

    let args = parse_low_raw(["--multiline-dotall"]).unwrap();
    assert_eq!(true, args.multiline_dotall);

    let args = parse_low_raw(["--multiline-dotall", "--no-multiline-dotall"])
        .unwrap();
    assert_eq!(false, args.multiline_dotall);
}

/// --no-config
#[derive(Debug)]

/// --no-pcre2-unicode
#[derive(Debug)]
struct NoPcre2Unicode;

impl Flag for NoPcre2Unicode {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "no-pcre2-unicode"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("pcre2-unicode")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"(DEPRECATED) Disable Unicode mode for PCRE2."
    }
    fn doc_long(&self) -> &'static str {
        r"
DEPRECATED. Use \flag{no-unicode} instead.
.sp
Note that Unicode mode is enabled by default.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.no_unicode = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_no_pcre2_unicode() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.no_unicode);

    let args = parse_low_raw(["--no-pcre2-unicode"]).unwrap();
    assert_eq!(true, args.no_unicode);

    let args =
        parse_low_raw(["--no-pcre2-unicode", "--pcre2-unicode"]).unwrap();
    assert_eq!(false, args.no_unicode);
}

/// --no-require-git
#[derive(Debug)]

/// --no-unicode
#[derive(Debug)]
struct NoUnicode;

impl Flag for NoUnicode {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "no-unicode"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("unicode")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Disable Unicode mode."
    }
    fn doc_long(&self) -> &'static str {
        r#"
This flag disables Unicode mode for all patterns given to ripgrep.
.sp
By default, ripgrep will enable "Unicode mode" in all of its regexes. This has
a number of consequences:
.sp
.IP \(bu 3n
\fB.\fP will only match valid UTF-8 encoded Unicode scalar values.
.sp
.IP \(bu 3n
Classes like \fB\\w\fP, \fB\\s\fP, \fB\\d\fP are all Unicode aware and much
bigger than their ASCII only versions.
.sp
.IP \(bu 3n
Case insensitive matching will use Unicode case folding.
.sp
.IP \(bu 3n
A large array of classes like \fB\\p{Emoji}\fP are available. (Although the
specific set of classes available varies based on the regex engine. In general,
the default regex engine has more classes available to it.)
.sp
.IP \(bu 3n
Word boundaries (\fB\\b\fP and \fB\\B\fP) use the Unicode definition of a word
character.
.PP
In some cases it can be desirable to turn these things off. This flag will do
exactly that. For example, Unicode mode can sometimes have a negative impact
on performance, especially when things like \fB\\w\fP are used frequently
(including via bounded repetitions like \fB\\w{100}\fP) when only their ASCII
interpretation is needed.
"#
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.no_unicode = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_no_unicode() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.no_unicode);

    let args = parse_low_raw(["--no-unicode"]).unwrap();
    assert_eq!(true, args.no_unicode);

    let args = parse_low_raw(["--no-unicode", "--unicode"]).unwrap();
    assert_eq!(false, args.no_unicode);

    let args = parse_low_raw(["--no-unicode", "--pcre2-unicode"]).unwrap();
    assert_eq!(false, args.no_unicode);

    let args = parse_low_raw(["--no-pcre2-unicode", "--unicode"]).unwrap();
    assert_eq!(false, args.no_unicode);
}

/// -0/--null
#[derive(Debug)]

/// --null-data
#[derive(Debug)]
struct NullData;

impl Flag for NullData {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "null-data"
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Use NUL as a line terminator."
    }
    fn doc_long(&self) -> &'static str {
        r"
Enabling this flag causes ripgrep to use \fBNUL\fP as a line terminator instead
of the default of \fP\\n\fP.
.sp
This is useful when searching large binary files that would otherwise have
very long lines if \fB\\n\fP were used as the line terminator. In particular,
ripgrep requires that, at a minimum, each line must fit into memory. Using
\fBNUL\fP instead can be a useful stopgap to keep memory requirements low and
avoid OOM (out of memory) conditions.
.sp
This is also useful for processing NUL delimited data, such as that emitted
when using ripgrep's \flag{null} flag or \fBfind\fP's \fB\-\-print0\fP flag.
.sp
Using this flag implies \flag{text}. It also overrides \flag{crlf}.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--null-data has no negation");
        args.crlf = false;
        args.null_data = true;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_null_data() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.null_data);

    let args = parse_low_raw(["--null-data"]).unwrap();
    assert_eq!(true, args.null_data);

    let args = parse_low_raw(["--null-data", "--crlf"]).unwrap();
    assert_eq!(false, args.null_data);
    assert_eq!(true, args.crlf);

    let args = parse_low_raw(["--crlf", "--null-data"]).unwrap();
    assert_eq!(true, args.null_data);
    assert_eq!(false, args.crlf);

    let args = parse_low_raw(["--null-data", "--no-crlf"]).unwrap();
    assert_eq!(true, args.null_data);
    assert_eq!(false, args.crlf);
}

/// --one-file-system
#[derive(Debug)]

/// -P/--pcre2
#[derive(Debug)]
struct PCRE2;

impl Flag for PCRE2 {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'P')
    }
    fn name_long(&self) -> &'static str {
        "pcre2"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-pcre2")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Enable PCRE2 matching."
    }
    fn doc_long(&self) -> &'static str {
        r"
When this flag is present, ripgrep will use the PCRE2 regex engine instead of
its default regex engine.
.sp
This is generally useful when you want to use features such as look-around
or backreferences.
.sp
Using this flag is the same as passing \fB\-\-engine=pcre2\fP. Users may
instead elect to use \fB\-\-engine=auto\fP to ask ripgrep to automatically
select the right regex engine based on the patterns given. This flag and the
\flag{engine} flag override one another.
.sp
Note that PCRE2 is an optional ripgrep feature. If PCRE2 wasn't included in
your build of ripgrep, then using this flag will result in ripgrep printing
an error message and exiting. PCRE2 may also have worse user experience in
some cases, since it has fewer introspection APIs than ripgrep's default
regex engine. For example, if you use a \fB\\n\fP in a PCRE2 regex without
the \flag{multiline} flag, then ripgrep will silently fail to match anything
instead of reporting an error immediately (like it does with the default regex
engine).
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.engine = if v.unwrap_switch() {
            EngineChoice::PCRE2
        } else {
            EngineChoice::Default
        };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_pcre2() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(EngineChoice::Default, args.engine);

    let args = parse_low_raw(["--pcre2"]).unwrap();
    assert_eq!(EngineChoice::PCRE2, args.engine);

    let args = parse_low_raw(["-P"]).unwrap();
    assert_eq!(EngineChoice::PCRE2, args.engine);

    let args = parse_low_raw(["-P", "--no-pcre2"]).unwrap();
    assert_eq!(EngineChoice::Default, args.engine);

    let args = parse_low_raw(["--engine=auto", "-P", "--no-pcre2"]).unwrap();
    assert_eq!(EngineChoice::Default, args.engine);

    let args = parse_low_raw(["-P", "--engine=auto"]).unwrap();
    assert_eq!(EngineChoice::Auto, args.engine);
}

/// --pcre2-version
#[derive(Debug)]

/// --regex-size-limit
#[derive(Debug)]
struct RegexSizeLimit;

impl Flag for RegexSizeLimit {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "regex-size-limit"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("NUM+SUFFIX?")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"The size limit of the compiled regex."
    }
    fn doc_long(&self) -> &'static str {
        r"
The size limit of the compiled regex, where the compiled regex generally
corresponds to a single object in memory that can match all of the patterns
provided to ripgrep. The default limit is generous enough that most reasonable
patterns (or even a small number of them) should fit.
.sp
This useful to change when you explicitly want to let ripgrep spend potentially
much more time and/or memory building a regex matcher.
.sp
The input format accepts suffixes of \fBK\fP, \fBM\fP or \fBG\fP which
correspond to kilobytes, megabytes and gigabytes, respectively. If no suffix is
provided the input is treated as bytes.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let v = v.unwrap_value();
        args.regex_size_limit = Some(convert::human_readable_usize(&v)?);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_regex_size_limit() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(None, args.regex_size_limit);

    #[cfg(target_pointer_width = "64")]
    {
        let args = parse_low_raw(["--regex-size-limit", "9G"]).unwrap();
        assert_eq!(Some(9 * (1 << 30)), args.regex_size_limit);

        let args = parse_low_raw(["--regex-size-limit=9G"]).unwrap();
        assert_eq!(Some(9 * (1 << 30)), args.regex_size_limit);

        let args =
            parse_low_raw(["--regex-size-limit=9G", "--regex-size-limit=0"])
                .unwrap();
        assert_eq!(Some(0), args.regex_size_limit);
    }

    let args = parse_low_raw(["--regex-size-limit=0K"]).unwrap();
    assert_eq!(Some(0), args.regex_size_limit);

    let args = parse_low_raw(["--regex-size-limit=0M"]).unwrap();
    assert_eq!(Some(0), args.regex_size_limit);

    let args = parse_low_raw(["--regex-size-limit=0G"]).unwrap();
    assert_eq!(Some(0), args.regex_size_limit);

    let result =
        parse_low_raw(["--regex-size-limit", "9999999999999999999999"]);
    assert!(result.is_err(), "{result:?}");

    let result = parse_low_raw(["--regex-size-limit", "9999999999999999G"]);
    assert!(result.is_err(), "{result:?}");
}

/// -e/--regexp
#[derive(Debug)]

/// -S/--smart-case
#[derive(Debug)]
struct SmartCase;

impl Flag for SmartCase {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'S')
    }
    fn name_long(&self) -> &'static str {
        "smart-case"
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Smart case search."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag instructs ripgrep to searches case insensitively if the pattern is
all lowercase. Otherwise, ripgrep will search case sensitively.
.sp
A pattern is considered all lowercase if both of the following rules hold:
.sp
.IP \(bu 3n
First, the pattern contains at least one literal character. For example,
\fBa\\w\fP contains a literal (\fBa\fP) but just \fB\\w\fP does not.
.sp
.IP \(bu 3n
Second, of the literals in the pattern, none of them are considered to be
uppercase according to Unicode. For example, \fBfoo\\pL\fP has no uppercase
literals but \fBFoo\\pL\fP does.
.PP
This overrides the \flag{case-sensitive} and \flag{ignore-case} flags.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--smart-case flag has no negation");
        args.case = CaseMode::Smart;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_smart_case() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(CaseMode::Sensitive, args.case);

    let args = parse_low_raw(["--smart-case"]).unwrap();
    assert_eq!(CaseMode::Smart, args.case);

    let args = parse_low_raw(["-S"]).unwrap();
    assert_eq!(CaseMode::Smart, args.case);

    let args = parse_low_raw(["-S", "-s"]).unwrap();
    assert_eq!(CaseMode::Sensitive, args.case);

    let args = parse_low_raw(["-S", "-i"]).unwrap();
    assert_eq!(CaseMode::Insensitive, args.case);

    let args = parse_low_raw(["-s", "-S"]).unwrap();
    assert_eq!(CaseMode::Smart, args.case);

    let args = parse_low_raw(["-i", "-S"]).unwrap();
    assert_eq!(CaseMode::Smart, args.case);
}

/// --sort-files
#[derive(Debug)]

/// --stop-on-nonmatch
#[derive(Debug)]
struct StopOnNonmatch;

impl Flag for StopOnNonmatch {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "stop-on-nonmatch"
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Stop searching after a non-match."
    }
    fn doc_long(&self) -> &'static str {
        r"
Enabling this option will cause ripgrep to stop reading a file once it
encounters a non-matching line after it has encountered a matching line.
This is useful if it is expected that all matches in a given file will be on
sequential lines, for example due to the lines being sorted.
.sp
This overrides the \flag{multiline} flag.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--stop-on-nonmatch has no negation");
        args.stop_on_nonmatch = true;
        args.multiline = false;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_stop_on_nonmatch() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.stop_on_nonmatch);

    let args = parse_low_raw(["--stop-on-nonmatch"]).unwrap();
    assert_eq!(true, args.stop_on_nonmatch);

    let args = parse_low_raw(["--stop-on-nonmatch", "-U"]).unwrap();
    assert_eq!(true, args.multiline);
    assert_eq!(false, args.stop_on_nonmatch);

    let args = parse_low_raw(["-U", "--stop-on-nonmatch"]).unwrap();
    assert_eq!(false, args.multiline);
    assert_eq!(true, args.stop_on_nonmatch);

    let args =
        parse_low_raw(["--stop-on-nonmatch", "--no-multiline"]).unwrap();
    assert_eq!(false, args.multiline);
    assert_eq!(true, args.stop_on_nonmatch);
}

/// -a/--text
#[derive(Debug)]

/// -a/--text
#[derive(Debug)]
struct Text;

impl Flag for Text {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'a')
    }
    fn name_long(&self) -> &'static str {
        "text"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-text")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Search binary files as if they were text."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag instructs ripgrep to search binary files as if they were text. When
this flag is present, ripgrep's binary file detection is disabled. This means
that when a binary file is searched, its contents may be printed if there is
a match. This may cause escape codes to be printed that alter the behavior of
your terminal.
.sp
When binary file detection is enabled, it is imperfect. In general, it uses
a simple heuristic. If a \fBNUL\fP byte is seen during search, then the file
is considered binary and searching stops (unless this flag is present).
Alternatively, if the \flag{binary} flag is used, then ripgrep will only quit
when it sees a \fBNUL\fP byte after it sees a match (or searches the entire
file).
.sp
This flag overrides the \flag{binary} flag.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.binary = if v.unwrap_switch() {
            BinaryMode::AsText
        } else {
            BinaryMode::Auto
        };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_text() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(BinaryMode::Auto, args.binary);

    let args = parse_low_raw(["--text"]).unwrap();
    assert_eq!(BinaryMode::AsText, args.binary);

    let args = parse_low_raw(["-a"]).unwrap();
    assert_eq!(BinaryMode::AsText, args.binary);

    let args = parse_low_raw(["-a", "--no-text"]).unwrap();
    assert_eq!(BinaryMode::Auto, args.binary);

    let args = parse_low_raw(["-a", "--binary"]).unwrap();
    assert_eq!(BinaryMode::SearchAndSuppress, args.binary);

    let args = parse_low_raw(["--binary", "-a"]).unwrap();
    assert_eq!(BinaryMode::AsText, args.binary);

    let args = parse_low_raw(["-a", "--no-binary"]).unwrap();
    assert_eq!(BinaryMode::Auto, args.binary);

    let args = parse_low_raw(["--binary", "--no-text"]).unwrap();
    assert_eq!(BinaryMode::Auto, args.binary);
}

/// -j/--threads
#[derive(Debug)]

/// -j/--threads
#[derive(Debug)]
struct Threads;

impl Flag for Threads {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'j')
    }
    fn name_long(&self) -> &'static str {
        "threads"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("NUM")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Set the approximate number of threads to use."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag sets the approximate number of threads to use. A value of \fB0\fP
(which is the default) causes ripgrep to choose the thread count using
heuristics.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let threads = convert::usize(&v.unwrap_value())?;
        args.threads = if threads == 0 { None } else { Some(threads) };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_threads() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(None, args.threads);

    let args = parse_low_raw(["--threads", "5"]).unwrap();
    assert_eq!(Some(5), args.threads);

    let args = parse_low_raw(["-j", "5"]).unwrap();
    assert_eq!(Some(5), args.threads);

    let args = parse_low_raw(["-j5"]).unwrap();
    assert_eq!(Some(5), args.threads);

    let args = parse_low_raw(["-j5", "-j10"]).unwrap();
    assert_eq!(Some(10), args.threads);

    let args = parse_low_raw(["-j5", "-j0"]).unwrap();
    assert_eq!(None, args.threads);
}

/// --trace
#[derive(Debug)]

/// -w/--word-regexp
#[derive(Debug)]
struct WordRegexp;

impl Flag for WordRegexp {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'w')
    }
    fn name_long(&self) -> &'static str {
        "word-regexp"
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Show matches surrounded by word boundaries."
    }
    fn doc_long(&self) -> &'static str {
        r"
When enabled, ripgrep will only show matches surrounded by word boundaries.
This is equivalent to surrounding every pattern with \fB\\b{start-half}\fP
and \fB\\b{end-half}\fP.
.sp
This overrides the \flag{line-regexp} flag.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--word-regexp has no negation");
        args.boundary = Some(BoundaryMode::Word);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_word_regexp() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(None, args.boundary);

    let args = parse_low_raw(["--word-regexp"]).unwrap();
    assert_eq!(Some(BoundaryMode::Word), args.boundary);

    let args = parse_low_raw(["-w"]).unwrap();
    assert_eq!(Some(BoundaryMode::Word), args.boundary);

    let args = parse_low_raw(["-x", "-w"]).unwrap();
    assert_eq!(Some(BoundaryMode::Word), args.boundary);

    let args = parse_low_raw(["-w", "-x"]).unwrap();
    assert_eq!(Some(BoundaryMode::Line), args.boundary);
}
