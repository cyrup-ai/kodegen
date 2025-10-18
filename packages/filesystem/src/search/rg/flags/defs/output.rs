//! Output category flags.

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

/// -A/--after-context
#[derive(Debug)]
struct AfterContext;

impl Flag for AfterContext {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'A')
    }
    fn name_long(&self) -> &'static str {
        "after-context"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("NUM")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        "Show NUM lines after each match."
    }
    fn doc_long(&self) -> &'static str {
        r"
Show \fINUM\fP lines after each match.
.sp
This overrides the \flag{passthru} flag and partially overrides the
\flag{context} flag.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.context.set_after(convert::usize(&v.unwrap_value())?);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_after_context() {
    let mkctx = |lines| {
        let mut mode = ContextMode::default();
        mode.set_after(lines);
        mode
    };

    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(ContextMode::default(), args.context);

    let args = parse_low_raw(["--after-context", "5"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["--after-context=5"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["-A", "5"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["-A5"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["-A5", "-A10"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(10), args.context);

    let args = parse_low_raw(["-A5", "-A0"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(0), args.context);

    let args = parse_low_raw(["-A5", "--passthru"]).expect("Test parsing should succeed");
    assert_eq!(ContextMode::Passthru, args.context);

    let args = parse_low_raw(["--passthru", "-A5"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(5), args.context);

    let n = usize::MAX.to_string();
    let args = parse_low_raw(["--after-context", n.as_str()]).expect("Test parsing should succeed");
    assert_eq!(mkctx(usize::MAX), args.context);

    #[cfg(target_pointer_width = "64")]
    {
        let n = (u128::from(u64::MAX) + 1).to_string();
        let result = parse_low_raw(["--after-context", n.as_str()]);
        assert!(result.is_err(), "{result:?}");
    }
}

/// --auto-hybrid-regex
#[derive(Debug)]

/// -B/--before-context
#[derive(Debug)]
struct BeforeContext;

impl Flag for BeforeContext {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'B')
    }
    fn name_long(&self) -> &'static str {
        "before-context"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("NUM")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        "Show NUM lines before each match."
    }
    fn doc_long(&self) -> &'static str {
        r"
Show \fINUM\fP lines before each match.
.sp
This overrides the \flag{passthru} flag and partially overrides the
\flag{context} flag.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.context.set_before(convert::usize(&v.unwrap_value())?);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_before_context() {
    let mkctx = |lines| {
        let mut mode = ContextMode::default();
        mode.set_before(lines);
        mode
    };

    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(ContextMode::default(), args.context);

    let args = parse_low_raw(["--before-context", "5"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["--before-context=5"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["-B", "5"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["-B5"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["-B5", "-B10"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(10), args.context);

    let args = parse_low_raw(["-B5", "-B0"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(0), args.context);

    let args = parse_low_raw(["-B5", "--passthru"]).expect("Test parsing should succeed");
    assert_eq!(ContextMode::Passthru, args.context);

    let args = parse_low_raw(["--passthru", "-B5"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(5), args.context);

    let n = usize::MAX.to_string();
    let args = parse_low_raw(["--before-context", n.as_str()]).expect("Test parsing should succeed");
    assert_eq!(mkctx(usize::MAX), args.context);

    #[cfg(target_pointer_width = "64")]
    {
        let n = (u128::from(u64::MAX) + 1).to_string();
        let result = parse_low_raw(["--before-context", n.as_str()]);
        assert!(result.is_err(), "{result:?}");
    }
}

/// --binary
#[derive(Debug)]

/// --byte-offset
#[derive(Debug)]
struct ByteOffset;

impl Flag for ByteOffset {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'b')
    }
    fn name_long(&self) -> &'static str {
        "byte-offset"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-byte-offset")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        "Print the byte offset for each matching line."
    }
    fn doc_long(&self) -> &'static str {
        r"
Print the 0-based byte offset within the input file before each line of output.
If \flag{only-matching} is specified, print the offset of the matched text
itself.
.sp
If ripgrep does transcoding, then the byte offset is in terms of the result
of transcoding and not the original data. This applies similarly to other
transformations on the data, such as decompression or a \flag{pre} filter.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.byte_offset = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_byte_offset() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.byte_offset);

    let args = parse_low_raw(["--byte-offset"]).expect("Test parsing should succeed");
    assert_eq!(true, args.byte_offset);

    let args = parse_low_raw(["-b"]).expect("Test parsing should succeed");
    assert_eq!(true, args.byte_offset);

    let args = parse_low_raw(["--byte-offset", "--no-byte-offset"]).expect("Test parsing should succeed");
    assert_eq!(false, args.byte_offset);

    let args = parse_low_raw(["--no-byte-offset", "-b"]).expect("Test parsing should succeed");
    assert_eq!(true, args.byte_offset);
}

/// -s/--case-sensitive
#[derive(Debug)]

/// --color
#[derive(Debug)]
struct Color;

impl Flag for Color {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "color"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("WHEN")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        "When to use color."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag controls when to use colors. The default setting is \fBauto\fP, which
means ripgrep will try to guess when to use colors. For example, if ripgrep is
printing to a tty, then it will use colors, but if it is redirected to a file
or a pipe, then it will suppress color output.
.sp
ripgrep will suppress color output by default in some other circumstances as
well. These include, but are not limited to:
.sp
.IP \(bu 3n
When the \fBTERM\fP environment variable is not set or set to \fBdumb\fP.
.sp
.IP \(bu 3n
When the \fBNO_COLOR\fP environment variable is set (regardless of value).
.sp
.IP \(bu 3n
When flags that imply no use for colors are given. For example,
\flag{vimgrep} and \flag{json}.
.
.PP
The possible values for this flag are:
.sp
.IP \fBnever\fP 10n
Colors will never be used.
.sp
.IP \fBauto\fP 10n
The default. ripgrep tries to be smart.
.sp
.IP \fBalways\fP 10n
Colors will always be used regardless of where output is sent.
.sp
.IP \fBansi\fP 10n
Like 'always', but emits ANSI escapes (even in a Windows console).
.
.PP
This flag also controls whether hyperlinks are emitted. For example, when
a hyperlink format is specified, hyperlinks won't be used when color is
suppressed. If one wants to emit hyperlinks but no colors, then one must use
the \flag{colors} flag to manually set all color styles to \fBnone\fP:
.sp
.EX
    \-\-colors 'path:none' \\
    \-\-colors 'line:none' \\
    \-\-colors 'column:none' \\
    \-\-colors 'match:none' \\
    \-\-colors 'highlight:none'
.EE
.sp
"
    }
    fn doc_choices(&self) -> &'static [&'static str] {
        &["never", "auto"]
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.color = match convert::str(&v.unwrap_value())? {
            "never" => ColorChoice::Never,
            "auto" => ColorChoice::Auto,
            unk => anyhow::bail!("choice '{unk}' is unrecognized"),
        };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_color() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(ColorChoice::Auto, args.color);

    let args = parse_low_raw(["--color", "never"]).expect("Test parsing should succeed");
    assert_eq!(ColorChoice::Never, args.color);

    let args = parse_low_raw(["--color", "auto"]).expect("Test parsing should succeed");
    assert_eq!(ColorChoice::Auto, args.color);

    let args = parse_low_raw(["--color=never"]).expect("Test parsing should succeed");
    assert_eq!(ColorChoice::Never, args.color);

    let args =
        parse_low_raw(["--color", "auto", "--color", "never"]).expect("Test parsing should succeed");
    assert_eq!(ColorChoice::Never, args.color);

    let args =
        parse_low_raw(["--color", "never", "--color", "auto"]).expect("Test parsing should succeed");
    assert_eq!(ColorChoice::Auto, args.color);

    let result = parse_low_raw(["--color", "foofoo"]);
    assert!(result.is_err(), "{result:?}");

    let result = parse_low_raw(["--color", "always"]);
    assert!(result.is_err(), "{result:?}");

    let result = parse_low_raw(["--color", "ansi"]);
    assert!(result.is_err(), "{result:?}");
}

/// --colors
#[derive(Debug)]

/// --colors
#[derive(Debug)]
struct Colors;

impl Flag for Colors {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "colors"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("COLOR_SPEC")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        "Configure color settings and styles."
    }
    fn doc_long(&self) -> &'static str {
        r#"
This flag specifies color settings for use in the output. This flag may be
provided multiple times. Settings are applied iteratively. Pre-existing color
labels are limited to one of eight choices: \fBred\fP, \fBblue\fP, \fBgreen\fP,
\fBcyan\fP, \fBmagenta\fP, \fByellow\fP, \fBwhite\fP and \fBblack\fP. Styles
are limited to \fBnobold\fP, \fBbold\fP, \fBnointense\fP, \fBintense\fP,
\fBnounderline\fP, \fBunderline\fP, \fBnoitalic\fP or \fBitalic\fP.
.sp
The format of the flag is
\fB{\fP\fItype\fP\fB}:{\fP\fIattribute\fP\fB}:{\fP\fIvalue\fP\fB}\fP.
\fItype\fP should be one of \fBpath\fP, \fBline\fP, \fBcolumn\fP,
\fBhighlight\fP or \fBmatch\fP. \fIattribute\fP can be \fBfg\fP, \fBbg\fP or
\fBstyle\fP. \fIvalue\fP is either a color (for \fBfg\fP and \fBbg\fP) or a
text style. A special format, \fB{\fP\fItype\fP\fB}:none\fP, will clear all
color settings for \fItype\fP.
.sp
For example, the following command will change the match color to magenta and
the background color for line numbers to yellow:
.sp
.EX
    rg \-\-colors 'match:fg:magenta' \-\-colors 'line:bg:yellow'
.EE
.sp
Another example, the following command will "highlight" the non-matching text
in matching lines:
.sp
.EX
    rg \-\-colors 'highlight:bg:yellow' \-\-colors 'highlight:fg:black'
.EE
.sp
The "highlight" color type is particularly useful for contrasting matching
lines with surrounding context printed by the \flag{before-context},
\flag{after-context}, \flag{context} or \flag{passthru} flags.
.sp
Extended colors can be used for \fIvalue\fP when the tty supports ANSI color
sequences. These are specified as either \fIx\fP (256-color) or
.IB x , x , x
(24-bit truecolor) where \fIx\fP is a number between \fB0\fP and \fB255\fP
inclusive. \fIx\fP may be given as a normal decimal number or a hexadecimal
number, which is prefixed by \fB0x\fP.
.sp
For example, the following command will change the match background color to
that represented by the rgb value (0,128,255):
.sp
.EX
    rg \-\-colors 'match:bg:0,128,255'
.EE
.sp
or, equivalently,
.sp
.EX
    rg \-\-colors 'match:bg:0x0,0x80,0xFF'
.EE
.sp
Note that the \fBintense\fP and \fBnointense\fP styles will have no effect when
used alongside these extended color codes.
"#
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let v = v.unwrap_value();
        let v = convert::str(&v)?;
        args.colors.push(v.parse()?);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_colors() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert!(args.colors.is_empty());

    let args = parse_low_raw(["--colors", "match:fg:magenta"]).expect("Test parsing should succeed");
    assert_eq!(args.colors, vec!["match:fg:magenta".parse().expect("Test parsing should succeed")]);

    let args = parse_low_raw([
        "--colors",
        "match:fg:magenta",
        "--colors",
        "line:bg:yellow",
    ])
    .expect("Test parsing should succeed");
    assert_eq!(
        args.colors,
        vec![
            "match:fg:magenta".parse().expect("Test parsing should succeed"),
            "line:bg:yellow".parse().expect("Test parsing should succeed")
        ]
    );

    let args = parse_low_raw(["--colors", "highlight:bg:240"]).expect("Test parsing should succeed");
    assert_eq!(args.colors, vec!["highlight:bg:240".parse().expect("Test parsing should succeed")]);

    let args = parse_low_raw([
        "--colors",
        "match:fg:magenta",
        "--colors",
        "highlight:bg:blue",
    ])
    .expect("Test parsing should succeed");
    assert_eq!(
        args.colors,
        vec![
            "match:fg:magenta".parse().expect("Test parsing should succeed"),
            "highlight:bg:blue".parse().expect("Test parsing should succeed")
        ]
    );
}

/// --column
#[derive(Debug)]

/// --column
#[derive(Debug)]
struct Column;

impl Flag for Column {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "column"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-column")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        "Show column numbers."
    }
    fn doc_long(&self) -> &'static str {
        r"
Show column numbers (1-based). This only shows the column numbers for the first
match on each line. This does not try to account for Unicode. One byte is equal
to one column. This implies \flag{line-number}.
.sp
When \flag{only-matching} is used, then the column numbers written correspond
to the start of each match.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.column = Some(v.unwrap_switch());
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_column() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.column);

    let args = parse_low_raw(["--column"]).expect("Test parsing should succeed");
    assert_eq!(Some(true), args.column);

    let args = parse_low_raw(["--column", "--no-column"]).expect("Test parsing should succeed");
    assert_eq!(Some(false), args.column);

    let args = parse_low_raw(["--no-column", "--column"]).expect("Test parsing should succeed");
    assert_eq!(Some(true), args.column);
}

/// -C/--context
#[derive(Debug)]

/// -C/--context
#[derive(Debug)]
struct Context;

impl Flag for Context {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'C')
    }
    fn name_long(&self) -> &'static str {
        "context"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("NUM")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Show NUM lines before and after each match."
    }
    fn doc_long(&self) -> &'static str {
        r"
Show \fINUM\fP lines before and after each match. This is equivalent to
providing both the \flag{before-context} and \flag{after-context} flags with
the same value.
.sp
This overrides the \flag{passthru} flag. The \flag{after-context} and
\flag{before-context} flags both partially override this flag, regardless of
the order. For example, \fB\-A2 \-C1\fP is equivalent to \fB\-A2 \-B1\fP.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.context.set_both(convert::usize(&v.unwrap_value())?);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_context() {
    let mkctx = |lines| {
        let mut mode = ContextMode::default();
        mode.set_both(lines);
        mode
    };

    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(ContextMode::default(), args.context);

    let args = parse_low_raw(["--context", "5"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["--context=5"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["-C", "5"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["-C5"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["-C5", "-C10"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(10), args.context);

    let args = parse_low_raw(["-C5", "-C0"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(0), args.context);

    let args = parse_low_raw(["-C5", "--passthru"]).expect("Test parsing should succeed");
    assert_eq!(ContextMode::Passthru, args.context);

    let args = parse_low_raw(["--passthru", "-C5"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(5), args.context);

    let n = usize::MAX.to_string();
    let args = parse_low_raw(["--context", n.as_str()]).expect("Test parsing should succeed");
    assert_eq!(mkctx(usize::MAX), args.context);

    #[cfg(target_pointer_width = "64")]
    {
        let n = (u128::from(u64::MAX) + 1).to_string();
        let result = parse_low_raw(["--context", n.as_str()]);
        assert!(result.is_err(), "{result:?}");
    }

    // Test the interaction between -A/-B and -C. Basically, -A/-B always
    // partially overrides -C, regardless of where they appear relative to
    // each other. This behavior is also how GNU grep works, and it also makes
    // logical sense to me: -A/-B are the more specific flags.
    let args = parse_low_raw(["-A1", "-C5"]).expect("Test parsing should succeed");
    let mut mode = ContextMode::default();
    mode.set_after(1);
    mode.set_both(5);
    assert_eq!(mode, args.context);
    assert_eq!((5, 1), args.context.get_limited());

    let args = parse_low_raw(["-B1", "-C5"]).expect("Test parsing should succeed");
    let mut mode = ContextMode::default();
    mode.set_before(1);
    mode.set_both(5);
    assert_eq!(mode, args.context);
    assert_eq!((1, 5), args.context.get_limited());

    let args = parse_low_raw(["-A1", "-B2", "-C5"]).expect("Test parsing should succeed");
    let mut mode = ContextMode::default();
    mode.set_before(2);
    mode.set_after(1);
    mode.set_both(5);
    assert_eq!(mode, args.context);
    assert_eq!((2, 1), args.context.get_limited());

    // These next three are like the ones above, but with -C before -A/-B. This
    // tests that -A and -B only partially override -C. That is, -C1 -A2 is
    // equivalent to -B1 -A2.
    let args = parse_low_raw(["-C5", "-A1"]).expect("Test parsing should succeed");
    let mut mode = ContextMode::default();
    mode.set_after(1);
    mode.set_both(5);
    assert_eq!(mode, args.context);
    assert_eq!((5, 1), args.context.get_limited());

    let args = parse_low_raw(["-C5", "-B1"]).expect("Test parsing should succeed");
    let mut mode = ContextMode::default();
    mode.set_before(1);
    mode.set_both(5);
    assert_eq!(mode, args.context);
    assert_eq!((1, 5), args.context.get_limited());

    let args = parse_low_raw(["-C5", "-A1", "-B2"]).expect("Test parsing should succeed");
    let mut mode = ContextMode::default();
    mode.set_before(2);
    mode.set_after(1);
    mode.set_both(5);
    assert_eq!(mode, args.context);
    assert_eq!((2, 1), args.context.get_limited());
}

/// --context-separator
#[derive(Debug)]

/// --context-separator
#[derive(Debug)]
struct ContextSeparator;

impl Flag for ContextSeparator {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "context-separator"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-context-separator")
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("SEPARATOR")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Set the separator for contextual chunks."
    }
    fn doc_long(&self) -> &'static str {
        r"
The string used to separate non-contiguous context lines in the output. This is
only used when one of the context flags is used (that is, \flag{after-context},
\flag{before-context} or \flag{context}). Escape sequences like \fB\\x7F\fP or
\fB\\t\fP may be used. The default value is \fB\-\-\fP.
.sp
When the context separator is set to an empty string, then a line break
is still inserted. To completely disable context separators, use the
\flag-negate{context-separator} flag.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        use crate::search::rg::flags::lowargs::ContextSeparator as Separator;

        args.context_separator = match v {
            FlagValue::Switch(true) => {
                unreachable!("flag can only be disabled")
            }
            FlagValue::Switch(false) => Separator::disabled(),
            FlagValue::Value(v) => Separator::new(&v)?,
        };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_context_separator() {
    use bstr::BString;

    use crate::search::rg::flags::lowargs::ContextSeparator as Separator;

    let getbytes = |ctxsep: Separator| ctxsep.into_bytes().map(BString::from);

    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(Some(BString::from("--")), getbytes(args.context_separator));

    let args = parse_low_raw(["--context-separator", "XYZ"]).expect("Test parsing should succeed");
    assert_eq!(Some(BString::from("XYZ")), getbytes(args.context_separator));

    let args = parse_low_raw(["--no-context-separator"]).expect("Test parsing should succeed");
    assert_eq!(None, getbytes(args.context_separator));

    let args = parse_low_raw([
        "--context-separator",
        "XYZ",
        "--no-context-separator",
    ])
    .expect("Test parsing should succeed");
    assert_eq!(None, getbytes(args.context_separator));

    let args = parse_low_raw([
        "--no-context-separator",
        "--context-separator",
        "XYZ",
    ])
    .expect("Test parsing should succeed");
    assert_eq!(Some(BString::from("XYZ")), getbytes(args.context_separator));

    // This checks that invalid UTF-8 can be used. This case isn't too tricky
    // to handle, because it passes the invalid UTF-8 as an escape sequence
    // that is itself valid UTF-8. It doesn't become invalid UTF-8 until after
    // the argument is parsed and then unescaped.
    let args = parse_low_raw(["--context-separator", r"\xFF"]).expect("Test parsing should succeed");
    assert_eq!(Some(BString::from(b"\xFF")), getbytes(args.context_separator));

    // In this case, we specifically try to pass an invalid UTF-8 argument to
    // the flag. In theory we might be able to support this, but because we do
    // unescaping and because unescaping wants valid UTF-8, we do a UTF-8 check
    // on the value. Since we pass invalid UTF-8, it fails. This demonstrates
    // that the only way to use an invalid UTF-8 separator is by specifying an
    // escape sequence that is itself valid UTF-8.
    #[cfg(unix)]
    {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

        let result = parse_low_raw([
            OsStr::from_bytes(b"--context-separator"),
            OsStr::from_bytes(&[0xFF]),
        ]);
        assert!(result.is_err(), "{result:?}");
    }
}

/// -c/--count
#[derive(Debug)]

/// --field-context-separator
#[derive(Debug)]
struct FieldContextSeparator;

impl Flag for FieldContextSeparator {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "field-context-separator"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("SEPARATOR")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Set the field context separator."
    }
    fn doc_long(&self) -> &'static str {
        r"
Set the field context separator. This separator is only used when printing
contextual lines. It is used to delimit file paths, line numbers, columns and
the contextual line itself. The separator may be any number of bytes, including
zero. Escape sequences like \fB\\x7F\fP or \fB\\t\fP may be used.
.sp
The \fB-\fP character is the default value.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        use crate::search::rg::flags::lowargs::FieldContextSeparator as Separator;

        args.field_context_separator = Separator::new(&v.unwrap_value())?;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_field_context_separator() {
    use bstr::BString;

    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(BString::from("-"), args.field_context_separator.into_bytes());

    let args = parse_low_raw(["--field-context-separator", "XYZ"]).expect("Test parsing should succeed");
    assert_eq!(
        BString::from("XYZ"),
        args.field_context_separator.into_bytes()
    );

    let args = parse_low_raw(["--field-context-separator=XYZ"]).expect("Test parsing should succeed");
    assert_eq!(
        BString::from("XYZ"),
        args.field_context_separator.into_bytes()
    );

    let args = parse_low_raw([
        "--field-context-separator",
        "XYZ",
        "--field-context-separator",
        "ABC",
    ])
    .expect("Test parsing should succeed");
    assert_eq!(
        BString::from("ABC"),
        args.field_context_separator.into_bytes()
    );

    let args = parse_low_raw(["--field-context-separator", r"\t"]).expect("Test parsing should succeed");
    assert_eq!(BString::from("\t"), args.field_context_separator.into_bytes());

    let args = parse_low_raw(["--field-context-separator", r"\x00"]).expect("Test parsing should succeed");
    assert_eq!(
        BString::from("\x00"),
        args.field_context_separator.into_bytes()
    );

    // This checks that invalid UTF-8 can be used. This case isn't too tricky
    // to handle, because it passes the invalid UTF-8 as an escape sequence
    // that is itself valid UTF-8. It doesn't become invalid UTF-8 until after
    // the argument is parsed and then unescaped.
    let args = parse_low_raw(["--field-context-separator", r"\xFF"]).expect("Test parsing should succeed");
    assert_eq!(
        BString::from(b"\xFF"),
        args.field_context_separator.into_bytes()
    );

    // In this case, we specifically try to pass an invalid UTF-8 argument to
    // the flag. In theory we might be able to support this, but because we do
    // unescaping and because unescaping wants valid UTF-8, we do a UTF-8 check
    // on the value. Since we pass invalid UTF-8, it fails. This demonstrates
    // that the only way to use an invalid UTF-8 separator is by specifying an
    // escape sequence that is itself valid UTF-8.
    #[cfg(unix)]
    {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

        let result = parse_low_raw([
            OsStr::from_bytes(b"--field-context-separator"),
            OsStr::from_bytes(&[0xFF]),
        ]);
        assert!(result.is_err(), "{result:?}");
    }
}

/// --field-match-separator
#[derive(Debug)]

/// --field-match-separator
#[derive(Debug)]
struct FieldMatchSeparator;

impl Flag for FieldMatchSeparator {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "field-match-separator"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("SEPARATOR")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Set the field match separator."
    }
    fn doc_long(&self) -> &'static str {
        r"
Set the field match separator. This separator is only used when printing
matching lines. It is used to delimit file paths, line numbers, columns and the
matching line itself. The separator may be any number of bytes, including zero.
Escape sequences like \fB\\x7F\fP or \fB\\t\fP may be used.
.sp
The \fB:\fP character is the default value.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        use crate::search::rg::flags::lowargs::FieldMatchSeparator as Separator;

        args.field_match_separator = Separator::new(&v.unwrap_value())?;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_field_match_separator() {
    use bstr::BString;

    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(BString::from(":"), args.field_match_separator.into_bytes());

    let args = parse_low_raw(["--field-match-separator", "XYZ"]).expect("Test parsing should succeed");
    assert_eq!(BString::from("XYZ"), args.field_match_separator.into_bytes());

    let args = parse_low_raw(["--field-match-separator=XYZ"]).expect("Test parsing should succeed");
    assert_eq!(BString::from("XYZ"), args.field_match_separator.into_bytes());

    let args = parse_low_raw([
        "--field-match-separator",
        "XYZ",
        "--field-match-separator",
        "ABC",
    ])
    .expect("Test parsing should succeed");
    assert_eq!(BString::from("ABC"), args.field_match_separator.into_bytes());

    let args = parse_low_raw(["--field-match-separator", r"\t"]).expect("Test parsing should succeed");
    assert_eq!(BString::from("\t"), args.field_match_separator.into_bytes());

    let args = parse_low_raw(["--field-match-separator", r"\x00"]).expect("Test parsing should succeed");
    assert_eq!(BString::from("\x00"), args.field_match_separator.into_bytes());

    // This checks that invalid UTF-8 can be used. This case isn't too tricky
    // to handle, because it passes the invalid UTF-8 as an escape sequence
    // that is itself valid UTF-8. It doesn't become invalid UTF-8 until after
    // the argument is parsed and then unescaped.
    let args = parse_low_raw(["--field-match-separator", r"\xFF"]).expect("Test parsing should succeed");
    assert_eq!(
        BString::from(b"\xFF"),
        args.field_match_separator.into_bytes()
    );

    // In this case, we specifically try to pass an invalid UTF-8 argument to
    // the flag. In theory we might be able to support this, but because we do
    // unescaping and because unescaping wants valid UTF-8, we do a UTF-8 check
    // on the value. Since we pass invalid UTF-8, it fails. This demonstrates
    // that the only way to use an invalid UTF-8 separator is by specifying an
    // escape sequence that is itself valid UTF-8.
    #[cfg(unix)]
    {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

        let result = parse_low_raw([
            OsStr::from_bytes(b"--field-match-separator"),
            OsStr::from_bytes(&[0xFF]),
        ]);
        assert!(result.is_err(), "{result:?}");
    }
}

/// -f/--file
#[derive(Debug)]

/// --heading
#[derive(Debug)]
struct Heading;

impl Flag for Heading {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "heading"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-heading")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Print matches grouped by each file."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag prints the file path above clusters of matches from each file instead
of printing the file path as a prefix for each matched line.
.sp
This is the default mode when printing to a tty.
.sp
When \fBstdout\fP is not a tty, then ripgrep will default to the standard
grep-like format. One can force this format in Unix-like environments by
piping the output of ripgrep to \fBcat\fP. For example, \fBrg\fP \fIfoo\fP \fB|
cat\fP.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.heading = Some(v.unwrap_switch());
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_heading() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.heading);

    let args = parse_low_raw(["--heading"]).expect("Test parsing should succeed");
    assert_eq!(Some(true), args.heading);

    let args = parse_low_raw(["--no-heading"]).expect("Test parsing should succeed");
    assert_eq!(Some(false), args.heading);

    let args = parse_low_raw(["--heading", "--no-heading"]).expect("Test parsing should succeed");
    assert_eq!(Some(false), args.heading);

    let args = parse_low_raw(["--no-heading", "--heading"]).expect("Test parsing should succeed");
    assert_eq!(Some(true), args.heading);
}

/// -h/--help
#[derive(Debug)]

/// -h/--help
#[derive(Debug)]
struct Help;

impl Flag for Help {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "help"
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'h')
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Show help output."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag prints the help output for ripgrep.
.sp
Unlike most other flags, the behavior of the short flag, \fB\-h\fP, and the
long flag, \fB\-\-help\fP, is different. The short flag will show a condensed
help output while the long flag will show a verbose help output. The verbose
help output has complete documentation, where as the condensed help output will
show only a single line for every flag.
"
    }

    fn update(&self, v: FlagValue, _: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--help has no negation");
        // Since this flag has different semantics for -h and --help and the
        // Flag trait doesn't support encoding this sort of thing, we handle it
        // as a special case in the parser.
        Ok(())
    }
}

/// -./--hidden
#[derive(Debug)]

/// --hostname-bin
#[derive(Debug)]
struct HostnameBin;

impl Flag for HostnameBin {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "hostname-bin"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("COMMAND")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Run a program to get this system's hostname."
    }
    fn doc_long(&self) -> &'static str {
        r#"
This flag controls how ripgrep determines this system's hostname. The flag's
value should correspond to an executable (either a path or something that can
be found via your system's \fBPATH\fP environment variable). When set, ripgrep
will run this executable, with no arguments, and treat its output (with leading
and trailing whitespace stripped) as your system's hostname.
.sp
When not set (the default, or the empty string), ripgrep will try to
automatically detect your system's hostname. On Unix, this corresponds
to calling \fBgethostname\fP. On Windows, this corresponds to calling
\fBGetComputerNameExW\fP to fetch the system's "physical DNS hostname."
.sp
ripgrep uses your system's hostname for producing hyperlinks.
"#
    }
    fn completion_type(&self) -> CompletionType {
        CompletionType::Executable
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let path = PathBuf::from(v.unwrap_value());
        args.hostname_bin =
            if path.as_os_str().is_empty() { None } else { Some(path) };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_hostname_bin() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.hostname_bin);

    let args = parse_low_raw(["--hostname-bin", "foo"]).expect("Test parsing should succeed");
    assert_eq!(Some(PathBuf::from("foo")), args.hostname_bin);

    let args = parse_low_raw(["--hostname-bin=foo"]).expect("Test parsing should succeed");
    assert_eq!(Some(PathBuf::from("foo")), args.hostname_bin);
}

/// --hyperlink-format
#[derive(Debug)]

/// --hyperlink-format
#[derive(Debug)]
struct HyperlinkFormat;

impl Flag for HyperlinkFormat {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "hyperlink-format"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("FORMAT")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Set the format of hyperlinks."
    }
    fn doc_long(&self) -> &'static str {
        static DOC: LazyLock<String> = LazyLock::new(|| {
            let mut doc = String::new();
            doc.push_str(
                r#"
Set the format of hyperlinks to use when printing results. Hyperlinks make
certain elements of ripgrep's output, such as file paths, clickable. This
generally only works in terminal emulators that support OSC-8 hyperlinks. For
example, the format \fBfile://{host}{path}\fP will emit an RFC 8089 hyperlink.
To see the format that ripgrep is using, pass the \flag{debug} flag.
.sp
Alternatively, a format string may correspond to one of the following aliases:
"#,
            );

            let mut aliases = grep::printer::hyperlink_aliases();
            aliases.sort_by_key(|alias| {
                alias.display_priority().unwrap_or(i16::MAX)
            });
            for (i, alias) in aliases.iter().enumerate() {
                doc.push_str(r"\fB");
                doc.push_str(alias.name());
                doc.push_str(r"\fP");
                doc.push_str(if i < aliases.len() - 1 { ", " } else { "." });
            }
            doc.push_str(
                r#"
The alias will be replaced with a format string that is intended to work for
the corresponding application.
.sp
The following variables are available in the format string:
.sp
.TP 12
\fB{path}\fP
Required. This is replaced with a path to a matching file. The path is
guaranteed to be absolute and percent encoded such that it is valid to put into
a URI. Note that a path is guaranteed to start with a /.
.TP 12
\fB{host}\fP
Optional. This is replaced with your system's hostname. On Unix, this
corresponds to calling \fBgethostname\fP. On Windows, this corresponds to
calling \fBGetComputerNameExW\fP to fetch the system's "physical DNS hostname."
Alternatively, if \flag{hostname-bin} was provided, then the hostname returned
from the output of that program will be returned. If no hostname could be
found, then this variable is replaced with the empty string.
.TP 12
\fB{line}\fP
Optional. If appropriate, this is replaced with the line number of a match. If
no line number is available (for example, if \fB\-\-no\-line\-number\fP was
given), then it is automatically replaced with the value 1.
.TP 12
\fB{column}\fP
Optional, but requires the presence of \fB{line}\fP. If appropriate, this is
replaced with the column number of a match. If no column number is available
(for example, if \fB\-\-no\-column\fP was given), then it is automatically
replaced with the value 1.
.TP 12
\fB{wslprefix}\fP
Optional. This is a special value that is set to
\fBwsl$/\fP\fIWSL_DISTRO_NAME\fP, where \fIWSL_DISTRO_NAME\fP corresponds to
the value of the equivalent environment variable. If the system is not Unix
or if the \fIWSL_DISTRO_NAME\fP environment variable is not set, then this is
replaced with the empty string.
.PP
A format string may be empty. An empty format string is equivalent to the
\fBnone\fP alias. In this case, hyperlinks will be disabled.
.sp
At present, ripgrep does not enable hyperlinks by default. Users must opt into
them. If you aren't sure what format to use, try \fBdefault\fP.
.sp
Like colors, when ripgrep detects that stdout is not connected to a tty, then
hyperlinks are automatically disabled, regardless of the value of this flag.
Users can pass \fB\-\-color=always\fP to forcefully emit hyperlinks.
.sp
Note that hyperlinks are only written when a path is also in the output
and colors are enabled. To write hyperlinks without colors, you'll need to
configure ripgrep to not colorize anything without actually disabling all ANSI
escape codes completely:
.sp
.EX
    \-\-colors 'path:none' \\
    \-\-colors 'line:none' \\
    \-\-colors 'column:none' \\
    \-\-colors 'match:none'
.EE
.sp
ripgrep works this way because it treats the \flag{color} flag as a proxy for
whether ANSI escape codes should be used at all. This means that environment
variables like \fBNO_COLOR=1\fP and \fBTERM=dumb\fP not only disable colors,
but hyperlinks as well. Similarly, colors and hyperlinks are disabled when
ripgrep is not writing to a tty. (Unless one forces the issue by setting
\fB\-\-color=always\fP.)
.sp
If you're searching a file directly, for example:
.sp
.EX
    rg foo path/to/file
.EE
.sp
then hyperlinks will not be emitted since the path given does not appear
in the output. To make the path appear, and thus also a hyperlink, use the
\flag{with-filename} flag.
.sp
For more information on hyperlinks in terminal emulators, see:
https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda
"#,
            );
            doc
        });
        &DOC
    }

    fn doc_choices(&self) -> &'static [&'static str] {
        static CHOICES: LazyLock<Vec<String>> = LazyLock::new(|| {
            let mut aliases = grep::printer::hyperlink_aliases();
            aliases.sort_by_key(|alias| {
                alias.display_priority().unwrap_or(i16::MAX)
            });
            aliases.iter().map(|alias| alias.name().to_string()).collect()
        });
        static BORROWED: LazyLock<Vec<&'static str>> =
            LazyLock::new(|| CHOICES.iter().map(|name| &**name).collect());
        &*BORROWED
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let v = v.unwrap_value();
        let string = convert::str(&v)?;
        let format = string.parse().context("invalid hyperlink format")?;
        args.hyperlink_format = format;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_hyperlink_format() {
    let parseformat = |format: &str| {
        format.parse::<grep::printer::HyperlinkFormat>().expect("Test parsing should succeed")
    };

    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(parseformat("none"), args.hyperlink_format);

    let args = parse_low_raw(["--hyperlink-format", "default"]).expect("Test parsing should succeed");
    #[cfg(windows)]
    assert_eq!(parseformat("file://{path}"), args.hyperlink_format);
    #[cfg(not(windows))]
    assert_eq!(parseformat("file://{host}{path}"), args.hyperlink_format);

    let args = parse_low_raw(["--hyperlink-format", "file"]).expect("Test parsing should succeed");
    assert_eq!(parseformat("file://{host}{path}"), args.hyperlink_format);

    let args = parse_low_raw([
        "--hyperlink-format",
        "file",
        "--hyperlink-format=grep+",
    ])
    .expect("Test parsing should succeed");
    assert_eq!(parseformat("grep+://{path}:{line}"), args.hyperlink_format);

    let args =
        parse_low_raw(["--hyperlink-format", "file://{host}{path}#{line}"])
            .expect("Test parsing should succeed");
    assert_eq!(
        parseformat("file://{host}{path}#{line}"),
        args.hyperlink_format
    );

    let result = parse_low_raw(["--hyperlink-format", "file://heythere"]);
    assert!(result.is_err(), "{result:?}");
}

/// --iglob
#[derive(Debug)]

/// --include-zero
#[derive(Debug)]
struct IncludeZero;

impl Flag for IncludeZero {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "include-zero"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-include-zero")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Include zero matches in summary output."
    }
    fn doc_long(&self) -> &'static str {
        r"
When used with \flag{count} or \flag{count-matches}, this causes ripgrep to
print the number of matches for each file even if there were zero matches. This
is disabled by default but can be enabled to make ripgrep behave more like
grep.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.include_zero = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_include_zero() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.include_zero);

    let args = parse_low_raw(["--include-zero"]).expect("Test parsing should succeed");
    assert_eq!(true, args.include_zero);

    let args = parse_low_raw(["--include-zero", "--no-include-zero"]).expect("Test parsing should succeed");
    assert_eq!(false, args.include_zero);
}

/// -v/--invert-match
#[derive(Debug)]

/// -n/--line-number
#[derive(Debug)]
struct LineNumber;

impl Flag for LineNumber {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'n')
    }
    fn name_long(&self) -> &'static str {
        "line-number"
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Show line numbers."
    }
    fn doc_long(&self) -> &'static str {
        r"
Show line numbers (1-based).
.sp
This is enabled by default when stdout is connected to a tty.
.sp
This flag can be disabled by \flag{no-line-number}.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--line-number has no automatic negation");
        args.line_number = Some(true);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_line_number() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.line_number);

    let args = parse_low_raw(["--line-number"]).expect("Test parsing should succeed");
    assert_eq!(Some(true), args.line_number);

    let args = parse_low_raw(["-n"]).expect("Test parsing should succeed");
    assert_eq!(Some(true), args.line_number);

    let args = parse_low_raw(["-n", "--no-line-number"]).expect("Test parsing should succeed");
    assert_eq!(Some(false), args.line_number);
}

/// -N/--no-line-number
#[derive(Debug)]

/// -N/--no-line-number
#[derive(Debug)]
struct LineNumberNo;

impl Flag for LineNumberNo {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'N')
    }
    fn name_long(&self) -> &'static str {
        "no-line-number"
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Suppress line numbers."
    }
    fn doc_long(&self) -> &'static str {
        r"
