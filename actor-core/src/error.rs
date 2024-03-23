use thiserror::Error;

pub type Result<T, E = Error> = core::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("actor name {0} is invalid, allowed chars a..=z, A..=Z, 0..=9, _")]
    ActorNameInvalid(String),
    #[error("duplicate actor name {0}")]
    DuplicateActorName(String),
    #[error("cannot spawn child actor while parent actor is terminating")]
    SpawnActorWhileTerminating,
    #[error("message {0} is not cloneable")]
    MessageCannotClone(String),
    #[error("the actor system is destroyed, any weak reference ref to that is invalid")]
    ActorSystemDestroyed,
}