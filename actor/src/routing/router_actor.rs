use async_trait::async_trait;

use crate::Actor;
use crate::context::ActorContext;

pub(crate) struct RouterActor;

#[async_trait]
impl Actor for RouterActor {
    type S = ();
    type A = ();

    async fn pre_start(context: &mut ActorContext, arg: Self::A) -> anyhow::Result<Self::S> {
        todo!()
    }
}