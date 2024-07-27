use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum MessageBuffer {
    Drop,
    Bound(usize),
    Unbound,
}

impl Display for MessageBuffer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageBuffer::Drop => {
                write!(f, "Drop")
            }
            MessageBuffer::Bound(s) => {
                write!(f, "Bound({s})")
            }
            MessageBuffer::Unbound => {
                write!(f, "Unbound")
            }
        }
    }
}
