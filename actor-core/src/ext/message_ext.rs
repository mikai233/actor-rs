use crate::{DynMessage, Message, OrphanMessage, SystemMessage};

pub trait UserMessageExt {
    fn into_dyn(self) -> DynMessage;
}

impl<M> UserMessageExt for M where M: Message {
    fn into_dyn(self) -> DynMessage {
        DynMessage::user(self)
    }
}

pub trait SystemMessageExt {
    fn into_dyn(self) -> DynMessage;
}

impl<M> SystemMessageExt for M where M: SystemMessage {
    fn into_dyn(self) -> DynMessage {
        DynMessage::system(self)
    }
}

pub trait OrphanMessageExt {
    fn into_dyn(self) -> DynMessage;
}

impl<M> OrphanMessageExt for M where M: OrphanMessage {
    fn into_dyn(self) -> DynMessage {
        DynMessage::orphan(self)
    }
}