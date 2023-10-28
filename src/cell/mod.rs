use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use crate::actor_ref::ActorRef;

pub(crate) mod runtime;
pub(crate) mod envelope;

#[derive(Debug, Clone)]
pub(crate) struct ActorCell {
    pub(crate) parent: Option<Arc<ActorCell>>,
    pub(crate) myself: ActorRef,
    pub(crate) children: Arc<RwLock<BTreeMap<String, ActorRef>>>,
}