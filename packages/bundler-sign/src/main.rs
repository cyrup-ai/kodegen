use anyhow::Result;
use clap::Parser;
use std::io::Write;
use std::path::{Path, PathBuf};
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

use kodegen_bundler_sign::config::{
    CertificateType, DEFAULT_KEYCHAIN, MacOSSetupConfig, PlatformConfig, SetupConfig,
};

// ============================================================================
// ERROR HANDLING STRATEGY
// ============================================================================
//
// This module distinguishes between CRITICAL and DECORATIVE I/O operations:
//
// CRITICAL I/O - Errors propagated with `?` operator:
//   • File operations: fs::write(), fs::read_to_string(), fs::create_dir_all()
//   • User input: io::stdin().read_line()
//   • External processes: Command execution, API calls
//   • GitHub API operations: Upload requests
//
//   These MUST succeed for the program to function correctly.
//   Errors are propagated to the caller for proper handling.
//
// DECORATIVE I/O - Errors ignored with `let _ =`:
//   • Terminal coloring: buffer.set_color(), writeln!(), bufwtr.print()
//   • Status messages: Success/warning/error indicators with colors
//
//   These are nice-to-have but non-essential. If stderr/stdout is closed,
//   TTY is detached, or output is redirected to a broken pipe, the program
//   should continue without colors - not crash.
//
// This follows Rust CLI ecosystem best practices (cargo, rustc, ripgrep).
//
// Example:
//   io::stdout().flush()?;              // Critical - propagate error
//   let _ = buffer.set_color(...);      // Decorative - ignore error
// ============================================================================

#[cfg(target_os = "linux")]
use kodegen_bundler_sign::linux;
#[cfg(target_os = "windows")]
use kodegen_bundler_sign::windows;
#[cfg(target_os = "macos")]
use kodegen_bundler_sign::{build_helper, macos};

#[derive(Parser)]
#[command(name = "kodegen_sign")]
#[command(version, about = "Configure code signing for kodegen daemon")]
struct Cli {
    /// Show current configuration
    #[arg(long)]
    show: bool,

    /// Interactive mode (prompt for credentials)
    #[arg(long, short = 'i', conflicts_with = "show")]
    interactive: bool,

    /// Build and sign macOS helper app
    #[arg(long, conflicts_with_all = ["show", "interactive"])]
    build_helper: bool,

    /// Upload helper to GitHub releases (requires --build-helper)
    #[arg(long, requires = "build_helper")]
    upload: bool,

    /// GitHub token for upload (defaults to `GITHUB_TOKEN` env var)
    #[arg(long)]
    github_token: Option<String>,

    /// Output directory for helper (defaults to target/helper)
    #[arg(long, default_value = "target/helper")]
    output_dir: PathBuf,

    /// Path to setup config file (TOML)
    #[arg(long, short = 'c', conflicts_with_all = ["interactive", "show", "build_helper"])]
    config: Option<PathBuf>,

    /// App Store Connect Issuer ID (macOS)
    #[arg(long, requires_all = ["key_id", "private_key"])]
    issuer_id: Option<String>,

    /// App Store Connect Key ID (macOS)
    #[arg(long, requires_all = ["issuer_id", "private_key"])]
    key_id: Option<String>,

    /// Path to .p8 private key file (macOS)
    #[arg(long, requires_all = ["issuer_id", "key_id"])]
    private_key: Option<PathBuf>,

    /// Notarize a macOS app bundle
    #[cfg(target_os = "macos")]
    #[arg(long, conflicts_with_all = ["show", "interactive", "build_helper", "config", "sign"])]
    notarize: Option<PathBuf>,

    /// Wait for notarization to complete (default: true)
    #[cfg(target_os = "macos")]
    #[arg(long, requires = "notarize", default_value = "true")]
    wait: bool,

    /// Sign a binary with entitlements
    #[cfg(target_os = "macos")]
    #[arg(long, conflicts_with_all = ["show", "interactive", "build_helper", "config", "notarize"])]
    sign: Option<PathBuf>,

    /// Signing identity for --sign
    #[cfg(target_os = "macos")]
    #[arg(long, requires = "sign")]
    identity: Option<String>,

    /// Path to entitlements.plist for --sign
    #[cfg(target_os = "macos")]
    #[arg(long, requires = "sign")]
    entitlements: Option<PathBuf>,

    /// Enable hardened runtime for --sign (default: true)
    #[cfg(target_os = "macos")]
    #[arg(long, requires = "sign", default_value = "true")]
    hardened_runtime: bool,

    /// Diagnose notarization setup
    #[cfg(target_os = "macos")]
    #[arg(long, conflicts_with_all = ["show", "interactive", "build_helper", "config", "notarize", "sign"])]
    diagnose_notarization: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.show {
        return show_config();
    }

    if cli.interactive {
        return run_interactive();
    }

    #[cfg(target_os = "macos")]
    if cli.build_helper {
        return build_and_upload_helper(&cli.output_dir, cli.upload, cli.github_token);
    }

    #[cfg(target_os = "macos")]
    if cli.diagnose_notarization {
        return macos::diagnose_notarization_setup().map_err(Into::into);
    }

