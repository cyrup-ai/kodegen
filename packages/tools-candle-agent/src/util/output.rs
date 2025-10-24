//! Terminal output utilities with color support
//!
//! Provides colored output functions for user-facing messages.
//! Uses termcolor for cross-platform color support.

use std::io::{self, Write};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

/// Print a success message in green with checkmark
pub fn print_success(msg: &str) -> io::Result<()> {
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
    write!(&mut stdout, "✓ ")?;
    stdout.reset()?;
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
    writeln!(&mut stdout, "{}", msg)?;
    stdout.reset()
}

/// Print an error message in red with X mark
pub fn print_error(msg: &str) -> io::Result<()> {
    let mut stderr = StandardStream::stderr(ColorChoice::Auto);
    stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
    write!(&mut stderr, "✗ ")?;
    stderr.reset()?;
    stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
    writeln!(&mut stderr, "{}", msg)?;
    stderr.reset()
}

/// Print an info message in cyan
pub fn print_info(msg: &str) -> io::Result<()> {
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)))?;
    writeln!(&mut stdout, "{}", msg)?;
    stdout.reset()
}

/// Print a warning message in yellow with warning symbol
pub fn print_warning(msg: &str) -> io::Result<()> {
    let mut stderr = StandardStream::stderr(ColorChoice::Auto);
    stderr.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true))?;
    write!(&mut stderr, "⚠ ")?;
    stderr.reset()?;
    stderr.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
    writeln!(&mut stderr, "{}", msg)?;
    stderr.reset()
}

/// Print a highlighted message (magenta, for important info)
pub fn print_highlight(msg: &str) -> io::Result<()> {
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Magenta)).set_bold(true))?;
    writeln!(&mut stdout, "{}", msg)?;
    stdout.reset()
}
