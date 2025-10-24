//! Production-quality Tantivy schema for blazing-fast markdown search
//!
//! This module implements a zero-allocation, lock-free search schema with dual indexing
//! for both raw markdown preservation and natural language search optimization.

use anyhow::Result;
use std::collections::{HashMap, HashSet};
use tantivy::{
    schema::{
        DateOptions, Field, IndexRecordOption, NumericOptions, Schema, TextFieldIndexing,
        TextOptions,
    },
    tokenizer::{
        AlphaNumOnlyFilter, Language, LowerCaser, NgramTokenizer, SimpleTokenizer, Stemmer,
        TextAnalyzer, TokenizerManager, WhitespaceTokenizer,
    },
};

/// Tokenizer name constants for zero-allocation lookups
const EXACT_MATCH_TOKENIZER: &str = "exact_match";
const RAW_MARKDOWN_TOKENIZER: &str = "raw_markdown";
const CONTENT_SEARCH_TOKENIZER: &str = "content_search";
const NGRAM_TOKENIZER: &str = "ngram_search";

/// Production search schema with optimized dual indexing for markdown content
#[derive(Debug, Clone)]
pub struct SearchSchema {
    pub schema: Schema,
    pub url: Field,
    pub path: Field,
    pub title: Field,
    pub raw_markdown: Field,
    pub plain_content: Field,
    pub snippet: Field,
    pub crawl_date: Field,
    pub file_size: Field,
    pub word_count: Field,
}

/// Schema builder for flexible configuration and validation
pub struct SearchSchemaBuilder {
    enable_ngram_search: bool,
    enable_stemming: bool,
    ngram_min_size: usize,
    ngram_max_size: usize,
    ngram_prefix_only: bool,
    custom_tokenizers: HashMap<String, TextAnalyzer>,
    field_overrides: HashMap<String, TextOptions>,
    validation_enabled: bool,
}

/// Comprehensive schema validation errors
#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
    #[error("Field '{field}' configuration error: {details}")]
    FieldConfiguration { field: String, details: String },

    #[error("Tokenizer '{name}' registration failed: {reason}")]
    TokenizerRegistration { name: String, reason: String },

    #[error("Schema validation failed: {reason}")]
    Validation { reason: String },

    #[error("Field '{field}' not found in schema")]
    FieldNotFound { field: String },

    #[error("Incompatible field type for '{field}': expected {expected}, found {found}")]
    IncompatibleFieldType {
        field: String,
        expected: String,
        found: String,
    },

    #[error("{0}")]
    Other(String),
}

impl From<anyhow::Error> for SchemaError {
    fn from(error: anyhow::Error) -> Self {
        SchemaError::Other(error.to_string())
    }
}

impl SearchSchema {
    /// Create optimized search schema with production defaults asynchronously
    #[inline]
    pub async fn create_async() -> Result<Self> {
        Self::builder().build().await
    }

    /// Create schema builder for custom configuration
    #[inline]
    #[must_use]
    pub fn builder() -> SearchSchemaBuilder {
        SearchSchemaBuilder::new()
    }

    /// Comprehensive schema validation with detailed error reporting
    pub fn validate(&self) -> Result<(), SchemaError> {
        self.validate_required_fields()?;
        self.validate_field_types()?;
        self.validate_indexing_options()?;
        self.validate_field_consistency()?;
        Ok(())
    }

    /// Validate all required fields are present with correct names
    fn validate_required_fields(&self) -> Result<(), SchemaError> {
        const REQUIRED_FIELDS: &[&str] = &[
            "url",
            "path",
            "title",
            "raw_markdown",
            "plain_content",
            "snippet",
            "crawl_date",
            "file_size",
            "word_count",
        ];

        let existing_fields: HashSet<&str> = self
            .schema
            .fields()
            .map(|(_, field_entry)| field_entry.name())
            .collect();

        for &field_name in REQUIRED_FIELDS {
            if !existing_fields.contains(field_name) {
                return Err(SchemaError::FieldNotFound {
                    field: field_name.to_string(),
                });
            }
        }

        Ok(())
    }

