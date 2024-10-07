use async_trait::async_trait;
use tracing::debug;

use actor_core::{DynMessage, Message};
use actor_core::actor::actor_selection::ActorSelectionPath;
use actor_core::actor::context::ActorContext1;
use actor_core::actor_path::root_actor_path::RootActorPath;
use actor_core::actor_path::TActorPath;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::EmptyCodec;
use actor_core::message::identify::Identify;

use crate::singleton::cluster_singleton_proxy::ClusterSingletonProxy;

#[derive(Debug, EmptyCodec)]
pub(super) struct TryToIdentifySingleton;

#[async_trait]
impl Message for TryToIdentifySingleton {
    type A = ClusterSingletonProxy;

    async fn handle(self: Box<Self>, context: &mut ActorContext1, actor: &mut Self::A) -> anyhow::Result<()> {
        if actor.identify_timer.is_some() {
            let paths = actor.singleton_paths();
            let paths_str = paths.iter().map(|p| p.as_str()).collect::<Vec<_>>();
            for (_, member) in &actor.host_singleton_members {
                let singleton_path = RootActorPath::new(member.address().clone(), "/").descendant(paths_str.clone());
                let selection = context.actor_selection(ActorSelectionPath::FullPath(singleton_path))?;
                debug!("try to identify singleton at [{}]", selection);
                selection.tell(DynMessage::system(Identify), Some(actor.identify_adapter.clone()));
            }
        }
        Ok(())
    }
}