Suppress line numbers.
.sp
Line numbers are off by default when stdout is not connected to a tty.
.sp
Line numbers can be forcefully turned on by \flag{line-number}.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(
            v.unwrap_switch(),
            "--no-line-number has no automatic negation"
        );
        args.line_number = Some(false);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_no_line_number() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.line_number);

    let args = parse_low_raw(["--no-line-number"]).expect("Test parsing should succeed");
    assert_eq!(Some(false), args.line_number);

    let args = parse_low_raw(["-N"]).expect("Test parsing should succeed");
    assert_eq!(Some(false), args.line_number);

    let args = parse_low_raw(["-N", "--line-number"]).expect("Test parsing should succeed");
    assert_eq!(Some(true), args.line_number);
}

/// -x/--line-regexp
#[derive(Debug)]

/// -M/--max-columns
#[derive(Debug)]
struct MaxColumns;

impl Flag for MaxColumns {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'M')
    }
    fn name_long(&self) -> &'static str {
        "max-columns"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("NUM")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Omit lines longer than this limit."
    }
    fn doc_long(&self) -> &'static str {
        r"
When given, ripgrep will omit lines longer than this limit in bytes. Instead of
printing long lines, only the number of matches in that line is printed.
.sp
When this flag is omitted or is set to \fB0\fP, then it has no effect.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let max = convert::u64(&v.unwrap_value())?;
        args.max_columns = if max == 0 { None } else { Some(max) };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_max_columns() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.max_columns);

    let args = parse_low_raw(["--max-columns", "5"]).expect("Test parsing should succeed");
    assert_eq!(Some(5), args.max_columns);

    let args = parse_low_raw(["-M", "5"]).expect("Test parsing should succeed");
    assert_eq!(Some(5), args.max_columns);

    let args = parse_low_raw(["-M5"]).expect("Test parsing should succeed");
    assert_eq!(Some(5), args.max_columns);

    let args = parse_low_raw(["--max-columns", "5", "-M0"]).expect("Test parsing should succeed");
    assert_eq!(None, args.max_columns);
}

