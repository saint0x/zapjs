use tokio::sync::mpsc;

// Re-export protocol types for use in tests
pub use splice::protocol::Message;

/// Test harness for setting up bidirectional communication channels
/// between mock Host and Worker implementations.
pub struct TestHarness {
    host_to_worker: (mpsc::Sender<Message>, mpsc::Receiver<Message>),
    worker_to_host: (mpsc::Sender<Message>, mpsc::Receiver<Message>),
}

impl TestHarness {
    /// Create a new test harness with bidirectional channels
    pub fn new() -> Self {
        let (host_tx, worker_rx) = mpsc::channel(100);
        let (worker_tx, host_rx) = mpsc::channel(100);

        Self {
            host_to_worker: (host_tx, worker_rx),
            worker_to_host: (worker_tx, host_rx),
        }
    }

    /// Split the harness into host and worker channel pairs
    /// Returns: ((host_tx, host_rx), (worker_tx, worker_rx))
    pub fn split(
        self,
    ) -> (
        (mpsc::Sender<Message>, mpsc::Receiver<Message>),
        (mpsc::Sender<Message>, mpsc::Receiver<Message>),
    ) {
        let (host_tx, worker_rx) = self.host_to_worker;
        let (worker_tx, host_rx) = self.worker_to_host;

        ((host_tx, host_rx), (worker_tx, worker_rx))
    }
}

impl Default for TestHarness {
    fn default() -> Self {
        Self::new()
    }
}
