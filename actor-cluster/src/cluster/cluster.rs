use actor_core::actor::actor_system::ActorSystem;
use actor_derive::AsAny;

#[derive(AsAny)]
pub struct Cluster {
    system: ActorSystem,
}

impl Cluster {
    pub fn new(system: ActorSystem) -> Self {
        todo!()
    }
}