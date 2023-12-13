use std::any::Any;
use anyhow::anyhow;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{Actor, CodecMessage, DynMessage, SystemMessage};
use crate::actor::actor_ref::ActorRef;
use crate::actor::actor_ref_provider::ActorRefProvider;
use crate::actor::context::{ActorContext, Context};
use crate::actor::decoder::MessageDecoder;
use crate::actor::fault_handing::ChildRestartStats;
use crate::actor::serialized_ref::SerializedActorRef;
use crate::delegate::system::SystemDelegate;
use crate::ext::{decode_bytes, encode_bytes};

#[derive(Debug)]
pub struct Failed {
    pub child: ActorRef,
}

impl CodecMessage for Failed {
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
                let child = provider.resolve_actor_ref(&serialized.path);
                let message = SystemDelegate::new(Failed { child });
                Ok(message.into())
            }
        }
        Some(Box::new(D))
    }

    fn encode(&self) -> Option<anyhow::Result<Vec<u8>>> {
        let serialized: SerializedActorRef = self.child.clone().into();
        Some(encode_bytes(&serialized))
    }

    fn dyn_clone(&self) -> Option<DynMessage> {
        None
    }
}

#[async_trait]
impl SystemMessage for Failed {
    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut dyn Actor) -> anyhow::Result<()> {
        let Self { child } = *self;
        let restart_stats = ChildRestartStats {
            child: child.clone(),
            max_nr_of_retries_count: 0,
            restart_time_window_start_nanos: 0,
        };
        let children = context.children().values().map(|child| {
            ChildRestartStats {
                child: child.clone(),
                max_nr_of_retries_count: 0,
                restart_time_window_start_nanos: 0,
            }
        }).collect();
        actor.supervisor_strategy().handle_failure(context, &child, anyhow!(""), restart_stats, children);
        Ok(())
    }
}