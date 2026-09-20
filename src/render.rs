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

use crate::date::{WeekId, format_weekday_full};
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
        WeekFraming::Current => format!(
            "{} left by end of {}",
            format_minutes(owed_minutes),
            format_weekday_full(today)
        ),
        WeekFraming::Closed if owed_minutes > 0 => {
            format!("Total still owed: {}", format_minutes(owed_minutes))
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

/// The §4.3 pairing anomalies for one date, in the minimal shape the
/// §7.3 renderers need. Milestone 5's classification result converts
/// into this; nothing here re-derives pairing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Anomalies {
    /// How many unmatched trailing `start`s the date has (i.e. how many
    /// open stints). 0 or 1 is normal and is NOT an anomaly (§4.3,
    /// §1.3, F1); 2 or more is the multi-open anomaly (E7).
    pub open_stint_count: usize,
    /// Local wall-clock time of each orphaned `end` (§4.3's empty-stack
    /// case), in the date's punch order (chronological, insertion-order
    /// tie-break). One entry per orphan — never deduplicated, never
    /// coalesced, even when two share a timestamp (E8, NOTES.md 32).
    pub orphaned_end_times: Vec<NaiveTime>,
}

impl Anomalies {
    /// The single definition of "this date has an anomaly", shared by
    /// both rendering forms. §4.3: exactly one trailing unmatched start
    /// is the ordinary open stint, not an anomaly.
    pub fn has_any(&self) -> bool {
        self.open_stint_count > 1 || !self.orphaned_end_times.is_empty()
    }

    /// `status`'s form (§7.3): one full line per anomaly, already
    /// prefixed with `[!] `, no trailing newline on any element.
    /// Empty vec when `has_any()` is false.
    pub fn detail_lines(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.open_stint_count > 1 {
            out.push(format!(
                "[!] {} open stints for this date (unmatched starts)",
                self.open_stint_count
            ));
        }
        for t in &self.orphaned_end_times {
            out.push(format!(
                "[!] orphaned end at {} (no matching start)",
                t.format("%H:%M")
            ));
        }
        out
    }

