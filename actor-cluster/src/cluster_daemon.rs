use std::collections::HashSet;
use std::fmt::{Debug, Formatter};

use async_trait::async_trait;
use etcd_client::{Client, EventType, WatchOptions};

use actor_core::Actor;
use actor_core::actor::context::ActorContext;

use crate::unique_address::UniqueAddress;

pub struct ClusterDaemon {
    pub(crate) eclient: Client,
    pub(crate) self_unique_address: UniqueAddress,
    pub(crate) roles: HashSet<String>,
}

impl Debug for ClusterDaemon {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("ClusterDaemon")
            .field("eclient", &"..")
            .field("self_unique_address", &self.self_unique_address)
            .field("roles", &self.roles)
            .finish()
    }
}

#[async_trait]
impl Actor for ClusterDaemon {
    async fn pre_start(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        // actor/mikai233/cluster/127.0.0.1:2551
        let (a, mut b) = self.eclient.watch("somekey", Some(WatchOptions::new().with_prefix())).await?;
        match b.message().await? {
            None => {}
            Some(d) => {
                for event in d.events() {
                    match event.event_type() {
                        EventType::Put => {}
                        EventType::Delete => {}
                    }
                }
            }
        }
        todo!()
    }
}