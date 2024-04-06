use std::ops::{Deref, DerefMut};

use bincode::{Decode, Encode};
use eyre::Context;
use tokio_util::bytes::{BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

use actor_core::ext::read_u32;
use actor_core::eyre;

#[derive(Debug, Clone, Encode, Decode)]
pub struct Packet {
    pub body: Vec<u8>,
}

impl Packet {
    pub fn new(body: Vec<u8>) -> Self {
        Self {
            body
        }
    }
}

impl Deref for Packet {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.body
    }
}

impl DerefMut for Packet {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.body
    }
}

#[derive(Debug)]
pub struct PacketCodec;

impl Encoder<Packet> for PacketCodec {
    type Error = eyre::Error;

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
    type Error = eyre::Error;

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
            Ok(Some(Packet::new(src[4..].to_vec())))
        };
    }
}