mod config;
#[cfg(feature = "gui")]
mod gui;
mod install;
mod wizard;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::timeout;

// Timeout constants for network operations
const CHECKSUM_DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(30); // Small text file
const BINARY_DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(600); // 10 min for 120MB
const CHROMIUM_INSTALL_TIMEOUT: Duration = Duration::from_secs(900); // 15 min for Chromium

/// Platform source indicator for installer behavior
#[derive(Clone, Debug, ValueEnum)]
pub enum PlatformSource {
    /// Running from Debian .deb postinst script
    #[value(name = "deb")]
    Deb,
    /// Running from RPM .rpm %post script
    #[value(name = "rpm")]
    Rpm,
    /// Running from macOS .pkg installer
    #[value(name = "pkg")]
    Pkg,
    /// Running from macOS .app bundle
    #[value(name = "dmg")]
    Dmg,
    /// Running from Windows .msi `CustomAction`
    #[value(name = "msi")]
    Msi,
    /// Running from NSIS .exe installer
    #[value(name = "nsis")]
    Nsis,
}

fn detect_platform_arch() -> Result<String> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    Ok(match (os, arch) {
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        _ => return Err(anyhow::anyhow!("Unsupported platform: {os}-{arch}")),
    }
    .to_string())
}

fn verify_checksum(file_path: &Path, expected_hash: &str) -> Result<bool> {
    let mut file = std::fs::File::open(file_path)
        .with_context(|| format!("Failed to open file for checksum: {}", file_path.display()))?;

    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)
        .with_context(|| format!("Failed to read file for checksum: {}", file_path.display()))?;

    let result = hasher.finalize();
    let actual_hash = hex::encode(result);

    Ok(actual_hash.eq_ignore_ascii_case(expected_hash))
}

async fn download_checksums(version: &str) -> Result<HashMap<String, String>> {
    let url =
        format!("https://github.com/cyrup-ai/kodegen/releases/download/{version}/checksums.txt");

    let response = match timeout(CHECKSUM_DOWNLOAD_TIMEOUT, reqwest::get(&url)).await {
        Ok(result) => result.with_context(|| format!("Failed to download checksums from {url}"))?,
        Err(_) => anyhow::bail!(
            "Timeout downloading checksums after {} seconds. \
             Check network connection or try: KODEGEN_HTTP_TIMEOUT={} {}",
            CHECKSUM_DOWNLOAD_TIMEOUT.as_secs(),
            CHECKSUM_DOWNLOAD_TIMEOUT.as_secs() * 2,
            std::env::current_exe()
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                .unwrap_or_else(|| "kodegen_install".to_string())
        ),
    };

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to download checksums (status: {})",
            response.status()
        );
    }

    let text = response
        .text()
        .await
        .context("Failed to read checksums response")?;

    let mut checksums = HashMap::new();

    for line in text.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            checksums.insert(parts[1].to_string(), parts[0].to_string());
        }
    }

    Ok(checksums)
}

/// RAII guard for temporary files that automatically cleans up on drop
struct TempFile(PathBuf);

impl Drop for TempFile {
    fn drop(&mut self) {
        // Silently attempt to remove the file
        // Ignore errors (file may not exist or already cleaned up)
        let _ = std::fs::remove_file(&self.0);
    }
}

impl TempFile {
    /// Create a new temporary file guard
    fn new(path: PathBuf) -> Self {
        TempFile(path)
    }

    /// Prevent cleanup on drop (for files that should persist)
    fn persist(self) {
        std::mem::forget(self);
    }
}

