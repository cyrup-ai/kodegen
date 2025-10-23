//! macOS bundling support for .app bundles and DMG disk images.
//!
//! This module provides bundling implementations for macOS application distribution,
//! including app bundle creation, DMG packaging, and code signing integration.
//!
//! # Supported Formats
//!
//! - **Application Bundle (.app)**: via [`app`] module
//! - **Disk Image (.dmg)**: via [`dmg`] module
//!
//! # Build Requirements
//!
//! | Format | Required Tools | Notes |
//! |--------|----------------|-------|
//! | .app | Xcode Command Line Tools | Info.plist generation |
//! | .dmg | `hdiutil` | Built into macOS |
//! | Code Signing | `codesign`, Developer ID | Optional but recommended |
//!
//! # Output Location
//!
//! Bundles are created in `target/release/bundle/`:
//! - `bundle/macos/MyApp.app` - Application bundle
//! - `bundle/dmg/MyApp_1.0.0.dmg` - Disk image
//!
//! # Code Signing
//!
//! The [`sign`] module provides code signing support. For automated certificate
//! provisioning and comprehensive signing setup, see the
//! [`kodegen_sign`](../../../../sign/index.html) crate.
//!
//! # Icon Conversion
//!
//! The [`icon`] module handles PNG to ICNS conversion for macOS app icons.
//!
//! # Minimum macOS Version
//!
//! Configure the minimum supported macOS version in bundle settings:
//!
//! ```toml
//! [package.metadata.bundle.macos]
//! minimum_system_version = "10.15"
//! ```

pub mod app;
pub mod dmg;
pub mod icon;
pub mod sign;
