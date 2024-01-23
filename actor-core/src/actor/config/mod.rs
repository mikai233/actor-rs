use std::any::Any;
use std::fmt::Debug;

use dyn_clone::DynClone;

use crate::ext::as_any::AsAny;

pub mod actor_setting;
pub mod core_config;
mod mailbox;

pub trait Config: Debug + Send + Sync + Sized + Clone {
    fn merge(&self, other: Self) -> Self;
}