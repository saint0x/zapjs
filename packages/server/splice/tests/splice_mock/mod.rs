//! Mock implementations for Splice protocol testing
//!
//! This module provides mock Host and Worker implementations that exactly replicate
//! the behavior of real Splice components, allowing for comprehensive testing without
//! spawning actual processes or using Unix sockets.

pub mod mock_host;
pub mod mock_worker;
pub mod test_harness;

pub use mock_host::{HostState, MockHost, MockHostBuilder};
pub use mock_worker::{MockWorker, MockWorkerBuilder, WorkerState};
pub use test_harness::TestHarness;