async fn download_signed_binary() -> Result<PathBuf> {
    use std::io::Write;
    use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

    let platform = detect_platform_arch()?;

    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)));
    let _ = writeln!(
        stdout,
        "📦 Downloading pre-built kodegend for {platform}..."
    );
    let _ = stdout.reset();

    // Determine archive extension and binary names
    let ext = if cfg!(windows) { "zip" } else { "tar.gz" };
    let source_binary_name = if cfg!(windows) {
        "sweetmcp-daemon.exe"
    } else {
        "sweetmcp-daemon"
    };
    let target_binary_name = if cfg!(windows) {
        "kodegend.exe"
    } else {
        "kodegend"
    };

    // Download binary archive (GitHub releases use sweetmcp-daemon naming)
    let archive_url = format!(
        "https://github.com/cyrup-ai/kodegen/releases/latest/download/sweetmcp-daemon-{platform}.{ext}"
    );

    let _ = writeln!(stdout, "   Downloading from: {archive_url}");

    let response = match timeout(BINARY_DOWNLOAD_TIMEOUT, reqwest::get(&archive_url)).await {
        Ok(result) => result.with_context(|| format!("Failed to request {archive_url}"))?,
        Err(_) => anyhow::bail!(
            "Timeout downloading binary after {} seconds ({} minutes). \
             The binary is ~120MB. On slow connections, increase timeout with: \
             KODEGEN_HTTP_TIMEOUT={} {}",
            BINARY_DOWNLOAD_TIMEOUT.as_secs(),
            BINARY_DOWNLOAD_TIMEOUT.as_secs() / 60,
            BINARY_DOWNLOAD_TIMEOUT.as_secs() * 2,
            std::env::current_exe()
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                .unwrap_or_else(|| "kodegen_install".to_string())
        ),
    };

    if !response.status().is_success() {
        anyhow::bail!("Failed to download binary (status: {})", response.status());
    }

    // Save archive to temp
    let temp_dir = std::env::temp_dir();
    let archive_path = temp_dir.join(format!("kodegend-{platform}.{ext}"));
    let _archive_guard = TempFile::new(archive_path.clone());

    let archive_bytes = match timeout(BINARY_DOWNLOAD_TIMEOUT, response.bytes()).await {
        Ok(result) => result.context("Failed to read archive bytes")?,
        Err(_) => anyhow::bail!(
            "Timeout reading binary archive after {} seconds. \
             Download may have stalled. Check network stability.",
            BINARY_DOWNLOAD_TIMEOUT.as_secs()
        ),
    };

    std::fs::write(&archive_path, &archive_bytes)
        .with_context(|| format!("Failed to write archive to {}", archive_path.display()))?;

    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
    let _ = writeln!(stdout, "   ✓ Downloaded archive");
    let _ = stdout.reset();

    // Download and verify checksum
    let _ = writeln!(stdout, "   Verifying checksum...");
    let checksums = download_checksums("latest").await?;
    let archive_name = format!("sweetmcp-daemon-{platform}.{ext}");

    if let Some(expected_hash) = checksums.get(&archive_name) {
        if !verify_checksum(&archive_path, expected_hash)? {
            // archive will be automatically cleaned up by _archive_guard on error
            anyhow::bail!("Checksum verification failed for {archive_name}");
        }
        let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
        let _ = writeln!(stdout, "   ✓ Checksum verified");
        let _ = stdout.reset();
    } else {
        let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)));
        let _ = writeln!(stdout, "   ⚠ No checksum available, skipping verification");
        let _ = stdout.reset();
    }

    // Extract binary from archive and rename to target name
    let temp_binary_path = temp_dir.join(source_binary_name);
    let _temp_binary_guard = TempFile::new(temp_binary_path.clone());
    let binary_path = temp_dir.join(target_binary_name);
    let binary_guard = TempFile::new(binary_path.clone());

    // Signature guard for Unix platforms - will be initialized during tar extraction
    let mut sig_guard: Option<TempFile> = None;

    if cfg!(windows) {
        // Extract ZIP
        let file = std::fs::File::open(&archive_path)
            .with_context(|| format!("Failed to open archive: {}", archive_path.display()))?;

        let mut archive = zip::ZipArchive::new(file).context("Failed to read ZIP archive")?;

        let mut binary_file = archive
            .by_name(source_binary_name)
            .with_context(|| format!("Binary {source_binary_name} not found in archive"))?;

        let mut output = std::fs::File::create(&temp_binary_path)
            .with_context(|| format!("Failed to create file: {}", temp_binary_path.display()))?;

        std::io::copy(&mut binary_file, &mut output)
            .context("Failed to extract binary from ZIP")?;
    } else {
        // Extract tar.gz
        use flate2::read::GzDecoder;
        use tar::Archive;

        let tar_gz = std::fs::File::open(&archive_path)
            .with_context(|| format!("Failed to open archive: {}", archive_path.display()))?;

        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);

        let mut found_binary = false;
        let source_sig_name = format!("{source_binary_name}.asc");
        let temp_sig_path = temp_dir.join(&source_sig_name);
        let _temp_sig_guard = TempFile::new(temp_sig_path.clone());

        for entry_result in archive.entries().context("Failed to read tar entries")? {
            let mut entry = entry_result.context("Failed to read tar entry")?;
            let path = entry.path().context("Failed to get entry path")?;
            let filename = path.file_name();

            if filename == Some(std::ffi::OsStr::new(source_binary_name)) {
                entry.unpack(&temp_binary_path).with_context(|| {
                    format!(
                        "Failed to extract {} to {}",
                        source_binary_name,
                        temp_binary_path.display()
                    )
                })?;
                found_binary = true;
            } else if filename == Some(std::ffi::OsStr::new(&source_sig_name)) {
                // Extract signature file for Linux
                entry.unpack(&temp_sig_path).with_context(|| {
                    format!(
                        "Failed to extract {} to {}",
                        source_sig_name,
                        temp_sig_path.display()
                    )
                })?;
            }
        }

        if !found_binary {
            // All temp files will be automatically cleaned up by their TempFile guards on error
            anyhow::bail!("Binary {source_binary_name} not found in tar.gz archive");
        }

        // Rename signature file if it exists (Linux only)
        if temp_sig_path.exists() {
            let target_sig_path = binary_path.with_extension("asc");
            sig_guard = Some(TempFile::new(target_sig_path.clone()));
            std::fs::rename(&temp_sig_path, &target_sig_path).with_context(|| {
                format!(
                    "Failed to rename signature file {} to {}",
                    temp_sig_path.display(),
                    target_sig_path.display()
                )
            })?;
        }
    }

    // Rename extracted binary from sweetmcp-daemon to kodegend
    std::fs::rename(&temp_binary_path, &binary_path).with_context(|| {
        format!(
            "Failed to rename {} to {}",
            temp_binary_path.display(),
            binary_path.display()
        )
    })?;

    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
    let _ = writeln!(stdout, "   ✓ Binary extracted");
    let _ = stdout.reset();

    // Set executable permissions (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&binary_path)
            .with_context(|| format!("Failed to get metadata: {}", binary_path.display()))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&binary_path, perms)
            .with_context(|| format!("Failed to set permissions: {}", binary_path.display()))?;
    }

    // Verify signature
    match is_binary_signed(&binary_path) {
        Ok(true) => {
            let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
            let _ = writeln!(stdout, "   ✓ Signature verified");
            let _ = stdout.reset();
        }
        Ok(false) => {
            let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)));
            let _ = writeln!(stdout, "   ⚠ Binary signature verification failed");
            let _ = stdout.reset();
        }
        Err(e) => {
            let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)));
            let _ = writeln!(stdout, "   ⚠ Could not verify signature: {e}");
            let _ = stdout.reset();
        }
    }

    // Prevent cleanup of final binary and signature files (they should persist)
    binary_guard.persist();
    if let Some(guard) = sig_guard {
        guard.persist();
    }

    // archive_path, temp_binary_path, and temp_sig_path will be automatically cleaned up
    // by their TempFile guards when they go out of scope

    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
    let _ = writeln!(stdout, "✓ Binary ready at: {}", binary_path.display());
    let _ = stdout.reset();
    Ok(binary_path)
}

