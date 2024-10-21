use anyhow::Context;
use serde::{Deserialize, Serialize};
use tokio_util::bytes::{BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

use actor_core::ext::read_u32;

#[derive(Debug, Clone, Serialize, Deserialize, derive_more::Deref)]
pub struct Packet(pub Vec<u8>);

#[derive(Debug)]
pub struct PacketCodec;

impl Encoder<Packet> for PacketCodec {
    type Error = anyhow::Error;

    fn encode(&mut self, item: Packet, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let len = item.len();
        let len = u32::try_from(len).context("packet too large")?;
        dst.put_u32(len);
        dst.put_slice(&item);
        Ok(())
    }
}

impl Decoder for PacketCodec {
    type Item = Packet;
    type Error = anyhow::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let buf_len = src.len();
        if buf_len < 4 {
            return Ok(None);
        }
        let body_len = read_u32(src, 0);
        return if body_len > (buf_len - 4) as u32 {
            src.reserve(body_len as usize);
            Ok(None)
        } else {
            let src = src.split_to(4 + body_len as usize);
            Ok(Some(Packet(src[4..].to_vec())))
        };
    }
}
