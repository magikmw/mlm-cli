//! Per-date anomaly facts and their rendering (SPEC §4.3, §4.3.1, §7.3).
//!
//! One module builds the fact set for a date from raw punch buckets, so
//! `status`, `mlm week` rows and the carry scan share one predicate and
//! the row `[!]`, the block and the marker cannot disagree. Only this
//! module constructs [`DateAnomalies`] field by field. Everything here is
//! pure: no database, no clock beyond the injected `now_utc`.

use chrono::{DateTime, Local, NaiveDate, NaiveTime, TimeZone, Utc};

use crate::date::{format_date, format_date_with_weekday};
use crate::render;
use crate::stint::{self, DayStints, Stint};
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

    /// One entry per overlapping pair of stints, ordered by first start
    /// then second start (§3.2.1).
    pub overlaps: Vec<OverlapPair>,

    /// The local time of the first punch on the date strictly after the
    /// latest open start (§3.2.2). `None` when nothing follows or no start
    /// is open. Feeds the E7 and stale-start stop hints.
    pub latest_open_next: Option<NaiveTime>,
}

/// One stint's part on a single date. `start` and `end` bound that part and
/// drive the overlap test. `full_start` and `full_end` are the stint's real
/// times and drive the label, matching the status stint list. They equal
/// `start` and `end` unless the stint is spliced across midnight. `open`
/// marks today's running stint, whose `end` is `now`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StintSpan {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub full_start: DateTime<Utc>,
    pub full_end: DateTime<Utc>,
    pub open: bool,
}

impl StintSpan {
    /// A span whose label times equal its bounds.
    fn whole(start: DateTime<Utc>, end: DateTime<Utc>, open: bool) -> Self {
        Self {
            start,
            end,
            full_start: start,
            full_end: end,
            open,
        }
    }

    fn has_length(&self) -> bool {
        self.start < self.end
    }

    fn overlaps(&self, other: &Self) -> bool {
        self.start < other.end && other.start < self.end
    }

    /// The stint's real range, as the status stint list prints it.
    fn label(&self) -> String {
        let start = render::local_time(self.full_start).format("%H:%M");
        if self.open {
            format!("{start}-now")
        } else {
            format!(
                "{start}-{}",
                render::local_time(self.full_end).format("%H:%M")
            )
        }
    }
}

/// Two stints on one date whose ranges overlap. `first` starts no later
/// than `second`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverlapPair {
    pub first: StintSpan,
    pub second: StintSpan,
}

