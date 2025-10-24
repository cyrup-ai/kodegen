//! Filter category flags.

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

/// --binary
#[derive(Debug)]
struct Binary;

impl Flag for Binary {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "binary"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-binary")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        "Search binary files."
    }
    fn doc_long(&self) -> &'static str {
        r"
Enabling this flag will cause ripgrep to search binary files. By default,
ripgrep attempts to automatically skip binary files in order to improve the
relevance of results and make the search faster.
.sp
Binary files are heuristically detected based on whether they contain a
\fBNUL\fP byte or not. By default (without this flag set), once a \fBNUL\fP
byte is seen, ripgrep will stop searching the file. Usually, \fBNUL\fP bytes
occur in the beginning of most binary files. If a \fBNUL\fP byte occurs after
a match, then ripgrep will not print the match, stop searching that file, and
emit a warning that some matches are being suppressed.
.sp
In contrast, when this flag is provided, ripgrep will continue searching a
file even if a \fBNUL\fP byte is found. In particular, if a \fBNUL\fP byte is
found then ripgrep will continue searching until either a match is found or
the end of the file is reached, whichever comes sooner. If a match is found,
then ripgrep will stop and print a warning saying that the search stopped
prematurely.
.sp
If you want ripgrep to search a file without any special \fBNUL\fP byte
handling at all (and potentially print binary data to stdout), then you should
use the \flag{text} flag.
.sp
The \flag{binary} flag is a flag for controlling ripgrep's automatic filtering
mechanism. As such, it does not need to be used when searching a file
explicitly or when searching stdin. That is, it is only applicable when
recursively searching a directory.
.sp
When the \flag{unrestricted} flag is provided for a third time, then this flag
is automatically enabled.
.sp
This flag overrides the \flag{text} flag.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.binary = if v.unwrap_switch() {
            BinaryMode::SearchAndSuppress
        } else {
            BinaryMode::Auto
        };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_binary() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(BinaryMode::Auto, args.binary);

    let args = parse_low_raw(["--binary"]).expect("Test parsing should succeed");
    assert_eq!(BinaryMode::SearchAndSuppress, args.binary);

    let args = parse_low_raw(["--binary", "--no-binary"]).expect("Test parsing should succeed");
    assert_eq!(BinaryMode::Auto, args.binary);

    let args = parse_low_raw(["--no-binary", "--binary"]).expect("Test parsing should succeed");
    assert_eq!(BinaryMode::SearchAndSuppress, args.binary);

    let args = parse_low_raw(["--binary", "-a"]).expect("Test parsing should succeed");
    assert_eq!(BinaryMode::AsText, args.binary);

    let args = parse_low_raw(["-a", "--binary"]).expect("Test parsing should succeed");
    assert_eq!(BinaryMode::SearchAndSuppress, args.binary);

    let args = parse_low_raw(["-a", "--no-binary"]).expect("Test parsing should succeed");
    assert_eq!(BinaryMode::Auto, args.binary);
}

/// --block-buffered
#[derive(Debug)]

/// -L/--follow
#[derive(Debug)]
struct Follow;

impl Flag for Follow {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'L')
    }
    fn name_long(&self) -> &'static str {
        "follow"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-follow")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Follow symbolic links."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag instructs ripgrep to follow symbolic links while traversing
directories. This behavior is disabled by default. Note that ripgrep will
check for symbolic link loops and report errors if it finds one. ripgrep will
also report errors for broken links. To suppress error messages, use the
\flag{no-messages} flag.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.follow = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_follow() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.follow);

    let args = parse_low_raw(["--follow"]).expect("Test parsing should succeed");
    assert_eq!(true, args.follow);

    let args = parse_low_raw(["-L"]).expect("Test parsing should succeed");
    assert_eq!(true, args.follow);

    let args = parse_low_raw(["-L", "--no-follow"]).expect("Test parsing should succeed");
    assert_eq!(false, args.follow);

    let args = parse_low_raw(["--no-follow", "-L"]).expect("Test parsing should succeed");
    assert_eq!(true, args.follow);
}

/// --generate
#[derive(Debug)]

/// -g/--glob
#[derive(Debug)]
struct Glob;

impl Flag for Glob {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'g')
    }
    fn name_long(&self) -> &'static str {
        "glob"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("GLOB")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Include or exclude file paths."
    }
    fn doc_long(&self) -> &'static str {
        r#"
