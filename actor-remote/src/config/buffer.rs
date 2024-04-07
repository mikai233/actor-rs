use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum Buffer {
    NoBuffer,
    Bound(usize),
    Unbound,
}

impl Default for Buffer {
    fn default() -> Self {
        Self::Bound(10000)
    }
}

impl Display for Buffer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Buffer::NoBuffer => {
                write!(f, "NoBuffer")
            }
            Buffer::Bound(s) => {
                write!(f, "Bound({s})")
            }
            Buffer::Unbound => {
                write!(f, "Unbound")
            }
        }
    }
}
