//! Input category flags.

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

/// --files
#[derive(Debug)]

/// --pre
#[derive(Debug)]
struct Pre;

impl Flag for Pre {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "pre"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-pre")
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("COMMAND")
    }
    fn doc_category(&self) -> Category {
        Category::Input
    }
    fn doc_short(&self) -> &'static str {
        r"Search output of COMMAND for each PATH."
    }
    fn doc_long(&self) -> &'static str {
        r#"
For each input \fIPATH\fP, this flag causes ripgrep to search the standard
output of \fICOMMAND\fP \fIPATH\fP instead of the contents of \fIPATH\fP.
This option expects the \fICOMMAND\fP program to either be a path or to be
available in your \fBPATH\fP. Either an empty string \fICOMMAND\fP or the
\fB\-\-no\-pre\fP flag will disable this behavior.
.sp
.TP 12
\fBWARNING\fP
When this flag is set, ripgrep will unconditionally spawn a process for every
file that is searched. Therefore, this can incur an unnecessarily large
performance penalty if you don't otherwise need the flexibility offered by this
flag. One possible mitigation to this is to use the \flag{pre-glob} flag to
limit which files a preprocessor is run with.
.PP
A preprocessor is not run when ripgrep is searching stdin.
.sp
When searching over sets of files that may require one of several
preprocessors, \fICOMMAND\fP should be a wrapper program which first classifies
\fIPATH\fP based on magic numbers/content or based on the \fIPATH\fP name and
then dispatches to an appropriate preprocessor. Each \fICOMMAND\fP also has its
standard input connected to \fIPATH\fP for convenience.
.sp
For example, a shell script for \fICOMMAND\fP might look like:
.sp
.EX
    case "$1" in
    *.pdf)
        exec pdftotext "$1" -
        ;;
    *)
        case $(file "$1") in
        *Zstandard*)
            exec pzstd -cdq
            ;;
        *)
            exec cat
            ;;
        esac
        ;;
    esac
.EE
.sp
The above script uses \fBpdftotext\fP to convert a PDF file to plain text. For
all other files, the script uses the \fBfile\fP utility to sniff the type of
the file based on its contents. If it is a compressed file in the Zstandard
format, then \fBpzstd\fP is used to decompress the contents to stdout.
.sp
This overrides the \flag{search-zip} flag.
"#
    }
    fn completion_type(&self) -> CompletionType {
        CompletionType::Executable
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let path = match v {
            FlagValue::Value(v) => PathBuf::from(v),
            FlagValue::Switch(yes) => {
                assert!(!yes, "there is no affirmative switch for --pre");
                args.pre = None;
                return Ok(());
            }
        };
        args.pre = if path.as_os_str().is_empty() { None } else { Some(path) };
        if args.pre.is_some() {
            args.search_zip = false;
        }
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_pre() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.pre);

    let args = parse_low_raw(["--pre", "foo/bar"]).expect("Test parsing should succeed");
    assert_eq!(Some(PathBuf::from("foo/bar")), args.pre);

    let args = parse_low_raw(["--pre", ""]).expect("Test parsing should succeed");
    assert_eq!(None, args.pre);

    let args = parse_low_raw(["--pre", "foo/bar", "--pre", ""]).expect("Test parsing should succeed");
    assert_eq!(None, args.pre);

    let args = parse_low_raw(["--pre", "foo/bar", "--pre="]).expect("Test parsing should succeed");
    assert_eq!(None, args.pre);

    let args = parse_low_raw(["--pre", "foo/bar", "--no-pre"]).expect("Test parsing should succeed");
    assert_eq!(None, args.pre);
}

/// --pre-glob
#[derive(Debug)]

/// --pre-glob
#[derive(Debug)]
struct PreGlob;

impl Flag for PreGlob {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "pre-glob"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("GLOB")
    }
    fn doc_category(&self) -> Category {
        Category::Input
    }
    fn doc_short(&self) -> &'static str {
        r"Include or exclude files from a preprocessor."
    }
    fn doc_long(&self) -> &'static str {
        r#"
This flag works in conjunction with the \flag{pre} flag. Namely, when one or
more \flag{pre-glob} flags are given, then only files that match the given set
of globs will be handed to the command specified by the \flag{pre} flag. Any
non-matching files will be searched without using the preprocessor command.
.sp
This flag is useful when searching many files with the \flag{pre} flag.
Namely, it provides the ability to avoid process overhead for files that
don't need preprocessing. For example, given the following shell script,
\fIpre-pdftotext\fP:
.sp
.EX
    #!/bin/sh
    pdftotext "$1" -
.EE
.sp
then it is possible to use \fB\-\-pre\fP \fIpre-pdftotext\fP
\fB\-\-pre\-glob\fP '\fI*.pdf\fP' to make it so ripgrep only executes
the \fIpre-pdftotext\fP command on files with a \fI.pdf\fP extension.
.sp
Multiple \flag{pre-glob} flags may be used. Globbing rules match
\fBgitignore\fP globs. Precede a glob with a \fB!\fP to exclude it.
.sp
This flag has no effect if the \flag{pre} flag is not used.
"#
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let glob = convert::string(v.unwrap_value())?;
        args.pre_glob.push(glob);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_pre_glob() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(Vec::<String>::new(), args.pre_glob);

    let args = parse_low_raw(["--pre-glob", "*.pdf"]).expect("Test parsing should succeed");
    assert_eq!(vec!["*.pdf".to_string()], args.pre_glob);

    let args =
        parse_low_raw(["--pre-glob", "*.pdf", "--pre-glob=foo"]).expect("Test parsing should succeed");
    assert_eq!(vec!["*.pdf".to_string(), "foo".to_string()], args.pre_glob);
}

