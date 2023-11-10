use std::any::Any;
use std::fmt::{Debug, Formatter};
use anyhow::anyhow;

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use url::quirks::search;

use crate::actor::context::ActorContext;

pub mod context;

pub trait Actor: Send + Sync + Sized + 'static {
    type S: State;
    type A: Arg;
    fn pre_start(&self, ctx: &mut ActorContext<Self>, arg: Self::A) -> anyhow::Result<Self::S>;

    #[allow(unused_variables)]
    fn post_stop(&self, ctx: &mut ActorContext<Self>, state: &mut Self::S) -> anyhow::Result<()> {
        Ok(())
    }
}

#[async_trait(? Send)]
pub trait Message: Any + Send + 'static {
    type T: Actor;
    async fn handle(self: Box<Self>, context: &mut ActorContext<'_, Self::T>, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()>;
}

pub trait SerializableMessage: Message + Serialize + DeserializeOwned {
    fn decoder() -> Box<dyn MessageDecoder>;
}

#[derive(Debug)]
pub struct DynamicMessage {
    pub(crate) name: &'static str,
    pub(crate) inner: Box<dyn Any + Send + 'static>,
}

impl DynamicMessage {
    pub(crate) fn new<M>(message: M) -> Self where M: Message {
        let name = std::any::type_name::<M>();
        Self {
            name,
            inner: Box::new(message),
        }
    }

    pub(crate) fn name(&self) -> &'static str {
        self.name
    }
    pub(crate) fn downcast<T>(self) -> anyhow::Result<Box<MessageDelegate<T>>> where T: Actor {
        match self.inner.downcast::<MessageDelegate<T>>() {
            Ok(m) => {
                Ok(m)
            }
            Err(_) => {
                let actor_name = std::any::type_name::<T>();
                Err(anyhow!("unexpected message {} to actor {}", self.name, actor_name))
            }
        }
    }
}

pub trait MessageDecoder {
    fn decode(&self, bytes: &[u8]) -> anyhow::Result<DynamicMessage>;
}

pub struct MessageDelegate<T> where T: Actor {
    pub(crate) message: Box<dyn Message<T=T>>,
}

impl<T> Debug for MessageDelegate<T> where T: Actor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MessageDelegate")
            .field("message", &"..")
            .finish()
    }
}

impl<T> MessageDelegate<T> where T: Actor {
    pub fn new<M>(message: M) -> Self where M: Message<T=T> {
        Self {
            message: Box::new(message)
        }
    }
}

#[async_trait(? Send)]
impl<T> Message for MessageDelegate<T> where T: Actor + Send + 'static {
    type T = T;

    async fn handle(self: Box<Self <>>, context: &mut ActorContext<'_, Self::T>, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        self.message.handle(context, state).await
    }
}

pub trait State: Any + 'static {}

impl<T> State for T where T: Any + 'static {}

pub trait Arg: Any + Send + 'static {}

impl<T> Arg for T where T: Any + Send + 'static {}

#[cfg(test)]
mod actor_test {
    use std::time::Duration;

    use anyhow::Ok;
    use tracing::info;

    use crate::actor::Actor;
    use crate::actor::context::ActorContext;
    use crate::props::Props;
    use crate::provider::ActorRefFactory;
    use crate::system::ActorSystem;

    #[tokio::test]
    async fn test_death_watch() -> anyhow::Result<()> {
        #[derive(Debug)]
        struct TestActor;

        impl Actor for TestActor {
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
