use super::primitives::{MemoryContent, MemoryNode, MemoryTypeEnum as MemoryType};
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

/// Memory node pool for zero-allocation `MemoryNode` reuse
pub struct MemoryNodePool {
    sender: mpsc::UnboundedSender<MemoryNode>,
    receiver: Arc<Mutex<mpsc::UnboundedReceiver<MemoryNode>>>,
    embedding_dimension: usize,
    max_capacity: usize,
}

impl MemoryNodePool {
    /// Create new memory node pool with specified capacity and embedding dimension
    #[inline]
    #[must_use]
    pub fn new(capacity: usize, embedding_dimension: usize) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();

        // Pre-allocate nodes to avoid allocations during runtime
        for _ in 0..capacity {
            let content = MemoryContent::text(String::with_capacity(1024)); // Pre-allocate string capacity
            let mut node = MemoryNode::new(MemoryType::Working, content); // Use Working type for pooled nodes

            // Set embedding if requested
            if embedding_dimension > 0 {
                let _ = node.set_embedding(vec![0.0; embedding_dimension]); // Pre-allocate embedding
            }

            let _ = sender.send(node);
        }

        Self {
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
            embedding_dimension,
            max_capacity: capacity,
        }
    }

    /// Acquire a node from the pool (zero-allocation in common case)
    ///
    /// # Errors
    ///
    /// Returns an error if node reset fails during acquisition
    #[inline]
    pub async fn acquire(&self) -> Result<PooledMemoryNode<'_>, super::primitives::MemoryError> {
        let mut receiver = self.receiver.lock().await;
        let mut node = receiver.try_recv().unwrap_or_else(|_| {
            // Fallback: create new node if pool is empty
            let content = MemoryContent::text(String::with_capacity(1024));
            let mut node = MemoryNode::new(MemoryType::Working, content);

            // Set embedding if requested
            if self.embedding_dimension > 0 {
                let _ = node.set_embedding(vec![0.0; self.embedding_dimension]);
            }

            node
        });
        drop(receiver); // Release lock before reset

        // Reset the node to clean state, reusing all allocations
        node.reset(MemoryType::Working).await?;

        Ok(PooledMemoryNode {
            node: std::mem::ManuallyDrop::new(node),
            pool: self,
            taken: false,
        })
    }

    /// Release a node back to the pool for reuse
    #[inline]
    fn release(&self, node: MemoryNode) {
        // Reset the node to a clean state for reuse
        // The reset() method preserves allocations for optimal performance

        // Return to pool (ignore if send fails)
        let _ = self.sender.send(node);
    }

    /// Get pool statistics
    #[inline]
    #[must_use]
    pub fn stats(&self) -> (usize, usize) {
        // Note: tokio mpsc doesn't expose queue length, returning max_capacity as estimate
        (self.max_capacity, self.max_capacity)
    }
}

/// Pooled memory node that automatically returns to pool on drop
pub struct PooledMemoryNode<'a> {
    node: std::mem::ManuallyDrop<MemoryNode>,
    pool: &'a MemoryNodePool,
    taken: bool,
}

impl PooledMemoryNode<'_> {
    /// Initialize the pooled node with content
    ///
    /// # Errors
    ///
    /// Returns an error if node reset or importance setting fails
    #[inline]
    pub async fn initialize(
        &mut self,
        content: String,
        memory_type: MemoryType,
    ) -> Result<(), super::primitives::MemoryError> {
        if !self.taken {
            // Reset the node to the requested type (reuses allocations)
            self.node.reset(memory_type).await?;

            // Set the content efficiently (reusing String allocation if already Text variant)
            match &mut self.node.base_memory.content {
                MemoryContent::Text(s) => {
                    s.clear();
                    s.push_str(&content);
                }
                _ => {
                    self.node.base_memory.content = MemoryContent::text(content);
                }
            }

            // Set importance based on memory type
            let importance = memory_type.base_importance();
            let _ = self.node.set_importance(importance);
        }
        Ok(())
    }

    /// Set embedding for the pooled node
    #[inline]
    pub fn set_embedding(&mut self, embedding: Vec<f32>) {
        if !self.taken {
            let _ = self.node.set_embedding(embedding);
        }
    }

    /// Get immutable reference to the inner node
    #[inline]
    pub fn as_ref(&self) -> Option<&MemoryNode> {
        if self.taken { None } else { Some(&self.node) }
    }

    /// Get mutable reference to the inner node
    #[inline]
    pub fn as_mut(&mut self) -> Option<&mut MemoryNode> {
        if self.taken {
            None
        } else {
            Some(&mut self.node)
        }
    }

    /// Take ownership of the inner node (prevents return to pool)
    #[inline]
    pub fn take(mut self) -> Option<MemoryNode> {
        if self.taken {
            None
        } else {
            self.taken = true;
            Some(std::mem::ManuallyDrop::into_inner(std::mem::replace(
                &mut self.node,
                std::mem::ManuallyDrop::new(MemoryNode::new(
                    MemoryType::Working,
                    MemoryContent::text(""),
                )),
            )))
        }
    }
}

impl std::ops::Deref for PooledMemoryNode<'_> {
    type Target = MemoryNode;

    #[inline]
    fn deref(&self) -> &Self::Target {
        // PooledMemoryNode always contains a valid node unless taken
        &self.node
    }
}

impl std::ops::DerefMut for PooledMemoryNode<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        // PooledMemoryNode always contains a valid node unless taken
        &mut self.node
    }
}

impl Drop for PooledMemoryNode<'_> {
    #[inline]
    fn drop(&mut self) {
        if !self.taken {
            let node = std::mem::ManuallyDrop::into_inner(std::mem::replace(
                &mut self.node,
                std::mem::ManuallyDrop::new(MemoryNode::new(
                    MemoryType::Working,
                    MemoryContent::text(""),
                )),
            ));
            self.pool.release(node);
        }
    }
}

/// Global memory node pool for zero-allocation operations
static MEMORY_NODE_POOL: std::sync::OnceLock<MemoryNodePool> = std::sync::OnceLock::new();

/// Initialize the global memory node pool
#[inline]
pub fn initialize_memory_node_pool(capacity: usize, embedding_dimension: usize) {
    let _ = MEMORY_NODE_POOL.set(MemoryNodePool::new(capacity, embedding_dimension));
}

/// Get a node from the global pool
#[inline]
pub async fn acquire_pooled_node()
-> Option<Result<PooledMemoryNode<'static>, super::primitives::MemoryError>> {
    match MEMORY_NODE_POOL.get() {
        Some(pool) => Some(pool.acquire().await),
        None => None,
    }
}

/// Get pool statistics from the global pool
#[inline]
#[must_use]
pub fn memory_node_pool_stats() -> Option<(usize, usize)> {
    MEMORY_NODE_POOL.get().map(MemoryNodePool::stats)
}
