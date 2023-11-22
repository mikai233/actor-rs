use crate::Actor;
use crate::context::ActorContext;

#[derive(Debug)]
pub struct ShardRegion;

impl Actor for ShardRegion {
    type S = ();
    type A = ();

    fn pre_start(&self, _context: &mut ActorContext, _arg: Self::A) -> anyhow::Result<Self::S> {
        todo!()
    }
}
