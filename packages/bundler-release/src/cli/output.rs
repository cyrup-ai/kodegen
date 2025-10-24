//! Colored terminal output for release operations
//!
//! Provides consistent, colored CLI output with proper formatting

use std::io::Write;
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

/// Output manager for consistent colored terminal output
#[derive(Debug)]
pub struct OutputManager {
    bufwtr: BufferWriter,
    verbose: bool,
    quiet: bool,
}

impl OutputManager {
    /// Create a new output manager
    pub fn new(verbose: bool, quiet: bool) -> Self {
        Self {
            bufwtr: BufferWriter::stdout(ColorChoice::Auto),
            verbose,
            quiet,
        }
    }

    /// Print an info message (normal output)
    pub fn info(&self, message: &str) {
        if self.quiet {
            return;
        }

        let mut buffer = self.bufwtr.buffer();
        let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)));
        let _ = write!(&mut buffer, "ℹ ");
        let _ = buffer.reset();
        let _ = writeln!(&mut buffer, " {}", message);
        let _ = self.bufwtr.print(&buffer);
    }

    /// Print a success message
    pub fn success(&self, message: &str) {
        if self.quiet {
            return;
        }

        let mut buffer = self.bufwtr.buffer();
        let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true));
        let _ = write!(&mut buffer, "✓");
        let _ = buffer.reset();
        let _ = writeln!(&mut buffer, " {}", message);
        let _ = self.bufwtr.print(&buffer);
    }

    /// Print a warning message
    pub fn warn(&self, message: &str) {
        if self.quiet {
            return;
        }

        let mut buffer = self.bufwtr.buffer();
        let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true));
        let _ = write!(&mut buffer, "⚠");
        let _ = buffer.reset();
        let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)));
        let _ = writeln!(&mut buffer, " {}", message);
        let _ = buffer.reset();
        let _ = self.bufwtr.print(&buffer);
    }

    /// Print an error message (always shown)
    pub fn error(&self, message: &str) {
        let bufwtr = BufferWriter::stderr(ColorChoice::Auto);
        let mut buffer = bufwtr.buffer();
        let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true));
        let _ = write!(&mut buffer, "✗");
        let _ = buffer.reset();
        let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Red)));
        let _ = writeln!(&mut buffer, " {}", message);
        let _ = buffer.reset();
        let _ = bufwtr.print(&buffer);
    }

    /// Print a verbose/debug message (only in verbose mode)
    pub fn verbose(&self, message: &str) {
        if !self.verbose || self.quiet {
            return;
        }

        let mut buffer = self.bufwtr.buffer();
        let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Blue)));
        let _ = write!(&mut buffer, "→");
        let _ = buffer.reset();
        let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::White)));
        let _ = writeln!(&mut buffer, " {}", message);
        let _ = buffer.reset();
        let _ = self.bufwtr.print(&buffer);
    }

    /// Print a progress message with spinner/activity indicator
    pub fn progress(&self, message: &str) {
        if self.quiet {
            return;
        }

        let mut buffer = self.bufwtr.buffer();
        let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Magenta)));
        let _ = write!(&mut buffer, "⋯");
        let _ = buffer.reset();
        let _ = writeln!(&mut buffer, " {}", message);
        let _ = self.bufwtr.print(&buffer);
    }

    /// Print a section header
    pub fn section(&self, title: &str) {
        if self.quiet {
            return;
        }

        let mut buffer = self.bufwtr.buffer();
        let _ = writeln!(&mut buffer);
        let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true));
        let _ = writeln!(&mut buffer, "═══ {} ═══", title);
        let _ = buffer.reset();
        let _ = self.bufwtr.print(&buffer);
    }

    /// Print indented text (for sub-items)
    pub fn indent(&self, message: &str) {
        if self.quiet {
            return;
        }

        let mut buffer = self.bufwtr.buffer();
        let _ = writeln!(&mut buffer, "    {}", message);
        let _ = self.bufwtr.print(&buffer);
    }

    /// Print a plain message (respects quiet mode)
    pub fn println(&self, message: &str) {
        if self.quiet {
            return;
        }

        let mut buffer = self.bufwtr.buffer();
        let _ = writeln!(&mut buffer, "{}", message);
        let _ = self.bufwtr.print(&buffer);
    }

    /// Check if verbose mode is enabled
    pub fn is_verbose(&self) -> bool {
        self.verbose
    }

    /// Check if quiet mode is enabled
    pub fn is_quiet(&self) -> bool {
        self.quiet
    }
}
