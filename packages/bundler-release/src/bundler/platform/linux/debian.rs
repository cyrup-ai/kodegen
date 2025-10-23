//! Debian package (.deb) bundler.
//!
//! Creates .deb packages as ar archives with proper Debian structure.
//!
//! A .deb file is an ar archive containing:
//! - debian-binary: Format version (2.0)
//! - control.tar.gz: Package metadata (control, md5sums, scripts)
//! - data.tar.gz: Files to install

use crate::bundler::{
    error::{Context, ErrorExt, Result},
    settings::{Arch, Settings},
    utils::fs,
};
use flate2::{write::GzEncoder, Compression};
use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};
use tar::HeaderMode;
use walkdir::WalkDir;

/// Bundle project as Debian package.
/// Returns vector with path to created .deb file.
pub fn bundle_project(settings: &Settings) -> Result<Vec<PathBuf>> {
    // Map architecture
    let arch = arch_to_debian(settings.binary_arch())?;
    
    // Create package name: {product}_{version}_{arch}.deb
    let package_base_name = format!(
        "{}_{}_{}",
        settings.product_name(),
        settings.version_string(),
        arch
    );
    let package_name = format!("{}.deb", package_base_name);
    
    // Setup directories
    let base_dir = settings.project_out_directory().join("bundle/deb");
    let package_dir = base_dir.join(&package_base_name);
    
    // Remove old package directory if it exists
    if package_dir.exists() {
        std::fs::remove_dir_all(&package_dir)
            .fs_context("removing old package directory", &package_dir)?;
    }
    
    let package_path = base_dir.join(&package_name);
    
    log::info!("Bundling {} ({})", package_name, package_path.display());
    
    // Generate data directory (binaries, resources, desktop file)
    let data_dir = generate_data(settings, &package_dir)
        .context("failed to generate data directory")?;
    
    // Copy custom files if specified
    fs::copy_custom_files(&settings.bundle_settings().deb.files, &data_dir)
        .context("failed to copy custom files")?;
    
    // Generate control directory
    let control_dir = package_dir.join("control");
    generate_control_file(settings, arch, &control_dir, &data_dir)
        .context("failed to generate control file")?;
    generate_scripts(settings, &control_dir)
        .context("failed to generate control scripts")?;
    generate_md5sums(&control_dir, &data_dir)
        .context("failed to generate md5sums file")?;
    
    // Create debian-binary file with format version
    let debian_binary_path = package_dir.join("debian-binary");
    std::fs::write(&debian_binary_path, "2.0\n")
        .fs_context("creating debian-binary file", &debian_binary_path)?;
    
    // Create tar.gz archives
    let control_tar_gz = tar_and_gzip_dir(control_dir)
        .context("failed to tar/gzip control directory")?;
    let data_tar_gz = tar_and_gzip_dir(data_dir)
        .context("failed to tar/gzip data directory")?;
    
    // Create final ar archive
    create_ar_archive(
        vec![debian_binary_path, control_tar_gz, data_tar_gz],
        &package_path,
    )
    .context("failed to create ar archive")?;
    
    Ok(vec![package_path])
}

/// Generate data directory with all files to be installed.
fn generate_data(settings: &Settings, package_dir: &Path) -> Result<PathBuf> {
    let data_dir = package_dir.join("data");
    let bin_dir = data_dir.join("usr/bin");
    
    // Copy all binaries
    for bin in settings.binaries() {
        let bin_path = settings.binary_path(bin);
        let dest = bin_dir.join(bin.name());
        fs::copy_file(&bin_path, &dest)
            .with_context(|| format!("failed to copy binary {:?}", bin_path))?;
        
        // Set executable permission on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755))
                .fs_context("setting executable permission", &dest)?;
        }
    }
    
    // Generate desktop file
    generate_desktop_file(settings, &data_dir)?;
    
    // Generate compressed changelog if provided
    generate_changelog(settings, &data_dir)?;
    
    Ok(data_dir)
}

/// Generate freedesktop.org desktop file at usr/share/applications/<name>.desktop
fn generate_desktop_file(settings: &Settings, data_dir: &Path) -> Result<()> {
    let desktop_path = data_dir
        .join("usr/share/applications")
        .join(format!("{}.desktop", settings.product_name()));
    
    let mut file = fs::create_file(&desktop_path)
        .context("failed to create desktop file")?;
    
    writeln!(file, "[Desktop Entry]")?;
    writeln!(file, "Type=Application")?;
    writeln!(file, "Name={}", settings.product_name())?;
    writeln!(file, "Exec={}", settings.product_name())?;
    writeln!(file, "Terminal=false")?;
    
    // Optional fields from settings
    if let Some(desc) = settings.bundle_settings().short_description.as_ref() {
        writeln!(file, "Comment={}", desc)?;
    }
    if let Some(category) = settings.bundle_settings().category.as_ref() {
        writeln!(file, "Categories={}", category)?;
    }
    
    file.flush()?;
    Ok(())
}

