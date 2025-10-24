//! Tests for transaction manager

#[cfg(test)]
mod tests {
    use crate::memory::transaction::*;
    
    #[tokio::test]
    async fn test_transaction_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
        let manager = TransactionManager::new();

        // Begin transaction
        let tx = manager
            .begin_transaction(IsolationLevel::ReadCommitted, None)
            .await?;

        let tx_impl = tx.lock().await;
        let tx_id = tx_impl.id();
        assert_eq!(tx_impl.state(), TransactionState::Active);
        drop(tx_impl);

        // Commit transaction
        manager.commit_transaction(tx_id).await?;

        // Transaction should no longer be active
        assert!(manager.get_transaction(&tx_id).await.is_none());
        Ok(())
    }
    
    #[tokio::test]
    async fn test_transaction_rollback() -> Result<(), Box<dyn std::error::Error>> {
        let manager = TransactionManager::new();

        // Begin transaction
        let tx = manager
            .begin_transaction(IsolationLevel::ReadCommitted, None)
            .await?;

        let tx_impl = tx.lock().await;
        let tx_id = tx_impl.id();
        drop(tx_impl);

        // Rollback transaction
        manager.rollback_transaction(tx_id).await?;

        // Transaction should no longer be active
        assert!(manager.get_transaction(&tx_id).await.is_none());
        Ok(())
    }
}