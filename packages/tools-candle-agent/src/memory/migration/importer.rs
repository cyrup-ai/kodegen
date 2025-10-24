//! Import functionality for memory data

use std::path::Path;
use tokio::io::AsyncReadExt;

use serde::{Deserialize, Serialize};

use crate::memory::migration::{MigrationError, Result};

/// Data importer
pub struct DataImporter;

impl DataImporter {
    /// Create a new importer
    pub fn new() -> Self {
        Self
    }

    /// Import data from JSON file
    pub async fn import_json<T>(&self, path: &Path) -> Result<Vec<T>>
    where
        T: for<'de> Deserialize<'de> + Send + 'static,
    {
        let mut file = tokio::fs::File::open(path).await?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).await?;

        let data: Vec<T> = serde_json::from_str(&contents)?;
        Ok(data)
    }

    /// Import data from CSV file with production-ready implementation
    pub async fn import_csv<T>(&self, path: &Path) -> Result<Vec<T>>
    where
        T: for<'de> Deserialize<'de> + Send + 'static,
    {
        use csv::ReaderBuilder;
        use tokio::fs::File;
        use tokio::io::AsyncReadExt;

        // Validate file exists and is readable
        if !path.exists() {
            return Err(MigrationError::FileNotFound(
                path.to_string_lossy().to_string(),
            ));
        }

        // Read file contents asynchronously
        let mut file = File::open(path).await?;

        let mut contents = String::new();
        file.read_to_string(&mut contents).await?;

        // Parse CSV with proper error handling
        let mut reader = ReaderBuilder::new()
            .has_headers(true)
            .flexible(true)
            .trim(csv::Trim::All)
            .from_reader(contents.as_bytes());

        let mut records = Vec::new();
        let mut line_number = 1; // Start from 1 for header

        for result in reader.deserialize() {
            line_number += 1;
            match result {
                Ok(record) => records.push(record),
                Err(e) => {
                    log::warn!("Failed to parse CSV record at line {}: {}", line_number, e);
                    // Continue processing other records instead of failing completely
                    continue;
                }
            }
        }

        log::info!(
            "Successfully imported {} records from CSV file: {}",
            records.len(),
            path.display()
        );

        Ok(records)
    }

    /// Import data from binary file
    pub async fn import_binary<T>(&self, path: &Path) -> Result<Vec<T>>
    where
        T: bincode::Decode<()>,
    {
        let mut file = tokio::fs::File::open(path).await?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await?;

        let data: Vec<T> = bincode::decode_from_slice(&buffer, bincode::config::standard())
            .map_err(|e| MigrationError::UnsupportedFormat(format!("Binary decoding failed: {e}")))?
            .0;
        Ok(data)
    }

    /// Import with validation for JSON/CSV formats only
    /// Note: Binary format requires separate validation due to different trait bounds
    pub async fn import_with_validation<T, F>(
        &self,
        path: &Path,
        format: ImportFormat,
        validator: F,
    ) -> Result<Vec<T>>
    where
        T: for<'de> Deserialize<'de> + Send + 'static + std::fmt::Debug,
        F: Fn(&T) -> Result<()> + Send + 'static,
    {
        let data = match format {
            ImportFormat::Json => self.import_json(path).await?,
            ImportFormat::Csv => self.import_csv(path).await?,
        };

        // Validate each item
        for item in &data {
            validator(item)?;
        }

        Ok(data)
    }
}

impl Default for DataImporter {
    fn default() -> Self {
        Self::new()
    }
}

/// Import format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportFormat {
    /// JSON format
    Json,
    /// CSV format
    Csv,
}

/// Import configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportConfig {
    /// Import format
    pub format: ImportFormat,

    /// Skip validation
    pub skip_validation: bool,

    /// Batch size for large imports
    pub batch_size: usize,

    /// Continue on error
    pub continue_on_error: bool,
}

impl Default for ImportConfig {
    fn default() -> Self {
        Self {
            format: ImportFormat::Json,
            skip_validation: false,
            batch_size: 1000,
            continue_on_error: false,
        }
    }
}
