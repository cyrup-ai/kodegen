//! Core Tantivy search engine implementation
//!
//! This module provides the main SearchEngine that manages the Tantivy index,
//! handles document indexing operations, and executes search queries.

use anyhow::{Result, Context};
use std::path::PathBuf;
use tantivy::{Index, IndexWriter, IndexReader, Term, IndexSettings};
use tantivy::query::QueryParser;
use tantivy::directory::MmapDirectory;

use crate::config::CrawlConfig;
use crate::runtime::{AsyncTask, spawn_async};
use super::schema::SearchSchema;
use super::errors::{SearchError, SearchResult, RetryConfig};
use super::runtime_helpers::retry_task;

/// Main search engine managing Tantivy index operations
#[derive(Clone)]
pub struct SearchEngine {
    index: Index,
    schema: SearchSchema,
    reader: IndexReader,
    query_parser: QueryParser,
    index_path: PathBuf,
}

impl SearchEngine {
    /// Create a new search engine instance asynchronously
    pub fn create_async(
        config: &CrawlConfig,
        on_result: impl FnOnce(Result<Self>) + Send + 'static,
    ) -> AsyncTask<()> {
        let config = config.clone();
        
        spawn_async(async move {
            let result = async {
                let index_dir = config.search_index_dir();
                let index_path_buf = index_dir.clone();
                let memory_limit = config.search_memory_limit();

                // Create index directory if it doesn't exist
                std::fs::create_dir_all(&index_dir)
                    .with_context(|| format!("Failed to create index directory: {:?}", index_dir))?;

                // Build search schema FIRST before creating/opening index
                // This ensures the index is created with the correct schema
                let schema_builder = SearchSchema::builder();
                let (build_tx, build_rx) = tokio::sync::oneshot::channel();
                let _build_task = schema_builder.build(move |result| {
                    let _ = build_tx.send(result);
                });
                let schema = build_rx.await
                    .map_err(|_| anyhow::anyhow!("Failed to receive schema build result"))?
                    .with_context(|| "Failed to build schema")?;
                
                // Open or create Tantivy index with the proper schema
                let index = if index_dir.join("meta.json").exists() {
                    Index::open_in_dir(&index_dir)
                        .with_context(|| format!("Failed to open existing index at {:?}", index_dir))?
                } else {
                    // Create index with the PROPER schema (not empty default)
                    let mmap_directory = MmapDirectory::open(&index_dir)
                        .with_context(|| format!("Failed to create index directory at {:?}", index_dir))?;
                    Index::create(mmap_directory, schema.schema.clone(), IndexSettings::default())
                        .with_context(|| "Failed to create new Tantivy index")?
                };

                // Register custom tokenizers with the index
                let (reg_tx, reg_rx) = tokio::sync::oneshot::channel();
                let _reg_task = SearchSchema::builder().register_tokenizers(index.tokenizers(), move |result| {
                    let _ = reg_tx.send(result);
                });
                reg_rx.await
                    .map_err(|_| anyhow::anyhow!("Failed to receive tokenizer registration result"))?
                    .with_context(|| "Failed to register tokenizers")?;

                // Configure index settings
                let limit = memory_limit; // Already calculated by config
                let mut index_writer: tantivy::IndexWriter = index.writer(limit)
                    .with_context(|| "Failed to create index writer")?;
                index_writer.commit()
                    .with_context(|| "Failed to commit initial index state")?;

                // Create reader for search operations
                let reader = index.reader()
                    .with_context(|| "Failed to create index reader")?;

                // Create query parser for multiple fields
                let query_parser = QueryParser::for_index(&index, vec![
                    schema.title,
                    schema.plain_content,
                    schema.raw_markdown,
                ]);

                Ok(SearchEngine {
                    index,
                    schema,
                    reader,
                    query_parser,
                    index_path: index_path_buf,
                })
            }.await;
            
            on_result(result);
        })
    }

    /// Get a reference to the search schema
    pub fn schema(&self) -> &SearchSchema {
        &self.schema
    }

