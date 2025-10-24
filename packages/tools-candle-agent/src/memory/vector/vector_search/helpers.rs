//! Helper utilities for vector search

/// Convert static string to Option<String> for embedding tasks
#[inline]
pub(crate) fn task_string(task: &'static str) -> Option<String> {
    Some(task.to_string())
}
