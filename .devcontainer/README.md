# Kodegen Multi-Platform Release Builder

This devcontainer enables building release bundles for **all platforms** (Linux, macOS, Windows) locally on your Mac.

## Architecture

### Linux Container (this devcontainer)
- Compiles `kodegen_release` for Linux (`x86_64-unknown-linux-gnu`)
- Builds Linux packages: `.deb`, `.rpm`, `.AppImage`
- Builds Windows packages via Wine/Linux: `.msi` (WiX via Wine), `.exe` (NSIS native)

### macOS Host (your Mac)
- Compiles `kodegen_release` for macOS (`x86_64-apple-darwin` or `aarch64-apple-darwin`)
- Builds macOS packages: `.app`, `.dmg`
- Uses native tools: `hdiutil`, `codesign`, `notarytool`

## Prerequisites

- **Docker Desktop for Mac** - Install from https://www.docker.com/products/docker-desktop/
- **VS Code** - With "Dev Containers" extension
- **macOS with Xcode Command Line Tools** - For macOS builds on host

## Quick Start

### 1. Open in Container

```bash
# Open project in VS Code
code /Volumes/samsung_t9/kodegen

# VS Code will prompt: "Reopen in Container"
# Or: Cmd+Shift+P → "Dev Containers: Reopen in Container"
```

First build takes ~10 minutes (downloads images, installs Wine + .NET 4.0).
Subsequent opens are instant (cached).

### 2. Build Linux + Windows Packages (Inside Container)

```bash
# Inside the devcontainer terminal:

# Build the release binary
cd packages/release
cargo build --release

# Build all Linux packages
cargo run --release -- bundle --build --all-platforms

# Or build specific platforms:
cargo run --release -- bundle --build --platform deb
cargo run --release -- bundle --build --platform rpm  
cargo run --release -- bundle --build --platform appimage
cargo run --release -- bundle --build --platform msi     # via Wine
cargo run --release -- bundle --build --platform nsis    # native Linux makensis
```

**Output**: `target/release/bundle/{deb,rpm,appimage,msi,nsis}/`

### 3. Build macOS Packages (On Host Mac)

```bash
# Exit the container (Cmd+Shift+P → "Dev Containers: Reopen Locally")
# Or open a separate terminal on your Mac

cd /Volumes/samsung_t9/kodegen/packages/release

# Build macOS release binary
cargo build --release

# Build macOS packages
cargo run --release -- bundle --build --platform app
cargo run --release -- bundle --build --platform dmg

# With code signing (optional):
cargo run --release -- bundle --build --platform app --platform dmg
```

**Output**: `target/release/bundle/{macos,dmg}/`

## How It Works

### Platform-Specific Compilation

The `kodegen_release` bundler uses Rust's `#[cfg(target_os = "...")]` gates:

```rust
#[cfg(target_os = "linux")]
pub mod linux;   // Compiles .deb, .rpm, AppImage bundlers

#[cfg(target_os = "macos")]  
pub mod macos;   // Compiles .app, .dmg bundlers

#[cfg(target_os = "windows")]
pub mod windows; // Compiles .msi, .nsis bundlers
```

**Linux Container**: Compiles with `target_os = "linux"` → gets Linux + Windows bundlers  
**macOS Host**: Compiles with `target_os = "macos"` → gets macOS bundlers

### Windows Builds on Linux

#### WiX Toolset (for .msi)
1. `kodegen_release` downloads WiX 3.14 binaries from GitHub (existing code)
2. Cached in `$HOME/.cache/cyrup/WixTools314`
3. Executes `candle.exe` and `light.exe` via Wine
4. Wine + .NET 4.0 already installed in container (Dockerfile)

#### NSIS (for .exe installers)
1. `kodegen_release` downloads NSIS from GitHub (existing code)  
2. Executes via **native Linux `makensis`** (no Wine needed)
3. `apt-get install nsis` provides the Linux-native compiler

### Dependencies Pre-installed

The Dockerfile includes everything needed:
- ✅ Rust nightly-2024-10-20
- ✅ Wine 64-bit + .NET Framework 4.0
- ✅ NSIS native Linux compiler
- ✅ Tools for .deb, .rpm, AppImage creation

## Complete Release Workflow

```bash
# 1. Inside container - Build Linux + Windows
cd packages/release
cargo build --release
cargo run --release -- bundle --build --all-platforms

# 2. On macOS host - Build macOS
cargo build --release
cargo run --release -- bundle --build --platform app --platform dmg

# 3. All bundles are now in target/release/bundle/
ls -la ../../target/release/bundle/
```

## Troubleshooting

### Wine Issues

If WiX fails with Wine errors:

```bash
# Re-initialize Wine
rm -rf ~/.wine
wine wineboot --init
xvfb-run winetricks --unattended dotnet40
wineserver -w
```

### Cache Issues

```bash
# Clear download cache
rm -rf ~/.cache/cyrup

# Clear build cache
cargo clean
```

### Performance

**First build**: ~10 minutes (Wine + .NET setup)  
**Subsequent builds**: Fast (everything cached)

Wine .NET installation is one-time per container volume.

## Advanced Usage

### Custom WiX/NSIS Versions

The code automatically downloads tools. To use specific versions, modify:
- `packages/release/src/bundler/platform/windows/msi/mod.rs` - WiX URL
- `packages/release/src/bundler/platform/windows/nsis/mod.rs` - NSIS URL

### Cross-Compilation Targets

Currently builds for host architecture. For cross-compilation, add Rust targets:

```bash
rustup target add x86_64-unknown-linux-gnu
rustup target add aarch64-unknown-linux-gnu
```

## References

- WiX on Wine: https://github.com/suchja/wix-toolset
- NSIS Linux: https://nsis.sourceforge.io/Docs/AppendixG.html
- Dev Containers: https://containers.dev/