/// --max-columns-preview
#[derive(Debug)]

/// --max-columns-preview
#[derive(Debug)]
struct MaxColumnsPreview;

impl Flag for MaxColumnsPreview {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "max-columns-preview"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-max-columns-preview")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Show preview for lines exceeding the limit."
    }
    fn doc_long(&self) -> &'static str {
        r"
Prints a preview for lines exceeding the configured max column limit.
.sp
When the \flag{max-columns} flag is used, ripgrep will by default completely
replace any line that is too long with a message indicating that a matching
line was removed. When this flag is combined with \flag{max-columns}, a preview
of the line (corresponding to the limit size) is shown instead, where the part
of the line exceeding the limit is not shown.
.sp
If the \flag{max-columns} flag is not set, then this has no effect.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.max_columns_preview = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_max_columns_preview() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.max_columns_preview);

    let args = parse_low_raw(["--max-columns-preview"]).expect("Test parsing should succeed");
    assert_eq!(true, args.max_columns_preview);

    let args =
        parse_low_raw(["--max-columns-preview", "--no-max-columns-preview"])
            .expect("Test parsing should succeed");
    assert_eq!(false, args.max_columns_preview);
}

/// -m/--max-count
#[derive(Debug)]

/// -0/--null
#[derive(Debug)]
struct Null;

