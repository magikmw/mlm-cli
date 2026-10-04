//! Shared rendering helpers for `status` (Milestone 10) and `week`
//! (Milestone 11): the §7.1/§7.2 week headline, the stop-hint phrases and
//! the carry marker line. Pure functions only — no DB access, no CLI
//! wiring, no `chrono::Local::now()` (PLAN.md contract 6: "now" is
//! always an injected parameter).
//!
//! This module is the single place that decides wording and sign
//! handling for both consumers, so that they never grow their own
//! diverging copies (see plans/milestone-9-shared-rendering.md). The
//! per-date anomaly facts and lines live in `anomalies.rs`, and the
//! lookback finding types in `lookback.rs`. Both are re-exported here.

// Milestone 9 is built ahead of its consumers (Milestones 10/11), so
// the public surface here has no in-crate caller yet outside the tests.
#![allow(dead_code)]

use chrono::{DateTime, Datelike, Local, NaiveDate, NaiveTime, Utc};

use crate::date::{WeekId, format_date_with_weekday, format_weekday_full};
use crate::storage::Punch;
use crate::time::format_minutes;
use crate::week::{self, WeekAccounting};

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

/// The point-in-time headline for the current week: `gap` is
/// `required - fulfillment`, where `required` is the slice of the target due
/// by the end of `today`. Monday to Thursday it names that slice and
/// today's weekday. Friday to Sunday the slice is the whole target, so the
/// line names the target alone.
fn current_week_headline(acct: &WeekAccounting, today: NaiveDate) -> String {
    let (required, gap) = week::required_and_gap(acct, today);
    let (word, magnitude) = if gap >= 0 {
        ("left to", gap)
    } else {
        ("over", -gap)
    };
    let amount = format_minutes(magnitude);
    if today.weekday().number_from_monday() >= 5 {
        format!("{amount} {word} {} target", format_minutes(required))
    } else {
        format!(
            "{amount} {word} {} by end of {}",
            format_minutes(required),
            format_weekday_full(today)
        )
    }
}

/// The §7.1/§7.2 headline for a week, in its bare form (no `Week NN:`
/// label, no fulfillment/target parenthetical).
///
/// The current week reads `acct`'s live-inclusive `fulfillment` and
/// `target` (see `current_week_headline`). Every other week reads its
/// signed `owed`. `today` is the injected "now" date (PLAN.md contract 6),
/// so the weekday in the deadline phrase is always *today's*, never that of
/// whatever date the caller happens to be displaying (§7.1, F11).
pub fn week_headline(acct: &WeekAccounting, today: NaiveDate) -> String {
    match week_framing(acct.week, today) {
        WeekFraming::Current => current_week_headline(acct, today),
        WeekFraming::Closed if acct.owed > 0 => {
            format!("Total behind: {}", format_minutes(acct.owed))
        }
        WeekFraming::Closed => format!("Total ahead: {}", format_minutes(-acct.owed)),
    }
}

