# Building Multi-Platform Release Bundles

This package supports building release bundles for all platforms (Linux, macOS, Windows) using a local .devcontainer setup.

## Quick Start

For complete setup instructions and usage documentation, see:

**[`.devcontainer/README.md`](../../.devcontainer/README.md)** in the project root.

## Overview

The release bundler creates platform-specific installers:

### Linux Packages
- **Debian (.deb)** - For Ubuntu, Debian, and derivatives
- **RPM (.rpm)** - For Fedora, RHEL, CentOS, and derivatives  
- **AppImage (.AppImage)** - Portable, self-contained Linux executables

### macOS Packages
- **App Bundle (.app)** - macOS application bundle
- **DMG (.dmg)** - macOS disk image installer

### Windows Packages
- **MSI (.msi)** - Windows Installer package via WiX Toolset
- **NSIS (.exe)** - Lightweight Windows installer via NSIS

## Building Locally

### Inside .devcontainer (Linux + Windows)

```bash
cd packages/bundler-release

# Build Linux packages
cargo run --release -- bundle --build --platform deb
cargo run --release -- bundle --build --platform rpm
cargo run --release -- bundle --build --platform appimage

# Build Windows packages (via Wine in Linux container)
cargo run --release -- bundle --build --platform msi
cargo run --release -- bundle --build --platform nsis
```

### On macOS Host (macOS Packages)

```bash
cd packages/bundler-release

# Build macOS packages
cargo run --release -- bundle --build --platform app
cargo run --release -- bundle --build --platform dmg
```

## Architecture

The bundler uses Rust's conditional compilation (`#[cfg(target_os = "...")]`) to include only platform-specific code:

- **Linux builds** → Compiles Linux + Windows bundlers
- **macOS builds** → Compiles macOS bundlers only

Windows installers are created in the Linux container using:
- **WiX** - Runs via Wine (.msi packages)
- **NSIS** - Runs natively on Linux (.exe packages)

## Documentation

- **Setup Guide**: [`.devcontainer/README.md`](../../.devcontainer/README.md)
- **Package Configuration**: [`src/bundler/settings.rs`](src/bundler/settings.rs)
- **Platform Modules**: [`src/bundler/platform/`](src/bundler/platform/)

## Dependencies

All required tools are pre-installed in the .devcontainer:
- Rust nightly-2024-10-20
- Wine 64-bit + .NET Framework 4.0 (for WiX)
- NSIS native Linux compiler
- Linux packaging tools (dpkg-dev, rpm, fakeroot)

See [`.devcontainer/Dockerfile`](../../.devcontainer/Dockerfile) for complete dependency list.
