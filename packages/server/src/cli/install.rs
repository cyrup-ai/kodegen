use anyhow::Result;
use kodegen_client_autoconfig::install_all_clients;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use std::io::Write;

/// Run the install command to configure MCP-compatible editors
pub fn run_install() -> Result<()> {
    let results = install_all_clients()?;

    let mut stdout = StandardStream::stdout(ColorChoice::Always);

    // Header
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true))?;
    writeln!(&mut stdout, "\n┌─────────────────────────────────────────────┐")?;
    writeln!(&mut stdout, "│   🔍 MCP Editor Configuration Results       │")?;
    writeln!(&mut stdout, "└─────────────────────────────────────────────┘\n")?;
    stdout.reset()?;

    let mut configured = 0;
    let mut skipped = 0;
    let mut failed = 0;

    for result in &results {
        if result.success {
            // Success - green checkmark
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
            write!(&mut stdout, "  ✓ ")?;
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
            writeln!(&mut stdout, "{}", result.client_name)?;
        } else {
            // Failed - red X or dim skip
            if result.message == "Not installed" {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Black)).set_intense(true))?;
                write!(&mut stdout, "  ○ ")?;
                writeln!(&mut stdout, "{}", result.client_name)?;
            } else {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
                write!(&mut stdout, "  ✗ ")?;
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
                writeln!(&mut stdout, "{}", result.client_name)?;
            }
        }

        // Config path
        if let Some(ref path) = result.config_path {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Black)).set_intense(true))?;
            writeln!(&mut stdout, "     {}", path.display())?;
        }

        // Status message
        if result.success {
            if result.message.contains("Already") {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
                writeln!(&mut stdout, "     {}\n", result.message)?;
                skipped += 1;
            } else {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
                writeln!(&mut stdout, "     {}\n", result.message)?;
                configured += 1;
            }
        } else {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Black)).set_intense(true))?;
            writeln!(&mut stdout, "     {}\n", result.message)?;
            failed += 1;
        }
        stdout.reset()?;
    }

    // Summary
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true))?;
    writeln!(&mut stdout, "─────────────────────────────────────────────")?;
    stdout.reset()?;

    write!(&mut stdout, "  ")?;
    if configured > 0 {
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
        write!(&mut stdout, "{} configured", configured)?;
        stdout.reset()?;
    }
    if skipped > 0 {
        if configured > 0 { write!(&mut stdout, " • ")?; }
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
        write!(&mut stdout, "{} already configured", skipped)?;
        stdout.reset()?;
    }
    if failed > 0 {
        if configured > 0 || skipped > 0 { write!(&mut stdout, " • ")?; }
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Black)).set_intense(true))?;
        write!(&mut stdout, "{} not installed", failed)?;
        stdout.reset()?;
    }
    writeln!(&mut stdout, "\n")?;

    Ok(())
}
