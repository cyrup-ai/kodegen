//! Linux bundling support for .deb, .rpm, and AppImage formats.
//!
//! This module provides bundling implementations for the major Linux package formats,
//! supporting a wide range of distributions.
//!
//! # Supported Distributions
//!
//! - **Debian/Ubuntu**: `.deb` packages via [`debian`] module
//! - **Fedora/RHEL/CentOS**: `.rpm` packages via [`rpm`] module
//! - **Universal**: AppImage portable format via [`appimage`] module
//!
//! # Build Requirements
//!
//! | Format | Required Tools |
//! |--------|----------------|
//! | .deb | `dpkg-deb`, `ar` (or Rust `ar` crate) |
//! | .rpm | `rpm-build` (or Rust `rpm` crate) |
//! | AppImage | `linuxdeploy`, `appimagetool` |
//!
//! # Output Location
//!
//! Bundles are created in `target/release/bundle/`:
//! - `bundle/deb/package_1.0.0_amd64.deb`
//! - `bundle/rpm/package-1.0.0-1.x86_64.rpm`
//! - `bundle/appimage/package_1.0.0_amd64.AppImage`
//!
//! # Desktop Integration
//!
//! The [`freedesktop`] module provides FreeDesktop.org specification support
//! for `.desktop` files, icons, and MIME types.

pub mod appimage;
pub mod debian;
pub mod freedesktop;
pub mod rpm;
