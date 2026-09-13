//! Calendar dates (`DATE`) and ISO week ids (`WEEK_ID`): parsing,
//! validation, formatting and week iteration.
//!
//! Spec basis: SPEC.md §1.3 (a week is an ISO year + ISO week tuple),
//! §2.3 (`week_targets.week_id` storage form), §3.5 (`DATE` argument),
//! §3.6 (`WEEK_ID` argument forms), §6.1 (hard errors).

// Milestone 2 lands this module ahead of its consumers (Milestones 4, 6,
// 8, 9, 10, 11). Everything here is exercised by the unit tests below;
// the `main.rs` call sites arrive with those later milestones.
#![allow(dead_code)]

use chrono::{Datelike, Days, NaiveDate, Weekday};

// ---------------------------------------------------------------------------
// Errors (PLAN.md contract 7, local to this milestone)
// ---------------------------------------------------------------------------

/// Which of this milestone's two argument kinds failed to parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgKind {
    Date,
    WeekId,
}

/// Machine-inspectable cause. Tests assert on this, not on message text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cause {
    /// Did not match the argument's grammar at all.
    Shape,
    /// Matched the grammar but a component is outside its legal range
    /// (week `0`, or a year outside `1000..=9999`).
    OutOfRange,
    /// Well-shaped `YYYY-MM-DD` that is not a real calendar date.
    NoSuchCalendarDate,
    /// Well-shaped `YYYY-WW` (or a bare `WW` against a defaulted year)
    /// whose week does not exist in that ISO year. Carries both the year
    /// the week was checked against and that year's real week count, so
    /// the message can name a year the input itself may not contain (a
    /// bare `WW`'s year comes from `today`).
    NoSuchIsoWeek { iso_year: i32, weeks_in_year: u32 },
    /// A resolved `DATE` is later than today's local calendar date.
    /// Only ever paired with `ArgKind::Date` (backdated-punches spec §3).
    Future,
}

/// A §6.1 hard error originating at the input edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateWeekError {
    pub arg: ArgKind,
    /// The user's input, verbatim and untruncated, for the stderr message.
    pub input: String,
    pub cause: Cause,
}

impl DateWeekError {
    pub fn new(arg: ArgKind, input: &str, cause: Cause) -> Self {
        Self {
            arg,
            input: input.to_string(),
            cause,
        }
    }
}

impl std::fmt::Display for DateWeekError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self.arg {
            ArgKind::Date => "DATE",
            ArgKind::WeekId => "WEEK_ID",
        };
        write!(f, "invalid {} \"{}\": ", name, self.input)?;
        match (&self.arg, &self.cause) {
            (ArgKind::Date, Cause::Shape) => write!(f, "expected YYYY-MM-DD or -N (N >= 1)"),
            (ArgKind::WeekId, Cause::Shape) => write!(f, "expected YYYY-WW or WW"),
            (ArgKind::Date, Cause::OutOfRange) => write!(f, "date offset is out of range"),
            (_, Cause::OutOfRange) => write!(f, "week number must be 1 or greater"),
            (_, Cause::NoSuchCalendarDate) => write!(f, "not a real calendar date"),
            (
                _,
                Cause::NoSuchIsoWeek {
                    iso_year,
                    weeks_in_year,
                },
            ) => write!(f, "{iso_year} has only {weeks_in_year} ISO weeks"),
            (_, Cause::Future) => write!(f, "date is in the future"),
        }
    }
}

impl std::error::Error for DateWeekError {}

// ---------------------------------------------------------------------------
// DATE
// ---------------------------------------------------------------------------

/// Is `s` exactly `dddd-dd-dd` in ASCII digits?
fn has_ymd_shape(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

/// Parse a `DATE` argument (§3.5): strictly zero-padded `YYYY-MM-DD`.
pub fn parse_date(s: &str) -> Result<NaiveDate, DateWeekError> {
    let err = |cause| DateWeekError::new(ArgKind::Date, s, cause);
    if !has_ymd_shape(s) {
        return Err(err(Cause::Shape));
    }
    let year: i32 = s[0..4].parse().map_err(|_| err(Cause::Shape))?;
    let month: u32 = s[5..7].parse().map_err(|_| err(Cause::Shape))?;
    let day: u32 = s[8..10].parse().map_err(|_| err(Cause::Shape))?;
    NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| err(Cause::NoSuchCalendarDate))
}

/// Core `DATE` resolver (backdated-punches spec §2, §4): accepts the
/// existing absolute `YYYY-MM-DD` grammar, or a relative `-N` shorthand
/// (`-` + one or more ASCII digits, N >= 1, leading zeros tolerated)
/// meaning N days before `today`. No future-date opinion — `status`
/// calls this directly; `start`/`stop`/`note` go through
/// `resolve_future_checked_date` instead, which adds that check.
pub fn resolve_date(s: &str, today: NaiveDate) -> Result<NaiveDate, DateWeekError> {
    let err = |cause| DateWeekError::new(ArgKind::Date, s, cause);
    if let Some(digits) = s.strip_prefix('-') {
        if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
            let n: u64 = match digits.parse() {
                Ok(n) => n,
                // The digit run itself doesn't fit in a u64 -- as
                // unrepresentable as any other offset chrono can't
                // handle, so the same Cause applies.
                Err(_) => return Err(err(Cause::OutOfRange)),
            };
            if n == 0 {
                return Err(err(Cause::Shape));
            }
            let resolved = today
                .checked_sub_days(Days::new(n))
                .ok_or_else(|| err(Cause::OutOfRange))?;
            // `parse_date` only ever accepts a 4-digit year (`1000..=9999`
            // via `has_ymd_shape`'s exactly-10-bytes check), so the two
            // spellings of DATE must agree on that same domain -- a `-N`
            // that resolves outside it would write a value `parse_date`
            // (and therefore the storage read-back path) can never parse
            // again.
            if !(MIN_YEAR..=MAX_YEAR).contains(&resolved.year()) {
                return Err(err(Cause::OutOfRange));
            }
            return Ok(resolved);
        }
    }
    parse_date(s)
}

