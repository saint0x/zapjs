//! IPC Connection Pool
//!
//! Provides a pool of persistent IPC connections to the TypeScript runtime.
//! This eliminates per-request connection overhead, significantly improving
//! throughput for handler invocations.
//!
//! Features:
//! - Pool of N persistent connections (default: 4)
//! - Health checks before use
//! - Automatic reconnection on failure
//! - Connection timeout handling
//! - Fair connection distribution

use crate::error::{ZapError, ZapResult};
use crate::ipc::{IpcClient, IpcEncoding, IpcMessage};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, Semaphore};
use tracing::{debug, error, warn};

/// Default number of connections in the pool
const DEFAULT_POOL_SIZE: usize = 4;

/// Default connection timeout in seconds
const DEFAULT_CONNECT_TIMEOUT_SECS: u64 = 5;

/// Default health check interval in seconds
const HEALTH_CHECK_INTERVAL_SECS: u64 = 30;

/// A pooled connection wrapper
struct PooledConnection {
    client: Option<IpcClient>,
    last_used: std::time::Instant,
    healthy: bool,
}

impl PooledConnection {
    fn new() -> Self {
        Self {
            client: None,
            last_used: std::time::Instant::now(),
            healthy: false,
        }
    }

    fn is_valid(&self) -> bool {
        self.client.is_some() && self.healthy
    }
}

/// Configuration for the connection pool
#[derive(Clone)]
pub struct PoolConfig {
    /// Number of connections in the pool
    pub size: usize,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Socket path for IPC
    pub socket_path: String,
    /// IPC encoding format
    pub encoding: IpcEncoding,
    /// Health check interval
    pub health_check_interval: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            size: DEFAULT_POOL_SIZE,
            connect_timeout: Duration::from_secs(DEFAULT_CONNECT_TIMEOUT_SECS),
            socket_path: String::new(),
            encoding: IpcEncoding::default(),
            health_check_interval: Duration::from_secs(HEALTH_CHECK_INTERVAL_SECS),
        }
    }
}

impl PoolConfig {
    /// Create a new pool configuration with the given socket path
    pub fn new(socket_path: String) -> Self {
        Self {
            socket_path,
            ..Default::default()
        }
    }

    /// Set the pool size
    pub fn size(mut self, size: usize) -> Self {
        self.size = size;
        self
    }

    /// Set the connect timeout
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Set the encoding format
    pub fn encoding(mut self, encoding: IpcEncoding) -> Self {
        self.encoding = encoding;
        self
    }
}

/// IPC Connection Pool
///
/// Manages a pool of persistent connections to the TypeScript IPC server.
/// Connections are reused across requests to eliminate connection overhead.
pub struct ConnectionPool {
    /// Pool configuration
    config: PoolConfig,
    /// Pooled connections (each wrapped in Mutex for exclusive access)
    connections: Vec<Arc<Mutex<PooledConnection>>>,
    /// Semaphore to limit concurrent connection acquisition
    semaphore: Arc<Semaphore>,
    /// Round-robin index for fair distribution
    next_index: AtomicUsize,
    /// Whether the pool is initialized
    initialized: std::sync::atomic::AtomicBool,
}

