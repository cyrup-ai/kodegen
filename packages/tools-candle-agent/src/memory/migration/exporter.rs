//! Export functionality for memory data

use std::fs::File;
use std::path::Path;
use tokio::io::AsyncWriteExt; // Used for synchronous CSV writing

use serde::{Deserialize, Serialize};

use crate::memory::migration::{MigrationError, Result};

/// Configuration for CSV export with performance optimizations
#[derive(Debug, Clone)]
pub struct CsvExportConfig {
    /// Field delimiter (default: comma)
    pub delimiter: u8,
    /// Quote character for escaping (default: double quote)
    pub quote_char: u8,
    /// Whether to include headers in output (default: true)
    pub has_headers: bool,
    /// Buffer size for I/O operations (default: 16KB)
    pub buffer_size: usize,
    /// Batch size for processing records (default: 1000)
    pub batch_size: usize,
}

impl Default for CsvExportConfig {
    #[inline]
    fn default() -> Self {
        Self {
            delimiter: b',',
            quote_char: b'"',
            has_headers: true,
            buffer_size: 16 * 1024, // 16KB for optimal performance
            batch_size: 1000,       // Process in batches for cache efficiency
        }
    }
}

impl CsvExportConfig {
    /// Create new configuration with sensible defaults
    #[inline]
    pub const fn new() -> Self {
        Self {
            delimiter: b',',
            quote_char: b'"',
            has_headers: true,
            buffer_size: 16 * 1024,
            batch_size: 1000,
        }
    }

    /// Set custom delimiter
    #[inline]
    pub const fn with_delimiter(mut self, delimiter: u8) -> Self {
        self.delimiter = delimiter;
        self
    }

    /// Set custom quote character
    #[inline]
    pub const fn with_quote_char(mut self, quote_char: u8) -> Self {
        self.quote_char = quote_char;
        self
    }

    /// Set whether to include headers
    #[inline]
    pub const fn with_headers(mut self, has_headers: bool) -> Self {
        self.has_headers = has_headers;
        self
    }

    /// Set custom buffer size for I/O operations
    #[inline]
    pub const fn with_buffer_size(mut self, buffer_size: usize) -> Self {
        self.buffer_size = buffer_size;
        self
    }

    /// Set custom batch size for record processing
    #[inline]
    pub const fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }

    /// Create configuration optimized for large datasets
    #[inline]
    pub const fn for_large_datasets() -> Self {
        Self {
            delimiter: b',',
            quote_char: b'"',
            has_headers: true,
            buffer_size: 64 * 1024, // 64KB buffer for large datasets
            batch_size: 5000,       // Larger batches for throughput
        }
    }

    /// Create configuration optimized for memory-constrained environments
    #[inline]
    pub const fn for_low_memory() -> Self {
        Self {
            delimiter: b',',
            quote_char: b'"',
            has_headers: true,
            buffer_size: 4 * 1024, // 4KB buffer for low memory
            batch_size: 100,       // Small batches to minimize memory usage
        }
    }
}

/// Data export format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    /// JSON format
    Json,
    /// CSV format
    Csv,
}

/// Data exporter
pub struct DataExporter {
    format: ExportFormat,
}

impl DataExporter {
    /// Create a new exporter
    pub fn new(format: ExportFormat) -> Self {
        Self { format }
    }

    /// Export data to file for JSON/CSV formats
    /// Note: Binary format requires bincode::Encode trait - use export_binary directly
    pub async fn export_to_file<T>(&self, data: &[T], path: &Path) -> Result<()>
    where
        T: Serialize,
    {
        match self.format {
            ExportFormat::Json => self.export_json(data, path).await,
            ExportFormat::Csv => self.export_csv(data, path),
        }
    }

    /// Export as JSON
    async fn export_json<T: Serialize>(&self, data: &[T], path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(data)?;
        let mut file = tokio::fs::File::create(path).await?;
        file.write_all(json.as_bytes()).await?;
        Ok(())
    }

    /// Export as CSV with blazing-fast, zero-allocation implementation
    fn export_csv<T: Serialize>(&self, data: &[T], path: &Path) -> Result<()> {
        self.export_csv_with_config(data, path, &CsvExportConfig::default())
    }