    /// Get the Tantivy index
    pub fn index(&self) -> &Index {
        &self.index
    }

    /// Create an index writer with configured memory limit and retry logic
    pub fn writer_with_retry(
        &self,
        memory_limit: Option<usize>,
        on_result: impl FnOnce(SearchResult<IndexWriter>) + Send + 'static,
    ) -> AsyncTask<()> {
        let limit = memory_limit.unwrap_or(50_000_000); // 50MB default
        let retry_config = RetryConfig::default();
        let engine = self.clone();
        
        let task = retry_task(retry_config, move || {
            let eng = engine.clone();
            spawn_async(async move {
                eng.index.writer(limit)
                    .map_err(|e| SearchError::WriterAcquisition(format!(
                        "Failed to acquire index writer with {}MB limit: {}", 
                        limit / 1_000_000, 
                        e
                    )))
            })
        });
        
        spawn_async(async move {
            match task.await {
                Ok(result) => on_result(result),
                Err(e) => on_result(Err(SearchError::Other(format!("Task execution failed: {}", e)))),
            }
        })
    }
    
    /// Create an index writer with configured memory limit (synchronous fallback)
    pub fn writer(
        &self,
        memory_limit: Option<usize>,
        on_result: impl FnOnce(Result<IndexWriter>) + Send + 'static,
    ) -> AsyncTask<()> {
        let index = self.index.clone();
        spawn_async(async move {
            let result = {
                let limit = memory_limit.unwrap_or(50_000_000); // 50MB default
                index.writer(limit)
                    .with_context(|| "Failed to create index writer")
            };
            
            on_result(result);
        })
    }

    /// Get the index reader
    pub fn reader(&self) -> &IndexReader {
        &self.reader
    }

    /// Get the query parser
    pub fn query_parser(&self) -> &QueryParser {
        &self.query_parser
    }

    /// Delete document by URL
    pub fn delete_document(
        &self,
        writer: &mut IndexWriter,
        url: String,
    ) -> Result<()> {
        let url_term = Term::from_field_text(self.schema.url, &url);
        writer.delete_term(url_term);
        Ok(())
    }

    /// Commit changes and optimize index with logging
    pub fn commit_and_optimize(
        &self,
        writer: &mut IndexWriter,
        on_result: impl FnOnce(SearchResult<()>) + Send + 'static,
    ) -> AsyncTask<()> {
        let start = std::time::Instant::now();
        let reader = self.reader.clone();
        
        // We need to perform the commit synchronously since we have a mutable reference
        let commit_result = writer.commit()
            .map_err(|e| SearchError::CommitFailed(format!("Index commit failed: {}", e)));
        
        spawn_async(async move {
            let result = (|| -> SearchResult<()> {
                commit_result?;
                
                let commit_duration = start.elapsed();
                tracing::info!(
                    duration_ms = commit_duration.as_millis(),
                    "Index commit completed"
                );
                
                // Reload reader to see changes
                reader.reload()
                    .map_err(|e| SearchError::Other(format!("Failed to reload reader: {}", e)))?;
                
                let total_duration = start.elapsed();
                tracing::debug!(
                    total_duration_ms = total_duration.as_millis(),
                    commit_duration_ms = commit_duration.as_millis(),
                    "Index commit and reload completed"
                );

                Ok(())
            })();
            
            on_result(result);
        })
    }

    /// Check if index exists and is valid with corruption detection
    pub fn validate_index(
        &self,
        on_result: impl FnOnce(SearchResult<bool>) + Send + 'static,
    ) -> AsyncTask<()> {
        let engine = self.clone();
        
        spawn_async(async move {
            let result = (|| -> SearchResult<bool> {
                let searcher = engine.reader.searcher();
                let num_docs = searcher.num_docs();
                
                // Try to perform a simple search to verify index integrity
                let test_query = engine.query_parser.parse_query("*")
                    .map_err(|e| SearchError::QueryParsing(format!("Failed to parse test query: {}", e)))?;
            
                match searcher.search(&test_query, &tantivy::collector::Count) {
                    Ok(_count) => {
                        tracing::debug!(
                            num_docs = num_docs,
                            "Index validation successful"
                        );
                        Ok(true)
                    }
                    Err(e) => {
                        tracing::error!(
                            error = %e,
                            "Index corruption detected during validation"
                        );
                        Err(SearchError::IndexCorruption(format!(
                            "Failed to execute test query: {}", e
                        )))
                    }
                }
            })();
            
            on_result(result);
        })
    }
    