fn is_binary_signed(binary: &Path) -> Result<bool> {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("codesign")
            .args(["--verify", "--verbose"])
            .arg(binary)
            .output()
            .context("Failed to run codesign")?;
        Ok(output.status.success())
    }
    #[cfg(target_os = "linux")]
    {
        // Check for .asc signature file and verify with gpg
        let sig_path = binary.with_extension("asc");
        if sig_path.exists() {
            let binary_str = binary
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid binary path"))?;
            let sig_str = sig_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid signature path"))?;

            let output = std::process::Command::new("gpg")
                .args(["--verify", sig_str, binary_str])
                .output()
                .context("Failed to run gpg verify")?;
            Ok(output.status.success())
        } else {
            // No signature file found
            Ok(false)
        }
    }
    #[cfg(target_os = "windows")]
    {
        // Verify Authenticode signature using PowerShell
        let binary_display = binary.display().to_string();
        let ps_command = format!(
            "(Get-AuthenticodeSignature '{}').Status -eq 'Valid'",
            binary_display
        );

        let output = std::process::Command::new("powershell")
            .args(["-Command", &ps_command])
            .output()
            .context("Failed to run PowerShell signature check")?;

        Ok(output.status.success())
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        // Unknown platform - skip verification
        Ok(true)
    }
}

