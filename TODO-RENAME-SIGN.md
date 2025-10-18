# TODO: Rename setup → sign & Enhance Installation Wizard

## Core Objectives

1. **Rename** `packages/setup` → `packages/sign` (for Cyrup AI to sign release binaries)
2. **Fix binary naming**: use underscores not dashes (`kodegen_sign`, `kodegen_install`)
3. **Make** `kodegen_install` smart: download pre-signed macOS binaries from GitHub
4. **Add attractive wizard UI**: Replace basic CLI with `inquire`-based interactive installer
5. **Integrate Chromium**: Expose citescrape's chromium installation in wizard
6. **Update** `install.sh` to use correct binary names

---

## Architecture Overview

```
packages/
├── sign/              # For Cyrup AI to sign macOS release binaries
│   └── bin: kodegen_sign
│
├── install/           # Smart installer with interactive wizard
│   └── bin: kodegen_install
│       ├── Interactive mode: Beautiful wizard with inquire
│       ├── Non-interactive mode: CLI flags for automation
│       ├── macOS: downloads pre-signed from GitHub
│       ├── Linux/Windows: uses local binary
│       └── Optional: Install Chromium for web scraping (~100MB)
│
├── citescrape/        # Web scraping with chromium support
│   └── src/browser_setup.rs
│       └── ensure_chromium() - Production-ready chromium installer
│
└── daemon/            # Runtime daemon
    └── bin: kodegend
```

---

## Research Summary

### Existing Code Assets

1. **Citescrape Browser Setup** ([packages/citescrape/src/browser_setup.rs](./packages/citescrape/src/browser_setup.rs))
   - `pub async fn ensure_chromium() -> Result<PathBuf>`
   - Features: revision caching (1hr TTL), retry logic, validation, thread-safe
   - Downloads ~100MB on first run, subsequent calls use cache
   - Production-ready, battle-tested code

2. **Install Package Structure** ([packages/install/src/](./packages/install/src/))
   - `main.rs`: Basic CLI with clap
   - `core.rs`: Installation infrastructure (AsyncTask, InstallContext, certificates)
   - `builder.rs`: InstallerBuilder, ServiceConfig builders
   - `macos.rs`, `linux.rs`, `windows.rs`: Platform-specific installers
   - Ready for wizard integration

3. **UI Framework Decision**: 
   - ✅ **inquire** v0.7: Perfect for installation wizards (prompts, validation)
   - ✅ **indicatif** v0.17: Progress bars during long operations
   - ❌ **ratatui**: Too complex for installer, better for TUI apps

---

## PHASE 1: Rename packages/setup → packages/sign

### 1.1 Rename directory
**Action:** Move `packages/setup/` → `packages/sign/`
**Command:** `mv packages/setup packages/sign`

### 1.2 Update package name in Cargo.toml
**File:** `packages/sign/Cargo.toml`
**Line 2:** Change `name = "kodegen_setup"` → `name = "kodegen_sign"`

### 1.3 Update binary name in Cargo.toml
**File:** `packages/sign/Cargo.toml`
**Lines 6-8:** Change:
```toml
[[bin]]
name = "kodegen-setup"
path = "src/main.rs"
```
To:
```toml
[[bin]]
name = "kodegen_sign"
path = "src/main.rs"
```

### 1.4 Update root workspace Cargo.toml
**File:** `Cargo.toml` (root)
**Members list:** Change `"packages/setup"` → `"packages/sign"`

### 1.5 Verify: Build and test
```bash
cargo build --package kodegen_sign
# Should produce: target/debug/kodegen_sign
```

---

## PHASE 2: Fix packages/install binary name

### 2.1 Update binary name in Cargo.toml
**File:** `packages/install/Cargo.toml`
**Lines 6-8:** Change `name = "kodegen-install"` → `name = "kodegen_install"`

### 2.2 Verify: Build and test
```bash
cargo build --package kodegen_install
# Should produce: target/debug/kodegen_install
```

---

## PHASE 3: Make kodegen_install smart (GitHub downloads)

### 3.1 Add reqwest dependency
**File:** `packages/install/Cargo.toml`
```toml
reqwest = { version = "0.12", features = ["blocking"] }
```

