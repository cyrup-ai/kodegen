use kodegen_mcp_tool::error::McpError;
use log::warn;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::time::{Duration, timeout};

/// Normalize all paths consistently
fn normalize_path(p: &str) -> String {
    expand_home(p).to_lowercase()
}

/// Expand home directory (~) in file paths
fn expand_home(filepath: &str) -> String {
    if (filepath.starts_with("~/") || filepath == "~")
        && let Some(home_dir) = dirs::home_dir()
    {
        return home_dir.join(&filepath[1..]).to_string_lossy().to_string();
    }
    filepath.to_string()
}

/// Recursively validates parent directories until it finds a valid one
fn validate_parent_directories(
    directory_path: &Path,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send + '_>> {
    Box::pin(async move {
        if let Some(parent_dir) = directory_path.parent() {
            // Base case: we've reached the root
            if parent_dir == directory_path {
                return false;
            }

            // Check if the parent directory exists
            if fs::metadata(parent_dir).await.is_ok() {
                return true;
            }

            // Parent doesn't exist, recursively check its parent
            return validate_parent_directories(parent_dir).await;
        }
        false
    })
}

/// Get the list of allowed directories from config
fn get_allowed_dirs(config: &kodegen_tools_config::ServerConfig) -> &[String] {
    &config.allowed_directories
}

/// Get the list of denied directories from config
fn get_denied_dirs(config: &kodegen_tools_config::ServerConfig) -> &[String] {
    &config.denied_directories
}

/// Returns (`is_allowed`, `restriction_reason`)
/// `restriction_reason` is `Some(message)` if denied, `None` if allowed
fn is_path_allowed(
    path_to_check: &str,
    config: &kodegen_tools_config::ServerConfig,
) -> (bool, Option<String>) {
    let allowed_dirs = get_allowed_dirs(config);
    let denied_dirs = get_denied_dirs(config);

    let mut normalized_path_to_check = normalize_path(path_to_check);
    if normalized_path_to_check.ends_with(std::path::MAIN_SEPARATOR) {
        normalized_path_to_check.pop();
    }

    // STEP 1: Check denied list first (blacklist takes precedence)
    if !denied_dirs.is_empty() {
        let denied_match = denied_dirs.iter().find(|denied_dir| {
            let mut normalized_denied = normalize_path(denied_dir);
            if normalized_denied.ends_with(std::path::MAIN_SEPARATOR) {
                normalized_denied.pop();
            }

            // Exact match or subdirectory
            normalized_path_to_check == normalized_denied
                || normalized_path_to_check.starts_with(&format!(
                    "{}{}",
                    normalized_denied,
                    std::path::MAIN_SEPARATOR
                ))
        });

        if let Some(denied_dir) = denied_match {
            let reason = format!(
                "Path is in denied directory: {denied_dir}\n\
                 Current denied directories: {denied_dirs:?}\n\
                 \n\
                 To modify restrictions:\n\
                 1. Remove from blacklist: unset KODEGEN_DENIED_DIRS or use set_config_value tool\n\
                 2. Or add to whitelist: export KODEGEN_ALLOWED_DIRS=\"{path_to_check}:$KODEGEN_ALLOWED_DIRS\""
            );
            return (false, Some(reason));
        }
    }

    // STEP 2: Check whitelist (if non-empty)
    if !allowed_dirs.is_empty() {
        // If root directory is allowed, all paths are allowed
        if allowed_dirs.contains(&"/".to_string()) {
            return (true, None);
        }

        let is_allowed = allowed_dirs.iter().any(|allowed_dir| {
            let mut normalized_allowed = normalize_path(allowed_dir);
            if normalized_allowed.ends_with(std::path::MAIN_SEPARATOR) {
                normalized_allowed.pop();
            }

            // Exact match
            if normalized_path_to_check == normalized_allowed {
                return true;
            }

            // Subdirectory check
            let subdir_check = normalized_path_to_check.starts_with(&format!(
                "{}{}",
                normalized_allowed,
                std::path::MAIN_SEPARATOR
            ));

            // Windows drive check
            if cfg!(windows) && normalized_allowed.ends_with(':') {
                return normalized_path_to_check.starts_with(&normalized_allowed);
            }

            subdir_check
        });

        if !is_allowed {
            let reason = format!(
                "Path not in allowed directories\n\
                 Current allowed directories: {allowed_dirs:?}\n\
                 \n\
                 To allow access to this path:\n\
                 1. Via environment variable: export KODEGEN_ALLOWED_DIRS=\"{path_to_check}:$KODEGEN_ALLOWED_DIRS\"\n\
                 2. Via MCP tool: set_config_value({{\"key\": \"allowed_directories\", \"value\": [\"{path_to_check}\", ...]}})\n\
                 3. Or allow all: export KODEGEN_ALLOWED_DIRS=\"/\""
            );
            return (false, Some(reason));
        }

        return (true, None);
    }

    // STEP 3: No restrictions = allow all
    (true, None)
}

/// Validates a path to ensure it can be accessed or created
///
/// # Errors
/// Returns error if path is denied, validation times out, or parent directories are invalid
pub async fn validate_path(
    requested_path: &str,
    config_manager: &kodegen_tools_config::ConfigManager,
) -> Result<PathBuf, McpError> {
    const PATH_VALIDATION_TIMEOUT: u64 = 10000; // 10 seconds

    let validation_operation = async {
        // Get current config
        let config = config_manager.get_config();

        // Expand home directory if present
        let expanded_path = expand_home(requested_path);

        // Convert to absolute path
        let absolute = if Path::new(&expanded_path).is_absolute() {
            PathBuf::from(&expanded_path)
        } else {
            std::env::current_dir()
                .map_err(McpError::Io)?
                .join(&expanded_path)
        };

        // Check if path is allowed (get detailed error)
        let (is_allowed, restriction_reason) =
            is_path_allowed(&absolute.to_string_lossy(), &config);
        if !is_allowed {
            let error_msg =
                restriction_reason.unwrap_or_else(|| format!("Path not allowed: {requested_path}"));
            warn!("Path access denied: {requested_path}");
            return Err(McpError::PermissionDenied(error_msg));
        }

        // Check if path exists
        match fs::metadata(&absolute).await {
            Ok(_) => {
                // If path exists, resolve any symlinks
                match fs::canonicalize(&absolute).await {
                    Ok(canonical) => Ok(canonical),
                    Err(_) => Ok(absolute), // Fall back to absolute path
                }
            }
            Err(_) => {
                // Path doesn't exist - validate parent directories
                if validate_parent_directories(&absolute).await {
                    Ok(absolute)
                } else {
                    // Return the absolute path anyway for operations that create paths
                    Ok(absolute)
                }
            }
        }
    };

    // Execute with timeout
    if let Ok(result) = timeout(
        Duration::from_millis(PATH_VALIDATION_TIMEOUT),
        validation_operation,
    )
    .await
    {
        result
    } else {
        warn!("Path validation timeout for: {requested_path}");
        Err(McpError::Other(anyhow::anyhow!(
            "Path validation timeout after 10 seconds"
        )))
    }
}
