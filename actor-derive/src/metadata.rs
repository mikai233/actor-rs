#[derive(Debug)]
pub enum CodecType {
    NonCodec,
    Codec,
}

#[derive(Debug)]
pub enum MessageImpl {
    Message,
    SystemMessage,
    OrphanMessage,
}