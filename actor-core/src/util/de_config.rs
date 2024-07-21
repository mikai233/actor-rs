use config::Value;
use serde::Deserialize;

pub fn de_maybe_off_config<'de, T>(value: Value) -> anyhow::Result<Option<T>>
where
    T: Deserialize<'de>,
{
    if value.origin().is_some_and(|v| v.to_lowercase() == "off") {
        Ok(None)
    } else {
        let value: T = value.try_deserialize()?;
        Ok(Some(value))
    }
}