/// Thin wrapper around [`resolve_date`] adding the future-date rejection
/// `start`/`stop`/`note` need (backdated-punches spec §3). `status` uses
/// `resolve_date` directly and keeps accepting future dates.
pub fn resolve_future_checked_date(s: &str, today: NaiveDate) -> Result<NaiveDate, DateWeekError> {
    let date = resolve_date(s, today)?;
    if date > today {
        return Err(DateWeekError::new(ArgKind::Date, s, Cause::Future));
    }
    Ok(date)
}

/// Canonical storage/display form: `YYYY-MM-DD`.
pub fn format_date(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

/// §7.1's status header form: `Thu 2026-02-12`.
pub fn format_date_with_weekday(d: NaiveDate) -> String {
    format!("{} {}", weekday_abbrev(d.weekday()), format_date(d))
}

/// `Thursday` — §7.1/§7.2's "left by end of <weekday>" headline word.
pub fn format_weekday_full(d: NaiveDate) -> String {
    match d.weekday() {
        Weekday::Mon => "Monday",
        Weekday::Tue => "Tuesday",
        Weekday::Wed => "Wednesday",
        Weekday::Thu => "Thursday",
        Weekday::Fri => "Friday",
        Weekday::Sat => "Saturday",
        Weekday::Sun => "Sunday",
    }
    .to_string()
}

fn weekday_abbrev(w: Weekday) -> &'static str {
    match w {
        Weekday::Mon => "Mon",
        Weekday::Tue => "Tue",
        Weekday::Wed => "Wed",
        Weekday::Thu => "Thu",
        Weekday::Fri => "Fri",
        Weekday::Sat => "Sat",
        Weekday::Sun => "Sun",
    }
}

// ---------------------------------------------------------------------------
// WeekId
// ---------------------------------------------------------------------------

const MIN_YEAR: i32 = 1000;
const MAX_YEAR: i32 = 9999;

/// Number of ISO weeks in an ISO year: always 52 or 53 (§6.1).
///
/// Derived from the ISO invariant that December 28 always falls in the
/// last ISO week of its own year.
pub fn iso_weeks_in_year(iso_year: i32) -> u32 {
    NaiveDate::from_ymd_opt(iso_year, 12, 28)
        .expect("December 28 exists in every year")
        .iso_week()
        .week()
}

/// An ISO-8601 week: (ISO year, week number). Always Monday-start (§1.3).
///
/// Invariant: `week` is a real ISO week for `iso_year` and
/// `1000 <= iso_year <= 9999`. Fields are private so it cannot be bypassed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WeekId {
    iso_year: i32,
    week: u8,
}

impl WeekId {
    /// Validated constructor. `input` only populates `DateWeekError::input`.
    pub fn new(iso_year: i32, week: u32, input: &str) -> Result<Self, DateWeekError> {
        let err = |cause| DateWeekError::new(ArgKind::WeekId, input, cause);
        if week == 0 || !(MIN_YEAR..=MAX_YEAR).contains(&iso_year) {
            return Err(err(Cause::OutOfRange));
        }
        let weeks_in_year = iso_weeks_in_year(iso_year);
        if week > weeks_in_year {
            return Err(err(Cause::NoSuchIsoWeek {
                iso_year,
                weeks_in_year,
            }));
        }
        Ok(Self {
            iso_year,
            week: week as u8,
        })
    }

    /// Infallible: every real date belongs to exactly one ISO week.
    pub fn from_date(date: NaiveDate) -> Self {
        let iso = date.iso_week();
        Self {
            iso_year: iso.year(),
            week: iso.week() as u8,
        }
    }

    /// The ISO week containing `today`.
    pub fn current(today: NaiveDate) -> Self {
        Self::from_date(today)
    }

    pub fn iso_year(self) -> i32 {
        self.iso_year
    }

    pub fn week(self) -> u32 {
        u32::from(self.week)
    }

    /// Explicit name for the §2.3 storage form (== `to_string()`).
    pub fn to_key(self) -> String {
        self.to_string()
    }

    /// Strict inverse of `to_key`: only zero-padded `YYYY-WW` is accepted.
    pub fn from_key(s: &str) -> Result<Self, DateWeekError> {
        let b = s.as_bytes();
        let shaped = b.len() == 7
            && b[4] == b'-'
            && b.iter()
                .enumerate()
                .all(|(i, c)| i == 4 || c.is_ascii_digit());
        if !shaped {
            return Err(DateWeekError::new(ArgKind::WeekId, s, Cause::Shape));
        }
        let year: i32 = s[0..4]
            .parse()
            .map_err(|_| DateWeekError::new(ArgKind::WeekId, s, Cause::Shape))?;
        let week: u32 = s[5..7]
            .parse()
            .map_err(|_| DateWeekError::new(ArgKind::WeekId, s, Cause::Shape))?;
        Self::new(year, week, s)
    }

    /// Monday of this week.
    pub fn start(self) -> NaiveDate {
        NaiveDate::from_isoywd_opt(self.iso_year, self.week(), Weekday::Mon)
            .expect("WeekId's invariant guarantees the ISO week exists")
    }

    /// Sunday of this week.
    pub fn end(self) -> NaiveDate {
        self.start() + Days::new(6)
    }