    /// Attempt to recover from index corruption
    pub fn recover_index(
        &self,
        config: &CrawlConfig,
        on_result: impl FnOnce(SearchResult<()>) + Send + 'static,
    ) -> AsyncTask<()> {
        let index_dir = config.search_index_dir();
        let backup_dir = index_dir.with_file_name("search_index.backup");
        
        spawn_async(async move {
            let result: SearchResult<()> = (|| -> SearchResult<()> {
                tracing::warn!("Attempting index recovery");
                
                // First, try to backup the corrupted index
                if index_dir.exists() {
                    if let Err(e) = std::fs::rename(&index_dir, &backup_dir) {
                        tracing::error!(
                            error = %e,
                            "Failed to backup corrupted index"
                        );
                    } else {
                        tracing::info!("Corrupted index backed up to {:?}", backup_dir);
                    }
                }
                
                // Create fresh index directory
                std::fs::create_dir_all(&index_dir)
                    .map_err(SearchError::Io)?;
                
                tracing::info!("Index recovery completed - reindexing required");
                Ok(())
            })();
            
            on_result(result);
        })
    }

    /// Get index statistics
    pub fn get_stats(
        &self,
        on_result: impl FnOnce(Result<IndexStats>) + Send + 'static,
    ) -> AsyncTask<()> {
        let reader = self.reader.clone();
        let last_commit = self.get_last_commit_time();
        let index_size_bytes = self.calculate_index_size();
        
        spawn_async(async move {
            let result = {
                let searcher = reader.searcher();
                let num_docs = searcher.num_docs() as usize;
                let num_segments = searcher.segment_readers().len();
                
                Ok(IndexStats {
                    num_documents: num_docs,
                    num_segments,
                    index_size_bytes,
                    last_commit,
                })
            };
            
            on_result(result);
        })
    }
    
    /// Get the last commit time from meta.json modification time
    fn get_last_commit_time(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        use std::time::SystemTime;
        
        let meta_path = self.index_path.join("meta.json");
        
        std::fs::metadata(&meta_path)
            .ok()
            .and_then(|metadata| metadata.modified().ok())
            .and_then(|system_time| {
                let duration = system_time.duration_since(SystemTime::UNIX_EPOCH).ok()?;
                let timestamp = duration.as_secs() as i64;
                chrono::DateTime::from_timestamp(timestamp, 0)
            })
    }
    
    /// Calculate the total size of the index directory
    fn calculate_index_size(&self) -> Option<u64> {
        use jwalk::WalkDir;
        
        if !self.index_path.exists() {
            return None;
        }
        
        let cpu_count = num_cpus::get();
        let parallelism = match cpu_count {
            1..=4 => cpu_count,
            5..=8 => cpu_count - 1,
            9..=16 => (cpu_count * 3) / 4,
            _ => 32,
        };
        
        let total_size: u64 = WalkDir::new(&self.index_path)
            .parallelism(jwalk::Parallelism::RayonNewPool(parallelism))
            .skip_hidden(false)
            .follow_links(false)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .filter_map(|entry| std::fs::metadata(entry.path()).ok())
            .map(|metadata| metadata.len())
            .sum();
        
        Some(total_size)
    }
}

/// Index statistics information
#[derive(Debug, Clone)]
pub struct IndexStats {
    pub num_documents: usize,
    pub num_segments: usize,
    pub index_size_bytes: Option<u64>,
    pub last_commit: Option<chrono::DateTime<chrono::Utc>>,
}