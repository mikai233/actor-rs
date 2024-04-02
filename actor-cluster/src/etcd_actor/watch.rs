use async_trait::async_trait;
use etcd_client::{WatchOptions, WatchResponse};
use tracing::debug;

use actor_core::{DynMessage, Message};
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_derive::{EmptyCodec, OrphanEmptyCodec};

use crate::etcd_actor::EtcdActor;
use crate::etcd_actor::watch_started::WatchStarted;

#[derive(Debug, EmptyCodec)]
pub struct Watch {
    pub key: String,
    pub options: Option<WatchOptions>,
    pub applicant: ActorRef,
}

#[async_trait]
impl Message for Watch {
    type A = EtcdActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        debug!("{} request watch key {}", self.applicant, self.key);
        let mut client = actor.client.clone();
        let myself = context.myself().clone();
        context.spawn_fut(async move {
            match client.watch(self.key, self.options).await {
                Ok((watcher, stream)) => {
                    self.applicant.tell(DynMessage::orphan(WatchResp::Started), ActorRef::no_sender());
                    let started = WatchStarted {
                        watcher,
                        stream,
                        applicant: self.applicant,
                    };
                    myself.cast_ns(started);
                }
                Err(error) => {
                    self.applicant.tell(
                        DynMessage::orphan(WatchResp::Failed(Some(error))),
                        ActorRef::no_sender(),
                    );
                }
            }
        });
        Ok(())
    }
}

#[derive(Debug, OrphanEmptyCodec)]
pub enum WatchResp {
    Update(WatchResponse),
    Started,
    Failed(Option<etcd_client::Error>),
}