//! CandleContext trait definition - mirrors cyrup-domain Context trait exactly
//!
//! This trait provides the core context interface for Candle-backed context implementations,
//! enabling trait composition, testability, and 'room to move' architecture benefits.

use std::collections::HashMap;
use std::path::PathBuf;

use cyrup_sugars::prelude::MessageChunk;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use tokio_stream::Stream;

/// `CandleContext` trait - mirrors `cyrup-domain::Context` exactly with Candle prefix
///
/// This trait enables:
/// - Trait composition for flexible context architectures  
/// - Testability with mock implementations
/// - Zero-cost abstractions via static dispatch
/// - 'Room to move' for future context providers
pub trait CandleContext: Send + Sync + 'static {
    /// Load context content from the source
    ///
    /// # Returns
    /// tokio Stream containing context content chunks
    fn load_content(&self) -> Pin<Box<dyn Stream<Item = CandleContextChunk> + Send>>;

    /// Get context metadata and information
    ///
    /// # Returns
    /// Context metadata for introspection
    fn get_metadata(&self) -> CandleContextMetadata;

    /// Refresh context content if it has changed
    ///
    /// # Returns
    /// tokio Stream indicating whether refresh was successful
    fn refresh(
        &self,
    ) -> Pin<Box<dyn Stream<Item = crate::domain::context::chunks::CandleRefreshResult> + Send>>;

    /// Get context capabilities and supported operations
    ///
    /// # Returns
    /// tokio Stream containing context capabilities
    fn get_capabilities(&self) -> Pin<Box<dyn Stream<Item = CandleContextCapabilities> + Send>>;
}

/// Context content chunk for streaming large contexts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleContextChunk {
    /// Chunk content
    pub content: String,
    /// Content type (text, code, data, etc.)
    pub content_type: CandleContextType,
    /// Chunk metadata
    pub metadata: Option<HashMap<String, String>>,
    /// Whether this is the last chunk
    pub is_final: bool,
    /// Chunk sequence number
    pub sequence: u64,
}

impl CandleContextChunk {
    /// Create new context chunk
    pub fn new(content: impl Into<String>, content_type: CandleContextType) -> Self {
        Self {
            content: content.into(),
            content_type,
            metadata: None,
            is_final: false,
            sequence: 0,
        }
    }

    /// Mark chunk as final
    #[must_use]
    pub fn with_final(mut self) -> Self {
        self.is_final = true;
        self
    }

    /// Set chunk sequence number
    #[must_use]
    pub fn with_sequence(mut self, sequence: u64) -> Self {
        self.sequence = sequence;
        self
    }

    /// Add metadata to chunk
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        if self.metadata.is_none() {
            self.metadata = Some(HashMap::new());
        }
        if let Some(ref mut metadata) = self.metadata {
            metadata.insert(key.into(), value.into());
        }
        self
    }
}

impl MessageChunk for CandleContextChunk {
    fn bad_chunk(error: String) -> Self {
        CandleContextChunk {
            content: format!("ERROR: {error}"),
            content_type: CandleContextType::Text,
            metadata: None,
            is_final: true,
            sequence: 0,
        }
    }

    fn error(&self) -> Option<&str> {
        if self.content.starts_with("ERROR: ") {
            Some(&self.content)
        } else {
            None
        }
    }
}

impl Default for CandleContextChunk {
    fn default() -> Self {
        CandleContextChunk {
            content: String::new(),
            content_type: CandleContextType::Text,
            metadata: None,
            is_final: false,
            sequence: 0,
        }
    }
}

/// Context content type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum CandleContextType {
    /// Plain text content
    #[default]
    Text,
    /// Code content with language
    Code(String),
    /// Markdown content
    Markdown,
    /// JSON data
    Json,
    /// XML data
    Xml,
    /// Binary data (base64 encoded)
    Binary,
    /// Directory listing
    Directory,
    /// File metadata
    FileInfo,
    /// Unknown content type
    Unknown,
}

/// Context metadata for introspection and management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleContextMetadata {
    /// Context name or identifier
    pub name: String,
    /// Context description
    pub description: String,
    /// Context source type (file, directory, url, etc.)
    pub source_type: CandleContextSource,
    /// Source location or path
    pub source_location: String,
    /// Content size in bytes (if known)
    pub content_size: Option<u64>,
    /// Last modified timestamp
    pub last_modified: Option<u64>,
    /// Context version or revision
    pub version: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl CandleContextMetadata {
    /// Create new context metadata
    pub fn new(
        name: impl Into<String>,
        source_type: CandleContextSource,
        location: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            source_type,
            source_location: location.into(),
            content_size: None,
            last_modified: None,
            version: None,
            metadata: HashMap::new(),
        }
    }

    /// Set description
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set content size
    #[must_use]
    pub fn with_size(mut self, size: u64) -> Self {
        self.content_size = Some(size);
        self
    }

    /// Set last modified timestamp
    #[must_use]
    pub fn with_last_modified(mut self, timestamp: u64) -> Self {
        self.last_modified = Some(timestamp);
        self
    }

    /// Add metadata field
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Context source type enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CandleContextSource {
    /// File system file
    File,
    /// File system directory
    Directory,
    /// Git repository
    Git,
    /// HTTP URL
    Http,
    /// Database query
    Database,
    /// In-memory content
    Memory,
    /// Environment variables
    Environment,
    /// Command output
    Command,
    /// Unknown source
    Unknown,
}

