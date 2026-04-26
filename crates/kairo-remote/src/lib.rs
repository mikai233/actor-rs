pub(crate) const REMOTE_CONFIG: &str = include_str!("../remote.toml");

pub mod config;
mod failure_detector;
pub mod remote_actor_ref;
pub mod remote_provider;
pub mod remote_setting;
mod remote_watcher;
pub mod transport;

#[cfg(test)]
mod test {
    use tracing::Level;

    use kairo_core::ext::init_logger;

    #[ctor::ctor]
    fn init() {
        init_logger(Level::DEBUG)
    }
}