impl ConnectionPool {
    /// Create a new connection pool with the given configuration
    pub fn new(config: PoolConfig) -> Self {
        let connections = (0..config.size)
            .map(|_| Arc::new(Mutex::new(PooledConnection::new())))
            .collect();

        Self {
            semaphore: Arc::new(Semaphore::new(config.size)),
            connections,
            config,
            next_index: AtomicUsize::new(0),
            initialized: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Create a pool with the given socket path and default settings
    pub fn with_socket(socket_path: String) -> Self {
        Self::new(PoolConfig::new(socket_path))
    }

    /// Initialize the connection pool by establishing all connections
    pub async fn initialize(&self) -> ZapResult<()> {
        if self.initialized.load(Ordering::Acquire) {
            return Ok(());
        }

        debug!("Initializing connection pool with {} connections", self.config.size);

        let mut init_count = 0;
        for (i, conn_mutex) in self.connections.iter().enumerate() {
            let mut conn = conn_mutex.lock().await;
            match self.create_connection().await {
                Ok(client) => {
                    conn.client = Some(client);
                    conn.healthy = true;
                    conn.last_used = std::time::Instant::now();
                    init_count += 1;
                    debug!("Connection {} initialized", i);
                }
                Err(e) => {
                    warn!("Failed to initialize connection {}: {}", i, e);
                    // Continue - we'll try to reconnect later
                }
            }
        }

        if init_count == 0 {
            return Err(ZapError::ipc("Failed to initialize any pool connections"));
        }

        self.initialized.store(true, Ordering::Release);
        debug!("Connection pool initialized with {}/{} connections", init_count, self.config.size);

        Ok(())
    }

    /// Create a new IPC connection
    async fn create_connection(&self) -> ZapResult<IpcClient> {
        let timeout = self.config.connect_timeout;

        tokio::time::timeout(
            timeout,
            IpcClient::connect_with_encoding(&self.config.socket_path, self.config.encoding),
        )
        .await
        .map_err(|_| ZapError::timeout("Connection pool connect timeout", timeout.as_millis() as u64))?
    }

    /// Get a connection from the pool, reconnecting if necessary
    async fn get_connection_index(&self) -> ZapResult<usize> {
        // Round-robin selection with wrap-around
        let index = self.next_index.fetch_add(1, Ordering::Relaxed) % self.config.size;
        Ok(index)
    }

    /// Execute a request-response operation using a pooled connection
    ///
    /// This method handles:
    /// - Connection acquisition from pool
    /// - Automatic reconnection on failure
    /// - Connection release back to pool
    pub async fn send_recv(&self, message: IpcMessage) -> ZapResult<IpcMessage> {
        // Acquire semaphore permit (limits concurrent usage)
        let _permit = self.semaphore.acquire().await.map_err(|_| {
            ZapError::ipc("Connection pool semaphore closed")
        })?;

        // Get a connection index
        let index = self.get_connection_index().await?;
        let conn_mutex = &self.connections[index];

        // Try with the existing connection first
        let mut conn = conn_mutex.lock().await;

        // Check if connection is valid
        if !conn.is_valid() {
            debug!("Connection {} invalid, reconnecting", index);
            match self.create_connection().await {
                Ok(client) => {
                    conn.client = Some(client);
                    conn.healthy = true;
                }
                Err(e) => {
                    conn.healthy = false;
                    return Err(e);
                }
            }
        }

        // Send and receive
        if let Some(client) = &mut conn.client {
            match client.send_recv(message.clone()).await {
                Ok(response) => {
                    conn.last_used = std::time::Instant::now();
                    Ok(response)
                }
                Err(e) => {
                    // Connection failed, mark as unhealthy
                    warn!("Connection {} failed: {}, marking unhealthy", index, e);
                    conn.healthy = false;
                    conn.client = None;

                    // Try to reconnect and retry once
                    match self.create_connection().await {
                        Ok(mut new_client) => {
                            match new_client.send_recv(message).await {
                                Ok(response) => {
                                    conn.client = Some(new_client);
                                    conn.healthy = true;
                                    conn.last_used = std::time::Instant::now();
                                    Ok(response)
                                }
                                Err(retry_err) => {
                                    error!("Retry also failed: {}", retry_err);
                                    Err(retry_err)
                                }
                            }
                        }
                        Err(reconnect_err) => {
                            error!("Reconnect failed: {}", reconnect_err);
                            Err(reconnect_err)
                        }
                    }
                }
            }
        } else {
            Err(ZapError::ipc("No connection available"))
        }
    }

    /// Perform health check on all connections
    pub async fn health_check(&self) -> (usize, usize) {
        let mut healthy = 0;
        let mut total = 0;

        for conn_mutex in &self.connections {
            total += 1;
            let conn = conn_mutex.lock().await;
            if conn.is_valid() {
                healthy += 1;
            }
        }

        (healthy, total)
    }

    /// Close all connections in the pool
    pub async fn close(&self) {
        debug!("Closing connection pool");

        for conn_mutex in &self.connections {
            let mut conn = conn_mutex.lock().await;
            conn.client = None;
            conn.healthy = false;
        }

        self.initialized.store(false, Ordering::Release);
    }

    /// Get pool configuration
    pub fn config(&self) -> &PoolConfig {
        &self.config
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        PoolStats {
            size: self.config.size,
            initialized: self.initialized.load(Ordering::Acquire),
        }
    }
}

/// Pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub size: usize,
    pub initialized: bool,
}

/// Global connection pool singleton
static GLOBAL_POOL: std::sync::OnceLock<Arc<ConnectionPool>> = std::sync::OnceLock::new();

/// Initialize the global connection pool
pub fn init_global_pool(socket_path: String) -> ZapResult<Arc<ConnectionPool>> {
    let pool = Arc::new(ConnectionPool::with_socket(socket_path));

    match GLOBAL_POOL.set(pool.clone()) {
        Ok(()) => Ok(pool),
        Err(_) => {
            // Pool already initialized, return existing
            Ok(GLOBAL_POOL.get().unwrap().clone())
        }
    }
}

/// Initialize the global connection pool with custom config
pub fn init_global_pool_with_config(config: PoolConfig) -> ZapResult<Arc<ConnectionPool>> {
    let pool = Arc::new(ConnectionPool::new(config));

    match GLOBAL_POOL.set(pool.clone()) {
        Ok(()) => Ok(pool),
        Err(_) => {
            // Pool already initialized, return existing
            Ok(GLOBAL_POOL.get().unwrap().clone())
        }
    }
}

/// Get the global connection pool (must be initialized first)
pub fn get_global_pool() -> Option<Arc<ConnectionPool>> {
    GLOBAL_POOL.get().cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_config_builder() {
        let config = PoolConfig::new("/tmp/test.sock".to_string())
            .size(8)
            .connect_timeout(Duration::from_secs(10))
            .encoding(IpcEncoding::Json);

        assert_eq!(config.size, 8);
        assert_eq!(config.connect_timeout, Duration::from_secs(10));
        assert_eq!(config.encoding, IpcEncoding::Json);
        assert_eq!(config.socket_path, "/tmp/test.sock");
    }

    #[test]
    fn test_pool_creation() {
        let pool = ConnectionPool::with_socket("/tmp/test.sock".to_string());

        assert_eq!(pool.config().size, DEFAULT_POOL_SIZE);
        assert_eq!(pool.connections.len(), DEFAULT_POOL_SIZE);
        assert!(!pool.initialized.load(Ordering::Acquire));
    }

    #[test]
    fn test_pool_stats() {
        let pool = ConnectionPool::with_socket("/tmp/test.sock".to_string());
        let stats = pool.stats();

        assert_eq!(stats.size, DEFAULT_POOL_SIZE);
        assert!(!stats.initialized);
    }

    #[tokio::test]
    async fn test_round_robin_index() {
        let pool = ConnectionPool::new(PoolConfig::new("/tmp/test.sock".to_string()).size(4));

        // Test round-robin distribution
        for expected in 0..12 {
            let index = pool.get_connection_index().await.unwrap();
            assert_eq!(index, expected % 4);
        }
    }
}
