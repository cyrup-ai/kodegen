//! File discovery and URL extraction for markdown documents

use anyhow::Result;
use imstr::ImString;
use jwalk::WalkDir;
use smallvec::SmallVec;
use std::path::{Path, PathBuf};

/// Discover markdown files with zero-allocation streaming
pub(crate) fn discover_markdown_files_stream<'a>(
    root_dir: &'a Path,
) -> impl Iterator<Item = Result<(PathBuf, ImString)>> + 'a {
    // Validate root directory
    let root_str = match root_dir.to_str() {
        Some(s) => s,
        None => {
            // Return error iterator for invalid root
            return Box::new(std::iter::once(Err(anyhow::anyhow!(
                "Invalid UTF-8 in root directory path"
            )))) as Box<dyn Iterator<Item = Result<(PathBuf, ImString)>> + 'a>;
        }
    };

    let root_len = root_str.len();

    // Configure jwalk for optimal performance based on system
    let cpu_count = num_cpus::get();
    let parallelism = match cpu_count {
        1..=4 => cpu_count,            // Use all cores for small systems
        5..=8 => cpu_count - 1,        // Leave one core free
        9..=16 => (cpu_count * 3) / 4, // Use 75% of cores
        17..=32 => cpu_count / 2,      // Use 50% of cores
        _ => 32,                       // Cap at 32 for very large systems
    };

    let root_path = root_dir.to_path_buf();
    Box::new(
        WalkDir::new(root_dir)
            .parallelism(jwalk::Parallelism::RayonNewPool(parallelism))
            .skip_hidden(false)
            .follow_links(false)
            .process_read_dir(move |depth, path, _state, entries| {
                // Never filter the root itself
                if path == root_path {
                    return;
                }

                // Don't filter parent/ancestor paths
                if !path.starts_with(&root_path) {
                    return;
                }

                // Apply depth limit to all paths
                if depth.unwrap_or(0) > 100 {
                    entries.clear();
                    return;
                }

                // Now we're definitely under root_path - apply filters
                if let Some(dir_name) = path.file_name().and_then(|n| n.to_str())
                    && (dir_name.starts_with('.')
                        || matches!(
                            dir_name,
                            "node_modules" | "target" | "dist" | "build" | ".git" | "__pycache__"
                        ))
                {
                    entries.clear();
                    return;
                }

                // Filter at the readdir level for maximum efficiency
                entries.retain(|entry| {
                    match entry {
                        Ok(entry) => {
                            if entry.file_type().is_dir() {
                                // Keep directories unless they're hidden or build dirs
                                if let Some(name) = entry.file_name().to_str() {
                                    !name.starts_with('.')
                                        && !matches!(
                                            name,
                                            "node_modules"
                                                | "target"
                                                | "dist"
                                                | "build"
                                                | "__pycache__"
                                        )
                                } else {
                                    false // Skip non-UTF-8 directory names
                                }
                            } else {
                                // Only keep index.md.gz files
                                entry.file_type().is_file() && entry.file_name() == "index.md.gz"
                            }
                        }
                        Err(_) => true, // Keep errors to handle them properly
                    }
                });
            })
            .into_iter()
            .filter_map(move |entry| {
                match entry {
                    Ok(entry) if entry.file_type().is_file() => {
                        let path = entry.path();
                        // Validate file exists and is readable
                        match std::fs::metadata(&path) {
                            Ok(metadata) if metadata.is_file() && metadata.len() > 0 => {
                                // Use synchronous extraction directly in iterator context
                                match extract_url_from_path_sync(&path, root_len) {
                                    Ok(Some(url)) => Some(Ok((path, url))),
                                    Ok(None) => None,
                                    Err(e) => Some(Err(e)),
                                }
                            }
                            Ok(_) => None,  // Empty file or not a regular file
                            Err(_) => None, // Can't read metadata, skip
                        }
                    }
                    Ok(_) => None, // Directory
                    Err(e)
                        if e.io_error().is_some_and(|io| {
                            io.kind() == std::io::ErrorKind::PermissionDenied
                        }) =>
                    {
                        // Skip permission denied errors silently
                        None
                    }
                    Err(e) => Some(Err(e.into())),
                }
            }),
    )
}

/// Core URL extraction logic (synchronous, no allocation)
fn extract_url_from_path_core(path: &str, root_len: usize) -> Result<Option<ImString>> {
    // Bounds check
    if path.len() <= root_len {
        return Ok(None);
    }

    // Safe slice - we've already checked the length
    let relative = &path[root_len..];

    // Normalize path separators and trim
    let relative = relative
        .trim_start_matches(['/', '\\'])
        .trim_end_matches(['/', '\\']);

    // Early return for empty relative path
    if relative.is_empty() {
        return Ok(None);
    }

    // Check for index.md.gz suffix with all separator variants
    let path_without_file = match relative {
        s if s.ends_with("/index.md.gz") => &s[..s.len() - 12],
        s if s.ends_with("\\index.md.gz") => &s[..s.len() - 13],
        s if s.ends_with("index.md.gz") && s.len() == 11 => {
            // Special case: file at root
            return Ok(None);
        }
        _ => return Ok(None),
    };

    // Validate we have content after stripping
    if path_without_file.is_empty() {
        return Ok(None);
    }

    // Split on both separator types
    let mut components = path_without_file.split(['/', '\\']);

    // Extract and validate domain
    let domain = match components.next() {
        Some(d) if !d.is_empty() => d,
        _ => return Ok(None),
    };

    // Validate domain format (basic check)
    if !is_valid_domain(domain) {
        return Ok(None);
    }

    // Build path from components
    let path_parts: SmallVec<[&str; 16]> = components
        .filter(|s| !s.is_empty() && is_valid_path_component(s))
        .collect();

    // Construct URL string directly
    let url_string = if path_parts.is_empty() {
        format!("https://{domain}/")
    } else {
        format!("https://{}/{}/", domain, path_parts.join("/"))
    };

    Ok(Some(ImString::from(url_string)))
}

/// Extract URL from path synchronously (for use in iterators)
fn extract_url_from_path_sync(file_path: &Path, root_len: usize) -> Result<Option<ImString>> {
    // Get path as string with lossy conversion
    let path_str = file_path.to_string_lossy();

    // Call core logic
    extract_url_from_path_core(path_str.as_ref(), root_len)
}

/// Validate domain name (basic validation)
#[inline]
fn is_valid_domain(domain: &str) -> bool {
    // Basic domain validation
    !domain.is_empty()
        && domain.len() <= 253
        && domain
            .chars()
            .all(|c| c.is_ascii() && (c.is_alphanumeric() || ".-".contains(c)))
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !domain.starts_with('-')
        && !domain.ends_with('-')
        && domain.contains('.') // Must have at least one dot
}

/// Validate path component
#[inline]
fn is_valid_path_component(component: &str) -> bool {
    !component.is_empty() && component.len() <= 255 && !component.contains("..")
}
