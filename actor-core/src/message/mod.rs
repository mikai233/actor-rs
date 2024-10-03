use std::any::Any;
use std::fmt::{Debug, Display};


pub mod death_watch_notification;
pub mod terminated;
pub mod terminate;
pub mod watch;
pub mod unwatch;
pub mod poison_pill;
pub mod execute;
pub mod suspend;
pub mod resume;
pub mod failed;
pub mod identify;
pub mod codec;
pub mod stop_child;
pub mod message_buffer;
pub mod address_terminated;
pub(crate) mod task_finish;
pub mod handler;

pub type DynMessage = Box<dyn Message>;

pub trait Message: Any + Send + Display + Debug {
    fn signature_sized() -> &'static str
    where
        Self: Sized;

    fn signature(&self) -> &'static str;

    fn as_any(&self) -> &dyn Any;

    fn into_any(self: Box<Self>) -> Box<dyn Any>;

    fn is_cloneable(&self) -> bool;

    fn clone_box(&self) -> Option<Box<dyn Message>>;
}

pub fn downcast_ref<M>(message: &impl AsRef<dyn Message>) -> Option<&M>
where
    M: Message,
{
    message.as_ref().as_any().downcast_ref()
}

pub fn downcast_into<M>(message: Box<dyn Message>) -> Result<Box<M>, Box<dyn Any>>
where
    M: Message,
{
    message.into_any().downcast()
}


