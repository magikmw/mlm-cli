//! Stint pairing (SPEC.md §4.3): turns one calendar date's punches into
//! completed stints, open stints, and anomalies via nearest-match (LIFO)
//! parentheses matching.
//!
//! Pure logic only: no DB access, no rendering, and no clock reads — the
//! current instant is injected (PLAN.md contract 6).

// Milestone 5 is built ahead of its consumers (Milestones 9/10/11), so the
// public surface here has no in-crate caller yet outside the tests.
#![allow(dead_code)]

use chrono::{DateTime, Utc};

use crate::storage::{Punch, PunchKind};

/// A completed (start, end) pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stint {
    pub start: Punch,
    pub end: Punch,
    /// Whole minutes from `start.at_utc` to `end.at_utc`. Always >= 0
    /// (guaranteed by the sort). 0 for a same-instant pair (E14).
    pub minutes: i64,
}

/// A trailing unmatched `start`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenStint {
    pub start: Punch,
    /// Whole minutes from `start.at_utc` to the supplied `now`, clamped at 0.
    /// Live figure — SPEC.md §2.4 forbids it from entering any day/week/carry
    /// total.
    pub minutes_so_far: i64,
}

/// An `end` that had no unmatched `start` to pair with (E8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrphanedEnd {
    /// The offending punch itself; never coalesced with another orphan.
    pub punch: Punch,
}

/// A renderable anomaly, in the two shapes SPEC.md §7.3 defines.
/// Milestone 5 produces the data; Milestone 9 produces the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anomaly {
    /// More than one trailing unmatched start (§4.3, E7).
    /// `count` == `DayStints::open.len()`, always >= 2 when present.
    MultipleOpenStints { count: usize },
    /// One per orphaned end (§4.3, E8), in ascending instant order.
    OrphanedEnd { punch: Punch },
}

/// Everything this milestone produces for one calendar date
/// (PLAN.md contract 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayStints {
    /// Completed stints, ascending by `end` instant (emission order). The
    /// only source of worked minutes (§2.4).
    pub completed: Vec<Stint>,
    /// Trailing unmatched starts, ascending by start instant. len 0 = nothing
    /// open; len 1 = the ordinary open stint (not an anomaly); len >= 2 = E7.
    pub open: Vec<OpenStint>,
    /// One entry per orphaned end, ascending by instant (E8).
    pub orphaned_ends: Vec<OrphanedEnd>,
    /// Precomputed at construction so consumers pay O(1) and there is exactly
    /// one definition of "has an anomaly".
    has_anomaly: bool,
}

impl DayStints {
    /// The one definition of "has an anomaly": more than one trailing open
    /// stint, or at least one orphaned end. Used both when `classify()`
    /// first builds a `DayStints` and when `classify_at()` recomputes the
    /// field after a boundary splice mutates `open`/`orphaned_ends`, so the
    /// two can never drift apart.
    fn compute_has_anomaly(open: &[OpenStint], orphaned_ends: &[OrphanedEnd]) -> bool {
        open.len() > 1 || !orphaned_ends.is_empty()
    }

    /// Cheap O(1) has-any-anomaly signal (Milestone 11's per-row `[!]`).
    pub fn has_anomaly(&self) -> bool {
        self.has_anomaly
    }

    /// Renderable-detail anomaly list (Milestone 10's status output).
    ///
    /// Emission order is fixed: `MultipleOpenStints` first (when more than one
    /// stint is open), then one `OrphanedEnd` per orphan in ascending-instant
    /// order — matching §7.3's example block.
    pub fn anomalies(&self) -> Vec<Anomaly> {
        let mut out = Vec::new();
        if self.open.len() > 1 {
            out.push(Anomaly::MultipleOpenStints {
                count: self.open.len(),
            });
        }
        out.extend(
            self.orphaned_ends
                .iter()
                .map(|o| Anomaly::OrphanedEnd { punch: o.punch }),
        );
        out
    }

    /// Sum of completed stint minutes: the day total. Excludes open-stint live
    /// time and orphaned ends by construction (§2.4).
    pub fn completed_minutes(&self) -> i64 {
        self.completed.iter().map(|s| s.minutes).sum()
    }

    /// True when at least one stint is open — distinct from `has_anomaly()`.
    pub fn is_ongoing(&self) -> bool {
        !self.open.is_empty()
    }
}

