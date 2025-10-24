//! File system utilities for bundling.
//!
//! Provides safe file operations with automatic directory creation,
//! symlink preservation, and comprehensive error handling.

use crate::bundler::error::Result;
use std::{
    fs::{self},
    io::{self},
    path::Path,
};

#[cfg(target_os = "linux")]
use std::fs::File;

#[cfg(target_os = "linux")]
use std::io::BufWriter;

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use std::{collections::HashMap, path::PathBuf};

/// Creates a new file at the given path, creating any parent directories as needed.
///
/// Returns a `BufWriter` for efficient writing operations.
#[cfg(target_os = "linux")]
pub fn create_file(path: &Path) -> Result<BufWriter<File>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = File::create(path)?;
    Ok(BufWriter::new(file))
}

/// Creates the given directory path, erasing it first if specified.
#[allow(dead_code)]
pub fn create_dir(path: &Path, erase: bool) -> Result<()> {
    if erase && path.exists() {
        remove_dir_all(path)?;
    }
    Ok(fs::create_dir(path)?)
}

/// Creates all of the directories of the specified path, erasing it first if specified.
#[allow(dead_code)]
pub fn create_dir_all(path: &Path, erase: bool) -> Result<()> {
    if erase && path.exists() {
        remove_dir_all(path)?;
    }
    Ok(fs::create_dir_all(path)?)
}

/// Removes the directory and its contents if it exists.
#[allow(dead_code)]
pub fn remove_dir_all(path: &Path) -> Result<()> {
    if path.exists() {
        Ok(fs::remove_dir_all(path)?)
    } else {
        Ok(())
    }
}

/// Makes a symbolic link to a directory.
#[cfg(unix)]
#[allow(dead_code)]
fn symlink_dir(src: &Path, dst: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(src, dst)
}

/// Makes a symbolic link to a directory.
#[cfg(windows)]
fn symlink_dir(src: &Path, dst: &Path) -> io::Result<()> {
    std::os::windows::fs::symlink_dir(src, dst)
}

/// Makes a symbolic link to a file.
#[cfg(unix)]
#[allow(dead_code)]
fn symlink_file(src: &Path, dst: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(src, dst)
}

/// Makes a symbolic link to a file.
#[cfg(windows)]
fn symlink_file(src: &Path, dst: &Path) -> io::Result<()> {
    std::os::windows::fs::symlink_file(src, dst)
}

/// Copies a regular file from one path to another, creating any parent
/// directories of the destination path as necessary.
///
/// Fails if the source path is a directory or doesn't exist.
pub fn copy_file(from: &Path, to: &Path) -> Result<()> {
    if !from.exists() {
        return Err(crate::bundler::error::Error::GenericError(format!(
            "{from:?} does not exist"
        )));
    }
    if !from.is_file() {
        return Err(crate::bundler::error::Error::GenericError(format!(
            "{from:?} is not a file"
        )));
    }
    if let Some(dest_dir) = to.parent() {
        fs::create_dir_all(dest_dir)?;
    }
    fs::copy(from, to)?;
    Ok(())
}

/// Recursively copies a directory from one path to another, creating any
/// parent directories of the destination path as necessary.
///
/// Preserves symlinks on platforms that support them.
/// Fails if the source path is not a directory or doesn't exist,
/// or if the destination path already exists.
#[allow(dead_code)]
pub fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    if !from.exists() {
        return Err(crate::bundler::error::Error::GenericError(format!(
            "{from:?} does not exist"
        )));
    }
    if !from.is_dir() {
        return Err(crate::bundler::error::Error::GenericError(format!(
            "{from:?} is not a Directory"
        )));
    }
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }

    for entry in walkdir::WalkDir::new(from) {
        let entry = entry?;
        debug_assert!(entry.path().starts_with(from));
        let rel_path = entry.path().strip_prefix(from)?;
        let dest_path = to.join(rel_path);

        if entry.file_type().is_symlink() {
            let target = fs::read_link(entry.path())?;
            if entry.path().is_dir() {
                symlink_dir(&target, &dest_path)?;
            } else {
                symlink_file(&target, &dest_path)?;
            }
        } else if entry.file_type().is_dir() {
            fs::create_dir_all(dest_path)?;
        } else {
            fs::copy(entry.path(), dest_path)?;
        }
    }

    Ok(())
}

/// Copies user-defined files specified in the configuration file to the package.
///
/// The configuration object maps the path in the package to the path of the file on the filesystem.
///
/// Expects a HashMap of PathBuf entries, representing destination and source paths,
/// and also a path of a directory. The files will be stored with respect to this directory.
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
pub fn copy_custom_files(files_map: &HashMap<PathBuf, PathBuf>, data_dir: &Path) -> Result<()> {
    for (pkg_path, path) in files_map.iter() {
        let pkg_path = if pkg_path.is_absolute() {
            pkg_path.strip_prefix("/")?
        } else {
            pkg_path
        };
        if path.is_file() {
            copy_file(path, &data_dir.join(pkg_path))?;
        } else {
            copy_dir(path, &data_dir.join(pkg_path))?;
        }
    }
    Ok(())
}
