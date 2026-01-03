use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

pub struct Metrics {
    start_time: Instant,
    total_requests: AtomicU64,
    successful_requests: AtomicU64,
    failed_requests: AtomicU64,
    timeout_requests: AtomicU64,
    cancelled_requests: AtomicU64,
    active_requests: AtomicU64,
}

impl Metrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            start_time: Instant::now(),
            total_requests: AtomicU64::new(0),
            successful_requests: AtomicU64::new(0),
            failed_requests: AtomicU64::new(0),
            timeout_requests: AtomicU64::new(0),
            cancelled_requests: AtomicU64::new(0),
            active_requests: AtomicU64::new(0),
        })
    }

    pub fn request_started(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.active_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn request_completed(&self) {
        self.successful_requests.fetch_add(1, Ordering::Relaxed);
        self.active_requests.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn request_failed(&self) {
        self.failed_requests.fetch_add(1, Ordering::Relaxed);
        self.active_requests.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn request_timeout(&self) {
        self.timeout_requests.fetch_add(1, Ordering::Relaxed);
        self.active_requests.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn request_cancelled(&self) {
        self.cancelled_requests.fetch_add(1, Ordering::Relaxed);
        self.active_requests.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn uptime_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }

    pub fn total_requests(&self) -> u64 {
        self.total_requests.load(Ordering::Relaxed)
    }

    pub fn active_requests(&self) -> u32 {
        self.active_requests.load(Ordering::Relaxed) as u32
    }

    pub fn successful_requests(&self) -> u64 {
        self.successful_requests.load(Ordering::Relaxed)
    }

    pub fn failed_requests(&self) -> u64 {
        self.failed_requests.load(Ordering::Relaxed)
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            start_time: Instant::now(),
            total_requests: AtomicU64::new(0),
            successful_requests: AtomicU64::new(0),
            failed_requests: AtomicU64::new(0),
            timeout_requests: AtomicU64::new(0),
            cancelled_requests: AtomicU64::new(0),
            active_requests: AtomicU64::new(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics() {
        let metrics = Metrics::new();

        metrics.request_started();
        assert_eq!(metrics.active_requests(), 1);
        assert_eq!(metrics.total_requests(), 1);

        metrics.request_completed();
        assert_eq!(metrics.active_requests(), 0);
        assert_eq!(metrics.successful_requests(), 1);
    }
}