/// Get binary paths based on platform source
///
/// Returns (`kodegen_path`, `kodegend_path`) tuple.
///
/// Platform-specific binary locations:
/// - Linux (deb/rpm): /usr/bin/ (installed by package manager)
/// - Windows (msi/nsis): C:\Program Files\Kodegen\ (installed by MSI)
/// - macOS (dmg/pkg): Contents/Resources/ (bundled in .app)
/// - None: Downloads from GitHub releases (fallback)
async fn get_bundled_binaries(
    platform_source: Option<PlatformSource>,
) -> Result<(PathBuf, PathBuf)> {
    use std::io::Write;
    use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

    let mut stdout = StandardStream::stdout(ColorChoice::Always);

    match platform_source {
        Some(PlatformSource::Deb | PlatformSource::Rpm) => {
            // Binaries already installed to /usr/bin by package manager
            let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
            let _ = writeln!(
                stdout,
                "📦 Using binaries from /usr/bin (installed by package manager)"
            );
            let _ = stdout.reset();

            Ok((
                PathBuf::from("/usr/bin/kodegen"),
                PathBuf::from("/usr/bin/kodegend"),
            ))
        }

        Some(PlatformSource::Msi | PlatformSource::Nsis) => {
            // Binaries in Program Files (installed by Windows installer)
            let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
            let _ = writeln!(stdout, "📦 Using binaries from Program Files");
            let _ = stdout.reset();

            let install_dir = PathBuf::from(r"C:\Program Files\Kodegen");
            Ok((
                install_dir.join("kodegen.exe"),
                install_dir.join("kodegend.exe"),
            ))
        }

        Some(PlatformSource::Dmg | PlatformSource::Pkg) => {
            // Extract from .app bundle Resources
            let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)));
            let _ = writeln!(stdout, "📦 Extracting binaries from .app bundle Resources");
            let _ = stdout.reset();

            extract_from_app_bundle()
        }

        None => {
            // Legacy behavior: download from GitHub releases
            let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)));
            let _ = writeln!(
                stdout,
                "📦 No platform source specified, downloading from GitHub"
            );
            let _ = stdout.reset();

            let kodegend_path = download_signed_binary().await?;

            // Assume kodegen is in same directory as kodegend
            let kodegen_path = kodegend_path
                .parent()
                .context("Invalid kodegend path")?
                .join(if cfg!(windows) {
                    "kodegen.exe"
                } else {
                    "kodegen"
                });

            Ok((kodegen_path, kodegend_path))
        }
    }
}

