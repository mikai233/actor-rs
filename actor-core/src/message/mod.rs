use std::any::Any;
use std::fmt::{Debug, Display, Formatter};

pub mod address_terminated;
pub mod death_watch_notification;
pub mod execute;
pub mod failed;
pub mod handler;
pub mod identify;
pub mod message_buffer;
pub mod poison_pill;
pub mod resume;
pub mod stop_child;
pub mod suspend;
pub(crate) mod task_finish;
pub mod terminate;
pub mod terminated;
pub mod unwatch;
pub mod watch;

pub type DynMessage = Box<dyn Message>;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Signature {
    pub name: &'static str,
    pub type_id: std::any::TypeId,
}

impl Signature {
    pub fn new<T: ?Sized + 'static>() -> Self {
        Self {
            name: std::any::type_name::<T>(),
            type_id: std::any::TypeId::of::<T>(),
        }
    }
}

impl Display for Signature {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

pub trait Message: Any + Send + Display + Debug {
    fn signature_sized() -> Signature
    where
        Self: Sized;

    fn signature(&self) -> Signature;

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