Include or exclude files and directories for searching that match the given
glob. This always overrides any other ignore logic. Multiple glob flags may
be used. Globbing rules match \fB.gitignore\fP globs. Precede a glob with a
\fB!\fP to exclude it. If multiple globs match a file or directory, the glob
given later in the command line takes precedence.
.sp
As an extension, globs support specifying alternatives:
.BI "\-g '" ab{c,d}* '
is equivalent to
.BI "\-g " "abc " "\-g " abd.
Empty alternatives like
.BI "\-g '" ab{,c} '
are not currently supported. Note that this syntax extension is also currently
enabled in \fBgitignore\fP files, even though this syntax isn't supported by
git itself. ripgrep may disable this syntax extension in gitignore files, but
it will always remain available via the \flag{glob} flag.
.sp
When this flag is set, every file and directory is applied to it to test for
a match. For example, if you only want to search in a particular directory
\fIfoo\fP, then
.BI "\-g " foo
is incorrect because \fIfoo/bar\fP does not match
the glob \fIfoo\fP. Instead, you should use
.BI "\-g '" foo/** '.
"#
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let glob = convert::string(v.unwrap_value())?;
        args.globs.push(glob);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_glob() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(Vec::<String>::new(), args.globs);

    let args = parse_low_raw(["--glob", "foo"]).expect("Test parsing should succeed");
    assert_eq!(vec!["foo".to_string()], args.globs);

    let args = parse_low_raw(["--glob=foo"]).expect("Test parsing should succeed");
    assert_eq!(vec!["foo".to_string()], args.globs);

    let args = parse_low_raw(["-g", "foo"]).expect("Test parsing should succeed");
    assert_eq!(vec!["foo".to_string()], args.globs);

    let args = parse_low_raw(["-gfoo"]).expect("Test parsing should succeed");
    assert_eq!(vec!["foo".to_string()], args.globs);

    let args = parse_low_raw(["--glob", "-foo"]).expect("Test parsing should succeed");
    assert_eq!(vec!["-foo".to_string()], args.globs);

    let args = parse_low_raw(["--glob=-foo"]).expect("Test parsing should succeed");
    assert_eq!(vec!["-foo".to_string()], args.globs);

    let args = parse_low_raw(["-g", "-foo"]).expect("Test parsing should succeed");
    assert_eq!(vec!["-foo".to_string()], args.globs);

    let args = parse_low_raw(["-g-foo"]).expect("Test parsing should succeed");
    assert_eq!(vec!["-foo".to_string()], args.globs);
}

/// --glob-case-insensitive
#[derive(Debug)]

/// --glob-case-insensitive
#[derive(Debug)]
struct GlobCaseInsensitive;

impl Flag for GlobCaseInsensitive {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "glob-case-insensitive"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-glob-case-insensitive")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Process all glob patterns case insensitively."
    }
    fn doc_long(&self) -> &'static str {
        r"
Process all glob patterns given with the \flag{glob} flag case insensitively.
This effectively treats \flag{glob} as \flag{iglob}.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.glob_case_insensitive = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_glob_case_insensitive() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.glob_case_insensitive);

    let args = parse_low_raw(["--glob-case-insensitive"]).expect("Test parsing should succeed");
    assert_eq!(true, args.glob_case_insensitive);

    let args = parse_low_raw([
        "--glob-case-insensitive",
        "--no-glob-case-insensitive",
    ])
    .expect("Test parsing should succeed");
    assert_eq!(false, args.glob_case_insensitive);

    let args = parse_low_raw([
        "--no-glob-case-insensitive",
        "--glob-case-insensitive",
    ])
    .expect("Test parsing should succeed");
    assert_eq!(true, args.glob_case_insensitive);
}

/// --heading
#[derive(Debug)]

/// -./--hidden
#[derive(Debug)]
struct Hidden;

impl Flag for Hidden {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'.')
    }
    fn name_long(&self) -> &'static str {
        "hidden"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-hidden")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Search hidden files and directories."
    }
    fn doc_long(&self) -> &'static str {
        r#"
Search hidden files and directories. By default, hidden files and directories
are skipped. Note that if a hidden file or a directory is whitelisted in
an ignore file, then it will be searched even if this flag isn't provided.
Similarly if a hidden file or directory is given explicitly as an argument to
ripgrep.
.sp
A file or directory is considered hidden if its base name starts with a dot
character (\fB.\fP). On operating systems which support a "hidden" file
attribute, like Windows, files with this attribute are also considered hidden.
.sp
Note that \flag{hidden} will include files and folders like \fB.git\fP
regardless of \flag{no-ignore-vcs}. To exclude such paths when using
\flag{hidden}, you must explicitly ignore them using another flag or ignore
file.
"#
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.hidden = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_hidden() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.hidden);

    let args = parse_low_raw(["--hidden"]).expect("Test parsing should succeed");
    assert_eq!(true, args.hidden);

    let args = parse_low_raw(["-."]).expect("Test parsing should succeed");
    assert_eq!(true, args.hidden);

    let args = parse_low_raw(["-.", "--no-hidden"]).expect("Test parsing should succeed");
    assert_eq!(false, args.hidden);

    let args = parse_low_raw(["--no-hidden", "-."]).expect("Test parsing should succeed");
    assert_eq!(true, args.hidden);
}

/// --hostname-bin
#[derive(Debug)]

/// --iglob
#[derive(Debug)]
struct IGlob;

impl Flag for IGlob {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "iglob"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("GLOB")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Include/exclude paths case insensitively."
    }
    fn doc_long(&self) -> &'static str {
        r"
Include or exclude files and directories for searching that match the given
glob. This always overrides any other ignore logic. Multiple glob flags may
be used. Globbing rules match \fB.gitignore\fP globs. Precede a glob with a
\fB!\fP to exclude it. If multiple globs match a file or directory, the glob
given later in the command line takes precedence. Globs used via this flag are
matched case insensitively.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let glob = convert::string(v.unwrap_value())?;
        args.iglobs.push(glob);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_iglob() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(Vec::<String>::new(), args.iglobs);

    let args = parse_low_raw(["--iglob", "foo"]).expect("Test parsing should succeed");
    assert_eq!(vec!["foo".to_string()], args.iglobs);

    let args = parse_low_raw(["--iglob=foo"]).expect("Test parsing should succeed");
    assert_eq!(vec!["foo".to_string()], args.iglobs);

    let args = parse_low_raw(["--iglob", "-foo"]).expect("Test parsing should succeed");
    assert_eq!(vec!["-foo".to_string()], args.iglobs);

    let args = parse_low_raw(["--iglob=-foo"]).expect("Test parsing should succeed");
    assert_eq!(vec!["-foo".to_string()], args.iglobs);
}

/// -i/--ignore-case
#[derive(Debug)]

/// --ignore-file
#[derive(Debug)]
struct IgnoreFile;

impl Flag for IgnoreFile {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "ignore-file"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("PATH")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Specify additional ignore files."
    }
    fn doc_long(&self) -> &'static str {
        r"
Specifies a path to one or more \fBgitignore\fP formatted rules files.
These patterns are applied after the patterns found in \fB.gitignore\fP,
\fB.rgignore\fP and \fB.ignore\fP are applied and are matched relative to the
current working directory. That is, files specified via this flag have lower
precedence than files automatically found in the directory tree. Multiple
additional ignore files can be specified by using this flag repeatedly. When
specifying multiple ignore files, earlier files have lower precedence than
later files.
.sp
If you are looking for a way to include or exclude files and directories
directly on the command line, then use \flag{glob} instead.
"
    }
    fn completion_type(&self) -> CompletionType {
        CompletionType::Filename
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let path = PathBuf::from(v.unwrap_value());
        args.ignore_file.push(path);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_ignore_file() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(Vec::<PathBuf>::new(), args.ignore_file);

    let args = parse_low_raw(["--ignore-file", "foo"]).expect("Test parsing should succeed");
    assert_eq!(vec![PathBuf::from("foo")], args.ignore_file);

    let args = parse_low_raw(["--ignore-file", "foo", "--ignore-file", "bar"])
        .expect("Test parsing should succeed");
    assert_eq!(
        vec![PathBuf::from("foo"), PathBuf::from("bar")],
        args.ignore_file
    );
}

/// --ignore-file-case-insensitive
#[derive(Debug)]

/// --ignore-file-case-insensitive
#[derive(Debug)]
struct IgnoreFileCaseInsensitive;

impl Flag for IgnoreFileCaseInsensitive {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "ignore-file-case-insensitive"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-ignore-file-case-insensitive")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Process ignore files case insensitively."
    }
    fn doc_long(&self) -> &'static str {
        r"
Process ignore files (\fB.gitignore\fP, \fB.ignore\fP, etc.) case
insensitively. Note that this comes with a performance penalty and is most
useful on case insensitive file systems (such as Windows).
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.ignore_file_case_insensitive = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_ignore_file_case_insensitive() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.ignore_file_case_insensitive);

    let args = parse_low_raw(["--ignore-file-case-insensitive"]).expect("Test parsing should succeed");
    assert_eq!(true, args.ignore_file_case_insensitive);

    let args = parse_low_raw([
        "--ignore-file-case-insensitive",
        "--no-ignore-file-case-insensitive",
    ])
    .expect("Test parsing should succeed");
    assert_eq!(false, args.ignore_file_case_insensitive);

    let args = parse_low_raw([
        "--no-ignore-file-case-insensitive",
        "--ignore-file-case-insensitive",
    ])
    .expect("Test parsing should succeed");
    assert_eq!(true, args.ignore_file_case_insensitive);
}

/// --include-zero
#[derive(Debug)]

/// --max-depth
#[derive(Debug)]
struct MaxDepth;

impl Flag for MaxDepth {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'd')
    }
    fn name_long(&self) -> &'static str {
        "max-depth"
    }
    fn aliases(&self) -> &'static [&'static str] {
        &["maxdepth"]
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("NUM")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Descend at most NUM directories."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag limits the depth of directory traversal to \fINUM\fP levels beyond
the paths given. A value of \fB0\fP only searches the explicitly given paths
themselves.
.sp
For example, \fBrg --max-depth 0 \fP\fIdir/\fP is a no-op because \fIdir/\fP
will not be descended into. \fBrg --max-depth 1 \fP\fIdir/\fP will search only
the direct children of \fIdir\fP.
.sp
An alternative spelling for this flag is \fB\-\-maxdepth\fP.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.max_depth = Some(convert::usize(&v.unwrap_value())?);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_max_depth() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.max_depth);

    let args = parse_low_raw(["--max-depth", "5"]).expect("Test parsing should succeed");
    assert_eq!(Some(5), args.max_depth);

    let args = parse_low_raw(["-d", "5"]).expect("Test parsing should succeed");
    assert_eq!(Some(5), args.max_depth);

    let args = parse_low_raw(["--max-depth", "5", "--max-depth=10"]).expect("Test parsing should succeed");
    assert_eq!(Some(10), args.max_depth);

    let args = parse_low_raw(["--max-depth", "0"]).expect("Test parsing should succeed");
    assert_eq!(Some(0), args.max_depth);

    let args = parse_low_raw(["--maxdepth", "5"]).expect("Test parsing should succeed");
    assert_eq!(Some(5), args.max_depth);
}

/// --max-filesize
#[derive(Debug)]

/// --max-filesize
#[derive(Debug)]
struct MaxFilesize;

impl Flag for MaxFilesize {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "max-filesize"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("NUM+SUFFIX?")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Ignore files larger than NUM in size."
    }
    fn doc_long(&self) -> &'static str {
        r"
Ignore files larger than \fINUM\fP in size. This does not apply to directories.
.sp
The input format accepts suffixes of \fBK\fP, \fBM\fP or \fBG\fP which
correspond to kilobytes, megabytes and gigabytes, respectively. If no suffix is
provided the input is treated as bytes.
.sp
Examples: \fB\-\-max-filesize 50K\fP or \fB\-\-max\-filesize 80M\fP.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let v = v.unwrap_value();
        args.max_filesize = Some(convert::human_readable_u64(&v)?);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_max_filesize() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.max_filesize);

    let args = parse_low_raw(["--max-filesize", "1024"]).expect("Test parsing should succeed");
    assert_eq!(Some(1024), args.max_filesize);

    let args = parse_low_raw(["--max-filesize", "1K"]).expect("Test parsing should succeed");
    assert_eq!(Some(1024), args.max_filesize);

    let args =
        parse_low_raw(["--max-filesize", "1K", "--max-filesize=1M"]).expect("Test parsing should succeed");
    assert_eq!(Some(1024 * 1024), args.max_filesize);
}

/// --mmap
#[derive(Debug)]

/// --no-ignore
#[derive(Debug)]
struct NoIgnore;

impl Flag for NoIgnore {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "no-ignore"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("ignore")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Don't use ignore files."
    }
    fn doc_long(&self) -> &'static str {
        r"
When set, ignore files such as \fB.gitignore\fP, \fB.ignore\fP and
\fB.rgignore\fP will not be respected. This implies \flag{no-ignore-dot},
\flag{no-ignore-exclude}, \flag{no-ignore-global}, \flag{no-ignore-parent} and
\flag{no-ignore-vcs}.
.sp
This does not imply \flag{no-ignore-files}, since \flag{ignore-file} is
specified explicitly as a command line argument.
.sp
When given only once, the \flag{unrestricted} flag is identical in
behavior to this flag and can be considered an alias. However, subsequent
\flag{unrestricted} flags have additional effects.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let yes = v.unwrap_switch();
        args.no_ignore_dot = yes;
        args.no_ignore_exclude = yes;
        args.no_ignore_global = yes;
        args.no_ignore_parent = yes;
        args.no_ignore_vcs = yes;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_no_ignore() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.no_ignore_dot);
    assert_eq!(false, args.no_ignore_exclude);
    assert_eq!(false, args.no_ignore_global);
    assert_eq!(false, args.no_ignore_parent);
    assert_eq!(false, args.no_ignore_vcs);

    let args = parse_low_raw(["--no-ignore"]).expect("Test parsing should succeed");
    assert_eq!(true, args.no_ignore_dot);
    assert_eq!(true, args.no_ignore_exclude);
    assert_eq!(true, args.no_ignore_global);
    assert_eq!(true, args.no_ignore_parent);
    assert_eq!(true, args.no_ignore_vcs);

    let args = parse_low_raw(["--no-ignore", "--ignore"]).expect("Test parsing should succeed");
    assert_eq!(false, args.no_ignore_dot);
    assert_eq!(false, args.no_ignore_exclude);
    assert_eq!(false, args.no_ignore_global);
    assert_eq!(false, args.no_ignore_parent);
    assert_eq!(false, args.no_ignore_vcs);
}

/// --no-ignore-dot
#[derive(Debug)]

/// --no-ignore-dot
#[derive(Debug)]
struct NoIgnoreDot;

impl Flag for NoIgnoreDot {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "no-ignore-dot"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("ignore-dot")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Don't use .ignore or .rgignore files."
    }
    fn doc_long(&self) -> &'static str {
        r"
Don't respect filter rules from \fB.ignore\fP or \fB.rgignore\fP files.
.sp
This does not impact whether ripgrep will ignore files and directories whose
names begin with a dot. For that, see the \flag{hidden} flag. This flag also
does not impact whether filter rules from \fB.gitignore\fP files are respected.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.no_ignore_dot = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_no_ignore_dot() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.no_ignore_dot);

    let args = parse_low_raw(["--no-ignore-dot"]).expect("Test parsing should succeed");
    assert_eq!(true, args.no_ignore_dot);

    let args = parse_low_raw(["--no-ignore-dot", "--ignore-dot"]).expect("Test parsing should succeed");
    assert_eq!(false, args.no_ignore_dot);
}

/// --no-ignore-exclude
#[derive(Debug)]

/// --no-ignore-exclude
#[derive(Debug)]
struct NoIgnoreExclude;

impl Flag for NoIgnoreExclude {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "no-ignore-exclude"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("ignore-exclude")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Don't use local exclusion files."
    }
    fn doc_long(&self) -> &'static str {
        r"
Don't respect filter rules from files that are manually configured for the repository.
For example, this includes \fBgit\fP's \fB.git/info/exclude\fP.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.no_ignore_exclude = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_no_ignore_exclude() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.no_ignore_exclude);

    let args = parse_low_raw(["--no-ignore-exclude"]).expect("Test parsing should succeed");
    assert_eq!(true, args.no_ignore_exclude);

    let args =
        parse_low_raw(["--no-ignore-exclude", "--ignore-exclude"]).expect("Test parsing should succeed");
    assert_eq!(false, args.no_ignore_exclude);
}

/// --no-ignore-files
#[derive(Debug)]

/// --no-ignore-files
#[derive(Debug)]
struct NoIgnoreFiles;

impl Flag for NoIgnoreFiles {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "no-ignore-files"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("ignore-files")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Don't use --ignore-file arguments."
    }
    fn doc_long(&self) -> &'static str {
        r"
When set, any \flag{ignore-file} flags, even ones that come after this flag,
are ignored.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.no_ignore_files = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_no_ignore_files() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.no_ignore_files);

    let args = parse_low_raw(["--no-ignore-files"]).expect("Test parsing should succeed");
    assert_eq!(true, args.no_ignore_files);

    let args = parse_low_raw(["--no-ignore-files", "--ignore-files"]).expect("Test parsing should succeed");
    assert_eq!(false, args.no_ignore_files);
}

/// --no-ignore-global
#[derive(Debug)]

/// --no-ignore-global
#[derive(Debug)]
struct NoIgnoreGlobal;

impl Flag for NoIgnoreGlobal {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "no-ignore-global"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("ignore-global")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Don't use global ignore files."
    }
    fn doc_long(&self) -> &'static str {
        r#"
Don't respect filter rules from ignore files that come from "global" sources
such as \fBgit\fP's \fBcore.excludesFile\fP configuration option (which
defaults to \fB$HOME/.config/git/ignore\fP).
"#
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.no_ignore_global = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_no_ignore_global() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.no_ignore_global);

    let args = parse_low_raw(["--no-ignore-global"]).expect("Test parsing should succeed");
    assert_eq!(true, args.no_ignore_global);

    let args =
        parse_low_raw(["--no-ignore-global", "--ignore-global"]).expect("Test parsing should succeed");
    assert_eq!(false, args.no_ignore_global);
}

/// --no-ignore-messages
#[derive(Debug)]

/// --no-ignore-parent
#[derive(Debug)]
struct NoIgnoreParent;

impl Flag for NoIgnoreParent {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "no-ignore-parent"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("ignore-parent")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Don't use ignore files in parent directories."
    }
    fn doc_long(&self) -> &'static str {
        r"
When this flag is set, filter rules from ignore files found in parent
directories are not respected. By default, ripgrep will ascend the parent
directories of the current working directory to look for any applicable ignore
files that should be applied. In some cases this may not be desirable.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.no_ignore_parent = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_no_ignore_parent() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.no_ignore_parent);

    let args = parse_low_raw(["--no-ignore-parent"]).expect("Test parsing should succeed");
    assert_eq!(true, args.no_ignore_parent);

    let args =
        parse_low_raw(["--no-ignore-parent", "--ignore-parent"]).expect("Test parsing should succeed");
    assert_eq!(false, args.no_ignore_parent);
}

/// --no-ignore-vcs
#[derive(Debug)]

/// --no-ignore-vcs
#[derive(Debug)]
struct NoIgnoreVcs;

impl Flag for NoIgnoreVcs {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "no-ignore-vcs"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("ignore-vcs")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Don't use ignore files from source control."
    }
    fn doc_long(&self) -> &'static str {
        r"
When given, filter rules from source control ignore files (e.g.,
\fB.gitignore\fP) are not respected. By default, ripgrep respects \fBgit\fP's
ignore rules for automatic filtering. In some cases, it may not be desirable
to respect the source control's ignore rules and instead only respect rules in
\fB.ignore\fP or \fB.rgignore\fP.
.sp
Note that this flag does not directly affect the filtering of source control
files or folders that start with a dot (\fB.\fP), like \fB.git\fP. These are
affected by \flag{hidden} and its related flags instead.
.sp
This flag implies \flag{no-ignore-parent} for source control ignore files as
well.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.no_ignore_vcs = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_no_ignore_vcs() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.no_ignore_vcs);

    let args = parse_low_raw(["--no-ignore-vcs"]).expect("Test parsing should succeed");
    assert_eq!(true, args.no_ignore_vcs);

    let args = parse_low_raw(["--no-ignore-vcs", "--ignore-vcs"]).expect("Test parsing should succeed");
    assert_eq!(false, args.no_ignore_vcs);
}

/// --no-messages
#[derive(Debug)]

/// --no-require-git
#[derive(Debug)]
struct NoRequireGit;

impl Flag for NoRequireGit {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "no-require-git"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("require-git")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Use .gitignore outside of git repositories."
    }
    fn doc_long(&self) -> &'static str {
        r"
When this flag is given, source control ignore files such as \fB.gitignore\fP
are respected even if no \fBgit\fP repository is present.
.sp
By default, ripgrep will only respect filter rules from source control ignore
files when ripgrep detects that the search is executed inside a source control
repository. For example, when a \fB.git\fP directory is observed.
.sp
This flag relaxes the default restriction. For example, it might be useful when
the contents of a \fBgit\fP repository are stored or copied somewhere, but
where the repository state is absent.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.no_require_git = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_no_require_git() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.no_require_git);

    let args = parse_low_raw(["--no-require-git"]).expect("Test parsing should succeed");
    assert_eq!(true, args.no_require_git);

    let args = parse_low_raw(["--no-require-git", "--require-git"]).expect("Test parsing should succeed");
    assert_eq!(false, args.no_require_git);
}

/// --no-unicode
#[derive(Debug)]

/// --one-file-system
#[derive(Debug)]
struct OneFileSystem;

impl Flag for OneFileSystem {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "one-file-system"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-one-file-system")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Skip directories on other file systems."
    }
    fn doc_long(&self) -> &'static str {
        r"
When enabled, ripgrep will not cross file system boundaries relative to where
the search started from.
.sp
Note that this applies to each path argument given to ripgrep. For example, in
the command
.sp
.EX
    rg \-\-one\-file\-system /foo/bar /quux/baz
.EE
.sp
ripgrep will search both \fI/foo/bar\fP and \fI/quux/baz\fP even if they are
on different file systems, but will not cross a file system boundary when
traversing each path's directory tree.
.sp
This is similar to \fBfind\fP's \fB\-xdev\fP or \fB\-mount\fP flag.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.one_file_system = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_one_file_system() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.one_file_system);

    let args = parse_low_raw(["--one-file-system"]).expect("Test parsing should succeed");
    assert_eq!(true, args.one_file_system);

    let args =
        parse_low_raw(["--one-file-system", "--no-one-file-system"]).expect("Test parsing should succeed");
    assert_eq!(false, args.one_file_system);
}

/// -o/--only-matching
#[derive(Debug)]

/// -t/--type
#[derive(Debug)]
struct Type;

impl Flag for Type {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_short(&self) -> Option<u8> {
        Some(b't')
    }
    fn name_long(&self) -> &'static str {
        "type"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("TYPE")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Only search files matching TYPE."
    }
    fn doc_long(&self) -> &'static str {
        r#"
This flag limits ripgrep to searching files matching \fITYPE\fP. Multiple
\flag{type} flags may be provided.
.sp
This flag supports the special value \fBall\fP, which will behave as if
\flag{type} was provided for every file type supported by ripgrep (including
any custom file types). The end result is that \fB\-\-type=all\fP causes
ripgrep to search in "whitelist" mode, where it will only search files it
recognizes via its type definitions.
.sp
Note that this flag has lower precedence than both the \flag{glob} flag and
any rules found in ignore files.
.sp
To see the list of available file types, use the \flag{type-list} flag.
"#
    }
    fn completion_type(&self) -> CompletionType {
        CompletionType::Filetype
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.type_changes.push(TypeChange::Select {
            name: convert::string(v.unwrap_value())?,
        });
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_type() {
    let select = |name: &str| TypeChange::Select { name: name.to_string() };

    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(Vec::<TypeChange>::new(), args.type_changes);

    let args = parse_low_raw(["--type", "rust"]).expect("Test parsing should succeed");
    assert_eq!(vec![select("rust")], args.type_changes);

    let args = parse_low_raw(["-t", "rust"]).expect("Test parsing should succeed");
    assert_eq!(vec![select("rust")], args.type_changes);

    let args = parse_low_raw(["-trust"]).expect("Test parsing should succeed");
    assert_eq!(vec![select("rust")], args.type_changes);

    let args = parse_low_raw(["-trust", "-tpython"]).expect("Test parsing should succeed");
    assert_eq!(vec![select("rust"), select("python")], args.type_changes);

    let args = parse_low_raw(["-tabcdefxyz"]).expect("Test parsing should succeed");
    assert_eq!(vec![select("abcdefxyz")], args.type_changes);
}

/// --type-not
#[derive(Debug)]

/// --type-not
#[derive(Debug)]
struct TypeNot;

impl Flag for TypeNot {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'T')
    }
    fn name_long(&self) -> &'static str {
        "type-not"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("TYPE")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Do not search files matching TYPE."
    }
    fn doc_long(&self) -> &'static str {
        r#"
Do not search files matching \fITYPE\fP. Multiple \flag{type-not} flags may be
provided. Use the \flag{type-list} flag to list all available types.
.sp
This flag supports the special value \fBall\fP, which will behave
as if \flag{type-not} was provided for every file type supported by
ripgrep (including any custom file types). The end result is that
\fB\-\-type\-not=all\fP causes ripgrep to search in "blacklist" mode, where it
will only search files that are unrecognized by its type definitions.
.sp
To see the list of available file types, use the \flag{type-list} flag.
"#
    }
    fn completion_type(&self) -> CompletionType {
        CompletionType::Filetype
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.type_changes.push(TypeChange::Negate {
            name: convert::string(v.unwrap_value())?,
        });
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_type_not() {
    let select = |name: &str| TypeChange::Select { name: name.to_string() };
    let negate = |name: &str| TypeChange::Negate { name: name.to_string() };

    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(Vec::<TypeChange>::new(), args.type_changes);

    let args = parse_low_raw(["--type-not", "rust"]).expect("Test parsing should succeed");
    assert_eq!(vec![negate("rust")], args.type_changes);

    let args = parse_low_raw(["-T", "rust"]).expect("Test parsing should succeed");
    assert_eq!(vec![negate("rust")], args.type_changes);

    let args = parse_low_raw(["-Trust"]).expect("Test parsing should succeed");
    assert_eq!(vec![negate("rust")], args.type_changes);

    let args = parse_low_raw(["-Trust", "-Tpython"]).expect("Test parsing should succeed");
    assert_eq!(vec![negate("rust"), negate("python")], args.type_changes);

    let args = parse_low_raw(["-Tabcdefxyz"]).expect("Test parsing should succeed");
    assert_eq!(vec![negate("abcdefxyz")], args.type_changes);

    let args = parse_low_raw(["-Trust", "-ttoml", "-Tjson"]).expect("Test parsing should succeed");
    assert_eq!(
        vec![negate("rust"), select("toml"), negate("json")],
        args.type_changes
    );
}

/// --type-list
#[derive(Debug)]

/// -u/--unrestricted
#[derive(Debug)]
struct Unrestricted;

impl Flag for Unrestricted {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'u')
    }
    fn name_long(&self) -> &'static str {
        "unrestricted"
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r#"Reduce the level of "smart" filtering."#
    }
    fn doc_long(&self) -> &'static str {
        r#"
This flag reduces the level of "smart" filtering. Repeated uses (up to 3) reduces
the filtering even more. When repeated three times, ripgrep will search every
file in a directory tree.
.sp
A single \flag{unrestricted} flag is equivalent to \flag{no-ignore}. Two
\flag{unrestricted} flags is equivalent to \flag{no-ignore} \flag{hidden}.
Three \flag{unrestricted} flags is equivalent to \flag{no-ignore} \flag{hidden}
\flag{binary}.
.sp
The only filtering ripgrep still does when \fB-uuu\fP is given is to skip
symbolic links and to avoid printing matches from binary files. Symbolic links
can be followed via the \flag{follow} flag, and binary files can be treated as
text files via the \flag{text} flag.
"#
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--unrestricted has no negation");
        args.unrestricted = args.unrestricted.saturating_add(1);
        anyhow::ensure!(
            args.unrestricted <= 3,
            "flag can only be repeated up to 3 times"
        );
        if args.unrestricted == 1 {
            NoIgnore.update(FlagValue::Switch(true), args)?;
        } else if args.unrestricted == 2 {
            Hidden.update(FlagValue::Switch(true), args)?;
        } else {
            assert_eq!(args.unrestricted, 3);
            Binary.update(FlagValue::Switch(true), args)?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_unrestricted() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.no_ignore_vcs);
    assert_eq!(false, args.hidden);
    assert_eq!(BinaryMode::Auto, args.binary);

    let args = parse_low_raw(["--unrestricted"]).expect("Test parsing should succeed");
    assert_eq!(true, args.no_ignore_vcs);
    assert_eq!(false, args.hidden);
    assert_eq!(BinaryMode::Auto, args.binary);

    let args = parse_low_raw(["--unrestricted", "-u"]).expect("Test parsing should succeed");
    assert_eq!(true, args.no_ignore_vcs);
    assert_eq!(true, args.hidden);
    assert_eq!(BinaryMode::Auto, args.binary);

    let args = parse_low_raw(["-uuu"]).expect("Test parsing should succeed");
    assert_eq!(true, args.no_ignore_vcs);
    assert_eq!(true, args.hidden);
    assert_eq!(BinaryMode::SearchAndSuppress, args.binary);

    let result = parse_low_raw(["-uuuu"]);
    assert!(result.is_err(), "{result:?}");
}
