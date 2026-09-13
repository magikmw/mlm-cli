//! Time-of-day and duration parsing/formatting (SPEC.md §3.1, §4.1,
//! §4.2, §6.1).
//!
//! Pure string-in/value-out logic: no database, no CLI, no calendar
//! dates. Every duration in this project is a plain `i64` minute count
//! (PLAN.md interface contract 11) and every duration shown to a user
//! goes through [`format_minutes`] — there is no second formatter.

// These are the wave-1 building blocks consumed by later milestones
// (4, 7, 8, 9, 10, 11); nothing in `main` calls them yet.
#![allow(dead_code)]

use chrono::NaiveTime;

/// Parses a `TIME` argument (§3.1): `HH:MM`, `HHMM`, or `HH` (minute
/// defaults to 0). Valid range is 00:00-23:59 inclusive; 24:00 is
/// rejected (§6.1), never treated as next-day midnight.
pub fn parse_time(input: &str) -> Result<NaiveTime, TimeParseError> {
    let invalid = || TimeParseError::InvalidFormat(input.to_string());

    let (hour, minute) = match input.split_once(':') {
        // `HH:MM` — 1-2 hour digits, exactly 2 minute digits.
        Some((hours, minutes)) => {
            if !is_digits(hours, 1, 2) || !is_digits(minutes, 2, 2) {
                return Err(invalid());
            }
            (parse_u32(hours)?, parse_u32(minutes)?)
        }
        // `HHMM` (exactly 4 digits) or `HH` (1-2 digits, minute 0).
        None => match input.len() {
            1 | 2 if is_digits(input, 1, 2) => (parse_u32(input)?, 0),
            4 if is_digits(input, 4, 4) => (parse_u32(&input[..2])?, parse_u32(&input[2..])?),
            _ => return Err(invalid()),
        },
    };

    NaiveTime::from_hms_opt(hour, minute, 0).ok_or(TimeParseError::OutOfRange {
        input: input.to_string(),
        hour,
        minute,
    })
}

/// True when `s` is between `min` and `max` ASCII digits, inclusive.
fn is_digits(s: &str, min: usize, max: usize) -> bool {
    (min..=max).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit())
}

/// Infallible in practice: callers check the digit shape first, and at
/// most 4 digits always fit in a `u32`.
fn parse_u32(s: &str) -> Result<u32, TimeParseError> {
    s.parse()
        .map_err(|_| TimeParseError::InvalidFormat(s.to_string()))
}

/// Parses a `DURATION` argument (§3.7/§4.2): `Hh`, `HhMMm`, or `MMm`,
/// unpadded. Converts straight to a minute count. Zero is legal,
/// negative is rejected (§6.1).
pub fn parse_duration(input: &str) -> Result<i64, DurationParseError> {
    let invalid = || DurationParseError::InvalidFormat(input.to_string());

    // A leading '-' is structurally part of the grammar so that a
    // well-shaped but negative value fails on its *sign* (§6.1) rather
    // than being lumped in with genuinely malformed input.
    let (negative, body) = match input.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, input),
    };

    let magnitude = if let Some(rest) = body.strip_suffix('m') {
        match rest.split_once('h') {
            // `HhMMm`
            Some((hours, minutes)) => checked_minutes(hours, minutes).ok_or_else(invalid)?,
            // `MMm` — the minute component is a magnitude, not a
            // wall-clock value, so it is deliberately not capped at 59.
            None => digits(rest).ok_or_else(invalid)?,
        }
    } else if let Some(rest) = body.strip_suffix('h') {
        checked_minutes(rest, "0").ok_or_else(invalid)?
    } else {
        return Err(invalid());
    };

    if negative && magnitude != 0 {
        return Err(DurationParseError::Negative(-magnitude));
    }
    Ok(magnitude)
}

/// `hours * 60 + minutes`, or `None` if either side isn't a plain run
/// of digits or the total overflows.
fn checked_minutes(hours: &str, minutes: &str) -> Option<i64> {
    digits(hours)?
        .checked_mul(60)?
        .checked_add(digits(minutes)?)
}

/// Parses a non-empty run of ASCII digits as an `i64`.
fn digits(s: &str) -> Option<i64> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

/// The one canonical duration display formatter (§4.2): always
/// `HHh MMm`, both sides zero-padded, hour part never dropped. A
/// negative value keeps the same padding with a single leading `-`.
pub fn format_minutes(minutes: i64) -> String {
    let sign = if minutes < 0 { "-" } else { "" };
    // `unsigned_abs` rather than `abs` so `i64::MIN` can't overflow.
    let magnitude = minutes.unsigned_abs();
    format!("{sign}{:02}h {:02}m", magnitude / 60, magnitude % 60)
}

