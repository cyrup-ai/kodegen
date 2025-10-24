use crate::types::ThoughtNode;
use lru::LruCache;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct StateManager {
    cache: Arc<Mutex<LruCache<String, ThoughtNode>>>,
    nodes: Arc<Mutex<HashMap<String, ThoughtNode>>>,
}

impl StateManager {
    pub fn new(cache_size: usize) -> Self {
        let cache_size = NonZeroUsize::new(cache_size).unwrap_or_else(|| {
            // This should never fail since 1 is always non-zero
            unsafe { NonZeroUsize::new_unchecked(1) }
        });
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(cache_size))),
            nodes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn get_node(&self, id: &str) -> Option<ThoughtNode> {
        // Fast path: cache-only check (no storage lock)
        {
            let mut cache = self.cache.lock().await;
            if let Some(node) = cache.get(id) {
                return Some(node.clone());
            }
        }
        
        // Cache miss: atomic storage read + cache update
        let nodes = self.nodes.lock().await;
        let node_opt = nodes.get(id).map(|n| n.clone());
        
        if let Some(ref node_data) = node_opt {
            let mut cache = self.cache.lock().await;
            cache.put(id.to_string(), node_data.clone());
        }
        
        node_opt
    }

    pub async fn save_node(&self, node: ThoughtNode) {
        let node_id = node.id.clone();
        
        // Atomic update: lock both in consistent order
        let mut nodes = self.nodes.lock().await;
        let mut cache = self.cache.lock().await;
        
        nodes.insert(node_id.clone(), node.clone());
        cache.put(node_id, node);
    }

    pub async fn get_children(&self, node_id: &str) -> Vec<ThoughtNode> {
        let node = match self.get_node(node_id).await {
            Some(n) => n,
            None => return vec![],
        };

        let mut children = vec![];
        for id in &node.children {
            if let Some(child) = self.get_node(id).await {
                children.push(child);
            }
        }

        children
    }

    pub async fn get_path(&self, node_id: &str) -> Vec<ThoughtNode> {
        let nodes = self.nodes.lock().await;
        let mut path = Vec::new();
        let mut current_id = node_id;

        while !current_id.is_empty() {
            match nodes.get(current_id) {
                Some(node) => {
                    path.push(node.clone());
                    current_id = node.parent_id.as_deref().unwrap_or("");
                }
                None => break,
            }
        }
        
        path.reverse();
        path
    }

    pub async fn get_all_nodes(&self) -> Vec<ThoughtNode> {
        let nodes = self.nodes.lock().await;
        nodes.values().cloned().collect()
    }

    pub async fn clear(&self) {
        // Atomic clear: lock both in consistent order
        let mut nodes = self.nodes.lock().await;
        let mut cache = self.cache.lock().await;
        
        nodes.clear();
        cache.clear();
    }
}
