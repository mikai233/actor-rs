pub mod failure_detector_registry;
pub mod phi_accrual_failure_detector;

pub trait FailureDetector: Send {
    fn is_available(&self) -> bool;

    fn is_monitoring(&self) -> bool;

    fn heartbeat(&mut self);
}