    /// `(Monday, Sunday)` — §7.2's header span.
    pub fn span(self) -> (NaiveDate, NaiveDate) {
        (self.start(), self.end())
    }

    /// All seven dates, Monday first (§7.2 mandates exactly 7 rows).
    pub fn dates(self) -> [NaiveDate; 7] {
        let start = self.start();
        std::array::from_fn(|i| start + Days::new(i as u64))
    }

    /// Does this week contain `date`?
    pub fn contains(self, date: NaiveDate) -> bool {
        Self::from_date(date) == self
    }

    /// Next ISO week, rolling 52↔53 correctly.
    pub fn next(self) -> Self {
        if self.week() < iso_weeks_in_year(self.iso_year) {
            Self {
                iso_year: self.iso_year,
                week: self.week + 1,
            }
        } else {
            let iso_year = self.iso_year + 1;
            assert!(iso_year <= MAX_YEAR, "WeekId year overflow past {MAX_YEAR}");
            Self { iso_year, week: 1 }
        }
    }

    /// Previous ISO week, rolling 52↔53 correctly.
    pub fn prev(self) -> Self {
        if self.week > 1 {
            Self {
                iso_year: self.iso_year,
                week: self.week - 1,
            }
        } else {
            let iso_year = self.iso_year - 1;
            assert!(
                iso_year >= MIN_YEAR,
                "WeekId year underflow past {MIN_YEAR}"
            );
            Self {
                iso_year,
                week: iso_weeks_in_year(iso_year) as u8,
            }
        }
    }
}

impl std::fmt::Display for WeekId {
    /// `YYYY-WW`, week zero-padded: `2026-07`, `2026-53`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04}-{:02}", self.iso_year, self.week)
    }
}

/// Parse a `WEEK_ID` argument (§3.6): full `YYYY-WW` or a bare `WW` whose
/// year defaults to `today`'s **ISO** year. Unpadded weeks are normalized.
pub fn parse_week_id(s: &str, today: NaiveDate) -> Result<WeekId, DateWeekError> {
    let shape_err = || DateWeekError::new(ArgKind::WeekId, s, Cause::Shape);
    let (year, week_str) = match s.split_once('-') {
        Some((y, w)) => {
            if y.len() != 4 || !y.bytes().all(|c| c.is_ascii_digit()) {
                return Err(shape_err());
            }
            (y.parse::<i32>().map_err(|_| shape_err())?, w)
        }
        None => (today.iso_week().year(), s),
    };
    if week_str.is_empty() || week_str.len() > 2 || !week_str.bytes().all(|c| c.is_ascii_digit()) {
        return Err(shape_err());
    }
    let week: u32 = week_str.parse().map_err(|_| shape_err())?;
    WeekId::new(year, week, s)
}

// ---------------------------------------------------------------------------
// Week iteration
// ---------------------------------------------------------------------------

/// Inclusive iterator over consecutive ISO weeks. Empty if `from > to`.
#[derive(Debug, Clone)]
pub struct WeekRange {
    cursor: Option<WeekId>,
    end: WeekId,
}

impl Iterator for WeekRange {
    type Item = WeekId;

    fn next(&mut self) -> Option<WeekId> {
        let current = self.cursor?;
        self.cursor = if current == self.end {
            None
        } else {
            Some(current.next())
        };
        Some(current)
    }
}

