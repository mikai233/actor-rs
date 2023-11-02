use crate::actor::context::ActorContext;
use crate::actor::Actor;
use crate::cell::envelope::UserEnvelope;

#[derive(Debug)]
pub struct ShardRegion {}

impl Actor for ShardRegion {
    type M = ();
    type S = ();
    type A = ();

    fn pre_start(&self, ctx: &mut ActorContext<Self>, arg: Self::A) -> anyhow::Result<Self::S> {
        todo!()
    }

    fn on_recv(
        &self,
        ctx: &mut ActorContext<Self>,
        state: &mut Self::S,
        message: UserEnvelope<Self::M>,
    ) -> anyhow::Result<()> {
        todo!()
    }
}
