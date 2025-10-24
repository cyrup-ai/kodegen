use anyhow::{Context, Result};
use chromiumoxide::fetcher::{BrowserFetcher, BrowserFetcherOptions};
use std::path::PathBuf;
use std::process::Command;
use tracing::{info, warn};

/// Find Chrome/Chromium executable on the system with platform-specific search paths.
pub async fn find_browser_executable() -> Result<PathBuf> {
    // First check environment variable which overrides all other methods
    if let Ok(path) = std::env::var("CHROMIUM_PATH") {
        let path = PathBuf::from(path);
        if path.exists() {
            info!(
                "Using browser from CHROMIUM_PATH environment variable: {}",
                path.display()
            );
            return Ok(path);
        }
        warn!(
            "CHROMIUM_PATH environment variable points to non-existent file: {}",
            path.display()
        );
    }

    // Common Chrome/Chromium installation paths by platform
    let paths = if cfg!(target_os = "windows") {
        vec![
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
            r"%PROGRAMFILES%\Google\Chrome\Application\chrome.exe",
            r"%PROGRAMFILES(X86)%\Google\Chrome\Application\chrome.exe",
            r"%LOCALAPPDATA%\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files\Chromium\Application\chrome.exe",
            r"C:\Program Files (x86)\Chromium\Application\chrome.exe",
        ]
    } else if cfg!(target_os = "macos") {
        vec![
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "/Applications/Google Chrome Beta.app/Contents/MacOS/Google Chrome Beta",
            "/Applications/Google Chrome Dev.app/Contents/MacOS/Google Chrome Dev",
            "/Applications/Google Chrome Canary.app/Contents/MacOS/Google Chrome Canary",
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
            "~/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "~/Applications/Chromium.app/Contents/MacOS/Chromium",
            "/opt/homebrew/bin/chromium",
        ]
    } else {
        // Linux
        vec![
            "/usr/bin/google-chrome",
            "/usr/bin/google-chrome-stable",
            "/usr/bin/chromium",
            "/usr/bin/chromium-browser",
            "/snap/bin/chromium",
            "/usr/local/bin/chromium",
            "/opt/google/chrome/chrome",
        ]
    };

    // Try each path
    for path_str in paths {
        let path = if path_str.starts_with('~') {
            // Expand home directory if path starts with ~
            if let Some(home) = dirs::home_dir() {
                home.join(&path_str[2..])
            } else {
                continue;
            }
        } else if path_str.contains('%') && cfg!(target_os = "windows") {
            // Expand environment variables on Windows (%VAR% tokens)
            let expanded = expand_windows_env_vars(path_str);
            PathBuf::from(expanded)
        } else {
            PathBuf::from(path_str)
        };

        if path.exists() {
            info!("Found browser at: {}", path.display());
            return Ok(path);
        }
    }

    // Use 'which' command to find Chromium on Unix systems
    if !cfg!(target_os = "windows") {
        for cmd in &["chromium", "chromium-browser", "google-chrome", "chrome"] {
            let output = Command::new("which").arg(cmd).output();

            if let Ok(output) = output
                && output.status.success()
            {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path_str.is_empty() {
                    let path = PathBuf::from(path_str);
                    info!("Found browser using 'which' command: {}", path.display());
                    return Ok(path);
                }
            }
        }
    }

    // No browser found, inform user
    warn!("No Chrome/Chromium executable found. Will download and use fetcher.");
    Err(anyhow::anyhow!("Chrome/Chromium executable not found"))
}

/// Expand Windows environment variables in the form %VAR% within a path string.
///
/// Iterates through the string, finding %VAR% patterns and replacing them with
/// their environment variable values. If a variable doesn't exist, the original
/// %VAR% token is preserved.
///
/// # Arguments
/// * `path` - Path string potentially containing %VAR% tokens
///
/// # Returns
/// Expanded path string with all available environment variables substituted
///
/// # Example
/// ```
/// let path = "%PROGRAMFILES%\\Google\\Chrome\\Application\\chrome.exe";
/// let expanded = expand_windows_env_vars(path);
/// // Result: "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe"
/// ```
fn expand_windows_env_vars(path: &str) -> String {
    let mut result = String::with_capacity(path.len());
    let mut chars = path.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '%' {
            // Found start of potential environment variable
            let var_name: String = chars.by_ref().take_while(|&c| c != '%').collect();

            if !var_name.is_empty() {
                // Try to expand the variable
                if let Ok(value) = std::env::var(&var_name) {
                    result.push_str(&value);
                } else {
                    // Variable not found, preserve original %VAR% token
                    result.push('%');
                    result.push_str(&var_name);
                    result.push('%');
                }
            } else {
                // Empty %% sequence, keep single %
                result.push('%');
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Downloads and manages Chromium browser if not found locally.
/// Returns a path to the downloaded executable.
pub async fn download_managed_browser() -> Result<PathBuf> {
    info!("Downloading managed Chromium browser...");

    // Create cache directory for downloaded browser
    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| {
            let fallback = std::env::temp_dir().join(".cache");
            warn!(
                "Could not determine system cache directory, using temp directory fallback: {}",
                fallback.display()
            );
            fallback
        })
        .join("enigo/chromium");

    std::fs::create_dir_all(&cache_dir).context("Failed to create cache directory")?;

    // Use fetcher to download Chrome
    let fetcher = BrowserFetcher::new(
        BrowserFetcherOptions::builder()
            .with_path(&cache_dir)
            .build()
            .context("Failed to build fetcher options")?,
    );

    // Download Chrome
    let revision_info = fetcher.fetch().await.context("Failed to fetch browser")?;

    info!(
        "Downloaded Chromium to: {}",
        revision_info.folder_path.display()
    );

    Ok(revision_info.executable_path)
}
