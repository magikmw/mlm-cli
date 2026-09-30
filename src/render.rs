//! Shared rendering helpers for `status` (Milestone 10) and `week`
//! (Milestone 11): the §7.1/§7.2 week-owed headline and the §7.3
//! anomaly rendering. Pure functions only — no DB access, no CLI
//! wiring, no `chrono::Local::now()` (PLAN.md contract 6: "now" is
//! always an injected parameter).
//!
//! This module is the single place that decides wording, sign
//! handling, and "what counts as an anomaly" for both consumers, so
//! that they never grow their own diverging copies (see
//! plans/milestone-9-shared-rendering.md).

// Milestone 9 is built ahead of its consumers (Milestones 10/11), so
// the public surface here has no in-crate caller yet outside the tests.
#![allow(dead_code)]

use chrono::{NaiveDate, NaiveTime};

use crate::date::{WeekId, format_date, format_date_with_weekday, format_weekday_full};
use crate::time::format_minutes;

/// Width of the left-hand label column shared by §7.1's lead lines and
/// §7.2's trailing block. Labels are left-aligned and padded to this
/// width; the value starts at column 16 (1-indexed).
pub const LABEL_WIDTH: usize = 15;

/// Which framing §7.1/§7.2's headline uses for a week.
///
/// NOTES.md decisions 18/25/26: the deadline framing exists only for
/// the week that "today" is actually inside; every other week reports
/// a plain final total.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeekFraming {
    /// `today` falls inside this week.
    Current,
    /// This week is already over, or has not started yet.
    Closed,
}

/// True iff `today` falls inside the ISO week identified by `week`.
///
/// Compares (ISO year, ISO week) tuples — never calendar year, never a
/// date-range containment check against a Mon-Sun span, both of which
/// go wrong across a Dec/Jan boundary (NOTES.md decision 2).
pub fn week_framing(week: WeekId, today: NaiveDate) -> WeekFraming {
    let today_week = WeekId::from_date(today);
    if week.iso_year() == today_week.iso_year() && week.week() == today_week.week() {
        WeekFraming::Current
    } else {
        WeekFraming::Closed
    }
}

/// The §7.1/§7.2 headline for a week, in its bare form (no `Week NN:`
/// label, no fulfillment/target parenthetical).
///
/// `owed_minutes` is Milestone 6's signed `owed` for that week.
/// `today` is the injected "now" date (PLAN.md contract 6) — the
/// weekday in the deadline phrase is always *today's*, never that of
/// whatever date the caller happens to be displaying (§7.1, F11).
pub fn week_headline(week: WeekId, owed_minutes: i64, today: NaiveDate) -> String {
    match week_framing(week, today) {
        WeekFraming::Current if owed_minutes >= 0 => format!(
            "{} left by end of {}",
            format_minutes(owed_minutes),
            format_weekday_full(today)
        ),
        WeekFraming::Current => {
            format!("{} ahead of target", format_minutes(-owed_minutes))
        }
        WeekFraming::Closed if owed_minutes > 0 => {
            format!("Total behind: {}", format_minutes(owed_minutes))
        }
        WeekFraming::Closed => format!("Total ahead: {}", format_minutes(-owed_minutes)),
    }
}

/// The complete §7.1 week line, label and all, e.g.
/// `Week 2026-07:  10h 45m left by end of Thursday (fulfillment 29h 15m / target 40h 00m)`
pub fn status_week_line(
    week: WeekId,
    owed_minutes: i64,
    fulfillment_minutes: i64,
    target_minutes: i64,
    carry_in_minutes: i64,
    today: NaiveDate,
) -> String {
    let label = format!("Week {}:", week);
    let headline = week_headline(week, owed_minutes, today);
    let parenthetical = match week_framing(week, today) {
        WeekFraming::Current => {
            let worked_minutes = fulfillment_minutes - carry_in_minutes;
            if carry_in_minutes == 0 {
                format!(
                    " (fulfillment {} / target {})",
                    format_minutes(fulfillment_minutes),
                    format_minutes(target_minutes)
                )
            } else {
                format!(
                    " (fulfillment {} = worked {} + carry-in {} / target {})",
                    format_minutes(fulfillment_minutes),
                    format_minutes(worked_minutes),
                    format_minutes(carry_in_minutes),
                    format_minutes(target_minutes)
                )
            }
        }
        WeekFraming::Closed => String::new(),
    };
    format!(
        "{:<w$}{}{}",
        label,
        headline,
        parenthetical,
        w = LABEL_WIDTH
    )
}

