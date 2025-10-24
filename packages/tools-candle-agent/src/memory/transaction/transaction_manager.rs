//! Transaction management implementation

use std::collections::HashMap;
use std::sync::Arc;

use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use tokio::sync::oneshot;
use tokio::sync::{Mutex, RwLock};

use crate::memory::transaction::{
    IsolationLevel, PendingCommit, PendingRollback, Result, Transaction, TransactionContext,
    TransactionError, TransactionState,
};

/// Transaction manager for coordinating transactions
pub struct TransactionManager {
    /// Database connection
    db: Arc<Surreal<Any>>,

    /// Active transactions
    active_transactions: Arc<RwLock<HashMap<String, Arc<Mutex<TransactionImpl>>>>>,

    /// Transaction log
    transaction_log: Arc<Mutex<Vec<TransactionLogEntry>>>,

    /// Lock manager
    lock_manager: Arc<LockManager>,
}

/// Transaction implementation
pub struct TransactionImpl {
    /// Transaction context
    pub context: TransactionContext,

    /// Transaction state
    pub state: TransactionState,

    /// Operations performed in this transaction
    pub operations: Vec<Operation>,

    /// Locks held by this transaction
    pub locks: Vec<Lock>,
}

/// Operation performed in a transaction
#[derive(Debug, Clone)]
pub enum Operation {
    /// Insert operation
    Insert {
        table: String,
        id: String,
        data: serde_json::Value,
    },

    /// Update operation
    Update {
        table: String,
        id: String,
        data: serde_json::Value,
    },

    /// Delete operation
    Delete { table: String, id: String },
}

/// Lock types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockType {
    /// Shared lock (for reads)
    Shared,

    /// Exclusive lock (for writes)
    Exclusive,
}

/// Lock held by a transaction
#[derive(Debug, Clone)]
pub struct Lock {
    /// Resource identifier
    pub resource: String,

    /// Lock type
    pub lock_type: LockType,

    /// Transaction holding the lock
    pub transaction_id: String,
}

/// Lock manager for handling concurrent access
struct LockManager {
    /// Locks by resource
    locks: RwLock<HashMap<String, Vec<Lock>>>,
}

/// Transaction log entry
#[derive(Debug, Clone)]
struct TransactionLogEntry {
    /// Transaction ID
    transaction_id: String,

    /// Timestamp
    timestamp: chrono::DateTime<chrono::Utc>,

    /// Action
    action: TransactionAction,
}

/// Transaction actions for logging
#[derive(Debug, Clone)]
enum TransactionAction {
    Begin,
    Commit,
    Rollback,
    #[allow(dead_code)]
    Abort(String),
}

impl TransactionManager {
    /// Create a new transaction manager
    pub fn new(db: Arc<Surreal<Any>>) -> Self {
        Self {
            db,
            active_transactions: Arc::new(RwLock::new(HashMap::new())),
            transaction_log: Arc::new(Mutex::new(Vec::new())),
            lock_manager: Arc::new(LockManager::new()),
        }
    }

    /// Begin a new transaction
    pub async fn begin_transaction(
        &self,
        isolation_level: IsolationLevel,
        timeout: Option<std::time::Duration>,
    ) -> Result<Arc<Mutex<TransactionImpl>>> {
        let context = TransactionContext {
            id: uuid::Uuid::new_v4().to_string(),
            isolation_level,
            started_at: std::time::Instant::now(),
            timeout,
        };

        let transaction = Arc::new(Mutex::new(TransactionImpl {
            context: context.clone(),
            state: TransactionState::Active,
            operations: Vec::new(),
            locks: Vec::new(),
        }));

        // Add to active transactions
        let transaction_id = context.id.clone();
        self.active_transactions
            .write()
            .await
            .insert(context.id, transaction.clone());

        // Log transaction begin
        self.log_action(transaction_id, TransactionAction::Begin)
            .await;

        Ok(transaction)
    }

    /// Get a transaction by ID
    pub async fn get_transaction(&self, id: &str) -> Option<Arc<Mutex<TransactionImpl>>> {
        self.active_transactions.read().await.get(id).cloned()
    }

