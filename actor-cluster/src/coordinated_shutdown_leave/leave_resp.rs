use actor_core::Message;

#[derive(Debug, Message, derive_more::Display)]
#[display("LeaveResp")]
pub(crate) struct LeaveResp;