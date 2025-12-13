//! Graceful Shutdown Implementation
//!
//! Provides signal handling and graceful shutdown capabilities for the Zap server.
//!
//! ## Features
//! - SIGTERM and SIGINT signal handling
//! - Configurable drain period for in-flight requests
//! - Connection tracking
//! - Proper resource cleanup
//!
//! ## Usage
//! ```rust
//! use zap_server::shutdown::{GracefulShutdown, ShutdownConfig};
//!
//! let shutdown = GracefulShutdown::new(ShutdownConfig::default());
//!
//! // In server loop
//! tokio::select! {
//!     _ = shutdown.wait() => {
//!         println!("Shutdown signal received");
//!         break;
//!     }
//!     result = listener.accept() => {
//!         // Handle connection
//!     }
//! }
//!
//! // Drain in-flight requests
//! shutdown.drain_connections().await;
//! ```

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::time::sleep;
use tracing::{info, warn};

/// Configuration for graceful shutdown
#[derive(Debug, Clone)]
pub struct ShutdownConfig {
    /// Maximum time to wait for in-flight requests to complete (default: 30s)
    pub drain_timeout: Duration,
    /// Whether to enable signal handling (default: true)
    pub enable_signal_handlers: bool,
    /// Poll interval for checking connection count during drain (default: 100ms)
    pub drain_poll_interval: Duration,
}

impl Default for ShutdownConfig {
    fn default() -> Self {
        Self {
            drain_timeout: Duration::from_secs(30),
            enable_signal_handlers: true,
            drain_poll_interval: Duration::from_millis(100),
        }
    }
}

impl ShutdownConfig {
    /// Create config for development (shorter timeout)
    pub fn development() -> Self {
        Self {
            drain_timeout: Duration::from_secs(5),
            ..Default::default()
        }
    }

    /// Create config for production (longer timeout)
    pub fn production() -> Self {
        Self::default()
    }

    /// Create config with custom drain timeout
    pub fn with_drain_timeout(mut self, timeout: Duration) -> Self {
        self.drain_timeout = timeout;
        self
    }

    /// Disable signal handlers (for testing or custom signal handling)
    pub fn without_signal_handlers(mut self) -> Self {
        self.enable_signal_handlers = false;
        self
    }
}

/// Graceful shutdown coordinator
///
/// Handles signal reception, connection tracking, and coordinated shutdown.
pub struct GracefulShutdown {
    /// Configuration
    config: ShutdownConfig,
    /// Shutdown signal notifier
    shutdown_notifier: Arc<Notify>,
    /// Whether shutdown has been triggered
    shutdown_triggered: Arc<AtomicBool>,
    /// Count of active connections
    active_connections: Arc<AtomicU64>,
    /// Whether we're currently draining
    draining: Arc<AtomicBool>,
}

impl GracefulShutdown {
    /// Create a new graceful shutdown coordinator
    pub fn new(config: ShutdownConfig) -> Self {
        let shutdown = Self {
            config: config.clone(),
            shutdown_notifier: Arc::new(Notify::new()),
            shutdown_triggered: Arc::new(AtomicBool::new(false)),
            active_connections: Arc::new(AtomicU64::new(0)),
            draining: Arc::new(AtomicBool::new(false)),
        };

        if config.enable_signal_handlers {
            shutdown.setup_signal_handlers();
        }

        shutdown
    }

    /// Set up signal handlers for SIGTERM and SIGINT
    fn setup_signal_handlers(&self) {
        let shutdown_notifier = self.shutdown_notifier.clone();
        let shutdown_triggered = self.shutdown_triggered.clone();

        tokio::spawn(async move {
            #[cfg(unix)]
            {
                use tokio::signal::unix::{signal, SignalKind};

                let mut sigterm = signal(SignalKind::terminate())
                    .expect("Failed to register SIGTERM handler");
                let mut sigint = signal(SignalKind::interrupt())
                    .expect("Failed to register SIGINT handler");

                tokio::select! {
                    _ = sigterm.recv() => {
                        info!("📡 Received SIGTERM, initiating graceful shutdown");
                    }
                    _ = sigint.recv() => {
                        info!("📡 Received SIGINT (Ctrl+C), initiating graceful shutdown");
                    }
                }
            }

            #[cfg(windows)]
            {
                use tokio::signal::ctrl_c;

                ctrl_c().await.expect("Failed to listen for Ctrl+C");
                info!("📡 Received Ctrl+C, initiating graceful shutdown");
            }

            shutdown_triggered.store(true, Ordering::SeqCst);
            shutdown_notifier.notify_waiters();
        });
    }

