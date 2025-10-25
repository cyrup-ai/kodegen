/*!
Defines a builder for haystacks.

A "haystack" represents something we want to search. It encapsulates the logic
for whether a haystack ought to be searched or not, separate from the standard
ignore rules and other filtering logic.

Effectively, a haystack wraps a directory entry and adds some light application
level logic around it.
*/

use std::path::Path;

/// A builder for constructing things to search over.
#[derive(Clone, Debug)]
pub(crate) struct HaystackBuilder {
    strip_dot_prefix: bool,
}

#[allow(dead_code)]
impl HaystackBuilder {
    /// Return a new haystack builder with a default configuration.
    pub(crate) fn new() -> HaystackBuilder {
        HaystackBuilder {
            strip_dot_prefix: false,
        }
    }

    /// Create a new haystack using this builder's configuration.
    ///
    /// If a directory entry could not be created or should otherwise not be
    /// searched, then this returns `None` after emitting any relevant log
    /// messages.
    pub(crate) fn build(&self, dent: ignore::DirEntry) -> Option<Haystack> {
        let hay = Haystack {
            dent,
            strip_dot_prefix: self.strip_dot_prefix,
        };
        // Silently skip DirEntry errors (non-fatal warnings)
        // If this entry was explicitly provided by an end user, then we always
        // want to search it.
        if hay.is_explicit() {
            return Some(hay);
        }
        // At this point, we only want to search something if it's explicitly a
        // file. This omits symlinks. (If ripgrep was configured to follow
        // symlinks, then they have already been followed by the directory
        // traversal.)
        if hay.is_file() {
            return Some(hay);
        }
        // We got nothing. Emit a debug message, but only if this isn't a
        // directory. Otherwise, emitting messages for directories is just
        // noisy.
        if !hay.is_dir() {
            log::debug!(
                "ignoring {}: failed to pass haystack filter: \
                 file type: {:?}, metadata: {:?}",
                hay.dent.path().display(),
                hay.dent.file_type(),
                hay.dent.metadata()
            );
        }
        None
    }

    /// When enabled, if the haystack's file path starts with `./` then it is
    /// stripped.
    ///
    /// This is useful when implicitly searching the current working directory.
    pub(crate) fn strip_dot_prefix(&mut self, yes: bool) -> &mut HaystackBuilder {
        self.strip_dot_prefix = yes;
        self
    }
}

/// A haystack is a thing we want to search.
///
/// Generally, a haystack is either a file or stdin.
#[derive(Clone, Debug)]
pub(crate) struct Haystack {
    dent: ignore::DirEntry,
    strip_dot_prefix: bool,
}

impl Haystack {
    /// Return the file path corresponding to this haystack.
    ///
    /// If this haystack corresponds to stdin, then a special `<stdin>` path
    /// is returned instead.
    pub(crate) fn path(&self) -> &Path {
        if self.strip_dot_prefix && self.dent.path().starts_with("./") {
            // We just checked starts_with("./"), so strip_prefix("./") should succeed
            // If it somehow doesn't, fall back to the original path to avoid panicking
            match self.dent.path().strip_prefix("./") {
                Ok(stripped) => stripped,
                Err(e) => {
                    log::error!("BUG: strip_prefix failed after starts_with check: {e}");
                    self.dent.path()
                }
            }
        } else {
            self.dent.path()
        }
    }

    /// Returns true if and only if this entry corresponds to stdin.
    pub(crate) fn is_stdin(&self) -> bool {
        self.dent.is_stdin()
    }

    /// Returns true if and only if this entry corresponds to a haystack to
    /// search that was explicitly supplied by an end user.
    ///
    /// Generally, this corresponds to either stdin or an explicit file path
    /// argument. e.g., in `rg foo some-file ./some-dir/`, `some-file` is
    /// an explicit haystack, but, e.g., `./some-dir/some-other-file` is not.
    ///
    /// However, note that ripgrep does not see through shell globbing. e.g.,
    /// in `rg foo ./some-dir/*`, `./some-dir/some-other-file` will be treated
    /// as an explicit haystack.
    pub(crate) fn is_explicit(&self) -> bool {
        // stdin is obvious. When an entry has a depth of 0, that means it
        // was explicitly provided to our directory iterator, which means it
        // was in turn explicitly provided by the end user. The !is_dir check
        // means that we want to search files even if their symlinks, again,
        // because they were explicitly provided. (And we never want to try
        // to search a directory.)
        self.is_stdin() || (self.dent.depth() == 0 && !self.is_dir())
    }

    /// Returns true if and only if this haystack points to a directory after
    /// following symbolic links.
    fn is_dir(&self) -> bool {
        let ft = match self.dent.file_type() {
            None => return false,
            Some(ft) => ft,
        };
        if ft.is_dir() {
            return true;
        }
        // If this is a symlink, then we want to follow it to determine
        // whether it's a directory or not.
        self.dent.path_is_symlink() && self.dent.path().is_dir()
    }

    /// Returns true if and only if this haystack points to a file.
    fn is_file(&self) -> bool {
        self.dent.file_type().is_some_and(|ft| ft.is_file())
    }
}
