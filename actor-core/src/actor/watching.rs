use ahash::HashMap;
use std::fmt::Debug;

use crate::actor_ref::ActorRef;
use crate::message::DynMessage;

#[derive(Debug, Default, derive_more::Deref, derive_more::DerefMut)]
pub(crate) struct Watching(HashMap<ActorRef, Option<DynMessage>>);