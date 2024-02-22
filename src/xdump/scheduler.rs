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
        xlog!('I', "Scheduler started");
        let mut next_start = Self::next_occurrence(self.start_time, &Local::now());
        let mut next_end = Self::next_occurrence(self.end_time, &Local::now());

        loop {
            let now = Local::now();

            xlog!(
                'D',
                format!(
                    "now: {:?}, next_start: {:?}, next_end: {:?}",
                    now, next_start, next_end
                )
            );

            if now >= next_start && now < next_end {
                xlog!('D', "Capture state changed - true");
                on_capture_state();
                next_start = Self::next_occurrence(self.start_time, &Local::now());
                next_end = Self::next_occurrence(self.end_time, &Local::now());
            }

            if now >= next_end {
                xlog!('D', "Capture state changed - false");
                off_capture_state();

                next_start =
                    Self::next_occurrence(self.start_time, &now) + chrono::Duration::days(1);
                next_end = Self::next_occurrence(self.end_time, &now) + chrono::Duration::days(1);
            }

            // Wait until the next second before checking again
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }

    fn next_occurrence(time: NaiveTime, now: &DateTime<Local>) -> DateTime<Local> {
        let today_date = now.date_naive();
        let mut target_datetime = Local
            .from_local_datetime(&today_date.and_time(time))
            .unwrap();

        target_datetime
    }
}
