//! Context implementation module for Candle context provider system
//!
//! This module contains `CandleContext<T>` and all type-specific implementations
//! for File, Files, Directory, and GitHub context operations.

use kodegen_tools_git::{
    CloneOpts, FetchOpts, GitError as GitGixError, MergeOpts, clone_repo, fetch, merge, open_repo,
};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::time::SystemTime;
use tokio_stream::Stream;
use uuid::Uuid;

use super::processor::CandleStreamingContextProcessor;
use super::types::{
    CandleContextError, CandleContextEvent, CandleDirectory, CandleFile, CandleFiles, CandleGithub,
    CandleImmutableDirectoryContext, CandleImmutableFileContext, CandleImmutableFilesContext,
    CandleImmutableGithubContext,
};
use crate::domain::context::CandleDocument as Document;

/// Context wrapper with zero Arc usage
#[derive(Debug)]
pub struct CandleContext<T> {
    source: CandleContextSourceType,
    processor: CandleStreamingContextProcessor,
    _marker: PhantomData<T>,
}

/// Candle context source types with immutable implementations
#[derive(Debug, Clone)]
pub enum CandleContextSourceType {
    File(CandleImmutableFileContext),
    Files(CandleImmutableFilesContext),
    Directory(CandleImmutableDirectoryContext),
    Github(CandleImmutableGithubContext),
}

impl<T> Clone for CandleContext<T> {
    fn clone(&self) -> Self {
        let processor_id = Uuid::new_v4().to_string();
        let processor = CandleStreamingContextProcessor::new(processor_id);
        Self {
            source: self.source.clone(),
            processor,
            _marker: PhantomData,
        }
    }
}

impl<T> CandleContext<T> {
    /// Create new Candle context with streaming processor
    #[inline]
    pub fn new(source: CandleContextSourceType) -> Self {
        let processor_id = Uuid::new_v4().to_string();
        let processor = CandleStreamingContextProcessor::new(processor_id);
        Self {
            source,
            processor,
            _marker: PhantomData,
        }
    }

    /// Create Candle context with event streaming
    #[inline]
    pub fn with_streaming(
        source: CandleContextSourceType,
    ) -> (Self, Pin<Box<dyn Stream<Item = CandleContextEvent> + Send>>) {
        let processor_id = Uuid::new_v4().to_string();
        let (processor, stream) = CandleStreamingContextProcessor::with_streaming(processor_id);
        let context = Self {
            source,
            processor,
            _marker: PhantomData,
        };
        (context, stream)
    }
}

// CandleContext<CandleFile> implementation
impl CandleContext<CandleFile> {
    /// Load single file - EXACT syntax: `CandleContext<CandleFile>::of("/path/to/file.txt")`
    #[inline]
    pub async fn of(path: impl AsRef<Path>) -> Self {
        use sha2::{Digest, Sha256};
        use tokio::io::AsyncReadExt;

        let path_ref = path.as_ref();
        let path_str = path_ref.to_string_lossy().to_string();

        // Read file metadata and content to compute hash
        let (size_bytes, modified, content_hash) = match tokio::fs::metadata(path_ref).await {
            Ok(metadata) => {
                let size = metadata.len();
                let modified_time = metadata.modified().unwrap_or_else(|_| SystemTime::now());

                // Compute content hash
                let hash = match tokio::fs::File::open(path_ref).await {
                    Ok(mut file) => {
                        let mut hasher = Sha256::new();
                        let mut buffer = vec![0u8; 8192];
                        loop {
                            match file.read(&mut buffer).await {
                                Ok(0) | Err(_) => break,
                                Ok(n) => hasher.update(&buffer[..n]),
                            }
                        }
                        let result = hasher.finalize();
                        result
                            .iter()
                            .fold(String::with_capacity(result.len() * 2), |mut s, b| {
                                use std::fmt::Write;
                                let _ = write!(&mut s, "{b:02x}");
                                s
                            })
                    }
                    Err(_) => String::new(),
                };

                (size, modified_time, hash)
            }
            Err(_) => (0, SystemTime::now(), String::new()),
        };

        let file_context = CandleImmutableFileContext {
            path: path_str,
            content_hash,
            size_bytes,
            modified,
            memory_integration: None,
        };
        Self::new(CandleContextSourceType::File(file_context))
    }

    /// Load documents asynchronously with streaming - returns unwrapped values
    #[inline]
    pub fn load(self) -> Pin<Box<dyn Stream<Item = Document> + Send>> {
        match self.source {
            CandleContextSourceType::File(file_context) => {
                self.processor.process_file_context(file_context)
            }
            _ => Box::pin(crate::async_stream::spawn_stream(move |_tx| async move {
                // Invalid context type for file loading
                log::error!("Invalid context type for file loading");
            })),
        }
    }
}

