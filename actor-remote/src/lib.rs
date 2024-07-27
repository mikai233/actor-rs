pub const REFERENCE: &'static str = include_str!("../reference.toml");

pub mod remote_provider;
pub mod transport;
pub mod remote_actor_ref;
pub mod config;
mod remote_watcher;
mod failure_detector;
pub(crate) mod artery;

#[cfg(test)]
mod test {
    use tracing::Level;

    use actor_core::ext::init_logger;

    #[ctor::ctor]
    fn init() {
        init_logger(Level::DEBUG)
    }
}