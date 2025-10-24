//! SIMD-optimized search algorithms
//!
//! This module implements the core search algorithms with SIMD optimization
//! for high-performance text search and matching.

use std::pin::Pin;
use tokio_stream::Stream;

use super::index::ChatSearchIndex;
use super::types::{MatchPosition, SearchResult, SearchResultMetadata};

impl ChatSearchIndex {
    /// Search with AND operator (all terms must match)
    pub fn search_and_stream(
        &self,
        terms: &[String],
        _fuzzy_matching: bool,
    ) -> Pin<Box<dyn Stream<Item = SearchResult> + Send>> {
        let self_clone = self.clone();
        let terms_clone = terms.to_vec();

        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            if terms_clone.is_empty() {
                return;
            }

            // Find documents that contain all terms
            let mut candidate_docs = None;

            for term in &terms_clone {
                if let Some(entries) = self_clone.inverted_index().get(&**term) {
                    let doc_ids: std::collections::HashSet<String> = entries
                        .value()
                        .iter()
                        .map(|entry| entry.doc_id.clone())
                        .collect();

                    candidate_docs = match candidate_docs {
                        None => Some(doc_ids),
                        Some(existing) => Some(existing.intersection(&doc_ids).cloned().collect()),
                    };
                } else {
                    // Term not found - no results
                    return;
                }
            }

            if let Some(doc_ids) = candidate_docs {
                for doc_id in doc_ids {
                    if let Some(message) = self_clone.document_store().get(&doc_id) {
                        // Get tags from tagger
                        let tags = if let Some(ref tagger) = self_clone.tagger {
                            let tag_ids = tagger.get_tags(&doc_id);
                            tag_ids
                                .iter()
                                .filter_map(|tid| {
                                    tagger.tags.get(tid).map(|e| e.value().name.clone())
                                })
                                .collect()
                        } else {
                            Vec::new()
                        };

                        let result = SearchResult {
                            message: message.value().clone(),
                            relevance_score: self_clone
                                .calculate_relevance_score(&terms_clone, &doc_id),
                            matching_terms: terms_clone.clone(),
                            highlighted_content: None,
                            tags,
                            context: Vec::new(),
                            match_positions: Self::find_match_positions(
                                &terms_clone,
                                &message.value().message.content,
                            ),
                            metadata: Some(SearchResultMetadata {
                                query_time_ms: 0.0,
                                index_version: 1,
                                total_matches: 1,
                            }),
                        };
                        let _ = tx.send(result);
                    }
                }
            }
        }))
    }

    /// Search with OR operator (any term must match)
    pub fn search_or_stream(
        &self,
        terms: &[String],
        _fuzzy_matching: bool,
    ) -> Pin<Box<dyn Stream<Item = SearchResult> + Send>> {
        let self_clone = self.clone();
        let terms_clone = terms.to_vec();

        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            let mut all_docs = std::collections::HashSet::new();

            for term in &terms_clone {
                if let Some(entries) = self_clone.inverted_index().get(&**term) {
                    for entry in entries.value() {
                        all_docs.insert(entry.doc_id.clone());
                    }
                }
            }

            for doc_id in all_docs {
                if let Some(message) = self_clone.document_store().get(&doc_id) {
                    let matching_terms: Vec<String> = terms_clone
                        .iter()
                        .filter(|term| {
                            self_clone
                                .inverted_index()
                                .get(&**term)
                                .is_some_and(|entries| {
                                    entries.value().iter().any(|e| e.doc_id == doc_id)
                                })
                        })
                        .cloned()
                        .collect();

                    // Get tags from tagger
                    let tags = if let Some(ref tagger) = self_clone.tagger {
                        let tag_ids = tagger.get_tags(&doc_id);
                        tag_ids
                            .iter()
                            .filter_map(|tid| tagger.tags.get(tid).map(|e| e.value().name.clone()))
                            .collect()
                    } else {
                        Vec::new()
                    };

                    let result = SearchResult {
                        message: message.value().clone(),
                        relevance_score: self_clone
                            .calculate_relevance_score(&matching_terms, &doc_id),
                        matching_terms,
                        highlighted_content: None,
                        tags,
                        context: Vec::new(),
                        match_positions: Self::find_match_positions(
                            &terms_clone,
                            &message.value().message.content,
                        ),
                        metadata: Some(SearchResultMetadata {
                            query_time_ms: 0.0,
                            index_version: 1,
                            total_matches: 1,
                        }),
                    };
                    let _ = tx.send(result);
                }
            }
        }))
    }

    /// Calculate relevance score using TF-IDF
    fn calculate_relevance_score(&self, terms: &[String], doc_id: &String) -> f32 {
        let mut score = 0.0;

        for term in terms {
            if let Some(tf_entry) = self.term_frequencies.get(&**term)
                && let Some(entries) = self.inverted_index().get(&**term)
            {
                for entry in entries.value() {
                    if entry.doc_id == *doc_id {
                        score += tf_entry.value().calculate_tfidf();
                        break;
                    }
                }
            }
        }

        #[allow(clippy::cast_precision_loss)]
        {
            score / terms.len() as f32
        }
    }

    /// Find match positions in content
    fn find_match_positions(terms: &[String], content: &str) -> Vec<MatchPosition> {
        let mut positions = Vec::new();
        let content_lower = content.to_lowercase();

        for term in terms {
            let term_lower = term.to_lowercase();
            let mut start = 0;

            while let Some(pos) = content_lower[start..].find(&term_lower) {
                let actual_pos = start + pos;
                positions.push(MatchPosition {
                    start: actual_pos,
                    end: actual_pos + term.len(),
                    term: term.clone(),
                });
                start = actual_pos + 1;
            }
        }

        positions.sort_by_key(|p| p.start);
        positions
    }

    /// Search with NOT operator (terms must not match)
    pub fn search_not_stream(
        &self,
        terms: &[String],
        _fuzzy_matching: bool,
    ) -> Pin<Box<dyn Stream<Item = SearchResult> + Send>> {
        let self_clone = self.clone();
        let terms_clone = terms.to_vec();

        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            let mut excluded_docs = std::collections::HashSet::new();

            // Collect all documents that contain any of the terms
            for term in &terms_clone {
                if let Some(entries) = self_clone.inverted_index().get(&**term) {
                    for entry in entries.value() {
                        excluded_docs.insert(entry.doc_id.clone());
                    }
                }
            }

            // Return all documents that don't contain any of the terms
            for entry in &self_clone.document_store {
                let doc_id = entry.key().clone();
                if !excluded_docs.contains(&doc_id) {
                    // Get tags from tagger
                    let tags = if let Some(ref tagger) = self_clone.tagger {
                        let tag_ids = tagger.get_tags(&doc_id);
                        tag_ids
                            .iter()
                            .filter_map(|tid| tagger.tags.get(tid).map(|e| e.value().name.clone()))
                            .collect()
                    } else {
                        Vec::new()
                    };

                    let result = SearchResult {
                        message: entry.value().clone(),
                        relevance_score: 1.0, // All non-matching documents have equal relevance
                        matching_terms: Vec::new(),
                        highlighted_content: None,
                        tags,
                        context: Vec::new(),
                        match_positions: Vec::new(),
                        metadata: Some(SearchResultMetadata {
                            query_time_ms: 0.0,
                            index_version: 1,
                            total_matches: 1,
                        }),
                    };
                    let _ = tx.send(result);
                }
            }
        }))
    }

    /// Search for exact phrase matching
    pub fn search_phrase_stream(
        &self,
        terms: &[String],
        _fuzzy_matching: bool,
    ) -> Pin<Box<dyn Stream<Item = SearchResult> + Send>> {
        let self_clone = self.clone();
        let terms_clone = terms.to_vec();

        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            if terms_clone.is_empty() {
                return;
            }

            // Build the phrase to search for
            let phrase: String = terms_clone
                .iter()
                .map(AsRef::as_ref)
                .collect::<Vec<_>>()
                .join(" ");

            // Search through all documents for exact phrase match
            for entry in &self_clone.document_store {
                let message = entry.value();
                let content = &message.message.content;
                let doc_id = entry.key();

                if content.to_lowercase().contains(&phrase.to_lowercase()) {
                    // Get tags from tagger
                    let tags = if let Some(ref tagger) = self_clone.tagger {
                        let tag_ids = tagger.get_tags(doc_id);
                        tag_ids
                            .iter()
                            .filter_map(|tid| tagger.tags.get(tid).map(|e| e.value().name.clone()))
                            .collect()
                    } else {
                        Vec::new()
                    };

                    let result = SearchResult {
                        message: message.clone(),
                        relevance_score: 1.0, // Exact phrase matches have high relevance
                        matching_terms: terms_clone.clone(),
                        highlighted_content: None,
                        tags,
                        context: Vec::new(),
                        match_positions: Self::find_phrase_positions(&phrase, content),
                        metadata: Some(SearchResultMetadata {
                            query_time_ms: 0.0,
                            index_version: 1,
                            total_matches: 1,
                        }),
                    };
                    let _ = tx.send(result);
                }
            }
        }))
    }

    /// Search with proximity constraint (terms within specified distance)
    pub fn search_proximity_stream(
        &self,
        terms: &[String],
        distance: u32,
        _fuzzy_matching: bool,
    ) -> Pin<Box<dyn Stream<Item = SearchResult> + Send>> {
        let self_clone = self.clone();
        let terms_clone = terms.to_vec();

        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            if terms_clone.len() < 2 {
                // Proximity search requires at least 2 terms
                return;
            }

            for entry in &self_clone.document_store {
                let message = entry.value();
                let content = &message.message.content;
                let tokens = self_clone.tokenize_with_simd(content);
                let doc_id = entry.key();

                // Check if terms appear within the specified distance
                if Self::check_proximity(&terms_clone, &tokens, distance) {
                    // Get tags from tagger
                    let tags = if let Some(ref tagger) = self_clone.tagger {
                        let tag_ids = tagger.get_tags(doc_id);
                        tag_ids
                            .iter()
                            .filter_map(|tid| tagger.tags.get(tid).map(|e| e.value().name.clone()))
                            .collect()
                    } else {
                        Vec::new()
                    };

                    let result = SearchResult {
                        message: message.clone(),
                        relevance_score: Self::calculate_proximity_score(
                            &terms_clone,
                            &tokens,
                            distance,
                        ),
                        matching_terms: terms_clone.clone(),
                        highlighted_content: None,
                        tags,
                        context: Vec::new(),
                        match_positions: Self::find_match_positions(&terms_clone, content),
                        metadata: Some(SearchResultMetadata {
                            query_time_ms: 0.0,
                            index_version: 1,
                            total_matches: 1,
                        }),
                    };
                    let _ = tx.send(result);
                }
            }
        }))
    }

    /// Find exact phrase positions in content
    fn find_phrase_positions(phrase: &str, content: &str) -> Vec<MatchPosition> {
        let mut positions = Vec::new();
        let content_lower = content.to_lowercase();
        let phrase_lower = phrase.to_lowercase();
        let mut start = 0;

        while let Some(pos) = content_lower[start..].find(&phrase_lower) {
            let actual_pos = start + pos;
            positions.push(MatchPosition {
                start: actual_pos,
                end: actual_pos + phrase.len(),
                term: phrase.to_string(),
            });
            start = actual_pos + 1;
        }

        positions
    }

    /// Check if terms appear within proximity distance
    fn check_proximity(terms: &[String], tokens: &[String], distance: u32) -> bool {
        let mut term_positions: std::collections::HashMap<String, Vec<usize>> =
            std::collections::HashMap::new();

        // Find positions of each term
        for (i, token) in tokens.iter().enumerate() {
            for term in terms {
                if token.to_lowercase() == term.to_lowercase() {
                    term_positions.entry(term.clone()).or_default().push(i);
                }
            }
        }

        // Check if we have all terms
        if term_positions.len() != terms.len() {
            return false;
        }

        // Check proximity for each combination
        let term_keys: Vec<String> = term_positions.keys().cloned().collect();
        for i in 0..term_keys.len() {
            for j in i + 1..term_keys.len() {
                let positions1 = &term_positions[&term_keys[i]];
                let positions2 = &term_positions[&term_keys[j]];

                let mut found_within_distance = false;
                for &pos1 in positions1 {
                    for &pos2 in positions2 {
                        if (i32::try_from(pos1).unwrap_or(i32::MAX)
                            - i32::try_from(pos2).unwrap_or(i32::MAX))
                        .abs()
                            <= i32::try_from(distance).unwrap_or(i32::MAX)
                        {
                            found_within_distance = true;
                            break;
                        }
                    }
                    if found_within_distance {
                        break;
                    }
                }

                if !found_within_distance {
                    return false;
                }
            }
        }

        true
    }

    /// Calculate proximity-based relevance score
    fn calculate_proximity_score(terms: &[String], tokens: &[String], distance: u32) -> f32 {
        let mut min_distance = distance as usize;
        let mut term_positions: std::collections::HashMap<String, Vec<usize>> =
            std::collections::HashMap::new();

        // Find positions of each term
        for (i, token) in tokens.iter().enumerate() {
            for term in terms {
                if token.to_lowercase() == term.to_lowercase() {
                    term_positions.entry(term.clone()).or_default().push(i);
                }
            }
        }

        // Find minimum distance between terms
        let term_keys: Vec<String> = term_positions.keys().cloned().collect();
        for i in 0..term_keys.len() {
            for j in i + 1..term_keys.len() {
                let positions1 = &term_positions[&term_keys[i]];
                let positions2 = &term_positions[&term_keys[j]];

                for &pos1 in positions1 {
                    for &pos2 in positions2 {
                        let dist = pos1.abs_diff(pos2);
                        if dist < min_distance {
                            min_distance = dist;
                        }
                    }
                }
            }
        }

        // Score inversely proportional to distance
        #[allow(clippy::cast_precision_loss)]
        {
            1.0 - (min_distance as f32 / distance as f32)
        }
    }
}
