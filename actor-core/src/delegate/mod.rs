use std::any::Any;

use anyhow::anyhow;

use crate::{Actor, CodecMessage};
use crate::delegate::system::SystemDelegate;
use crate::delegate::user::UserDelegate;

pub mod user;
pub mod system;

pub(crate) enum MessageDelegate<A> where A: Actor {
    User(Box<UserDelegate<A>>),
    System(Box<SystemDelegate>),
}

impl<A> MessageDelegate<A> where A: Actor {
    pub(crate) fn name(&self) -> &'static str {
        match self {
            MessageDelegate::User(m) => { m.name }
            MessageDelegate::System(m) => { m.name }
        }
    }

    pub fn into_any(self) -> Box<dyn Any> {
        match self {
            MessageDelegate::User(m) => m.message.into_any(),
            MessageDelegate::System(m) => m.message.into_any(),
        }
    }

    pub fn as_any(&self) -> &dyn Any {
        match self {
            MessageDelegate::User(m) => m.message.as_any(),
            MessageDelegate::System(m) => m.message.as_any(),
        }
    }
}

pub(crate) enum MessageDelegateRef<'a, A> where A: Actor {
    User(&'a UserDelegate<A>),
    System(&'a SystemDelegate),
}

impl<'a, A> MessageDelegateRef<'a, A> where A: Actor {
    pub(crate) fn name(&self) -> &'static str {
        match self {
            MessageDelegateRef::User(m) => { m.name }
            MessageDelegateRef::System(m) => { m.name }
        }
    }

    pub fn into_any(self) -> &'a dyn Any {
        match self {
            MessageDelegateRef::User(m) => m.message.as_any(),
            MessageDelegateRef::System(m) => m.message.as_any(),
        }
    }
}

pub(crate) fn downcast_box_message<M: CodecMessage>(name: &'static str, msg: Box<dyn Any>) -> anyhow::Result<M> {
    match msg.downcast::<M>() {
        Ok(m) => {
            Ok(*m)
        }
        Err(_) => {
            Err(anyhow!("message {} cannot downcast to {}", name, std::any::type_name::<M>()))
        }
    }
}