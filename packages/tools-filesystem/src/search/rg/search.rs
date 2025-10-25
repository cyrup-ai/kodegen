/*!
Defines a very high level "search worker" abstraction.

A search worker manages the high level interaction points between the matcher
(i.e., which regex engine is used), the searcher (i.e., how data is actually
read and matched using the regex engine) and the printer. For example, the
search worker is where things like preprocessors or decompression happens.
*/

use std::{io, path::Path};

use grep::matcher::Matcher;
use grep_pcre2;

/// The configuration for the search worker.
///
/// Among a few other things, the configuration primarily controls the way we
/// show search results to users at a very high level.
#[derive(Clone, Debug)]
struct Config {
    preprocessor: Option<std::path::PathBuf>,
    preprocessor_globs: ignore::overrides::Override,
    search_zip: bool,
    binary_implicit: grep::searcher::BinaryDetection,
    binary_explicit: grep::searcher::BinaryDetection,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            preprocessor: None,
            preprocessor_globs: ignore::overrides::Override::empty(),
            search_zip: false,
            binary_implicit: grep::searcher::BinaryDetection::none(),
            binary_explicit: grep::searcher::BinaryDetection::none(),
        }
    }
}

/// A builder for configuring and constructing a search worker.
#[derive(Clone, Debug)]
pub(crate) struct SearchWorkerBuilder {
    config: Config,
    command_builder: grep::cli::CommandReaderBuilder,
}

impl Default for SearchWorkerBuilder {
    fn default() -> SearchWorkerBuilder {
        SearchWorkerBuilder::new()
    }
}

impl SearchWorkerBuilder {
    /// Create a new builder for configuring and constructing a search worker.
    pub(crate) fn new() -> SearchWorkerBuilder {
        let mut command_builder = grep::cli::CommandReaderBuilder::new();
        command_builder.async_stderr(true);

        SearchWorkerBuilder {
            config: Config::default(),
            command_builder,
        }
    }

    /// Create a new search worker using the given searcher, matcher and
    /// printer.
    pub(crate) fn build<W: io::Write>(
        &self,
        matcher: PatternMatcher,
        searcher: grep::searcher::Searcher,
        printer: Printer<W>,
    ) -> SearchWorker<W> {
        let config = self.config.clone();
        let command_builder = self.command_builder.clone();
        let decomp_builder = config.search_zip.then(|| {
            let mut decomp_builder = grep::cli::DecompressionReaderBuilder::new();
            decomp_builder.async_stderr(true);
            decomp_builder
        });
        SearchWorker {
            config,
            command_builder,
            decomp_builder,
            matcher,
            searcher,
            printer,
        }
    }

    /// Set the path to a preprocessor command.
    ///
    /// When this is set, instead of searching files directly, the given
    /// command will be run with the file path as the first argument, and the
    /// output of that command will be searched instead.
    pub(crate) fn preprocessor(
        &mut self,
        cmd: Option<std::path::PathBuf>,
    ) -> anyhow::Result<&mut SearchWorkerBuilder> {
        if let Some(ref prog) = cmd {
            let bin = grep::cli::resolve_binary(prog)?;
            self.config.preprocessor = Some(bin);
        } else {
            self.config.preprocessor = None;
        }
        Ok(self)
    }

    /// Set the globs for determining which files should be run through the
    /// preprocessor. By default, with no globs and a preprocessor specified,
    /// every file is run through the preprocessor.
    pub(crate) fn preprocessor_globs(
        &mut self,
        globs: ignore::overrides::Override,
    ) -> &mut SearchWorkerBuilder {
        self.config.preprocessor_globs = globs;
        self
    }

    /// Enable the decompression and searching of common compressed files.
    ///
    /// When enabled, if a particular file path is recognized as a compressed
    /// file, then it is decompressed before searching.
    ///
    /// Note that if a preprocessor command is set, then it overrides this
    /// setting.
    pub(crate) fn search_zip(&mut self, yes: bool) -> &mut SearchWorkerBuilder {
        self.config.search_zip = yes;
        self
    }

