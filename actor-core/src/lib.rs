#[cfg(feature = "derive")]
pub use actor_derive::{self, *};

pub const REFERENCE: &'static str = include_str!("../reference.toml");

pub mod ext;
mod cell;
pub mod message;
pub mod event;
pub mod routing;
pub mod actor;
pub mod config;
pub mod pattern;
pub mod actor_path;
pub mod actor_ref;
pub mod provider;
pub mod async_ref;
pub mod util;