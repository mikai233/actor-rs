use std::any::Any;
use std::mem::MaybeUninit;
use std::sync::Arc;

use tracing::trace;

use actor_derive::EmptyCodec;

use crate::{Actor, CodecMessage, DynMessage, Message};
use crate::actor::actor_ref::ActorRef;
use crate::actor::actor_ref_factory::ActorRefFactory;
use crate::actor::actor_ref_provider::ActorRefProvider;
use crate::actor::context::{ActorContext, Context};
use crate::actor::decoder::MessageDecoder;
use crate::actor::routed_actor_ref::RoutedActorRef;
use crate::actor::serialized_ref::SerializedActorRef;
use crate::delegate::user::UserDelegate;
use crate::ext::{decode_bytes, encode_bytes};
use crate::message::terminated::WatchTerminated;
use crate::routing::router_config::RouterConfig;

pub struct RouterActor {
    pub routed_ref: MaybeUninit<RoutedActorRef>,
}

impl Actor for RouterActor {}

pub(crate) struct WatchRouteeTerminated(ActorRef);

impl Message for WatchRouteeTerminated {
    type A = RouterActor;

    fn handle(self: Box<Self>, _context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}

impl CodecMessage for WatchRouteeTerminated {
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn decoder() -> Option<Box<dyn MessageDecoder>> where Self: Sized {
        #[derive(Clone)]
        struct D;
        impl MessageDecoder for D {
            fn decode(&self, provider: &ActorRefProvider, bytes: &[u8]) -> anyhow::Result<DynMessage> {
                let serialized: SerializedActorRef = decode_bytes(bytes)?;
                let terminated = provider.resolve_actor_ref(&serialized.path);
                let message = UserDelegate::new(WatchRouteeTerminated(terminated));
                Ok(message.into())
            }
        }
        Some(Box::new(D))
    }

    fn encode(&self) -> Option<anyhow::Result<Vec<u8>>> {
        let serialized: SerializedActorRef = self.0.clone().into();
        Some(encode_bytes(&serialized))
    }

    fn dyn_clone(&self) -> Option<DynMessage> {
        None
    }
}

impl WatchTerminated for WatchRouteeTerminated {
    fn watch_actor(&self) -> &ActorRef {
        &self.0
    }
}

#[derive(EmptyCodec)]
pub(crate) struct InitRoutedRef(pub(crate) RoutedActorRef);

impl Message for InitRoutedRef {
    type A = RouterActor;

    fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let myself = context.myself();
        trace!("{} routed ref init", myself);
        actor.routed_ref.write(self.0);
        let routed_ref = unsafe { actor.routed_ref.assume_init_ref() };
        let router_props = &routed_ref.router_props;
        let mut routee_props = router_props.with_router(None);
        match &**router_props.router_config.as_ref().unwrap() {
            RouterConfig::PoolRouterConfig(pool) => {
                let nr_of_routees = pool.nr_of_instances(context.system());
                let mut routees = vec![];
                for _ in 0..nr_of_routees {
                    let routee = pool.new_routee(routee_props.clone(), context)?;
                    routees.push(Arc::new(routee));
                }
                routed_ref.add_routees(routees);
            }
            RouterConfig::GroupRouterConfig(group) => {
                todo!()
            }
        }
        Ok(())
    }
}