/// Pair one calendar date's punches into stints (SPEC.md §4.3).
///
/// `punches` — all punches for a single date, in any order; the ordering used
/// for pairing is derived here, not trusted from the caller.
/// `now` — the injected current instant (PLAN.md contract 6). Never read from
/// the system clock inside this module.
pub fn classify(punches: &[Punch], now: DateTime<Utc>) -> DayStints {
    debug_assert!(
        punches.iter().all(|q| q.date == punches[0].date),
        "classify() expects all punches to share one date"
    );

    // Step 1: order the date's punches (SPEC.md §4.3 step 1). Stable sort, so
    // the result stays deterministic even for duplicate keys.
    let mut sorted: Vec<Punch> = punches.to_vec();
    // Ties at an identical instant break first by kind (`Start` before `End`,
    // which is `PunchKind`'s own `Ord`), then by `id` (insertion order). The
    // kind tie-break is what makes a same-instant pair match cleanly (E14)
    // regardless of which of the two was typed first.
    sorted.sort_by_key(|q| (q.at_utc, q.kind, q.id));

    // Step 2: nearest-match (LIFO) scan, processed one tied-instant group at
    // a time (§4.3's original step 2, generalized per boundary-stint-pairing
    // spec §3.2). Every group of size 1 (no tie) reduces to exactly the old
    // flat scan: a lone `End` has nothing to skip in phase (a) besides
    // itself, and a lone `Start` only ever gets pushed in phase (b).
    let mut stack: Vec<Punch> = Vec::new();
    let mut completed: Vec<Stint> = Vec::new();
    let mut orphaned_ends: Vec<OrphanedEnd> = Vec::new();

    let mut i = 0;
    while i < sorted.len() {
        let group_at = sorted[i].at_utc;
        let group_end = sorted[i..]
            .iter()
            .position(|q| q.at_utc != group_at)
            .map_or(sorted.len(), |off| i + off);
        close_group(
            &sorted[i..group_end],
            &mut stack,
            &mut completed,
            &mut orphaned_ends,
        );
        i = group_end;
    }

    // Whatever is left on the stack is open, in ascending start-instant order.
    let open: Vec<OpenStint> = stack
        .into_iter()
        .map(|start| OpenStint {
            start,
            // Clamped: a future-dated open start is reachable (§3.2 does no
            // chronology check at insert) and a negative elapsed time on a
            // stint line would read as a bug.
            minutes_so_far: (now - start.at_utc).num_minutes().max(0),
        })
        .collect();

    let has_anomaly = DayStints::compute_has_anomaly(&open, &orphaned_ends);

    DayStints {
        completed,
        open,
        orphaned_ends,
        has_anomaly,
    }
}

/// Closes one tied-instant `group` (all punches sharing one `at_utc`)
/// against `stack` (carried in from strictly earlier groups), per
/// boundary-stint-pairing spec §3.2: an `End` preferentially closes an
/// already-open `Start` from before this instant, before this group's own
/// `Start`s are pushed and before any of this group's `End`s pair against
/// each other.
///
/// `group` must already be sorted by `(at_utc, kind, id)` — i.e. a
/// contiguous slice of `classify`'s own `sorted` vector — so that, within
/// the group, all `Start`s precede all `End`s and each kind's own
/// sub-sequence is already in ascending-`id` order.
fn close_group(
    group: &[Punch],
    stack: &mut Vec<Punch>,
    completed: &mut Vec<Stint>,
    orphaned_ends: &mut Vec<OrphanedEnd>,
) {
    // §3.2 step 1: each End in the group, in ascending id order, closes a
    // Start carried in from a strictly earlier group — before any of this
    // group's own Starts are pushed.
    let mut unclosed_ends: Vec<Punch> = Vec::new();
    for punch in group.iter().filter(|q| q.kind == PunchKind::End) {
        match stack.pop() {
            Some(start) => completed.push(Stint {
                start,
                end: *punch,
                minutes: (punch.at_utc - start.at_utc).num_minutes(),
            }),
            None => unclosed_ends.push(*punch),
        }
    }

    // §3.2 step 2: push the group's own Starts (LIFO, id order).
    for punch in group.iter().filter(|q| q.kind == PunchKind::Start) {
        stack.push(*punch);
    }

    // §3.2 step 3: any End that found nothing open before this group now
    // pops against the stack again, which holds only this group's own
    // just-pushed Starts (or is empty) — same-instant E14 pairing, or an
    // ordinary orphaned-end anomaly.
    for punch in unclosed_ends {
        match stack.pop() {
            Some(start) => completed.push(Stint {
                start,
                end: punch,
                minutes: (punch.at_utc - start.at_utc).num_minutes(),
            }),
            None => orphaned_ends.push(OrphanedEnd { punch }),
        }
    }
}