// CandleContext<CandleFiles> implementation
impl CandleContext<CandleFiles> {
    /// Glob pattern for files - EXACT syntax: `CandleContext<CandleFiles>::glob("**/*.{rs,md}")`
    #[inline]
    pub fn glob(pattern: impl AsRef<str>) -> Self {
        let pattern_str = pattern.as_ref().to_string();
        let files_context = CandleImmutableFilesContext {
            paths: Vec::new(), // Would be populated by glob expansion
            pattern: pattern_str,
            total_files: 0,
            memory_integration: None,
        };
        Self::new(CandleContextSourceType::Files(files_context))
    }

    /// Load documents asynchronously with streaming - returns unwrapped values
    #[inline]
    pub fn load(self) -> Pin<Box<dyn Stream<Item = Document> + Send>> {
        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            match self.source {
                CandleContextSourceType::Files(files_context) => {
                    // Expand glob pattern and load files
                    match glob::glob(&files_context.pattern) {
                        Ok(paths) => {
                            for entry in paths.flatten() {
                                if let Ok(content) = tokio::fs::read_to_string(&entry).await {
                                    let document = Document {
                                        data: content,
                                        format: Some(
                                            crate::domain::context::CandleContentFormat::Text,
                                        ),
                                        media_type: Some(
                                            crate::domain::context::CandleDocumentMediaType::TXT,
                                        ),
                                        additional_props: {
                                            let mut props = HashMap::new();
                                            props.insert(
                                                "id".to_string(),
                                                serde_json::Value::String(
                                                    Uuid::new_v4().to_string(),
                                                ),
                                            );
                                            props.insert(
                                                "path".to_string(),
                                                serde_json::Value::String(
                                                    entry.to_string_lossy().to_string(),
                                                ),
                                            );
                                            props
                                        },
                                    };
                                    let _ = tx.send(document);
                                }
                            }
                        }
                        Err(e) => {
                            log::error!(
                                "Streaming error in {}: {:?}",
                                "Glob pattern error",
                                CandleContextError::ContextNotFound(format!(
                                    "Glob pattern error: {e}"
                                ))
                            );
                        }
                    }
                }
                _ => {
                    log::error!(
                        "Streaming error in {}: {:?}",
                        "Invalid context type for files loading",
                        CandleContextError::ContextNotFound("Invalid context type".to_string())
                    );
                }
            }
        }))
    }
}

// CandleContext<CandleDirectory> implementation
impl CandleContext<CandleDirectory> {
    /// Load all files from directory - EXACT syntax: `CandleContext<CandleDirectory>::of("/path/to/dir")`
    #[inline]
    pub fn of(path: impl AsRef<Path>) -> Self {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let directory_context = CandleImmutableDirectoryContext {
            path: path_str,
            recursive: true,
            extensions: Vec::new(),
            max_depth: None,
            memory_integration: None,
        };
        Self::new(CandleContextSourceType::Directory(directory_context))
    }

