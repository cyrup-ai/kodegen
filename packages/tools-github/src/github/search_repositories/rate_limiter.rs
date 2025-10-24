//! Rate limiting support

use crate::github::search_repositories::types::{SearchError, SearchResult};
use chrono::{DateTime, Utc};
use log::info;
use std::time::Duration;
use tokio::time::sleep;

/// Rate limiting support
pub(crate) struct RateLimiter {
    pub(crate) remaining: u32,
    pub(crate) reset_time: DateTime<Utc>,
    pub(crate) last_check: DateTime<Utc>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            remaining: 5000,
            reset_time: Utc::now() + chrono::Duration::hours(1),
            last_check: Utc::now(),
        }
    }

    pub fn update(&mut self, remaining: u32, reset_time: DateTime<Utc>) {
        self.remaining = remaining;
        self.reset_time = reset_time;
        self.last_check = Utc::now();
    }

    pub fn can_make_request(&self) -> bool {
        self.remaining > 0 || Utc::now() > self.reset_time
    }

    pub fn check_and_reset_if_expired(&mut self) {
        // If we're past the reset time, refresh the rate limit to default
        if Utc::now() > self.reset_time {
            self.remaining = 5000;
            self.reset_time = Utc::now() + chrono::Duration::hours(1);
            self.last_check = Utc::now();
        }
    }

    pub async fn wait_if_needed(&self, buffer: u32) -> SearchResult<()> {
        if self.remaining <= buffer && Utc::now() < self.reset_time {
            let wait_time = (self.reset_time - Utc::now())
                .to_std()
                .unwrap_or(Duration::from_secs(60));

            if wait_time > Duration::from_secs(300) {
                return Err(SearchError::RateLimitExceeded {
                    remaining: self.remaining,
                    reset_time: self.reset_time,
                });
            }

            info!("Rate limit approaching, waiting {wait_time:?}");
            sleep(wait_time).await;
        }
        Ok(())
    }
}
