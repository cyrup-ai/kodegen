use anyhow::Result;

use crate::runtime::{spawn_async, AsyncTask};
use crate::search::IndexingSender;

/// Trigger search index optimization
/// 
/// This function requests optimization of the search index for better performance.
/// It should be called periodically or after large batch operations.
/// 
/// # Arguments
/// * `indexing_sender` - The indexing service sender to use (required)
/// * `force` - Whether to force optimization even if not needed
/// * `on_result` - Callback invoked with the optimization result
pub fn optimize_search_index(
    indexing_sender: &IndexingSender,
    force: bool,
    on_result: impl FnOnce(Result<(), anyhow::Error>) + Send + 'static,
) -> AsyncTask<()> {
    let indexing_sender = indexing_sender.clone();
    spawn_async(async move {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let task = indexing_sender.optimize(force, move |optimize_result| {
            let _ = tx.send(optimize_result);
        });
        let _guard = crate::runtime::TaskGuard::new(task, "optimize_search_index");
        
        let result = match rx.await {
            Ok(result) => result,
            Err(_) => Err(anyhow::anyhow!("Optimization task failed to complete"))
        };
        
        on_result(result);
    })
}