/// Extract binaries from macOS .app bundle Resources directory
///
/// Searches for the .app bundle containing this installer binary,
/// then returns paths to kodegen and kodegend from Contents/Resources/.
///
/// Returns (`kodegen_path`, `kodegend_path`) tuple with paths to Resources directory.
fn extract_from_app_bundle() -> Result<(PathBuf, PathBuf)> {
    use std::io::Write;
    use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

    let mut stdout = StandardStream::stdout(ColorChoice::Always);

    // Get current executable path
    let exe_path = std::env::current_exe().context("Failed to get current executable path")?;

    // Traverse up to find .app bundle (Contents/MacOS/kodegen_install)
    let mut current = exe_path.as_path();
    let app_bundle = loop {
        if let Some(parent) = current.parent() {
            if parent
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with(".app"))
            {
                break parent;
            }
            current = parent;
        } else {
            anyhow::bail!("Not running from within a .app bundle");
        }
    };

    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)));
    let _ = writeln!(stdout, "📦 Found .app bundle: {}", app_bundle.display());
    let _ = stdout.reset();

    // Binaries are in Contents/Resources/
    let resources_dir = app_bundle.join("Contents").join("Resources");

    let kodegen_source = resources_dir.join("kodegen");
    let kodegend_source = resources_dir.join("kodegend");

    // Verify binaries exist
    if !kodegen_source.exists() {
        anyhow::bail!(
            "kodegen binary not found in bundle: {}",
            kodegen_source.display()
        );
    }
    if !kodegend_source.exists() {
        anyhow::bail!(
            "kodegend binary not found in bundle: {}",
            kodegend_source.display()
        );
    }

    // Verify binaries are executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        for (name, path) in [("kodegen", &kodegen_source), ("kodegend", &kodegend_source)] {
            let metadata = std::fs::metadata(path)
                .with_context(|| format!("Failed to read metadata for {name}"))?;

            if metadata.permissions().mode() & 0o111 == 0 {
                anyhow::bail!(
                    "{} binary is not executable: {}\n\
                     Permissions: {:o}",
                    name,
                    path.display(),
                    metadata.permissions().mode()
                );
            }
        }
    }

    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
    let _ = writeln!(stdout, "✓ Found bundled binaries in .app Resources");
    let _ = writeln!(stdout, "   kodegen:  {}", kodegen_source.display());
    let _ = writeln!(stdout, "   kodegend: {}", kodegend_source.display());
    let _ = stdout.reset();

    Ok((kodegen_source, kodegend_source))
}

/// Install Chromium using citescrape's `download_managed_browser`
///
/// Chromium is REQUIRED - installation fails if this fails
async fn install_chromium() -> Result<PathBuf> {
    use kodegen_tools_citescrape::download_managed_browser;
    use std::io::Write;
    use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)));
    let _ = writeln!(stdout, "\n📥 Installing Chromium...");
    let _ = stdout.reset();
    let _ = writeln!(stdout, "   This may take 30-60 seconds (~100MB download)");

    let chromium_path = match timeout(CHROMIUM_INSTALL_TIMEOUT, download_managed_browser()).await {
        Ok(result) => result
            .context("Failed to download Chromium - check network connection and disk space")?,
        Err(_) => anyhow::bail!(
            "Timeout installing Chromium after {} seconds ({} minutes). \
             Chromium is ~100MB and required for citescrape functionality. \
             Increase timeout with: KODEGEN_CHROMIUM_TIMEOUT={} {}",
            CHROMIUM_INSTALL_TIMEOUT.as_secs(),
            CHROMIUM_INSTALL_TIMEOUT.as_secs() / 60,
            CHROMIUM_INSTALL_TIMEOUT.as_secs() * 2,
            std::env::current_exe()
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                .unwrap_or_else(|| "kodegen_install".to_string())
        ),
    };

    // Verify installation
    if !chromium_path.exists() {
        anyhow::bail!("Chromium path not found: {}", chromium_path.display());
    }

    Ok(chromium_path)
}

#[derive(Parser, Clone)]
#[command(name = "kodegen-install")]
#[command(version, about = "Install kodegen daemon as a system service")]
struct Cli {
    /// Path to kodegend binary to install (used with --from-source or as fallback)
    #[arg(long, default_value = "./target/release/kodegend")]
    binary: PathBuf,

    /// Don't start service after install
    #[arg(long)]
    no_start: bool,

    /// Show what would be done without doing it
    #[arg(long)]
    dry_run: bool,

    /// Uninstall instead of install
    #[arg(long)]
    uninstall: bool,

    /// Force building from source instead of downloading binary
    #[arg(long)]
    from_source: bool,

    /// Only download binary (fail if unavailable, don't fall back to local build)
    #[arg(long, conflicts_with = "from_source")]
    binary_only: bool,

    /// Platform source indicator (affects binary location detection)
    ///
    /// Tells the installer where binaries are located based on the
    /// installation source. Used by `BUNDLE_2` for binary extraction logic.
    #[arg(long = "from-platform", value_enum)]
    pub from_platform: Option<PlatformSource>,

