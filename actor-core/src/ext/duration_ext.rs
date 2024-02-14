use std::time::Duration;

pub trait DurationExt {
    fn millis(self) -> Duration;

    fn seconds(self) -> Duration;
}

impl DurationExt for u64 {
    fn millis(self) -> Duration {
        Duration::from_millis(self)
    }

    fn seconds(self) -> Duration {
        Duration::from_secs(self)
    }
}