impl Flag for Null {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'0')
    }
    fn name_long(&self) -> &'static str {
        "null"
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Print a NUL byte after file paths."
    }
    fn doc_long(&self) -> &'static str {
        r"
Whenever a file path is printed, follow it with a \fBNUL\fP byte. This includes
printing file paths before matches, and when printing a list of matching files
such as with \flag{count}, \flag{files-with-matches} and \flag{files}. This
option is useful for use with \fBxargs\fP.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--null has no negation");
        args.null = true;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_null() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.null);

    let args = parse_low_raw(["--null"]).expect("Test parsing should succeed");
    assert_eq!(true, args.null);

    let args = parse_low_raw(["-0"]).expect("Test parsing should succeed");
    assert_eq!(true, args.null);
}

/// --null-data
#[derive(Debug)]

/// -o/--only-matching
#[derive(Debug)]
struct OnlyMatching;

impl Flag for OnlyMatching {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'o')
    }
    fn name_long(&self) -> &'static str {
        "only-matching"
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Print only matched parts of a line."
    }
    fn doc_long(&self) -> &'static str {
        r"
Print only the matched (non-empty) parts of a matching line, with each such
part on a separate output line.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--only-matching does not have a negation");
        args.only_matching = true;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_only_matching() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.only_matching);

    let args = parse_low_raw(["--only-matching"]).expect("Test parsing should succeed");
    assert_eq!(true, args.only_matching);

    let args = parse_low_raw(["-o"]).expect("Test parsing should succeed");
    assert_eq!(true, args.only_matching);
}