/// Why a `TIME` argument was rejected (§6.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeParseError {
    /// Doesn't match the `HH:MM` / `HHMM` / `HH` shape at all.
    InvalidFormat(String),
    /// Matched a shape, but the hour and/or minute is out of range
    /// (including the explicit `24:00` boundary case).
    OutOfRange {
        input: String,
        hour: u32,
        minute: u32,
    },
    /// No `TIME` given where one was required: `--date` resolved to a
    /// day other than today, so "default to now" has no meaning
    /// (backdated-punches spec §3). This is a "missing input" error,
    /// not a parse failure -- it carries no offending string.
    Required,
}

/// Why a `DURATION` argument was rejected (§6.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DurationParseError {
    /// Doesn't match the `Hh` / `HhMMm` / `MMm` shape.
    InvalidFormat(String),
    /// Matched the shape, but the value is negative.
    Negative(i64),
}

impl std::fmt::Display for TimeParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFormat(input) => write!(
                f,
                "invalid time {input:?}: expected HH:MM, HHMM, or HH (24-hour)"
            ),
            Self::OutOfRange { input, .. } => write!(
                f,
                "time {input:?} is out of range: valid times are 00:00 through 23:59"
            ),
            Self::Required => write!(
                f,
                "TIME is required when --date targets a day other than today"
            ),
        }
    }
}

impl std::error::Error for TimeParseError {}

impl std::fmt::Display for DurationParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFormat(input) => write!(
                f,
                "invalid duration {input:?}: expected a form like 20h, 33h30m, or 45m"
            ),
            Self::Negative(minutes) => {
                write!(f, "duration cannot be negative (got {minutes} minutes)")
            }
        }
    }
}

