use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use actor::actor::{Actor, Message, SystemMessage};
use actor::actor::context::ActorContext;
use actor_derive::{MessageCodec, EmptyCodec, SystemMessageCodec};

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

#[derive(Serialize, Deserialize, SystemMessageCodec)]
struct SysMessage;

#[async_trait]
impl SystemMessage for SysMessage {
    async fn handle(self: Box<Self>, _context: &mut ActorContext) -> anyhow::Result<()> {
        println!("handle SysMessage");
        Ok(())
    }
}

struct ActorA;

impl Actor for ActorA {
    type S = ();
    type A = ();

    fn pre_start(&self, _context: &mut ActorContext, _arg: Self::A) -> anyhow::Result<Self::S> {
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    Ok(())
}
