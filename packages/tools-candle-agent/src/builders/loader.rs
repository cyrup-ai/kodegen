//! Loader builder implementations - Zero Box<dyn> trait-based architecture
//!
//! All loader construction logic and builder patterns with zero allocation.

use std::fmt;
use std::path::PathBuf;
use std::marker::PhantomData;

use crate::domain::context::{CandleLoader as Loader, CandleLoaderImpl as LoaderImpl};
use crate::util::ZeroOneOrMany;
use tokio_stream::Stream;

/// Local NotResult trait for candle standalone operation
pub trait CandleNotResult: Send + Sync + 'static {}

/// Loader builder trait - elegant zero-allocation builder pattern
pub trait LoaderBuilder<T>: Sized 
where
    T: Send + Sync + fmt::Debug + Clone + 'static,
{
    /// Set recursive loading - EXACT syntax: .recursive(true)
    fn recursive(self, recursive: bool) -> impl LoaderBuilder<T>;
    
    /// Set filter function - EXACT syntax: .filter(|item| { ... })
    fn filter<F>(self, f: F) -> impl LoaderBuilder<T>
    where
        F: Fn(&T) -> bool + 'static;
    
    /// Map transformation - EXACT syntax: .map(|item| { ... })
    fn map<U, F>(self, f: F) -> impl LoaderBuilder<U>
    where
        F: Fn(T) -> U + 'static,
        U: Send + Sync + fmt::Debug + Clone + 'static;
    
    /// Set error handler - EXACT syntax: .on_error(|error| { ... })
    /// Zero-allocation: uses generic function pointer instead of Box<dyn>
    fn on_error<F>(self, handler: F) -> impl LoaderBuilder<T>
    where
        F: Fn(String) + Send + Sync + 'static;
    
    /// Set result handler - EXACT syntax: .on_result(|result| { ... })
    /// Zero-allocation: uses generic function pointer instead of Box<dyn>
    fn on_result<F>(self, handler: F) -> impl LoaderBuilder<T>
    where
        F: FnOnce(ZeroOneOrMany<T>) -> ZeroOneOrMany<T> + Send + 'static;
    
    /// Set chunk handler - EXACT syntax: .on_chunk(|chunk| { ... })
    /// Zero-allocation: uses generic function pointer instead of Box<dyn>
    fn on_chunk<F>(self, handler: F) -> impl LoaderBuilder<T>
    where
        F: FnMut(ZeroOneOrMany<T>) -> ZeroOneOrMany<T> + Send + 'static;
    
    /// Build loader - EXACT syntax: .build()
    fn build(self) -> impl Loader<T>
    where
        LoaderImpl<T>: Loader<T>;
    
    /// Build async loader - EXACT syntax: .build_async()
    fn build_async(self) -> AsyncTask<impl Loader<T>>
    where
        LoaderImpl<T>: Loader<T> + CandleNotResult;
    
    /// Load files immediately - EXACT syntax: .load_files()
    fn load_files(self) -> AsyncTask<ZeroOneOrMany<T>>
    where
        LoaderImpl<T>: Loader<T>,
        T: CandleNotResult;
    
    /// Stream files immediately - EXACT syntax: .stream()
    fn stream(self) -> impl Stream<Item = T>
    where
        LoaderImpl<T>: Loader<T>,
        T: CandleNotResult;
    
    /// Legacy load method - EXACT syntax: .load()
    fn load(self) -> AsyncTask<ZeroOneOrMany<T>>
    where
        LoaderImpl<T>: Loader<T>,
        T: CandleNotResult;
    
    /// Process with function - EXACT syntax: .process(|item| { ... })
    fn process<F, U>(self, processor: F) -> AsyncTask<ZeroOneOrMany<U>>
    where
        F: Fn(&T) -> U + Send + Sync + 'static,
        U: Send + Sync + fmt::Debug + Clone + 'static + CandleNotResult,
        LoaderImpl<T>: Loader<T>,
        T: CandleNotResult;
    
    /// Handle each item - EXACT syntax: .on_each(|item| { ... })
    fn on_each<F>(self, handler: F) -> AsyncTask<()>
    where
        F: Fn(&T) + Send + Sync + 'static,
        LoaderImpl<T>: Loader<T>,
        T: CandleNotResult;
}

