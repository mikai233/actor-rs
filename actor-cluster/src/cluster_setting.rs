use std::fmt::{Debug, Formatter};

use etcd_client::Client;
use typed_builder::TypedBuilder;

use actor_core::actor::actor_system::ActorSystem;
use actor_core::message::message_registration::MessageRegistration;

use crate::config::ClusterConfig;

#[derive(Clone, TypedBuilder)]
pub struct ClusterSetting {
    pub system: ActorSystem,
    pub config: ClusterConfig,
    pub reg: MessageRegistration,
    pub eclient: Client,
}

impl Debug for ClusterSetting {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("ClusterSetting")
            .field("system", &self.system)
            .field("config", &self.config)
            .field("reg", &self.reg)
            .field("eclient", &"..")
            .finish()
    }
}