/// Classifies `punches` (one calendar date) same as `classify()`, then
/// resolves an unambiguous midnight-spanning stint against its immediate
/// neighbors (boundary-stint-pairing spec §4.1/§4.2).
///
/// `prev_punches`/`next_punches` must be the literal adjacent calendar
/// dates' punches — `punches[0].date.pred_opt()`/`.succ_opt()` — never
/// further out (pass `&[]` when a neighbor has no data; an empty slice
/// already classifies correctly as "nothing open, nothing orphaned").
///
/// Never panics for any combination of empty/non-empty
/// `prev_punches`/`punches`/`next_punches` — including `punches` empty
/// with a non-empty neighbor, the ordinary shape for every idle calendar
/// date. All internal self-checks are gated on `.first()` before comparing
/// any `[0]`-indexed date.
///
/// With both neighbors empty, reduces exactly to `classify(punches,
/// now)`'s output.
pub fn classify_at(
    prev_punches: &[Punch],
    punches: &[Punch],
    next_punches: &[Punch],
    now: DateTime<Utc>,
) -> DayStints {
    debug_assert!(
        prev_punches
            .iter()
            .all(|q| Some(q.date) == prev_punches.first().map(|f| f.date)),
        "classify_at() expects prev_punches to share one date"
    );
    debug_assert!(
        punches
            .iter()
            .all(|q| Some(q.date) == punches.first().map(|f| f.date)),
        "classify_at() expects punches to share one date"
    );
    debug_assert!(
        next_punches
            .iter()
            .all(|q| Some(q.date) == next_punches.first().map(|f| f.date)),
        "classify_at() expects next_punches to share one date"
    );
    if let (Some(prev_first), Some(day_first)) = (prev_punches.first(), punches.first()) {
        debug_assert!(
            Some(prev_first.date) == day_first.date.pred_opt(),
            "classify_at() expects prev_punches' date to be punches' date minus one day"
        );
    }
    if let (Some(day_first), Some(next_first)) = (punches.first(), next_punches.first()) {
        debug_assert!(
            Some(next_first.date) == day_first.date.succ_opt(),
            "classify_at() expects next_punches' date to be punches' date plus one day"
        );
    }

    let prev = classify(prev_punches, now);
    let mut day = classify(punches, now);
    let next = classify(next_punches, now);

    // (prev, day) as the (A, A+1) pair: if it fires, day loses the matched
    // orphan. The completed stint itself belongs to prev's own view
    // (produced by prev's own classify_at call at the caller level) — never
    // added here.
    if splice_candidate(prev.open.len(), &day, punches) {
        day.orphaned_ends.remove(0);
    }

    // (day, next) as the (A, A+1) pair: if it fires, day gains the
    // completed stint and loses the matched open entry.
    if splice_candidate(day.open.len(), &next, next_punches) {
        let open = day.open.remove(0);
        let end = next
            .orphaned_ends
            .first()
            .expect("splice_candidate only returns Some when `next` has exactly one orphan")
            .punch;
        day.completed.push(Stint {
            start: open.start,
            end,
            minutes: (end.at_utc - open.start.at_utc).num_minutes(),
        });
    }

    day.has_anomaly = DayStints::compute_has_anomaly(&day.open, &day.orphaned_ends);
    day
}

