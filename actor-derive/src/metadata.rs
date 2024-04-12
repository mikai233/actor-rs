use syn::Path;

#[derive(Debug)]
pub enum CodecType {
    NonCodec,
    Codec,
}

pub enum MessageImpl {
    Message,
    SystemMessage,
    OrphanMessage,
    Delegate(Path),
}