    /// Load documents asynchronously with streaming - returns unwrapped values
    #[inline]
    pub fn load(self) -> Pin<Box<dyn Stream<Item = Document> + Send>> {
        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            // Use spawn for async directory traversal
            let _ = tokio::task::spawn(async move {
                match self.source {
                    CandleContextSourceType::Directory(directory_context) => {
                        // Traverse directory and load files

                        fn traverse_dir(
                            path: String,
                            recursive: bool,
                            extensions: Vec<String>,
                            max_depth: Option<usize>,
                            current_depth: usize,
                            sender: tokio::sync::mpsc::UnboundedSender<Document>,
                        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), std::io::Error>> + Send>> {
                            Box::pin(async move {
                            if let Some(max) = max_depth
                                && current_depth > max
                            {
                                return Ok(());
                            }

                            let mut entries = tokio::fs::read_dir(&path).await?;
                            while let Some(entry) = entries.next_entry().await? {
                                let path = entry.path();

                                if path.is_file() {
                                    let should_include = if extensions.is_empty() {
                                        true
                                    } else {
                                        path.extension().and_then(|ext| ext.to_str()).is_some_and(
                                            |ext| extensions.contains(&ext.to_string()),
                                        )
                                    };

                                    if should_include
                                        && let Ok(content) = tokio::fs::read_to_string(&path).await
                                    {
                                        let document = Document {
                                                data: content,
                                                format: Some(crate::domain::context::CandleContentFormat::Text),
                                                media_type: Some(
                                                    crate::domain::context::CandleDocumentMediaType::TXT,
                                                ),
                                                additional_props: {
                                                    let mut props = HashMap::new();
                                                    props.insert(
                                                        "id".to_string(),
                                                        serde_json::Value::String(
                                                            Uuid::new_v4().to_string(),
                                                        ),
                                                    );
                                                    props.insert(
                                                        "path".to_string(),
                                                        serde_json::Value::String(
                                                            path.to_string_lossy().to_string(),
                                                        ),
                                                    );
                                                    props
                                                }};
                                        let _ = sender.send(document);
                                    }
                                } else if path.is_dir()
                                    && recursive
                                    && let Some(path_str) = path.to_str()
                                {
                                    traverse_dir(
                                        path_str.to_string(),
                                        recursive,
                                        extensions.clone(),
                                        max_depth,
                                        current_depth + 1,
                                        sender.clone(),
                                    ).await?;
                                }
                            }
                            Ok(())
                            })
                        }

                        match traverse_dir(
                            directory_context.path.clone(),
                            directory_context.recursive,
                            directory_context.extensions.clone(),
                            directory_context.max_depth,
                            0,
                            tx.clone(),
                        ).await {
                            Ok(()) => {
                                // Documents are sent directly by traverse_dir
                            }
                            Err(e) => {
                                log::error!("Streaming error in {}: {:?}", "Directory traversal failed", CandleContextError::ContextNotFound(format!(
                                        "Directory traversal error: {e}"
                                    )));
                            }
                        }
                    }
                    _ => {
                        log::error!("Streaming error in {}: {:?}", "Invalid context type for directory loading", CandleContextError::ContextNotFound("Invalid context type".to_string()));
                    }
                }
            }).await;
        }))
    }
}

// CandleContext<CandleGithub> implementation
impl CandleContext<CandleGithub> {
    /// Glob pattern for GitHub files - EXACT syntax: `CandleContext<CandleGithub>::glob("/repo/**/*.{rs,md}")`
    #[inline]
    pub fn glob(pattern: impl AsRef<str>) -> Self {
        let pattern_str = pattern.as_ref().to_string();
        let github_context = CandleImmutableGithubContext {
            repository_url: String::new(), // Would be extracted from pattern
            branch: "main".to_string(),
            pattern: pattern_str,
            auth_token: None,
            memory_integration: None,
        };
        Self::new(CandleContextSourceType::Github(github_context))
    }

    /// Get cache directory for GitHub repositories
    fn get_github_cache_dir() -> PathBuf {
        std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_or_else(
                |_| std::path::PathBuf::from("/tmp/cyrup/github"),
                |home| std::path::PathBuf::from(home).join(".cache/cyrup/github"),
            )
    }

    /// Create document from file with GitHub metadata
    fn create_github_document(
        content: String,
        relative_path: String,
        repository_url: String,
        branch: String,
    ) -> Document {
        let mut props = HashMap::new();
        props.insert(
            "id".to_string(),
            serde_json::Value::String(Uuid::new_v4().to_string()),
        );
        props.insert("path".to_string(), serde_json::Value::String(relative_path));
        props.insert(
            "repository".to_string(),
            serde_json::Value::String(repository_url),
        );
        props.insert("branch".to_string(), serde_json::Value::String(branch));

        Document {
            data: content,
            format: Some(crate::domain::context::CandleContentFormat::Text),
            media_type: Some(crate::domain::context::CandleDocumentMediaType::TXT),
            additional_props: props,
        }
    }

    /// Build authenticated URL by embedding token if provided
    fn build_auth_url(repo_url: &str, auth_token: Option<&String>) -> String {
        if let Some(token) = auth_token {
            // Inject token into HTTPS URL: https://github.com -> https://TOKEN@github.com
            if repo_url.starts_with("https://") {
                repo_url.replace("https://", &format!("https://{token}@"))
            } else {
                // For non-HTTPS URLs, return as-is (SSH, git://, etc.)
                repo_url.to_string()
            }
        } else {
            repo_url.to_string()
        }
    }

    /// Clone or update a git repository
    async fn get_or_clone_repo(
        repo_url: &str,
        branch: &str,
        auth_token: Option<&String>,
        cache_dir: &Path,
    ) -> Result<PathBuf, GitGixError> {
        // Generate cache path from repo URL
        let repo_name = repo_url
            .trim_end_matches(".git")
            .split('/')
            .next_back()
            .unwrap_or("repo");
        let repo_path = cache_dir.join(repo_name);

        if repo_path.exists() {
            Self::update_repo(&repo_path, branch, auth_token).await
        } else {
            Self::clone_repo(repo_url, branch, auth_token, &repo_path, cache_dir).await
        }
    }