/// Everything one calendar date's `status`/`week` anomaly rendering
/// needs, as plain data (SPEC.md §4.3/§4.3.1/§7.3). Callers compute
/// every fact; this type never reads
/// a clock, a database, or `stint.rs` itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateAnomalies {
    /// The date this block belongs to. Fills every `D` in this date's
    /// own `fix:`/`or:` lines (§7.3).
    pub date: NaiveDate,

    /// How many trailing unmatched starts this date has. 0 or 1 is
    /// normal; >= 2 is E7.
    pub open_stint_count: usize,

    /// The most recently opened start's local time, when
    /// `open_stint_count >= 1`. Only consumed when `open_stint_count
    /// > 1` (E7's fix line names it, e.g. "14:00" in the spec's worked
    /// example: "the latest open start on the date"). `None` iff
    /// `open_stint_count == 0`.
    pub latest_open_start: Option<NaiveTime>,

    /// One entry per orphaned end (E8), ascending by instant. Never
    /// deduplicated, never coalesced.
    pub orphaned_end_times: Vec<NaiveTime>,

    /// Set iff this date has exactly one open start AND it is stale
    /// (age >= 2). Never set when
    /// `open_stint_count != 1` (E7 owns the 2+ case entirely, §4.3).
    pub stale_open_start: Option<StaleOpenStart>,

    /// Set to the previous calendar date iff this date and its
    /// predecessor form a refused join with THIS date as the later
    /// (orphaned-end) side (§4.3.1).
    pub refused_join_with_previous: Option<NaiveDate>,

    /// Set to the next calendar date iff this date and its successor
    /// form a refused join with THIS date as the earlier
    /// (open-start) side (§4.3.1).
    pub refused_join_with_next: Option<NaiveDate>,
}

/// The one stale-open-start fact a date can carry (§4.3/§7.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaleOpenStart {
    /// Local time of the (only) open start.
    pub time: NaiveTime,
    /// `today - date` in whole days. Always >= 2 -- callers never
    /// construct this with a smaller value.
    pub age_days: i64,
}

impl DateAnomalies {
    /// True iff this date has ANY anomaly under the new, wider
    /// definition: E7, E8, stale, or either refused-join direction.
    /// The single definition `week`'s per-row `[!]` and `status`'s
    /// block-or-not decision both key off.
    pub fn has_any(&self) -> bool {
        self.open_stint_count > 1
            || !self.orphaned_end_times.is_empty()
            || self.stale_open_start.is_some()
            || self.refused_join_with_previous.is_some()
            || self.refused_join_with_next.is_some()
    }

    /// `status`'s per-date block (§7.3), in the fixed order: E7, then
    /// stale, then E8 (ascending instant), then refused-join-with-
    /// previous, then refused-join-with-next. Empty vec when
    /// `has_any()` is false. Does NOT include lookback lines — those
    /// come from separate `LookbackFinding`s, appended by the caller
    /// never by this type.
    pub fn detail_lines(&self) -> Vec<String> {
        let mut out = Vec::new();
        out.extend(self.e7_lines());
        out.extend(self.stale_lines());
        out.extend(self.e8_lines());
        out.extend(self.refused_join_lines());
        out
    }

    fn e7_lines(&self) -> Vec<String> {
        let Some(latest) = self.latest_open_start.filter(|_| self.open_stint_count > 1) else {
            return Vec::new();
        };
        let d = format_date(self.date);
        let latest = latest.format("%H:%M");
        vec![
            format!(
                "[!] {} open stints for this date (only one can be open at a time)",
                self.open_stint_count
            ),
            format!(
                "    fix: mlm delete punch --date {d}  (lists punches; rerun with the extra start's number)"
            ),
            format!(
                "     or: mlm stop HH:MM --date {d}  (a time after {latest} closes the {latest} start)"
            ),
        ]
    }