    /// Force GUI wizard mode (default: auto-detect based on TTY)
    ///
    /// Enables interactive wizard even when other flags are set.
    /// Used for .msi/.nsis/.app installers to show GUI.
    #[arg(long)]
    pub gui: bool,

    /// Non-interactive mode for CI/server environments
    ///
    /// Runs installation without any prompts or interaction.
    /// Used for .deb/.rpm postinst scripts.
    #[arg(long)]
    pub no_interaction: bool,
}

/// Determine if GUI mode should be used based on CLI flags and platform
#[cfg(feature = "gui")]
fn should_use_gui(cli: &Cli) -> bool {
    // Explicit GUI flag has highest priority
    if cli.gui {
        return true;
    }

    // Platform sources that expect GUI (graphical installers)
    if let Some(ref platform) = cli.from_platform {
        match platform {
            PlatformSource::Dmg | PlatformSource::Pkg => true, // macOS installers
            PlatformSource::Msi | PlatformSource::Nsis => true, // Windows installers
            _ => false, // Deb/Rpm are headless (package manager postinst)
        }
    } else {
        false // No platform indicator = CLI wizard mode
    }
}

/// Run installation in GUI mode
#[cfg(feature = "gui")]
async fn run_gui_mode(cli: &Cli) -> Result<()> {
    // Delegate to GUI module's run_gui_installation (implemented in SUBTASK 5)
    let result = gui::run_gui_installation(cli).await?;

    // Log completion to stdout for CI/logging integration
    use std::io::Write;
    use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true));
    let _ = writeln!(stdout, "\n✅ Installation completed successfully");
    let _ = stdout.reset();
    let _ = writeln!(stdout, "   Data directory: {}", result.data_dir.display());

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let cli = Cli::parse();

    // Log platform source for diagnostics
    if let Some(ref platform) = cli.from_platform {
        log::info!("Running from platform: {platform:?}");
    }

    if cli.no_interaction {
        log::info!("Non-interactive mode enabled");
    }

    if cli.gui {
        log::info!("GUI mode requested");
    }

    if cli.uninstall {
        return run_uninstall(&cli).await;
    }

    // Check if GUI mode should be used
    #[cfg(feature = "gui")]
    if should_use_gui(&cli) {
        return run_gui_mode(&cli).await;
    }

    // Check if running in non-interactive mode (CLI flags provided)
    if wizard::is_non_interactive(&cli) {
        use std::io::Write;
        use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

        let mut stdout = StandardStream::stdout(ColorChoice::Always);
        let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)));
        let _ = writeln!(stdout, "🤖 Running in non-interactive mode");
        let _ = stdout.reset();
        return run_install(&cli).await;
    }

    // Interactive wizard mode (default when no flags provided)
    match wizard::run_wizard() {
        Ok(options) => run_install_with_options(&options, &cli).await,
        Err(e) => {
            use std::io::Write;
            use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

            let mut stderr = StandardStream::stderr(ColorChoice::Always);
            let _ = stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true));
            let _ = writeln!(stderr, "❌ Installation cancelled: {e}");
            let _ = stderr.reset();
            std::process::exit(1);
        }
    }
}

