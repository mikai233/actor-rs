use std::any::Any;
use std::fmt::Debug;

use crate::actor::context::ActorContext;
use crate::cell::envelope::UserEnvelope;

pub mod context;

pub trait Actor: Send + Sync + Sized + 'static {
    type M: Message;
    type S: State;
    type A: Arg;
    fn pre_start(&self, ctx: &mut ActorContext<Self>, arg: Self::A) -> anyhow::Result<Self::S>;

    fn on_recv(
        &self,
        ctx: &mut ActorContext<Self>,
        state: &mut Self::S,
        envelope: UserEnvelope<Self::M>,
    ) -> anyhow::Result<()>;

    #[allow(unused_variables)]
    fn post_stop(&self, ctx: &mut ActorContext<Self>, state: &mut Self::S) -> anyhow::Result<()> {
        Ok(())
    }
}

pub trait Message: Any + Send + Sized + 'static {
    fn downcast(
        message: Box<dyn Any + Send + 'static>,
    ) -> Result<Self, Box<dyn Any + Send + 'static>> {
        match message.downcast::<Self>() {
            Ok(message) => Ok(*message),
            Err(message) => Err(message),
        }
    }
}

impl<T> Message for T where T: Any + Send + Sized + 'static {}

pub trait State: Any + 'static {}

impl<T> State for T where T: Any + 'static {}

pub trait Arg: Any + Send + 'static {}

impl<T> Arg for T where T: Any + Send + 'static {}

#[cfg(test)]
mod actor_test {
    use std::time::Duration;
    use crate::actor::context::ActorContext;
    use crate::actor::Actor;
    use crate::cell::envelope::UserEnvelope;
    use crate::props::Props;
    use crate::provider::ActorRefFactory;
    use anyhow::Ok;
    use tracing::info;
    use crate::system::ActorSystem;

    #[tokio::test]
    async fn test_death_watch() -> anyhow::Result<()> {
        #[derive(Debug)]
        struct TestActor;

        impl Actor for TestActor {
            type M = ();
            type S = ();
            type A = usize;

            fn pre_start(
                &self,
                ctx: &mut ActorContext<Self>,
                arg: Self::A,
            ) -> anyhow::Result<Self::S> {
                info!("actor {} pre start", ctx.myself);
                for _ in 0..3 {
                    let n = arg - 1;
                    if n > 0 {
                        ctx.actor_of(TestActor, arg - 1, Props::default(), None)?;
                    }
                }
                Ok(())
            }

            fn on_recv(
                &self,
                ctx: &mut ActorContext<Self>,
                state: &mut Self::S,
                envelope: UserEnvelope<Self::M>,
            ) -> anyhow::Result<()> {
                Ok(())
            }

            fn post_stop(&self, ctx: &mut ActorContext<Self>, state: &mut Self::S) -> anyhow::Result<()> {
                info!("actor {} post stop",ctx.myself);
                Ok(())
            }
        }

        let system = ActorSystem::new("game".to_string(), "127.0.0.1:12121".parse()?)?;
        let actor = system.actor_of(TestActor, 3, Props::default(), None)?;
        tokio::time::sleep(Duration::from_secs(1)).await;
        system.stop(&actor);
        tokio::time::sleep(Duration::from_secs(3)).await;
        Ok(())
    }
}
