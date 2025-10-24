//! OtherBehaviors category flags.

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
struct Files;

impl Flag for Files {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "files"
    }
    fn doc_category(&self) -> Category {
        Category::OtherBehaviors
    }
    fn doc_short(&self) -> &'static str {
        r"Print each file that would be searched."
    }
    fn doc_long(&self) -> &'static str {
        r"
Print each file that would be searched without actually performing the search.
This is useful to determine whether a particular file is being searched or not.
.sp
This overrides \flag{type-list}.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch());
        args.mode.update(Mode::Files);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_files() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(Mode::Search(SearchMode::Standard), args.mode);

    let args = parse_low_raw(["--files"]).expect("Test parsing should succeed");
    assert_eq!(Mode::Files, args.mode);
}

/// -g/--glob
#[derive(Debug)]

/// --no-config
#[derive(Debug)]
struct NoConfig;

impl Flag for NoConfig {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "no-config"
    }
    fn doc_category(&self) -> Category {
        Category::OtherBehaviors
    }
    fn doc_short(&self) -> &'static str {
        r"Never read configuration files."
    }
    fn doc_long(&self) -> &'static str {
        r"
When set, ripgrep will never read configuration files. When this flag is
present, ripgrep will not respect the \fBRIPGREP_CONFIG_PATH\fP environment
variable.
.sp
If ripgrep ever grows a feature to automatically read configuration files in
pre-defined locations, then this flag will also disable that behavior as well.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--no-config has no negation");
        args.no_config = true;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_no_config() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.no_config);

    let args = parse_low_raw(["--no-config"]).expect("Test parsing should succeed");
    assert_eq!(true, args.no_config);
}

/// --no-ignore
#[derive(Debug)]



/// --pre
#[derive(Debug)]



/// -u/--unrestricted
#[derive(Debug)]

/// --version
#[derive(Debug)]
struct Version;

impl Flag for Version {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'V')
    }
    fn name_long(&self) -> &'static str {
        "version"
    }
    fn doc_category(&self) -> Category {
        Category::OtherBehaviors
    }
    fn doc_short(&self) -> &'static str {
        r"Print ripgrep's version."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag prints ripgrep's version. This also may print other relevant
information, such as the presence of target specific optimizations and the
\fBgit\fP revision that this build of ripgrep was compiled from.
"
    }

    fn update(&self, v: FlagValue, _: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--version has no negation");
        // Since this flag has different semantics for -V and --version and the
        // Flag trait doesn't support encoding this sort of thing, we handle it
        // as a special case in the parser.
        Ok(())
    }
}


