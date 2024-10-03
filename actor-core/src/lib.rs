use async_trait::async_trait;
use bincode::{Decode, Encode};

#[cfg(feature = "derive")]
pub use actor_derive::{self, *};

use crate::actor::context::{ActorContext, Context};
use crate::actor::directive::Directive;
use crate::actor_ref::ActorRef;
use crate::message::MessageDecoder;

pub const REFERENCE: &'static str = include_str!("../reference.toml");

pub mod ext;
mod cell;
pub mod delegate;
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