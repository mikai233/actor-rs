use serde::{Deserialize, Serialize};

use crate::util::duration::ConfigDuration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreaker {
    #[serde(rename = "max-failures")]
    pub max_failures: usize,
    #[serde(rename = "call-timeout")]
    pub call_timeout: ConfigDuration,
    #[serde(rename = "reset-timeout")]
    pub reset_timeout: ConfigDuration,
    #[serde(rename = "exponential-backoff-factor")]
    pub exponential_backoff_factor: f64,
    #[serde(rename = "random-factor")]
    pub random_factor: f64,
    #[serde(rename = "exception-allowlist")]
    pub exception_allowlist: Vec<String>,
}