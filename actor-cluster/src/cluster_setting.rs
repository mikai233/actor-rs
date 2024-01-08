use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::net::SocketAddrV4;

use etcd_client::Client;
use typed_builder::TypedBuilder;

use actor_core::actor::actor_system::ActorSystem;
use actor_core::message::message_registration::MessageRegistration;

#[derive(Clone, TypedBuilder)]
pub struct ClusterSetting {
    pub system: ActorSystem,
    pub addr: SocketAddrV4,
    pub reg: MessageRegistration,
    pub eclient: Client,
    pub roles: HashSet<String>,
}

impl Debug for ClusterSetting {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("ClusterSetting")
            .field("system", &self.system)
            .field("addr", &self.addr)
            .field("reg", &self.reg)
            .field("eclient", &"..")
            .finish()
    }
}