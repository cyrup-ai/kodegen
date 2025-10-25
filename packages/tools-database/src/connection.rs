//! Database connection setup and pooling utilities
//!
//! This module provides connection pool setup with SSH tunnel support,
//! connection warmup, and configuration from ConfigManager.

use crate::{
    SSHConfig, SSHTunnel, TunnelConfig, establish_tunnel, rewrite_dsn_for_tunnel,
    ExposeSecret, SecretString,
};
use anyhow::{Result, Context};
use kodegen_tools_config::ConfigManager;
use sqlx::pool::PoolOptions;
use sqlx::AnyPool;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Warm up connection pool by pre-establishing min_connections
///
/// Concurrently acquires min_connections to force pool establishment.
/// Ensures database is reachable before tool registration.
///
/// # Errors
/// Returns error if all warmup connections fail
pub async fn warmup_pool(pool: &AnyPool, min_connections: u32) -> Result<()> {
    let start = Instant::now();

    // Acquire min_connections concurrently to force establishment
    let mut handles = Vec::new();
    for i in 0..min_connections {
        let pool_clone = pool.clone();
        let handle = tokio::spawn(async move {
            sqlx::query("SELECT 1")
                .fetch_one(&pool_clone)
                .await
                .map_err(|e| anyhow::anyhow!("Warmup connection {} failed: {}", i + 1, e))
        });
        handles.push(handle);
    }

    // Wait for all warmup queries to complete
    let mut success_count = 0;
    for (i, handle) in handles.into_iter().enumerate() {
        match handle.await {
            Ok(Ok(_)) => success_count += 1,
            Ok(Err(e)) => log::warn!("Connection {} warmup failed: {}", i + 1, e),
            Err(e) => log::warn!("Connection {} warmup task panicked: {}", i + 1, e),
        }
    }

    let elapsed = start.elapsed();

    if success_count > 0 {
        log::info!(
            "✓ Connection pool warmed up: {}/{} connections ready ({:?})",
            success_count,
            min_connections,
            elapsed
        );

        if elapsed > Duration::from_secs(2) {
            log::warn!(
                "Pool warmup was slow ({:?}), queries may have experienced high latency",
                elapsed
            );
        }

        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Pool warmup failed: 0/{} connections established",
            min_connections
        ))
    }
}

/// Database pool setup result
pub struct DatabaseConnection {
    /// Configured connection pool
    pub pool: Arc<AnyPool>,
    /// Final connection URL (possibly rewritten for tunnel)
    pub connection_url: String,
    /// SSH tunnel guard (if SSH was used)
    pub tunnel: Option<SSHTunnel>,
}

/// Setup database connection pool with optional SSH tunnel
///
/// This function:
/// 1. Establishes SSH tunnel if ssh_config provided
/// 2. Installs sqlx drivers
/// 3. Builds connection pool from ConfigManager settings
/// 4. Warms up pool with min_connections
///
/// # Arguments
/// * `config_manager` - Configuration for pool settings
/// * `dsn` - Database connection string
/// * `ssh_config` - Optional SSH tunnel configuration
///
/// # Errors
/// Returns error if tunnel setup, connection, or warmup fails
pub async fn setup_database_pool(
    config_manager: &ConfigManager,
    dsn: &str,
    ssh_config: Option<(SSHConfig, TunnelConfig)>,
) -> Result<DatabaseConnection> {
    // Establish tunnel if SSH configured
    let (final_dsn, tunnel) = if let Some((ssh_cfg, tunnel_cfg)) = ssh_config {
        let tunnel = establish_tunnel(ssh_cfg, tunnel_cfg).await?;
        let tunneled_dsn = rewrite_dsn_for_tunnel(dsn, tunnel.local_port())?;
        log::info!("✓ SSH tunnel established for database connection");
        (tunneled_dsn, Some(tunnel))
    } else {
        (SecretString::from(dsn.to_string()), None)
    };

    // Extract min_connections BEFORE pool block for warmup access
    let min_connections = config_manager
        .get_value("db_min_connections")
        .and_then(|v| match v {
            kodegen_tools_config::ConfigValue::Number(n) => Some(n as u32),
            _ => None,
        })
        .unwrap_or(2); // 2 connections default for responsiveness

    // Install database drivers for sqlx::any
    // This MUST be called before creating AnyPool or AnyConnection
    // It registers the compiled-in drivers (postgres, mysql, sqlite) based on cargo features
    sqlx::any::install_default_drivers();

    // Connect to database with timeout configuration
    let pool = {
        // Get timeout configuration from ConfigManager
        let acquire_timeout = config_manager
            .get_value("db_acquire_timeout_secs")
            .and_then(|v| match v {
                kodegen_tools_config::ConfigValue::Number(n) => {
                    Some(Duration::from_secs(n as u64))
                }
                _ => None,
            })
            .unwrap_or(Duration::from_secs(30)); // 30s default

        let idle_timeout = config_manager
            .get_value("db_idle_timeout_secs")
            .and_then(|v| match v {
                kodegen_tools_config::ConfigValue::Number(n) => {
                    Some(Duration::from_secs(n as u64))
                }
                _ => None,
            })
            .unwrap_or(Duration::from_secs(600)); // 10 minutes default

        let max_lifetime = config_manager
            .get_value("db_max_lifetime_secs")
            .and_then(|v| match v {
                kodegen_tools_config::ConfigValue::Number(n) => {
                    Some(Duration::from_secs(n as u64))
                }
                _ => None,
            })
            .unwrap_or(Duration::from_secs(1800)); // 30 minutes default

        let max_connections = config_manager
            .get_value("db_max_connections")
            .and_then(|v| match v {
                kodegen_tools_config::ConfigValue::Number(n) => Some(n as u32),
                _ => None,
            })
            .unwrap_or(10); // 10 connections default

        // Build pool with PoolOptions
        PoolOptions::new()
            .max_connections(max_connections)
            .min_connections(min_connections)
            .acquire_timeout(acquire_timeout)
            .idle_timeout(Some(idle_timeout))
            .max_lifetime(Some(max_lifetime))
            .test_before_acquire(true) // Verify connection health
            .after_connect(|conn, _meta| {
                Box::pin(async move {
                    // Simple ping to verify connection liveness
                    // This runs on NEW connections (test_before_acquire handles reused ones)
                    sqlx::query("SELECT 1").fetch_one(conn).await?;

                    // Optional: Set application name for easier monitoring
                    // Database-specific examples (commented out by default):
                    // PostgreSQL: conn.execute("SET application_name = 'kodegen'").await?;
                    // MySQL: conn.execute("SET @@session.time_zone = '+00:00'").await?;

                    Ok(())
                })
            })
            .connect(final_dsn.expose_secret())
            .await
            .context("Failed to connect to database")?
    };

    // Warmup: Force synchronous connection establishment
    warmup_pool(&pool, min_connections).await?;

    log::info!(
        "✓ Database connected ({})",
        crate::detect_database_type(final_dsn.expose_secret())?
    );

    Ok(DatabaseConnection {
        pool: Arc::new(pool),
        connection_url: final_dsn.expose_secret().to_string(),
        tunnel,
    })
}