/// Generate compressed changelog at usr/share/doc/<name>/changelog.gz
fn generate_changelog(settings: &Settings, data_dir: &Path) -> Result<()> {
    if let Some(changelog_path) = &settings.bundle_settings().deb.changelog {
        let dest = data_dir.join(format!(
            "usr/share/doc/{}/changelog.gz",
            settings.product_name()
        ));
        
        let mut src = File::open(changelog_path)
            .fs_context("opening changelog file", changelog_path)?;
        let dest_file = fs::create_file(&dest)
            .context("failed to create changelog destination")?;
        let mut encoder = GzEncoder::new(dest_file, Compression::new(9));
        
        io::copy(&mut src, &mut encoder)?;
        let mut finished = encoder.finish()?;
        finished.flush()?;
    }
    Ok(())
}

/// Generate control file with package metadata.
fn generate_control_file(
    settings: &Settings,
    arch: &str,
    control_dir: &Path,
    data_dir: &Path,
) -> Result<()> {
    let control_path = control_dir.join("control");
    let mut file = fs::create_file(&control_path)
        .context("failed to create control file")?;
    
    // Package name in kebab-case
    let package = settings.product_name().to_lowercase().replace(' ', "-");
    writeln!(file, "Package: {}", package)?;
    writeln!(file, "Version: {}", settings.version_string())?;
    writeln!(file, "Architecture: {}", arch)?;
    
    // Installed size in KB
    let size_kb = calculate_dir_size(data_dir)? / 1024;
    writeln!(file, "Installed-Size: {}", size_kb)?;
    
    // Maintainer from authors or publisher
    let maintainer = settings.authors()
        .map(|a| a.join(", "))
        .or_else(|| settings.bundle_settings().publisher.clone())
        .unwrap_or_else(|| "Unknown".to_string());
    writeln!(file, "Maintainer: {}", maintainer)?;
    
    // Optional fields
    if let Some(section) = &settings.bundle_settings().deb.section {
        writeln!(file, "Section: {}", section)?;
    }
    
    if let Some(priority) = &settings.bundle_settings().deb.priority {
        writeln!(file, "Priority: {}", priority)?;
    } else {
        writeln!(file, "Priority: optional")?;
    }
    
    if let Some(homepage) = settings.homepage() {
        writeln!(file, "Homepage: {}", homepage)?;
    }
    
    // Dependencies
    if let Some(depends) = &settings.bundle_settings().deb.depends {
        writeln!(file, "Depends: {}", depends.join(", "))?;
    }
    
    // Recommends
    if let Some(recommends) = &settings.bundle_settings().deb.recommends {
        writeln!(file, "Recommends: {}", recommends.join(", "))?;
    }
    
    // Provides
    if let Some(provides) = &settings.bundle_settings().deb.provides {
        writeln!(file, "Provides: {}", provides.join(", "))?;
    }
    
    // Conflicts
    if let Some(conflicts) = &settings.bundle_settings().deb.conflicts {
        writeln!(file, "Conflicts: {}", conflicts.join(", "))?;
    }
    
    // Replaces
    if let Some(replaces) = &settings.bundle_settings().deb.replaces {
        writeln!(file, "Replaces: {}", replaces.join(", "))?;
    }
    
    // Description (required)
    let short = settings.bundle_settings().short_description
        .as_deref()
        .unwrap_or("(no description)");
    writeln!(file, "Description: {}", short)?;
    
    if let Some(long) = &settings.bundle_settings().long_description {
        for line in long.lines() {
            if line.trim().is_empty() {
                writeln!(file, " .")?;  // Debian policy for blank lines
            } else {
                writeln!(file, " {}", line.trim())?;
            }
        }
    }
    
    file.flush()?;
    Ok(())
}

