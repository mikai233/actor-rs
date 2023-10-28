use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub(crate) protocol: String,
    pub(crate) system: String,
    pub(crate) host: Option<String>,
    pub(crate) port: Option<u16>,
}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}://{}", self.protocol, self.system)?;
        if let Some(host) = &self.host {
            write!(f, "@{}", host)?;
        }
        if let Some(port) = &self.port {
            write!(f, ":{}", port)?;
        }
        Ok(())
    }
}