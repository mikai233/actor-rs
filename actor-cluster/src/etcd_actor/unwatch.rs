use async_trait::async_trait;
use tracing::debug;

use actor_core::{DynMessage, Message};
use actor_core::{EmptyCodec, OrphanEmptyCodec};
use actor_core::actor::context::ActorContext;
use actor_core::actor_ref::ActorRef;
use actor_core::ext::option_ext::OptionExt;

use crate::etcd_actor::EtcdActor;

#[derive(Debug, EmptyCodec)]
pub struct Unwatch {
    pub id: i64,
    pub applicant: Option<ActorRef>,
}

#[async_trait]
impl Message for Unwatch {
    type A = EtcdActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        match self.applicant.as_ref() {
            None => {
                debug!("unwatch {}", self.id);
            }
            Some(applicant) => {
                debug!("{} request unwatch {}", applicant, self.id);
            }
        }
        match actor.watcher.remove(&self.id) {
            None => {
                self.applicant.foreach(|applicant| {
                    applicant.tell(
                        DynMessage::orphan(UnwatchResp::Failed(None)),
                        ActorRef::no_sender(),
                    );
                });
            }
            Some(mut watcher) => {
                context.spawn_fut(async move {
                    match watcher.watcher.cancel().await {
                        Ok(_) => {
                            self.applicant.foreach(|applicant| {
                                applicant.tell(
                                    DynMessage::orphan(UnwatchResp::Success),
                                    ActorRef::no_sender(),
                                );
                            });
                        }
                        Err(error) => {
                            self.applicant.foreach(|applicant| {
                                applicant.tell(
                                    DynMessage::orphan(UnwatchResp::Failed(Some(error))),
                                    ActorRef::no_sender(),
                                );
                            });
                        }
                    }
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, OrphanEmptyCodec)]
pub enum UnwatchResp {
    Success,
    Failed(Option<etcd_client::Error>),
}