use crate::{Actor, Message};
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

    pub fn into_raw(self) -> Box<dyn Message<A=A>> {
        match self {
            MessageDelegate::User(m) => m.message,
            MessageDelegate::AsyncUser(m) => m.message,
            MessageDelegate::System(m) => m.message
        }
    }
}