/// The complete §7.1 week line, label and all, e.g.
/// `Week 2026-07:  02h 45m left to 32h 00m by end of Thursday (fulfillment 29h 15m / target 40h 00m)`
pub fn status_week_line(acct: &WeekAccounting, today: NaiveDate) -> String {
    let label = format!("Week {}:", acct.week);
    let headline = week_headline(acct, today);
    let parenthetical = match week_framing(acct.week, today) {
        WeekFraming::Current if acct.carry_in == 0 => format!(
            " (fulfillment {} / target {})",
            format_minutes(acct.fulfillment),
            format_minutes(acct.target)
        ),
        WeekFraming::Current => format!(
            " (fulfillment {} = worked {} + carry-in {} / target {})",
            format_minutes(acct.fulfillment),
            format_minutes(acct.fulfillment - acct.carry_in),
            format_minutes(acct.carry_in),
            format_minutes(acct.target)
        ),
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

pub use crate::anomalies::{DateAnomalies, StaleOpenStart};
pub use crate::lookback::{LookbackFinding, LookbackKind};

/// The exact, unparameterized §7.2 footer line. The caller decides
/// whether to append it (at least one row has `[!]`) and where (after
/// the table, after one blank line) — this constant is the only thing
/// this task owns.
pub const WEEK_ANOMALY_FOOTER: &str = "[!] needs a look: run mlm status DATE for details and fixes";

/// Footer when a `mlm week` row shows `[i]`.
pub const WEEK_OVERNIGHT_FOOTER: &str = "[i] open stint: run mlm status to stop it";

/// Monday footer for the current week's view. `date` is the overnight
/// stint's date.
pub fn week_overnight_monday_footer(date: NaiveDate) -> String {
    format!(
        "[i] open stint on {} (last week): run mlm status to stop it",
        format_date_with_weekday(date)
    )
}

/// Local wall-clock time of a UTC instant.
pub fn local_time(at_utc: DateTime<Utc>) -> NaiveTime {
    at_utc.with_timezone(&Local).time()
}

/// The first punch of any kind on `start.date` that is strictly later than
/// `start`, by `(at_utc, kind, id)`, as a local time. Punches at `start`'s
/// own instant and punches on other dates are ignored. `punches` may span
/// several dates.
pub fn next_punch_after(punches: &[Punch], start: &Punch) -> Option<NaiveTime> {
    punches
        .iter()
        .filter(|p| p.date == start.date && p.at_utc > start.at_utc)
        .min_by_key(|p| (p.at_utc, p.kind, p.id))
        .map(|p| local_time(p.at_utc))
}

fn stop_window(start: NaiveTime, next: Option<NaiveTime>) -> String {
    let s = start.format("%H:%M");
    match next {
        None => format!("a time after {s}"),
        Some(n) => format!("a time between {s} and {}", n.format("%H:%M")),
    }
}

/// `(a time after S)` or `(a time between S and N)`, parentheses included.
pub fn stop_time_hint(start: NaiveTime, next: Option<NaiveTime>) -> String {
    format!("({})", stop_window(start, next))
}

/// The E7 form of [`stop_time_hint`], with a ` closes the S start` tail
/// inside the parentheses.
pub fn stop_time_hint_closes(start: NaiveTime, next: Option<NaiveTime>) -> String {
    format!(
        "({} closes the {} start)",
        stop_window(start, next),
        start.format("%H:%M")
    )
}

/// Data behind the carry-in anomaly marker (SPEC §3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CarryMarker {
    /// Number of earlier weeks with an anomaly, at least 1.
    pub weeks: usize,
    /// The oldest such week.
    pub oldest: WeekId,
}

/// The marker line. One week names that week alone, several weeks name the
/// count and the oldest.
pub fn carry_marker_line(m: &CarryMarker) -> String {
    debug_assert!(m.weeks >= 1, "a marker needs at least one week");
    let id = m.oldest;
    if m.weeks == 1 {
        format!("[!] carry-in includes anomalies from 1 earlier week, {id} (see mlm week {id})")
    } else {
        format!(
            "[!] carry-in includes anomalies from {} earlier weeks, oldest {id} (see mlm week {id})",
            m.weeks
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::PunchKind;

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

    /// Accounting with `worked` derived from `fulfillment - carry_in`.
    /// Current-week framing reads `fulfillment` and `target`, closed-week
    /// framing reads `owed`, so tests pass all of them explicitly.
    fn acct(
        week: WeekId,
        owed: i64,
        fulfillment: i64,
        target: i64,
        carry_in: i64,
    ) -> WeekAccounting {
        WeekAccounting {
            week,
            target,
            carry_in,
            worked: fulfillment - carry_in,
            fulfillment,
            owed,
            carry_out: -owed,
        }
    }

    /// A closed-week accounting that only carries `owed`.
    fn closed(week: WeekId, owed: i64) -> WeekAccounting {
        acct(week, owed, 2400 - owed, 2400, 0)
    }

    /// Current-week (2026-07, target 40h) accounting with `fulfillment`.
    fn current(fulfillment: i64) -> WeekAccounting {
        acct(wk(2026, 7), 2400 - fulfillment, fulfillment, 2400, 0)
    }

    // ---- 4.1 headline, current week ----

    #[test]
    fn t1_current_week_headline_matches_the_worked_example() {
        assert_eq!(
            week_headline(&current(1755), TODAY()),
            "02h 45m left to 32h 00m by end of Thursday"
        );
    }

    // ---- 4.2 headline, past week ----

    #[test]
    fn t2_past_week_headline_plain_total() {
        assert_eq!(
            week_headline(&closed(wk(2026, 6), 190), TODAY()),
            "Total behind: 03h 10m"
        );
    }

    #[test]
    fn t3_status_first_example_past_week() {
        assert_eq!(
            week_headline(&closed(wk(2026, 2), 100), TODAY()),
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
            week_headline(&current(1755), TODAY()),
            "02h 45m left to 32h 00m by end of Thursday"
        );
    }

    #[test]
    fn t5_f11_status_week_line_uses_todays_weekday() {
        assert_eq!(
            status_week_line(&acct(wk(2026, 7), 645, 1755, 2400, 0), TODAY()),
            "Week 2026-07:  02h 45m left to 32h 00m by end of Thursday (fulfillment 29h 15m / target 40h 00m)"
        );
    }

    // ---- §5: carry-in inline ----

    #[test]
    fn t34_carry_in_zero_is_byte_identical_to_current_output() {
        assert_eq!(
            status_week_line(&acct(wk(2026, 7), 645, 1755, 2400, 0), TODAY()),
            "Week 2026-07:  02h 45m left to 32h 00m by end of Thursday (fulfillment 29h 15m / target 40h 00m)"
        );
    }

    #[test]
    fn t35_carry_in_deficit_worked_example() {
        assert_eq!(
            status_week_line(&acct(wk(2026, 7), 645, 1755, 2400, -130), TODAY()),
            "Week 2026-07:  02h 45m left to 32h 00m by end of Thursday (fulfillment 29h 15m = worked 31h 25m + carry-in -02h 10m / target 40h 00m)"
        );
    }

    #[test]
    fn t36_carry_in_fulfillment_goes_negative_worked_example() {
        assert_eq!(
            status_week_line(&acct(wk(2026, 7), 2520, -120, 2400, -300), TODAY()),
            "Week 2026-07:  34h 00m left to 32h 00m by end of Thursday (fulfillment -02h 00m = worked 03h 00m + carry-in -05h 00m / target 40h 00m)"
        );
    }

    #[test]
    fn t37_carry_in_surplus_worked_example() {
        assert_eq!(
            status_week_line(&acct(wk(2026, 7), 0, 1920, 2400, 50), TODAY()),
            "Week 2026-07:  00h 00m left to 32h 00m by end of Thursday (fulfillment 32h 00m = worked 31h 10m + carry-in 00h 50m / target 40h 00m)"
        );
    }

    #[test]
    fn t38_carry_in_gains_no_parenthetical_on_a_closed_week() {
        let line = status_week_line(&acct(wk(2026, 2), 100, 999, 999, -130), TODAY());
        assert_eq!(line, "Week 2026-02:  Total behind: 01h 40m");
        assert!(!line.contains("carry-in"));
    }

    // ---- 4.4 F10: past date shows plain total ----

    #[test]
    fn t6_f10_status_week_line_past_week_is_plain_total() {
        let line = status_week_line(&acct(wk(2026, 2), 100, 999, 999, 0), TODAY());
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
        assert!(!line.contains("by end of"));
        assert!(!line.contains("fulfillment"));
    }

    // ---- 4.5 future week ----

    #[test]
    fn t7_future_week_gets_the_same_plain_form_as_closed() {
        assert_eq!(
            week_headline(&closed(wk(2026, 10), 2400), TODAY()),
            "Total behind: 40h 00m"
        );
    }

    // ---- 4.6 Total ahead and sign handling ----

    #[test]
    fn t8_zero_owed_on_a_closed_week_is_total_ahead() {
        assert_eq!(
            week_headline(&closed(wk(2026, 6), 0), TODAY()),
            "Total ahead: 00h 00m"
        );
    }

    #[test]
    fn t9_negative_owed_renders_as_a_magnitude_under_total_ahead() {
        assert_eq!(
            week_headline(&closed(wk(2026, 6), -50), TODAY()),
            "Total ahead: 00h 50m"
        );
    }

    #[test]
    fn t10_one_minute_owed_is_still_owed_the_boundary_above_t8() {
        assert_eq!(
            week_headline(&closed(wk(2026, 6), 1), TODAY()),
            "Total behind: 00h 01m"
        );
    }

    #[test]
    fn t11_current_week_past_required_reads_over_with_the_weekday() {
        assert_eq!(
            week_headline(&current(2070), TODAY()),
            "02h 30m over 32h 00m by end of Thursday"
        );
    }

    #[test]
    fn t11b_current_week_over_boundary_one_minute() {
        assert_eq!(
            week_headline(&current(1921), TODAY()),
            "00h 01m over 32h 00m by end of Thursday"
        );
        assert_eq!(
            week_headline(&current(1920), TODAY()),
            "00h 00m left to 32h 00m by end of Thursday"
        );
        assert_eq!(
            week_headline(&current(1919), TODAY()),
            "00h 01m left to 32h 00m by end of Thursday"
        );
    }

    #[test]
    fn t11c_status_week_line_over_form_short_parenthetical() {
        assert_eq!(
            status_week_line(&acct(wk(2026, 7), -275, 2675, 2400, 0), TODAY()),
            "Week 2026-07:  12h 35m over 32h 00m by end of Thursday (fulfillment 44h 35m / target 40h 00m)"
        );
    }

    #[test]
    fn t11d_status_week_line_over_form_carry_in_surplus_alone() {
        // No work logged at all; a carry-in surplus alone puts the week
        // over today's slice.
        assert_eq!(
            status_week_line(&acct(wk(2026, 7), -50, 2450, 2400, 2450), TODAY()),
            "Week 2026-07:  08h 50m over 32h 00m by end of Thursday (fulfillment 40h 50m = worked 00h 00m + carry-in 40h 50m / target 40h 00m)"
        );
    }

    #[test]
    fn t12_negative_zero_never_renders_with_a_minus_sign() {
        assert_eq!(format_minutes(-0i64), "00h 00m");
        assert!(!week_headline(&closed(wk(2026, 6), 0), TODAY()).contains("-00h 00m"));
        assert!(!week_headline(&current(1920), TODAY()).contains("-00h 00m"));
    }

    // ---- point-in-time: Mon-Thu slice, Fri-Sun whole target ----

    #[test]
    fn pit_friday_to_sunday_headline_names_the_whole_target() {
        for today in [d(2026, 2, 13), d(2026, 2, 14), d(2026, 2, 15)] {
            assert_eq!(
                week_headline(&current(2290), today),
                "01h 50m left to 40h 00m target"
            );
            assert_eq!(
                week_headline(&current(2400), today),
                "00h 00m left to 40h 00m target"
            );
            assert_eq!(
                week_headline(&current(2401), today),
                "00h 01m over 40h 00m target"
            );
            assert_eq!(
                week_headline(&current(2470), today),
                "01h 10m over 40h 00m target"
            );
        }
    }

    #[test]
    fn pit_friday_required_is_not_rounded_to_five_minutes() {
        let acct = acct(wk(2026, 7), 2011, 0, 2011, 0);
        assert_eq!(
            week_headline(&acct, d(2026, 2, 13)),
            "33h 31m left to 33h 31m target"
        );
    }

    #[test]
    fn pit_monday_to_thursday_required_is_floor_fifth_times_weekday() {
        // 2011 / 5 floors to 402; Monday 402, Tuesday 804, Wednesday 1206.
        let acct = acct(wk(2026, 7), 2011, 0, 2011, 0);
        assert_eq!(
            week_headline(&acct, d(2026, 2, 9)),
            "06h 42m left to 06h 42m by end of Monday"
        );
        assert_eq!(
            week_headline(&acct, d(2026, 2, 10)),
            "13h 24m left to 13h 24m by end of Tuesday"
        );
        assert_eq!(
            week_headline(&acct, d(2026, 2, 11)),
            "20h 06m left to 20h 06m by end of Wednesday"
        );
    }

    #[test]
    fn pit_zero_target_headline() {
        let zero = acct(wk(2026, 7), 0, 0, 0, 0);
        assert_eq!(
            week_headline(&zero, d(2026, 2, 9)),
            "00h 00m left to 00h 00m by end of Monday"
        );
        let two_hours = acct(wk(2026, 7), -120, 120, 0, 0);
        assert_eq!(
            week_headline(&two_hours, d(2026, 2, 9)),
            "02h 00m over 00h 00m by end of Monday"
        );
    }

    #[test]
    fn pit_closed_and_future_weeks_ignore_required() {
        // A future week gets the closed headline whatever today is.
        assert_eq!(
            week_headline(&closed(wk(2026, 8), 2400), d(2026, 2, 13)),
            "Total behind: 40h 00m"
        );
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

    // ---- week footer ----

    #[test]
    fn t_week_footer_matches_spec_example() {
        assert_eq!(
            WEEK_ANOMALY_FOOTER,
            "[!] needs a look: run mlm status DATE for details and fixes"
        );
    }

    #[test]
    fn t33_every_produced_string_is_ascii() {
        let headlines = [
            week_headline(&current(1755), TODAY()),
            week_headline(&closed(wk(2026, 6), 190), TODAY()),
            week_headline(&closed(wk(2026, 6), 0), TODAY()),
            week_headline(&closed(wk(2026, 6), -50), TODAY()),
            week_headline(&current(2070), TODAY()),
            week_headline(&current(2290), d(2026, 2, 13)),
            status_week_line(&acct(wk(2026, 7), 645, 1755, 2400, 0), TODAY()),
            status_week_line(&acct(wk(2026, 2), 100, 999, 999, 0), TODAY()),
            status_week_line(&acct(wk(2026, 7), 645, 1755, 2400, -130), TODAY()),
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
                ..DateAnomalies::none(d(2026, 9, 21))
            },
            DateAnomalies {
                open_stint_count: 1,
                stale_open_start: Some(StaleOpenStart {
                    time: t(9, 0),
                    age_days: 5,
                }),
                ..DateAnomalies::none(d(2026, 9, 21))
            },
            DateAnomalies {
                orphaned_end_times: vec![t(18, 0)],
                ..DateAnomalies::none(d(2026, 9, 21))
            },
            DateAnomalies {
                refused_join_with_previous: Some(d(2026, 9, 21)),
                ..DateAnomalies::none(d(2026, 9, 22))
            },
            DateAnomalies {
                refused_join_with_next: Some(d(2026, 9, 22)),
                ..DateAnomalies::none(d(2026, 9, 21))
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
        assert!(WEEK_OVERNIGHT_FOOTER.is_ascii());
        assert!(week_overnight_monday_footer(d(2026, 10, 4)).is_ascii());
    }

    #[test]
    fn overnight_footers_have_exact_text() {
        assert_eq!(
            WEEK_OVERNIGHT_FOOTER,
            "[i] open stint: run mlm status to stop it"
        );
        assert_eq!(
            week_overnight_monday_footer(d(2026, 10, 4)),
            "[i] open stint on Sun 2026-10-04 (last week): run mlm status to stop it"
        );
    }

    // ---- clean-pair helper and stop-hint phrases ----

    fn punch(id: i64, kind: PunchKind, date: NaiveDate, h: u32, m: u32) -> Punch {
        crate::storage::punch_from_local(id, kind, date, t(h, m), &Local)
            .expect("test punch is a real local time")
    }

    fn start_at(id: i64, date: NaiveDate, h: u32, m: u32) -> Punch {
        punch(id, PunchKind::Start, date, h, m)
    }

    fn end_at(id: i64, date: NaiveDate, h: u32, m: u32) -> Punch {
        punch(id, PunchKind::End, date, h, m)
    }

    #[test]
    fn next_punch_after_none_when_no_later_punch() {
        let day = d(2026, 1, 14);
        let s = start_at(1, day, 9, 0);
        assert_eq!(next_punch_after(&[s], &s), None);
    }

    #[test]
    fn next_punch_after_finds_a_later_start() {
        let day = d(2026, 1, 14);
        let s = start_at(1, day, 9, 0);
        let later = start_at(2, day, 11, 0);
        assert_eq!(next_punch_after(&[s, later], &s), Some(t(11, 0)));
    }

    #[test]
    fn next_punch_after_finds_a_later_end() {
        let day = d(2026, 1, 14);
        let s = start_at(1, day, 9, 0);
        let later = end_at(2, day, 12, 30);
        assert_eq!(next_punch_after(&[s, later], &s), Some(t(12, 30)));
    }

    #[test]
    fn next_punch_after_ignores_a_punch_at_the_same_minute() {
        let day = d(2026, 1, 14);
        let s = start_at(1, day, 9, 0);
        let same = end_at(2, day, 9, 0);
        assert_eq!(next_punch_after(&[s, same], &s), None);
    }

    #[test]
    fn next_punch_after_is_none_for_a_start_at_2359() {
        let day = d(2026, 1, 14);
        let s = start_at(1, day, 23, 59);
        let next_day = start_at(2, d(2026, 1, 15), 0, 5);
        assert_eq!(next_punch_after(&[s, next_day], &s), None);
    }

    #[test]
    fn next_punch_after_ignores_other_dates() {
        let day = d(2026, 1, 14);
        let s = start_at(1, day, 9, 0);
        let before = start_at(2, d(2026, 1, 13), 20, 0);
        let after = start_at(3, d(2026, 1, 15), 10, 0);
        assert_eq!(next_punch_after(&[before, s, after], &s), None);
    }

    #[test]
    fn next_punch_after_returns_the_earlier_of_two_later_punches() {
        let day = d(2026, 1, 14);
        let s = start_at(1, day, 9, 0);
        let far = start_at(2, day, 15, 0);
        let near = start_at(3, day, 11, 0);
        assert_eq!(next_punch_after(&[s, far, near], &s), Some(t(11, 0)));
    }

    #[test]
    fn stop_time_hint_without_a_next_punch() {
        assert_eq!(stop_time_hint(t(18, 2), None), "(a time after 18:02)");
    }

    #[test]
    fn stop_time_hint_with_a_next_punch() {
        assert_eq!(
            stop_time_hint(t(18, 2), Some(t(19, 0))),
            "(a time between 18:02 and 19:00)"
        );
    }

    #[test]
    fn stop_time_hint_one_minute_window() {
        assert_eq!(
            stop_time_hint(t(18, 2), Some(t(18, 3))),
            "(a time between 18:02 and 18:03)"
        );
    }

    #[test]
    fn stop_time_hint_closes_without_a_next_punch() {
        assert_eq!(
            stop_time_hint_closes(t(14, 0), None),
            "(a time after 14:00 closes the 14:00 start)"
        );
    }

    #[test]
    fn stop_time_hint_closes_with_a_next_punch() {
        assert_eq!(
            stop_time_hint_closes(t(14, 0), Some(t(15, 0))),
            "(a time between 14:00 and 15:00 closes the 14:00 start)"
        );
    }

    // ---- carry marker line ----

    #[test]
    fn carry_marker_line_plural() {
        let m = CarryMarker {
            weeks: 2,
            oldest: wk(2026, 38),
        };
        assert_eq!(
            carry_marker_line(&m),
            "[!] carry-in includes anomalies from 2 earlier weeks, oldest 2026-38 (see mlm week 2026-38)"
        );
    }

    #[test]
    fn carry_marker_line_singular() {
        let m = CarryMarker {
            weeks: 1,
            oldest: wk(2026, 38),
        };
        assert_eq!(
            carry_marker_line(&m),
            "[!] carry-in includes anomalies from 1 earlier week, 2026-38 (see mlm week 2026-38)"
        );
    }

    #[test]
    fn carry_marker_line_is_ascii() {
        let m = CarryMarker {
            weeks: 3,
            oldest: wk(2026, 1),
        };
        assert!(carry_marker_line(&m).is_ascii());
    }
}
