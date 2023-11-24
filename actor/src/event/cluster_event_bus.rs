use std::collections::HashMap;

use async_trait::async_trait;
use etcd_client::{Client, GetOptions};
use serde::{Deserialize, Serialize};
use tracing::error;

use actor_derive::MessageCodec;

use crate::{Actor, Message};
use crate::actor_ref::{ActorRef, SerializedActorRef};
use crate::actor_ref::TActorRef;
use crate::context::{ActorContext, Context};
use crate::message::terminated::WatchTerminated;
use crate::provider::{ActorRefFactory, TActorRefProvider};
use crate::system::ActorSystem;

#[derive(Debug)]
pub(crate) struct ClusterEventBusActor;

pub(crate) struct State {
    client: Client,
    subscribers: HashMap<String, Vec<ClusterSubscriber>>,
}

#[derive(Debug)]
struct ClusterSubscriber {
    path: String,
    subscriber: ActorRef,
}

#[async_trait]
impl Actor for ClusterEventBusActor {
    type S = State;
    type A = ();

    async fn pre_start(&self, context: &mut ActorContext, _arg: Self::A) -> anyhow::Result<Self::S> {
        let mut client = context.system.eclient().clone();
        let subscribers = match self.get_subscribers(context, &mut client).await {
            Ok(subscribers) => subscribers,
            Err(err) => {
                error!("get cluster subscribers error {:?}", err);
                HashMap::new()
            }
        };
        let state = State {
            client,
            subscribers,
        };
        Ok(state)
    }
}

impl ClusterEventBusActor {
    async fn get_subscribers(&self, context: &mut ActorContext, client: &mut Client) -> anyhow::Result<HashMap<String, Vec<ClusterSubscriber>>> {
        let mut subscribers = HashMap::new();
        let prefix = format!("{}/{}", context.system.name(), "event_bus");
        let get_opts = GetOptions::new().with_prefix();
        let resp = client.get(&*prefix, Some(get_opts)).await?;
        for kv in resp.kvs() {
            let key = kv.key_str()?;
            match key.split("/").last() {
                None => {
                    error!("no event name found at {}", prefix);
                }
                Some(event) => {
                    let path = kv.value_str()?;
                    let actor_ref = context.system.provider().resolve_actor_ref(path);
                    let watch_ref: SerializedActorRef = actor_ref.clone().into();
                    let subscriber = ClusterSubscriber {
                        path: path.to_string(),
                        subscriber: actor_ref,
                    };
                    let w = WatchSubscriberTerminate {
                        watch: watch_ref,
                    };
                    context.watch(w);
                    subscribers.entry(event.to_string()).or_insert(Vec::new()).push(subscriber);
                }
            }
        }
        Ok(subscribers)
    }
}

#[derive(Debug, Serialize, Deserialize, MessageCodec)]
#[actor(ClusterEventBusActor)]
struct WatchSubscriberTerminate {
    watch: SerializedActorRef,
}

impl WatchTerminated for WatchSubscriberTerminate {
    fn watch_actor(&self, system: &ActorSystem) -> ActorRef {
        system.provider().resolve_actor_ref(&self.watch.path)
    }
}

impl Message for WatchSubscriberTerminate {
    type T = ClusterEventBusActor;

    fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        for subscribers in state.subscribers.values_mut() {
            subscribers.retain(|subscriber| {
                subscriber.path != self.watch.path
            });
        }
        Ok(())
    }
}