bitflags::bitflags! {
    /// Context capability flags for zero-allocation capability checks
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct ContextCapabilityFlags: u8 {
        /// Context supports real-time updates
        const REALTIME_UPDATES = 1 << 0;
        /// Context can be refreshed
        const REFRESH = 1 << 1;
        /// Context supports streaming
        const STREAMING = 1 << 2;
        /// Context supports search/filtering
        const SEARCH = 1 << 3;
    }
}

/// Context capabilities and supported operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleContextCapabilities {
    /// Capability flags
    pub flags: ContextCapabilityFlags,
    /// Maximum content size supported
    pub max_content_size: Option<u64>,
    /// Supported content types
    pub supported_content_types: Vec<CandleContextType>,
    /// Required permissions or capabilities
    pub required_permissions: Vec<String>,
}

impl Default for CandleContextCapabilities {
    fn default() -> Self {
        Self {
            flags: ContextCapabilityFlags::REFRESH | ContextCapabilityFlags::STREAMING,
            max_content_size: Some(10 * 1024 * 1024), // 10MB
            supported_content_types: vec![CandleContextType::Text],
            required_permissions: Vec::new(),
        }
    }
}

impl MessageChunk for CandleContextCapabilities {
    fn bad_chunk(error: String) -> Self {
        CandleContextCapabilities {
            flags: ContextCapabilityFlags::empty(),
            max_content_size: None,
            supported_content_types: vec![CandleContextType::Text],
            required_permissions: vec![format!("ERROR: {}", error)],
        }
    }

    fn error(&self) -> Option<&str> {
        if let Some(perm) = self.required_permissions.first()
            && perm.starts_with("ERROR: ")
        {
            return Some(perm);
        }
        None
    }
}

/// File context implementation
#[derive(Debug, Clone)]
pub struct CandleFileContext {
    /// File path
    pub path: PathBuf,
    /// Context metadata
    pub metadata: CandleContextMetadata,
}

impl CandleFileContext {
    /// Create new file context
    pub fn new(path: impl Into<PathBuf>) -> Self {
        let path_buf = path.into();
        let metadata = CandleContextMetadata::new(
            path_buf.file_name().unwrap_or_default().to_string_lossy(),
            CandleContextSource::File,
            path_buf.to_string_lossy(),
        );

        Self {
            path: path_buf,
            metadata,
        }
    }

    /// Create file context with description
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.metadata = self.metadata.with_description(description);
        self
    }
}

impl CandleContext for CandleFileContext {
    fn load_content(&self) -> Pin<Box<dyn Stream<Item = CandleContextChunk> + Send>> {
        let path = self.path.clone();

        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            if let Ok(content) = tokio::fs::read_to_string(&path).await {
                let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                let content_type = match extension {
                    "rs" | "py" | "js" | "ts" | "go" | "java" | "cpp" | "c" | "h" => {
                        CandleContextType::Code(extension.to_string())
                    }
                    "md" => CandleContextType::Markdown,
                    "json" => CandleContextType::Json,
                    "xml" | "html" => CandleContextType::Xml,
                    _ => CandleContextType::Text,
                };

                let chunk = CandleContextChunk::new(content, content_type)
                    .with_final()
                    .with_sequence(0)
                    .with_metadata("file_path", path.to_string_lossy());

                let _ = tx.send(chunk);
            } else {
                let error_chunk = CandleContextChunk::new(
                    format!("Error reading file: {}", path.display()),
                    CandleContextType::Text,
                )
                .with_final();
                let _ = tx.send(error_chunk);
            }
        }))
    }

    fn get_metadata(&self) -> CandleContextMetadata {
        self.metadata.clone()
    }

    fn refresh(
        &self,
    ) -> Pin<Box<dyn Stream<Item = crate::domain::context::chunks::CandleRefreshResult> + Send>>
    {
        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            // For files, always return success since file system access is always "fresh"
            let _ = tx.send(crate::domain::context::chunks::CandleRefreshResult::success());
        }))
    }

    fn get_capabilities(&self) -> Pin<Box<dyn Stream<Item = CandleContextCapabilities> + Send>> {
        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            let capabilities = CandleContextCapabilities {
                flags: ContextCapabilityFlags::REALTIME_UPDATES
                    | ContextCapabilityFlags::REFRESH
                    | ContextCapabilityFlags::STREAMING,
                max_content_size: Some(100 * 1024 * 1024), // 100MB
                supported_content_types: vec![
                    CandleContextType::Text,
                    CandleContextType::Code("*".to_string()),
                    CandleContextType::Markdown,
                    CandleContextType::Json,
                    CandleContextType::Xml,
                ],
                required_permissions: vec!["file_read".to_string()],
            };
            let _ = tx.send(capabilities);
        }))
    }
}