    /// `week`'s form (§7.3): the suffix to append to a date's row.
    /// `"  [!]"` when `has_any()`, `""` otherwise.
    pub fn row_marker(&self) -> &'static str {
        if self.has_any() { "  [!]" } else { "" }
    }
}

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
            "Total still owed: 03h 10m"
        );
    }

    #[test]
    fn t3_status_first_example_past_week() {
        assert_eq!(
            week_headline(wk(2026, 2), 100, TODAY()),
            "Total still owed: 01h 40m"
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
        assert_eq!(line, "Week 2026-02:  Total still owed: 01h 40m");
        assert!(!line.contains("carry-in"));
    }

    // ---- 4.4 F10: past date shows plain total ----

    #[test]
    fn t6_f10_status_week_line_past_week_is_plain_total() {
        let line = status_week_line(wk(2026, 2), 100, 999, 999, 0, TODAY());
        assert_eq!(line, "Week 2026-02:  Total still owed: 01h 40m");
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
            "Total still owed: 40h 00m"
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
            "Total still owed: 00h 01m"
        );
    }

    #[test]
    fn t11_current_week_already_ahead_passes_sign_through_verbatim() {
        assert_eq!(
            week_headline(wk(2026, 7), -150, TODAY()),
            "-02h 30m left by end of Thursday"
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

    // ---- 4.8 anomaly detail lines ----

    #[test]
    fn t16_multi_open_then_orphan_in_that_order() {
        let a = Anomalies {
            open_stint_count: 2,
            orphaned_end_times: vec![t(18, 0)],
        };
        assert_eq!(
            a.detail_lines(),
            vec![
                "[!] 2 open stints for this date (unmatched starts)".to_string(),
                "[!] orphaned end at 18:00 (no matching start)".to_string(),
            ]
        );
    }

    #[test]
    fn t17_three_open_stints_reports_the_count() {
        let a = Anomalies {
            open_stint_count: 3,
            orphaned_end_times: vec![],
        };
        assert_eq!(
            a.detail_lines(),
            vec!["[!] 3 open stints for this date (unmatched starts)".to_string()]
        );
    }

    #[test]
    fn t18_three_orphans_are_never_coalesced() {
        let a = Anomalies {
            open_stint_count: 0,
            orphaned_end_times: vec![t(9, 15), t(13, 0), t(18, 0)],
        };
        assert_eq!(
            a.detail_lines(),
            vec![
                "[!] orphaned end at 09:15 (no matching start)".to_string(),
                "[!] orphaned end at 13:00 (no matching start)".to_string(),
                "[!] orphaned end at 18:00 (no matching start)".to_string(),
            ]
        );
    }

    #[test]
    fn t19_same_timestamp_orphans_are_distinct_not_deduplicated() {
        let a = Anomalies {
            open_stint_count: 0,
            orphaned_end_times: vec![t(13, 0), t(13, 0)],
        };
        assert_eq!(
            a.detail_lines(),
            vec![
                "[!] orphaned end at 13:00 (no matching start)".to_string(),
                "[!] orphaned end at 13:00 (no matching start)".to_string(),
            ]
        );
    }

    #[test]
    fn t20_one_open_stint_is_not_an_anomaly() {
        let a = Anomalies {
            open_stint_count: 1,
            orphaned_end_times: vec![],
        };
        assert!(a.detail_lines().is_empty());
    }

    #[test]
    fn t21_default_has_no_detail_lines() {
        assert!(Anomalies::default().detail_lines().is_empty());
    }

    #[test]
    fn t22_orphan_hour_is_zero_padded() {
        let a = Anomalies {
            open_stint_count: 0,
            orphaned_end_times: vec![t(7, 5)],
        };
        assert_eq!(
            a.detail_lines(),
            vec!["[!] orphaned end at 07:05 (no matching start)".to_string()]
        );
    }

    #[test]
    fn every_detail_line_starts_with_the_marker_and_has_no_trailing_whitespace() {
        let cases = [
            Anomalies {
                open_stint_count: 2,
                orphaned_end_times: vec![t(18, 0)],
            },
            Anomalies {
                open_stint_count: 0,
                orphaned_end_times: vec![t(9, 15), t(13, 0)],
            },
        ];
        for a in cases {
            for line in a.detail_lines() {
                assert!(line.starts_with("[!] "), "{line}");
                assert_eq!(line, line.trim_end(), "{line}");
                assert!(!line.contains('\n'), "{line}");
            }
        }
    }

    // ---- 4.9 has_any / row_marker ----

    #[test]
    fn t23_default_has_no_anomaly_and_no_marker() {
        let a = Anomalies::default();
        assert!(!a.has_any());
        assert_eq!(a.row_marker(), "");
    }

    #[test]
    fn t24_one_open_stint_has_no_anomaly_and_no_marker() {
        let a = Anomalies {
            open_stint_count: 1,
            orphaned_end_times: vec![],
        };
        assert!(!a.has_any());
        assert_eq!(a.row_marker(), "");
    }

    #[test]
    fn t25_two_open_stints_has_anomaly_and_marker() {
        let a = Anomalies {
            open_stint_count: 2,
            orphaned_end_times: vec![],
        };
        assert!(a.has_any());
        assert_eq!(a.row_marker(), "  [!]");
    }

    #[test]
    fn t26_one_orphan_has_anomaly_and_marker() {
        let a = Anomalies {
            open_stint_count: 0,
            orphaned_end_times: vec![t(18, 0)],
        };
        assert!(a.has_any());
        assert_eq!(a.row_marker(), "  [!]");
    }

    #[test]
    fn t27_one_open_plus_one_orphan_has_anomaly_and_marker() {
        let a = Anomalies {
            open_stint_count: 1,
            orphaned_end_times: vec![t(18, 0)],
        };
        assert!(a.has_any());
        assert_eq!(a.row_marker(), "  [!]");
    }

    #[test]
    fn t28_five_open_plus_two_orphans_has_anomaly_and_marker() {
        let a = Anomalies {
            open_stint_count: 5,
            orphaned_end_times: vec![t(1, 0), t(2, 0)],
        };
        assert!(a.has_any());
        assert_eq!(a.row_marker(), "  [!]");
    }

    // ---- 4.10 no-divergence invariant ----

    fn fixtures() -> Vec<Anomalies> {
        vec![
            Anomalies::default(),
            Anomalies {
                open_stint_count: 1,
                orphaned_end_times: vec![],
            },
            Anomalies {
                open_stint_count: 2,
                orphaned_end_times: vec![],
            },
            Anomalies {
                open_stint_count: 0,
                orphaned_end_times: vec![t(18, 0)],
            },
            Anomalies {
                open_stint_count: 1,
                orphaned_end_times: vec![t(18, 0)],
            },
            Anomalies {
                open_stint_count: 5,
                orphaned_end_times: vec![t(1, 0), t(2, 0)],
            },
        ]
    }

    #[test]
    fn t29_has_any_agrees_with_detail_lines_nonempty() {
        for a in fixtures() {
            assert_eq!(a.has_any(), !a.detail_lines().is_empty(), "{a:?}");
        }
    }

    #[test]
    fn t30_row_marker_empty_iff_not_has_any() {
        for a in fixtures() {
            assert_eq!(a.row_marker().is_empty(), !a.has_any(), "{a:?}");
        }
    }

    #[test]
    fn t31_exhaustive_sweep_of_open_count_and_orphan_length() {
        for open_stint_count in 0..=4usize {
            for orphan_len in 0..=3usize {
                let orphaned_end_times = (0..orphan_len).map(|i| t(i as u32, 0)).collect();
                let a = Anomalies {
                    open_stint_count,
                    orphaned_end_times,
                };
                assert_eq!(a.has_any(), !a.detail_lines().is_empty(), "{a:?}");
                assert_eq!(a.row_marker().is_empty(), !a.has_any(), "{a:?}");
            }
        }
    }

    // ---- 4.11 row composition sanity ----

    #[test]
    fn t32_row_marker_matches_the_spec_example_byte_for_byte() {
        let a = Anomalies {
            open_stint_count: 0,
            orphaned_end_times: vec![t(18, 0)],
        };
        assert_eq!(
            format!("  Wed 2026-02-11   08h 00m{}", a.row_marker()),
            "  Wed 2026-02-11   08h 00m  [!]"
        );
    }

    // ---- 4.12 plain-ASCII audit ----

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
        for a in fixtures() {
            for line in a.detail_lines() {
                assert!(line.is_ascii(), "{line}");
            }
            assert!(a.row_marker().is_ascii());
        }
    }
}