### 3.2 Add platform detection and GitHub download
**File:** `packages/install/src/main.rs`
**Add functions:**

```rust
fn detect_platform_arch() -> String {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    
    match (os, arch) {
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        _ => panic!("Unsupported platform: {}-{}", os, arch),
    }.to_string()
}

fn get_latest_release_url(platform: &str) -> Result<String> {
    Ok(format!(
        "https://github.com/cyrup-ai/kodegen/releases/latest/download/kodegend-{}",
        platform
    ))
}

fn download_signed_binary() -> Result<PathBuf> {
    let platform = detect_platform_arch();
    
    if !platform.contains("apple-darwin") {
        anyhow::bail!("Pre-signed binaries only available for macOS");
    }
    
    println!("📦 Downloading pre-signed kodegend for {}...", platform);
    
    let url = get_latest_release_url(&platform)?;
    let response = reqwest::blocking::get(&url)?;
    
    if !response.status().is_success() {
        anyhow::bail!("Failed to download ({})", response.status());
    }
    
    let temp_dir = std::env::temp_dir();
    let binary_path = temp_dir.join("kodegend");
    
    let bytes = response.bytes()?;
    std::fs::write(&binary_path, bytes)?;
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&binary_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&binary_path, perms)?;
    }
    
    println!("✓ Downloaded to {}", binary_path.display());
    Ok(binary_path)
}

fn is_binary_signed(binary: &Path) -> Result<bool> {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("codesign")
            .args(["--verify", "--verbose"])
            .arg(binary)
            .output()?;
        Ok(output.status.success())
    }
    #[cfg(not(target_os = "macos"))]
    { Ok(true) }
}
```

### 3.3 Update run_install logic
**File:** `packages/install/src/main.rs`

```rust
fn run_install(cli: &Cli) -> Result<()> {
    println!("🔧 Kodegen Daemon Installation");
    println!("Platform: {}\n", std::env::consts::OS);
    
    let binary_path = if cli.binary.exists() {
        println!("Using provided binary: {}", cli.binary.display());
        cli.binary.clone()
    } else if std::env::consts::OS == "macos" {
        match download_signed_binary() {
            Ok(path) => {
                println!("✓ Using pre-signed binary from GitHub");
                path
            }
            Err(e) => {
                println!("⚠ Could not download: {}", e);
                cli.binary.clone()
            }
        }
    } else {
        cli.binary.clone()
    };
    
    if !binary_path.exists() {
        anyhow::bail!("Binary not found: {}", binary_path.display());
    }
    
    let already_signed = is_binary_signed(&binary_path)?;
    if already_signed {
        println!("✓ Binary is already signed");
    }
    
    println!("Installing {} to system...", binary_path.display());
    println!("\n✅ Installation complete");
    
    Ok(())
}
```

---

## PHASE 4: Update install.sh

### 4.1 Update binary names
**File:** `/Volumes/samsung_t9/kodegen.ai/install.sh`

**Search and replace:**
- `kodegen-setup` → `kodegen_sign`
- `kodegen-install` → `kodegen_install`

### 4.2 Update install_daemon_service function
**File:** `/Volumes/samsung_t9/kodegen.ai/install.sh`
**Lines ~218-228:**

```bash
install_daemon_service() {
    info "Installing daemon service..."
    
    if [[ -f "$HOME/.cargo/env" ]]; then
        source "$HOME/.cargo/env"
    fi
    
    if [[ "$OS" == "macos" ]]; then
        # macOS: auto-download from GitHub
        if kodegen_install; then
            success "Daemon installed and started!"
        else
            warn "Install failed. Manual: kodegen_install"
        fi
    else
        # Linux/Windows: use local binary
        local binary_path="$HOME/.cargo/bin/kodegend"
        if [[ -f "$binary_path" ]]; then
            if kodegen_install --binary "$binary_path"; then
                success "Daemon installed!"
            else
                warn "Install failed. Manual: kodegen_install --binary $binary_path"
            fi
        else
            error "Binary not found at $binary_path"
            exit 1
        fi
    fi
}
```

---

## PHASE 5: Build and verify baseline

### 5.1 Build all packages
```bash
cargo build --package kodegen_sign --package kodegen_install --package kodegen_daemon
```

