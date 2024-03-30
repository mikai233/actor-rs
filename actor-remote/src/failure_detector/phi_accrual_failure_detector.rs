use std::collections::VecDeque;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tracing::log::debug;
use tracing::warn;

use actor_core::event::event_stream::EventStream;

use crate::failure_detector::FailureDetector;

pub trait Clock {}

/// Implementation of 'The Phi Accrual Failure Detector' by Hayashibara et al. as defined in their paper:
/// [https://oneofus.la/have-emacs-will-hack/files/HDY04.pdf]
///
/// The suspicion level of failure is given by a value called φ (phi).
/// The basic idea of the φ failure detector is to express the value of φ on a scale that
/// is dynamically adjusted to reflect current network conditions. A configurable
/// threshold is used to decide if φ is considered to be a failure.
///
/// The value of φ is calculated as:
///
/// {{{
/// φ = -log10(1 - F(timeSinceLastHeartbeat)
/// }}}
/// where F is the cumulative distribution function of a normal distribution with mean
/// and standard deviation estimated from historical heartbeat inter-arrival times.
///
/// [threshold] A low threshold is prone to generate many wrong suspicions but ensures a quick detection in the event
///  of a real crash. Conversely, a high threshold generates fewer mistakes but needs more time to detect
///  actual crashes
/// [max_sample_size] Number of samples to use for calculation of mean and standard deviation of
///  inter-arrival times.
/// [min_std_deviation] Minimum standard deviation to use for the normal distribution used when calculating phi.
///  Too low standard deviation might result in too much sensitivity for sudden, but normal, deviations
///  in heartbeat inter arrival times.
/// [acceptable_heartbeat_pause] Duration corresponding to number of potentially lost/delayed
///  heartbeats that will be accepted before considering it to be an anomaly.
///  This margin is important to be able to survive sudden, occasional, pauses in heartbeat
///  arrivals, due to for example garbage collect or network drop.
/// [first_heartbeat_estimate] Bootstrap the stats with heartbeats that corresponds to
///  to this duration, with a with rather high standard deviation (since environment is unknown
///  in the beginning)
/// @param clock The clock, returning current time in milliseconds, but can be faked for testing
///  purposes. It is only used for measuring intervals (duration).
pub struct PhiAccrualFailureDetector {
    pub threshold: f64,
    pub max_sample_size: i32,
    pub min_std_deviation: Duration,
    pub acceptable_heartbeat_pause: Duration,
    pub first_heartbeat_estimate: Duration,
    pub event_stream: Option<EventStream>,
    state: State,
    first_heartbeat: HeartbeatHistory,
}

impl PhiAccrualFailureDetector {
    fn check_valid(&self) {
        assert!(self.threshold > 0.0, "failure-detector.threshold must be > 0");
        assert!(self.max_sample_size > 0, "failure-detector.max-sample-size must be > 0");
    }

    pub fn new(
        threshold: f64,
        max_sample_size: i32,
        min_std_deviation: Duration,
        acceptable_heartbeat_pause: Duration,
        first_heartbeat_estimate: Duration,
    ) -> Self {
        let first_heartbeat = Self::first_heartbeat(first_heartbeat_estimate, max_sample_size);
        let detector = Self {
            threshold,
            max_sample_size,
            min_std_deviation,
            acceptable_heartbeat_pause,
            first_heartbeat_estimate,
            event_stream: None,
            state: State {
                history: first_heartbeat.clone(),
                timestamp: None,
            },
            first_heartbeat,
        };
        detector.check_valid();
        detector
    }

    fn first_heartbeat(first_heartbeat_estimate: Duration, max_sample_size: i32) -> HeartbeatHistory {
        let mean = first_heartbeat_estimate.as_millis();
        let std_deviation = mean / 4;
        HeartbeatHistory::new(max_sample_size)
            .add_interval((mean - std_deviation) as i64)
            .add_interval((mean + std_deviation) as i64)
    }

    fn calc_phi(&self, timestamp: i64) -> f64 {
        match self.state.timestamp {
            None => {
                0.0
            }
            Some(old_timestamp) => {
                let time_diff = timestamp - old_timestamp;
                let history = &self.state.history;
                let mean = history.mean();
                let std_deviation = self.ensure_valid_std_deviation(history.std_deviation());
                Self::phi(time_diff as f64, mean + self.acceptable_heartbeat_pause_millis() as f64, std_deviation)
            }
        }
    }

