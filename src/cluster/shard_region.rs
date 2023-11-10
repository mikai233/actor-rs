use crate::actor::context::ActorContext;
use crate::actor::Actor;
use crate::cell::envelope::UserEnvelope;

#[derive(Debug)]
pub struct ShardRegion {}

impl Actor for ShardRegion {
    type S = ();
    type A = ();

    fn pre_start(&self, ctx: &mut ActorContext<Self>, arg: Self::A) -> anyhow::Result<Self::S> {
        todo!()
    }
}