/// Every overlapping pair among the spans, ordered by first start then
/// second start. Zero-length spans are dropped. Spans that share a start
/// sort longest first, so the outer stint is `first`.
pub fn overlap_pairs(mut spans: Vec<StintSpan>) -> Vec<OverlapPair> {
    spans.retain(StintSpan::has_length);
    spans.sort_by_key(|s| (s.start, std::cmp::Reverse(s.end)));
    let mut out = Vec::new();
    for (i, first) in spans.iter().enumerate() {
        for second in &spans[i + 1..] {
            if first.overlaps(second) {
                out.push(OverlapPair {
                    first: *first,
                    second: *second,
                });
            }
        }
    }
    out
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
            overlaps: Vec::new(),
            latest_open_next: None,
        }
    }

    /// True iff this date has ANY anomaly: E7, E8, stale, overlap, or
    /// either refused-join direction. The single definition `week`'s
    /// per-row `[!]` and `status`'s block-or-not decision both key off.
    pub fn has_any(&self) -> bool {
        self.open_stint_count > 1
            || !self.overlaps.is_empty()
            || !self.orphaned_end_times.is_empty()
            || self.stale_open_start.is_some()
            || self.refused_join_with_previous.is_some()
            || self.refused_join_with_next.is_some()
    }

    /// `status`'s per-date block (§7.3), in the fixed order: E7, then
    /// stale, then E8 (ascending instant), then overlap (per pair), then
    /// refused-join-with-previous, then refused-join-with-next. Empty vec
    /// when `has_any()` is false. Does NOT include lookback lines.
    pub fn detail_lines(&self) -> Vec<String> {
        let mut out = Vec::new();
        out.extend(self.e7_lines());
        out.extend(self.stale_lines());
        out.extend(self.e8_lines());
        out.extend(self.overlap_lines());
        out.extend(self.refused_join_lines());
        out
    }

    fn overlap_lines(&self) -> Vec<String> {
        let d = format_date(self.date);
        self.overlaps
            .iter()
            .flat_map(|p| {
                [
                    format!(
                        "[!] overlapping stints {} and {} (the overlap counts twice)",
                        p.first.label(),
                        p.second.label()
                    ),
                    format!(
                        "    fix: mlm delete punch --date {d}  (lists punches; rerun with the number of the punch to remove)"
                    ),
                ]
            })
            .collect()
    }

    fn e7_lines(&self) -> Vec<String> {
        let Some(latest_time) = self.latest_open_start.filter(|_| self.open_stint_count > 1) else {
            return Vec::new();
        };
        let d = format_date(self.date);
        vec![
            format!(
                "[!] {} open stints for this date (only one can be open at a time)",
                self.open_stint_count
            ),
            format!(
                "    fix: mlm delete punch --date {d}  (lists punches; rerun with the extra start's number)"
            ),
            format!(
                "     or: mlm stop HH:MM --date {d}  {}",
                render::stop_time_hint_closes(latest_time, self.latest_open_next)
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
            format!(
                "    fix: mlm stop HH:MM --date {d}  {}",
                render::stop_time_hint(stale.time, self.latest_open_next)
            ),
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

/// The one fact builder. `now_utc` ends today's open stint for the overlap
/// test (an open stint shown as `HH:MM-now`).
pub fn build_facts(
    ctx: &DateContext,
    b: &DateBuckets<'_>,
    date: NaiveDate,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
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
        overlaps: overlap_pairs(date_spans(ctx, date, today, now_utc)),
        latest_open_next: latest_open_next(ctx, b),
    }
}

/// N after the latest open start (§3.2.2).
fn latest_open_next(ctx: &DateContext, b: &DateBuckets<'_>) -> Option<NaiveTime> {
    let open = ctx.day.open.last()?;
    render::next_punch_after(b.day, &open.start)
}

/// Local midnight at the start of `date`, as a UTC instant. A zone whose
/// midnight falls in a DST gap uses the date's naive midnight read as UTC.
fn local_midnight(date: NaiveDate) -> DateTime<Utc> {
    let naive = date.and_time(NaiveTime::MIN);
    Local
        .from_local_datetime(&naive)
        .earliest()
        .map_or_else(|| naive.and_utc(), |t| t.with_timezone(&Utc))
}

fn day_end(date: NaiveDate) -> DateTime<Utc> {
    local_midnight(date.succ_opt().unwrap_or(date))
}

/// A completed stint's part on `date`. Only a spliced stint ends on a later
/// date. Its bounds stop at the date's end and its label keeps the real end.
fn closed_span(stint: &Stint, date: NaiveDate) -> StintSpan {
    let (start, full_end) = (stint.start.at_utc, stint.end.at_utc);
    let end = if stint.end.date > date {
        day_end(date)
    } else {
        full_end
    };
    StintSpan {
        start,
        end,
        full_start: start,
        full_end,
        open: false,
    }
}

/// Today's most recent open stint, up to now. Past dates and a start at or
/// after now give nothing (a zero-minute stint is never flagged).
fn live_span(
    ctx: &DateContext,
    date: NaiveDate,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
) -> Option<StintSpan> {
    if date != today {
        return None;
    }
    let start = ctx.day.open.last()?.start.at_utc;
    (start < now_utc).then_some(StintSpan::whole(start, now_utc, true))
}

/// The spans one date compares. A stint spliced in from the previous date
/// adds none here. Its orphaned end is the date's first punch, so no other
/// stint on the date can start before the end and overlap the part from
/// local midnight.
fn date_spans(
    ctx: &DateContext,
    date: NaiveDate,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
) -> Vec<StintSpan> {
    let mut spans: Vec<StintSpan> = ctx
        .day
        .completed
        .iter()
        .map(|s| closed_span(s, date))
        .collect();
    spans.extend(live_span(ctx, date, today, now_utc));
    spans
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
            overlaps: Vec::new(),
            latest_open_next: None,
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

#[cfg(test)]
mod overlap_tests {
    use super::*;
    use crate::storage::{PunchKind, punch_from_local};

    fn past() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 2, 1).unwrap()
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 2, 12).unwrap()
    }

    fn t(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    fn pk(id: i64, kind: PunchKind, date: NaiveDate, h: u32, m: u32) -> Punch {
        punch_from_local(id, kind, date, t(h, m), &Local).unwrap()
    }

    fn st(id: i64, date: NaiveDate, h: u32, m: u32) -> Punch {
        pk(id, PunchKind::Start, date, h, m)
    }

    fn en(id: i64, date: NaiveDate, h: u32, m: u32) -> Punch {
        pk(id, PunchKind::End, date, h, m)
    }

    fn at(date: NaiveDate, h: u32, m: u32) -> DateTime<Utc> {
        st(0, date, h, m).at_utc
    }

    fn now_at(date: NaiveDate, h: u32, m: u32) -> DateTime<Utc> {
        at(date, h, m)
    }

    fn span(sh: u32, sm: u32, eh: u32, em: u32) -> StintSpan {
        StintSpan {
            start: at(past(), sh, sm),
            end: at(past(), eh, em),
            full_start: at(past(), sh, sm),
            full_end: at(past(), eh, em),
            open: false,
        }
    }

    fn starts(pairs: &[OverlapPair]) -> Vec<(DateTime<Utc>, DateTime<Utc>)> {
        pairs
            .iter()
            .map(|p| (p.first.start, p.second.start))
            .collect()
    }

    fn facts_on(
        prev: &[Punch],
        day: &[Punch],
        next: &[Punch],
        date: NaiveDate,
        now: DateTime<Utc>,
    ) -> DateAnomalies {
        let b = DateBuckets { prev, day, next };
        date_facts(&b, date, today(), now)
    }

    fn past_facts(day: &[Punch]) -> DateAnomalies {
        facts_on(&[], day, &[], past(), now_at(today(), 18, 0))
    }

    // ---- overlap_pairs ----

    #[test]
    fn nested_pair_has_outer_first() {
        let pairs = overlap_pairs(vec![span(10, 0, 11, 0), span(9, 0, 12, 0)]);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].first, span(9, 0, 12, 0));
        assert_eq!(pairs[0].second, span(10, 0, 11, 0));
    }

    #[test]
    fn identical_ranges_are_one_pair() {
        let pairs = overlap_pairs(vec![span(9, 0, 10, 0), span(9, 0, 10, 0)]);
        assert_eq!(pairs.len(), 1);
    }

    #[test]
    fn shared_start_puts_the_longer_stint_first() {
        let pairs = overlap_pairs(vec![span(9, 0, 11, 0), span(9, 0, 12, 0)]);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].first, span(9, 0, 12, 0));
    }

    #[test]
    fn shared_end_is_one_pair() {
        let pairs = overlap_pairs(vec![span(9, 0, 12, 0), span(10, 0, 12, 0)]);
        assert_eq!(pairs.len(), 1);
    }

    #[test]
    fn touching_stints_do_not_overlap() {
        assert!(overlap_pairs(vec![span(9, 0, 10, 0), span(10, 0, 11, 0)]).is_empty());
    }

    #[test]
    fn disjoint_stints_do_not_overlap() {
        assert!(overlap_pairs(vec![span(9, 0, 10, 0), span(11, 0, 12, 0)]).is_empty());
    }

    #[test]
    fn zero_length_span_is_dropped() {
        assert!(overlap_pairs(vec![span(9, 0, 12, 0), span(10, 0, 10, 0)]).is_empty());
    }

    #[test]
    fn three_nested_spans_give_three_pairs_in_order() {
        let pairs = overlap_pairs(vec![
            span(10, 0, 11, 0),
            span(9, 0, 12, 0),
            span(9, 30, 11, 30),
        ]);
        let a = at(past(), 9, 0);
        let b = at(past(), 9, 30);
        let c = at(past(), 10, 0);
        assert_eq!(starts(&pairs), vec![(a, b), (a, c), (b, c)]);
    }

    #[test]
    fn four_nested_spans_give_six_pairs() {
        let pairs = overlap_pairs(vec![
            span(9, 0, 12, 0),
            span(9, 10, 11, 50),
            span(9, 20, 11, 40),
            span(9, 30, 11, 30),
        ]);
        assert_eq!(pairs.len(), 6);
    }

    // ---- facts over punches ----

    #[test]
    fn closed_nest_on_a_past_date_is_one_overlap() {
        let d = past();
        let a = past_facts(&[
            st(1, d, 9, 0),
            st(2, d, 10, 0),
            en(3, d, 11, 0),
            en(4, d, 12, 0),
        ]);
        assert_eq!(a.overlaps.len(), 1);
        assert!(a.has_any());
    }

    #[test]
    fn todays_open_stint_containing_a_closed_one_overlaps() {
        let d = today();
        let day = [st(1, d, 9, 0), st(2, d, 10, 0), en(3, d, 11, 0)];
        let a = facts_on(&[], &day, &[], d, now_at(d, 12, 0));
        assert_eq!(a.overlaps.len(), 1);
        assert!(a.overlaps[0].first.open);
        assert_eq!(a.overlaps[0].first.label(), "09:00-now");
        assert_eq!(a.overlaps[0].second.label(), "10:00-11:00");
    }

    #[test]
    fn open_stint_on_a_past_date_is_excluded() {
        let d = past();
        let a = past_facts(&[st(1, d, 9, 0), st(2, d, 10, 0), en(3, d, 11, 0)]);
        assert!(a.overlaps.is_empty());
    }

    #[test]
    fn live_span_needs_now_after_the_start() {
        let d = today();
        let day = [st(1, d, 9, 0), en(2, d, 12, 0), st(3, d, 14, 0)];
        assert!(
            facts_on(&[], &day, &[], d, now_at(d, 13, 0))
                .overlaps
                .is_empty()
        );
        assert!(
            facts_on(&[], &day, &[], d, now_at(d, 14, 0))
                .overlaps
                .is_empty()
        );
        let nested = [st(1, d, 9, 0), en(2, d, 15, 0), st(3, d, 14, 0)];
        assert_eq!(
            facts_on(&[], &nested, &[], d, now_at(d, 16, 0))
                .overlaps
                .len(),
            1
        );
    }

    #[test]
    fn only_the_latest_open_start_enters_the_spans() {
        let d = today();
        let two_open = [st(1, d, 9, 0), st(2, d, 10, 0)];
        let a = facts_on(&[], &two_open, &[], d, now_at(d, 12, 0));
        assert_eq!(a.open_stint_count, 2);
        assert!(a.overlaps.is_empty());
    }

    #[test]
    fn spliced_stint_start_side_uses_the_part_on_the_date() {
        let d = past();
        let next_day = d.succ_opt().unwrap();
        let day = [st(1, d, 22, 30), st(2, d, 23, 0), en(3, d, 23, 30)];
        let next = [en(4, next_day, 1, 0)];
        let a = facts_on(&[], &day, &next, d, now_at(today(), 18, 0));
        assert_eq!(a.overlaps.len(), 1);
        let first = a.overlaps[0].first;
        assert_eq!(first.end, local_midnight(next_day));
        assert_eq!(first.label(), "22:30-01:00");
    }

    #[test]
    fn spliced_stint_adds_nothing_on_the_receiving_date() {
        let d = past();
        let next_day = d.succ_opt().unwrap();
        let prev = [st(1, d, 23, 0)];
        let day = [
            en(2, next_day, 1, 0),
            st(3, next_day, 2, 0),
            en(4, next_day, 3, 0),
        ];
        let a = facts_on(&prev, &day, &[], next_day, now_at(today(), 18, 0));
        assert!(a.overlaps.is_empty());
        assert!(a.orphaned_end_times.is_empty());
    }

    #[test]
    fn spliced_label_keeps_real_times_in_the_line() {
        let d = past();
        let next_day = d.succ_opt().unwrap();
        let day = [st(1, d, 23, 30), st(2, d, 23, 40), en(3, d, 23, 50)];
        let next = [en(4, next_day, 0, 45)];
        let a = facts_on(&[], &day, &next, d, now_at(today(), 18, 0));
        assert_eq!(
            a.detail_lines()[0],
            "[!] overlapping stints 23:30-00:45 and 23:40-23:50 (the overlap counts twice)"
        );
    }

    // ---- rendering ----

    fn nested_fixture() -> DateAnomalies {
        let d = past();
        past_facts(&[
            st(1, d, 9, 0),
            st(2, d, 10, 0),
            en(3, d, 11, 0),
            en(4, d, 12, 0),
        ])
    }

    #[test]
    fn overlap_lines_text() {
        assert_eq!(
            nested_fixture().detail_lines(),
            vec![
                "[!] overlapping stints 09:00-12:00 and 10:00-11:00 (the overlap counts twice)"
                    .to_string(),
                "    fix: mlm delete punch --date 2026-02-01  (lists punches; rerun with the number of the punch to remove)"
                    .to_string(),
            ]
        );
    }

    #[test]
    fn three_nested_stints_give_three_line_groups() {
        let d = past();
        let a = past_facts(&[
            st(1, d, 9, 0),
            st(2, d, 9, 30),
            st(3, d, 10, 0),
            en(4, d, 11, 0),
            en(5, d, 11, 30),
            en(6, d, 12, 0),
        ]);
        let lines = a.detail_lines();
        assert_eq!(lines.len(), 6);
        for chunk in lines.chunks(2) {
            assert!(chunk[0].starts_with("[!] overlapping stints"));
            assert!(chunk[1].starts_with("    fix: mlm delete punch"));
        }
    }

    #[test]
    fn block_order_puts_overlap_between_e8_and_refused_joins() {
        let pair = nested_fixture().overlaps[0];
        let a = DateAnomalies {
            date: past(),
            open_stint_count: 2,
            latest_open_start: Some(t(14, 0)),
            orphaned_end_times: vec![t(10, 0)],
            stale_open_start: Some(StaleOpenStart {
                time: t(9, 0),
                age_days: 5,
            }),
            refused_join_with_previous: Some(past().pred_opt().unwrap()),
            refused_join_with_next: Some(past().succ_opt().unwrap()),
            overlaps: vec![pair],
            latest_open_next: None,
        };
        let lines = a.detail_lines();
        let heads: Vec<&str> = lines
            .iter()
            .filter(|l| l.starts_with("[!]"))
            .map(|l| &l[..l.len().min(14)])
            .collect();
        assert_eq!(
            heads,
            vec![
                "[!] 2 open sti",
                "[!] unclosed s",
                "[!] orphaned e",
                "[!] overlappin",
                "[!] not joined",
                "[!] not joined",
            ]
        );
        assert!(lines.iter().all(|l| l.is_ascii()));
    }

    #[test]
    fn overlap_only_fact_marks_the_row() {
        let a = DateAnomalies {
            overlaps: nested_fixture().overlaps,
            ..DateAnomalies::none(past())
        };
        assert!(a.has_any());
        assert_eq!(a.row_marker(), "  [!]");
    }

    #[test]
    fn has_any_matches_block_for_overlap_fixtures() {
        let d = past();
        let cases = [
            nested_fixture(),
            past_facts(&[
                st(1, d, 9, 0),
                en(2, d, 10, 0),
                st(3, d, 10, 0),
                en(4, d, 11, 0),
            ]),
            past_facts(&[st(1, d, 9, 0), st(2, d, 10, 0), en(3, d, 11, 0)]),
        ];
        for a in cases {
            assert_eq!(a.has_any(), !a.detail_lines().is_empty(), "{a:?}");
        }
    }

    // ---- clean-pair stop hints ----

    fn fix_line(a: &DateAnomalies) -> String {
        a.detail_lines()
            .into_iter()
            .find(|l| l.contains("mlm stop HH:MM"))
            .expect("a stop hint line")
    }

    fn stale_fixture() -> Vec<Punch> {
        let d = past();
        vec![st(1, d, 9, 0), st(2, d, 10, 0), en(3, d, 10, 30)]
    }

    fn e7_fixture() -> Vec<Punch> {
        let d = past();
        vec![
            st(1, d, 9, 0),
            st(2, d, 14, 0),
            st(3, d, 15, 0),
            en(4, d, 15, 30),
        ]
    }

    #[test]
    fn stale_without_later_punch_is_unchanged() {
        let a = past_facts(&[st(1, past(), 9, 0)]);
        assert_eq!(
            fix_line(&a),
            "    fix: mlm stop HH:MM --date 2026-02-01  (a time after 09:00)"
        );
        assert_eq!(a.latest_open_next, None);
    }

    #[test]
    fn stale_with_later_punches_names_the_window() {
        let a = past_facts(&stale_fixture());
        assert_eq!(
            fix_line(&a),
            "    fix: mlm stop HH:MM --date 2026-02-01  (a time between 09:00 and 10:00)"
        );
    }

    #[test]
    fn window_ends_at_the_first_later_punch_of_any_kind() {
        let d = past();
        let a = past_facts(&[
            st(1, d, 9, 0),
            st(2, d, 11, 0),
            en(3, d, 11, 30),
            st(4, d, 10, 0),
            en(5, d, 10, 15),
        ]);
        assert_eq!(a.latest_open_next, Some(t(10, 0)));
        assert!(fix_line(&a).ends_with("(a time between 09:00 and 10:00)"));
    }

    #[test]
    fn punch_at_the_starts_own_minute_does_not_count() {
        let d = past();
        let a = past_facts(&[st(1, d, 9, 0), st(2, d, 9, 0)]);
        assert_eq!(a.latest_open_next, None);
        assert_eq!(
            fix_line(&a),
            "     or: mlm stop HH:MM --date 2026-02-01  (a time after 09:00 closes the 09:00 start)"
        );
    }

    #[test]
    fn clean_day_has_no_latest_open_next() {
        let d = past();
        let a = past_facts(&[st(1, d, 9, 0), en(2, d, 17, 0)]);
        assert_eq!(a.latest_open_next, None);
    }

    #[test]
    fn e7_without_later_punch_is_unchanged() {
        let d = past();
        let a = past_facts(&[st(1, d, 9, 0), st(2, d, 14, 0)]);
        assert_eq!(
            fix_line(&a),
            "     or: mlm stop HH:MM --date 2026-02-01  (a time after 14:00 closes the 14:00 start)"
        );
    }

    #[test]
    fn e7_with_later_punch_names_the_window() {
        let a = past_facts(&e7_fixture());
        assert_eq!(
            fix_line(&a),
            "     or: mlm stop HH:MM --date 2026-02-01  (a time between 14:00 and 15:00 closes the 14:00 start)"
        );
    }

    // ---- follow-the-hint round trips ----

    fn with_end(base: Vec<Punch>, h: u32, m: u32) -> DateAnomalies {
        let mut day = base;
        day.push(en(99, past(), h, m));
        past_facts(&day)
    }

    #[test]
    fn stale_hint_window_never_creates_an_overlap() {
        for (h, m) in [(9, 1), (9, 30), (10, 0)] {
            let a = with_end(stale_fixture(), h, m);
            assert!(a.overlaps.is_empty(), "end {h}:{m} gave {a:?}");
        }
    }

    #[test]
    fn e7_hint_window_never_creates_an_overlap() {
        for (h, m) in [(14, 1), (14, 30), (15, 0)] {
            let a = with_end(e7_fixture(), h, m);
            assert!(a.overlaps.is_empty(), "end {h}:{m} gave {a:?}");
        }
    }

    #[test]
    fn adjacent_minute_window_holds() {
        let d = past();
        let base = vec![st(1, d, 18, 2), st(2, d, 18, 3), en(3, d, 18, 30)];
        let a = past_facts(&base);
        assert!(fix_line(&a).contains("between 18:02 and 18:03"));
        assert!(with_end(base, 18, 3).overlaps.is_empty());
    }

    #[test]
    fn a_stop_after_the_window_creates_the_overlap() {
        let a = with_end(e7_fixture(), 16, 0);
        assert_eq!(a.overlaps.len(), 1);
    }
}
