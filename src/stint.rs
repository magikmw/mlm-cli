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

    // Step 2: nearest-match (LIFO) scan.
    let mut stack: Vec<Punch> = Vec::new();
    let mut completed: Vec<Stint> = Vec::new();
    let mut orphaned_ends: Vec<OrphanedEnd> = Vec::new();

    for punch in sorted {
        match punch.kind {
            PunchKind::Start => stack.push(punch),
            PunchKind::End => match stack.pop() {
                Some(start) => completed.push(Stint {
                    start,
                    end: punch,
                    minutes: (punch.at_utc - start.at_utc).num_minutes(),
                }),
                None => orphaned_ends.push(OrphanedEnd { punch }),
            },
        }
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

    let has_anomaly = open.len() > 1 || !orphaned_ends.is_empty();

    DayStints {
        completed,
        open,
        orphaned_ends,
        has_anomaly,
    }
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

    // --- T10: cross-midnight is two separate dates (accepted limitation) -

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
