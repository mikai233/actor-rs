use config::{Map, Value};
use imstr::ImString;

#[derive(Debug)]
pub(crate) struct ArterySettings {
    pub(crate) enabled: bool,
}

impl ArterySettings {
    pub(crate) fn new(config: Map<String, Value>) -> anyhow::Result<Self> {}
}

#[derive(Debug, Clone)]
pub(crate) struct Canonical {
    pub(crate) port: u16,
    pub(crate) host_name: ImString,
}

impl Canonical {
    pub(crate) fn new(mut config: Map<String, Value>) -> anyhow::Result<Self> {
        let port = config.remove("port").ok_or_else(|| anyhow::anyhow!("port not found"))?.try_deserialize::<u16>()?;
    }

    pub(crate) fn get_host_name(key: &str, config: &mut Map<String, Value>) -> anyhow::Result<ImString> {
        let host_name = config.remove(key).ok_or_else(|| anyhow::anyhow!("{} not found", key))?.try_deserialize::<String>()?;
        match host_name.as_str() {
            "<getHostAddress>" => {}
            "<getHostName>" => {}
            other => {}
        }
        Ok(ImString::new(host_name))
    }
}