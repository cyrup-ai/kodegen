//! Per-domain concurrency limiter
//!
//! This module provides domain-level concurrency limiting to prevent
//! rate limiting and bot detection when crawling websites.

use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::{Semaphore, OwnedSemaphorePermit};

/// Per-domain concurrency limiter using lock-free DashMap
///
/// Each domain gets its own semaphore to limit concurrent requests,
/// preventing rate limiting and reducing bot detection risk.
pub struct DomainLimiter {
    domain_semaphores: DashMap<String, Arc<Semaphore>>,
    max_per_domain: usize,
}

impl DomainLimiter {
    /// Create a new domain limiter with the specified per-domain limit
    ///
    /// # Arguments
    /// * `max_per_domain` - Maximum concurrent requests per domain
    pub fn new(max_per_domain: usize) -> Self {
        Self {
            domain_semaphores: DashMap::new(),
            max_per_domain,
        }
    }
    
    /// Acquire permit for domain (creates semaphore if not exists)
    ///
    /// Returns an owned permit that will be released when dropped.
    /// The semaphore is lazily created on first access for each domain.
    ///
    /// # Arguments
    /// * `domain` - Domain to acquire permit for
    /// 
    /// # Panics
    /// Panics if the semaphore is closed (which should never happen in normal operation)
    pub async fn acquire(&self, domain: String) -> OwnedSemaphorePermit {
        let semaphore = self.domain_semaphores
            .entry(domain.clone())
            .or_insert_with(|| Arc::new(Semaphore::new(self.max_per_domain)))
            .clone();
        
        // Semaphore acquire_owned only fails if semaphore is closed.
        // We never close semaphores, so this should always succeed.
        // If it fails, the semaphore system is in an invalid state and we cannot continue.
        loop {
            match semaphore.clone().acquire_owned().await {
                Ok(permit) => return permit,
                Err(_) => {
                    // Semaphore was closed - this indicates a serious bug
                    log::error!("Semaphore for domain '{}' was closed unexpectedly - replacing", domain);
                    
                    // Replace the closed semaphore with a fresh one
                    let new_semaphore = Arc::new(Semaphore::new(self.max_per_domain));
                    self.domain_semaphores.insert(domain.clone(), new_semaphore.clone());
                    
                    // Try the new semaphore - it should succeed immediately
                    // If it doesn't, loop will retry (defensive programming)
                    match new_semaphore.acquire_owned().await {
                        Ok(permit) => return permit,
                        Err(_) => {
                            // Even new semaphore failed - extremely unlikely, retry loop
                            log::error!("Fresh semaphore also failed - retrying");
                            continue;
                        }
                    }
                }
            }
        }
    }
}
