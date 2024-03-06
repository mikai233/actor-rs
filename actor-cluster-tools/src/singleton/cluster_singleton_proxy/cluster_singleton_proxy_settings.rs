use std::time::Duration;

use typed_builder::TypedBuilder;

#[derive(Debug, Clone, TypedBuilder)]
pub struct ClusterSingletonProxySettings {
    #[builder(default = "singleton".to_string())]
    pub singleton_name: String,
    #[builder(default = None)]
    pub role: Option<String>,
    pub singleton_identification_interval: Duration,
    pub buffer_size: usize,
}