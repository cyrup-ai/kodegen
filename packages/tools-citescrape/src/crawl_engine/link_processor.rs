//! Link processing and queue management
//!
//! This module handles extracting links from pages and managing the crawl queue.

use anyhow::Result;
use bloomfilter::Bloom;
use chromiumoxide::Page;
use log::{debug, warn};
use std::collections::VecDeque;

use super::crawl_types::CrawlQueue;
use crate::page_extractor::extractors::extract_links;

/// State for managing the crawl queue and visited URLs
/// Uses Bloom filter to prevent unbounded memory growth on large crawls
pub struct CrawlState {
    pub queue: VecDeque<CrawlQueue>,
    pub visited_urls: Bloom<String>,
    pub max_depth: u8,
}

/// Process links from the current page and add them to the crawl queue
pub async fn process_page_links(
    page: Page,
    current_item: CrawlQueue,
    crawl_state: CrawlState,
    config: &crate::config::CrawlConfig,
) -> Result<(VecDeque<CrawlQueue>, Bloom<String>)> {
    let CrawlState {
        queue,
        visited_urls,
        max_depth,
    } = crawl_state;
    let mut crawl_queue = queue;
    let config = config.clone();

    // Extract links for next depth level if we haven't reached max depth
    if current_item.depth < max_depth {
        match extract_links(page.clone()).await {
            Ok(links) => {
                let filtered_links = super::crawler::extract_valid_urls(&links, &config);
                debug!(
                    target: "citescrape::links",
                    "Found {} links on {}, {} after filtering",
                    links.len(),
                    current_item.url,
                    filtered_links.len()
                );

                // Add new links to queue (Bloom filter check - 1% false positive rate)
                for link_url in filtered_links {
                    if !visited_urls.check(&link_url) {
                        // Validate URL before adding to queue to prevent parse failures downstream
                        if url::Url::parse(&link_url).is_ok() {
                            crawl_queue.push_back(CrawlQueue {
                                url: link_url,
                                depth: current_item.depth + 1,
                            });
                        } else {
                            warn!(
                                target: "citescrape::links",
                                "Skipping invalid URL: {link_url}"
                            );
                        }
                    }
                }
            }
            Err(e) => {
                warn!(
                    target: "citescrape::links",
                    "Failed to extract links from {}: {}",
                    current_item.url,
                    e
                );
            }
        }
    }
    Ok((crawl_queue, visited_urls))
}
