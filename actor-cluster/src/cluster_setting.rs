use std::fmt::Debug;

use typed_builder::TypedBuilder;

use actor_core::actor::actor_system::ActorSystem;
use actor_core::ext::etcd_client::EtcdClient;
use actor_core::message::message_registration::MessageRegistration;

use crate::config::ClusterConfig;

#[derive(Debug, Clone, TypedBuilder)]
pub struct ClusterSetting {
    pub system: ActorSystem,
    pub config: ClusterConfig,
    pub reg: MessageRegistration,
    #[builder(setter(into))]
    pub client: EtcdClient,
}