/// --path-separator
#[derive(Debug)]

/// --path-separator
#[derive(Debug)]
struct PathSeparator;

impl Flag for PathSeparator {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "path-separator"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("SEPARATOR")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Set the path separator for printing paths."
    }
    fn doc_long(&self) -> &'static str {
        r"
Set the path separator to use when printing file paths. This defaults to your
platform's path separator, which is \fB/\fP on Unix and \fB\\\fP on Windows.
This flag is intended for overriding the default when the environment demands
it (e.g., cygwin). A path separator is limited to a single byte.
.sp
Setting this flag to an empty string reverts it to its default behavior. That
is, the path separator is automatically chosen based on the environment.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let s = convert::string(v.unwrap_value())?;
        let raw = Vec::unescape_bytes(&s);
        args.path_separator = if raw.is_empty() {
            None
        } else if raw.len() == 1 {
            Some(raw[0])
        } else {
            anyhow::bail!(
                "A path separator must be exactly one byte, but \
                 the given separator is {len} bytes: {sep}\n\
                 In some shells on Windows '/' is automatically \
                 expanded. Use '//' instead.",
                len = raw.len(),
                sep = s,
            )
        };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_path_separator() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.path_separator);

    let args = parse_low_raw(["--path-separator", "/"]).expect("Test parsing should succeed");
    assert_eq!(Some(b'/'), args.path_separator);

    let args = parse_low_raw(["--path-separator", r"\"]).expect("Test parsing should succeed");
    assert_eq!(Some(b'\\'), args.path_separator);

    let args = parse_low_raw(["--path-separator", r"\x00"]).expect("Test parsing should succeed");
    assert_eq!(Some(0), args.path_separator);

    let args = parse_low_raw(["--path-separator", r"\0"]).expect("Test parsing should succeed");
    assert_eq!(Some(0), args.path_separator);

    let args = parse_low_raw(["--path-separator", "\x00"]).expect("Test parsing should succeed");
    assert_eq!(Some(0), args.path_separator);

    let args = parse_low_raw(["--path-separator", "\0"]).expect("Test parsing should succeed");
    assert_eq!(Some(0), args.path_separator);

    let args =
        parse_low_raw(["--path-separator", r"\x00", "--path-separator=/"])
            .expect("Test parsing should succeed");
    assert_eq!(Some(b'/'), args.path_separator);

    let result = parse_low_raw(["--path-separator", "foo"]);
    assert!(result.is_err(), "{result:?}");

    let result = parse_low_raw(["--path-separator", r"\\x00"]);
    assert!(result.is_err(), "{result:?}");
}

/// -P/--pcre2
#[derive(Debug)]

/// -q/--quiet
#[derive(Debug)]

/// -q/--quiet
#[derive(Debug)]
struct Quiet;

impl Flag for Quiet {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'q')
    }
    fn name_long(&self) -> &'static str {
        "quiet"
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Do not print anything to stdout."
    }
    fn doc_long(&self) -> &'static str {
        r"
