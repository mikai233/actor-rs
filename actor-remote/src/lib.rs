pub const REFERENCE: &'static str = include_str!("../reference.json");

pub mod artery;
pub mod codec;
pub mod config;
mod failure_detector;
pub mod remote_actor_ref;
pub mod remote_provider;
mod remote_watcher;

#[cfg(test)]
mod test {
    use tracing::Level;

    use actor_core::ext::init_logger;

    #[ctor::ctor]
    fn init() {
        init_logger(Level::DEBUG)
    }
}
