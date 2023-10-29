use crate::actor::Actor;
use crate::actor::context::ActorContext;

#[derive(Debug)]
struct Guardian;

impl Actor for Guardian {
    type M = ();
    type S = ();
    type A = ();

    fn pre_start(&self, ctx: &mut ActorContext<Self>, arg: Self::A) -> anyhow::Result<Self::S> {
        Ok(())
    }

    fn on_recv(&self, ctx: &mut ActorContext<Self>, state: &mut Self::S, message: Self::M) -> anyhow::Result<()> {
        Ok(())
    }
}