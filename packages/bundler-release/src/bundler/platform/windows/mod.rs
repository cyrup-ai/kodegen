//! Windows bundling support for MSI and NSIS installers.
//!
//! This module provides bundling implementations for Windows installer creation,
//! including WiX MSI packages and NSIS executables.
//!
//! # Supported Formats
//!
//! - **MSI Installer (.msi)**: via [`msi`] module using WiX Toolset
//! - **NSIS Installer (.exe)**: via [`nsis`] module using NSIS
//!
//! # Build Requirements
//!
//! | Format | Required Tools | Download |
//! |--------|----------------|----------|
//! | .msi | WiX Toolset 3.x or 4.x | Auto-downloaded by bundler |
//! | .exe (NSIS) | NSIS 3.x | Auto-downloaded by bundler |
//! | Code Signing | `osslsigncode` or `signtool.exe` | Optional |
//!
//! # Output Location
//!
//! Bundles are created in `target/release/bundle/`:
//! - `bundle/msi/MyApp_1.0.0_x64.msi` - MSI installer
//! - `bundle/nsis/MyApp_1.0.0_x64-setup.exe` - NSIS installer
//!
//! # Code Signing
//!
//! The [`sign`] module provides Authenticode code signing support using osslsigncode.
//! For comprehensive signing setup, see the
//! [`kodegen_sign`](../../../../sign/index.html) crate.
//!
//! # Icon Conversion
//!
//! The [`icon`] module (Windows-only) handles PNG to ICO conversion for Windows icons.
//!
//! # Installer Customization
//!
//! Both MSI and NSIS support extensive customization:
//!
//! ```toml
//! [package.metadata.bundle.windows.wix]
//! language = ["en-US"]
//! license = "LICENSE.rtf"
//!
//! [package.metadata.bundle.windows.nsis]
//! installer_mode = "perMachine"
//! compression = "lzma"
//! ```

#[cfg(windows)]
pub mod icon;
#[cfg(any(target_os = "windows", target_os = "linux"))]
pub mod msi;
#[cfg(any(target_os = "windows", target_os = "linux"))]
pub mod nsis;
#[cfg(any(target_os = "windows", target_os = "linux"))]
pub mod sign;
pub mod util;