    /// Commit a transaction
    pub async fn commit_transaction(&self, id: String) -> Result<()> {
        let transaction = self.active_transactions.write().await.remove(&id).ok_or(
            TransactionError::InvalidState("Transaction not found".to_string()),
        )?;

        let mut tx = transaction.lock().await;

        // Check state
        if tx.state != TransactionState::Active {
            return Err(TransactionError::InvalidState(format!(
                "Cannot commit transaction in state {:?}",
                tx.state
            )));
        }

        // Change state
        tx.state = TransactionState::Committing;

        // Execute operations and capture result
        let commit_result = async {
            let sdk_tx =
                self.db.transaction().await.map_err(|e| {
                    TransactionError::DatabaseError(format!("BEGIN failed: {:?}", e))
                })?;

            for operation in &tx.operations {
                match operation {
                    Operation::Insert { table, id, data } => {
                        sdk_tx
                            .create::<Option<serde_json::Value>>((table.as_str(), id.as_str()))
                            .content(data.clone())
                            .await
                            .map_err(|e| {
                                TransactionError::DatabaseError(format!("INSERT failed: {:?}", e))
                            })?;
                    }
                    Operation::Update { table, id, data } => {
                        sdk_tx
                            .update::<Option<serde_json::Value>>((table.as_str(), id.as_str()))
                            .content(data.clone())
                            .await
                            .map_err(|e| {
                                TransactionError::DatabaseError(format!("UPDATE failed: {:?}", e))
                            })?;
                    }
                    Operation::Delete { table, id } => {
                        sdk_tx
                            .delete::<Option<serde_json::Value>>((table.as_str(), id.as_str()))
                            .await
                            .map_err(|e| {
                                TransactionError::DatabaseError(format!("DELETE failed: {:?}", e))
                            })?;
                    }
                }
            }

            sdk_tx
                .commit()
                .await
                .map_err(|e| TransactionError::DatabaseError(format!("COMMIT failed: {:?}", e)))?;

            Ok::<(), TransactionError>(())
        }
        .await;

        // ALWAYS release locks regardless of success/failure
        let id_for_log = id.clone();
        for lock in &tx.locks {
            let _ = self
                .lock_manager
                .release_lock(&lock.resource, id.clone())
                .await;
        }

        // Now handle the result
        match commit_result {
            Ok(()) => {
                tx.state = TransactionState::Committed;
                self.log_action(id_for_log, TransactionAction::Commit).await;
                Ok(())
            }
            Err(e) => {
                tx.state = TransactionState::Aborted;
                self.log_action(id_for_log, TransactionAction::Rollback)
                    .await;
                Err(e)
            }
        }
    }

    /// Rollback a transaction
    pub async fn rollback_transaction(&self, id: String) -> Result<()> {
        let transaction = self.active_transactions.write().await.remove(&id).ok_or(
            TransactionError::InvalidState("Transaction not found".to_string()),
        )?;

        let mut tx = transaction.lock().await;

        // Check state
        if tx.state != TransactionState::Active {
            return Err(TransactionError::InvalidState(format!(
                "Cannot rollback transaction in state {:?}",
                tx.state
            )));
        }

        // Change state
        tx.state = TransactionState::RollingBack;

        // No database rollback needed - operations are only executed during commit
        // SDK transaction handles automatic rollback if commit fails

        // Release all locks
        let id_for_log = id.clone();
        for lock in &tx.locks {
            self.lock_manager
                .release_lock(&lock.resource, id.clone())
                .await?;
        }

        // Mark as aborted
        tx.state = TransactionState::Aborted;

        // Log rollback
        self.log_action(id_for_log, TransactionAction::Rollback)
            .await;

        Ok(())
    }

    /// Abort a transaction due to a critical error or timeout
    pub async fn abort_transaction(&self, id: String, reason: String) -> Result<()> {
        let transaction = self.active_transactions.write().await.remove(&id).ok_or(
            TransactionError::InvalidState("Transaction not found".to_string()),
        )?;

        let mut tx = transaction.lock().await;

        // Check state - abort can happen from any active state
        if matches!(
            tx.state,
            TransactionState::Committed | TransactionState::Aborted
        ) {
            return Err(TransactionError::InvalidState(format!(
                "Cannot abort transaction in state {:?}",
                tx.state
            )));
        }

        // Change state
        tx.state = TransactionState::Aborting;

        // Force release all locks (more aggressive than rollback)
        for lock in &tx.locks {
            self.lock_manager
                .release_lock(&lock.resource, id.clone())
                .await
                .ok();
        }
        tx.locks.clear();

        // Mark as aborted
        tx.state = TransactionState::Aborted;

        let id_for_log = id.clone();

        // Log abort with reason
        self.log_action(id_for_log, TransactionAction::Abort(reason))
            .await;

        Ok(())
    }

    /// Acquire a lock for a transaction
    pub async fn acquire_lock(
        &self,
        transaction_id: String,
        resource: String,
        lock_type: LockType,
    ) -> Result<()> {
        self.lock_manager
            .acquire_lock(resource.clone(), lock_type, transaction_id.clone())
            .await?;

        // Add to transaction's lock list
        if let Some(transaction) = self.get_transaction(&transaction_id).await {
            let mut tx = transaction.lock().await;
            tx.locks.push(Lock {
                resource,
                lock_type,
                transaction_id,
            });
        }

        Ok(())
    }

    /// Add insert operation to transaction
    pub async fn add_insert(
        &self,
        transaction_id: &str,
        table: String,
        id: String,
        data: serde_json::Value,
    ) -> Result<()> {
        let transaction = self
            .get_transaction(transaction_id)
            .await
            .ok_or_else(|| TransactionError::InvalidState("Transaction not found".to_string()))?;

        let mut tx = transaction.lock().await;

        if tx.state != TransactionState::Active {
            return Err(TransactionError::InvalidState(format!(
                "Cannot add operation to transaction in state {:?}",
                tx.state
            )));
        }

        tx.operations.push(Operation::Insert { table, id, data });
        Ok(())
    }

