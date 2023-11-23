use std::net::SocketAddrV4;

use crate::message::MessageRegistration;

pub struct Config {
    name: String,
    addr: SocketAddrV4,
    reg: MessageRegistration,
    client: etcd_client::Client,
}