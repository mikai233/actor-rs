#[derive(Debug)]
pub(crate) enum CodecType {
    NoneSerde,
    Serde,
}

#[derive(Debug)]
pub(crate) enum MessageImpl {
    Message,
    AsyncMessage,
    SystemMessage,
    DeferredMessage,
    UntypedMessage,
}