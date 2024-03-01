use thiserror::Error;

pub type Result<T, E = Error> = core::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("duplicate actor name {0}")]
    DuplicateActorName(String),
    #[error("cannot spawn child actor while parent actor is terminating")]
    SpawnActorWhileTerminating,
}