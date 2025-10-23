//! CLI output helpers with colored terminal support

use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

/// Print success message (green checkmark)
pub fn success(message: &str) {
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
    let _ = writeln!(&mut stdout, "✓ {message}");
    let _ = stdout.reset();
    log::info!("{message}");
}

/// Print error message (red X)
pub fn error(message: &str) {
    let mut stderr = StandardStream::stderr(ColorChoice::Auto);
    let _ = stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true));
    let _ = writeln!(&mut stderr, "✗ {message}");
    let _ = stderr.reset();
    log::error!("{message}");
}

/// Print warning message (yellow)
pub fn warning(message: &str) {
    let mut stderr = StandardStream::stderr(ColorChoice::Auto);
    let _ = stderr.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)));
    let _ = writeln!(&mut stderr, "⚠ {message}");
    let _ = stderr.reset();
    log::warn!("{message}");
}

/// Print info message (cyan)
pub fn info(message: &str) {
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)));
    let _ = writeln!(&mut stdout, "ℹ {message}");
    let _ = stdout.reset();
    log::info!("{message}");
}
