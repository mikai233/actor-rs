use std::any::Any;
use std::fmt::Debug;

use dyn_clone::DynClone;

use crate::ext::as_any::AsAny;

pub mod actor_setting;
pub mod actor_config;

pub trait Config: Debug + Send + Sync + Any + AsAny + DynClone {
    fn merge(&self, other: Box<dyn Config>) -> anyhow::Result<Box<dyn Config>>;
}

dyn_clone::clone_trait_object!(Config);
