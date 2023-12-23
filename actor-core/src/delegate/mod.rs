use std::any::Any;

use crate::Actor;
use crate::delegate::system::SystemDelegate;
use crate::delegate::user::{AsyncUserDelegate, UserDelegate};

pub mod user;
pub mod system;

pub(crate) enum MessageDelegate<A> where A: Actor {
    User(Box<UserDelegate<A>>),
    AsyncUser(Box<AsyncUserDelegate<A>>),
    System(Box<SystemDelegate>),
}

impl<A> MessageDelegate<A> where A: Actor {
    pub(crate) fn name(&self) -> &'static str {
        match self {
            MessageDelegate::User(m) => { m.name }
            MessageDelegate::AsyncUser(m) => { m.name }
            MessageDelegate::System(m) => { m.name }
        }
    }

    pub fn into_any(self) -> Box<dyn Any> {
        match self {
            MessageDelegate::User(m) => m.message.into_any(),
            MessageDelegate::AsyncUser(m) => m.message.into_any(),
            MessageDelegate::System(m) => m.message.into_any(),
        }
    }

    pub fn as_any(&self) -> &dyn Any {
        match self {
            MessageDelegate::User(m) => m.message.as_any(),
            MessageDelegate::AsyncUser(m) => m.message.as_any(),
            MessageDelegate::System(m) => m.message.as_any(),
        }
    }
}

pub(crate) enum MessageDelegateRef<'a, A> where A: Actor {
    User(&'a UserDelegate<A>),
    AsyncUser(&'a AsyncUserDelegate<A>),
    System(&'a SystemDelegate),
}

impl<'a, A> MessageDelegateRef<'a, A> where A: Actor {
    pub(crate) fn name(&self) -> &'static str {
        match self {
            MessageDelegateRef::User(m) => { m.name }
            MessageDelegateRef::AsyncUser(m) => { m.name }
            MessageDelegateRef::System(m) => { m.name }
        }
    }

    pub fn into_any(self) -> &'a dyn Any {
        match self {
            MessageDelegateRef::User(m) => m.message.as_any(),
            MessageDelegateRef::AsyncUser(m) => m.message.as_any(),
            MessageDelegateRef::System(m) => m.message.as_any(),
        }
    }
}