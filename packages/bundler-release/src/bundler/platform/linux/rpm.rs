//! RPM package (.rpm) bundler for Red Hat-based distributions.
//!
//! Creates RPM packages using the rpm-rs crate with proper metadata,
//! dependencies, and file structure.

use crate::bundler::{
    error::{Context, ErrorExt, Result},
    settings::{Arch, Settings},
};
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

/// Bundle project as RPM package
pub fn bundle_project(settings: &Settings) -> Result<Vec<PathBuf>> {
    // Map architecture to RPM arch strings
    let arch = match settings.binary_arch() {
        Arch::X86_64 => "x86_64",
        Arch::X86 => "i686",
        Arch::AArch64 => "aarch64",
        Arch::Armhf => "armhf",
        Arch::Armel => "armel",
        Arch::Riscv64 => "riscv64",
        _ => {
            return Err(crate::bundler::Error::ArchError(format!(
                "Unsupported architecture for RPM: {:?}",
                settings.binary_arch()
            )))
        }
    };

    log::info!("Building RPM package for {}", settings.product_name());

    // Determine license from bundle settings or use default
    let license = settings
        .bundle_settings()
        .copyright
        .as_deref()
        .unwrap_or("Unknown");

    // Get summary (short description)
    let summary = settings
        .bundle_settings()
        .short_description
        .as_deref()
        .or_else(|| Some(settings.description()))
        .unwrap_or("(no description)");

    // Configure compression
    let compression = match settings.rpm_settings().compression.as_deref() {
        Some("gzip") => rpm::CompressionType::Gzip,
        Some("xz") => rpm::CompressionType::Xz,
        Some("zstd") => rpm::CompressionType::Zstd,
        Some("bzip2") => rpm::CompressionType::Bzip2,
        _ => rpm::CompressionType::Gzip, // default
    };

    let build_config = rpm::BuildConfig::default().compression(compression);

    // Create PackageBuilder
    let mut builder = rpm::PackageBuilder::new(
        settings.product_name(),
        settings.version_string(),
        license,
        arch,
        summary,
    )
    .using_config(build_config)
    .release(settings.rpm_settings().release.clone())
    .epoch(settings.rpm_settings().epoch);

    // Set optional metadata
    if let Some(desc) = settings.bundle_settings().long_description.as_ref() {
        builder = builder.description(desc);
    }

    if let Some(homepage) = settings.bundle_settings().homepage.as_ref() {
        builder = builder.url(homepage);
    }

    if let Some(vendor) = settings.bundle_settings().publisher.as_ref() {
        builder = builder.vendor(vendor);
    }

    // Add dependencies
    if let Some(depends) = &settings.rpm_settings().depends {
        for dep_str in depends {
            // Parse dependency string (e.g., "glibc >= 2.17")
            let dep = parse_dependency(dep_str)?;
            builder = builder.requires(dep);
        }
    }

    // Add provides
    if let Some(provides) = &settings.rpm_settings().provides {
        for prov_str in provides {
            let dep = parse_dependency(prov_str)?;
            builder = builder.provides(dep);
        }
    }

    // Add conflicts
    if let Some(conflicts) = &settings.rpm_settings().conflicts {
        for conf_str in conflicts {
            let dep = parse_dependency(conf_str)?;
            builder = builder.conflicts(dep);
        }
    }

    // Add obsoletes
    if let Some(obsoletes) = &settings.rpm_settings().obsoletes {
        for obs_str in obsoletes {
            let dep = parse_dependency(obs_str)?;
            builder = builder.obsoletes(dep);
        }
    }

    // Add recommends
    if let Some(recommends) = &settings.rpm_settings().recommends {
        for rec_str in recommends {
            let dep = parse_dependency(rec_str)?;
            builder = builder.recommends(dep);
        }
    }

    // Add binaries
    for binary in settings.binaries() {
        let src_path = settings.binary_path(binary);
        let dest_path = format!("/usr/bin/{}", binary.name());

        log::debug!(
            "Adding binary: {} -> {}",
            src_path.display(),
            dest_path
        );

        // Read binary content
        let content = std::fs::read(&src_path)
            .with_context(|| format!("Failed to read binary {:?}", src_path))?;

        // Add with executable permissions
        builder = builder
            .with_file_contents(
                content,
                rpm::FileOptions::new(&dest_path)
                    .mode(rpm::FileMode::regular(0o755))
                    .user("root")
                    .group("root"),
            )?;
    }

    // Add custom files from RpmSettings
    for (dest, src) in &settings.rpm_settings().files {
        let content = std::fs::read(src)
            .with_context(|| format!("Failed to read custom file {:?}", src))?;

        builder = builder
            .with_file_contents(
                content,
                rpm::FileOptions::new(dest.to_string_lossy().as_ref())
                    .mode(rpm::FileMode::regular(0o644))
                    .user("root")
                    .group("root"),
            )?;
    }

    // Add install/uninstall scripts
    if let Some(pre_install) = &settings.rpm_settings().pre_install_script {
        let script = std::fs::read_to_string(pre_install)
            .with_context(|| format!("Failed to read pre-install script {:?}", pre_install))?;
        builder = builder.pre_install_script(script);
    }

    if let Some(post_install) = &settings.rpm_settings().post_install_script {
        let script = std::fs::read_to_string(post_install)
            .with_context(|| format!("Failed to read post-install script {:?}", post_install))?;
        builder = builder.post_install_script(script);
    }

    if let Some(pre_remove) = &settings.rpm_settings().pre_remove_script {
        let script = std::fs::read_to_string(pre_remove)
            .with_context(|| format!("Failed to read pre-remove script {:?}", pre_remove))?;
        builder = builder.pre_uninstall_script(script);
    }

    if let Some(post_remove) = &settings.rpm_settings().post_remove_script {
        let script = std::fs::read_to_string(post_remove)
            .with_context(|| format!("Failed to read post-remove script {:?}", post_remove))?;
        builder = builder.post_uninstall_script(script);
    }

    // Build the package
    let pkg = builder
        .build()
        .context("Failed to build RPM package")?;

    // Create output directory
    let output_dir = settings.project_out_directory().join("bundle/rpm");
    std::fs::create_dir_all(&output_dir)
        .fs_context("creating RPM output directory", &output_dir)?;

    // Construct package filename
    let package_name = format!(
        "{}-{}-{}.{}.rpm",
        settings.product_name(),
        settings.version_string(),
        settings.rpm_settings().release,
        arch
    );

    let output_path = output_dir.join(package_name);

    // Write RPM to disk
    let mut file = File::create(&output_path)
        .fs_context("creating RPM file", &output_path)?;

    pkg.write(&mut file)
        .context("Failed to write RPM package")?;

    file.flush()
        .fs_context("flushing RPM file", &output_path)?;

    log::info!("✓ Created RPM: {}", output_path.display());

    Ok(vec![output_path])
}