    /// Update existing repository
    async fn update_repo(
        repo_path: &Path,
        branch: &str,
        _auth_token: Option<&String>,
    ) -> Result<PathBuf, GitGixError> {
        // Open repository
        let repo_handle = open_repo(repo_path)
            .await
            .map_err(|e| GitGixError::Gix(Box::new(e)))?
            .map_err(|e| GitGixError::Gix(Box::new(e)))?;

        // Fetch from origin
        let fetch_opts = FetchOpts::from_remote("origin");
        fetch(repo_handle.clone(), fetch_opts)
            .await
            .map_err(|e| GitGixError::Gix(Box::new(e)))?;

        // Merge remote branch (gitoxide automatically does fast-forward if possible)
        let remote_branch = format!("origin/{branch}");
        let merge_opts = MergeOpts::new(remote_branch);
        merge(repo_handle, merge_opts)
            .await
            .map_err(|e| GitGixError::Gix(Box::new(e)))?;

        Ok(repo_path.to_path_buf())
    }

    /// Clone fresh repository
    async fn clone_repo(
        repo_url: &str,
        branch: &str,
        auth_token: Option<&String>,
        repo_path: &Path,
        cache_dir: &Path,
    ) -> Result<PathBuf, GitGixError> {
        tokio::fs::create_dir_all(cache_dir).await.ok();

        // Build authenticated URL
        let auth_url = Self::build_auth_url(repo_url, auth_token);

        // Create clone options
        let opts = CloneOpts::new(auth_url, repo_path).branch(branch);

        // Execute clone
        let _repo_handle = clone_repo(opts)
            .await
            .map_err(|e| GitGixError::Gix(Box::new(e)))?
            .map_err(|e| GitGixError::Gix(Box::new(e)))?;

        Ok(repo_path.to_path_buf())
    }

    /// Load documents asynchronously with streaming - returns unwrapped values
    #[inline]
    pub fn load(self) -> Pin<Box<dyn Stream<Item = Document> + Send>> {
        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            match self.source {
                CandleContextSourceType::Github(github_context) => {
                    // Validate repository URL
                    if github_context.repository_url.is_empty() {
                        log::error!(
                            "Streaming error in {}: {:?}",
                            "GitHub repository URL missing",
                            CandleContextError::ContextNotFound(
                                "GitHub repository URL is required".to_string()
                            )
                        );
                        return;
                    }

                    // Determine cache directory (use standard location)
                    let cache_dir = Self::get_github_cache_dir();

                    // Clone or update repository
                    match Self::get_or_clone_repo(
                        &github_context.repository_url,
                        &github_context.branch,
                        github_context.auth_token.as_ref(),
                        &cache_dir,
                    )
                    .await
                    {
                        Ok(repo_path) => {
                            // Build glob pattern for files in repository
                            let glob_pattern =
                                format!("{}/{}", repo_path.display(), github_context.pattern);

                            // Match files using glob pattern
                            match glob::glob(&glob_pattern) {
                                Ok(paths) => {
                                    for entry in paths.flatten() {
                                        // Read file content
                                        if let Ok(content) = tokio::fs::read_to_string(&entry).await
                                        {
                                            let relative_path = entry
                                                .strip_prefix(&repo_path)
                                                .unwrap_or(&entry)
                                                .to_string_lossy()
                                                .to_string();

                                            let document = Self::create_github_document(
                                                content,
                                                relative_path,
                                                github_context.repository_url.clone(),
                                                github_context.branch.clone(),
                                            );

                                            let _ = tx.send(document);
                                        }
                                    }
                                }
                                Err(e) => {
                                    log::error!(
                                        "Streaming error in {}: {:?}",
                                        "Glob pattern expansion failed",
                                        CandleContextError::PatternError(format!(
                                            "Glob pattern error for '{}': {}",
                                            github_context.pattern, e
                                        ))
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            log::error!(
                                "Streaming error in {}: {:?}",
                                "GitHub repository access failed",
                                CandleContextError::ProviderUnavailable(format!(
                                    "Failed to clone/update repository '{}': {}",
                                    github_context.repository_url, e
                                ))
                            );
                        }
                    }
                }
                _ => {
                    log::error!(
                        "Streaming error in {}: {:?}",
                        "Invalid context type for GitHub loading",
                        CandleContextError::ContextNotFound("Invalid context type".to_string())
                    );
                }
            }
        }))
    }
}
