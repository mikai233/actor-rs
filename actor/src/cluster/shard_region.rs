use async_trait::async_trait;
use crate::Actor;
use crate::context::ActorContext;

#[derive(Debug)]
pub struct ShardRegion;

#[async_trait]
impl Actor for ShardRegion {
    type S = ();
    type A = ();

    async fn pre_start(&self, _context: &mut ActorContext, _arg: Self::A) -> anyhow::Result<Self::S> {
        todo!()
    }
}