/// Run installation with wizard-collected options
async fn run_install_with_options(options: &wizard::InstallOptions, cli: &Cli) -> Result<()> {
    use indicatif::{ProgressBar, ProgressStyle};
    use std::io::Write;
    use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
    use tokio::sync::mpsc;

    // Use termcolor for starting message
    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)));
    let _ = writeln!(stdout, "\n⚡ Starting installation...\n");
    let _ = stdout.reset();

    // Create progress bar with enhanced style
    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("\n[{bar:50.cyan/blue}] {pos:>3}%  {msg}\n")
            .context("Invalid progress bar template")?
            .progress_chars("█▓░"),
    );

    // Determine binary paths (kodegen + kodegend) using platform-aware detection
    let (_kodegen_path, kodegend_path) = if options.dry_run {
        // Dry run doesn't need real binaries
        (
            PathBuf::from("./target/release/kodegen"),
            PathBuf::from("./target/release/kodegend"),
        )
    } else {
        // Use get_bundled_binaries() for platform-aware detection
        get_bundled_binaries(cli.from_platform.clone()).await?
    };

    // Compatibility: existing install_kodegen_daemon() expects single binary_path
    let binary_path = kodegend_path.clone();

    pb.set_message("Binaries located");
    pb.set_position(10);

    // Determine config path
    let config_path = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?
        .join("kodegen")
        .join("config.toml");

    pb.set_message("Validating prerequisites...");
    pb.set_position(20);

    if !options.dry_run && !binary_path.exists() {
        pb.finish_and_clear();
        anyhow::bail!("Binary not found: {}", binary_path.display());
    }

    // Create channel for real progress updates
    let (tx, mut rx) = mpsc::unbounded_channel::<install::core::InstallProgress>();

    // Spawn task to update progress bar from installation events
    let pb_clone = pb.clone();
    let progress_task = tokio::spawn(async move {
        while let Some(progress) = rx.recv().await {
            if progress.is_error {
                pb_clone.set_message(format!("❌ [{}] {}", progress.step, progress.message));
            } else {
                let pos = (progress.progress * 40.0) as u64 + 20; // Map 0-1 to 20-60
                pb_clone.set_position(pos);
                pb_clone.set_message(format!("[{}] {}", progress.step, progress.message));
            }
        }
    });

    pb.set_message("Installing daemon...");
    pb.set_position(25);

    // Call installation with real progress channel
    let result = install::config::install_kodegen_daemon(
        binary_path.clone(),
        config_path,
        options.auto_start,
        Some(tx),
    )
    .await;

    // Wait for all progress updates to complete
    progress_task.await.ok();

    // Check if daemon installation failed and get results
    let install_result = result?;

    pb.set_message("Daemon installed");
    pb.set_position(60);

    // Install Chromium (REQUIRED)
    pb.set_message("Installing Chromium (~100MB)...");
    pb.set_position(65);

    match install_chromium().await {
        Ok(chromium_path) => {
            pb.set_message("Chromium installed successfully");
            pb.set_position(85);

            let mut stdout = StandardStream::stdout(ColorChoice::Always);
            let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
            let _ = writeln!(
                stdout,
                "\n✓ Chromium installed at: {}",
                chromium_path.display()
            );
            let _ = stdout.reset();
        }
        Err(e) => {
            // Chromium is REQUIRED - fail installation
            pb.set_message("Chromium installation FAILED");
            pb.finish_and_clear();

            let mut stderr = StandardStream::stderr(ColorChoice::Always);
            let _ = stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true));
            let _ = writeln!(stderr, "\n❌ FATAL: Chromium installation failed");
            let _ = stderr.reset();
            let _ = stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)));
            let _ = writeln!(stderr, "   Error: {e}");
            let _ = stderr.reset();
            let _ = writeln!(stderr, "   Chromium is required for kodegen functionality.");
            let _ = writeln!(stderr, "   Please check:");
            let _ = writeln!(stderr, "   • Network connection is available");
            let _ = writeln!(stderr, "   • ~100MB free disk space");
            let _ = writeln!(
                stderr,
                "   • Firewall allows access to chromium download servers\n"
            );
            return Err(e);
        }
    }

    pb.set_message("Complete!");
    pb.set_position(100);
    pb.finish_and_clear();

    wizard::show_completion(options, &install_result);

    Ok(())
}

