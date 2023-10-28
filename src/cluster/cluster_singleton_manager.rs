use std::fmt::{Debug, Formatter};
use crate::actor::Actor;
use crate::actor::context::ActorContext;


#[derive(Debug)]
pub struct ClusterSingletonManager {}

#[derive(Debug)]
pub enum Message {}

pub struct State {
    client: etcd_client::Client,
}

impl Debug for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State")
            .field("client", &"..")
            .finish()
    }
}

impl Actor for ClusterSingletonManager {
    type M = Message;
    type S = State;
    type A = ();

    fn pre_start(&self, ctx: &mut ActorContext<Self>, arg: Self::A) -> anyhow::Result<Self::S> {
        todo!()
    }

    fn on_recv(&self, ctx: &mut ActorContext<Self>, state: &mut Self::S, message: Self::M) -> anyhow::Result<()> {
        todo!()
    }
}