/// Parse a dependency string into an rpm::Dependency
///
/// Supports formats:
/// - "package-name" -> any version
/// - "package-name >= 1.0.0" -> version constraint
/// - "package-name = 2.0.0" -> exact version
fn parse_dependency(dep_str: &str) -> Result<rpm::Dependency> {
    let parts: Vec<&str> = dep_str.split_whitespace().collect();

    match parts.len() {
        1 => {
            // Just a package name, any version
            Ok(rpm::Dependency::any(parts[0]))
        }
        3 => {
            // Package name, operator, version
            let name = parts[0];
            let op = parts[1];
            let version = parts[2];

            match op {
                "=" | "==" => Ok(rpm::Dependency::eq(name, version)),
                ">=" => Ok(rpm::Dependency::greater_eq(name, version)),
                ">" => Ok(rpm::Dependency::greater(name, version)),
                "<=" => Ok(rpm::Dependency::less_eq(name, version)),
                "<" => Ok(rpm::Dependency::less(name, version)),
                _ => Err(crate::bundler::Error::GenericError(format!(
                    "Unknown dependency operator: {}",
                    op
                ))),
            }
        }
        _ => Err(crate::bundler::Error::GenericError(format!(
            "Invalid dependency format: {}. Expected 'package' or 'package OP version'",
            dep_str
        ))),
    }
}

/// Extension trait for accessing RPM settings
trait SettingsExt {
    fn rpm_settings(&self) -> &crate::bundler::settings::RpmSettings;
}

impl SettingsExt for Settings {
    fn rpm_settings(&self) -> &crate::bundler::settings::RpmSettings {
        &self.bundle_settings().rpm
    }
}