/// Inclusive week sequence, `from..=to`. Empty when `from > to`.
pub fn week_range(from: WeekId, to: WeekId) -> WeekRange {
    WeekRange {
        cursor: (from <= to).then_some(from),
        end: to,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// A known date; panics on a typo in the test itself.
    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("test fixture is a real date")
    }

    /// A known-valid week; panics on a typo in the test itself.
    fn wk(y: i32, w: u32) -> WeekId {
        WeekId::new(y, w, "test").expect("test fixture is a real ISO week")
    }

    /// §7.1's worked example: a Thursday in ISO week 2026-07.
    const TODAY: fn() -> NaiveDate = || NaiveDate::from_ymd_opt(2026, 2, 12).unwrap();

    fn date_err(input: &str, cause: Cause) -> DateWeekError {
        DateWeekError::new(ArgKind::Date, input, cause)
    }

    fn week_err(input: &str, cause: Cause) -> DateWeekError {
        DateWeekError::new(ArgKind::WeekId, input, cause)
    }

    // --- 5.1 parse_date (T1-T21) -------------------------------------------

    #[test]
    fn parse_date_accepts_canonical_dates() {
        assert_eq!(parse_date("2026-02-12").unwrap(), d(2026, 2, 12)); // T1
        assert_eq!(parse_date("2026-01-01").unwrap(), d(2026, 1, 1)); // T2
        assert_eq!(parse_date("2026-12-31").unwrap(), d(2026, 12, 31)); // T3
        assert_eq!(parse_date("2024-02-29").unwrap(), d(2024, 2, 29)); // T4
    }

    #[test]
    fn parse_date_rejects_nonexistent_calendar_dates() {
        for input in [
            "2023-02-29",
            "2026-02-30",
            "2026-13-01",
            "2026-00-10",
            "2026-02-00",
        ] {
            // T5, T6, T8, T9, T10
            assert_eq!(
                parse_date(input).unwrap_err(),
                date_err(input, Cause::NoSuchCalendarDate),
                "input {input}"
            );
        }
    }

    #[test]
    fn parse_date_rejects_wrong_shapes() {
        for input in [
            "13/02/2026",        // T7
            "",                  // T11
            "abc",               // T12
            "2026-2-12",         // T13 (D4: strict zero-padding)
            "2026-02-2",         // T14
            "26-02-12",          // T15
            "2026-02-12T09:00",  // T16
            "2026-02-121",       // T17
            " 2026-02-12",       // T18
            "2026-02-12 ",       // T18
            "+2026-02-12",       // T19 (guards chrono's %Y leniency)
            "2026_02_12",        // T20
            "2026-02-1\u{ff12}", // T21 (non-ASCII digit)
        ] {
            assert_eq!(
                parse_date(input).unwrap_err(),
                date_err(input, Cause::Shape),
                "input {input:?}"
            );
        }
    }

    // --- 5.2 formatting (T22-T29) ------------------------------------------

    #[test]
    fn format_date_is_zero_padded_ymd() {
        assert_eq!(format_date(d(2026, 2, 12)), "2026-02-12"); // T22
        assert_eq!(format_date(d(2026, 1, 5)), "2026-01-05"); // T23
    }

    #[test]
    fn format_date_round_trips_through_parse_date() {
        // T24
        let mut date = d(2019, 1, 1);
        while date <= d(2032, 12, 31) {
            assert_eq!(parse_date(&format_date(date)).unwrap(), date);
            date = date + Days::new(11);
        }
    }

    #[test]
    fn format_date_with_weekday_matches_spec_examples() {
        assert_eq!(format_date_with_weekday(d(2026, 2, 12)), "Thu 2026-02-12"); // T25
        assert_eq!(format_date_with_weekday(d(2026, 1, 5)), "Mon 2026-01-05"); // T26
        assert_eq!(format_date_with_weekday(d(2026, 2, 9)), "Mon 2026-02-09"); // T27
    }

    #[test]
    fn format_weekday_full_gives_the_headline_word() {
        assert_eq!(format_weekday_full(d(2026, 2, 12)), "Thursday"); // T28
        assert_eq!(format_weekday_full(d(2026, 2, 9)), "Monday");
        assert_eq!(format_weekday_full(d(2026, 2, 15)), "Sunday");
    }

    #[test]
    fn weekday_rendering_is_ascii_and_three_chars() {
        // T29
        let expected = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        for (i, want) in expected.iter().enumerate() {
            let date = d(2026, 2, 9) + Days::new(i as u64);
            let rendered = format_date_with_weekday(date);
            assert!(rendered.is_ascii(), "{rendered} must be ASCII");
            assert_eq!(&rendered[..3], *want);
            assert!(format_weekday_full(date).is_ascii());
        }
    }

    // --- 5.3 iso_weeks_in_year (T30-T39) -----------------------------------

    #[test]
    fn iso_weeks_in_year_knows_the_53_week_years() {
        assert_eq!(iso_weeks_in_year(2026), 53); // T30
        assert_eq!(iso_weeks_in_year(2032), 53); // T31
        assert_eq!(iso_weeks_in_year(2020), 53); // T32 (leap, Jan 1 Wednesday)
    }

    #[test]
    fn iso_weeks_in_year_knows_the_52_week_years() {
        assert_eq!(iso_weeks_in_year(2027), 52); // T33 (the spec's counterexample)
        assert_eq!(iso_weeks_in_year(2021), 52); // T34
        assert_eq!(iso_weeks_in_year(2025), 52); // T35
        assert_eq!(iso_weeks_in_year(2019), 52); // T36
        assert_eq!(iso_weeks_in_year(2024), 52); // T37 (leap but not 53)
    }

    /// Independent derivation: an ISO year has 53 weeks iff its calendar
    /// year contains 53 Thursdays.
    fn thursdays_in_calendar_year(y: i32) -> u32 {
        let mut date = d(y, 1, 1);
        let mut count = 0;
        while date.year() == y {
            if date.weekday() == Weekday::Thu {
                count += 1;
            }
            date = date + Days::new(1);
        }
        count
    }

    #[test]
    fn iso_weeks_in_year_matches_an_independent_formula() {
        // T38, T39
        for y in 1900..=2100 {
            let got = iso_weeks_in_year(y);
            assert!(got == 52 || got == 53, "{y} -> {got}");
            assert_eq!(got, thursdays_in_calendar_year(y), "year {y}");

            let dec31 = d(y, 12, 31);
            let leap = NaiveDate::from_ymd_opt(y, 2, 29).is_some();
            let rule_53 =
                dec31.weekday() == Weekday::Thu || (leap && dec31.weekday() == Weekday::Fri);
            assert_eq!(got == 53, rule_53, "year {y}");
        }
    }

    // --- 5.4 parse_week_id (T40-T69) ---------------------------------------

    #[test]
    fn parse_week_id_accepts_all_four_equivalent_forms() {
        let today = TODAY();
        let full = parse_week_id("2026-07", today).unwrap(); // T40
        let unpadded = parse_week_id("2026-7", today).unwrap(); // T41
        let bare = parse_week_id("7", today).unwrap(); // T42
        let bare_padded = parse_week_id("07", today).unwrap(); // T43
        assert_eq!(full, wk(2026, 7));
        // T44
        assert!(full == unpadded && unpadded == bare && bare == bare_padded);
    }

    #[test]
    fn parse_week_id_accepts_real_weeks_including_53() {
        let today = TODAY();
        assert_eq!(parse_week_id("2026-01", today).unwrap(), wk(2026, 1)); // T45
        assert_eq!(parse_week_id("2026-53", today).unwrap(), wk(2026, 53)); // T46
        assert_eq!(parse_week_id("2020-53", today).unwrap(), wk(2020, 53)); // T48
    }

    #[test]
    fn parse_week_id_rejects_weeks_that_do_not_exist_in_their_year() {
        let today = TODAY();
        // T47 (the spec's own E3 example), T49, T50
        for (input, year, weeks) in [
            ("2027-53", 2027, 52),
            ("2021-53", 2021, 52),
            ("2026-54", 2026, 53),
        ] {
            assert_eq!(
                parse_week_id(input, today).unwrap_err(),
                week_err(
                    input,
                    Cause::NoSuchIsoWeek {
                        iso_year: year,
                        weeks_in_year: weeks
                    }
                ),
                "input {input}"
            );
        }
    }

    #[test]
    fn parse_week_id_rejects_week_zero_as_out_of_range() {
        let today = TODAY();
        for input in ["0", "2026-00"] {
            // T51, T52
            assert_eq!(
                parse_week_id(input, today).unwrap_err(),
                week_err(input, Cause::OutOfRange),
                "input {input}"
            );
        }
    }

    #[test]
    fn parse_week_id_rejects_wrong_shapes() {
        let today = TODAY();
        for input in [
            "abcd",     // T53
            "2026-W07", // T54 (§2.3: no `W`)
            "W07",      // T55
            "2026/07",  // T56
            "-5",       // T57
            "2026-",    // T58
            "-07",      // T59
            "2026-007", // T60
            "999-07",   // T61
            "20267",    // T62
            "",         // T63
            " 7 ",      // T64
            "+1",       // T65 (§1.2: no relative notation)
            "-1",       // T65
        ] {
            assert_eq!(
                parse_week_id(input, today).unwrap_err(),
                week_err(input, Cause::Shape),
                "input {input:?}"
            );
        }
    }

    #[test]
    fn bare_week_is_validated_against_the_defaulted_year() {
        // T66: 2027 has only 52 weeks.
        assert_eq!(
            parse_week_id("53", d(2027, 6, 15)).unwrap_err(),
            week_err(
                "53",
                Cause::NoSuchIsoWeek {
                    iso_year: 2027,
                    weeks_in_year: 52
                }
            )
        );
        // The message must name the *defaulted* year, which the input
        // itself does not contain.
        let msg = parse_week_id("53", d(2027, 6, 15)).unwrap_err().to_string();
        assert_eq!(msg, "invalid WEEK_ID \"53\": 2027 has only 52 ISO weeks");
    }

    #[test]
    fn bare_week_defaults_to_todays_iso_year_not_calendar_year() {
        // T67: 2025-12-30's calendar year is 2025 but its ISO year is 2026.
        assert_eq!(parse_week_id("7", d(2025, 12, 30)).unwrap(), wk(2026, 7));
        // T68: 2027-01-02's calendar year is 2027 but its ISO year is 2026.
        assert_eq!(parse_week_id("7", d(2027, 1, 2)).unwrap(), wk(2026, 7));
    }

    #[test]
    fn full_week_id_ignores_today() {
        // T69
        for today in [d(2020, 1, 1), d(2026, 2, 12), d(2030, 6, 1)] {
            assert_eq!(parse_week_id("2026-07", today).unwrap(), wk(2026, 7));
        }
    }

    // --- 5.5 from_date / contains (T70-T82) --------------------------------

    #[test]
    fn from_date_maps_dates_to_their_iso_week() {
        let cases = [
            ((2026, 2, 12), (2026, 7)),   // T70 (§7.1)
            ((2026, 1, 5), (2026, 2)),    // T71 (§7.1 second example)
            ((2026, 1, 1), (2026, 1)),    // T72
            ((2025, 12, 29), (2026, 1)),  // T73 Dec/Jan boundary
            ((2025, 12, 28), (2025, 52)), // T74
            ((2026, 1, 4), (2026, 1)),    // T75
            ((2026, 12, 28), (2026, 53)), // T76
            ((2027, 1, 3), (2026, 53)),   // T77
            ((2027, 1, 4), (2027, 1)),    // T78
            ((2021, 1, 3), (2020, 53)),   // T79
            ((2021, 1, 4), (2021, 1)),    // T80
        ];
        for ((y, m, day), (wy, w)) in cases {
            assert_eq!(
                WeekId::from_date(d(y, m, day)),
                wk(wy, w),
                "date {y}-{m}-{day}"
            );
        }
    }

    #[test]
    fn every_date_lies_inside_its_own_week() {
        // T81
        let mut date = d(2019, 1, 1);
        while date <= d(2032, 12, 31) {
            let week = WeekId::from_date(date);
            assert!(week.contains(date), "{date}");
            assert!(week.start() <= date && date <= week.end(), "{date}");
            date = date + Days::new(1);
        }
    }

    #[test]
    fn contains_is_exclusive_at_both_ends() {
        // T82
        let week = wk(2026, 1);
        assert!(week.contains(d(2025, 12, 29)));
        assert!(!week.contains(d(2025, 12, 28)));
        assert!(!week.contains(d(2026, 1, 5)));
    }

    // --- 5.6 start/end/span/dates (T83-T93) --------------------------------

    #[test]
    fn span_matches_the_spec_worked_examples() {
        let cases = [
            ((2026, 7), (2026, 2, 9), (2026, 2, 15)), // T83 (§7.2 header)
            ((2026, 2), (2026, 1, 5), (2026, 1, 11)), // T84
            ((2026, 6), (2026, 2, 2), (2026, 2, 8)),  // T85
            ((2026, 1), (2025, 12, 29), (2026, 1, 4)), // T86 Dec/Jan crossing
            ((2026, 53), (2026, 12, 28), (2027, 1, 3)), // T87
            ((2020, 53), (2020, 12, 28), (2021, 1, 3)), // T88
            ((2027, 1), (2027, 1, 4), (2027, 1, 10)), // T89
        ];
        for ((y, w), (sy, sm, sd), (ey, em, ed)) in cases {
            assert_eq!(wk(y, w).span(), (d(sy, sm, sd), d(ey, em, ed)), "{y}-{w}");
        }
    }

    #[test]
    fn dates_lists_seven_consecutive_days_monday_first() {
        // T90
        let got = wk(2026, 7).dates();
        let want: [NaiveDate; 7] = std::array::from_fn(|i| d(2026, 2, 9) + Days::new(i as u64));
        assert_eq!(got, want);
        // T91: crosses the year boundary mid-array.
        assert_eq!(wk(2026, 1).dates()[0], d(2025, 12, 29));
        assert_eq!(wk(2026, 1).dates()[6], d(2026, 1, 4));
    }

    #[test]
    fn dates_is_internally_consistent_for_every_week() {
        // T92, T93
        for week in week_range(wk(2019, 1), wk(2032, 52)) {
            let dates = week.dates();
            assert_eq!(dates[0].weekday(), Weekday::Mon);
            assert_eq!(dates[6].weekday(), Weekday::Sun);
            assert_eq!(dates[0], week.start());
            assert_eq!(dates[6], week.end());
            assert_eq!(week.end(), week.start() + Days::new(6));
            for (i, date) in dates.iter().enumerate() {
                assert_eq!(*date, dates[0] + Days::new(i as u64));
                assert_eq!(WeekId::from_date(*date), week);
            }
        }
    }

    // --- 5.7 Display / to_key / from_key (T94-T103) ------------------------

    #[test]
    fn display_is_the_zero_padded_storage_form() {
        assert_eq!(wk(2026, 7).to_string(), "2026-07"); // T94
        assert_eq!(wk(2026, 53).to_string(), "2026-53"); // T95
        assert_eq!(wk(2026, 7).to_key(), wk(2026, 7).to_string()); // T96
    }

    #[test]
    fn from_key_accepts_only_the_canonical_form() {
        assert_eq!(WeekId::from_key("2026-07").unwrap(), wk(2026, 7)); // T97
        for input in ["2026-7", "7", "2026-007", "abcdefg", ""] {
            // T98, T99
            assert_eq!(
                WeekId::from_key(input).unwrap_err(),
                week_err(input, Cause::Shape),
                "input {input:?}"
            );
        }
        // T100: validity is still enforced on read-back.
        assert_eq!(
            WeekId::from_key("2027-53").unwrap_err(),
            week_err(
                "2027-53",
                Cause::NoSuchIsoWeek {
                    iso_year: 2027,
                    weeks_in_year: 52
                }
            )
        );
    }

    #[test]
    fn keys_round_trip_through_both_parsers() {
        let sample = [wk(2026, 53), wk(2020, 53), wk(2026, 1), wk(2027, 1)];
        for week in sample {
            assert_eq!(WeekId::from_key(&week.to_key()).unwrap(), week); // T101
            // T102: the lenient parser also accepts the canonical form.
            assert_eq!(parse_week_id(&week.to_string(), TODAY()).unwrap(), week);
        }
    }

    #[test]
    fn key_ordering_is_lexicographic_and_chronological() {
        // T103
        let mut weeks = [
            wk(2027, 1),
            wk(2026, 1),
            wk(2026, 53),
            wk(2020, 53),
            wk(2026, 7),
            wk(2025, 52),
        ];
        weeks.sort();
        let mut keys: Vec<String> = weeks.iter().map(|w| w.to_key()).collect();
        let sorted_by_value = keys.clone();
        keys.sort();
        assert_eq!(keys, sorted_by_value);
    }

    // --- 5.8 next/prev/week_range/Ord (T104-T119) --------------------------

    #[test]
    fn next_rolls_over_at_the_years_real_week_count() {
        assert_eq!(wk(2026, 7).next(), wk(2026, 8)); // T104
        assert_eq!(wk(2026, 52).next(), wk(2026, 53)); // T105
        assert_eq!(wk(2026, 53).next(), wk(2027, 1)); // T106
        assert_eq!(wk(2027, 52).next(), wk(2028, 1)); // T107
    }

    #[test]
    fn prev_rolls_back_onto_53_week_years() {
        assert_eq!(wk(2027, 1).prev(), wk(2026, 53)); // T108
        assert_eq!(wk(2026, 1).prev(), wk(2025, 52)); // T109
        assert_eq!(wk(2021, 1).prev(), wk(2020, 53)); // T110
    }

    #[test]
    fn next_and_prev_are_inverses_and_agree_with_date_arithmetic() {
        // T111, T112
        for week in week_range(wk(2019, 1), wk(2032, 52)) {
            assert_eq!(week.next().prev(), week);
            assert_eq!(week.prev().next(), week);
            assert_eq!(week.next().start(), week.start() + Days::new(7));
        }
    }

    #[test]
    fn week_range_is_inclusive_and_crosses_rollovers() {
        // T113
        let got: Vec<WeekId> = week_range(wk(2026, 52), wk(2027, 2)).collect();
        assert_eq!(
            got,
            vec![wk(2026, 52), wk(2026, 53), wk(2027, 1), wk(2027, 2)]
        );
        // T114
        assert_eq!(
            week_range(wk(2026, 7), wk(2026, 7)).collect::<Vec<_>>(),
            vec![wk(2026, 7)]
        );
        // T115
        assert_eq!(
            week_range(wk(2027, 1), wk(2026, 53)).collect::<Vec<_>>(),
            Vec::<WeekId>::new()
        );
    }

    #[test]
    fn week_range_counts_whole_years() {
        // T116
        assert_eq!(week_range(wk(2026, 1), wk(2026, 53)).count(), 53);
        assert_eq!(week_range(wk(2027, 1), wk(2027, 52)).count(), 52);
    }

    #[test]
    fn week_range_emits_gapless_increasing_weeks() {
        // T117
        let weeks: Vec<WeekId> = week_range(wk(2019, 10), wk(2032, 5)).collect();
        assert!(weeks.len() > 600);
        for pair in weeks.windows(2) {
            assert!(pair[0] < pair[1]);
            assert_eq!(pair[0].next(), pair[1]);
            assert_eq!(pair[1].start(), pair[0].start() + Days::new(7));
        }
    }

    #[test]
    fn ord_is_chronological() {
        // T118
        assert!(wk(2026, 53) < wk(2027, 1));
        assert!(wk(2026, 7) < wk(2026, 8));
        assert!(wk(2025, 52) < wk(2026, 1));
        // T119
        let mut shuffled = vec![
            wk(2027, 1),
            wk(2020, 53),
            wk(2026, 53),
            wk(2026, 1),
            wk(2025, 52),
            wk(2026, 7),
        ];
        let mut by_start = shuffled.clone();
        shuffled.sort();
        by_start.sort_by_key(|w| w.start());
        assert_eq!(shuffled, by_start);
    }

    // --- 5.9 error shape (T120-T125) ---------------------------------------

    #[test]
    fn date_errors_carry_the_verbatim_input_and_arg_kind() {
        // T120
        for input in ["13/02/2026", "2026-02-30", "", "2026-2-12"] {
            let err = parse_date(input).unwrap_err();
            assert_eq!(err.arg, ArgKind::Date);
            assert_eq!(err.input, input);
        }
    }

    #[test]
    fn week_errors_carry_the_verbatim_input_and_arg_kind() {
        // T121
        for input in ["abcd", "2027-53", "0", "2026-W07"] {
            let err = parse_week_id(input, TODAY()).unwrap_err();
            assert_eq!(err.arg, ArgKind::WeekId);
            assert_eq!(err.input, input);
        }
        let err = WeekId::from_key("2026-7").unwrap_err();
        assert_eq!(err.arg, ArgKind::WeekId);
        assert_eq!(err.input, "2026-7");
    }

    #[test]
    fn no_such_iso_week_message_names_the_year_and_its_week_count() {
        // T122
        let msg = parse_week_id("2027-53", TODAY()).unwrap_err().to_string();
        assert!(msg.contains("2027"), "{msg}");
        assert!(msg.contains("52"), "{msg}");
        assert!(msg.contains("2027-53"), "{msg}");
        assert!(msg.is_ascii() && !msg.contains('\n'), "{msg}");
    }

    #[test]
    fn shape_message_is_a_single_ascii_line_quoting_the_input() {
        // T123
        let msg = parse_date("13/02/2026").unwrap_err().to_string();
        assert!(msg.contains("13/02/2026"), "{msg}");
        assert!(!msg.contains('\n'), "{msg}");
        assert!(msg.is_ascii(), "{msg}");
    }

    #[test]
    fn date_week_error_is_a_std_error() {
        // T124
        fn assert_err<E: std::error::Error>() {}
        assert_err::<DateWeekError>();
    }

    #[test]
    fn junk_input_never_panics() {
        // T125
        let junk = [
            "",
            " ",
            "\0",
            "-",
            "--",
            "999999999999",
            "2026-99999999999",
            "\u{1f642}",
            "2026-02-12\n",
            "\u{1d7da}\u{1d7d8}\u{1d7da}\u{1d7de}-\u{1d7d8}\u{1d7da}-\u{1d7d9}\u{1d7da}",
        ];
        for input in junk {
            assert!(parse_date(input).is_err(), "parse_date {input:?}");
            assert!(parse_week_id(input, TODAY()).is_err(), "week {input:?}");
            assert!(WeekId::from_key(input).is_err(), "from_key {input:?}");
        }
    }

    // --- resolve_date (backdated-punches spec §2, §5) ------------------

    #[test]
    fn resolve_date_passes_through_absolute_dates() {
        let today = d(2026, 2, 12);
        assert_eq!(resolve_date("2026-02-12", today).unwrap(), d(2026, 2, 12));
        assert_eq!(resolve_date("2026-01-05", today).unwrap(), d(2026, 1, 5));
    }

    #[test]
    fn resolve_date_accepts_relative_shorthand() {
        let today = d(2026, 2, 12);
        assert_eq!(resolve_date("-1", today).unwrap(), d(2026, 2, 11));
        assert_eq!(resolve_date("-7", today).unwrap(), d(2026, 2, 5));
        assert_eq!(resolve_date("-30", today).unwrap(), d(2026, 1, 13));
    }

    #[test]
    fn resolve_date_shorthand_crosses_a_leap_year_boundary() {
        // 2027-03-01 minus 1 day is 2027-02-28 (2027 is not a leap year).
        let today = d(2027, 3, 1);
        assert_eq!(resolve_date("-1", today).unwrap(), d(2027, 2, 28));

        // 2028-03-01 minus 1 day is 2028-02-29: 2028 actually is a leap
        // year (divisible by 4, not by 100), so this lands on a real
        // Feb 29 rather than merely crossing a month boundary in a
        // non-leap year like the case above.
        let today = d(2028, 3, 1);
        assert_eq!(resolve_date("-1", today).unwrap(), d(2028, 2, 29));
    }

    #[test]
    fn resolve_date_zero_padded_shorthand_matches_unpadded() {
        let today = d(2026, 2, 12);
        assert_eq!(
            resolve_date("-01", today).unwrap(),
            resolve_date("-1", today).unwrap()
        );
        assert_eq!(
            resolve_date("-007", today).unwrap(),
            resolve_date("-7", today).unwrap()
        );
    }

    #[test]
    fn resolve_date_rejects_minus_zero_as_shape() {
        let today = d(2026, 2, 12);
        assert_eq!(
            resolve_date("-0", today).unwrap_err(),
            date_err("-0", Cause::Shape)
        );
        assert_eq!(
            resolve_date("-00", today).unwrap_err(),
            date_err("-00", Cause::Shape)
        );
    }

    #[test]
    fn resolve_date_rejects_malformed_shorthand_as_shape() {
        let today = d(2026, 2, 12);
        for input in ["-1.5", "-abc", "+1", "- 1", "-1 ", "-1\n", "--1"] {
            assert_eq!(
                resolve_date(input, today).unwrap_err(),
                date_err(input, Cause::Shape),
                "input {input:?}"
            );
        }
    }

    #[test]
    fn resolve_date_absurdly_large_n_is_out_of_range_not_panic() {
        let today = d(2026, 2, 12);
        for input in [
            "-999999999999999999999999", // 24 digits: overflows the u64 parse itself
            "-99999999999999",           // 14 digits: parses as u64, overflows checked_sub_days
            "-18446744073709551615",     // u64::MAX exactly: parses as u64 (the largest
                                         // value that can), still overflows
                                         // checked_sub_days -- exercises the boundary
                                         // right at the parse/arithmetic seam rather
                                         // than deep in unrepresentable territory.
        ] {
            let err = resolve_date(input, today).unwrap_err();
            assert_eq!(err, date_err(input, Cause::OutOfRange), "input {input}");
        }
    }

    #[test]
    fn resolve_date_today_and_future_are_both_accepted_by_the_core_resolver() {
        let today = d(2026, 2, 12);
        assert_eq!(resolve_date("2026-02-12", today).unwrap(), today);
        assert_eq!(resolve_date("2026-02-13", today).unwrap(), d(2026, 2, 13));
        assert_eq!(resolve_date("2030-01-01", today).unwrap(), d(2030, 1, 1));
    }

    #[test]
    fn resolve_date_rejects_a_resolved_year_below_the_app_domain_floor() {
        // today - 374782 days = 0999-12-31: representable by chrono's
        // checked_sub_days (nowhere near overflow) but its year (999)
        // falls outside the app's 1000..=9999 domain, the same domain
        // parse_date's 4-digit year already enforces. Without the
        // fix this resolves "successfully" to a date parse_date can
        // never parse back -- exactly the data-corruption bug in the
        // review finding.
        let today = d(2026, 2, 12);
        assert_eq!(
            resolve_date("-374782", today).unwrap_err(),
            date_err("-374782", Cause::OutOfRange)
        );
        // And confirm the underlying chrono arithmetic really does land
        // on 0999-12-31, not something else -- pins the N used above.
        let raw = today.checked_sub_days(Days::new(374782)).unwrap();
        assert_eq!(raw, NaiveDate::from_ymd_opt(999, 12, 31).unwrap());
    }

    #[test]
    fn resolve_date_accepts_a_resolved_year_at_the_domain_floor() {
        // today - 374781 days = 1000-01-01: the boundary value that must
        // still succeed (regression guard for the inside edge of the
        // 1000..=9999 domain).
        let today = d(2026, 2, 12);
        assert_eq!(
            resolve_date("-374781", today).unwrap(),
            NaiveDate::from_ymd_opt(1000, 1, 1).unwrap()
        );
    }

    #[test]
    fn resolve_date_out_of_range_message_is_date_specific() {
        let today = d(2026, 2, 12);
        let msg = resolve_date("-99999999999999", today)
            .unwrap_err()
            .to_string();
        assert!(!msg.contains("week number"), "{msg}");
    }

    // --- resolve_future_checked_date ------------------------------------

    #[test]
    fn future_checked_rejects_future_absolute_date() {
        let today = d(2026, 2, 12);
        let err = resolve_future_checked_date("2026-02-13", today).unwrap_err();
        assert_eq!(err, date_err("2026-02-13", Cause::Future));
    }

    #[test]
    fn future_checked_shorthand_is_never_mistaken_for_future() {
        // A positive N-days-before-today shorthand can never itself
        // resolve to the future, so exercise this via an already-future
        // *absolute* date fed alongside a shorthand test of the boundary:
        // -N always resolves to today or earlier, so there is no -N
        // input that reaches the future branch -- this test instead
        // pins that -1/-N shorthand is always accepted (never mistakenly
        // rejected as "future").
        let today = d(2026, 2, 12);
        assert!(resolve_future_checked_date("-1", today).is_ok());
        assert!(resolve_future_checked_date("-1000", today).is_ok());
    }

    #[test]
    fn future_checked_accepts_today_and_past() {
        let today = d(2026, 2, 12);
        assert_eq!(
            resolve_future_checked_date("2026-02-12", today).unwrap(),
            today
        );
        assert_eq!(
            resolve_future_checked_date("2026-01-05", today).unwrap(),
            d(2026, 1, 5)
        );
        assert_eq!(
            resolve_future_checked_date("-1", today).unwrap(),
            d(2026, 2, 11)
        );
    }

    #[test]
    fn future_checked_still_propagates_shape_and_range_errors() {
        let today = d(2026, 2, 12);
        assert_eq!(
            resolve_future_checked_date("abc", today).unwrap_err(),
            date_err("abc", Cause::Shape)
        );
        assert_eq!(
            resolve_future_checked_date("-0", today).unwrap_err(),
            date_err("-0", Cause::Shape)
        );
    }

    #[test]
    fn future_error_message_says_future() {
        let today = d(2026, 2, 12);
        let msg = resolve_future_checked_date("2026-02-13", today)
            .unwrap_err()
            .to_string();
        assert!(msg.contains("future"), "{msg}");
    }
}