    /// Wait for shutdown signal
    ///
    /// This should be used in a tokio::select! block in the main server loop.
    pub async fn wait(&self) {
        self.shutdown_notifier.notified().await;
    }

    /// Check if shutdown has been triggered
    pub fn is_shutdown(&self) -> bool {
        self.shutdown_triggered.load(Ordering::SeqCst)
    }

    /// Trigger shutdown programmatically (for testing or custom shutdown logic)
    pub fn trigger(&self) {
        info!("🛑 Shutdown triggered programmatically");
        self.shutdown_triggered.store(true, Ordering::SeqCst);
        self.shutdown_notifier.notify_waiters();
    }

    /// Increment active connection count
    pub fn connection_started(&self) {
        self.active_connections.fetch_add(1, Ordering::SeqCst);
    }

    /// Decrement active connection count
    pub fn connection_finished(&self) {
        let prev = self.active_connections.fetch_sub(1, Ordering::SeqCst);

        // Log warning if count goes negative (shouldn't happen)
        if prev == 0 {
            warn!("⚠️  Connection finished but counter was already 0");
        }
    }

    /// Get current active connection count
    pub fn active_connection_count(&self) -> u64 {
        self.active_connections.load(Ordering::SeqCst)
    }

    /// Create a connection guard that automatically tracks connection lifetime
    ///
    /// The connection is tracked from creation until the guard is dropped.
    pub fn connection_guard(&self) -> ConnectionGuard {
        self.connection_started();
        ConnectionGuard {
            shutdown: self.clone(),
        }
    }

    /// Drain active connections with timeout
    ///
    /// Waits for all in-flight connections to complete, up to the configured timeout.
    /// Returns true if all connections drained successfully, false if timeout occurred.
    pub async fn drain_connections(&self) -> bool {
        self.draining.store(true, Ordering::SeqCst);

        let active = self.active_connection_count();
        if active == 0 {
            info!("✅ No active connections to drain");
            return true;
        }

        info!("⏳ Draining {} active connection(s), timeout: {:?}",
              active, self.config.drain_timeout);

        let start = std::time::Instant::now();
        let mut last_count = active;

        loop {
            let current_count = self.active_connection_count();

            if current_count == 0 {
                info!("✅ All connections drained successfully");
                return true;
            }

            // Log progress if count changed
            if current_count != last_count {
                info!("⏳ {} connection(s) remaining...", current_count);
                last_count = current_count;
            }

            // Check timeout
            if start.elapsed() >= self.config.drain_timeout {
                warn!("⚠️  Drain timeout reached with {} connection(s) still active", current_count);
                return false;
            }

            sleep(self.config.drain_poll_interval).await;
        }
    }

    /// Check if currently draining
    pub fn is_draining(&self) -> bool {
        self.draining.load(Ordering::SeqCst)
    }

    /// Get shutdown configuration
    pub fn config(&self) -> &ShutdownConfig {
        &self.config
    }
}

impl Clone for GracefulShutdown {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            shutdown_notifier: self.shutdown_notifier.clone(),
            shutdown_triggered: self.shutdown_triggered.clone(),
            active_connections: self.active_connections.clone(),
            draining: self.draining.clone(),
        }
    }
}

/// RAII guard for tracking connection lifetime
///
/// Automatically increments connection count on creation and decrements on drop.
pub struct ConnectionGuard {
    shutdown: GracefulShutdown,
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.shutdown.connection_finished();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_shutdown_trigger() {
        let config = ShutdownConfig::default().without_signal_handlers();
        let shutdown = GracefulShutdown::new(config);

        assert!(!shutdown.is_shutdown());

        shutdown.trigger();

        assert!(shutdown.is_shutdown());
    }

