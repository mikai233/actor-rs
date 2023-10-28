use crate::net::connection::Connection;

pub trait ConnectionListener: Send + Sync + Clone + 'static {
    fn on_connected(&self, connection: &Connection);
    fn on_recv(&self, connection: &Connection, frame: Vec<u8>);
    fn on_disconnected(&self, connection: &Connection);
}