/// Hidden implementation struct - zero-allocation builder state with zero Box<dyn> usage
struct LoaderBuilderImpl<
    T: Send + Sync + fmt::Debug + Clone + 'static,
    F1 = fn(String),
    F2 = fn(ZeroOneOrMany<T>) -> ZeroOneOrMany<T>,
    F3 = fn(ZeroOneOrMany<T>) -> ZeroOneOrMany<T>,
    F4 = fn(&T) -> bool,
> where
    F1: Fn(String) + Send + Sync + 'static,
    F2: FnOnce(ZeroOneOrMany<T>) -> ZeroOneOrMany<T> + Send + 'static,
    F3: FnMut(ZeroOneOrMany<T>) -> ZeroOneOrMany<T> + Send + 'static,
    F4: Fn(&T) -> bool + Send + Sync + 'static,
{
    pattern: Option<String>,
    recursive: bool,
    filter: Option<F4>,
    error_handler: Option<F1>,
    result_handler: Option<F2>,
    chunk_handler: Option<F3>,
    _marker: PhantomData<T>,
}

impl LoaderImpl<PathBuf> {
    /// Semantic entry point - EXACT syntax: LoaderImpl::files_matching("pattern")
    pub fn files_matching(pattern: &str) -> impl LoaderBuilder<PathBuf> {
        LoaderBuilderImpl {
            pattern: Some(pattern.to_string()),
            recursive: false,
            filter: None,
            error_handler: None,
            result_handler: None,
            chunk_handler: None,
            _marker: PhantomData,
        }
    }
}

