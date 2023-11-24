use std::fmt::{Debug, Formatter};
use async_trait::async_trait;

use crate::Actor;
use crate::context::ActorContext;

#[derive(Debug)]
pub struct ClusterSingletonManager;

#[derive(Debug)]
pub enum Message {}

pub struct State {
    client: etcd_client::Client,
}

impl Debug for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State").field("client", &"..").finish()
    }
}

#[async_trait]
impl Actor for ClusterSingletonManager {
    type S = State;
    type A = ();

    async fn pre_start(&self, _context: &mut ActorContext, _arg: Self::A) -> anyhow::Result<Self::S> {
        todo!()
    }
}