    fn stale_lines(&self) -> Vec<String> {
        let Some(stale) = self.stale_open_start else {
            return Vec::new();
        };
        debug_assert!(stale.age_days >= 2, "stale open start must be age >= 2");
        let d = format_date(self.date);
        let time = stale.time.format("%H:%M");
        vec![
            format!(
                "[!] unclosed start at {time}, {} days old (no stop recorded)",
                stale.age_days
            ),
            format!("    fix: mlm stop HH:MM --date {d}  (a time after {time})"),
            format!(
                "     or: mlm delete punch --date {d}  (lists punches; rerun with the start's number)"
            ),
        ]
    }

    fn e8_lines(&self) -> Vec<String> {
        let d = format_date(self.date);
        let mut out = Vec::new();
        for t in &self.orphaned_end_times {
            let time = t.format("%H:%M");
            out.push(format!("[!] orphaned end at {time} (no matching start)"));
            out.push(format!(
                "    fix: mlm start HH:MM --date {d}  (a time before {time})"
            ));
            out.push(format!(
                "     or: mlm delete punch --date {d}  (lists punches; rerun with the end's number)"
            ));
        }
        out
    }

    fn refused_join_lines(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(prev) = self.refused_join_with_previous {
            out.push(refused_join_line(prev, prev, self.date));
        }
        if let Some(next) = self.refused_join_with_next {
            out.push(refused_join_line(next, self.date, next));
        }
        out
    }

    /// `week`'s per-row suffix (§7.2's "row gets `[!]`" rule):
    /// `"  [!]"` when `has_any()`, `""` otherwise.
    pub fn row_marker(&self) -> &'static str {
        if self.has_any() { "  [!]" } else { "" }
    }
}

fn refused_join_line(named: NaiveDate, earlier: NaiveDate, later: NaiveDate) -> String {
    format!(
        "[!] not joined to {} across midnight (a join needs exactly one open start on {} and exactly one unmatched end, as {}'s first punch)",
        format_date_with_weekday(named),
        format_date_with_weekday(earlier),
        format_date_with_weekday(later),
    )
}

/// One window date's finding from the §7.1/§7.4 14-day lookback.
/// Deliberately narrower than `DateAnomalies`: a lookback line never
/// carries E8/refused-join detail (§1.2's residual-gap bullets), only
/// "this window date has an open start" in one of two shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LookbackFinding {
    /// The window date (age 2-14) this finding is about.
    pub date: NaiveDate,
    /// `today - date`. Always in 2..=14.
    pub age_days: i64,
    pub kind: LookbackKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookbackKind {
    /// Exactly one open start on the window date. `punch_id` is that
    /// start's row id, so `commands::start` can tell whether it's the
    /// punch it just wrote (§7.4).
    SingleOpenStart { time: NaiveTime, punch_id: i64 },
    /// Two or more open starts on the window date (that date's own E7).
    MultipleOpenStints { count: usize },
}

impl LookbackFinding {
    /// `status`'s block form (§7.1, one finding per line-group,
    /// appended after a date's own `DateAnomalies::detail_lines()`,
    /// oldest window date first).
    pub fn detail_lines(&self) -> Vec<String> {
        debug_assert!(
            (2..=14).contains(&self.age_days),
            "lookback age must be 2..=14"
        );
        let d = format_date(self.date);
        let wdy = format_date_with_weekday(self.date);
        match self.kind {
            LookbackKind::SingleOpenStart { time, .. } => {
                let time = time.format("%H:%M");
                vec![
                    format!(
                        "[!] unclosed start on {wdy} at {time} ({} days ago)",
                        self.age_days
                    ),
                    format!("    fix: mlm stop HH:MM --date {d}  (a time after {time})"),
                    format!(
                        "     or: mlm delete punch --date {d}  (lists punches; rerun with the start's number)"
                    ),
                ]
            }
            LookbackKind::MultipleOpenStints { count } => vec![
                format!(
                    "[!] {count} open stints on {wdy} ({} days ago)",
                    self.age_days
                ),
                format!("    fix: see mlm status {d}"),
            ],
        }
    }

