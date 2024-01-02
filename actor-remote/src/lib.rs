pub mod remote_provider;
pub mod net;
pub mod remote_actor_ref;

#[cfg(test)]
mod test {
    use tracing::Level;
    use actor_core::ext::init_logger;

    #[ctor::ctor]
    fn init() {
        init_logger(Level::DEBUG)
    }
}