use std::time::Duration;

use actor_core::actor::{actor_system::{ActorSystem, WeakActorSystem}, props::Props};

use crate::cluster::Cluster;

pub trait DowningProvider {
    fn down_removal_margin(&self) -> Duration;

    fn downing_actor_props(&self) -> Option<Props>;
}

#[derive(Debug, Clone)]
pub struct NoDowning {
    pub system: WeakActorSystem,
}

impl DowningProvider for NoDowning {
    fn down_removal_margin(&self) -> Duration {
        Cluster::new(self.system.upgrade().unwrap()).set
        Duration::from_secs(10)
    }

    fn downing_actor_props(&self) -> Option<Props> {
        None
    }
}
