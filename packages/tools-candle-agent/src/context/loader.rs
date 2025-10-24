//! Core File Loading Interface and Implementation
//!
//! This module provides the core file loading interface and implementation
//! for pattern-based file loading and streaming operations.
//! Originally from loader.rs.

use std::fmt;
use std::path::PathBuf;

use tokio_stream::Stream;
use crate::async_stream;
use crate::AsyncTask;

use cyrup_sugars::ZeroOneOrMany as ZeroOneOrMany;

/// Trait defining the core file loading interface
pub trait Loader<T>: Send + Sync + fmt::Debug + Clone
where
    T: Send + Sync + fmt::Debug + Clone + 'static,
{
    /// Get the current file pattern
    fn pattern(&self) -> Option<&str>;

    /// Get the recursive setting
    fn recursive(&self) -> bool;

    /// Load all files matching the criteria
    fn load_all(&self) -> AsyncTask<ZeroOneOrMany<T>>
    where
        T: Send + 'static;

    /// Stream files one by one
    fn stream_files(&self) -> impl Stream<Item = T>
    where
        T: Send + 'static;

    /// Process each file with a processor function
    fn process_each<F, U>(&self, processor: F) -> AsyncTask<ZeroOneOrMany<U>>
    where
        F: Fn(&T) -> U + Send + Sync + 'static,
        U: Send + Sync + fmt::Debug + Clone + 'static;

    /// Create new loader with pattern
    fn new(pattern: impl Into<String>) -> Self;

    /// Set recursive loading
    fn with_recursive(self, recursive: bool) -> Self;

    /// Apply filter to results
    fn with_filter<F>(self, filter: F) -> Self
    where
        F: Fn(&T) -> bool + Send + Sync + 'static;
}

/// Type alias for filter function
pub type FilterFn<T> = std::sync::Arc<dyn Fn(&T) -> bool + Send + Sync>;

/// Implementation of the Loader trait for PathBuf
pub struct LoaderImpl<T: Send + Sync + fmt::Debug + Clone + 'static> {
    pattern: Option<String>,
    recursive: bool,
    filter_fn: Option<FilterFn<T>>,
}

// LoaderImpl implements NotResult since it contains no Result types

impl<T: Send + Sync + fmt::Debug + Clone + 'static> std::fmt::Debug for LoaderImpl<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoaderImpl")
            .field("pattern", &self.pattern)
            .field("recursive", &self.recursive)
            .field("filter_fn", &"<function>")
            .finish()
    }
}

impl<T: Send + Sync + fmt::Debug + Clone + 'static> Clone for LoaderImpl<T> {
    fn clone(&self) -> Self {
        Self {
            pattern: self.pattern.clone(),
            recursive: self.recursive,
            filter_fn: self.filter_fn.clone(),
        }
    }
}

impl Loader<PathBuf> for LoaderImpl<PathBuf> {
    fn pattern(&self) -> Option<&str> {
        self.pattern.as_deref()
    }

    fn recursive(&self) -> bool {
        self.recursive
    }

    fn load_all(&self) -> AsyncTask<ZeroOneOrMany<PathBuf>>
    where
        PathBuf: Send + 'static,
    {
        let pattern = self.pattern.clone();
        let recursive = self.recursive;
        let filter_fn = self.filter_fn.clone();
        
        tokio::spawn(async move {
            let mut results: Vec<PathBuf> = match pattern {
                Some(p) => {
                    let glob_pattern = if recursive && !p.contains("**") {
                        format!("**/{}", p)
                    } else {
                        p
                    };
                    
                    match glob::glob(&glob_pattern) {
                        Ok(paths) => paths.filter_map(Result::ok).collect(),
                        Err(_) => Vec::new(), // Return empty on pattern error
                    }
                }
                None => Vec::new(),
            };

            // Apply filter if present
            if let Some(ref filter) = filter_fn {
                results.retain(|item| filter(item));
            }

            // Convert Vec<PathBuf> to ZeroOneOrMany<PathBuf> without unwrap
            match results.len() {
                0 => ZeroOneOrMany::None,
                1 => {
                    let mut iter = results.into_iter();
                    if let Some(item) = iter.next() {
                        ZeroOneOrMany::One(item)
                    } else {
                        ZeroOneOrMany::None
                    }
                }
                _ => ZeroOneOrMany::many(results),
            }
        })
    }

    fn stream_files(&self) -> impl Stream<Item = PathBuf>
    where
        PathBuf: Send + 'static,
    {
        let pattern = self.pattern.clone();
        let recursive = self.recursive;
        let filter_fn = self.filter_fn.clone();

        async_stream::spawn_stream(move |tx| async move {
            if let Some(p) = pattern {
                let glob_pattern = if recursive && !p.contains("**") {
                    format!("**/{}", p)
                } else {
                    p
                };
                
                if let Ok(paths) = glob::glob(&glob_pattern) {
                    for path in paths.filter_map(Result::ok) {
                        // Apply filter before sending
                        if let Some(ref filter) = filter_fn {
                            if !filter(&path) {
                                continue;
                            }
                        }
                        
                        if tx.send(path).is_err() {
                            break;
                        }
                    }
                }
            }
        })
    }

    fn process_each<F, U>(&self, processor: F) -> AsyncTask<ZeroOneOrMany<U>>
    where
        F: Fn(&PathBuf) -> U + Send + Sync + 'static,
        U: Send + Sync + fmt::Debug + Clone + 'static + Send + 'static,
    {
        let load_task = self.load_all();
        tokio::spawn(async move {
            let paths = load_task.collect();
            let results: Vec<U> = match paths {
                ZeroOneOrMany::None => Vec::new(),
                ZeroOneOrMany::One(path) => vec![processor(&path)],
                ZeroOneOrMany::Many(paths) => paths.iter().map(|p| processor(p)).collect()};

            // Convert Vec<U> to ZeroOneOrMany<U> without unwrap
            match results.len() {
                0 => ZeroOneOrMany::None,
                1 => {
                    let mut iter = results.into_iter();
                    if let Some(item) = iter.next() {
                        ZeroOneOrMany::One(item)
                    } else {
                        ZeroOneOrMany::None
                    }
                }
                _ => ZeroOneOrMany::many(results)}
        })
    }

    fn new(pattern: impl Into<String>) -> Self {
        Self {
            pattern: Some(pattern.into()),
            recursive: false,
            filter_fn: None,
        }
    }

    fn with_recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }

    fn with_filter<F>(mut self, filter: F) -> Self
    where
        F: Fn(&PathBuf) -> bool + Send + Sync + 'static,
    {
        self.filter_fn = Some(std::sync::Arc::new(filter));
        self
    }
}

// Generic implementation for other types
impl<T: Send + Sync + fmt::Debug + Clone + 'static> LoaderImpl<T> {
    // Iterator functionality removed - use pattern-based loading instead
}

// Builder implementations moved to cyrup/src/builders/loader.rs

// Type alias for convenience
pub type DefaultLoader<T> = LoaderImpl<T>;

// Backward compatibility aliases
pub type FileLoader<T> = LoaderImpl<T>;