impl std::error::Error for DurationParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(hour: u32, minute: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(hour, minute, 0).unwrap()
    }

    // --- TIME, valid (plan cases 1-10) ---

    #[test]
    fn parses_colon_form() {
        assert_eq!(parse_time("9:05").unwrap(), t(9, 5));
        assert_eq!(parse_time("17:30").unwrap(), t(17, 30));
    }

    #[test]
    fn parses_compact_four_digit_form() {
        assert_eq!(parse_time("0905").unwrap(), t(9, 5));
        assert_eq!(parse_time("1730").unwrap(), t(17, 30));
    }

    #[test]
    fn parses_hour_only_form_with_minute_defaulting_to_zero() {
        assert_eq!(parse_time("9").unwrap(), t(9, 0));
        assert_eq!(parse_time("17").unwrap(), t(17, 0));
        assert_eq!(parse_time("0").unwrap(), t(0, 0));
    }

    #[test]
    fn parses_range_boundaries() {
        assert_eq!(parse_time("00:00").unwrap(), t(0, 0));
        assert_eq!(parse_time("23:59").unwrap(), t(23, 59));
        assert_eq!(parse_time("2359").unwrap(), t(23, 59));
    }

    // --- TIME, invalid (plan cases 11-22) ---

    #[test]
    fn rejects_out_of_range_hour() {
        assert_eq!(
            parse_time("25:00"),
            Err(TimeParseError::OutOfRange {
                input: "25:00".to_string(),
                hour: 25,
                minute: 0
            })
        );
    }

    #[test]
    fn rejects_twenty_four_hundred_rather_than_treating_it_as_next_midnight() {
        for input in ["24:00", "2400", "24"] {
            assert!(
                matches!(
                    parse_time(input),
                    Err(TimeParseError::OutOfRange { hour: 24, .. })
                ),
                "expected {input:?} to be out of range"
            );
        }
    }

    #[test]
    fn rejects_out_of_range_minute() {
        assert_eq!(
            parse_time("9:75"),
            Err(TimeParseError::OutOfRange {
                input: "9:75".to_string(),
                hour: 9,
                minute: 75
            })
        );
    }

    #[test]
    fn rejects_malformed_time_shapes() {
        for input in [
            "abc",     // non-digit
            "",        // empty
            "9:5",     // 1-digit minute in the colon form
            "905",     // 3 digits is not HHMM
            "-9:00",   // signed
            "9:00:00", // seconds
            "9: 00",   // embedded space
        ] {
            assert_eq!(
                parse_time(input),
                Err(TimeParseError::InvalidFormat(input.to_string())),
                "expected {input:?} to be an invalid format"
            );
        }
    }

    // --- DURATION, valid (plan cases 23-30) ---

    #[test]
    fn parses_hours_only_duration() {
        assert_eq!(parse_duration("20h").unwrap(), 1200);
        assert_eq!(parse_duration("0h").unwrap(), 0);
        assert_eq!(parse_duration("100h").unwrap(), 6000);
    }

    #[test]
    fn parses_minutes_only_duration() {
        assert_eq!(parse_duration("45m").unwrap(), 45);
        assert_eq!(parse_duration("0m").unwrap(), 0);
    }

    #[test]
    fn parses_combined_duration() {
        assert_eq!(parse_duration("33h30m").unwrap(), 2010);
        assert_eq!(parse_duration("1h0m").unwrap(), 60);
        assert_eq!(parse_duration("5h5m").unwrap(), 305);
    }

    // --- DURATION, invalid (plan cases 31-40) ---

    #[test]
    fn rejects_malformed_duration_shapes() {
        for input in [
            "10",     // no unit
            "10x",    // unknown unit
            "",       // empty
            "h",      // no digits
            "30m20h", // wrong order
            "20H",    // wrong case
            "20 h",   // embedded space
            "20h-5m", // sign is only legal as a leading token
        ] {
            assert_eq!(
                parse_duration(input),
                Err(DurationParseError::InvalidFormat(input.to_string())),
                "expected {input:?} to be an invalid format"
            );
        }
    }

    #[test]
    fn rejects_negative_duration_that_otherwise_matches_the_grammar() {
        assert_eq!(
            parse_duration("-5h"),
            Err(DurationParseError::Negative(-300))
        );
        assert_eq!(
            parse_duration("-45m"),
            Err(DurationParseError::Negative(-45))
        );
    }

    #[test]
    fn accepts_a_minute_component_of_sixty_or_more() {
        // DURATION is a magnitude, not a wall-clock offset, so the `m`
        // component is not capped at 59 (plan §1.3, flagged decision).
        assert_eq!(parse_duration("1h90m").unwrap(), 150);
        assert_eq!(parse_duration("90m").unwrap(), 90);
    }

    #[test]
    fn treats_negative_zero_as_plain_zero() {
        // Zero is legal (§6.1) and -0 is not a negative value.
        assert_eq!(parse_duration("-0h").unwrap(), 0);
        assert_eq!(parse_duration("-0m").unwrap(), 0);
    }

    // --- format_minutes (plan cases 41-47) ---

    #[test]
    fn formats_positive_durations_zero_padded() {
        assert_eq!(format_minutes(465), "07h 45m");
        assert_eq!(format_minutes(20), "00h 20m");
        assert_eq!(format_minutes(0), "00h 00m");
        assert_eq!(format_minutes(2400), "40h 00m");
    }

    #[test]
    fn formats_negative_durations_with_a_single_leading_sign() {
        assert_eq!(format_minutes(-50), "-00h 50m");
        assert_eq!(format_minutes(-200), "-03h 20m");
        assert_eq!(format_minutes(-1), "-00h 01m");
    }

    // --- round trip (plan case 48) ---

    #[test]
    fn input_and_display_grammars_agree_on_the_same_value() {
        assert_eq!(format_minutes(parse_duration("33h30m").unwrap()), "33h 30m");
    }

    // --- error surface (contract 7: Display + std::error::Error) ---

    #[test]
    fn errors_render_as_a_single_ascii_line() {
        let errors: Vec<Box<dyn std::error::Error>> = vec![
            Box::new(parse_time("abc").unwrap_err()),
            Box::new(parse_time("25:00").unwrap_err()),
            Box::new(parse_duration("10x").unwrap_err()),
            Box::new(parse_duration("-5h").unwrap_err()),
        ];
        for err in errors {
            let msg = err.to_string();
            assert!(!msg.is_empty());
            assert!(msg.is_ascii(), "message not ASCII: {msg:?}");
            assert!(!msg.contains('\n'), "message has a newline: {msg:?}");
        }
    }

    #[test]
    fn required_variant_renders_the_backdated_reason() {
        let err = TimeParseError::Required;
        let msg = err.to_string();
        assert_eq!(
            msg,
            "TIME is required when --date targets a day other than today"
        );
        assert!(msg.is_ascii());
        assert!(!msg.contains('\n'));
    }

    #[test]
    fn required_variant_is_distinguishable_from_the_other_variants() {
        assert_ne!(
            TimeParseError::Required,
            TimeParseError::InvalidFormat("x".to_string())
        );
        assert_ne!(
            TimeParseError::Required,
            TimeParseError::OutOfRange {
                input: "x".to_string(),
                hour: 1,
                minute: 1
            }
        );
    }
}
