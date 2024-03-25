pub(crate) const REMOTE_CONFIG_NAME: &'static str = "remote.toml";
pub(crate) const REMOTE_CONFIG: &'static str = include_str!("../remote.toml");

pub mod remote_provider;
pub mod net;
pub mod remote_actor_ref;
pub mod remote_setting;
pub mod config;
mod remote_watcher;
mod failure_detector;

#[cfg(test)]
mod test {
    use tracing::Level;

    use actor_core::ext::init_logger;

    #[ctor::ctor]
    fn init() {
        init_logger(Level::DEBUG)
    }
}