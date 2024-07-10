use std::fmt::{Display, Formatter};
use std::sync::OnceLock;

use bincode::{Decode, Encode};
use imstr::ImString;
use serde::{Deserialize, Serialize};

const UNDEFINED: i32 = 0;

#[derive(
    Debug,
    Clone,
    Ord,
    PartialOrd,
    Eq,
    PartialEq,
    Hash,
    Encode,
    Decode,
    Serialize,
    Deserialize
)]
pub struct Version {
    pub version: ImString,
}

impl Version {
    pub fn zero() -> &'static Version {
        static ZERO: OnceLock<Version> = OnceLock::new();
        ZERO.get_or_init(|| Version { version: "0.0.0".to_string() })
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.version)
    }
}