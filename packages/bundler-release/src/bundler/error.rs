//! Error types for bundler operations.
//!
//! Provides comprehensive error handling with contextual error chaining,
//! filesystem-specific errors, and platform-specific error variants.
//!
//! # Features
//!
//! - **Context trait**: Add context to errors similar to anyhow
//! - **ErrorExt trait**: Filesystem operations with automatic path context
//! - **bail! macro**: Early return with formatted error messages
//! - **Platform-specific errors**: Conditional compilation for OS-specific variants
//!
//! # Example
//!
//! ```no_run
//! # use std::path::{Path, PathBuf};
//! # use serde::Deserialize;
//! # type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
//! # 
//! # // Mock ErrorExt trait
//! # trait ErrorExt<T> {
//! #     fn fs_context(self, context: &'static str, path: impl Into<PathBuf>) -> Result<T>;
//! # }
//! # impl<T> ErrorExt<T> for std::result::Result<T, std::io::Error> {
//! #     fn fs_context(self, context: &'static str, path: impl Into<PathBuf>) -> Result<T> {
//! #         self.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
//! #     }
//! # }
//! #
//! # // Mock Context trait
//! # trait Context<T> {
//! #     fn context<C: std::fmt::Display>(self, context: C) -> Result<T>;
//! # }
//! # impl<T, E: std::error::Error + 'static> Context<T> for std::result::Result<T, E> {
//! #     fn context<C: std::fmt::Display>(self, context: C) -> Result<T> {
//! #         self.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
//! #     }
//! # }
//! #
//! # // Mock bail macro
//! # macro_rules! bail {
//! #     ($msg:expr) => { return Err($msg.into()) };
//! # }
//! #
//! #[derive(Deserialize)]
//! struct Config {
//!     app_name: Option<String>,
//! }
//!
//! impl Config {
//!     fn is_valid(&self) -> bool {
//!         self.app_name.is_some()
//!     }
//! }
//!
//! fn read_config(path: &Path) -> Result<Config> {
//!     let contents = std::fs::read_to_string(path)
//!         .fs_context("reading config file", path)?;
//!     
//!     let config: Config = serde_json::from_str(&contents)
//!         .context("parsing config JSON")?;
//!     
//!     if !config.is_valid() {
//!         bail!("invalid config: missing required field 'app_name'");
//!     }
//!     
//!     Ok(config)
//! }
//! ```

use std::{
    fmt::Display,
    io, num,
    path::{self, PathBuf},
};
use thiserror::Error as DeriveError;

/// Errors returned by the bundler.
///
/// This enum covers all error conditions that can occur during bundling,
/// including I/O errors, platform-specific errors, and errors from external crates.
#[derive(Debug, DeriveError)]
#[non_exhaustive]
pub enum Error {
    /// Error with context. Created by the [`Context`] trait.
    ///
    /// Allows wrapping errors with additional context strings for better debugging.
    #[error("{0}: {1}")]
    Context(String, Box<Self>),

    /// File system error with path context.
    ///
    /// Automatically includes the path that caused the error for better diagnostics.
    /// Created by the [`ErrorExt`] trait's `fs_context` method.
    #[error("{context} {path}: {error}")]
    Fs {
        /// Context describing the operation (e.g., "reading config file")
        context: &'static str,
        /// Path that was being accessed
        path: PathBuf,
        /// The underlying I/O error
        error: io::Error,
    },

    /// Child process execution error.
    ///
    /// Used when external commands fail (e.g., code signing tools, installers).
    #[error("failed to run command {command}: {error}")]
    CommandFailed {
        /// Command that failed to execute
        command: String,
        /// The underlying error
        error: io::Error,
    },

