use std::time::Duration;

use actor_core::actor::props::Props;

use crate::cluster::Cluster;

pub trait DowningProvider {
    fn down_removal_margin(&self) -> Option<Duration>;

    fn downing_actor_props(&self) -> Option<Props>;
}

#[derive(Debug, Clone)]
pub struct NoDowning {
    pub cluster: Cluster,
}

impl DowningProvider for NoDowning {
    fn down_removal_margin(&self) -> Option<Duration> {
        self.cluster.settings.down_removal_margin
    }

    fn downing_actor_props(&self) -> Option<Props> {
        None
    }
}
