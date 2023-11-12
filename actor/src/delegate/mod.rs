use crate::actor::Actor;
use crate::delegate::system::SystemDelegate;
use crate::delegate::user::UserDelegate;

pub(crate) mod user;
pub(crate) mod system;


pub(crate) enum MessageDelegate<T> where T: Actor {
    User(Box<UserDelegate<T>>),
    System(Box<SystemDelegate>),
}

impl<T> MessageDelegate<T> where T: Actor {
    pub(crate) fn name(&self) -> &'static str {
        match self {
            MessageDelegate::User(m) => { m.name }
            MessageDelegate::System(m) => { m.name }
        }
    }
}