/// Returns `true` when `earlier_open_count` is 1, `later` has exactly one
/// orphaned end, and that orphan is `later_punches`'s chronologically first
/// punch (boundary-stint-pairing spec §4.1's three-part gate). `false`
/// otherwise — no splice.
///
/// Takes `later_punches` raw (not just the already-computed `later`)
/// because the "chronologically first punch" check needs the same
/// `(at_utc, kind, id)` ordering `classify()` uses internally, which
/// `DayStints` does not expose.
pub(crate) fn splice_candidate(
    earlier_open_count: usize,
    later: &DayStints,
    later_punches: &[Punch],
) -> bool {
    if earlier_open_count != 1 || later.orphaned_ends.len() != 1 {
        return false;
    }
    later_punches
        .iter()
        .min_by_key(|q| (q.at_utc, q.kind, q.id))
        .is_some_and(|first| first.id == later.orphaned_ends[0].punch.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, NaiveDate, TimeZone};

    /// All fixtures live on one date, at a fixed UTC offset of zero, so a
    /// wall-clock `HH:MM` in a test name is also the UTC instant.
    const TEST_DATE: (i32, u32, u32) = (2026, 2, 12);

    fn date() -> NaiveDate {
        NaiveDate::from_ymd_opt(TEST_DATE.0, TEST_DATE.1, TEST_DATE.2).unwrap()
    }

    /// `at("09:00")` — the UTC instant for that wall clock on `TEST_DATE`.
    fn at(hhmm: &str) -> DateTime<Utc> {
        at_on(date(), hhmm)
    }

    fn at_on(d: NaiveDate, hhmm: &str) -> DateTime<Utc> {
        let (h, m) = hhmm.split_once(':').expect("HH:MM");
        Utc.with_ymd_and_hms(
            d.year(),
            d.month(),
            d.day(),
            h.parse().unwrap(),
            m.parse().unwrap(),
            0,
        )
        .unwrap()
    }

    /// A punch on an explicit date, for the cross-midnight case.
    fn p_on(id: i64, d: NaiveDate, hhmm: &str, kind: PunchKind) -> Punch {
        Punch {
            id,
            at_utc: at_on(d, hhmm),
            date: d,
            kind,
        }
    }

    /// `p(1, "09:00", PunchKind::Start)` — a punch on `TEST_DATE`.
    fn p(id: i64, hhmm: &str, kind: PunchKind) -> Punch {
        Punch {
            id,
            at_utc: at(hhmm),
            date: date(),
            kind,
        }
    }

    use PunchKind::{End, Start};

    /// (start instant, end instant, minutes) for each completed stint.
    fn completed_spans(d: &DayStints) -> Vec<(DateTime<Utc>, DateTime<Utc>, i64)> {
        d.completed
            .iter()
            .map(|s| (s.start.at_utc, s.end.at_utc, s.minutes))
            .collect()
    }

    // --- T1..T3b: the happy path (F1, F2, F3) ---------------------------

    #[test]
    fn single_start_is_one_open_stint_not_an_anomaly() {
        let d = classify(&[p(1, "09:00", Start)], at("09:30"));

        assert!(d.completed.is_empty());
        assert_eq!(d.open.len(), 1);
        assert_eq!(d.open[0].start.id, 1);
        assert_eq!(d.open[0].minutes_so_far, 30);
        assert!(d.orphaned_ends.is_empty());
        assert!(!d.has_anomaly());
        assert!(d.anomalies().is_empty());
        assert!(d.is_ongoing());
        assert_eq!(d.completed_minutes(), 0);
    }

    #[test]
    fn ordinary_pair_is_one_completed_stint() {
        let d = classify(&[p(1, "09:00", Start), p(2, "17:00", End)], at("17:45"));

        assert_eq!(completed_spans(&d), vec![(at("09:00"), at("17:00"), 480)]);
        assert!(d.open.is_empty());
        assert!(d.orphaned_ends.is_empty());
        assert!(!d.has_anomaly());
        assert!(!d.is_ongoing());
        assert_eq!(d.completed_minutes(), 480);
    }

    #[test]
    fn out_of_order_entry_pairs_by_time_not_entry_order() {
        // SPEC.md §4.3's worked example, verbatim.
        let d = classify(
            &[
                p(1, "09:00", Start),
                p(2, "14:00", Start),
                p(3, "18:00", End),
                p(4, "13:00", End),
            ],
            at("19:00"),
        );

        assert_eq!(
            completed_spans(&d),
            vec![
                (at("09:00"), at("13:00"), 240),
                (at("14:00"), at("18:00"), 240),
            ]
        );
        assert!(d.open.is_empty());
        assert!(d.orphaned_ends.is_empty());
        assert!(!d.has_anomaly());
        assert_eq!(d.completed_minutes(), 480);
    }

    #[test]
    fn entry_order_does_not_change_result() {
        // Same four instants as above, ids assigned in a different permutation.
        let d = classify(
            &[
                p(1, "13:00", End),
                p(2, "09:00", Start),
                p(3, "18:00", End),
                p(4, "14:00", Start),
            ],
            at("19:00"),
        );

        assert_eq!(
            completed_spans(&d),
            vec![
                (at("09:00"), at("13:00"), 240),
                (at("14:00"), at("18:00"), 240),
            ]
        );
        assert!(d.open.is_empty());
        assert!(d.orphaned_ends.is_empty());
        assert_eq!(d.completed_minutes(), 480);
    }

    // --- T7, T7b: E14, the same-instant pair ----------------------------

    #[test]
    fn same_instant_pair_is_a_legal_zero_length_stint() {
        let d = classify(&[p(1, "09:00", Start), p(2, "09:00", End)], at("17:45"));

        assert_eq!(completed_spans(&d), vec![(at("09:00"), at("09:00"), 0)]);
        assert!(d.open.is_empty());
        assert!(d.orphaned_ends.is_empty());
        assert!(!d.has_anomaly());
        assert_eq!(d.completed_minutes(), 0);
    }

    #[test]
    fn same_instant_pair_still_pairs_when_end_entered_first() {
        // The `end` has the LOWER id: this fails under a pure (at_utc, id)
        // sort and passes only with §4.3's kind-before-id tie-break.
        let d = classify(&[p(1, "09:00", End), p(2, "09:00", Start)], at("17:45"));

        assert_eq!(completed_spans(&d), vec![(at("09:00"), at("09:00"), 0)]);
        assert!(d.open.is_empty());
        assert!(d.orphaned_ends.is_empty());
        assert!(!d.has_anomaly());
        assert_eq!(d.completed_minutes(), 0);
    }

    // --- §3.2: tied-instant-group scan (boundary-stint-pairing) ---------

    #[test]
    fn tied_end_and_start_closes_earlier_open_start() {
        // Spec §3.2's worked example: an `end` at a tied instant must
        // preferentially close a `start` carried in from strictly earlier,
        // before pairing with the same-instant `start`.
        let d = classify(
            &[
                p(1, "08:00", Start),
                p(2, "09:00", End),
                p(3, "09:00", Start),
            ],
            at("09:30"),
        );

        assert_eq!(completed_spans(&d), vec![(at("08:00"), at("09:00"), 60)]);
        assert_eq!(d.open.len(), 1);
        assert_eq!(d.open[0].start.id, 3);
        assert_eq!(d.open[0].start.at_utc, at("09:00"));
        assert!(d.orphaned_ends.is_empty());
        assert!(!d.has_anomaly());
    }

    #[test]
    fn tied_group_two_ends_one_start_nothing_open_before() {
        // Nothing open before the tied group: one `end` zero-pairs with the
        // group's own `start`, the other is an ordinary orphaned end.
        let d = classify(
            &[p(1, "09:00", End), p(2, "09:00", End), p(3, "09:00", Start)],
            at("09:30"),
        );

        assert_eq!(d.completed.len(), 1);
        assert_eq!(d.completed[0].start.id, 3);
        assert_eq!(d.completed[0].end.id, 1);
        assert_eq!(d.completed[0].minutes, 0);
        assert!(d.open.is_empty());
        assert_eq!(d.orphaned_ends.len(), 1);
        assert_eq!(d.orphaned_ends[0].punch.id, 2);
        assert!(d.has_anomaly());
    }

    // --- T4, T4b: open-stint duration against the injected `now` (F9) ---

    #[test]
    fn open_stint_duration_is_measured_against_supplied_now() {
        let punches = [
            p(1, "09:00", Start),
            p(2, "13:00", End),
            p(3, "17:30", Start),
        ];
        let d = classify(&punches, at("17:45"));

        assert_eq!(completed_spans(&d), vec![(at("09:00"), at("13:00"), 240)]);
        assert_eq!(d.open.len(), 1);
        assert_eq!(d.open[0].minutes_so_far, 15);
        // The live 15 minutes is NOT folded into the day total (§2.4).
        assert_eq!(d.completed_minutes(), 240);
        assert!(d.is_ongoing());
        assert!(!d.has_anomaly());
    }

    #[test]
    fn open_stint_duration_changes_only_with_now() {
        let punches = [
            p(1, "09:00", Start),
            p(2, "13:00", End),
            p(3, "17:30", Start),
        ];
        let early = classify(&punches, at("17:45"));
        let late = classify(&punches, at("18:45"));

        assert_eq!(early.open[0].minutes_so_far, 15);
        assert_eq!(late.open[0].minutes_so_far, 75);
        // Nothing but the open stint is time-sensitive: no hidden clock read.
        assert_eq!(early.completed, late.completed);
        assert_eq!(early.completed_minutes(), late.completed_minutes());
    }

    #[test]
    fn open_stint_in_the_future_clamps_to_zero_minutes() {
        // Reachable today: SPEC.md §3.2 imposes no chronology check at insert,
        // so `mlm start 23:00` typed at 09:00 leaves a future open start. A
        // negative elapsed time would read as a bug (plan §7.3).
        let d = classify(&[p(1, "23:00", Start)], at("09:00"));

        assert_eq!(d.open.len(), 1);
        assert_eq!(d.open[0].minutes_so_far, 0);
    }

    // --- T5..T5c: E7, more than one trailing unmatched start ------------

    #[test]
    fn two_dangling_starts_are_two_open_stints_plus_multi_open_anomaly() {
        let d = classify(&[p(1, "09:00", Start), p(2, "11:00", Start)], at("12:00"));

        assert!(d.completed.is_empty());
        assert_eq!(
            d.open
                .iter()
                .map(|o| (o.start.at_utc, o.minutes_so_far))
                .collect::<Vec<_>>(),
            vec![(at("09:00"), 180), (at("11:00"), 60)]
        );
        assert!(d.orphaned_ends.is_empty());
        assert!(d.has_anomaly());
        // Exactly ONE summary anomaly for the date, not one per extra start.
        assert_eq!(
            d.anomalies(),
            vec![Anomaly::MultipleOpenStints { count: 2 }]
        );
        assert_eq!(d.completed_minutes(), 0);
    }

    #[test]
    fn three_dangling_starts_report_count_three() {
        let d = classify(
            &[
                p(1, "09:00", Start),
                p(2, "10:00", Start),
                p(3, "11:00", Start),
            ],
            at("12:00"),
        );

        assert_eq!(d.open.len(), 3);
        assert_eq!(
            d.anomalies(),
            vec![Anomaly::MultipleOpenStints { count: 3 }]
        );
    }

    #[test]
    fn mixed_completed_and_multi_open() {
        let d = classify(
            &[
                p(1, "08:00", Start),
                p(2, "09:00", End),
                p(3, "10:00", Start),
                p(4, "11:00", Start),
            ],
            at("12:00"),
        );

        // An anomaly does not suppress the good stint.
        assert_eq!(completed_spans(&d), vec![(at("08:00"), at("09:00"), 60)]);
        assert_eq!(d.open.len(), 2);
        assert!(d.has_anomaly());
        assert_eq!(d.completed_minutes(), 60);
    }

    // --- T6..T6d: E8, orphaned ends -------------------------------------

    #[test]
    fn orphaned_end_produces_anomaly_and_no_stint() {
        let d = classify(&[p(1, "18:00", End)], at("18:30"));

        assert!(d.completed.is_empty());
        assert!(d.open.is_empty());
        assert_eq!(d.orphaned_ends.len(), 1);
        assert_eq!(d.orphaned_ends[0].punch.at_utc, at("18:00"));
        assert!(d.has_anomaly());
        assert_eq!(
            d.anomalies(),
            vec![Anomaly::OrphanedEnd {
                punch: p(1, "18:00", End)
            }]
        );
        assert_eq!(d.completed_minutes(), 0);
        assert!(!d.is_ongoing());
    }

    #[test]
    fn two_orphaned_ends_are_never_coalesced() {
        let d = classify(&[p(1, "12:00", End), p(2, "18:00", End)], at("19:00"));

        assert_eq!(d.orphaned_ends.len(), 2);
        assert_eq!(
            d.anomalies(),
            vec![
                Anomaly::OrphanedEnd {
                    punch: p(1, "12:00", End)
                },
                Anomaly::OrphanedEnd {
                    punch: p(2, "18:00", End)
                },
            ]
        );
        assert_eq!(d.completed_minutes(), 0);
    }

    #[test]
    fn orphaned_end_before_a_clean_pair() {
        let d = classify(
            &[p(1, "08:00", End), p(2, "09:00", Start), p(3, "10:00", End)],
            at("11:00"),
        );

        // The orphan does not consume the later start.
        assert_eq!(
            d.orphaned_ends
                .iter()
                .map(|o| o.punch.at_utc)
                .collect::<Vec<_>>(),
            vec![at("08:00")]
        );
        assert_eq!(completed_spans(&d), vec![(at("09:00"), at("10:00"), 60)]);
        assert!(d.has_anomaly());
        assert_eq!(d.completed_minutes(), 60);
    }

    #[test]
    fn orphan_and_multi_open_emit_both_anomalies_in_order() {
        let d = classify(
            &[
                p(1, "08:00", End),
                p(2, "09:00", Start),
                p(3, "10:00", Start),
            ],
            at("11:00"),
        );

        // Fixed emission order: multi-open first, then orphans (§7.3).
        assert_eq!(
            d.anomalies(),
            vec![
                Anomaly::MultipleOpenStints { count: 2 },
                Anomaly::OrphanedEnd {
                    punch: p(1, "08:00", End)
                },
            ]
        );
    }

    // --- T8: nested entry is nearest-match, not outermost ---------------

    #[test]
    fn nested_entry_pairs_nearest_not_outermost() {
        let d = classify(
            &[
                p(1, "09:00", Start),
                p(2, "10:00", Start),
                p(3, "11:00", End),
                p(4, "12:00", End),
            ],
            at("13:00"),
        );

        assert_eq!(
            completed_spans(&d),
            vec![
                (at("10:00"), at("11:00"), 60),
                (at("09:00"), at("12:00"), 180),
            ]
        );
        // 10:00 paired with 11:00, explicitly NOT 09:00 with 11:00.
        assert_eq!(d.completed[0].start.at_utc, at("10:00"));
        assert_eq!(d.completed[0].end.at_utc, at("11:00"));
        assert!(d.open.is_empty());
        assert!(d.orphaned_ends.is_empty());
        assert!(!d.has_anomaly());
        // Overlap double-counts wall clock by design (SPEC.md §4.3, §2.4).
        assert_eq!(d.completed_minutes(), 240);
    }

    // --- T9: no punches --------------------------------------------------

    #[test]
    fn no_punches_produces_empty_classification() {
        let d = classify(&[], at("12:00"));

        assert!(d.completed.is_empty());
        assert!(d.open.is_empty());
        assert!(d.orphaned_ends.is_empty());
        assert!(!d.has_anomaly());
        assert!(d.anomalies().is_empty());
        assert_eq!(d.completed_minutes(), 0);
        assert!(!d.is_ongoing());
    }

    // --- T10: plain classify() treats cross-midnight halves as two
    // independent dates, each with its own anomaly signal in isolation.
    // Resolving them into one splice pair is classify_at()'s job (see its
    // own tests below); this test only pins classify()'s per-date behavior,
    // unchanged by this changeset. ---

    #[test]
    fn cross_midnight_halves_are_two_separate_anomalies() {
        let day1 = NaiveDate::from_ymd_opt(2026, 2, 12).unwrap();
        let day2 = NaiveDate::from_ymd_opt(2026, 2, 13).unwrap();

        let d1 = classify(&[p_on(1, day1, "23:30", Start)], at_on(day2, "00:45"));
        let d2 = classify(&[p_on(2, day2, "00:45", End)], at_on(day2, "01:00"));

        // Day 1: the ordinary open-stint case, not an anomaly.
        assert_eq!(d1.open.len(), 1);
        assert!(!d1.has_anomaly());
        // Day 2: an orphaned end, which IS an anomaly.
        assert_eq!(d2.orphaned_ends.len(), 1);
        assert!(d2.has_anomaly());
        assert_eq!(d1.completed_minutes() + d2.completed_minutes(), 0);
    }

    // --- T11, T12: contract-2 and §2.4 invariants -----------------------

    #[test]
    fn has_anomaly_matches_anomalies_nonempty() {
        let cases: Vec<Vec<Punch>> = vec![
            vec![],
            vec![p(1, "09:00", Start)],
            vec![p(1, "09:00", Start), p(2, "17:00", End)],
            vec![p(1, "09:00", Start), p(2, "11:00", Start)],
            vec![p(1, "18:00", End)],
            vec![p(1, "09:00", Start), p(2, "09:00", End)],
            vec![
                p(1, "08:00", End),
                p(2, "09:00", Start),
                p(3, "10:00", Start),
            ],
        ];

        for punches in cases {
            let d = classify(&punches, at("19:00"));
            assert_eq!(
                d.has_anomaly(),
                !d.anomalies().is_empty(),
                "mismatch for {punches:?}"
            );
        }
    }

    // --- classify_at: boundary-stint-pairing spec §4.1/§4.2 --------------

    #[test]
    fn splice_next_direction_closes_open_into_completed() {
        let day1 = date();
        let day2 = day1.succ_opt().unwrap();
        let prev_punches: Vec<Punch> = vec![];
        let punches = vec![p_on(1, day1, "23:30", Start)];
        let next_punches = vec![p_on(2, day2, "00:45", End)];

        let d = classify_at(&prev_punches, &punches, &next_punches, at_on(day2, "01:00"));

        assert_eq!(d.completed.len(), 1);
        assert_eq!(d.completed[0].start.id, 1);
        assert_eq!(d.completed[0].end.id, 2);
        assert_eq!(d.completed[0].minutes, 75);
        assert!(d.open.is_empty());
        assert!(d.orphaned_ends.is_empty());
        assert!(!d.has_anomaly());
    }

    #[test]
    fn splice_prev_direction_clears_orphan_without_adding_stint() {
        let day1 = date();
        let day2 = day1.succ_opt().unwrap();
        let prev_punches = vec![p_on(1, day1, "23:30", Start)];
        let punches = vec![p_on(2, day2, "00:45", End)];
        let next_punches: Vec<Punch> = vec![];

        let d = classify_at(&prev_punches, &punches, &next_punches, at_on(day2, "01:00"));

        // The stint belongs to day1's own view (its own classify_at call),
        // never day2's.
        assert!(d.completed.is_empty());
        assert!(d.open.is_empty());
        assert!(d.orphaned_ends.is_empty());
        assert!(!d.has_anomaly());
    }

    #[test]
    fn splice_both_directions_fire_independently() {
        let day1 = date();
        let day2 = day1.succ_opt().unwrap();
        let day3 = day2.succ_opt().unwrap();

        let prev_punches = vec![p_on(1, day1, "23:00", Start)];
        // day2's first punch (00:45 end) closes day1's dangling open; day2's
        // own trailing open (23:00) closes into day3's first-punch orphan.
        let punches = vec![p_on(10, day2, "00:45", End), p_on(11, day2, "23:00", Start)];
        let next_punches = vec![p_on(20, day3, "00:30", End)];

        let d = classify_at(&prev_punches, &punches, &next_punches, at_on(day3, "01:00"));

        assert_eq!(d.completed.len(), 1);
        assert_eq!(d.completed[0].start.id, 11);
        assert_eq!(d.completed[0].end.id, 20);
        assert_eq!(d.completed[0].minutes, 90);
        assert!(d.open.is_empty());
        assert!(d.orphaned_ends.is_empty());
        assert!(!d.has_anomaly());
    }

    #[test]
    fn splice_prev_two_opens_stays_unspliced() {
        let day1 = date();
        let day2 = day1.succ_opt().unwrap();
        let prev_punches = vec![p_on(1, day1, "09:00", Start), p_on(2, day1, "11:00", Start)];
        let punches = vec![p_on(3, day2, "00:45", End)];
        let next_punches: Vec<Punch> = vec![];

        let d = classify_at(&prev_punches, &punches, &next_punches, at_on(day2, "01:00"));

        // prev has 2 opens (E7), so the gate never fires: day2's orphan
        // stays exactly as plain classify() would report it.
        assert_eq!(d.orphaned_ends.len(), 1);
        assert_eq!(d.orphaned_ends[0].punch.id, 3);
        assert!(d.open.is_empty());
        assert!(d.has_anomaly());
    }

    #[test]
    fn splice_day_two_orphans_stays_unspliced() {
        let day1 = date();
        let day2 = day1.succ_opt().unwrap();
        let prev_punches = vec![p_on(1, day1, "23:00", Start)];
        let punches = vec![p_on(2, day2, "00:45", End), p_on(3, day2, "01:00", End)];
        let next_punches: Vec<Punch> = vec![];

        let d = classify_at(&prev_punches, &punches, &next_punches, at_on(day2, "02:00"));

        // day2 has 2 orphans (never coalesced), so the gate never fires.
        assert_eq!(d.orphaned_ends.len(), 2);
        assert!(d.open.is_empty());
        assert!(d.has_anomaly());
    }

    #[test]
    fn splice_orphan_not_first_punch_of_day_stays_unspliced() {
        let day1 = date();
        let day2 = day1.succ_opt().unwrap();
        let prev_punches = vec![p_on(1, day1, "23:00", Start)];
        let punches = vec![
            p_on(2, day2, "09:00", Start),
            p_on(3, day2, "10:00", End),
            p_on(4, day2, "13:00", End),
        ];
        let next_punches: Vec<Punch> = vec![];

        let d = classify_at(&prev_punches, &punches, &next_punches, at_on(day2, "14:00"));

        // day2's only orphan (13:00) isn't its first punch (09:00 start is)
        // so no splice: the 09:00-10:00 pair and the 13:00 orphan both stay
        // exactly as plain classify() would report them.
        assert_eq!(d.completed.len(), 1);
        assert_eq!(d.completed[0].start.id, 2);
        assert_eq!(d.completed[0].end.id, 3);
        assert_eq!(d.orphaned_ends.len(), 1);
        assert_eq!(d.orphaned_ends[0].punch.id, 4);
        assert!(d.has_anomaly());
    }

    #[test]
    fn splice_genuine_gap_two_dates_out_no_splice() {
        let day1 = date();
        let day2 = day1.succ_opt().unwrap();
        let prev_punches = vec![p_on(1, day1, "23:00", Start)];
        let punches: Vec<Punch> = vec![]; // day2: no punches at all
        let next_punches: Vec<Punch> = vec![];

        // The real matching orphan (if any) is two dates out on day3, which
        // this call never sees — classify_at only ever looks one day out.
        let d = classify_at(&prev_punches, &punches, &next_punches, at_on(day2, "12:00"));

        assert!(d.completed.is_empty());
        assert!(d.open.is_empty());
        assert!(d.orphaned_ends.is_empty());
        assert!(!d.has_anomaly());
    }

    #[test]
    fn classify_at_empty_neighbors_matches_classify() {
        let cases: Vec<Vec<Punch>> = vec![
            vec![],
            vec![p(1, "09:00", Start)],
            vec![p(1, "09:00", Start), p(2, "17:00", End)],
            vec![p(1, "09:00", Start), p(2, "11:00", Start)],
            vec![p(1, "18:00", End)],
            vec![p(1, "09:00", Start), p(2, "09:00", End)],
            vec![
                p(1, "08:00", Start),
                p(2, "09:00", End),
                p(3, "09:00", Start),
            ],
            vec![
                p(1, "09:00", Start),
                p(2, "10:00", Start),
                p(3, "11:00", End),
                p(4, "12:00", End),
            ],
        ];

        for punches in cases {
            let direct = classify(&punches, at("19:00"));
            let via_at = classify_at(&[], &punches, &[], at("19:00"));
            assert_eq!(direct, via_at, "mismatch for {punches:?}");
        }
    }

    #[test]
    fn classify_at_empty_punches_bordering_nonempty_prev_no_panic() {
        let prev_punches = vec![p(1, "09:00", Start)];
        let punches: Vec<Punch> = vec![];
        let next_punches: Vec<Punch> = vec![];

        // Must not panic in any build profile: the whole point of this test
        // is that no debug_assert indexes an empty `punches` unguarded.
        let d = classify_at(&prev_punches, &punches, &next_punches, at("10:00"));

        assert!(d.completed.is_empty());
        assert!(d.open.is_empty());
        assert!(d.orphaned_ends.is_empty());
        assert!(!d.has_anomaly());
    }

    #[test]
    fn classify_at_empty_punches_bordering_nonempty_next_no_panic() {
        let prev_punches: Vec<Punch> = vec![];
        let punches: Vec<Punch> = vec![];
        let next_punches = vec![p(1, "08:00", End)];

        let d = classify_at(&prev_punches, &punches, &next_punches, at("10:00"));

        assert!(d.completed.is_empty());
        assert!(d.open.is_empty());
        assert!(d.orphaned_ends.is_empty());
        assert!(!d.has_anomaly());
    }

    #[test]
    fn tied_end_start_at_next_boundary_never_reaches_orphan_check() {
        let day1 = date();
        let day2 = day1.succ_opt().unwrap();
        let punches = vec![p_on(1, day1, "22:00", Start)];
        // next's first instant is a tied End+Start group: per §3.2 it
        // zero-pairs internally (nothing open before it on day2's own
        // timeline) before orphaned_ends is ever populated, so this shape
        // never reaches the splice check as a genuine orphan at all.
        let next_punches = vec![
            p_on(10, day2, "00:00", End),
            p_on(11, day2, "00:00", Start),
            p_on(12, day2, "08:00", End),
        ];

        let d = classify_at(&[], &punches, &next_punches, at_on(day2, "09:00"));

        assert_eq!(d.open.len(), 1);
        assert_eq!(d.open[0].start.id, 1);
        assert!(d.completed.is_empty());
    }

    #[test]
    fn tied_group_two_ends_at_next_boundary_multi_orphan_stays_unspliced() {
        let day1 = date();
        let day2 = day1.succ_opt().unwrap();
        let punches = vec![p_on(1, day1, "22:00", Start)];
        // next's first instant is a tied group of 2 Ends (no Start in the
        // group): both fail to close (nothing open before) and both remain
        // genuine orphans at day2's own index 0/1 — orphaned_ends.len() == 2
        // overall, so the splice gate (== 1) correctly fails, for a
        // different reason than the tied End+Start case above.
        let next_punches = vec![
            p_on(10, day2, "00:00", End),
            p_on(11, day2, "00:00", End),
            p_on(12, day2, "08:00", Start),
        ];

        // Pin down *why* it doesn't splice.
        let next_direct = classify(&next_punches, at_on(day2, "09:00"));
        assert_eq!(next_direct.orphaned_ends.len(), 2);

        let d = classify_at(&[], &punches, &next_punches, at_on(day2, "09:00"));

        assert_eq!(d.open.len(), 1);
        assert_eq!(d.open[0].start.id, 1);
        assert!(d.completed.is_empty());
    }

    #[test]
    fn single_end_at_next_first_instant_is_splice_eligible() {
        let day1 = date();
        let day2 = day1.succ_opt().unwrap();
        let punches = vec![p_on(1, day1, "22:00", Start)];
        // A lone End in its own group at next's first instant, nothing open
        // before it on day2's own timeline: an ordinary orphan at index 0 —
        // the positive control distinguishing "reaches the check" from
        // "actually splices."
        let next_punches = vec![p_on(10, day2, "00:00", End), p_on(11, day2, "08:00", Start)];

        let d = classify_at(&[], &punches, &next_punches, at_on(day2, "09:00"));

        assert_eq!(d.completed.len(), 1);
        assert_eq!(d.completed[0].start.id, 1);
        assert_eq!(d.completed[0].end.id, 10);
        assert_eq!(d.completed[0].minutes, 120);
        assert!(d.open.is_empty());
        assert!(d.orphaned_ends.is_empty());
        assert!(!d.has_anomaly());
    }

    #[test]
    fn completed_minutes_excludes_open_and_orphans() {
        let d = classify(
            &[
                p(1, "09:00", Start),
                p(2, "10:00", End),
                p(3, "11:00", End),
                p(4, "12:00", Start),
            ],
            at("13:00"),
        );

        assert_eq!(d.completed_minutes(), 60);
        assert_eq!(d.open.len(), 1);
        assert_eq!(d.open[0].minutes_so_far, 60);
        assert_eq!(d.orphaned_ends.len(), 1);
        assert_eq!(d.orphaned_ends[0].punch.at_utc, at("11:00"));
    }
}