    /// Set the binary detection that should be used when searching files
    /// found via a recursive directory search.
    ///
    /// Generally, this binary detection may be
    /// `grep::searcher::BinaryDetection::quit` if we want to skip binary files
    /// completely.
    ///
    /// By default, no binary detection is performed.
    pub(crate) fn binary_detection_implicit(
        &mut self,
        detection: grep::searcher::BinaryDetection,
    ) -> &mut SearchWorkerBuilder {
        self.config.binary_implicit = detection;
        self
    }

    /// Set the binary detection that should be used when searching files
    /// explicitly supplied by an end user.
    ///
    /// Generally, this binary detection should NOT be
    /// `grep::searcher::BinaryDetection::quit`, since we never want to
    /// automatically filter files supplied by the end user.
    ///
    /// By default, no binary detection is performed.
    pub(crate) fn binary_detection_explicit(
        &mut self,
        detection: grep::searcher::BinaryDetection,
    ) -> &mut SearchWorkerBuilder {
        self.config.binary_explicit = detection;
        self
    }
}

/// The pattern matcher used by a search worker.
#[derive(Clone, Debug)]
pub enum PatternMatcher {
    RustRegex(grep::regex::RegexMatcher),
    PCRE2(grep_pcre2::RegexMatcher),
}

/// The printer used by a search worker.
///
/// The `W` type parameter refers to the type of the underlying writer.
/// For MCP, we only use JSON output.
#[derive(Clone, Debug)]
pub(crate) enum Printer<W> {
    /// A JSON printer, which emits results in the JSON Lines format.
    Json(grep::printer::JSON<W>),
}

impl<W: io::Write> Printer<W> {
    /// Return a mutable reference to the underlying printer's writer.
    pub(crate) fn get_mut(&mut self) -> &mut W {
        match *self {
            Printer::Json(ref mut p) => p.get_mut(),
        }
    }
}

/// A worker for executing searches.
///
/// It is intended for a single worker to execute many searches, and is
/// generally intended to be used from a single thread. When searching using
/// multiple threads, it is better to create a new worker for each thread.
#[derive(Clone, Debug)]
pub(crate) struct SearchWorker<W> {
    config: Config,
    command_builder: grep::cli::CommandReaderBuilder,
    /// This is `None` when `search_zip` is not enabled, since in this case it
    /// can never be used. We do this because building the reader can sometimes
    /// do non-trivial work (like resolving the paths of decompression binaries
    /// on Windows).
    decomp_builder: Option<grep::cli::DecompressionReaderBuilder>,
    matcher: PatternMatcher,
    searcher: grep::searcher::Searcher,
    printer: Printer<W>,
}

impl<W: io::Write> SearchWorker<W> {
    /// Execute a search over the given haystack.
    pub(crate) fn search(&mut self, haystack: &super::haystack::Haystack) -> io::Result<()> {
        let bin = if haystack.is_explicit() {
            self.config.binary_explicit.clone()
        } else {
            self.config.binary_implicit.clone()
        };
        let path = haystack.path();
        log::trace!("{}: binary detection: {:?}", path.display(), bin);

        self.searcher.set_binary_detection(bin);
        if haystack.is_stdin() {
            self.search_reader(path, &mut io::stdin().lock())
        } else if self.should_preprocess(path) {
            self.search_preprocessor(path)
        } else if self.should_decompress(path) {
            self.search_decompress(path)
        } else {
            self.search_path(path)
        }
    }

    /// Return a mutable reference to the underlying printer.
    pub(crate) fn printer(&mut self) -> &mut Printer<W> {
        &mut self.printer
    }

    /// Returns true if and only if the given file path should be
    /// decompressed before searching.
    fn should_decompress(&self, path: &Path) -> bool {
        self.decomp_builder
            .as_ref()
            .is_some_and(|decomp_builder| decomp_builder.get_matcher().has_command(path))
    }

    /// Returns true if and only if the given file path should be run through
    /// the preprocessor.
    fn should_preprocess(&self, path: &Path) -> bool {
        if self.config.preprocessor.is_none() {
            return false;
        }
        if self.config.preprocessor_globs.is_empty() {
            return true;
        }
        !self
            .config
            .preprocessor_globs
            .matched(path, false)
            .is_ignore()
    }

