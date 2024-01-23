use std::fmt::Debug;

pub mod actor_setting;
pub mod core_config;
mod mailbox;

pub trait Config: Debug + Send + Sync + Sized + Clone {
    fn merge(&self, other: Self) -> Self;
}