use crate::DynamicMessage;

pub trait MessageFactory: Send + 'static {
    fn message(&self) -> DynamicMessage;
}