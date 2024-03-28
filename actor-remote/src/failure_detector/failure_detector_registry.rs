use std::fmt::Debug;
use std::hash::Hash;

pub trait FailureDetectorRegistry: Debug + Send {
    type A: Hash + Eq;

    fn is_available(&self, resource: &Self::A) -> bool;

    fn is_monitoring(&self, resource: &Self::A) -> bool;

    fn heartbeat(&mut self, resource: Self::A);

    fn remove(&mut self, resource: &Self::A);

    fn reset(&mut self);
}