/// Generate MD5 checksums for all files in data directory.
fn generate_md5sums(control_dir: &Path, data_dir: &Path) -> Result<()> {
    let md5sums_path = control_dir.join("md5sums");
    let mut file = fs::create_file(&md5sums_path)
        .context("failed to create md5sums file")?;
    
    for entry in WalkDir::new(data_dir) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        
        // Calculate MD5 hash
        let mut src = File::open(entry.path())
            .fs_context("opening file for MD5", entry.path())?;
        let mut context = md5::Context::new();
        io::copy(&mut src, &mut context)?;
        let digest = context.compute();
        
        // Write in format: "hex_digest  relative_path"
        for byte in digest.iter() {
            write!(file, "{:02x}", byte)?;
        }
        
        let rel_path = entry.path().strip_prefix(data_dir)?;
        writeln!(file, "  {}", rel_path.display())?;
    }
    
    file.flush()?;
    Ok(())
}

/// Generate maintainer scripts (preinst, postinst, prerm, postrm).
fn generate_scripts(settings: &Settings, control_dir: &Path) -> Result<()> {
    let scripts = [
        (&settings.bundle_settings().deb.pre_install_script, "preinst"),
        (&settings.bundle_settings().deb.post_install_script, "postinst"),
        (&settings.bundle_settings().deb.pre_remove_script, "prerm"),
        (&settings.bundle_settings().deb.post_remove_script, "postrm"),
    ];
    
    for (script_opt, name) in scripts {
        if let Some(script_path) = script_opt {
            let dest = control_dir.join(name);
            let mut src = File::open(script_path)
                .fs_context("opening script file", script_path)?;
            
            // Create with executable permissions
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                let mut dest_file = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .mode(0o755)
                    .open(&dest)
                    .fs_context("creating script file", &dest)?;
                
                io::copy(&mut src, &mut dest_file)?;
            }
            
            #[cfg(not(unix))]
            {
                let mut dest_file = File::create(&dest)
                    .fs_context("creating script file", &dest)?;
                io::copy(&mut src, &mut dest_file)?;
            }
        }
    }
    
    Ok(())
}

/// Create tar.gz archive from directory.
fn tar_and_gzip_dir(src_dir: PathBuf) -> Result<PathBuf> {
    let dest_path = src_dir.with_extension("tar.gz");
    let tar_gz = File::create(&dest_path)
        .fs_context("creating tar.gz file", &dest_path)?;
    let enc = GzEncoder::new(tar_gz, Compression::default());
    let mut tar = tar::Builder::new(enc);
    
    for entry in WalkDir::new(&src_dir) {
        let entry = entry?;
        let path = entry.path();
        
        if path == src_dir {
            continue;
        }
        
        let rel_path = path.strip_prefix(&src_dir)?;
        let metadata = std::fs::metadata(path)
            .fs_context("reading metadata", path)?;
        
        let mut header = tar::Header::new_gnu();
        header.set_metadata_in_mode(&metadata, HeaderMode::Deterministic);
        
        if entry.file_type().is_dir() {
            tar.append_data(&mut header, rel_path, &mut io::empty())?;
        } else {
            let mut file = File::open(path)
                .fs_context("opening file for tar", path)?;
            tar.append_data(&mut header, rel_path, &mut file)?;
        }
    }
    
    let enc = tar.into_inner()?;
    let mut finished = enc.finish()?;
    finished.flush()?;
    
    Ok(dest_path)
}

/// Create ar archive (final .deb package).
fn create_ar_archive(files: Vec<PathBuf>, dest: &Path) -> Result<()> {
    let dest_file = File::create(dest)
        .fs_context("creating .deb archive", dest)?;
    let mut builder = ar::Builder::new(dest_file);
    
    for path in &files {
        builder.append_path(path)
            .with_context(|| format!("appending {:?} to ar archive", path))?;
    }
    
    let mut finished = builder.into_inner()?;
    finished.sync_all()?;
    
    Ok(())
}

/// Map Rust architecture to Debian architecture string.
fn arch_to_debian(arch: Arch) -> Result<&'static str> {
    match arch {
        Arch::X86_64 => Ok("amd64"),
        Arch::X86 => Ok("i386"),
        Arch::AArch64 => Ok("arm64"),
        Arch::Armhf => Ok("armhf"),
        Arch::Armel => Ok("armel"),
        Arch::Riscv64 => Ok("riscv64"),
        _ => Err(crate::bundler::error::Error::ArchError(
            format!("Unsupported architecture for Debian: {:?}", arch)
        )),
    }
}

/// Calculate total size of directory in bytes.
fn calculate_dir_size(dir: &Path) -> Result<u64> {
    let mut total = 0u64;
    for entry in WalkDir::new(dir) {
        let entry = entry?;
        if entry.file_type().is_file() {
            total += entry.metadata()?.len();
        }
    }
    Ok(total)
}