Do not print anything to stdout. If a match is found in a file, then ripgrep
will stop searching. This is useful when ripgrep is used only for its exit code
(which will be an error code if no matches are found).
.sp
When \flag{files} is used, ripgrep will stop finding files after finding the
first file that does not match any ignore rules.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--quiet has no negation");
        args.quiet = true;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_quiet() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.quiet);

    let args = parse_low_raw(["--quiet"]).expect("Test parsing should succeed");
    assert_eq!(true, args.quiet);

    let args = parse_low_raw(["-q"]).expect("Test parsing should succeed");
    assert_eq!(true, args.quiet);

    // flags like -l and --json cannot override -q, regardless of order
    let args = parse_low_raw(["-q", "--json"]).expect("Test parsing should succeed");
    assert_eq!(true, args.quiet);

    let args = parse_low_raw(["-q", "--count"]).expect("Test parsing should succeed");
    assert_eq!(true, args.quiet);

    let args = parse_low_raw(["-q", "--count-matches"]).expect("Test parsing should succeed");
    assert_eq!(true, args.quiet);
}

/// --regex-size-limit
#[derive(Debug)]

/// -r/--replace
#[derive(Debug)]
struct Replace;

impl Flag for Replace {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'r')
    }
    fn name_long(&self) -> &'static str {
        "replace"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("REPLACEMENT")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Replace matches with the given text."
    }
    fn doc_long(&self) -> &'static str {
        r#"