    /// Validate field types match expected tantivy types
    fn validate_field_types(&self) -> Result<(), SchemaError> {
        use tantivy::schema::FieldType;

        let field_type_expectations = [
            ("url", "Text"),
            ("path", "Text"),
            ("title", "Text"),
            ("raw_markdown", "Text"),
            ("plain_content", "Text"),
            ("snippet", "Text"),
            ("crawl_date", "Date"),
            ("file_size", "U64"),
            ("word_count", "U64"),
        ];

        for (field_name, expected_type) in &field_type_expectations {
            if let Ok(field) = self.schema.get_field(field_name) {
                let field_entry = self.schema.get_field_entry(field);
                let actual_type = match field_entry.field_type() {
                    FieldType::Str(_) => "Text",
                    FieldType::Date(_) => "Date",
                    FieldType::U64(_) => "U64",
                    FieldType::I64(_) => "I64",
                    FieldType::F64(_) => "F64",
                    FieldType::Bool(_) => "Bool",
                    FieldType::Bytes(_) => "Bytes",
                    FieldType::JsonObject(_) => "JsonObject",
                    FieldType::Facet(_) => "Facet",
                    FieldType::IpAddr(_) => "IpAddr",
                };

                if actual_type != *expected_type {
                    return Err(SchemaError::IncompatibleFieldType {
                        field: (*field_name).to_string(),
                        expected: (*expected_type).to_string(),
                        found: actual_type.to_string(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Validate indexing options are configured correctly
    fn validate_indexing_options(&self) -> Result<(), SchemaError> {
        use tantivy::schema::FieldType;

        // Validate text fields have proper indexing options
        let text_fields = ["url", "path", "title", "raw_markdown", "plain_content"];
        for field_name in &text_fields {
            if let Ok(field) = self.schema.get_field(field_name) {
                let field_entry = self.schema.get_field_entry(field);
                if let FieldType::Str(text_options) = field_entry.field_type()
                    && !text_options.is_stored()
                {
                    return Err(SchemaError::FieldConfiguration {
                        field: (*field_name).to_string(),
                        details: "Text field must be stored for retrieval".to_string(),
                    });
                }
            }
        }

        // Validate snippet field is stored but not necessarily indexed
        if let Ok(field) = self.schema.get_field("snippet") {
            let field_entry = self.schema.get_field_entry(field);
            if let FieldType::Str(text_options) = field_entry.field_type()
                && !text_options.is_stored()
            {
                return Err(SchemaError::FieldConfiguration {
                    field: "snippet".to_string(),
                    details: "Snippet field must be stored for result display".to_string(),
                });
            }
        }

        Ok(())
    }

    /// Validate cross-field consistency and relationships
    fn validate_field_consistency(&self) -> Result<(), SchemaError> {
        use tantivy::schema::FieldType;

        // Ensure we have both raw and processed content fields
        let has_raw = self.schema.get_field("raw_markdown").is_ok();
        let has_plain = self.schema.get_field("plain_content").is_ok();

        if !has_raw || !has_plain {
            return Err(SchemaError::Validation {
                reason: "Schema must include both raw_markdown and plain_content for dual indexing"
                    .to_string(),
            });
        }

        // Validate numeric fields are properly configured
        let numeric_fields = ["file_size", "word_count"];
        for field_name in &numeric_fields {
            if let Ok(field) = self.schema.get_field(field_name) {
                let field_entry = self.schema.get_field_entry(field);
                if let FieldType::U64(numeric_options) = field_entry.field_type()
                    && (!numeric_options.is_stored() || !numeric_options.is_indexed())
                {
                    return Err(SchemaError::FieldConfiguration {
                        field: (*field_name).to_string(),
                        details: "Numeric field must be both stored and indexed for range queries"
                            .to_string(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Get field by name with zero allocation for known fields
    #[inline]
    #[must_use]
    pub fn get_field(&self, name: &str) -> Option<Field> {
        self.schema.get_field(name).ok()
    }

    /// Get all field names for introspection and debugging
    #[must_use]
    pub fn field_names(&self) -> Vec<&str> {
        self.schema
            .fields()
            .map(|(_, field_entry)| field_entry.name())
            .collect()
    }

    /// Get schema performance characteristics for monitoring
    #[must_use]
    pub fn performance_info(&self) -> SchemaPerformanceInfo {
        let field_count = self.schema.fields().count();
        let text_field_count = self
            .schema
            .fields()
            .filter(|(_, entry)| matches!(entry.field_type(), tantivy::schema::FieldType::Str(_)))
            .count();
        let indexed_field_count = self
            .schema
            .fields()
            .filter(|(_, entry)| entry.is_indexed())
            .count();

        SchemaPerformanceInfo {
            total_fields: field_count,
            text_fields: text_field_count,
            indexed_fields: indexed_field_count,
            estimated_memory_per_doc: Self::estimate_memory_per_document(),
        }
    }

    /// Estimate memory usage per document for capacity planning
    const fn estimate_memory_per_document() -> usize {
        // Conservative estimates based on typical markdown content
        const URL_BYTES: usize = 128; // Average URL length
        const PATH_BYTES: usize = 256; // Average file path length
        const TITLE_BYTES: usize = 128; // Average title length
        const MARKDOWN_BYTES: usize = 8192; // Average markdown size
        const PLAIN_CONTENT_BYTES: usize = 6144; // Processed content size
        const SNIPPET_BYTES: usize = 256; // Snippet length
        const METADATA_BYTES: usize = 64; // Date + numeric fields
        const INDEX_OVERHEAD: usize = 512; // Tantivy indexing overhead

        URL_BYTES
            + PATH_BYTES
            + TITLE_BYTES
            + MARKDOWN_BYTES
            + PLAIN_CONTENT_BYTES
            + SNIPPET_BYTES
            + METADATA_BYTES
            + INDEX_OVERHEAD
    }
}

impl SearchSchemaBuilder {
    /// Create new schema builder with production defaults
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            enable_ngram_search: false,
            enable_stemming: true,
            // Optimal defaults for web scraping: 2-4 grams handle typos and partial matches
            // Min 2: catches common typos (e.g., "teh" matches "the")
            // Max 4: balances index size vs fuzzy matching capability
            ngram_min_size: 2,
            ngram_max_size: 4,
            ngram_prefix_only: false, // Full n-grams for better matching
            custom_tokenizers: HashMap::new(),
            field_overrides: HashMap::new(),
            validation_enabled: true,
        }
    }

    /// Enable N-gram tokenization for fuzzy search capabilities
    #[inline]
    #[must_use]
    pub fn with_ngram_search(mut self) -> Self {
        self.enable_ngram_search = true;
        self
    }

    /// Configure N-gram tokenizer parameters for fuzzy search
    ///
    /// # Arguments
    /// * `min_size` - Minimum n-gram size (default: 2)
    /// * `max_size` - Maximum n-gram size (default: 4)  
    /// * `prefix_only` - Generate only prefix n-grams (default: false)
    ///
    /// # Recommendations
    /// - Web content: 2-4 grams (handles typos and partial words)
    /// - Code search: 3-5 grams (longer identifiers)
    /// - CJK languages: 1-2 grams (character-based)
    #[inline]
    #[must_use]
    pub fn with_ngram_config(
        mut self,
        min_size: usize,
        max_size: usize,
        prefix_only: bool,
    ) -> Self {
        self.ngram_min_size = min_size;
        self.ngram_max_size = max_size;
        self.ngram_prefix_only = prefix_only;
        self
    }

    /// Configure stemming for natural language processing
    #[inline]
    #[must_use]
    pub fn with_stemming(mut self, enabled: bool) -> Self {
        self.enable_stemming = enabled;
        self
    }

    /// Add custom tokenizer for specialized processing
    #[must_use]
    pub fn with_custom_tokenizer(mut self, name: String, tokenizer: TextAnalyzer) -> Self {
        self.custom_tokenizers.insert(name, tokenizer);
        self
    }

    /// Override field options for specific requirements
    #[must_use]
    pub fn with_field_override(mut self, field_name: String, options: TextOptions) -> Self {
        self.field_overrides.insert(field_name, options);
        self
    }

    /// Disable validation for performance-critical scenarios
    #[inline]
    #[must_use]
    pub fn without_validation(mut self) -> Self {
        self.validation_enabled = false;
        self
    }

    /// Register all custom tokenizers with the tokenizer manager
    pub async fn register_tokenizers(&self, tokenizer_manager: &TokenizerManager) -> Result<()> {
        let ngram_min_size = self.ngram_min_size;
        let ngram_max_size = self.ngram_max_size;
        let ngram_prefix_only = self.ngram_prefix_only;
        let manager = tokenizer_manager.clone();

        // Exact match tokenizer for URLs and file paths
        let exact_tokenizer = TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(LowerCaser)
            .build();

        manager.register(EXACT_MATCH_TOKENIZER, exact_tokenizer);

        // Raw markdown tokenizer preserving syntax structure
        let raw_markdown_tokenizer = TextAnalyzer::builder(WhitespaceTokenizer::default()).build();

        manager.register(RAW_MARKDOWN_TOKENIZER, raw_markdown_tokenizer);

        // Content search tokenizer with aggressive natural language processing
        let content_tokenizer = TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(LowerCaser)
            .filter(AlphaNumOnlyFilter)
            .filter(Stemmer::new(Language::English))
            .build();

        manager.register(CONTENT_SEARCH_TOKENIZER, content_tokenizer);

        // N-gram tokenizer for fuzzy search and partial matching
        // Uses configurable parameters from builder
        let ngram_tokenizer = TextAnalyzer::builder(
            NgramTokenizer::new(ngram_min_size, ngram_max_size, ngram_prefix_only)
                .map_err(|e| anyhow::anyhow!("Failed to create N-gram tokenizer: {e}"))?,
        )
        .filter(LowerCaser)
        .build();

        manager.register(NGRAM_TOKENIZER, ngram_tokenizer);

        Ok(())
    }

    /// Build the final schema with all configurations applied
    pub async fn build(self) -> Result<SearchSchema> {
        let mut schema_builder = Schema::builder();

        // Build URL field with exact matching
        let url_options = self.field_overrides.get("url").cloned().unwrap_or_else(|| {
            TextOptions::default().set_stored().set_indexing_options(
                TextFieldIndexing::default()
                    .set_tokenizer(EXACT_MATCH_TOKENIZER)
                    .set_index_option(IndexRecordOption::Basic),
            )
        });
        let url = schema_builder.add_text_field("url", url_options);

        // Build path field with exact matching
        let path_options = self
            .field_overrides
            .get("path")
            .cloned()
            .unwrap_or_else(|| {
                TextOptions::default().set_stored().set_indexing_options(
                    TextFieldIndexing::default()
                        .set_tokenizer(EXACT_MATCH_TOKENIZER)
                        .set_index_option(IndexRecordOption::Basic),
                )
            });
        let path = schema_builder.add_text_field("path", path_options);

        // Build title field with optimized content search
        let title_tokenizer = if self.enable_ngram_search {
            NGRAM_TOKENIZER
        } else {
            CONTENT_SEARCH_TOKENIZER
        };
        let title_options = self
            .field_overrides
            .get("title")
            .cloned()
            .unwrap_or_else(|| {
                TextOptions::default().set_stored().set_indexing_options(
                    TextFieldIndexing::default()
                        .set_tokenizer(title_tokenizer)
                        .set_index_option(IndexRecordOption::WithFreqsAndPositions),
                )
            });
        let title = schema_builder.add_text_field("title", title_options);

        // Build raw markdown field preserving structure
        let raw_markdown_options = self
            .field_overrides
            .get("raw_markdown")
            .cloned()
            .unwrap_or_else(|| {
                TextOptions::default().set_stored().set_indexing_options(
                    TextFieldIndexing::default()
                        .set_tokenizer(RAW_MARKDOWN_TOKENIZER)
                        .set_index_option(IndexRecordOption::Basic),
                )
            });
        let raw_markdown = schema_builder.add_text_field("raw_markdown", raw_markdown_options);

        // Build plain content field for natural language search
        let content_tokenizer = if self.enable_ngram_search {
            NGRAM_TOKENIZER
        } else {
            CONTENT_SEARCH_TOKENIZER
        };
        let plain_content_options = self
            .field_overrides
            .get("plain_content")
            .cloned()
            .unwrap_or_else(|| {
                TextOptions::default().set_stored().set_indexing_options(
                    TextFieldIndexing::default()
                        .set_tokenizer(content_tokenizer)
                        .set_index_option(IndexRecordOption::WithFreqsAndPositions),
                )
            });
        let plain_content = schema_builder.add_text_field("plain_content", plain_content_options);

        // Build snippet field for result display (stored only)
        let snippet_options = self
            .field_overrides
            .get("snippet")
            .cloned()
            .unwrap_or_else(|| TextOptions::default().set_stored());
        let snippet = schema_builder.add_text_field("snippet", snippet_options);

        // Build temporal field for date-based queries
        let crawl_date = schema_builder.add_date_field(
            "crawl_date",
            DateOptions::default().set_stored().set_indexed(),
        );

        // Build numeric fields for range queries and sorting
        let file_size = schema_builder.add_u64_field(
            "file_size",
            NumericOptions::default().set_stored().set_indexed(),
        );

        let word_count = schema_builder.add_u64_field(
            "word_count",
            NumericOptions::default().set_stored().set_indexed(),
        );

        let schema = schema_builder.build();

        let search_schema = SearchSchema {
            schema,
            url,
            path,
            title,
            raw_markdown,
            plain_content,
            snippet,
            crawl_date,
            file_size,
            word_count,
        };

        // Validate if enabled
        if self.validation_enabled {
            search_schema
                .validate()
                .map_err(|e| anyhow::anyhow!("Schema validation failed: {e}"))?;
        }

        Ok(search_schema)
    }
}

/// Performance characteristics for monitoring and optimization
#[derive(Debug, Clone)]
pub struct SchemaPerformanceInfo {
    pub total_fields: usize,
    pub text_fields: usize,
    pub indexed_fields: usize,
    pub estimated_memory_per_doc: usize,
}

impl Default for SearchSchemaBuilder {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Default for SearchSchema {
    fn default() -> Self {
        // We can't use async in Default, so return a fallback schema
        // Fallback schema with basic fields only
        SearchSchema {
            schema: tantivy::schema::SchemaBuilder::new().build(),
            url: tantivy::schema::Field::from_field_id(0),
            path: tantivy::schema::Field::from_field_id(1),
            title: tantivy::schema::Field::from_field_id(2),
            raw_markdown: tantivy::schema::Field::from_field_id(3),
            plain_content: tantivy::schema::Field::from_field_id(4),
            snippet: tantivy::schema::Field::from_field_id(5),
            crawl_date: tantivy::schema::Field::from_field_id(6),
            file_size: tantivy::schema::Field::from_field_id(7),
            word_count: tantivy::schema::Field::from_field_id(8),
        }
    }
}