    #[cfg(target_os = "macos")]
    if let Some(app_path) = cli.notarize {
        return run_notarize(&app_path, cli.wait);
    }

    #[cfg(target_os = "macos")]
    if let Some(binary_path) = cli.sign {
        let identity = cli
            .identity
            .ok_or_else(|| anyhow::anyhow!("--identity is required when using --sign"))?;
        return run_sign(
            &binary_path,
            &identity,
            cli.entitlements.as_deref(),
            cli.hardened_runtime,
        );
    }

    if let Some(config_path) = cli.config {
        return run_from_config(&config_path);
    }

    if let (Some(issuer), Some(key), Some(pk)) = (cli.issuer_id, cli.key_id, cli.private_key) {
        return run_from_args(&issuer, &key, &pk);
    }

    // Default: interactive
    let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
    let mut buffer = bufwtr.buffer();
    // Informational output - errors ignored (see module-level docs)
    let _ = writeln!(
        &mut buffer,
        "No mode specified. Running interactive setup...\n"
    );
    let _ = bufwtr.print(&buffer);
    run_interactive()
}

fn show_config() -> Result<()> {
    let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
    let mut buffer = bufwtr.buffer();
    // Header output - errors ignored (see module-level docs)
    let _ = writeln!(&mut buffer, "📋 Current Setup Configuration\n");
    let _ = bufwtr.print(&buffer);

    #[cfg(target_os = "macos")]
    {
        macos::show_config()?;
    }

    #[cfg(target_os = "linux")]
    {
        linux::show_config()?;
    }

    #[cfg(target_os = "windows")]
    {
        windows::show_config()?;
    }

    Ok(())
}

fn run_interactive() -> Result<()> {
    let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
    let mut buffer = bufwtr.buffer();
    // Welcome banner - errors ignored (see module-level docs)
    let _ = writeln!(&mut buffer, "{}", "=".repeat(60));
    let _ = writeln!(&mut buffer, "🔧 Kodegen Interactive Setup");
    let _ = writeln!(&mut buffer, "Platform: {}", std::env::consts::OS);
    let _ = writeln!(&mut buffer, "{}", "=".repeat(60));
    let _ = writeln!(&mut buffer);
    let _ = bufwtr.print(&buffer);

    #[cfg(target_os = "macos")]
    return macos::interactive_setup().map_err(Into::into);

    #[cfg(target_os = "linux")]
    return linux::interactive_setup().map_err(Into::into);

    #[cfg(target_os = "windows")]
    return windows::interactive_setup().map_err(Into::into);
}

fn run_from_config(config_path: &PathBuf) -> Result<()> {
    let content = std::fs::read_to_string(config_path)?;
    let config: SetupConfig = toml::from_str(&content)?;

    #[cfg(target_os = "macos")]
    {
        if let PlatformConfig::MacOS(macos_config) = config.platform {
            return macos::setup_from_config(&macos_config, config.dry_run, config.verbose)
                .map_err(Into::into);
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let PlatformConfig::Linux(linux_config) = config.platform {
            return linux::setup_from_config(&linux_config, config.dry_run, config.verbose)
                .map_err(Into::into);
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let PlatformConfig::Windows(windows_config) = config.platform {
            return windows::setup_from_config(&windows_config, config.dry_run, config.verbose)
                .map_err(Into::into);
        }
    }

    anyhow::bail!("Platform mismatch in config file")
}

fn run_from_args(issuer_id: &str, key_id: &str, private_key: &Path) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let config = MacOSSetupConfig {
            issuer_id: issuer_id.to_string(),
            key_id: key_id.to_string(),
            private_key_path: private_key.to_path_buf(),
            certificate_type: CertificateType::DeveloperIdApplication,
            common_name: "Kodegen Helper".to_string(),
            keychain: DEFAULT_KEYCHAIN.to_string(),
        };
        macos::setup_from_config(&config, false, false).map_err(Into::into)
    }

    #[cfg(not(target_os = "macos"))]
    {
        anyhow::bail!("API credentials only apply to macOS setup")
    }
}

