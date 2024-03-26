use async_trait::async_trait;
use bincode::{Decode, Encode};

use actor_derive::CSystemCodec;

use crate::{Actor, SystemMessage};
use crate::actor::address::Address;
use crate::actor::context::ActorContext;

#[derive(Debug, Clone, Encode, Decode, CSystemCodec)]
pub struct AddressTerminated {
    pub address: Address,
}

#[async_trait]
impl SystemMessage for AddressTerminated {
    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut dyn Actor) -> anyhow::Result<()> {
        todo!()
    }
}