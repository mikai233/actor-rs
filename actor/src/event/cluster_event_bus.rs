use std::collections::HashMap;

use async_trait::async_trait;
use etcd_client::{Client, GetOptions};
use tracing::error;

use actor_derive::EmptyCodec;

use crate::{Actor, Message};
use crate::actor_ref::ActorRef;
use crate::context::{ActorContext, Context};
use crate::message::terminated::WatchTerminated;
use crate::provider::{ActorRefFactory, TActorRefProvider};

#[derive(Debug)]
pub(crate) struct ClusterEventBusActor;

#[derive(Debug)]
struct ClusterSubscriber {
    path: String,
    subscriber: ActorRef,
}

#[async_trait]
impl Actor for ClusterEventBusActor {
    async fn pre_start(&mut self, _context: &mut ActorContext) -> anyhow::Result<()> {
        todo!()
        // let mut client = context.system.eclient().clone();
        // let subscribers = match self.get_subscribers(context, &mut client).await {
        //     Ok(subscribers) => subscribers,
        //     Err(err) => {
        //         error!("get cluster subscribers error {:?}", err);
        //         HashMap::new()
        //     }
        // };
        // let state = State {
        //     client,
        //     subscribers,
        // };
        // Ok(state)
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
                    let subscriber = ClusterSubscriber {
                        path: path.to_string(),
                        subscriber: actor_ref,
                    };
                    let w = WatchSubscriberTerminate {
                        watch: subscriber.subscriber.clone(),
                    };
                    context.watch(w);
                    subscribers.entry(event.to_string()).or_insert(Vec::new()).push(subscriber);
                }
            }
        }
        Ok(subscribers)
    }
}

#[derive(Debug, EmptyCodec)]
struct WatchSubscriberTerminate {
    watch: ActorRef,
}

impl WatchTerminated for WatchSubscriberTerminate {
    fn watch_actor(&self) -> &ActorRef {
        &self.watch
    }
}

impl Message for WatchSubscriberTerminate {
    type A = ClusterEventBusActor;

    fn handle(self: Box<Self>, _context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        // for subscribers in state.subscribers.values_mut() {
        //     subscribers.retain(|subscriber| {
        //         subscriber.path != self.watch.path()
        //     });
        // }
        Ok(())
    }
}