    /// Add update operation to transaction
    pub async fn add_update(
        &self,
        transaction_id: &str,
        table: String,
        id: String,
        data: serde_json::Value,
    ) -> Result<()> {
        let transaction = self
            .get_transaction(transaction_id)
            .await
            .ok_or_else(|| TransactionError::InvalidState("Transaction not found".to_string()))?;

        let mut tx = transaction.lock().await;

        if tx.state != TransactionState::Active {
            return Err(TransactionError::InvalidState(format!(
                "Cannot add operation to transaction in state {:?}",
                tx.state
            )));
        }

        tx.operations.push(Operation::Update { table, id, data });
        Ok(())
    }

    /// Add delete operation to transaction
    pub async fn add_delete(&self, transaction_id: &str, table: String, id: String) -> Result<()> {
        let transaction = self
            .get_transaction(transaction_id)
            .await
            .ok_or_else(|| TransactionError::InvalidState("Transaction not found".to_string()))?;

        let mut tx = transaction.lock().await;

        if tx.state != TransactionState::Active {
            return Err(TransactionError::InvalidState(format!(
                "Cannot add operation to transaction in state {:?}",
                tx.state
            )));
        }

        tx.operations.push(Operation::Delete { table, id });
        Ok(())
    }

    /// Log a transaction action
    /// Get transaction history for debugging and monitoring
    pub async fn get_transaction_history(&self) -> Vec<String> {
        let log = self.transaction_log.lock().await;
        log.iter()
            .map(|entry| {
                format!(
                    "[{}] Transaction {} - {:?}",
                    entry.timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
                    entry.transaction_id,
                    entry.action
                )
            })
            .collect()
    }

    /// Get transaction logs for a specific transaction ID
    pub async fn get_transaction_logs(&self, target_id: &str) -> Vec<String> {
        let log = self.transaction_log.lock().await;
        log.iter()
            .filter(|entry| entry.transaction_id == target_id)
            .map(|entry| {
                format!(
                    "[{}] {:?}",
                    entry.timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
                    entry.action
                )
            })
            .collect()
    }

    async fn log_action(&self, transaction_id: String, action: TransactionAction) {
        let entry = TransactionLogEntry {
            transaction_id,
            timestamp: chrono::Utc::now(),
            action,
        };

        self.transaction_log.lock().await.push(entry);
    }
}

impl Default for TransactionManager {
    fn default() -> Self {
        // NOTE: Default cannot provide a database connection
        // Callers should use TransactionManager::new(db) instead
        panic!("TransactionManager requires database connection. Use TransactionManager::new(db)")
    }
}

impl LockManager {
    /// Create a new lock manager
    fn new() -> Self {
        Self {
            locks: RwLock::new(HashMap::new()),
        }
    }

    /// Acquire a lock
    async fn acquire_lock(
        &self,
        resource: String,
        lock_type: LockType,
        transaction_id: String,
    ) -> Result<()> {
        let mut locks = self.locks.write().await;
        let resource_locks = locks.entry(resource.clone()).or_insert_with(Vec::new);

        // Check for conflicts
        for existing_lock in resource_locks.iter() {
            if existing_lock.transaction_id != transaction_id {
                match (existing_lock.lock_type, lock_type) {
                    (LockType::Exclusive, _) | (_, LockType::Exclusive) => {
                        return Err(TransactionError::Conflict(format!(
                            "Resource {} is locked",
                            resource
                        )));
                    }
                    _ => {} // Shared locks are compatible
                }
            }
        }

        // Add the lock
        resource_locks.push(Lock {
            resource,
            lock_type,
            transaction_id,
        });

        Ok(())
    }

    /// Release a lock
    async fn release_lock(&self, resource: &str, transaction_id: String) -> Result<()> {
        let mut locks = self.locks.write().await;

        if let Some(resource_locks) = locks.get_mut(resource) {
            resource_locks.retain(|lock| lock.transaction_id != transaction_id);

            // Remove empty entries
            if resource_locks.is_empty() {
                locks.remove(resource);
            }
        }

        Ok(())
    }
}

impl Transaction for TransactionImpl {
    fn id(&self) -> String {
        self.context.id.clone()
    }

    fn state(&self) -> TransactionState {
        self.state
    }

    fn isolation_level(&self) -> IsolationLevel {
        self.context.isolation_level
    }

    fn commit(self) -> PendingCommit {
        let (tx, rx) = oneshot::channel();

        tokio::spawn(async move {
            // This would be handled by the TransactionManager
            let _ = tx.send(Ok(()));
        });

        PendingCommit::new(rx)
    }

    fn rollback(self) -> PendingRollback {
        let (tx, rx) = oneshot::channel();

        tokio::spawn(async move {
            // This would be handled by the TransactionManager
            let _ = tx.send(Ok(()));
        });

        PendingRollback::new(rx)
    }
}
