use crate::context::ActorContext;
use crate::props::Props;
use crate::routing::router::{Routee, Router};
use crate::routing::router_actor::RouterActor;
use crate::system::ActorSystem;

pub trait RouterConfig {
    fn create_router(&self, system: ActorSystem) -> Router;
    fn create_router_actor(&self) -> RouterActor;
}

pub trait Pool {
    fn nr_of_instances(sys: &ActorSystem) -> usize;
    fn new_routee(routee_props: Props, context: ActorContext) -> Box<dyn Routee>;
}

// #[derive(Debug, EmptyCodec)]
// pub enum RouterMessage {
//     GetRoutees,
//     AddRoutee(Box<dyn Routee>),
//     RemoveRoutee(Box<dyn Routee>),
// }
//
// impl Message for RouterMessage {
//     type A = RouterActor;
//
//     fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
//         todo!()
//     }
// }