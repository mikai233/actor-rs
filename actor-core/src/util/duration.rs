use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct ConfigDuration {
    days: Option<u64>,
    hours: Option<u64>,
    minutes: Option<u64>,
    seconds: Option<u64>,
    milliseconds: Option<u64>,
}

impl ConfigDuration {
    pub fn to_std_duration(&self) -> std::time::Duration {
        let days = self.days.unwrap_or(0);
        let hours = self.hours.unwrap_or(0);
        let minutes = self.minutes.unwrap_or(0);
        let seconds = self.seconds.unwrap_or(0);
        let milliseconds = self.milliseconds.unwrap_or(0);
        let duration = std::time::Duration::from_secs(
            days * 24 * 60 * 60 + hours * 60 * 60 + minutes * 60 + seconds,
        );
        duration + std::time::Duration::from_millis(milliseconds)
    }

    pub fn from_millis(millis: u64) -> Self {
        Self {
            days: None,
            hours: None,
            minutes: None,
            seconds: None,
            milliseconds: Some(millis),
        }
    }

    pub fn from_secs(secs: u64) -> Self {
        Self {
            days: None,
            hours: None,
            minutes: None,
            seconds: Some(secs),
            milliseconds: None,
        }
    }

    pub fn from_mins(mins: u64) -> Self {
        Self {
            days: None,
            hours: None,
            minutes: Some(mins),
            seconds: None,
            milliseconds: None,
        }
    }

    pub fn from_hours(hours: u64) -> Self {
        Self {
            days: None,
            hours: Some(hours),
            minutes: None,
            seconds: None,
            milliseconds: None,
        }
    }

    pub fn from_days(days: u64) -> Self {
        Self {
            days: Some(days),
            hours: None,
            minutes: None,
            seconds: None,
            milliseconds: None,
        }
    }
}