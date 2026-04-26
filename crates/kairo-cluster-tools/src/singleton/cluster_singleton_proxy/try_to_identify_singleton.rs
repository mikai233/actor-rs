use async_trait::async_trait;
use tracing::debug;

use kairo_core::EmptyCodec;
use kairo_core::actor::actor_selection::ActorSelectionPath;
use kairo_core::actor::context::ActorContext;
use kairo_core::actor_path::TActorPath;
use kairo_core::actor_path::root_actor_path::RootActorPath;
use kairo_core::actor_ref::actor_ref_factory::ActorRefFactory;
use kairo_core::message::identify::Identify;
use kairo_core::{DynMessage, Message};

use crate::singleton::cluster_singleton_proxy::ClusterSingletonProxy;

#[derive(Debug, EmptyCodec)]
pub(super) struct TryToIdentifySingleton;

#[async_trait]
impl Message for TryToIdentifySingleton {
    type A = ClusterSingletonProxy;

    async fn handle(
        self: Box<Self>,
        context: &mut ActorContext,
        actor: &mut Self::A,
    ) -> anyhow::Result<()> {
        if actor.identify_timer.is_some() {
            let paths = actor.singleton_paths();
            let paths_str = paths.iter().map(|p| p.as_str()).collect::<Vec<_>>();
            for member in actor.host_singleton_members.values() {
                let singleton_path =
                    RootActorPath::new(member.address().clone(), "/").descendant(paths_str.clone());
                let selection =
                    context.actor_selection(ActorSelectionPath::FullPath(singleton_path))?;
                debug!("try to identify singleton at [{}]", selection);
                selection.tell(
                    DynMessage::system(Identify),
                    Some(actor.identify_adapter.clone()),
                );
            }
        }
        Ok(())
    }
}