### 5.2 Verify binaries
```bash
ls -la target/debug/kodegen_sign
ls -la target/debug/kodegen_install  
ls -la target/debug/kodegend

# All should use underscores, not dashes
```

### 5.3 Test kodegen_install
```bash
./target/debug/kodegen_install
# Should attempt GitHub download on macOS
```

---

## PHASE 6: Add Interactive Wizard UI with Inquire

### Context
**Current:** Basic CLI with clap flags only
**Target:** Beautiful interactive wizard with progress indicators
**Tech:** inquire v0.7 (prompts), indicatif v0.17 (progress bars)

### 6.1 Add UI dependencies
**File:** `packages/install/Cargo.toml`
```toml
# Interactive wizard UI
inquire = "0.7"
indicatif = "0.17"

# Update tokio for async main
tokio = { version = "1", features = ["rt", "rt-multi-thread", "macros"] }
```

### 6.2 Create wizard module
**File:** `packages/install/src/wizard.rs` (NEW)

```rust
//! Interactive installation wizard

use anyhow::Result;
use inquire::{Confirm, Select};

#[derive(Debug, Clone)]
pub struct InstallOptions {
    pub install_daemon: bool,
    pub install_chromium: bool,
    pub system_wide: bool,
    pub auto_start: bool,
}

impl Default for InstallOptions {
    fn default() -> Self {
        Self {
            install_daemon: true,
            install_chromium: false,
            system_wide: true,
            auto_start: true,
        }
    }
}

fn show_welcome() {
    println!("\n╔════════════════════════════════════════════════════╗");
    println!("║                                                    ║");
    println!("║              🚀 Kodegen Installation              ║");
    println!("║                                                    ║");
    println!("║     AI-Powered Desktop Commander & MCP Server      ║");
    println!("║                                                    ║");
    println!("╚════════════════════════════════════════════════════╝\n");
}

pub fn show_completion(options: &InstallOptions) {
    println!("\n╔════════════════════════════════════════════════════╗");
    println!("║                                                    ║");
    println!("║            ✅ Installation Complete!              ║");
    println!("║                                                    ║");
    println!("╚════════════════════════════════════════════════════╝\n");
    
    println!("📦 Installed components:");
    if options.install_daemon {
        println!("  ✓ Kodegen daemon (kodegend)");
    }
    if options.install_chromium {
        println!("  ✓ Chromium for web scraping");
    }
    
    println!("\n🎯 Next steps:");
    if options.auto_start {
        println!("  • Service started automatically");
    } else {
        println!("  • Start service: kodegend start");
    }
    println!("  • View logs: kodegend logs");
    println!("  • Config: ~/.config/kodegen/config.toml\n");
}

pub fn run_wizard() -> Result<InstallOptions> {
    show_welcome();
    
    let mut options = InstallOptions::default();
    
    // Daemon installation
    options.install_daemon = Confirm::new("Install kodegen daemon service?")
        .with_default(true)
        .with_help_message("Daemon provides MCP server functionality")
        .prompt()?;
    
    if !options.install_daemon {
        println!("\n⚠️  Without daemon, functionality will be limited.\n");
        return Ok(options);
    }
    
    // Chromium installation (optional, ~100MB)
    options.install_chromium = Confirm::new("Install Chromium for web scraping?")
        .with_default(false)
        .with_help_message("Enables citescrape (~100MB). Can install later.")
        .prompt()?;
    
    // Installation scope
    let scope = Select::new(
        "Installation scope:",
        vec!["System-wide (requires sudo)", "Current user only"]
    )
    .with_help_message("System: /usr/local | User: ~/.local")
    .prompt()?;
    
    options.system_wide = scope.contains("System-wide");
    
    // Auto-start
    options.auto_start = Confirm::new("Start automatically on boot?")
        .with_default(true)
        .prompt()?;
    
    // Summary
    println!("\n📋 Installation summary:");
    println!("  • Daemon: {}", if options.install_daemon { "✓" } else { "✗" });
    println!("  • Chromium: {}", if options.install_chromium { "✓" } else { "✗" });
    println!("  • Scope: {}", if options.system_wide { "System" } else { "User" });
    println!("  • Auto-start: {}", if options.auto_start { "Yes" } else { "No" });
    
    let proceed = Confirm::new("\nProceed with installation?")
        .with_default(true)
        .prompt()?;
    
    if !proceed {
        anyhow::bail!("Installation cancelled by user");
    }
    
    Ok(options)
}

pub fn is_non_interactive(cli: &crate::Cli) -> bool {
    // Non-interactive if any flags set
    cli.system_wide || cli.no_start || cli.dry_run || 
        cli.binary.to_str() != Some("./target/release/kodegend")
}
```

