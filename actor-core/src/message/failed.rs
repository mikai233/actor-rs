use crate::actor_ref::ActorRef;
use actor_derive::Message;

#[derive(Debug, Message, derive_more::Display)]
#[display("Failed {{ child: {}, error: {:?} }}", child, error)]
pub(crate) struct Failed {
    pub(crate) child: ActorRef,
    pub(crate) error: anyhow::Error,
}