#[cfg(target_os = "macos")]
fn build_and_upload_helper(
    output_dir: &std::path::Path,
    upload: bool,
    github_token: Option<String>,
) -> Result<()> {
    use sha2::Digest;

    let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
    let mut buffer = bufwtr.buffer();
    let _ = writeln!(&mut buffer, "🔨 Building macOS helper app...");
    let _ = bufwtr.print(&buffer);

    std::fs::create_dir_all(output_dir)?;

    // Build and sign helper using existing module
    let zip_path = build_helper::build_and_sign_helper(output_dir)
        .map_err(|e| anyhow::anyhow!("Failed to build helper: {e}"))?;

    let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
    let mut buffer = bufwtr.buffer();
    let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
    let _ = write!(&mut buffer, "✓ ");
    let _ = buffer.reset();
    let _ = writeln!(&mut buffer, "Helper packaged: {}", zip_path.display());
    let _ = bufwtr.print(&buffer);

    // Calculate SHA256 for verification
    let zip_data = std::fs::read(&zip_path)?;
    let hash = sha2::Sha256::digest(&zip_data);
    let hash_hex = hex::encode(hash);

    let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
    let mut buffer = bufwtr.buffer();
    let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
    let _ = write!(&mut buffer, "✓ ");
    let _ = buffer.reset();
    let _ = writeln!(&mut buffer, "SHA256: {hash_hex}");
    let _ = bufwtr.print(&buffer);

    if upload {
        // Try to get token from arg or env var
        let token = github_token.or_else(|| std::env::var("GITHUB_TOKEN").ok());

        if let Some(token) = token {
            upload_to_github(&zip_path, &token)?;

            let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
            let mut buffer = bufwtr.buffer();
            let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
            let _ = write!(&mut buffer, "✓ ");
            let _ = buffer.reset();
            let _ = writeln!(&mut buffer, "Uploaded to GitHub releases");
            let _ = bufwtr.print(&buffer);
        } else {
            let bufwtr = BufferWriter::stderr(ColorChoice::Auto);
            let mut buffer = bufwtr.buffer();

            let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Red)));
            let _ = writeln!(&mut buffer, "❌ GitHub token required for upload");
            let _ = buffer.reset();
            let _ = writeln!(
                &mut buffer,
                "   Set GITHUB_TOKEN environment variable or use --github-token"
            );
            let _ = bufwtr.print(&buffer);

            std::process::exit(1);
        }
    } else {
        let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
        let mut buffer = bufwtr.buffer();
        let _ = writeln!(&mut buffer, "\n📦 To upload to GitHub releases:");
        let _ = writeln!(
            &mut buffer,
            "   cargo run --package kodegen_sign -- --build-helper --upload"
        );
        let _ = writeln!(&mut buffer, "   (requires GITHUB_TOKEN env var)");
        let _ = bufwtr.print(&buffer);
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn upload_to_github(zip_path: &PathBuf, token: &str) -> Result<()> {
    let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
    let mut buffer = bufwtr.buffer();

    let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Blue)));
    let _ = write!(&mut buffer, "🚀 ");
    let _ = buffer.reset();
    let _ = writeln!(&mut buffer, "Uploading to GitHub releases...");
    let _ = bufwtr.print(&buffer);

    // Determine architecture for asset name
    let arch = std::env::consts::ARCH;
    let asset_name = format!("KodegenHelper.app-macos-{arch}.zip");

    // Read file
    let file_data = std::fs::read(zip_path)?;

    // Create runtime for async operations
    let runtime = tokio::runtime::Runtime::new()?;

    runtime.block_on(async {
        // Create octocrab instance
        let octocrab = octocrab::Octocrab::builder()
            .personal_token(token.to_string())
            .build()?;

        // Get latest release
        let release = octocrab
            .repos("cyrup-ai", "kodegen")
            .releases()
            .get_latest()
            .await?;

        let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
        let mut buffer = bufwtr.buffer();
        let _ = writeln!(&mut buffer, "   Uploading to release: {}", release.tag_name);
        let _ = bufwtr.print(&buffer);

        // Upload asset using octocrab's upload_asset builder pattern (0.42 API)
        octocrab
            .repos("cyrup-ai", "kodegen")
            .releases()
            .upload_asset(*release.id, &asset_name, bytes::Bytes::from(file_data))
            .send()
            .await?;

        let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
        let mut buffer = bufwtr.buffer();

        let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
        let _ = write!(&mut buffer, "✓ ");
        let _ = buffer.reset();
        let _ = writeln!(&mut buffer, "Asset uploaded: {asset_name}");
        let _ = writeln!(
            &mut buffer,
            "   URL: {}/releases/download/{}/{}",
            release.html_url, release.tag_name, asset_name
        );
        let _ = bufwtr.print(&buffer);

        Ok::<_, anyhow::Error>(())
    })?;

    Ok(())
}

#[cfg(target_os = "macos")]
fn run_notarize(app_path: &Path, wait: bool) -> Result<()> {
    let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
    let mut buffer = bufwtr.buffer();
    let _ = writeln!(&mut buffer, "🔐 Starting notarization...\n");
    let _ = bufwtr.print(&buffer);

    // Load auth from environment variables
    let auth = macos::NotarizationAuth::from_env()?;

    // Run notarization
    macos::notarize(app_path, &auth, wait)?;

    Ok(())
}

#[cfg(target_os = "macos")]
fn run_sign(
    binary_path: &Path,
    identity: &str,
    entitlements_path: Option<&Path>,
    hardened_runtime: bool,
) -> Result<()> {
    let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
    let mut buffer = bufwtr.buffer();
    let _ = writeln!(&mut buffer, "✍️  Signing binary...\n");
    let _ = bufwtr.print(&buffer);

    // Sign with entitlements
    macos::sign_with_entitlements(binary_path, identity, entitlements_path, hardened_runtime)?;

    let mut buffer = bufwtr.buffer();
    let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
    let _ = writeln!(&mut buffer, "\n✅ Signing complete!");
    let _ = buffer.reset();
    let _ = bufwtr.print(&buffer);

    Ok(())
}
