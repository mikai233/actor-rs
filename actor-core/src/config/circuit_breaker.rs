use serde::{Deserialize, Serialize};

use super::duration::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, derive_more::Constructor)]
pub struct CircuitBreaker {
    pub max_failures: usize,
    pub call_timeout: Duration,
    pub reset_timeout: Duration,
    pub exponential_backoff_factor: f64,
    pub random_factor: f64,
    pub exception_allowlist: Vec<String>,
}
