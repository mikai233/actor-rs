use std::fmt::Debug;

pub trait FailureDetectorRegistry: Debug + Send {
    type A;

    fn is_available(&self, resource: &Self::A) -> bool;

    fn is_monitoring(&self, resource: &Self::A) -> bool;

    fn heartbeat(&self, resource: Self::A);

    fn remove(&mut self, resource: &Self::A);

    fn reset(&mut self);
}