//! Document builder types and state

use crate::domain::context::{
    CandleContentFormat as ContentFormat, CandleDocumentChunk as DocumentChunk,
    CandleDocumentMediaType as DocumentMediaType,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::path::PathBuf;

/// Document builder data enumeration for zero-allocation type tracking
#[derive(Debug, Clone)]
pub(crate) enum DocumentBuilderData {
    File(PathBuf),
    Url(String),
    Github {
        repo: String,
        path: String,
        branch: Option<String>,
    },
    Glob(String),
    Text(String),
}

/// Hidden implementation struct - zero-allocation builder state using DOMAIN OBJECTS
#[derive(Clone)]
pub(crate) struct DocumentBuilderImpl<F1 = fn(String), F2 = fn(DocumentChunk) -> DocumentChunk>
where
    F1: Fn(String) + Send + Sync + 'static + Clone,
    F2: Fn(DocumentChunk) -> DocumentChunk + Send + Sync + 'static + Clone,
{
    pub(crate) data: DocumentBuilderData,
    pub(crate) format: Option<ContentFormat>,
    pub(crate) media_type: Option<DocumentMediaType>,
    pub(crate) additional_props: BTreeMap<String, Value>,
    pub(crate) encoding: Option<String>,
    pub(crate) max_size: Option<usize>,
    pub(crate) timeout_ms: Option<u64>,
    pub(crate) retry_attempts: u8,
    pub(crate) cache_enabled: bool,
    pub(crate) error_handler: Option<F1>,
    pub(crate) chunk_handler: Option<F2>,
    pub(crate) _marker: PhantomData<(F1, F2)>,
}

impl DocumentBuilderImpl {
    /// Create a new document builder with optimal defaults
    #[allow(clippy::new_ret_no_self)]
    pub(crate) fn new(data: DocumentBuilderData) -> impl super::trait_def::DocumentBuilder {
        DocumentBuilderImpl::<fn(String), fn(DocumentChunk) -> DocumentChunk> {
            data,
            format: None,
            media_type: None,
            additional_props: BTreeMap::new(),
            encoding: None,
            max_size: None,
            timeout_ms: None,
            retry_attempts: 3,
            cache_enabled: true,
            error_handler: None,
            chunk_handler: None,
            _marker: PhantomData,
        }
    }
}