    /// Search the given file path by first asking the preprocessor for the
    /// data to search instead of opening the path directly.
    ///
    /// PRECONDITION: This function must only be called when preprocessor is Some,
    /// as guaranteed by `should_preprocess()` check.
    fn search_preprocessor(&mut self, path: &Path) -> io::Result<()> {
        use std::{fs::File, process::Stdio};

        // should_preprocess() ensures preprocessor is Some before calling this
        // If it's None, this indicates a programming error in the contract
        let bin = self.config.preprocessor.as_ref()
            .ok_or_else(|| io::Error::other(
                "BUG: search_preprocessor called with None preprocessor - should_preprocess() contract violated"
            ))?;
        let mut cmd = std::process::Command::new(bin);
        cmd.arg(path).stdin(Stdio::from(File::open(path)?));

        let mut rdr = self.command_builder.build(&mut cmd).map_err(|err| {
            io::Error::other(format!(
                "preprocessor command could not start: '{cmd:?}': {err}",
            ))
        })?;
        let result = self.search_reader(path, &mut rdr).map_err(|err| {
            io::Error::other(format!("preprocessor command failed: '{cmd:?}': {err}"))
        });
        let close_result = rdr.close();
        result?;
        close_result?;
        Ok(())
    }

    /// Attempt to decompress the data at the given file path and search the
    /// result. If the given file path isn't recognized as a compressed file,
    /// then search it without doing any decompression.
    fn search_decompress(&mut self, path: &Path) -> io::Result<()> {
        let Some(ref decomp_builder) = self.decomp_builder else {
            return self.search_path(path);
        };
        let mut rdr = decomp_builder.build(path)?;
        let result = self.search_reader(path, &mut rdr);
        let close_result = rdr.close();
        result?;
        close_result?;
        Ok(())
    }

    /// Search the contents of the given file path.
    fn search_path(&mut self, path: &Path) -> io::Result<()> {
        use self::PatternMatcher::{PCRE2, RustRegex};

        let (searcher, printer) = (&mut self.searcher, &mut self.printer);
        match self.matcher {
            RustRegex(ref m) => search_path(m, searcher, printer, path),
            PCRE2(ref m) => search_path(m, searcher, printer, path),
        }
    }

    /// Executes a search on the given reader, which may or may not correspond
    /// directly to the contents of the given file path. Instead, the reader
    /// may actually cause something else to be searched (for example, when
    /// a preprocessor is set or when decompression is enabled). In those
    /// cases, the file path is used for visual purposes only.
    ///
    /// Generally speaking, this method should only be used when there is no
    /// other choice. Searching via `search_path` provides more opportunities
    /// for optimizations (such as memory maps).
    fn search_reader<R: io::Read>(&mut self, path: &Path, rdr: &mut R) -> io::Result<()> {
        use self::PatternMatcher::{PCRE2, RustRegex};

        let (searcher, printer) = (&mut self.searcher, &mut self.printer);
        match self.matcher {
            RustRegex(ref m) => search_reader(m, searcher, printer, path, rdr),
            PCRE2(ref m) => search_reader(m, searcher, printer, path, rdr),
        }
    }
}

/// Search the contents of the given file path using the given matcher,
/// searcher and printer.
fn search_path<M: Matcher, W: io::Write>(
    matcher: M,
    searcher: &mut grep::searcher::Searcher,
    printer: &mut Printer<W>,
    path: &Path,
) -> io::Result<()> {
    match *printer {
        Printer::Json(ref mut p) => {
            let mut sink = p.sink_with_path(&matcher, path);
            searcher.search_path(&matcher, path, &mut sink)?;
            Ok(())
        }
    }
}

/// Search the contents of the given reader using the given matcher, searcher
/// and printer.
fn search_reader<M: Matcher, R: io::Read, W: io::Write>(
    matcher: M,
    searcher: &mut grep::searcher::Searcher,
    printer: &mut Printer<W>,
    path: &Path,
    mut rdr: R,
) -> io::Result<()> {
    match *printer {
        Printer::Json(ref mut p) => {
            let mut sink = p.sink_with_path(&matcher, path);
            searcher.search_reader(&matcher, &mut rdr, &mut sink)?;
            Ok(())
        }
    }
}
