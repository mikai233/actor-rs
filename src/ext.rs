use std::sync::atomic::{AtomicI64, Ordering};

use anyhow::{anyhow, Ok};
use bytes::BytesMut;
use serde::{Deserialize, Serialize};
use tracing_subscriber::fmt::time::LocalTime;

const BASE64_CHARS: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+~";
static ACTOR_NAME_OFFSET: AtomicI64 = AtomicI64::new(0);

pub fn read_u16(src: &BytesMut, offset: usize) -> anyhow::Result<u16> {
    let mut u16_bytes = [0u8; 2];
    u16_bytes.copy_from_slice(&src[offset..(offset + 2)]);
    let num_u16 = u16::from_be_bytes(u16_bytes);
    Ok(num_u16)
}

pub fn read_u32(src: &BytesMut, offset: usize) -> anyhow::Result<u32> {
    let mut u32_bytes = [0u8; 4];
    u32_bytes.copy_from_slice(&src[offset..(offset + 4)]);
    let num_u32 = u32::from_be_bytes(u32_bytes);
    Ok(num_u32)
}

pub fn encode_bytes<T>(value: &T) -> anyhow::Result<Vec<u8>>
where
    T: Serialize,
{
    let bytes = bincode::serialize(value)?;
    Ok(bytes)
}

pub fn decode_bytes<'a, T>(bytes: &'a [u8]) -> anyhow::Result<T>
where
    T: Deserialize<'a>,
{
    let value = bincode::deserialize(bytes)?;
    Ok(value)
}

pub fn init_logger(level: tracing::Level) {
    let format = tracing_subscriber::fmt::format()
        .with_timer(LocalTime::rfc_3339())
        .pretty()
        .compact();
    tracing_subscriber::FmtSubscriber::builder()
        .event_format(format)
        .with_max_level(level)
        .init();
}

fn base64(l: i64, mut s: String) -> String {
    let index = l & 63;
    let (_, c) = BASE64_CHARS
        .char_indices()
        .find(|x| x.0 == index as usize)
        .unwrap();
    s.push(c);
    let next = (l >> 6).abs();
    if next == 0 {
        s
    } else {
        base64(next, s)
    }
}

pub fn random_actor_name() -> String {
    let num = ACTOR_NAME_OFFSET.fetch_add(1, Ordering::Relaxed);
    base64(num, "$".to_string())
}

pub fn check_name(name: &String) -> anyhow::Result<()> {
    let valid = name.chars().all(|c| BASE64_CHARS.contains(c));
    if valid {
        Ok(())
    } else {
        Err(anyhow!(
            "name {} is invalid, allowed chars {}",
            name,
            BASE64_CHARS
        ))
    }
}

#[cfg(test)]
mod ext_test {
    use crate::ext::random_actor_name;

    #[test]
    fn test_random_name() {
        for _ in 0..10000 {
            random_actor_name();
        }
    }
}
