use async_trait::async_trait;
use bincode::{Decode, Encode};
use tracing::info;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::Message;
use actor_derive::MessageCodec;

use crate::common::singleton_actor::SingletonActor;

#[derive(Debug, Encode, Decode, MessageCodec)]
pub struct Greet(pub usize);

#[async_trait]
impl Message for Greet {
    type A = SingletonActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> eyre::Result<()> {
        println!("{:?}", *self);
        info!("{} recv {:?}", context.myself(), *self);
        Ok(())
    }
}