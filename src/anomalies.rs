//! Per-date anomaly facts and their rendering (SPEC §4.3, §4.3.1, §7.3).
//!
//! One module builds the fact set for a date from raw punch buckets, so
//! `status`, `mlm week` rows and the carry scan share one predicate and
//! the row `[!]`, the block and the marker cannot disagree. Only this
//! module constructs [`DateAnomalies`] field by field. Everything here is
//! pure: no database, no clock beyond the injected `now_utc`.

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};

use crate::date::{format_date, format_date_with_weekday};
use crate::render;
use crate::stint::{self, DayStints};
use crate::storage::Punch;

/// Raw punch buckets for one date and its two calendar neighbours. Each
/// slice holds punches of exactly one date, or is empty.
#[derive(Debug, Clone, Copy)]
pub struct DateBuckets<'a> {
    pub prev: &'a [Punch],
    pub day: &'a [Punch],
    pub next: &'a [Punch],
}

/// Every classification one date needs: the splice-aware view plus the
/// three raw single-date views the refused-join predicate reads.
pub struct DateContext {
    pub day: DayStints,
    pub prev_raw: DayStints,
    pub day_raw: DayStints,
    pub next_raw: DayStints,
}

/// Everything one calendar date's anomaly rendering needs, as plain data
/// (SPEC.md §4.3/§4.3.1/§7.3).
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
    /// A date with no anomaly. Fixtures extend it with struct update
    /// syntax, so a field added later never breaks them.
    pub fn none(date: NaiveDate) -> Self {
        Self {
            date,
            open_stint_count: 0,
            latest_open_start: None,
            orphaned_end_times: Vec::new(),
            stale_open_start: None,
            refused_join_with_previous: None,
            refused_join_with_next: None,
        }
    }

    /// True iff this date has ANY anomaly: E7, E8, stale, or either
    /// refused-join direction. The single definition `week`'s per-row
    /// `[!]` and `status`'s block-or-not decision both key off.
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
    /// `has_any()` is false. Does NOT include lookback lines.
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

/// Classifies the date's three buckets: the splice-aware day view and the
/// three raw views.
pub fn classify_context(b: &DateBuckets<'_>, now_utc: DateTime<Utc>) -> DateContext {
    DateContext {
        day: stint::classify_at(b.prev, b.day, b.next, now_utc),
        prev_raw: stint::classify(b.prev, now_utc),
        day_raw: stint::classify(b.day, now_utc),
        next_raw: stint::classify(b.next, now_utc),
    }
}

/// The one fact builder. `_now_utc` is reserved for facts that read the
/// clock (an open stint shown as `HH:MM-now`).
pub fn build_facts(
    ctx: &DateContext,
    b: &DateBuckets<'_>,
    date: NaiveDate,
    today: NaiveDate,
    _now_utc: DateTime<Utc>,
) -> DateAnomalies {
    DateAnomalies {
        date,
        open_stint_count: ctx.day.open.len(),
        latest_open_start: ctx
            .day
            .open
            .last()
            .map(|o| render::local_time(o.start.at_utc)),
        orphaned_end_times: orphan_times(&ctx.day),
        stale_open_start: stale_fact(&ctx.day, date, today),
        refused_join_with_previous: refused_neighbor(
            &ctx.prev_raw,
            &ctx.day_raw,
            b.day,
            date.pred_opt(),
        ),
        refused_join_with_next: refused_neighbor(
            &ctx.day_raw,
            &ctx.next_raw,
            b.next,
            date.succ_opt(),
        ),
    }
}

/// `classify_context` then `build_facts`.
pub fn date_facts(
    b: &DateBuckets<'_>,
    date: NaiveDate,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
) -> DateAnomalies {
    let ctx = classify_context(b, now_utc);
    build_facts(&ctx, b, date, today, now_utc)
}

fn orphan_times(day: &DayStints) -> Vec<NaiveTime> {
    day.orphaned_ends
        .iter()
        .map(|o| render::local_time(o.punch.at_utc))
        .collect()
}

fn stale_fact(day: &DayStints, date: NaiveDate, today: NaiveDate) -> Option<StaleOpenStart> {
    stint::stale_open_start(day, date, today).map(|s| StaleOpenStart {
        time: render::local_time(s.at),
        age_days: s.age_days,
    })
}

