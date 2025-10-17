/*!
Defines all of the flags available in ripgrep.

Each flag corresponds to a unit struct with a corresponding implementation
of `Flag`. Note that each implementation of `Flag` might actually have many
possible manifestations of the same "flag." That is, each implementation of
`Flag` can have the following flags available to an end user of ripgrep:

* The long flag name.
* An optional short flag name.
* An optional negated long flag name.
* An arbitrarily long list of aliases.

The idea is that even though there are multiple flags that a user can type,
one implementation of `Flag` corresponds to a single _logical_ flag inside of
ripgrep. For example, `-E`, `--encoding` and `--no-encoding` all manipulate the
same encoding state in ripgrep.
*/

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

use super::CompletionType;

/// A list of all flags in ripgrep via implementations of `Flag`.
///
/// The order of these flags matter. It determines the order of the flags in
/// the generated documentation (`-h`, `--help` and the man page) within each
/// category. (This is why the deprecated flags are last.)

// Module declarations
mod filter;
mod input;
mod logging;
mod other_behaviors;
mod output;
mod output_modes;
mod search;

// Re-export all flags
pub(super) use filter::*;
pub(super) use input::*;
pub(super) use logging::*;
pub(super) use other_behaviors::*;
pub(super) use output::*;
pub(super) use output_modes::*;
pub(super) use search::*;

pub(super) const FLAGS: &[&dyn Flag] = &[
    // -e/--regexp and -f/--file should come before anything else in the
    // same category.
    &Regexp,
    &File,
    &AfterContext,
    &BeforeContext,
    &Binary,
    &BlockBuffered,
    &ByteOffset,
    &CaseSensitive,
    &Color,
    &Colors,
    &Column,
    &Context,
    &ContextSeparator,
    &Count,
    &CountMatches,
    &Crlf,
    &Debug,
    &DfaSizeLimit,
    &Encoding,
    &Engine,
    &FieldContextSeparator,
    &FieldMatchSeparator,
    &Files,
    &FixedStrings,
    &Follow,
    &Generate,
    &Glob,
    &GlobCaseInsensitive,
    &Heading,
    &Help,
    &Hidden,
    &HostnameBin,
    &HyperlinkFormat,
    &IGlob,
    &IgnoreCase,
    &IgnoreFile,
    &IgnoreFileCaseInsensitive,
    &IncludeZero,
    &InvertMatch,
    &JSON,
    &LineBuffered,
    &LineNumber,
    &LineNumberNo,
    &LineRegexp,
    &MaxColumns,
    &MaxColumnsPreview,
    &MaxCount,
    &MaxDepth,
    &MaxFilesize,
    &Mmap,
    &Multiline,
    &MultilineDotall,
    &NoConfig,
    &NoIgnore,
    &NoIgnoreDot,
    &NoIgnoreExclude,
    &NoIgnoreFiles,
    &NoIgnoreGlobal,
    &NoIgnoreMessages,
    &NoIgnoreParent,
    &NoIgnoreVcs,
    &NoMessages,
    &NoRequireGit,
    &NoUnicode,
    &Null,
    &NullData,
    &OneFileSystem,
    &OnlyMatching,
    &PathSeparator,
    &Passthru,
    &PCRE2,
    &PCRE2Version,
    &Pre,
    &PreGlob,
    &Pretty,
    &Quiet,
    &RegexSizeLimit,
    &Replace,
    &SearchZip,
    &SmartCase,
    &Sort,
    &Sortr,
    &Stats,
    &StopOnNonmatch,
    &Text,
    &Threads,
    &Trace,
    &Trim,
    &Type,
    &TypeNot,
    &TypeAdd,
    &TypeClear,
    &TypeList,
    &Unrestricted,
    &Version,
    &Vimgrep,
    &WithFilename,
    &WithFilenameNo,
    &WordRegexp,
    // DEPRECATED (make them show up last in their respective categories)
    &AutoHybridRegex,
    &NoPcre2Unicode,
    &SortFiles,
];

mod convert {
    use std::ffi::{OsStr, OsString};

    use anyhow::Context;

    pub(super) fn str(v: &OsStr) -> anyhow::Result<&str> {
        let Some(s) = v.to_str() else {
            anyhow::bail!("value is not valid UTF-8")
        };
        Ok(s)
    }

    pub(super) fn string(v: OsString) -> anyhow::Result<String> {
        let Ok(s) = v.into_string() else {
            anyhow::bail!("value is not valid UTF-8")
        };
        Ok(s)
    }

    pub(super) fn usize(v: &OsStr) -> anyhow::Result<usize> {
        str(v)?.parse().context("value is not a valid number")
    }