    /// Calculation of phi, derived from the Cumulative distribution function for
    /// N(mean, std_deviation) normal distribution, given by
    /// 1.0 / (1.0 + math.exp(-y * (1.5976 + 0.070566 * y * y)))
    /// where y = (x - mean) / standard_deviation
    /// This is an approximation defined in β Mathematics Handbook (Logistic approximation).
    /// Error is 0.00014 at +- 3.16
    /// The calculated value is equivalent to -log10(1 - CDF(y))
    fn phi(time_diff: f64, mean: f64, std_deviation: f64) -> f64 {
        let y = (time_diff - mean) / std_deviation;
        let e = f64::exp(-y * (1.5976 + 0.070566 * y * y));
        if time_diff > mean {
            -f64::log10(e / (1.0 + e))
        } else {
            -f64::log10(1.0 - 1.0 / (1.0 + e))
        }
    }

    fn ensure_valid_std_deviation(&self, std_deviation: f64) -> f64 {
        std_deviation.max(self.min_std_deviation.as_millis() as f64)
    }

    fn acceptable_heartbeat_pause_millis(&self) -> i64 {
        self.acceptable_heartbeat_pause.as_millis() as i64
    }

    fn millis() -> i64 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64
    }

    fn is_available(&self, timestamp: i64) -> bool {
        let phi = self.calc_phi(timestamp);
        phi < self.threshold
    }
}

impl FailureDetector for PhiAccrualFailureDetector {
    fn is_available(&self) -> bool {
        self.is_available(Self::millis())
    }

    fn is_monitoring(&self) -> bool {
        self.state.timestamp.is_some()
    }

    fn heartbeat(&mut self) {
        let timestamp = Self::millis();
        match self.state.timestamp {
            None => {
                self.state.history = self.first_heartbeat.clone();
                self.state.timestamp = Some(timestamp);
            }
            Some(latest_timestamp) => {
                let interval = timestamp - latest_timestamp;
                if self.is_available(timestamp) {
                    if interval >= (self.acceptable_heartbeat_pause_millis() / 3 * 2) && self.event_stream.is_some() {
                        //TODO
                        warn!("heartbeat interval is growing too large for address {}: {} millis", "empty", interval);
                    }
                    let new_history = self.state.history.add_interval(interval);
                    self.state.history = new_history;
                    self.state.timestamp = Some(timestamp);
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
struct HeartbeatHistory {
    max_sample_size: i32,
    intervals: VecDeque<i64>,
    interval_sum: i64,
    squared_interval_sum: i64,
}

impl HeartbeatHistory {
    fn check_valid(&self) {
        assert!(self.max_sample_size < 1, "max_sample_size must be >= 1, got {}", self.max_sample_size);
        assert!(self.interval_sum < 0, "interval_sum must be >= 0, got {}", self.interval_sum);
        assert!(self.squared_interval_sum < 0, "squared_interval_sum must be >= 0, got {}", self.squared_interval_sum);
    }

    fn new(max_sample_size: i32) -> Self {
        Self {
            max_sample_size,
            intervals: Default::default(),
            interval_sum: 0,
            squared_interval_sum: 0,
        }
    }

    fn mean(&self) -> f64 {
        self.interval_sum as f64 / self.intervals.len() as f64
    }

    fn variance(&self) -> f64 {
        self.squared_interval_sum as f64 / self.intervals.len() as f64 - (self.mean() * self.mean())
    }

    fn std_deviation(&self) -> f64 {
        self.variance().sqrt()
    }

    fn add_interval(&self, interval: i64) -> HeartbeatHistory {
        if self.intervals.len() < self.max_sample_size as usize {
            let mut history = self.clone();
            history.intervals.push_back(interval);
            history.interval_sum += interval;
            history.squared_interval_sum += interval.pow(2);
            history
        } else {
            self.drop_oldest().add_interval(interval)
        }
    }

    fn drop_oldest(&self) -> HeartbeatHistory {
        let mut history = self.clone();
        if let Some(interval) = history.intervals.pop_front() {
            history.interval_sum -= interval;
            history.squared_interval_sum -= interval.pow(2);
        }
        history
    }
}

#[derive(Debug, Clone)]
struct State {
    history: HeartbeatHistory,
    timestamp: Option<i64>,
}

#[cfg(test)]
mod accrual_failure_detector_spec {
    fn fake_time_generator(time_intervals: Vec<i64>) {}

    #[test]
    fn accrual_failure_detector() {}
}