    /// Generic I/O error.
    #[error("{0}")]
    IoError(#[from] io::Error),

    /// Image processing error (icon conversion, resizing).
    #[error("{0}")]
    ImageError(#[from] image::ImageError),

    /// Error walking directory (used in resource bundling).
    #[error("{0}")]
    WalkdirError(#[from] walkdir::Error),

    /// Path prefix stripping error.
    #[error("{0}")]
    StripError(#[from] path::StripPrefixError),

    /// Number conversion error (e.g., version components).
    #[error("{0}")]
    ConvertError(#[from] num::TryFromIntError),

    /// ZIP archive creation/extraction error.
    #[error("{0}")]
    ZipError(#[from] zip::result::ZipError),

    /// Hex encoding/decoding error (checksums).
    #[error("{0}")]
    HexError(#[from] hex::FromHexError),

    /// Handlebars template rendering error.
    #[error("{0}")]
    HandleBarsError(#[from] handlebars::RenderError),

    /// Handlebars template parsing error.
    #[error("{0}")]
    Template(#[from] handlebars::TemplateError),

    /// JSON serialization/deserialization error.
    #[error("{0}")]
    JsonError(#[from] serde_json::error::Error),

    /// Regular expression error (macOS/Windows code signing).
    #[cfg(any(target_os = "macos", windows))]
    #[error("{0}")]
    RegexError(#[from] regex::Error),

    /// HTTP request error (downloading resources).
    #[error("{0}")]
    HttpError(#[from] Box<ureq::Error>),

    /// Invalid glob pattern (Windows file matching).
    #[cfg(windows)]
    #[error("{0}")]
    GlobPattern(#[from] glob::PatternError),

    /// Glob execution error (Windows file matching).
    #[cfg(windows)]
    #[error("{0}")]
    Glob(#[from] glob::GlobError),

    /// URL parsing error.
    #[error("{0}")]
    UrlParse(#[from] url::ParseError),

    /// Hash mismatch for downloaded files.
    #[error("hash mismatch of downloaded file: expected {expected}, got {actual}")]
    HashMismatch {
        /// Expected hash value
        expected: String,
        /// Actual hash value
        actual: String,
    },

    /// Binary parsing error (PE/Mach-O/ELF analysis).
    #[error("binary parse error: {0}")]
    BinaryParseError(#[from] goblin::error::Error),

    /// Package type not supported on target platform.
    #[error("package type {package_type} not supported on {platform}")]
    InvalidPackageType {
        /// The requested package type
        package_type: String,
        /// The current platform
        platform: String,
    },

    /// Unsupported CPU architecture.
    #[error("unsupported architecture: {0}")]
    ArchError(String),

    /// Required icon paths not found in configuration.
    #[error("could not find icon paths in bundle configuration")]
    IconPathError,

    /// Background image not found (DMG creation).
    #[error("could not find background file in bundle configuration")]
    BackgroundPathError,

    /// Generic error with custom message.
    #[error("{0}")]
    GenericError(String),

    /// No bundled project found for updater generation.
    #[error("unable to find a bundled project for the updater")]
    UnableToFindProject,

    /// String is not valid UTF-8.
    #[error("string is not UTF-8")]
    Utf8(#[from] std::str::Utf8Error),

    /// Semantic version parsing error.
    #[error("{0}")]
    SemverError(#[from] semver::Error),

    // ============= Platform-Specific Errors =============

    /// Windows SignTool not found in Windows SDK.
    #[cfg(windows)]
    #[error("SignTool not found in Windows SDK")]
    SignToolNotFound,

    /// Failed to open Windows registry.
    #[cfg(windows)]
    #[error("failed to open registry {0}")]
    OpenRegistry(String),

    /// Failed to get Windows registry value.
    #[cfg(windows)]
    #[error("failed to get {0} value from registry")]
    GetRegistryValue(String),

    /// Failed to enumerate Windows registry keys.
    #[cfg(windows)]
    #[error("failed to enumerate registry keys")]
    FailedToEnumerateRegKeys,

    /// Unsupported OS bitness (Windows).
    #[cfg(windows)]
    #[error("unsupported OS bitness")]
    UnsupportedBitness,

    /// Application signing failed (all platforms).
    #[error("failed to sign app: {0}")]
    Sign(String),

    /// Time handling error (macOS notarization timestamps).
    #[cfg(target_os = "macos")]
    #[error("{0}")]
    TimeError(#[from] time::error::Error),

    /// Property list (plist) parsing/writing error.
    #[cfg(target_os = "macos")]
    #[error("{0}")]
    Plist(#[from] plist::Error),

    /// RPM package creation error.
    #[cfg(target_os = "linux")]
    #[error("{0}")]
    RpmError(#[from] rpm::Error),

    /// macOS notarization failed.
    #[cfg(target_os = "macos")]
    #[error("failed to notarize app: {0}")]
    AppleNotarization(#[from] NotarizeAuthError),
}

/// macOS notarization authentication errors.
///
/// Provides clear error messages for missing Apple Developer credentials.
#[cfg(target_os = "macos")]
#[derive(Debug, thiserror::Error)]
pub enum NotarizeAuthError {
    /// Team ID required for app-specific password authentication.
    #[error(
        "The team ID is now required for notarization with app-specific password. \
         Please set the APPLE_TEAM_ID environment variable. \
         You can find your team ID at https://developer.apple.com/account#MembershipDetailsCard"
    )]
    TeamId,

    /// API key file not found at expected path.
    #[error("could not find API key file {file_name}. Please set APPLE_API_KEY_PATH")]
    ApiKey {
        /// Expected filename for the API key
        file_name: String,
    },

    /// No notarization credentials found.
    #[error(
        "no notarization credentials found. Please set either: \
         (1) APPLE_ID, APPLE_PASSWORD, and APPLE_TEAM_ID or \
         (2) APPLE_API_KEY, APPLE_API_ISSUER, and APPLE_API_KEY_PATH"
    )]
    Credentials,
}

/// Convenient type alias for Result.
pub type Result<T> = std::result::Result<T, Error>;

/// Trait for adding context to errors.
///
/// Similar to `anyhow::Context` but integrated with bundler's Error type.
/// Works with both `Result<T, E>` and `Option<T>`.
///
/// # Examples
///
/// ```
/// # use std::path::PathBuf;
/// # type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
/// # 
/// # // Mock Context trait
/// # trait Context<T> {
/// #     fn context<C: std::fmt::Display>(self, context: C) -> Result<T>;
/// # }
/// # impl<T> Context<T> for Option<T> {
/// #     fn context<C: std::fmt::Display>(self, context: C) -> Result<T> {
/// #         self.ok_or_else(|| context.to_string().into())
/// #     }
/// # }
/// #
/// struct Binary {
///     path: PathBuf,
/// }
///
/// impl Binary {
///     fn is_main(&self) -> bool {
///         true
///     }
/// }
///
/// fn find_all_binaries() -> Result<Vec<Binary>> {
///     Ok(vec![])
/// }
///
/// fn find_binary() -> Result<PathBuf> {
///     let binaries = find_all_binaries()?;
///     
///     Ok(binaries
///         .into_iter()
///         .find(|b| b.is_main())
///         .context("no main binary found in configuration")?
///         .path)
/// }
/// ```
pub trait Context<T> {
    /// Add context to an error.
    fn context<C>(self, context: C) -> Result<T>
    where
        C: Display + Send + Sync + 'static;

    /// Add context to an error using a closure (lazy evaluation).
    ///
    /// Use this when context string construction is expensive.
    fn with_context<C, F>(self, f: F) -> Result<T>
    where
        C: Display + Send + Sync + 'static,
        F: FnOnce() -> C;
}

impl<T> Context<T> for Result<T> {
    fn context<C>(self, context: C) -> Result<T>
    where
        C: Display + Send + Sync + 'static,
    {
        self.map_err(|e| Error::Context(context.to_string(), Box::new(e)))
    }

    fn with_context<C, F>(self, f: F) -> Result<T>
    where
        C: Display + Send + Sync + 'static,
        F: FnOnce() -> C,
    {
        self.map_err(|e| Error::Context(f().to_string(), Box::new(e)))
    }
}

impl<T> Context<T> for Option<T> {
    fn context<C>(self, context: C) -> Result<T>
    where
        C: Display + Send + Sync + 'static,
    {
        self.ok_or_else(|| Error::GenericError(context.to_string()))
    }

    fn with_context<C, F>(self, f: F) -> Result<T>
    where
        C: Display + Send + Sync + 'static,
        F: FnOnce() -> C,
    {
        self.ok_or_else(|| Error::GenericError(f().to_string()))
    }
}

/// Extension trait for filesystem operations with automatic path context.
///
/// Wraps I/O errors with the path that caused them for better diagnostics.
///
/// # Examples
///
/// ```no_run
/// # use std::path::{Path, PathBuf};
/// # type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
/// #
/// # // Mock ErrorExt trait
/// # trait ErrorExt<T> {
/// #     fn fs_context(self, context: &'static str, path: impl Into<PathBuf>) -> Result<T>;
/// # }
/// # impl<T> ErrorExt<T> for std::result::Result<T, std::io::Error> {
/// #     fn fs_context(self, context: &'static str, path: impl Into<PathBuf>) -> Result<T> {
/// #         self.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
/// #     }
/// # }
/// #
/// fn create_package_dir(path: &Path) -> Result<()> {
///     std::fs::create_dir_all(path)
///         .fs_context("creating package directory", path)?;
///     Ok(())
/// }
/// ```
pub trait ErrorExt<T> {
    /// Add filesystem context to an I/O error.
    ///
    /// The `context` should be a present-tense verb phrase describing the operation,
    /// e.g., "reading file", "creating directory", "copying binary".
    fn fs_context(self, context: &'static str, path: impl Into<PathBuf>) -> Result<T>;
}

impl<T> ErrorExt<T> for std::result::Result<T, std::io::Error> {
    fn fs_context(self, context: &'static str, path: impl Into<PathBuf>) -> Result<T> {
        self.map_err(|error| Error::Fs {
            context,
            path: path.into(),
            error,
        })
    }
}

/// Macro for early return with error.
///
/// Converts the message into a [`Error::GenericError`] and returns immediately.
///
/// # Examples
///
/// ```ignore
/// bail!("operation failed");
/// bail!("invalid value: {}", value);
/// bail!(format!("expected {} but got {}", expected, actual));
/// ```
#[macro_export]
macro_rules! bail {
    ($msg:literal $(,)?) => {
        return Err($crate::bundler::error::Error::GenericError($msg.into()))
    };
    ($err:expr $(,)?) => {
        return Err($crate::bundler::error::Error::GenericError($err.to_string()))
    };
    ($fmt:expr, $($arg:tt)*) => {
        return Err($crate::bundler::error::Error::GenericError(format!($fmt, $($arg)*)))
    };
}