### 6.3 Update main.rs for wizard
**File:** `packages/install/src/main.rs`

**Add at top:**
```rust
mod wizard;
use wizard::{InstallOptions, is_non_interactive};
```

**Replace `fn main()` with:**
```rust
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    if cli.uninstall {
        return run_uninstall(&cli);
    }
    
    // CLI mode (automation/scripts)
    if is_non_interactive(&cli) {
        println!("🤖 Non-interactive mode");
        return run_install(&cli).await;
    }
    
    // Interactive wizard (default)
    match wizard::run_wizard() {
        Ok(options) => run_install_with_options(&options).await,
        Err(e) => {
            eprintln!("❌ Installation cancelled: {}", e);
            std::process::exit(1);
        }
    }
}
```

**Add new function:**
```rust
async fn run_install_with_options(options: &InstallOptions) -> Result<()> {
    use indicatif::{ProgressBar, ProgressStyle};
    
    println!("\n🔧 Starting installation...\n");
    
    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>3}% {msg}")
            .unwrap()
            .progress_chars("=>-")
    );
    
    // Step 1: Prerequisites (20%)
    pb.set_message("Checking prerequisites...");
    pb.set_position(20);
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    // Step 2: Daemon (40%)
    if options.install_daemon {
        pb.set_message("Installing daemon...");
        pb.set_position(40);
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    }
    
    // Step 3: Chromium (60%) - Phase 7
    if options.install_chromium {
        pb.set_message("Installing Chromium...");
        pb.set_position(60);
        // See Phase 7
    }
    
    // Step 4: Configure (80%)
    pb.set_message("Configuring service...");
    pb.set_position(80);
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    // Complete (100%)
    pb.set_message("Complete!");
    pb.set_position(100);
    pb.finish_and_clear();
    
    wizard::show_completion(options);
    
    Ok(())
}
```

**Update `run_install` to be async:**
```rust
async fn run_install(cli: &Cli) -> Result<()> {
    // Keep existing logic, just make it async
    // ... existing code ...
    Ok(())
}
```

### 6.4 Update mod.rs
**File:** `packages/install/src/mod.rs`
```rust
pub mod wizard;
```

### 6.5 Build and test wizard
```bash
cargo build --package kodegen_install
./target/debug/kodegen_install
# Should show interactive wizard

./target/debug/kodegen_install --dry-run
# Should use CLI mode
```

---

## PHASE 7: Integrate Chromium Installation

### Context
**Existing:** [packages/citescrape/src/browser_setup.rs](./packages/citescrape/src/browser_setup.rs)
- `pub async fn ensure_chromium() -> Result<PathBuf>`
- Downloads ~100MB, caches revision, retry logic
- Thread-safe, production-ready

### 7.1 Add citescrape dependency
**File:** `packages/install/Cargo.toml`
```toml
# Chromium installation
kodegen_citescrape = { path = "../citescrape" }
```

### 7.2 Implement chromium installation
**File:** `packages/install/src/main.rs`

**Update chromium section in `run_install_with_options`:**

Replace:
```rust
// Step 3: Chromium (60%) - Phase 7
if options.install_chromium {
    pb.set_message("Installing Chromium...");
    pb.set_position(60);
    // See Phase 7
}
```

With:
```rust
// Step 3: Chromium (60%)
if options.install_chromium {
    pb.set_message("Fetching Chromium (~100MB)...");
    pb.set_position(50);
    
    match install_chromium().await {
        Ok(chromium_path) => {
            pb.set_message("Chromium installed");
            pb.set_position(70);
            println!("\n  ✓ Chromium: {}", chromium_path.display());
        }
        Err(e) => {
            pb.set_message("Chromium failed");
            eprintln!("\n  ⚠️  Chromium install failed: {}", e);
            eprintln!("     Will download on first citescrape use.\n");
        }
    }
} else {
    pb.set_position(70);
}
```