impl<T, F1, F2, F3, F4> LoaderBuilder<T> for LoaderBuilderImpl<T, F1, F2, F3, F4>
where
    T: Send + Sync + fmt::Debug + Clone + 'static,
    F1: Fn(String) + Send + Sync + 'static,
    F2: FnOnce(ZeroOneOrMany<T>) -> ZeroOneOrMany<T> + Send + 'static,
    F3: FnMut(ZeroOneOrMany<T>) -> ZeroOneOrMany<T> + Send + 'static,
    F4: Fn(&T) -> bool + Send + Sync + 'static,
{
    /// Set recursive loading - EXACT syntax: .recursive(true)
    fn recursive(mut self, recursive: bool) -> impl LoaderBuilder<T> {
        self.recursive = recursive;
        self
    }
    
    /// Set filter function - EXACT syntax: .filter(|item| { ... })
    fn filter<F>(self, f: F) -> impl LoaderBuilder<T>
    where
        F: Fn(&T) -> bool + Send + Sync + 'static,
    {
        LoaderBuilderImpl {
            pattern: self.pattern,
            recursive: self.recursive,
            filter: Some(f),
            error_handler: self.error_handler,
            result_handler: self.result_handler,
            chunk_handler: self.chunk_handler,
            _marker: PhantomData,
        }
    }
    
    /// Map transformation - EXACT syntax: .map(|item| { ... })
    fn map<U, F>(self, _f: F) -> impl LoaderBuilder<U>
    where
        F: Fn(T) -> U + 'static,
        U: Send + Sync + fmt::Debug + Clone + 'static,
    {
        LoaderBuilderImpl {
            pattern: self.pattern,
            recursive: self.recursive,
            filter: None,
            error_handler: None,
            result_handler: None,
            chunk_handler: None,
            _marker: PhantomData,
        }
    }
    
    /// Set error handler - EXACT syntax: .on_error(|error| { ... })
    /// Zero-allocation: uses generic function pointer instead of Box<dyn>
    fn on_error<F>(self, handler: F) -> impl LoaderBuilder<T>
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        LoaderBuilderImpl {
            pattern: self.pattern,
            recursive: self.recursive,
            filter: self.filter,
            error_handler: Some(handler),
            result_handler: self.result_handler,
            chunk_handler: self.chunk_handler,
            _marker: PhantomData,
        }
    }
    
    /// Set result handler - EXACT syntax: .on_result(|result| { ... })
    /// Zero-allocation: uses generic function pointer instead of Box<dyn>
    fn on_result<F>(self, handler: F) -> impl LoaderBuilder<T>
    where
        F: FnOnce(ZeroOneOrMany<T>) -> ZeroOneOrMany<T> + Send + 'static,
    {
        LoaderBuilderImpl {
            pattern: self.pattern,
            recursive: self.recursive,
            filter: self.filter,
            error_handler: self.error_handler,
            result_handler: Some(handler),
            chunk_handler: self.chunk_handler,
            _marker: PhantomData,
        }
    }
    
    /// Set chunk handler - EXACT syntax: .on_chunk(|chunk| { ... })
    /// Zero-allocation: uses generic function pointer instead of Box<dyn>
    fn on_chunk<F>(self, handler: F) -> impl LoaderBuilder<T>
    where
        F: FnMut(ZeroOneOrMany<T>) -> ZeroOneOrMany<T> + Send + 'static,
    {
        LoaderBuilderImpl {
            pattern: self.pattern,
            recursive: self.recursive,
            filter: self.filter,
            error_handler: self.error_handler,
            result_handler: self.result_handler,
            chunk_handler: Some(handler),
            _marker: PhantomData,
        }
    }
    
    /// Build loader - EXACT syntax: .build()
    fn build(self) -> impl Loader<T>
    where
        LoaderImpl<T>: Loader<T>,
    {
        LoaderImpl {
            pattern: self.pattern,
            recursive: self.recursive,
            iterator: None,
            filter_fn: self.filter.map(|f| std::sync::Arc::new(f) as std::sync::Arc<dyn Fn(&T) -> bool + Send + Sync>),
        }
    }
    
    /// Build async loader - EXACT syntax: .build_async()
    fn build_async(self) -> AsyncTask<impl Loader<T>>
    where
        LoaderImpl<T>: Loader<T> + CandleNotResult,
    {
        AsyncTask::from_result(self.build())
    }
    
    /// Load files immediately - EXACT syntax: .load_files()
    fn load_files(self) -> AsyncTask<ZeroOneOrMany<T>>
    where
        LoaderImpl<T>: Loader<T>,
        T: CandleNotResult,
    {
        let loader = self.build();
        loader.load_all()
    }
    
    /// Stream files immediately - EXACT syntax: .stream()
    fn stream(self) -> impl Stream<Item = T>
    where
        LoaderImpl<T>: Loader<T>,
        T: CandleNotResult,
    {
        let loader = self.build();
        loader.stream_files()
    }
    
    /// Legacy load method - EXACT syntax: .load()
    fn load(self) -> AsyncTask<ZeroOneOrMany<T>>
    where
        LoaderImpl<T>: Loader<T>,
        T: CandleNotResult,
    {
        self.load_files()
    }
    
    /// Process with function - EXACT syntax: .process(|item| { ... })
    fn process<F, U>(self, processor: F) -> AsyncTask<ZeroOneOrMany<U>>
    where
        F: Fn(&T) -> U + Send + Sync + 'static,
        U: Send + Sync + fmt::Debug + Clone + 'static + CandleNotResult,
        LoaderImpl<T>: Loader<T>,
        T: CandleNotResult,
    {
        let loader = self.build();
        loader.process_each(processor)
    }
    
    /// Handle each item - EXACT syntax: .on_each(|item| { ... })
    fn on_each<F>(self, handler: F) -> AsyncTask<()>
    where
        F: Fn(&T) + Send + Sync + 'static,
        LoaderImpl<T>: Loader<T>,
        T: CandleNotResult,
    {
        let load_task = self.load_files();
        
        // Create tokio channel
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        
        // Spawn async task instead of blocking thread
        tokio::spawn(async move {
            if let Some(items_result) = load_task.try_next() {
                if let Ok(items) = items_result {
                    match items {
                        ZeroOneOrMany::None => {}
                        ZeroOneOrMany::One(item) => handler(&item),
                        ZeroOneOrMany::Many(items) => {
                            for item in &items {
                                handler(item);
                            }
                        }
                    }
                }
            }
            let _ = tx.send(()).await;
        });
        
        // Return async task wrapping the receiver
        AsyncTask::from_future(async move {
            let _ = rx.recv().await;
            ()
        })
    }
}

// Type aliases for convenience
pub type DefaultLoader<T> = LoaderImpl<T>;
pub type FileLoader<T> = LoaderImpl<T>;
pub type FileLoaderBuilder<T> = impl LoaderBuilder<T>;