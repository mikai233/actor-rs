use std::fmt::{Display, Formatter};

#[derive(Debug, Copy, Clone, Default)]
pub enum Directive {
    #[default]
    Resume,
    Stop,
    Escalate,
}

impl Display for Directive {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Directive::Resume => {
                write!(f, "Resume")
            }
            Directive::Stop => {
                write!(f, "Stop")
            }
            Directive::Escalate => {
                write!(f, "Escalate")
            }
        }
    }
}