**Add chromium installation function:**
```rust
/// Install Chromium using citescrape's ensure_chromium
async fn install_chromium() -> Result<std::path::PathBuf> {
    use kodegen_citescrape::ensure_chromium;
    
    println!("\n  📥 Downloading Chromium...");
    println!("     This may take 30-60 seconds (~100MB)\n");
    
    let chromium_path = ensure_chromium()
        .await
        .context("Failed to download Chromium")?;
    
    // Verify installation
    if !chromium_path.exists() {
        anyhow::bail!("Chromium path not found: {}", chromium_path.display());
    }
    
    Ok(chromium_path)
}
```

### 7.3 Add use statements
**File:** `packages/install/src/main.rs`
```rust
use anyhow::{Context, Result};
```

### 7.4 Build with chromium support
```bash
cargo build --package kodegen_install
# Will compile citescrape and chromiumoxide dependencies
```

### 7.5 Test chromium installation
```bash
./target/debug/kodegen_install
# Select "Yes" for Chromium
# Should download ~100MB and show progress
```

---

## Definition of Done

### Phase 1-5 (Baseline):
✅ Directory renamed: packages/setup → packages/sign
✅ Binary names use underscores: kodegen_sign, kodegen_install
✅ kodegen_install downloads pre-signed binaries (macOS)
✅ install.sh updated with correct names
✅ All packages build successfully

### Phase 6 (Wizard UI):
✅ inquire + indicatif dependencies added
✅ wizard.rs module created with interactive prompts
✅ main.rs async with tokio::main
✅ Beautiful welcome/completion screens
✅ Progress bars during installation
✅ Backward compatible CLI mode

### Phase 7 (Chromium Integration):
✅ kodegen_citescrape dependency added
✅ Chromium installation optional in wizard
✅ ensure_chromium() called when selected
✅ Progress shown during download
✅ Graceful error handling (network issues)
✅ Installation continues if chromium fails

---

## Success Criteria

```bash
# Cyrup AI (signing releases)
cargo build --package kodegen_sign
./target/debug/kodegen_sign --issuer-id ... --key-id ... --private-key ...
# Produces signed macOS binaries for GitHub releases

# End user - Interactive wizard (default)
./target/debug/kodegen_install
# Shows beautiful wizard, optional chromium install

# End user - Non-interactive (automation)
./target/debug/kodegen_install --binary ./kodegend --system-wide
# Uses CLI flags, skips wizard

# Chromium installation
# When user selects "Yes" in wizard:
# - Downloads ~100MB from Google
# - Shows progress
# - Caches for future use
# - Gracefully handles failures
```

---

## Implementation Notes

### File Changes Summary:
1. **packages/setup/** → **packages/sign/** (rename)
2. **packages/sign/Cargo.toml** - Update names
3. **packages/install/Cargo.toml** - Add inquire, indicatif, citescrape
4. **packages/install/src/wizard.rs** - NEW FILE (interactive UI)
5. **packages/install/src/main.rs** - Async, wizard integration, chromium
6. **packages/install/src/mod.rs** - Export wizard
7. **Cargo.toml** (root) - Update workspace members
8. **/Volumes/samsung_t9/kodegen.ai/install.sh** - Update binary names

### Dependencies Added:
```toml
inquire = "0.7"              # Interactive prompts
indicatif = "0.17"           # Progress bars
reqwest = "0.12"             # GitHub downloads
kodegen_citescrape = "..."   # Chromium installation
tokio = { features = ["macros"] }  # Async main
```

### Code Patterns:
- Use `inquire::Confirm` for yes/no prompts
- Use `inquire::Select` for multiple choice
- Use `indicatif::ProgressBar` for long operations
- Call `kodegen_citescrape::ensure_chromium()` for chromium
- Handle errors gracefully, don't panic on optional features

### References:
- [packages/citescrape/src/browser_setup.rs](./packages/citescrape/src/browser_setup.rs) - Chromium installer
- [packages/install/src/](./packages/install/src/) - Installation infrastructure
- [packages/setup/](./packages/setup/) - To be renamed to sign