async fn run_install(cli: &Cli) -> Result<()> {
    use std::io::Write;
    use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true));
    let _ = writeln!(stdout, "🔧 Kodegen Daemon Installation");
    let _ = stdout.reset();
    let _ = writeln!(stdout, "Platform: {}\n", std::env::consts::OS);

    // Determine installation method based on flags
    let (kodegen_path, kodegend_path) = if cli.from_source {
        // Force building from source (existing logic)
        let _ = writeln!(stdout, "🔨 Building from source (--from-source specified)");

        if !cli.binary.exists() {
            anyhow::bail!(
                "Source binary not found: {}\n\
                 Build the project first with: cargo build --release",
                cli.binary.display()
            );
        }

        // Assume kodegen is in same directory as kodegend
        let kodegen = cli
            .binary
            .parent()
            .context("Invalid binary path")?
            .join(if cfg!(windows) {
                "kodegen.exe"
            } else {
                "kodegen"
            });

        (kodegen, cli.binary.clone())
    } else {
        // Use platform-aware binary detection (NEW)
        get_bundled_binaries(cli.from_platform.clone()).await?
    };

    // Display binary paths
    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)));
    let _ = writeln!(stdout, "\n📍 Binary locations:");
    let _ = stdout.reset();
    let _ = writeln!(stdout, "   kodegen:  {}", kodegen_path.display());
    let _ = writeln!(stdout, "   kodegend: {}", kodegend_path.display());

    // Verify binaries exist and are executable
    for (name, path) in [("kodegen", &kodegen_path), ("kodegend", &kodegend_path)] {
        if !path.exists() {
            anyhow::bail!("Binary not found: {} at {}", name, path.display());
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(path)?;
            if metadata.permissions().mode() & 0o111 == 0 {
                anyhow::bail!(
                    "Binary not executable: {} at {}\n\
                     Run: chmod +x {}",
                    name,
                    path.display(),
                    path.display()
                );
            }
        }
    }

    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
    let _ = writeln!(stdout, "✓ All binaries verified\n");
    let _ = stdout.reset();

    // Continue with existing installation logic using kodegend_path
    let binary_path = kodegend_path; // For compatibility with line 621

    let already_signed = is_binary_signed(&binary_path)?;
    if already_signed {
        let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
        let _ = writeln!(stdout, "✓ Binary is already signed");
        let _ = stdout.reset();
    }

    let _ = writeln!(stdout, "Installing {} to system...", binary_path.display());

    // Determine config path
    let config_path = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?
        .join("kodegen")
        .join("config.toml");

    // Call the actual installation logic (no progress channel in CLI mode)
    let auto_start = !cli.no_start;
    let install_result =
        install::config::install_kodegen_daemon(binary_path, config_path, auto_start, None).await?;

    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true));
    let _ = writeln!(
        stdout,
        "\n✅ Daemon installed to: {}",
        install_result.data_dir.display()
    );
    let _ = stdout.reset();
    let _ = writeln!(
        stdout,
        "   Service: {}",
        install_result.service_path.display()
    );

    if !install_result.certificates_installed {
        let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)));
        let _ = writeln!(stdout, "   ⚠ Certificate installation had issues");
        let _ = stdout.reset();
    }
    if !install_result.host_entries_added {
        let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)));
        let _ = writeln!(stdout, "   ⚠ Host entries not added (may require sudo)");
        let _ = stdout.reset();
    }

    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)));
    let _ = writeln!(stdout, "\n📦 Installing Chromium (required)...");
    let _ = stdout.reset();

    match install_chromium().await {
        Ok(chromium_path) => {
            let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
            let _ = writeln!(
                stdout,
                "✓ Chromium installed at: {}",
                chromium_path.display()
            );
            let _ = stdout.reset();
        }
        Err(e) => {
            let mut stderr = StandardStream::stderr(ColorChoice::Always);
            let _ = stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true));
            let _ = writeln!(stderr, "\n❌ FATAL: Chromium installation failed");
            let _ = stderr.reset();
            let _ = stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)));
            let _ = writeln!(stderr, "   Error: {e}");
            let _ = stderr.reset();
            let _ = writeln!(stderr, "   Chromium is required for kodegen functionality.");
            return Err(e);
        }
    }

    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true));
    let _ = writeln!(stdout, "\n✅ Installation complete");
    let _ = stdout.reset();

    Ok(())
}

async fn run_uninstall(_cli: &Cli) -> Result<()> {
    use std::io::Write;
    use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true));
    let _ = writeln!(stdout, "🗑️  Kodegen Daemon Uninstallation\n");
    let _ = stdout.reset();

    // Call the actual uninstallation logic
    install::uninstall::uninstall_kodegen_daemon()
        .await
        .context("Uninstallation failed")?;

    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true));
    let _ = writeln!(stdout, "✅ Uninstallation completed successfully!");
    let _ = stdout.reset();
    Ok(())
}