/// -p/--pretty
#[derive(Debug)]

/// -e/--regexp
#[derive(Debug)]
struct Regexp;

impl Flag for Regexp {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'e')
    }
    fn name_long(&self) -> &'static str {
        "regexp"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("PATTERN")
    }
    fn doc_category(&self) -> Category {
        Category::Input
    }
    fn doc_short(&self) -> &'static str {
        r"A pattern to search for."
    }
    fn doc_long(&self) -> &'static str {
        r"
A pattern to search for. This option can be provided multiple times, where
all patterns given are searched, in addition to any patterns provided by
\flag{file}. Lines matching at least one of the provided patterns are printed.
This flag can also be used when searching for patterns that start with a dash.
.sp
For example, to search for the literal \fB\-foo\fP:
.sp
.EX
    rg \-e \-foo
.EE
.sp
You can also use the special \fB\-\-\fP delimiter to indicate that no more
flags will be provided. Namely, the following is equivalent to the above:
.sp
.EX
    rg \-\- \-foo
.EE
.sp
When \flag{file} or \flag{regexp} is used, then ripgrep treats all positional
arguments as files or directories to search.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let regexp = convert::string(v.unwrap_value())?;
        args.patterns.push(PatternSource::Regexp(regexp));
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_regexp() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(Vec::<PatternSource>::new(), args.patterns);

    let args = parse_low_raw(["--regexp", "foo"]).expect("Test parsing should succeed");
    assert_eq!(vec![PatternSource::Regexp("foo".to_string())], args.patterns);

    let args = parse_low_raw(["--regexp=foo"]).expect("Test parsing should succeed");
    assert_eq!(vec![PatternSource::Regexp("foo".to_string())], args.patterns);

    let args = parse_low_raw(["-e", "foo"]).expect("Test parsing should succeed");
    assert_eq!(vec![PatternSource::Regexp("foo".to_string())], args.patterns);

    let args = parse_low_raw(["-efoo"]).expect("Test parsing should succeed");
    assert_eq!(vec![PatternSource::Regexp("foo".to_string())], args.patterns);

    let args = parse_low_raw(["--regexp", "-foo"]).expect("Test parsing should succeed");
    assert_eq!(vec![PatternSource::Regexp("-foo".to_string())], args.patterns);

    let args = parse_low_raw(["--regexp=-foo"]).expect("Test parsing should succeed");
    assert_eq!(vec![PatternSource::Regexp("-foo".to_string())], args.patterns);

    let args = parse_low_raw(["-e", "-foo"]).expect("Test parsing should succeed");
    assert_eq!(vec![PatternSource::Regexp("-foo".to_string())], args.patterns);

    let args = parse_low_raw(["-e-foo"]).expect("Test parsing should succeed");
    assert_eq!(vec![PatternSource::Regexp("-foo".to_string())], args.patterns);

    let args = parse_low_raw(["--regexp=foo", "--regexp", "bar"]).expect("Test parsing should succeed");
    assert_eq!(
        vec![
            PatternSource::Regexp("foo".to_string()),
            PatternSource::Regexp("bar".to_string())
        ],
        args.patterns
    );

    // While we support invalid UTF-8 arguments in general, patterns must be
    // valid UTF-8.
    #[cfg(unix)]
    {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

        let bytes = &[b'A', 0xFF, b'Z'][..];
        let result = parse_low_raw([
            OsStr::from_bytes(b"-e"),
            OsStr::from_bytes(bytes),
        ]);
        assert!(result.is_err(), "{result:?}");
    }
}

/// -r/--replace
#[derive(Debug)]

/// -z/--search-zip
#[derive(Debug)]
struct SearchZip;

impl Flag for SearchZip {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'z')
    }
    fn name_long(&self) -> &'static str {
        "search-zip"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-search-zip")
    }
    fn doc_category(&self) -> Category {
        Category::Input
    }
    fn doc_short(&self) -> &'static str {
        r"Search in compressed files."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag instructs ripgrep to search in compressed files. Currently gzip,
bzip2, xz, LZ4, LZMA, Brotli and Zstd files are supported. This option expects
the decompression binaries (such as \fBgzip\fP) to be available in your
\fBPATH\fP. If the required binaries are not found, then ripgrep will not
emit an error messages by default. Use the \flag{debug} flag to see more
information.
.sp
Note that this flag does not make ripgrep search archive formats as directory
trees. It only makes ripgrep detect compressed files and then decompress them
before searching their contents as it would any other file.
.sp
This overrides the \flag{pre} flag.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.search_zip = if v.unwrap_switch() {
            args.pre = None;
            true
        } else {
            false
        };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_search_zip() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.search_zip);

    let args = parse_low_raw(["--search-zip"]).expect("Test parsing should succeed");
    assert_eq!(true, args.search_zip);

    let args = parse_low_raw(["-z"]).expect("Test parsing should succeed");
    assert_eq!(true, args.search_zip);

    let args = parse_low_raw(["-z", "--no-search-zip"]).expect("Test parsing should succeed");
    assert_eq!(false, args.search_zip);

    let args = parse_low_raw(["--pre=foo", "--no-search-zip"]).expect("Test parsing should succeed");
    assert_eq!(Some(PathBuf::from("foo")), args.pre);
    assert_eq!(false, args.search_zip);

    let args = parse_low_raw(["--pre=foo", "--search-zip"]).expect("Test parsing should succeed");
    assert_eq!(None, args.pre);
    assert_eq!(true, args.search_zip);

    let args = parse_low_raw(["--pre=foo", "-z", "--no-search-zip"]).expect("Test parsing should succeed");
    assert_eq!(None, args.pre);
    assert_eq!(false, args.search_zip);
}
