use serde::{Deserialize, Serialize};

use actor::actor::{Actor, Message};
use actor::actor::context::ActorContext;
use actor_derive::MessageCodec;

#[derive(MessageCodec, Serialize, Deserialize)]
#[message(mtype = "serde", actor = "ActorA")]
struct MessageA;

impl Message for MessageA {
    type T = ActorA;

    fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        println!("handle MessageA");
        Ok(())
    }
}

struct ActorA;

impl Actor for ActorA {
    type S = ();
    type A = ();

    fn pre_start(&self, context: &mut ActorContext, arg: Self::A) -> anyhow::Result<Self::S> {
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    Ok(())
}