    /// `start`'s stderr form (§7.4). A single open start prints three
    /// lines laid out like `detail_lines()`, `warning: ` in place of
    /// `[!] `; two or more open starts stay one line. No trailing
    /// newline either way.
    pub fn stderr_line(&self) -> String {
        debug_assert!(
            (2..=14).contains(&self.age_days),
            "lookback age must be 2..=14"
        );
        let d = format_date(self.date);
        let wdy = format_date_with_weekday(self.date);
        match self.kind {
            LookbackKind::SingleOpenStart { time, .. } => {
                let time = time.format("%H:%M");
                format!(
                    "warning: unclosed start on {wdy} at {time} ({} days ago)\n    fix: mlm stop HH:MM --date {d}  (a time after {time})\n     or: mlm delete punch --date {d}  (lists punches; rerun with the start's number)",
                    self.age_days
                )
            }
            LookbackKind::MultipleOpenStints { count } => format!(
                "warning: {count} open stints on {wdy} ({} days ago); see mlm status {d}",
                self.age_days
            ),
        }
    }
}

/// The exact, unparameterized §7.2 footer line. The caller decides
/// whether to append it (at least one row has `[!]`) and where (after
/// the table, after one blank line) — this constant is the only thing
/// this task owns.
pub const WEEK_ANOMALY_FOOTER: &str = "[!] needs a look: run mlm status DATE for details and fixes";

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("test fixture is a real date")
    }

    fn wk(y: i32, w: u32) -> WeekId {
        WeekId::new(y, w, "test").expect("test fixture is a real ISO week")
    }

    fn t(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).expect("test fixture is a real time")
    }

    /// Reference "today" throughout: a Thursday in ISO week 2026-07.
    const TODAY: fn() -> NaiveDate = || NaiveDate::from_ymd_opt(2026, 2, 12).unwrap();

    // ---- 4.1 headline, current week ----

    #[test]
    fn t1_current_week_headline_matches_the_worked_example() {
        assert_eq!(
            week_headline(wk(2026, 7), 645, TODAY()),
            "10h 45m left by end of Thursday"
        );
    }

    // ---- 4.2 headline, past week ----

    #[test]
    fn t2_past_week_headline_plain_total() {
        assert_eq!(
            week_headline(wk(2026, 6), 190, TODAY()),
            "Total behind: 03h 10m"
        );
    }

    #[test]
    fn t3_status_first_example_past_week() {
        assert_eq!(
            week_headline(wk(2026, 2), 100, TODAY()),
            "Total behind: 01h 40m"
        );
    }

    // ---- 4.3 F11: different day within current week uses TODAY's weekday ----

    #[test]
    fn t4_f11_headline_uses_todays_weekday_not_the_displayed_dates() {
        // The status caller is displaying Monday 2026-02-09's stints,
        // but "today" is Thursday 2026-02-12. week_headline() takes no
        // "date being displayed" parameter at all -- there is no
        // argument a caller could pass that would make the weekday come
        // out as anything other than today's. A regression that added
        // a `date` parameter would have to change this call to break
        // this test, which is the signal we want.
        assert_eq!(
            week_headline(wk(2026, 7), 645, TODAY()),
            "10h 45m left by end of Thursday"
        );
    }

    #[test]
    fn t5_f11_status_week_line_uses_todays_weekday() {
        assert_eq!(
            status_week_line(wk(2026, 7), 645, 1755, 2400, 0, TODAY()),
            "Week 2026-07:  10h 45m left by end of Thursday (fulfillment 29h 15m / target 40h 00m)"
        );
    }

    // ---- §5: carry-in inline ----

    #[test]
    fn t34_carry_in_zero_is_byte_identical_to_current_output() {
        assert_eq!(
            status_week_line(wk(2026, 7), 645, 1755, 2400, 0, TODAY()),
            "Week 2026-07:  10h 45m left by end of Thursday (fulfillment 29h 15m / target 40h 00m)"
        );
    }

    #[test]
    fn t35_carry_in_deficit_worked_example() {
        assert_eq!(
            status_week_line(wk(2026, 7), 645, 1755, 2400, -130, TODAY()),
            "Week 2026-07:  10h 45m left by end of Thursday (fulfillment 29h 15m = worked 31h 25m + carry-in -02h 10m / target 40h 00m)"
        );
    }

    #[test]
    fn t36_carry_in_fulfillment_goes_negative_worked_example() {
        assert_eq!(
            status_week_line(wk(2026, 7), 2520, -120, 2400, -300, TODAY()),
            "Week 2026-07:  42h 00m left by end of Thursday (fulfillment -02h 00m = worked 03h 00m + carry-in -05h 00m / target 40h 00m)"
        );
    }

    #[test]
    fn t37_carry_in_surplus_worked_example() {
        assert_eq!(
            status_week_line(wk(2026, 7), 0, 2400, 2400, 50, TODAY()),
            "Week 2026-07:  00h 00m left by end of Thursday (fulfillment 40h 00m = worked 39h 10m + carry-in 00h 50m / target 40h 00m)"
        );
    }

    #[test]
    fn t38_carry_in_gains_no_parenthetical_on_a_closed_week() {
        let line = status_week_line(wk(2026, 2), 100, 999, 999, -130, TODAY());
        assert_eq!(line, "Week 2026-02:  Total behind: 01h 40m");
        assert!(!line.contains("carry-in"));
    }

    // ---- 4.4 F10: past date shows plain total ----

    #[test]
    fn t6_f10_status_week_line_past_week_is_plain_total() {
        let line = status_week_line(wk(2026, 2), 100, 999, 999, 0, TODAY());
        assert_eq!(line, "Week 2026-02:  Total behind: 01h 40m");
        for weekday in [
            "Monday",
            "Tuesday",
            "Wednesday",
            "Thursday",
            "Friday",
            "Saturday",
            "Sunday",
        ] {
            assert!(
                !line.contains(weekday),
                "{line} should not contain {weekday}"
            );
        }
        assert!(!line.contains("left by end of"));
        assert!(!line.contains("fulfillment"));
    }

    // ---- 4.5 future week ----

    #[test]
    fn t7_future_week_gets_the_same_plain_form_as_closed() {
        assert_eq!(
            week_headline(wk(2026, 10), 2400, TODAY()),
            "Total behind: 40h 00m"
        );
    }

    // ---- 4.6 Total ahead and sign handling ----

    #[test]
    fn t8_zero_owed_on_a_closed_week_is_total_ahead() {
        assert_eq!(
            week_headline(wk(2026, 6), 0, TODAY()),
            "Total ahead: 00h 00m"
        );
    }

    #[test]
    fn t9_negative_owed_renders_as_a_magnitude_under_total_ahead() {
        assert_eq!(
            week_headline(wk(2026, 6), -50, TODAY()),
            "Total ahead: 00h 50m"
        );
    }

    #[test]
    fn t10_one_minute_owed_is_still_owed_the_boundary_above_t8() {
        assert_eq!(
            week_headline(wk(2026, 6), 1, TODAY()),
            "Total behind: 00h 01m"
        );
    }

    #[test]
    fn t11_current_week_already_ahead_drops_the_weekday() {
        assert_eq!(
            week_headline(wk(2026, 7), -150, TODAY()),
            "02h 30m ahead of target"
        );
    }

    #[test]
    fn t11b_current_week_ahead_boundary_one_minute() {
        assert_eq!(
            week_headline(wk(2026, 7), -1, TODAY()),
            "00h 01m ahead of target"
        );
    }

    #[test]
    fn t11c_status_week_line_ahead_form_short_parenthetical() {
        assert_eq!(
            status_week_line(wk(2026, 7), -275, 2675, 2400, 0, TODAY()),
            "Week 2026-07:  04h 35m ahead of target (fulfillment 44h 35m / target 40h 00m)"
        );
    }

    #[test]
    fn t11d_status_week_line_ahead_form_carry_in_surplus_alone() {
        // No work logged at all; a carry-in surplus alone puts the week
        // ahead of target.
        assert_eq!(
            status_week_line(wk(2026, 7), -50, 2450, 2400, 2450, TODAY()),
            "Week 2026-07:  00h 50m ahead of target (fulfillment 40h 50m = worked 00h 00m + carry-in 40h 50m / target 40h 00m)"
        );
    }

    #[test]
    fn t12_negative_zero_never_renders_with_a_minus_sign() {
        assert_eq!(format_minutes(-0i64), "00h 00m");
        assert!(!week_headline(wk(2026, 6), 0, TODAY()).contains("-00h 00m"));
        assert!(!week_headline(wk(2026, 7), 0, TODAY()).contains("-00h 00m"));
    }

    // ---- 4.7 ISO year-boundary framing ----

    #[test]
    fn t13_dec29_today_is_current_for_the_january_week_it_belongs_to() {
        // Mon 2025-12-29 is in ISO week 1 of 2026; a calendar-year
        // comparison would wrongly say Closed.
        assert_eq!(
            week_framing(wk(2026, 1), d(2025, 12, 29)),
            WeekFraming::Current
        );
    }

    #[test]
    fn t14_jan1_2027_today_is_current_for_iso_week_53_of_2026() {
        // 2026 has 53 ISO weeks; Fri 2027-01-01 is in week 53 of 2026.
        assert_eq!(
            week_framing(wk(2026, 53), d(2027, 1, 1)),
            WeekFraming::Current
        );
    }

    #[test]
    fn t15_same_calendar_year_different_iso_week_is_closed() {
        assert_eq!(
            week_framing(wk(2025, 1), d(2025, 12, 29)),
            WeekFraming::Closed
        );
    }

    // ---- DateAnomalies / LookbackFinding (open-punch-visibility) ----

    fn da(date: NaiveDate) -> DateAnomalies {
        DateAnomalies {
            date,
            open_stint_count: 0,
            latest_open_start: None,
            orphaned_end_times: Vec::new(),
            stale_open_start: None,
            refused_join_with_previous: None,
            refused_join_with_next: None,
        }
    }

    #[test]
    fn t_e7_reworded_matches_spec_example() {
        let a = DateAnomalies {
            open_stint_count: 2,
            latest_open_start: Some(t(14, 0)),
            ..da(d(2026, 9, 21))
        };
        assert_eq!(
            a.detail_lines(),
            vec![
                "[!] 2 open stints for this date (only one can be open at a time)".to_string(),
                "    fix: mlm delete punch --date 2026-09-21  (lists punches; rerun with the extra start's number)".to_string(),
                "     or: mlm stop HH:MM --date 2026-09-21  (a time after 14:00 closes the 14:00 start)".to_string(),
            ]
        );
    }

    #[test]
    fn t_stale_matches_spec_example() {
        let a = DateAnomalies {
            open_stint_count: 1,
            stale_open_start: Some(StaleOpenStart {
                time: t(9, 0),
                age_days: 5,
            }),
            ..da(d(2026, 9, 21))
        };
        assert_eq!(
            a.detail_lines(),
            vec![
                "[!] unclosed start at 09:00, 5 days old (no stop recorded)".to_string(),
                "    fix: mlm stop HH:MM --date 2026-09-21  (a time after 09:00)".to_string(),
                "     or: mlm delete punch --date 2026-09-21  (lists punches; rerun with the start's number)".to_string(),
            ]
        );
    }

    #[test]
    fn t_e8_gains_fix_lines_text_unchanged() {
        let a = DateAnomalies {
            orphaned_end_times: vec![t(18, 0)],
            ..da(d(2026, 9, 21))
        };
        assert_eq!(
            a.detail_lines(),
            vec![
                "[!] orphaned end at 18:00 (no matching start)".to_string(),
                "    fix: mlm start HH:MM --date 2026-09-21  (a time before 18:00)".to_string(),
                "     or: mlm delete punch --date 2026-09-21  (lists punches; rerun with the end's number)".to_string(),
            ]
        );
    }

    #[test]
    fn t_refused_join_previous_side() {
        let a = DateAnomalies {
            refused_join_with_previous: Some(d(2026, 9, 21)),
            ..da(d(2026, 9, 22))
        };
        assert_eq!(
            a.detail_lines(),
            vec![
                "[!] not joined to Mon 2026-09-21 across midnight (a join needs exactly one open start on Mon 2026-09-21 and exactly one unmatched end, as Tue 2026-09-22's first punch)".to_string(),
            ]
        );
    }

    #[test]
    fn t_refused_join_next_side() {
        let a = DateAnomalies {
            refused_join_with_next: Some(d(2026, 9, 22)),
            ..da(d(2026, 9, 21))
        };
        assert_eq!(
            a.detail_lines(),
            vec![
                "[!] not joined to Tue 2026-09-22 across midnight (a join needs exactly one open start on Mon 2026-09-21 and exactly one unmatched end, as Tue 2026-09-22's first punch)".to_string(),
            ]
        );
    }

    #[test]
    fn t_lookback_single_block_matches_spec_example() {
        let f = LookbackFinding {
            date: d(2026, 9, 21),
            age_days: 5,
            kind: LookbackKind::SingleOpenStart {
                time: t(9, 0),
                punch_id: 1,
            },
        };
        assert_eq!(
            f.detail_lines(),
            vec![
                "[!] unclosed start on Mon 2026-09-21 at 09:00 (5 days ago)".to_string(),
                "    fix: mlm stop HH:MM --date 2026-09-21  (a time after 09:00)".to_string(),
                "     or: mlm delete punch --date 2026-09-21  (lists punches; rerun with the start's number)".to_string(),
            ]
        );
    }

    #[test]
    fn t_lookback_multi_block_matches_spec_example() {
        let f = LookbackFinding {
            date: d(2026, 9, 21),
            age_days: 5,
            kind: LookbackKind::MultipleOpenStints { count: 2 },
        };
        assert_eq!(
            f.detail_lines(),
            vec![
                "[!] 2 open stints on Mon 2026-09-21 (5 days ago)".to_string(),
                "    fix: see mlm status 2026-09-21".to_string(),
            ]
        );
    }

    #[test]
    fn t_ordering_all_kinds_at_once() {
        // Unrealistic combination (stale + E7 together) is impossible
        // under the real stale/E7 predicate, but render.rs enforces no such
        // invariant -- this test only pins detail_lines()'s ordering.
        let a = DateAnomalies {
            date: d(2026, 9, 21),
            open_stint_count: 2,
            latest_open_start: Some(t(14, 0)),
            orphaned_end_times: vec![t(10, 0), t(11, 0)],
            stale_open_start: Some(StaleOpenStart {
                time: t(9, 0),
                age_days: 5,
            }),
            refused_join_with_previous: Some(d(2026, 9, 20)),
            refused_join_with_next: Some(d(2026, 9, 22)),
        };
        let lines = a.detail_lines();
        assert_eq!(lines.len(), 3 + 3 + 3 + 3 + 1 + 1);
        assert!(lines[0].starts_with("[!] 2 open stints"));
        assert!(lines[3].starts_with("[!] unclosed start at 09:00"));
        assert!(lines[6].starts_with("[!] orphaned end at 10:00"));
        assert!(lines[9].starts_with("[!] orphaned end at 11:00"));
        assert!(lines[12].starts_with("[!] not joined to Sun 2026-09-20"));
        assert!(lines[13].starts_with("[!] not joined to Tue 2026-09-22"));
    }

    #[test]
    fn t_lookback_ordering_oldest_first() {
        // This only proves rendering preserves caller-supplied order;
        // the actual "oldest first" scan guarantee is the lookback
        // scan's own job.
        let findings = [
            LookbackFinding {
                date: d(2026, 9, 20),
                age_days: 6,
                kind: LookbackKind::SingleOpenStart {
                    time: t(8, 0),
                    punch_id: 1,
                },
            },
            LookbackFinding {
                date: d(2026, 9, 21),
                age_days: 5,
                kind: LookbackKind::MultipleOpenStints { count: 2 },
            },
        ];
        let combined: Vec<String> = findings.iter().flat_map(|f| f.detail_lines()).collect();
        assert!(combined[0].contains("2026-09-20"));
        assert!(combined[3].contains("2026-09-21"));
    }

    #[test]
    fn t_week_footer_matches_spec_example() {
        assert_eq!(
            WEEK_ANOMALY_FOOTER,
            "[!] needs a look: run mlm status DATE for details and fixes"
        );
    }

    #[test]
    fn t_lookback_stderr_single_is_three_lines_like_status_block() {
        let f = LookbackFinding {
            date: d(2026, 9, 21),
            age_days: 5,
            kind: LookbackKind::SingleOpenStart {
                time: t(9, 0),
                punch_id: 1,
            },
        };
        assert_eq!(
            f.stderr_line(),
            "warning: unclosed start on Mon 2026-09-21 at 09:00 (5 days ago)\n    fix: mlm stop HH:MM --date 2026-09-21  (a time after 09:00)\n     or: mlm delete punch --date 2026-09-21  (lists punches; rerun with the start's number)"
        );
    }

    #[test]
    fn t_lookback_stderr_multi_matches_spec_example() {
        let f = LookbackFinding {
            date: d(2026, 9, 21),
            age_days: 5,
            kind: LookbackKind::MultipleOpenStints { count: 2 },
        };
        assert_eq!(
            f.stderr_line(),
            "warning: 2 open stints on Mon 2026-09-21 (5 days ago); see mlm status 2026-09-21"
        );
    }

    #[test]
    fn t_lookback_stderr_single_matches_block_fix_or_wording_exactly() {
        // §7.4: the single-open-start warning is laid out
        // identically to status's own lookback block, "warning: "
        // swapped in for the leading "[!] " -- same fix/or wording,
        // same double-space before the parenthetical.
        let f = LookbackFinding {
            date: d(2026, 9, 21),
            age_days: 5,
            kind: LookbackKind::SingleOpenStart {
                time: t(9, 0),
                punch_id: 1,
            },
        };
        let block = f.detail_lines();
        let stderr = f.stderr_line();
        let stderr_lines: Vec<&str> = stderr.split('\n').collect();
        assert_eq!(stderr_lines.len(), 3);
        assert_eq!(stderr_lines[0], format!("warning: {}", &block[0][4..]));
        assert_eq!(stderr_lines[1], block[1]);
        assert_eq!(stderr_lines[2], block[2]);
    }

    #[test]
    fn t_date_anomalies_new_fixtures_never_contain_old_e7_suffix() {
        let fixtures = [
            DateAnomalies {
                open_stint_count: 2,
                latest_open_start: Some(t(14, 0)),
                ..da(d(2026, 9, 21))
            },
            DateAnomalies {
                open_stint_count: 1,
                stale_open_start: Some(StaleOpenStart {
                    time: t(9, 0),
                    age_days: 5,
                }),
                ..da(d(2026, 9, 21))
            },
        ];
        for a in fixtures {
            for line in a.detail_lines() {
                assert!(!line.contains("(unmatched starts)"), "{line}");
            }
        }
    }

    #[test]
    fn t33_every_produced_string_is_ascii() {
        let headlines = [
            week_headline(wk(2026, 7), 645, TODAY()),
            week_headline(wk(2026, 6), 190, TODAY()),
            week_headline(wk(2026, 6), 0, TODAY()),
            week_headline(wk(2026, 6), -50, TODAY()),
            week_headline(wk(2026, 7), -150, TODAY()),
            status_week_line(wk(2026, 7), 645, 1755, 2400, 0, TODAY()),
            status_week_line(wk(2026, 2), 100, 999, 999, 0, TODAY()),
            status_week_line(wk(2026, 7), 645, 1755, 2400, -130, TODAY()),
        ];
        for h in headlines {
            assert!(h.is_ascii(), "{h}");
        }
    }

    // Extends t33's plain-ASCII sweep to the open-punch-visibility
    // literals (DateAnomalies/LookbackFinding/WEEK_ANOMALY_FOOTER),
    // split into its own function to stay under the cognitive
    // complexity budget.
    #[test]
    fn t33b_open_punch_visibility_literals_are_ascii() {
        let date_anomaly_fixtures = [
            DateAnomalies {
                open_stint_count: 2,
                latest_open_start: Some(t(14, 0)),
                ..da(d(2026, 9, 21))
            },
            DateAnomalies {
                open_stint_count: 1,
                stale_open_start: Some(StaleOpenStart {
                    time: t(9, 0),
                    age_days: 5,
                }),
                ..da(d(2026, 9, 21))
            },
            DateAnomalies {
                orphaned_end_times: vec![t(18, 0)],
                ..da(d(2026, 9, 21))
            },
            DateAnomalies {
                refused_join_with_previous: Some(d(2026, 9, 21)),
                ..da(d(2026, 9, 22))
            },
            DateAnomalies {
                refused_join_with_next: Some(d(2026, 9, 22)),
                ..da(d(2026, 9, 21))
            },
        ];
        for a in date_anomaly_fixtures {
            for line in a.detail_lines() {
                assert!(line.is_ascii(), "{line}");
            }
            assert!(a.row_marker().is_ascii());
        }

        let lookback_fixtures = [
            LookbackFinding {
                date: d(2026, 9, 21),
                age_days: 5,
                kind: LookbackKind::SingleOpenStart {
                    time: t(9, 0),
                    punch_id: 1,
                },
            },
            LookbackFinding {
                date: d(2026, 9, 21),
                age_days: 5,
                kind: LookbackKind::MultipleOpenStints { count: 2 },
            },
        ];
        for f in lookback_fixtures {
            for line in f.detail_lines() {
                assert!(line.is_ascii(), "{line}");
            }
            assert!(f.stderr_line().is_ascii());
        }

        assert!(WEEK_ANOMALY_FOOTER.is_ascii());
    }
}
