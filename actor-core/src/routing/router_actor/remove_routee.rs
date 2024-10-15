use crate::actor::context::ActorContext;
use crate::routing::routee::Routee;
use actor_derive::Message;

#[derive(Debug, Message, derive_more::Display)]
#[display("RemoveRoutee {{ routee: {routee} }}")]
pub struct RemoveRoutee {
    pub routee: Routee,
}