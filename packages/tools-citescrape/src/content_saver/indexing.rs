use anyhow::Result;

use crate::search::IndexingSender;

/// Trigger search index optimization
///
/// This function requests optimization of the search index for better performance.
/// It should be called periodically or after large batch operations.
///
/// # Arguments
/// * `indexing_sender` - The indexing service sender to use (required)
/// * `force` - Whether to force optimization even if not needed
pub async fn optimize_search_index(indexing_sender: &IndexingSender, force: bool) -> Result<()> {
    indexing_sender.optimize(force).await
}
