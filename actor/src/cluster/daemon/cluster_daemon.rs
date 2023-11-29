use async_trait::async_trait;

use crate::Actor;

#[derive(Debug)]
pub(crate) struct ClusterDaemon;

#[async_trait]
impl Actor for ClusterDaemon {}