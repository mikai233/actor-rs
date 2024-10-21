use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum BufferType {
    Drop,
    Bound(usize),
    Unbound,
}

impl Display for BufferType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BufferType::Drop => {
                write!(f, "Drop")
            }
            BufferType::Bound(s) => {
                write!(f, "Bound({s})")
            }
            BufferType::Unbound => {
                write!(f, "Unbound")
            }
        }
    }
}