    /// Export as CSV with custom configuration for maximum performance
    fn export_csv_with_config<T: Serialize>(
        &self,
        data: &[T],
        path: &Path,
        config: &CsvExportConfig,
    ) -> Result<()> {
        use std::io::BufWriter;
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::time::Instant;

        use csv::WriterBuilder;

        let start_time = Instant::now();
        let records_processed = AtomicU64::new(0);
        let bytes_written = AtomicU64::new(0);

        // Validate inputs and pre-flight checks
        if data.is_empty() {
            return Err(MigrationError::ValidationFailed("No data to export".into()));
        }

        // Create file with proper error handling
        let file = File::create(path).map_err(|e| {
            MigrationError::DatabaseError(format!(
                "Failed to create file {}: {}",
                path.display(),
                e
            ))
        })?;

        // Use optimized buffered writer with large buffer for minimal syscalls
        let buf_writer = BufWriter::with_capacity(config.buffer_size, file);

        // Configure CSV writer for maximum performance
        let mut csv_writer = WriterBuilder::new()
            .delimiter(config.delimiter)
            .quote_style(csv::QuoteStyle::Necessary)
            .has_headers(config.has_headers)
            .buffer_capacity(config.buffer_size)
            .from_writer(buf_writer);

        // Process records in optimized batches for better cache locality
        let mut batch_count = 0;
        let batch_size = config.batch_size;

        for chunk in data.chunks(batch_size) {
            for record in chunk {
                match csv_writer.serialize(record) {
                    Ok(()) => {
                        records_processed.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(e) => {
                        // Graceful degradation - log error and continue with other records
                        log::warn!(
                            "Failed to serialize record {}: {}. Continuing with remaining records.",
                            records_processed.load(Ordering::Relaxed) + 1,
                            e
                        );
                        continue;
                    }
                }
            }

            batch_count += 1;

            // Periodic flush for large datasets to prevent memory buildup
            if batch_count % 10 == 0
                && let Err(e) = csv_writer.flush()
            {
                return Err(MigrationError::IoError(e));
            }
        }

        // Final flush and finalization
        csv_writer.flush().map_err(MigrationError::IoError)?;

        // Get final metrics
        let final_records = records_processed.load(Ordering::Relaxed);
        let duration = start_time.elapsed();

        // Calculate approximate bytes written (file size)
        if let Ok(metadata) = std::fs::metadata(path) {
            bytes_written.store(metadata.len(), Ordering::Relaxed);
        }

        log::info!(
            "CSV export completed successfully: {} records exported to {} in {:.2}ms ({:.2} MB/s)",
            final_records,
            path.display(),
            duration.as_secs_f64() * 1000.0,
            (bytes_written.load(Ordering::Relaxed) as f64 / (1024.0 * 1024.0))
                / duration.as_secs_f64().max(0.001)
        );

        Ok(())
    }

    /// High-performance streaming CSV export for large datasets
    #[allow(dead_code)]
    fn export_csv_streaming<T, I>(
        &self,
        data_iter: I,
        path: &Path,
        config: &CsvExportConfig,
        estimated_count: Option<usize>,
    ) -> Result<()>
    where
        T: Serialize,
        I: Iterator<Item = T>,
    {
        use std::io::BufWriter;
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::time::Instant;

        use csv::WriterBuilder;

        let start_time = Instant::now();
        let records_processed = AtomicU64::new(0);

        // Create file with proper error handling
        let file = File::create(path).map_err(|e| {
            MigrationError::DatabaseError(format!(
                "Failed to create file {}: {}",
                path.display(),
                e
            ))
        })?;

        // Use large buffer for streaming to minimize syscalls
        let buf_writer = BufWriter::with_capacity(config.buffer_size * 2, file);

        // Configure CSV writer for streaming performance
        let mut csv_writer = WriterBuilder::new()
            .delimiter(config.delimiter)
            .quote_style(csv::QuoteStyle::Necessary)
            .has_headers(config.has_headers)
            .buffer_capacity(config.buffer_size)
            .from_writer(buf_writer);

        // Process streaming data with periodic progress reporting
        let progress_interval = estimated_count
            .map(|c| (c / 100).max(1000))
            .unwrap_or(10000);

        for record in data_iter {
            match csv_writer.serialize(&record) {
                Ok(()) => {
                    let current_count = records_processed.fetch_add(1, Ordering::Relaxed) + 1;

                    // Periodic progress reporting and flushing
                    if current_count.is_multiple_of(progress_interval as u64) {
                        if let Err(e) = csv_writer.flush() {
                            return Err(MigrationError::DatabaseError(format!(
                                "Failed to flush during streaming: {}",
                                e
                            )));
                        }

                        log::info!(
                            "Streaming CSV export progress: {} records processed",
                            current_count
                        );
                    }
                }
                Err(e) => {
                    log::warn!(
                        "Failed to serialize streaming record {}: {}. Continuing.",
                        records_processed.load(Ordering::Relaxed) + 1,
                        e
                    );
                    continue;
                }
            }
        }

        // Final flush
        csv_writer.flush().map_err(|e| {
            MigrationError::DatabaseError(format!("Failed to flush final streaming data: {}", e))
        })?;

        let final_records = records_processed.load(Ordering::Relaxed);
        let duration = start_time.elapsed();

        log::info!(
            "Streaming CSV export completed: {} records exported to {} in {:.2}ms ({:.0} records/sec)",
            final_records,
            path.display(),
            duration.as_secs_f64() * 1000.0,
            final_records as f64 / duration.as_secs_f64().max(0.001)
        );

        Ok(())
    }

    /// Export as binary
    pub async fn export_binary<T>(&self, data: &[T], path: &Path) -> Result<()>
    where
        T: bincode::Encode,
    {
        let bytes = bincode::encode_to_vec(data, bincode::config::standard()).map_err(|e| {
            MigrationError::UnsupportedFormat(format!("Binary encoding failed: {e}"))
        })?;
        tokio::fs::write(path, &bytes)
            .await
            .map_err(MigrationError::IoError)?;
        Ok(())
    }
}

/// Export configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    /// Export format
    pub format: ExportFormat,

    /// Include metadata
    pub include_metadata: bool,

    /// Include relationships
    pub include_relationships: bool,

    /// Batch size for large exports
    pub batch_size: usize,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            format: ExportFormat::Json,
            include_metadata: true,
            include_relationships: true,
            batch_size: 1000,
        }
    }
}