    #[tokio::test]
    async fn test_connection_tracking() {
        let config = ShutdownConfig::default().without_signal_handlers();
        let shutdown = GracefulShutdown::new(config);

        assert_eq!(shutdown.active_connection_count(), 0);

        shutdown.connection_started();
        assert_eq!(shutdown.active_connection_count(), 1);

        shutdown.connection_started();
        assert_eq!(shutdown.active_connection_count(), 2);

        shutdown.connection_finished();
        assert_eq!(shutdown.active_connection_count(), 1);

        shutdown.connection_finished();
        assert_eq!(shutdown.active_connection_count(), 0);
    }

    #[tokio::test]
    async fn test_connection_guard() {
        let config = ShutdownConfig::default().without_signal_handlers();
        let shutdown = GracefulShutdown::new(config);

        assert_eq!(shutdown.active_connection_count(), 0);

        {
            let _guard = shutdown.connection_guard();
            assert_eq!(shutdown.active_connection_count(), 1);
        }

        // Guard dropped, count should be 0
        assert_eq!(shutdown.active_connection_count(), 0);
    }

    #[tokio::test]
    async fn test_drain_no_connections() {
        let config = ShutdownConfig::default().without_signal_handlers();
        let shutdown = GracefulShutdown::new(config);

        let success = shutdown.drain_connections().await;
        assert!(success);
    }

    #[tokio::test]
    async fn test_drain_with_connections() {
        let config = ShutdownConfig::default()
            .without_signal_handlers()
            .with_drain_timeout(Duration::from_secs(2));
        let shutdown = GracefulShutdown::new(config);

        shutdown.connection_started();
        shutdown.connection_started();

        let shutdown_clone = shutdown.clone();
        tokio::spawn(async move {
            sleep(Duration::from_millis(500)).await;
            shutdown_clone.connection_finished();
            sleep(Duration::from_millis(500)).await;
            shutdown_clone.connection_finished();
        });

        let success = shutdown.drain_connections().await;
        assert!(success);
        assert_eq!(shutdown.active_connection_count(), 0);
    }

    #[tokio::test]
    async fn test_drain_timeout() {
        let config = ShutdownConfig::default()
            .without_signal_handlers()
            .with_drain_timeout(Duration::from_millis(100));
        let shutdown = GracefulShutdown::new(config);

        // Add connections that won't finish
        shutdown.connection_started();
        shutdown.connection_started();

        let success = shutdown.drain_connections().await;
        assert!(!success); // Should timeout
        assert_eq!(shutdown.active_connection_count(), 2);
    }

    #[tokio::test]
    async fn test_wait_for_shutdown() {
        let config = ShutdownConfig::default().without_signal_handlers();
        let shutdown = GracefulShutdown::new(config);

        let shutdown_clone = shutdown.clone();
        tokio::spawn(async move {
            sleep(Duration::from_millis(100)).await;
            shutdown_clone.trigger();
        });

        shutdown.wait().await;
        assert!(shutdown.is_shutdown());
    }

    #[tokio::test]
    async fn test_shutdown_select() {
        let config = ShutdownConfig::default().without_signal_handlers();
        let shutdown = GracefulShutdown::new(config);

        let shutdown_clone = shutdown.clone();
        tokio::spawn(async move {
            sleep(Duration::from_millis(50)).await;
            shutdown_clone.trigger();
        });

        let mut task_ran = false;

        tokio::select! {
            _ = shutdown.wait() => {
                // Shutdown signal received
            }
            _ = sleep(Duration::from_secs(10)) => {
                task_ran = true;
            }
        }

        assert!(!task_ran);
        assert!(shutdown.is_shutdown());
    }

    #[test]
    fn test_config_builder() {
        let config = ShutdownConfig::development()
            .with_drain_timeout(Duration::from_secs(10))
            .without_signal_handlers();

        assert_eq!(config.drain_timeout, Duration::from_secs(10));
        assert!(!config.enable_signal_handlers);
    }
}
