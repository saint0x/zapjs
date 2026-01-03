use crate::supervisor::{Supervisor, WorkerInfo};
use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;
use tokio::fs;
use tracing::{info, warn};

#[derive(Debug, Error)]
pub enum ReloadError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Worker spawn failed: {0}")]
    SpawnFailed(String),

    #[error("Incompatible exports")]
    IncompatibleExports,
}

pub struct ReloadManager {
    binary_path: PathBuf,
    current_hash: Option<Vec<u8>>,
}

impl ReloadManager {
    pub fn new(binary_path: PathBuf) -> Self {
        Self {
            binary_path,
            current_hash: None,
        }
    }

    pub async fn check_for_changes(&mut self) -> Result<bool, ReloadError> {
        let new_hash = self.hash_binary().await?;

        if let Some(ref current) = self.current_hash {
            if &new_hash != current {
                info!("Binary changed, hot reload triggered");
                self.current_hash = Some(new_hash);
                return Ok(true);
            }
        } else {
            self.current_hash = Some(new_hash);
        }

        Ok(false)
    }

    async fn hash_binary(&self) -> Result<Vec<u8>, ReloadError> {
        let data = fs::read(&self.binary_path).await?;
        Ok(sha2::Sha256::digest(&data).to_vec())
    }

    pub async fn perform_reload(
        &self,
        old_supervisor: &mut Supervisor,
        drain_timeout: Duration,
    ) -> Result<(), ReloadError> {
        info!("Starting hot reload sequence");

        // Drain in-flight requests
        // Note: This needs router integration which we'll handle in Phase 4
        info!("Draining in-flight requests (max {:?})", drain_timeout);
        tokio::time::sleep(Duration::from_millis(100)).await; // Placeholder

        // Graceful shutdown of old worker
        if let Err(e) = old_supervisor.graceful_shutdown(Duration::from_secs(5)).await {
            warn!("Error during graceful shutdown: {}", e);
        }

        // Supervisor will be restarted by the main loop
        info!("Hot reload complete");

        Ok(())
    }
}

use sha2::Digest;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_reload_manager_creation() {
        let manager = ReloadManager::new(PathBuf::from("/tmp/test"));
        assert!(manager.current_hash.is_none());
    }
}
