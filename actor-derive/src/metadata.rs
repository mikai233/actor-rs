#[derive(Debug)]
pub enum CodecType {
    NoneSerde,
    Serde,
}

#[derive(Debug)]
pub enum MessageImpl {
    Message,
    SystemMessage,
    OrphanMessage,
}