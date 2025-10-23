//! Interactive installation wizard for `kodegen_install`

use anyhow::Result;
use std::path::PathBuf;

/// Installation options gathered from interactive wizard
#[derive(Debug, Clone)]
pub struct InstallOptions {
    pub dry_run: bool,
    pub auto_start: bool,
}

impl Default for InstallOptions {
    fn default() -> Self {
        Self {
            dry_run: false,
            auto_start: true,
        }
    }
}

/// Results from actual installation (what was really installed)
#[derive(Debug, Clone)]
pub struct InstallationResult {
    pub data_dir: PathBuf,
    pub service_path: PathBuf,
    pub service_started: bool,
    pub certificates_installed: bool,
    pub host_entries_added: bool,
    pub fluent_voice_installed: bool,
}

/// Display welcome banner
fn show_welcome() {
    use std::io::Write;
    use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    
    // Top border with cyan color
    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)));
    let _ = writeln!(stdout, "\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    let _ = stdout.reset();
    
    // Brand name in cyan, centered
    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true));
    let _ = writeln!(stdout, "\n                    K O D E G E N . ᴀ ɪ");
    let _ = stdout.reset();
    
    // Tagline in white
    let _ = writeln!(stdout, "\n              Ultimate MCP Auto-Coding Toolset");
    
    // Bottom border
    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)));
    let _ = writeln!(stdout, "\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    let _ = stdout.reset();
    
    let _ = writeln!(stdout, "Installing system daemon service...\n");
    let _ = writeln!(stdout, "This will install:");
    let _ = writeln!(stdout, "  • Kodegen MCP Server daemon");
    let _ = writeln!(stdout, "  • TLS certificates for secure communication");
    let _ = writeln!(stdout, "  • System service configuration");
    let _ = writeln!(stdout, "  • Chromium browser (~100MB for web scraping)\n");
    
    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)));
    let _ = writeln!(stdout, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    let _ = stdout.reset();
}

/// Display installation completion summary
pub fn show_completion(_options: &InstallOptions, result: &InstallationResult) {
    use std::io::Write;
    use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
    
    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    
    // Top border
    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)));
    let _ = writeln!(stdout, "\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    let _ = stdout.reset();
    
    // Success header in green
    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true));
    let _ = writeln!(stdout, "\n                    ✓ INSTALLATION COMPLETE\n");
    let _ = stdout.reset();
    
    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)));
    let _ = writeln!(stdout, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    let _ = stdout.reset();
    
    let _ = writeln!(stdout, "Installed components:");
    
    // Show components with status indicators
    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
    let _ = writeln!(stdout, "  ✓ Kodegen daemon service");
    let _ = stdout.reset();
    
    if result.certificates_installed {
        let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
        let _ = writeln!(stdout, "  ✓ TLS certificates (CA + server)");
        let _ = stdout.reset();
    } else {
        let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)));
        let _ = writeln!(stdout, "  ⚠ TLS certificates (installation failed)");
        let _ = stdout.reset();
    }
    
    if result.host_entries_added {
        let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
        let _ = writeln!(stdout, "  ✓ Host file entries");
        let _ = stdout.reset();
    } else {
        let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)));
        let _ = writeln!(stdout, "  ⚠ Host file entries (skipped)");
        let _ = stdout.reset();
    }
    
    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
    let _ = writeln!(stdout, "  ✓ System service configuration");
    let _ = stdout.reset();
    
    if result.fluent_voice_installed {
        let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
        let _ = writeln!(stdout, "  ✓ Fluent-voice components");
        let _ = stdout.reset();
    } else {
        let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)));
        let _ = writeln!(stdout, "  ⚠ Fluent-voice components (optional)");
        let _ = stdout.reset();
    }
    
    // Service status
    let _ = writeln!(stdout, "\nService status:");
    if result.service_started {
        let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
        let _ = writeln!(stdout, "  ✓ Running at {}", result.service_path.display());
        let _ = stdout.reset();
    } else {
        let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)));
        let _ = writeln!(stdout, "  ⚠ Installed but not started");
        let _ = stdout.reset();
    }
    
    // Installation location
    let _ = writeln!(stdout, "\nInstallation location:");
    let _ = writeln!(stdout, "  {}", result.data_dir.display());
    
    // Bottom border
    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)));
    let _ = writeln!(stdout, "\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    let _ = stdout.reset();
    
    // Next steps
    let _ = writeln!(stdout, "\nNext: Restart your MCP client (Claude Desktop, Cursor, Windsurf)");
    
    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)));
    let _ = writeln!(stdout, "\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    let _ = stdout.reset();
}

/// Run interactive installation wizard
pub fn run_wizard() -> Result<InstallOptions> {
    use indicatif::{ProgressBar, ProgressStyle};
    use std::time::Duration;
    
    show_welcome();
    
    // Show progress spinner while preparing
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner())
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
    );
    spinner.set_message("Preparing installation...");
    spinner.enable_steady_tick(Duration::from_millis(80));
    
    // Simulate brief preparation
    std::thread::sleep(Duration::from_millis(500));
    
    spinner.finish_and_clear();
    
    // No prompts - return defaults immediately
    // Always: auto-start, not dry-run (system-wide is implicit for system daemons)
    Ok(InstallOptions {
        dry_run: false,
        auto_start: true,
    })
}

/// Check if running in non-interactive mode (CLI flags provided)
///
/// Returns true if the installer should skip the interactive wizard and run
/// in automated CLI mode.
///
/// Non-interactive mode is triggered by (priority order):
/// 1. Explicit `--no-interaction` flag (highest priority)
/// 2. `--gui` flag forces interactive mode (overrides auto-detection)
/// 3. `--from-platform` specified (package manager postinst script)
/// 4. Any other CLI flags that indicate automated installation (legacy)
///
/// Priority reasoning:
/// - `--no-interaction` always wins (explicit non-interactive command)
/// - `--gui` overrides auto-detection (explicit interactive request)  
/// - `--from-platform` implies package manager context (automated)
/// - Legacy flags (`dry_run`, custom binary) suggest CLI usage
pub fn is_non_interactive(cli: &crate::Cli) -> bool {
    // Priority 1: Explicit non-interactive flag always takes precedence
    if cli.no_interaction {
        return true;
    }
    
    // Priority 2: GUI flag explicitly requests interactive mode
    if cli.gui {
        return false;
    }
    
    // Priority 3: Platform-specified installations are typically automated
    // Package managers (deb, rpm) run postinst scripts in non-interactive context
    if cli.from_platform.is_some() {
        return true;
    }
    
    // Priority 4: Legacy detection - any explicit CLI flags indicate non-interactive
    cli.no_start 
        || cli.dry_run 
        || cli.uninstall
        || cli.binary.to_str() != Some("./target/release/kodegend")
}
