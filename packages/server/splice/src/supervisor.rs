use crate::protocol::{Message, Role, PROTOCOL_VERSION, CAP_STREAMING, CAP_CANCELLATION};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

#[derive(Debug, Error)]
pub enum SupervisorError {
    #[error("Failed to spawn worker: {0}")]
    SpawnFailed(#[from] std::io::Error),

    #[error("Worker crashed: exit code {0:?}")]
    WorkerCrashed(Option<i32>),

    #[error("Worker failed to connect within timeout")]
    ConnectTimeout,

    #[error("Max restart attempts exceeded")]
    MaxRestartsExceeded,

    #[error("Circuit breaker open")]
    CircuitBreakerOpen,
}

#[derive(Debug, Clone)]
pub struct SupervisorConfig {
    pub max_restarts: usize,
    pub restart_backoff: Vec<Duration>,
    pub health_check_interval: Duration,
    pub drain_timeout: Duration,
    pub connect_timeout: Duration,
}

impl Default for SupervisorConfig {
    fn default() -> Self {
        Self {
            max_restarts: 10,
            restart_backoff: vec![
                Duration::from_millis(0),
                Duration::from_millis(100),
                Duration::from_millis(500),
                Duration::from_secs(2),
                Duration::from_secs(5),
            ],
            health_check_interval: Duration::from_secs(5),
            drain_timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(10),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerState {
    Starting,
    Ready,
    Draining,
    Failed,
    CircuitBreaker,
}

#[derive(Debug, Clone)]
pub struct WorkerInfo {
    pub pid: u32,
    pub state: WorkerState,
    pub started_at: Instant,
    pub restart_count: usize,
    pub active_requests: u32,
    pub total_requests: u64,
}

pub struct Supervisor {
    config: SupervisorConfig,
    worker_path: PathBuf,
    socket_path: PathBuf,
    worker: Option<Child>,
    worker_info: Option<WorkerInfo>,
    circuit_breaker_until: Option<Instant>,
}

impl Supervisor {
    pub fn new(
        config: SupervisorConfig,
        worker_path: PathBuf,
        socket_path: PathBuf,
    ) -> Self {
        Self {
            config,
            worker_path,
            socket_path,
            worker: None,
            worker_info: None,
            circuit_breaker_until: None,
        }
    }

    pub async fn start(&mut self) -> Result<WorkerInfo, SupervisorError> {
        self.spawn_worker(0).await
    }

    async fn spawn_worker(&mut self, restart_count: usize) -> Result<WorkerInfo, SupervisorError> {
        // Check circuit breaker
        if let Some(until) = self.circuit_breaker_until {
            if Instant::now() < until {
                return Err(SupervisorError::CircuitBreakerOpen);
            } else {
                // Reset circuit breaker
                self.circuit_breaker_until = None;
                info!("Circuit breaker reset");
            }
        }

        // Check max restarts
        if restart_count >= self.config.max_restarts {
            error!("Max restart attempts exceeded");
            self.circuit_breaker_until = Some(Instant::now() + Duration::from_secs(30));
            return Err(SupervisorError::MaxRestartsExceeded);
        }

        // Apply backoff if restarting
        if restart_count > 0 {
            let backoff_idx = (restart_count - 1).min(self.config.restart_backoff.len() - 1);
            let backoff = self.config.restart_backoff[backoff_idx];
            if !backoff.is_zero() {
                info!("Restart backoff: {:?}", backoff);
                tokio::time::sleep(backoff).await;
            }
        }

        // Spawn worker process
        info!(
            "Spawning worker: {} (attempt {}/{})",
            self.worker_path.display(),
            restart_count + 1,
            self.config.max_restarts
        );

        let mut cmd = Command::new(&self.worker_path);
        cmd.env("ZAP_SOCKET", &self.socket_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let mut child = cmd.spawn()?;
        let pid = child.id().unwrap_or(0);

        info!("Worker spawned with PID {}", pid);

        let worker_info = WorkerInfo {
            pid,
            state: WorkerState::Starting,
            started_at: Instant::now(),
            restart_count,
            active_requests: 0,
            total_requests: 0,
        };

        self.worker = Some(child);
        self.worker_info = Some(worker_info.clone());

        Ok(worker_info)
    }

    pub async fn restart(&mut self) -> Result<WorkerInfo, SupervisorError> {
        // Shutdown current worker if exists
        if let Some(ref mut child) = self.worker {
            info!("Stopping current worker");
            let _ = child.kill().await;
        }

        let restart_count = self
            .worker_info
            .as_ref()
            .map(|w| w.restart_count + 1)
            .unwrap_or(0);

        self.spawn_worker(restart_count).await
    }

    pub async fn graceful_shutdown(&mut self, timeout: Duration) -> Result<(), SupervisorError> {
        if let Some(ref mut child) = self.worker {
            info!("Initiating graceful shutdown");

            // Send SIGTERM
            #[cfg(unix)]
            {
                use nix::sys::signal::{kill, Signal};
                use nix::unistd::Pid;
                if let Some(pid) = child.id() {
                    let _ = kill(Pid::from_raw(pid as i32), Signal::SIGTERM);
                }
            }

            // Wait for graceful shutdown
            let shutdown = tokio::time::timeout(timeout, child.wait());

            match shutdown.await {
                Ok(Ok(status)) => {
                    info!("Worker exited gracefully: {:?}", status);
                }
                Ok(Err(e)) => {
                    error!("Error waiting for worker: {}", e);
                }
                Err(_) => {
                    warn!("Worker did not exit within timeout, sending SIGKILL");
                    let _ = child.kill().await;
                }
            }
        }

        self.worker = None;
        self.worker_info = None;

        Ok(())
    }

    pub fn worker_info(&self) -> Option<&WorkerInfo> {
        self.worker_info.as_ref()
    }

    pub fn update_state(&mut self, state: WorkerState) {
        if let Some(ref mut info) = self.worker_info {
            info.state = state;
        }
    }

    pub fn is_ready(&self) -> bool {
        self.worker_info
            .as_ref()
            .map(|w| w.state == WorkerState::Ready)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supervisor_config_default() {
        let config = SupervisorConfig::default();
        assert_eq!(config.max_restarts, 10);
        assert_eq!(config.restart_backoff.len(), 5);
    }
}