Replaces every match with the text given when printing results. Neither this
flag nor any other ripgrep flag will modify your files.
.sp
Capture group indices (e.g., \fB$\fP\fI5\fP) and names (e.g., \fB$\fP\fIfoo\fP)
are supported in the replacement string. Capture group indices are numbered
based on the position of the opening parenthesis of the group, where the
leftmost such group is \fB$\fP\fI1\fP. The special \fB$\fP\fI0\fP group
corresponds to the entire match.
.sp
The name of a group is formed by taking the longest string of letters, numbers
and underscores (i.e. \fB[_0-9A-Za-z]\fP) after the \fB$\fP. For example,
\fB$\fP\fI1a\fP will be replaced with the group named \fI1a\fP, not the
group at index \fI1\fP. If the group's name contains characters that aren't
letters, numbers or underscores, or you want to immediately follow the group
with another string, the name should be put inside braces. For example,
\fB${\fP\fI1\fP\fB}\fP\fIa\fP will take the content of the group at index
\fI1\fP and append \fIa\fP to the end of it.
.sp
If an index or name does not refer to a valid capture group, it will be
replaced with an empty string.
.sp
In shells such as Bash and zsh, you should wrap the pattern in single quotes
instead of double quotes. Otherwise, capture group indices will be replaced by
expanded shell variables which will most likely be empty.
.sp
To write a literal \fB$\fP, use \fB$$\fP.
.sp
Note that the replacement by default replaces each match, and not the entire
line. To replace the entire line, you should match the entire line.
.sp
This flag can be used with the \flag{only-matching} flag.
"#
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.replace = Some(convert::string(v.unwrap_value())?.into());
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_replace() {
    use bstr::BString;

    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.replace);

    let args = parse_low_raw(["--replace", "foo"]).expect("Test parsing should succeed");
    assert_eq!(Some(BString::from("foo")), args.replace);

    let args = parse_low_raw(["--replace", "-foo"]).expect("Test parsing should succeed");
    assert_eq!(Some(BString::from("-foo")), args.replace);

    let args = parse_low_raw(["-r", "foo"]).expect("Test parsing should succeed");
    assert_eq!(Some(BString::from("foo")), args.replace);

    let args = parse_low_raw(["-r", "foo", "-rbar"]).expect("Test parsing should succeed");
    assert_eq!(Some(BString::from("bar")), args.replace);

    let args = parse_low_raw(["-r", "foo", "-r", ""]).expect("Test parsing should succeed");
    assert_eq!(Some(BString::from("")), args.replace);
}

/// -z/--search-zip
#[derive(Debug)]

/// --sort-files
#[derive(Debug)]
struct SortFiles;

impl Flag for SortFiles {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "sort-files"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-sort-files")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"(DEPRECATED) Sort results by file path."
    }
    fn doc_long(&self) -> &'static str {
        r"
DEPRECATED. Use \fB\-\-sort=path\fP instead.
.sp
This flag instructs ripgrep to sort search results by file path
lexicographically in ascending order. Note that this currently disables all
parallelism and runs search in a single thread.
.sp
This flag overrides \flag{sort} and \flag{sortr}.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.sort = if v.unwrap_switch() {
            Some(SortMode { reverse: false, kind: SortModeKind::Path })
        } else {
            None
        };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_sort_files() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.sort);

    let args = parse_low_raw(["--sort-files"]).expect("Test parsing should succeed");
    assert_eq!(
        Some(SortMode { reverse: false, kind: SortModeKind::Path }),
        args.sort
    );

    let args = parse_low_raw(["--sort-files", "--no-sort-files"]).expect("Test parsing should succeed");
    assert_eq!(None, args.sort);

    let args = parse_low_raw(["--sort", "created", "--sort-files"]).expect("Test parsing should succeed");
    assert_eq!(
        Some(SortMode { reverse: false, kind: SortModeKind::Path }),
        args.sort
    );

    let args = parse_low_raw(["--sort-files", "--sort", "created"]).expect("Test parsing should succeed");
    assert_eq!(
        Some(SortMode { reverse: false, kind: SortModeKind::Created }),
        args.sort
    );

    let args = parse_low_raw(["--sortr", "created", "--sort-files"]).expect("Test parsing should succeed");
    assert_eq!(
        Some(SortMode { reverse: false, kind: SortModeKind::Path }),
        args.sort
    );

    let args = parse_low_raw(["--sort-files", "--sortr", "created"]).expect("Test parsing should succeed");
    assert_eq!(
        Some(SortMode { reverse: true, kind: SortModeKind::Created }),
        args.sort
    );

    let args = parse_low_raw(["--sort=path", "--no-sort-files"]).expect("Test parsing should succeed");
    assert_eq!(None, args.sort);

    let args = parse_low_raw(["--sortr=path", "--no-sort-files"]).expect("Test parsing should succeed");
    assert_eq!(None, args.sort);
}

/// --sort
#[derive(Debug)]

/// --sort
#[derive(Debug)]
struct Sort;

impl Flag for Sort {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "sort"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("SORTBY")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Sort results in ascending order."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag enables sorting of results in ascending order. The possible values
for this flag are:
.sp
.TP 12
\fBnone\fP
(Default) Do not sort results. Fastest. Can be multi-threaded.
.TP 12
\fBpath\fP
Sort by file path. Always single-threaded. The order is determined by sorting
files in each directory entry during traversal. This means that given the files
\fBa/b\fP and \fBa+\fP, the latter will sort after the former even though
\fB+\fP would normally sort before \fB/\fP.
.TP 12
\fBmodified\fP
Sort by the last modified time on a file. Always single-threaded.
.TP 12
\fBaccessed\fP
Sort by the last accessed time on a file. Always single-threaded.
.TP 12
\fBcreated\fP
Sort by the creation time on a file. Always single-threaded.
.PP
If the chosen (manually or by-default) sorting criteria isn't available on your
system (for example, creation time is not available on ext4 file systems), then
ripgrep will attempt to detect this, print an error and exit without searching.
.sp
To sort results in reverse or descending order, use the \flag{sortr} flag.
Also, this flag overrides \flag{sortr}.
.sp
Note that sorting results currently always forces ripgrep to abandon
parallelism and run in a single thread.
"
    }
    fn doc_choices(&self) -> &'static [&'static str] {
        &["none", "path", "modified", "accessed", "created"]
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let kind = match convert::str(&v.unwrap_value())? {
            "none" => {
                args.sort = None;
                return Ok(());
            }
            "path" => SortModeKind::Path,
            "modified" => SortModeKind::LastModified,
            "accessed" => SortModeKind::LastAccessed,
            "created" => SortModeKind::Created,
            unk => anyhow::bail!("choice '{unk}' is unrecognized"),
        };
        args.sort = Some(SortMode { reverse: false, kind });
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_sort() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.sort);

    let args = parse_low_raw(["--sort", "path"]).expect("Test parsing should succeed");
    assert_eq!(
        Some(SortMode { reverse: false, kind: SortModeKind::Path }),
        args.sort
    );

    let args = parse_low_raw(["--sort", "path", "--sort=created"]).expect("Test parsing should succeed");
    assert_eq!(
        Some(SortMode { reverse: false, kind: SortModeKind::Created }),
        args.sort
    );

    let args = parse_low_raw(["--sort=none"]).expect("Test parsing should succeed");
    assert_eq!(None, args.sort);

    let args = parse_low_raw(["--sort", "path", "--sort=none"]).expect("Test parsing should succeed");
    assert_eq!(None, args.sort);
}

