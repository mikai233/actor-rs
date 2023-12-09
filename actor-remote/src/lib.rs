pub mod remote_provider;
pub mod net;
mod remote_actor_ref;
mod message_registration;

#[cfg(test)]
mod test {
    use tracing::Level;
    use actor_core::ext::init_logger;

    #[ctor::ctor]
    fn init() {
        init_logger(Level::DEBUG)
    }
}