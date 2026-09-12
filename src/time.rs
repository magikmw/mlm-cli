//! Time-of-day math: parsing hours and computing durations between points.

use chrono::{Duration, NaiveTime};

/// Parses a time string like "9:00", "09:00", or "17:30" into a `NaiveTime`.
pub fn parse_hm(s: &str) -> Result<NaiveTime, chrono::ParseError> {
    NaiveTime::parse_from_str(s, "%H:%M")
}

/// Duration between two points in a day. Assumes `stop` is after `start`
/// (no overnight wraparound handling yet).
pub fn between(start: NaiveTime, stop: NaiveTime) -> Duration {
    stop - start
}

/// Formats a `Duration` as "Hh Mm".
pub fn format_duration(d: Duration) -> String {
    let mins = d.num_minutes();
    format!("{}h {}m", mins / 60, mins % 60)
}