/// --sortr
#[derive(Debug)]

/// --sortr
#[derive(Debug)]
struct Sortr;

impl Flag for Sortr {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "sortr"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("SORTBY")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Sort results in descending order."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag enables sorting of results in descending order. The possible values
for this flag are:
.sp
.TP 12
\fBnone\fP
(Default) Do not sort results. Fastest. Can be multi-threaded.
.TP 12
\fBpath\fP
Sort by file path. Always single-threaded. The order is determined by sorting
files in each directory entry during traversal. This means that given the files
\fBa/b\fP and \fBa+\fP, the latter will sort before the former even though
\fB+\fP would normally sort after \fB/\fP when doing a reverse lexicographic
sort.
.TP 12
\fBmodified\fP
Sort by the last modified time on a file. Always single-threaded.
.TP 12
\fBaccessed\fP
Sort by the last accessed time on a file. Always single-threaded.
.TP 12
\fBcreated\fP
Sort by the creation time on a file. Always single-threaded.
.PP
If the chosen (manually or by-default) sorting criteria isn't available on your
system (for example, creation time is not available on ext4 file systems), then
ripgrep will attempt to detect this, print an error and exit without searching.
.sp
To sort results in ascending order, use the \flag{sort} flag. Also, this flag
overrides \flag{sort}.
.sp
Note that sorting results currently always forces ripgrep to abandon
parallelism and run in a single thread.
"
    }
    fn doc_choices(&self) -> &'static [&'static str] {
        &["none", "path", "modified", "accessed", "created"]
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let kind = match convert::str(&v.unwrap_value())? {
            "none" => {
                args.sort = None;
                return Ok(());
            }
            "path" => SortModeKind::Path,
            "modified" => SortModeKind::LastModified,
            "accessed" => SortModeKind::LastAccessed,
            "created" => SortModeKind::Created,
            unk => anyhow::bail!("choice '{unk}' is unrecognized"),
        };
        args.sort = Some(SortMode { reverse: true, kind });
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_sortr() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.sort);

    let args = parse_low_raw(["--sortr", "path"]).expect("Test parsing should succeed");
    assert_eq!(
        Some(SortMode { reverse: true, kind: SortModeKind::Path }),
        args.sort
    );

    let args = parse_low_raw(["--sortr", "path", "--sortr=created"]).expect("Test parsing should succeed");
    assert_eq!(
        Some(SortMode { reverse: true, kind: SortModeKind::Created }),
        args.sort
    );

    let args = parse_low_raw(["--sortr=none"]).expect("Test parsing should succeed");
    assert_eq!(None, args.sort);

    let args = parse_low_raw(["--sortr", "path", "--sortr=none"]).expect("Test parsing should succeed");
    assert_eq!(None, args.sort);

    let args = parse_low_raw(["--sort=path", "--sortr=path"]).expect("Test parsing should succeed");
    assert_eq!(
        Some(SortMode { reverse: true, kind: SortModeKind::Path }),
        args.sort
    );

    let args = parse_low_raw(["--sortr=path", "--sort=path"]).expect("Test parsing should succeed");
    assert_eq!(
        Some(SortMode { reverse: false, kind: SortModeKind::Path }),
        args.sort
    );
}

/// --stats
#[derive(Debug)]

/// --trim
#[derive(Debug)]
struct Trim;

impl Flag for Trim {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "trim"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-trim")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Trim prefix whitespace from matches."
    }
    fn doc_long(&self) -> &'static str {
        r"
When set, all ASCII whitespace at the beginning of each line printed will be
removed.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.trim = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_trim() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.trim);

    let args = parse_low_raw(["--trim"]).expect("Test parsing should succeed");
    assert_eq!(true, args.trim);

    let args = parse_low_raw(["--trim", "--no-trim"]).expect("Test parsing should succeed");
    assert_eq!(false, args.trim);
}

/// -t/--type
#[derive(Debug)]

/// --vimgrep
#[derive(Debug)]
struct Vimgrep;

impl Flag for Vimgrep {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "vimgrep"
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Print results in a vim compatible format."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag instructs ripgrep to print results with every match on its own line,
including line numbers and column numbers.
.sp
With this option, a line with more than one match will be printed in its
entirety more than once. For that reason, the total amount of output as a
result of this flag can be quadratic in the size of the input. For example,
if the pattern matches every byte in an input file, then each line will be
repeated for every byte matched. For this reason, users should only use this
flag when there is no other choice. Editor integrations should prefer some
other way of reading results from ripgrep, such as via the \flag{json} flag.
One alternative to avoiding exorbitant memory usage is to force ripgrep into
single threaded mode with the \flag{threads} flag. Note though that this will
not impact the total size of the output, just the heap memory that ripgrep will
use.
"
    }
    fn doc_choices(&self) -> &'static [&'static str] {
        &[]
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--vimgrep has no negation");
        args.vimgrep = true;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_vimgrep() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.vimgrep);

    let args = parse_low_raw(["--vimgrep"]).expect("Test parsing should succeed");
    assert_eq!(true, args.vimgrep);
}

/// --with-filename
#[derive(Debug)]

/// --with-filename
#[derive(Debug)]
struct WithFilename;

impl Flag for WithFilename {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'H')
    }
    fn name_long(&self) -> &'static str {
        "with-filename"
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Print the file path with each matching line."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag instructs ripgrep to print the file path for each matching line.
This is the default when more than one file is searched. If \flag{heading} is
enabled (the default when printing to a tty), the file path will be shown above
clusters of matches from each file; otherwise, the file name will be shown as a
prefix for each matched line.
.sp
This flag overrides \flag{no-filename}.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--with-filename has no defined negation");
        args.with_filename = Some(true);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_with_filename() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.with_filename);

    let args = parse_low_raw(["--with-filename"]).expect("Test parsing should succeed");
    assert_eq!(Some(true), args.with_filename);

    let args = parse_low_raw(["-H"]).expect("Test parsing should succeed");
    assert_eq!(Some(true), args.with_filename);
}

/// --no-filename
#[derive(Debug)]

/// --no-filename
#[derive(Debug)]
struct WithFilenameNo;

impl Flag for WithFilenameNo {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'I')
    }
    fn name_long(&self) -> &'static str {
        "no-filename"
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Never print the path with each matching line."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag instructs ripgrep to never print the file path with each matching
line. This is the default when ripgrep is explicitly instructed to search one
file or stdin.
.sp
This flag overrides \flag{with-filename}.
"
    }
    fn doc_choices(&self) -> &'static [&'static str] {
        &[]
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--no-filename has no defined negation");
        args.with_filename = Some(false);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_with_filename_no() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.with_filename);

    let args = parse_low_raw(["--no-filename"]).expect("Test parsing should succeed");
    assert_eq!(Some(false), args.with_filename);

    let args = parse_low_raw(["-I"]).expect("Test parsing should succeed");
    assert_eq!(Some(false), args.with_filename);

    let args = parse_low_raw(["-I", "-H"]).expect("Test parsing should succeed");
    assert_eq!(Some(true), args.with_filename);

    let args = parse_low_raw(["-H", "-I"]).expect("Test parsing should succeed");
    assert_eq!(Some(false), args.with_filename);
}