/// One direction of the refused-join check. `earlier` has the open
/// start(s), `later` the orphaned end(s). Returns the neighbour date to
/// name when the join is refused. Both views are raw single-date
/// classifications, never the splice-resolved one.
fn refused_neighbor(
    earlier: &DayStints,
    later: &DayStints,
    later_punches: &[Punch],
    neighbor: Option<NaiveDate>,
) -> Option<NaiveDate> {
    stint::is_refused_join(earlier, later, later_punches)
        .then_some(neighbor)
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{PunchKind, punch_from_local};
    use chrono::{Local, TimeZone};

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("test fixture is a real date")
    }

    fn t(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).expect("test fixture is a real time")
    }

    fn punch(id: i64, kind: PunchKind, date: NaiveDate, h: u32, m: u32) -> Punch {
        punch_from_local(id, kind, date, t(h, m), &Local).expect("test punch is a real local time")
    }

    fn start(id: i64, date: NaiveDate, h: u32, m: u32) -> Punch {
        punch(id, PunchKind::Start, date, h, m)
    }

    fn end(id: i64, date: NaiveDate, h: u32, m: u32) -> Punch {
        punch(id, PunchKind::End, date, h, m)
    }

    /// Thu 2026-02-12 18:00 local.
    fn now() -> DateTime<Utc> {
        Local
            .from_local_datetime(&d(2026, 2, 12).and_hms_opt(18, 0, 0).unwrap())
            .unwrap()
            .with_timezone(&Utc)
    }

    fn today() -> NaiveDate {
        d(2026, 2, 12)
    }

    fn facts(prev: &[Punch], day: &[Punch], next: &[Punch], date: NaiveDate) -> DateAnomalies {
        let b = DateBuckets { prev, day, next };
        date_facts(&b, date, today(), now())
    }

    // ---- rendering (moved from render.rs, text unchanged) ----

    #[test]
    fn t_e7_reworded_matches_spec_example() {
        let a = DateAnomalies {
            open_stint_count: 2,
            latest_open_start: Some(t(14, 0)),
            ..DateAnomalies::none(d(2026, 9, 21))
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
            ..DateAnomalies::none(d(2026, 9, 21))
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
            ..DateAnomalies::none(d(2026, 9, 21))
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
            ..DateAnomalies::none(d(2026, 9, 22))
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
            ..DateAnomalies::none(d(2026, 9, 21))
        };
        assert_eq!(
            a.detail_lines(),
            vec![
                "[!] not joined to Tue 2026-09-22 across midnight (a join needs exactly one open start on Mon 2026-09-21 and exactly one unmatched end, as Tue 2026-09-22's first punch)".to_string(),
            ]
        );
    }

    #[test]
    fn t_ordering_all_kinds_at_once() {
        // Unrealistic combination (stale + E7 together) is impossible
        // under the real stale/E7 predicate, but this type enforces no
        // such invariant -- this test only pins detail_lines()'s ordering.
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
    fn t_date_anomalies_new_fixtures_never_contain_old_e7_suffix() {
        let fixtures = [
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
        ];
        for a in fixtures {
            for line in a.detail_lines() {
                assert!(!line.contains("(unmatched starts)"), "{line}");
            }
        }
    }

    // ---- DateAnomalies::none ----

    #[test]
    fn none_is_a_date_with_no_anomaly() {
        let a = DateAnomalies::none(d(2026, 2, 12));
        assert_eq!(a.date, d(2026, 2, 12));
        assert!(!a.has_any());
        assert!(a.detail_lines().is_empty());
        assert_eq!(a.row_marker(), "");
    }

    #[test]
    fn row_marker_is_set_when_any_anomaly() {
        let a = DateAnomalies {
            orphaned_end_times: vec![t(18, 0)],
            ..DateAnomalies::none(d(2026, 2, 12))
        };
        assert_eq!(a.row_marker(), "  [!]");
    }

    // ---- build_facts / date_facts over hand-built buckets ----

    #[test]
    fn facts_e7_two_open_starts() {
        let day = [start(1, today(), 9, 0), start(2, today(), 14, 0)];
        let a = facts(&[], &day, &[], today());
        assert_eq!(a.open_stint_count, 2);
        assert_eq!(a.latest_open_start, Some(t(14, 0)));
        assert_eq!(a.stale_open_start, None);
        assert!(a.detail_lines()[0].starts_with("[!] 2 open stints"));
    }

    #[test]
    fn facts_latest_open_start_is_the_instant_latest() {
        // Ids run opposite to the clock, so only the instant decides.
        let day = [start(5, today(), 9, 0), start(1, today(), 14, 0)];
        let a = facts(&[], &day, &[], today());
        assert_eq!(a.latest_open_start, Some(t(14, 0)));
    }

    #[test]
    fn facts_stale_single_open_start_at_age_two() {
        let date = d(2026, 2, 10);
        let a = facts(&[], &[start(1, date, 9, 0)], &[], date);
        assert_eq!(
            a.stale_open_start,
            Some(StaleOpenStart {
                time: t(9, 0),
                age_days: 2
            })
        );
        assert_eq!(a.open_stint_count, 1);
    }

    #[test]
    fn facts_stale_keeps_growing_past_the_lookback_window() {
        let date = d(2026, 1, 28);
        let a = facts(&[], &[start(1, date, 9, 0)], &[], date);
        assert_eq!(a.stale_open_start.map(|s| s.age_days), Some(15));
    }

    #[test]
    fn facts_open_start_yesterday_is_not_stale() {
        let date = d(2026, 2, 11);
        let a = facts(&[], &[start(1, date, 9, 0)], &[], date);
        assert_eq!(a.stale_open_start, None);
        assert!(!a.has_any());
    }

    #[test]
    fn facts_orphaned_end() {
        let day = [end(1, today(), 18, 0)];
        let a = facts(&[], &day, &[], today());
        assert_eq!(a.orphaned_end_times, vec![t(18, 0)]);
        assert!(a.detail_lines()[0].starts_with("[!] orphaned end at 18:00"));
    }

    #[test]
    fn facts_refused_join_names_previous_on_the_later_date() {
        let yesterday = d(2026, 2, 11);
        let prev = [start(1, yesterday, 9, 0), start(2, yesterday, 11, 0)];
        let day = [end(3, today(), 8, 0)];
        let a = facts(&prev, &day, &[], today());
        assert_eq!(a.refused_join_with_previous, Some(yesterday));
        assert_eq!(a.refused_join_with_next, None);
    }

    #[test]
    fn facts_refused_join_names_next_on_the_earlier_date() {
        let yesterday = d(2026, 2, 11);
        let day = [start(1, yesterday, 9, 0), start(2, yesterday, 11, 0)];
        let next = [end(3, today(), 8, 0)];
        let a = facts(&[], &day, &next, yesterday);
        assert_eq!(a.refused_join_with_next, Some(today()));
        assert_eq!(a.refused_join_with_previous, None);
    }

    #[test]
    fn facts_splice_is_not_an_anomaly_on_either_date() {
        let yesterday = d(2026, 2, 11);
        let prev = [start(1, yesterday, 22, 0)];
        let day = [end(2, today(), 1, 0)];
        assert!(!facts(&prev, &day, &[], today()).has_any());
        assert!(!facts(&[], &prev, &day, yesterday).has_any());
    }

    #[test]
    fn facts_clean_day_has_no_anomaly() {
        let day = [start(1, today(), 9, 0), end(2, today(), 17, 0)];
        let a = facts(&[], &day, &[], today());
        assert_eq!(a, DateAnomalies::none(today()));
    }

    #[test]
    fn has_any_matches_a_nonempty_block_over_every_fixture() {
        let yesterday = d(2026, 2, 11);
        let old = d(2026, 2, 10);
        let cases = [
            facts(
                &[],
                &[start(1, today(), 9, 0), start(2, today(), 14, 0)],
                &[],
                today(),
            ),
            facts(&[], &[start(1, old, 9, 0)], &[], old),
            facts(&[], &[end(1, today(), 18, 0)], &[], today()),
            facts(
                &[start(1, yesterday, 9, 0), start(2, yesterday, 11, 0)],
                &[end(3, today(), 8, 0)],
                &[],
                today(),
            ),
            facts(
                &[],
                &[start(1, yesterday, 9, 0), start(2, yesterday, 11, 0)],
                &[end(3, today(), 8, 0)],
                yesterday,
            ),
            facts(
                &[start(1, yesterday, 22, 0)],
                &[end(2, today(), 1, 0)],
                &[],
                today(),
            ),
            facts(
                &[],
                &[start(1, today(), 9, 0), end(2, today(), 17, 0)],
                &[],
                today(),
            ),
            DateAnomalies::none(today()),
        ];
        for a in cases {
            assert_eq!(a.has_any(), !a.detail_lines().is_empty(), "{a:?}");
            assert_eq!(a.row_marker().is_empty(), !a.has_any(), "{a:?}");
        }
    }

    #[test]
    fn date_facts_equals_classify_context_then_build_facts() {
        let day = [start(1, today(), 9, 0), start(2, today(), 14, 0)];
        let b = DateBuckets {
            prev: &[],
            day: &day,
            next: &[],
        };
        let ctx = classify_context(&b, now());
        assert_eq!(
            build_facts(&ctx, &b, today(), today(), now()),
            date_facts(&b, today(), today(), now())
        );
    }
}
