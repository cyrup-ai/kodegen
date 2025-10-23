//! Tests for Git push operations
//!
//! Note: These tests require a remote repository to be properly tested.
//! Most tests are marked as ignored by default.

#[cfg(test)]
mod tests {
    use kodegen_tools_git::{open_repo, push, push_current_branch, push_tags, PushOpts};

    #[tokio::test]
    #[ignore] // Requires remote repository
    async fn test_push_current_branch() {
        // This test requires a real remote repository
        // Run with: cargo test test_push_current_branch -- --ignored
    }

    #[tokio::test]
    #[ignore] // Requires remote repository
    async fn test_push_with_tags() {
        // This test requires a real remote repository
        // Run with: cargo test test_push_with_tags -- --ignored
    }
}
