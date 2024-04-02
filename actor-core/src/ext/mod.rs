use std::any::type_name;
use std::sync::atomic::{AtomicI64, Ordering};

use bincode::{Decode, Encode};
use bytes::BytesMut;
use eyre::{anyhow, Context, Ok};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::time::LocalTime;

pub mod option_ext;
pub mod as_any;
pub mod etcd_client;
pub mod maybe_ref;
pub mod duration_ext;
pub mod message_ext;

const BASE64_CHARS: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+~";
static ACTOR_NAME_OFFSET: AtomicI64 = AtomicI64::new(0);

pub fn read_u16(src: &BytesMut, offset: usize) -> u16 {
    let mut u16_bytes = [0u8; 2];
    u16_bytes.copy_from_slice(&src[offset..(offset + 2)]);
    u16::from_be_bytes(u16_bytes)
}

pub fn read_u32(src: &BytesMut, offset: usize) -> u32 {
    let mut u32_bytes = [0u8; 4];
    u32_bytes.copy_from_slice(&src[offset..(offset + 4)]);
    u32::from_be_bytes(u32_bytes)
}

pub fn encode_bytes<T>(value: &T) -> eyre::Result<Vec<u8>> where T: Encode {
    bincode::encode_to_vec(value, bincode::config::standard()).context(type_name::<T>())
}

pub fn decode_bytes<T>(bytes: &[u8]) -> eyre::Result<T> where T: Decode {
    bincode::decode_from_slice(bytes, bincode::config::standard()).context(type_name::<T>()).map(|(t, _)| t)
}

pub fn init_logger(level: tracing::Level) {
    let format = tracing_subscriber::fmt::format()
        .with_timer(LocalTime::rfc_3339())
        .pretty();
    tracing_subscriber::FmtSubscriber::builder()
        .event_format(format)
        .with_max_level(level)
        .init();
}

pub fn init_logger_with_filter(filter: impl Into<EnvFilter>) {
    let format = tracing_subscriber::fmt::format()
        .with_timer(LocalTime::rfc_3339())
        .pretty()
        .with_file(false);
    tracing_subscriber::FmtSubscriber::builder()
        .event_format(format)
        .with_env_filter(filter)
        .init();
}

pub(crate) fn base64(l: i64, mut s: String) -> String {
    let index = (l & 63) as usize;
    let c = BASE64_CHARS
        .get(index..index + 1)
        .unwrap();
    s.push_str(c);
    let next = (l >> 6).abs();
    if next == 0 {
        s
    } else {
        base64(next, s)
    }
}

pub(crate) fn random_actor_name() -> String {
    random_name("$".to_string())
}

pub(crate) fn random_name(prefix: String) -> String {
    let num = ACTOR_NAME_OFFSET.fetch_add(1, Ordering::Relaxed);
    base64(num, prefix)
}

pub(crate) fn check_name(name: &String) -> eyre::Result<()> {
    let valid = name.chars().all(|c| match c {
        'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => true,
        _ => false
    });
    if valid {
        Ok(())
    } else {
        Err(anyhow!( "name {} is invalid, allowed chars a..=z, A..=Z, 0..=9, _", name, ))
    }
}