    pub(super) fn u64(v: &OsStr) -> anyhow::Result<u64> {
        str(v)?.parse().context("value is not a valid number")
    }

    pub(super) fn human_readable_u64(v: &OsStr) -> anyhow::Result<u64> {
        grep::cli::parse_human_readable_size(str(v)?).context("invalid size")
    }

    pub(super) fn human_readable_usize(v: &OsStr) -> anyhow::Result<usize> {
        let size = human_readable_u64(v)?;
        let Ok(size) = usize::try_from(size) else {
            anyhow::bail!("size is too big")
        };
        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn available_shorts() {
        let mut total = vec![false; 128];
        for byte in 0..=0x7F {
            match byte {
                b'.' | b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' => {
                    total[usize::from(byte)] = true
                }
                _ => continue,
            }
        }

        let mut taken = vec![false; 128];
        for flag in FLAGS.iter() {
            let Some(short) = flag.name_short() else { continue };
            taken[usize::from(short)] = true;
        }

        for byte in 0..=0x7F {
            if total[usize::from(byte)] && !taken[usize::from(byte)] {
                eprintln!("{}", char::from(byte));
            }
        }
    }

    #[test]
    fn shorts_all_ascii_alphanumeric() {
        for flag in FLAGS.iter() {
            let Some(byte) = flag.name_short() else { continue };
            let long = flag.name_long();
            assert!(
                byte.is_ascii_alphanumeric() || byte == b'.',
                "\\x{byte:0X} is not a valid short flag for {long}",
            )
        }
    }

    #[test]
    fn longs_all_ascii_alphanumeric() {
        for flag in FLAGS.iter() {
            let long = flag.name_long();
            let count = long.chars().count();
            assert!(count >= 2, "flag '{long}' is less than 2 characters");
            assert!(
                long.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'),
                "flag '{long}' does not match ^[-0-9A-Za-z]+$",
            );
            for alias in flag.aliases() {
                let count = alias.chars().count();
                assert!(
                    count >= 2,
                    "flag '{long}' has alias '{alias}' that is \
                     less than 2 characters",
                );
                assert!(
                    alias
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-'),
                    "flag '{long}' has alias '{alias}' that does not \
                     match ^[-0-9A-Za-z]+$",
                );
            }
            let Some(negated) = flag.name_negated() else { continue };
            let count = negated.chars().count();
            assert!(
                count >= 2,
                "flag '{long}' has negation '{negated}' that is \
                 less than 2 characters",
            );
            assert!(
                negated.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'),
                "flag '{long}' has negation '{negated}' that \
                 does not match ^[-0-9A-Za-z]+$",
            );
        }
    }

    #[test]
    fn shorts_no_duplicates() {
        let mut taken = vec![false; 128];
        for flag in FLAGS.iter() {
            let Some(short) = flag.name_short() else { continue };
            let long = flag.name_long();
            assert!(
                !taken[usize::from(short)],
                "flag {long} has duplicate short flag {}",
                char::from(short)
            );
            taken[usize::from(short)] = true;
        }
    }

    #[test]
    fn longs_no_duplicates() {
        use std::collections::BTreeSet;

        let mut taken = BTreeSet::new();
        for flag in FLAGS.iter() {
            let long = flag.name_long();
            assert!(taken.insert(long), "flag {long} has a duplicate name");
            for alias in flag.aliases() {
                assert!(
                    taken.insert(alias),
                    "flag {long} has an alias {alias} that is duplicative"
                );
            }
            let Some(negated) = flag.name_negated() else { continue };
            assert!(
                taken.insert(negated),
                "negated flag {negated} has a duplicate name"
            );
        }
    }

    #[test]
    fn non_switches_have_variable_names() {
        for flag in FLAGS.iter() {
            if flag.is_switch() {
                continue;
            }
            let long = flag.name_long();
            assert!(
                flag.doc_variable().is_some(),
                "flag '{long}' should have a variable name"
            );
        }
    }

    #[test]
    fn switches_have_no_choices() {
        for flag in FLAGS.iter() {
            if !flag.is_switch() {
                continue;
            }
            let long = flag.name_long();
            let choices = flag.doc_choices();
            assert!(
                choices.is_empty(),
                "switch flag '{long}' \
                 should not have any choices but has some: {choices:?}",
            );
        }
    }

    #[test]
    fn choices_ascii_alphanumeric() {
        for flag in FLAGS.iter() {
            let long = flag.name_long();
            for choice in flag.doc_choices() {
                assert!(
                    choice.chars().all(|c| c.is_ascii_alphanumeric()
                        || c == '-'
                        || c == ':'
                        || c == '+'),
                    "choice '{choice}' for flag '{long}' does not match \
                     ^[-+:0-9A-Za-z]+$",
                )
            }
        }
    }
}
