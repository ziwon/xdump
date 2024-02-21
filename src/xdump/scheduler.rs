use chrono::{DateTime, Duration, Local, NaiveTime, TimeZone, Timelike};
use thiserror::Error;

use super::{
    config::Config,
    global::{off_capture_state, on_capture_state},
};
use xdump::xlog;

#[derive(Debug, Error)]
pub enum SchedulerError {
    #[error("Time parsing error: {0}")]
    TimeParsingError(String),

    #[error("Invalid time error: {0}")]
    InvalidTimeError(String),
}

pub struct Scheduler {
    /// start time to start capturing
    start_time: NaiveTime,

    /// end time to end capturing (24h format)
    end_time: NaiveTime,
}

impl Scheduler {
    pub fn new(config: &Config) -> Result<Self, SchedulerError> {
        let start_time = NaiveTime::parse_from_str(&config.start_time, "%H:%M")
            .map_err(|e| SchedulerError::TimeParsingError(e.to_string()))?;
        let end_time = NaiveTime::parse_from_str(&config.end_time, "%H:%M")
            .map_err(|e| SchedulerError::TimeParsingError(e.to_string()))?;

        if end_time.hour() <= 12 {
            return Err(SchedulerError::InvalidTimeError(
                "End time must be greater than 12 hours.".into(),
            ));
        }

        if start_time >= end_time {
            return Err(SchedulerError::InvalidTimeError(
                "Start time must be before end time.".into(),
            ));
        }

        Ok(Self {
            start_time,
            end_time,
        })
    }

    pub async fn run(&self) {
        xlog!('I', "started");

        let mut next_start = Self::next_occurrence(self.start_time);
        let mut next_end = Self::next_occurrence(self.end_time);

        loop {
            let now = Local::now();
            xlog!(
                'D',
                format!("now: {:?}, ns: {:?}, ne: {:?}", now, next_start, next_end)
            );

            if now >= next_start {
                xlog!('D', "Capture state changed - true");
                on_capture_state();
                next_start = Self::next_occurrence(self.start_time) + Duration::days(1);
            }
            if now >= next_end {
                xlog!('D', "Capture state changed - false");
                off_capture_state();
                next_end = Self::next_occurrence(self.end_time) + Duration::days(1);
            }

            // Waiting for the next capture
            let sleep_duration = std::cmp::min(
                next_start.signed_duration_since(now),
                next_end.signed_duration_since(now),
            );

            let sleep_duration = sleep_duration
                .to_std()
                .unwrap_or_else(|_| tokio::time::Duration::from_secs(1)); // Fallback to 1s if negative duration

            xlog!('D', format!("Waiting for next event: {:?}", sleep_duration));
            tokio::time::sleep(sleep_duration).await;
        }
    }

    fn next_occurrence(time: NaiveTime) -> DateTime<Local> {
        let now = Local::now();
        let today_date = now.date_naive();
        let target_datetime = Local
            .from_local_datetime(&today_date.and_time(time))
            .unwrap();

        target_datetime
    }
}
