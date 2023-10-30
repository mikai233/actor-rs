use std::any::Any;
use std::fmt::Debug;

use crate::actor::context::ActorContext;

pub mod context;

pub trait Actor: Send + Sync + Sized + 'static {
    type M: Message;
    type S: State;
    type A: Arg;
    fn pre_start(&self, ctx: &mut ActorContext<Self>, arg: Self::A) -> anyhow::Result<Self::S>;

    fn on_recv(&self, ctx: &mut ActorContext<Self>, state: &mut Self::S, message: Self::M) -> anyhow::Result<()>;

    #[allow(unused_variables)]
    fn post_stop(&self, ctx: &mut ActorContext<Self>, state: &mut Self::S) -> anyhow::Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    fn transform(&self, message: Box<dyn Any + Send + 'static>) -> Option<Self::M> {
        None
    }
}

pub trait Message: Any + Send + Sized + Debug + 'static {
    fn downcast(message: Box<dyn Any + Send + 'static>) -> Result<Self, Box<dyn Any + Send + 'static>> {
        match message.downcast::<Self>() {
            Ok(message) => {
                Ok(*message)
            }
            Err(message) => {
                Err(message)
            }
        }
    }
}

impl<T> Message for T where T: Any + Send + Sized + Debug + 'static {}

pub trait State: Any + 'static {}

impl<T> State for T where T: Any + 'static {}

pub trait Arg: Any + Send + 'static {}

impl<T> Arg for T where T: Any + Send + 'static {}
