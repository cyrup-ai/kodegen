//! GitHub API utilities

use crate::runtime::AsyncTask;
use std::future::Future;

/// Spawn an async task for GitHub API operations.
///
/// This is a convenience wrapper around `AsyncTask::spawn_async`
/// that maintains API consistency with Git operations.
#[inline]
pub fn spawn_task<T, F>(work: F) -> AsyncTask<T>
where
    T: Send + 'static,
    F: Future<Output = T> + Send + 'static,
{
    AsyncTask::spawn_async(work)
}
