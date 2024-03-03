use actor_derive::OrphanEmptyCodec;
use crate::routing::routee::Routee;

#[derive(OrphanEmptyCodec)]
pub struct GetRouteesResp {
    pub routees: Vec<Box<dyn Routee>>,
}