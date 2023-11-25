use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use actor::{Actor, Message};
use actor::context::ActorContext;
use actor_derive::{EmptyCodec, MessageCodec};

#[derive(EmptyCodec)]
struct LocalMessage;

impl Message for LocalMessage {
    type T = ActorA;

    fn handle(self: Box<Self>, _context: &mut ActorContext, _state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        println!("handle LocalMessage");
        Ok(())
    }
}

#[derive(Serialize, Deserialize, MessageCodec)]
#[actor(ActorA)]
struct RemoteMessage;

impl Message for RemoteMessage {
    type T = ActorA;

    fn handle(self: Box<Self>, _context: &mut ActorContext, _state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        println!("handle RemoteMessage");
        Ok(())
    }
}

struct ActorA;

#[async_trait]
impl Actor for ActorA {
    type S = ();
    type A = ();

    async fn pre_start(&self, _context: &mut ActorContext, _arg: Self::A) -> anyhow::Result<Self::S> {
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    Ok(())
}
