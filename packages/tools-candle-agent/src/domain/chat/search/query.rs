//! Query processing and expansion functionality

use std::collections::HashMap;

use super::types::{ProcessedQuery, QueryMetadata, QueryOperator, SearchOptions};
use crate::domain::util::duration_to_micros_u64;

/// Query processor for advanced query handling
pub struct QueryProcessor {
    /// Enable query expansion
    expansion_enabled: bool,
    /// Expansion dictionary for synonyms
    expansion_dict: HashMap<String, Vec<String>>,
}

impl QueryProcessor {
    /// Create a new query processor
    #[must_use]
    pub fn new() -> Self {
        Self {
            expansion_enabled: false,
            expansion_dict: HashMap::new(),
        }
    }

    /// Process a query string
    ///
    /// # Errors
    ///
    /// Returns error if query parsing or processing fails
    pub fn process_query(
        &self,
        query: &str,
        options: &SearchOptions,
    ) -> Result<ProcessedQuery, Box<dyn std::error::Error + Send + Sync>> {
        let start_time = std::time::Instant::now();

        let terms: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();

        let expanded_terms = if options.enable_query_expansion {
            Self::expand_terms_sync(&terms, &options.expansion_dictionary)
        } else {
            Vec::new()
        };

        let result = ProcessedQuery {
            original: query.to_string(),
            terms,
            expanded_terms,
            operator: QueryOperator::And,
            metadata: QueryMetadata {
                processed_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                processing_time_us: duration_to_micros_u64(start_time.elapsed()),
                expansion_applied: options.enable_query_expansion,
                normalization_applied: true,
            },
        };

        Ok(result)
    }

    /// Expand query terms using synonyms
    fn expand_terms_sync(
        terms: &[String],
        dictionary: &HashMap<String, Vec<String>>,
    ) -> Vec<String> {
        let mut expanded = Vec::new();

        for term in terms {
            if let Some(synonyms) = dictionary.get(term) {
                expanded.extend(synonyms.clone());
            }
        }

        expanded
    }
}

impl Default for QueryProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for QueryProcessor {
    fn clone(&self) -> Self {
        Self {
            expansion_enabled: self.expansion_enabled,
            expansion_dict: self.expansion_dict.clone(),
        }
    }
}
