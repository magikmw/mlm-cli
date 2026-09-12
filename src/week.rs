//! Week accounting: target, carry, fulfillment (SPEC §1.1, §2.3, §2.4, §5).
//!
//! Pure integer arithmetic over injected data. No clock, no SQL, no CLI,
//! no rendering. Every minute value is a signed `i64` — carry, owed and
//! fulfillment are all legitimately negative (SPEC §5) and nothing here
//! is ever clamped.

// TODO(integration): drop this once Milestones 9/10/11 consume the module.
// Until then nothing in the binary references it, so every item is
// "dead" from the non-test build's point of view.
#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};

use crate::date::WeekId;

/// SPEC §2.3 / §6.2: 40h, used when a week has no `week_targets` row.
pub const DEFAULT_WEEK_TARGET_MINUTES: i64 = 2400;

/// A source of per-week completed-stint totals.
///
/// Implementations MUST report **completed stints only** (SPEC §2.4,
/// NOTES decisions 37/38): an open stint's live minutes and an orphaned
/// `end` contribute nothing here. This module consumes whatever number it
/// is handed and never inspects punches itself, so a source that helpfully
/// folded in live minutes would make every carry figure silently wrong and
/// no test in this module would catch it. Assert that rule at the
/// integration seam (PLAN.md contract 10's wave-3 adapter).
pub trait WeekData {
    /// The earliest ISO week with any punch or note data, or `None` when
    /// there is no data at all.
    fn earliest_data_week(&self) -> Option<WeekId>;

    /// Total completed-stint minutes recorded in `week`. `0` for a week
    /// with no data — never an error, never an `Option` (SPEC §6.2: an
    /// empty week is valid).
    fn worked_minutes(&self, week: WeekId) -> i64;
}

/// A source of sparse per-week target overrides (the `week_targets` table).
pub trait WeekTargets {
    /// `Some(minutes)` when the week has an override row — **including a
    /// legal `0`** — and `None` when it has none (SPEC §2.3 / §6.2).
    ///
    /// `Some(0)` is a deliberate week off and is *not* the same as `None`;
    /// collapsing the two with `unwrap_or(0)` would make a week off
    /// indistinguishable from a default 40h week.
    fn target_override(&self, week: WeekId) -> Option<i64>;
}

/// One week's fully-computed accounting (PLAN.md interface contract 3).
///
/// All minute fields are signed and nothing is clamped anywhere (SPEC §5's
/// closing note). Deliberately carries **no** "is this the current week"
/// boolean: that comparison is Milestone 9's sole job, a pure function of
/// `week` and an injected "now" — which is why this module never needs to
/// know the date at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeekAccounting {
    /// Which week this row describes.
    pub week: WeekId,
    /// The override if one is set (including `0`), else
    /// `DEFAULT_WEEK_TARGET_MINUTES`. Never adjusted by carry (SPEC §1.1).
    pub target: i64,
    /// The previous week's `carry_out`; `0` for the first week of the walk.
    pub carry_in: i64,
    /// Completed-stint minutes for this week (NOTES 37/38).
    pub worked: i64,
    /// `worked + carry_in`.
    pub fulfillment: i64,
    /// `target - fulfillment`. Negative means ahead of target.
    pub owed: i64,
    /// `fulfillment - target`; becomes the next week's `carry_in`.
    /// Algebraically `-owed`; both are stored because contract 3 names both
    /// and SPEC §7.2's output renders both.
    pub carry_out: i64,
}

/// In-memory `WeekData` + `WeekTargets`. Used by this module's tests, and
/// reusable by Milestones 9/10/11's tests and by the wave-3 adapter that
/// loads one `GROUP BY` query's worth of rows into it.
#[derive(Debug, Default, Clone)]
pub struct WeekLedger {
    /// Sparse: an absent week has worked 0 minutes.
    worked: BTreeMap<WeekId, i64>,
    /// Weeks with ANY punch or note data. Deliberately *not* derived from
    /// `worked.keys()`: a week can have data and zero worked minutes (a
    /// note-only week, or a week whose only punches are an unmatched
    /// `start` and an orphaned `end`) and must still anchor the walk.
    data_weeks: BTreeSet<WeekId>,
    /// Sparse: an absent week uses `DEFAULT_WEEK_TARGET_MINUTES`.
    targets: BTreeMap<WeekId, i64>,
}

impl WeekLedger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a week's worked minutes **and** mark it a data week.
    pub fn with_worked(mut self, week: WeekId, minutes: i64) -> Self {
        self.worked.insert(week, minutes);
        self.data_weeks.insert(week);
        self
    }

    /// Mark a week as having data without adding worked minutes — a
    /// note-only week, or one whose punches produce no completed stint.
    pub fn with_data_week(mut self, week: WeekId) -> Self {
        self.data_weeks.insert(week);
        self
    }

    /// Record a target override. `0` is legal and meaningful (SPEC §2.3).
    pub fn with_target(mut self, week: WeekId, minutes: i64) -> Self {
        self.targets.insert(week, minutes);
        self
    }
}

impl WeekData for WeekLedger {
    fn earliest_data_week(&self) -> Option<WeekId> {
        // Relies on `WeekId`'s year-then-week `Ord` being chronological.
        self.data_weeks.iter().next().copied()
    }

    fn worked_minutes(&self, week: WeekId) -> i64 {
        self.worked.get(&week).copied().unwrap_or(0)
    }
}

impl WeekTargets for WeekLedger {
    fn target_override(&self, week: WeekId) -> Option<i64> {
        self.targets.get(&week).copied()
    }
}

/// The week's target: its override if it has one (including `0`), else the
/// 40h default (SPEC §2.3 / §6.2).
fn resolve_target(week: WeekId, targets: &impl WeekTargets) -> i64 {
    targets
        .target_override(week)
        .unwrap_or(DEFAULT_WEEK_TARGET_MINUTES)
}

/// The full carry chain: every week from the walk's start through
/// `through`, chronologically, inclusive at both ends. Never empty.
///
/// The walk visits **every** ISO week in the range, including ones with no
/// data at all (SPEC §2.4): an idle week goes through the identical
/// arithmetic with `worked = 0`, which is exactly how it accrues its full
/// deficit and carries it forward. There is deliberately no "skip empty
/// weeks" shortcut — that shortcut is the bug §2.4 was amended to prevent.
///
/// Cost is O(weeks between start and `through`) — pure integer arithmetic,
/// which §2.4 explicitly accepts rather than materializing a carry table.
pub fn week_series(
    through: WeekId,
    data: &impl WeekData,
    targets: &impl WeekTargets,
) -> Vec<WeekAccounting> {
    // `min` is what makes a `through` earlier than all data terminate:
    // walking forward from a later earliest week would never reach it.
    // Carry only ever flows forward, so there is nothing to walk back to.
    let start = match data.earliest_data_week() {
        Some(earliest) => earliest.min(through),
        None => through,
    };

    let mut out = Vec::new();
    let mut carry_in = 0i64;
    let mut current = start;

    loop {
        let target = resolve_target(current, targets);
        let worked = data.worked_minutes(current);
        // Each figure straight from its own SPEC §5 formula, so a sign
        // error in one cannot hide behind another. Nothing is clamped:
        // no `max`, no `saturating_*`, no `abs`, no `clamp`.
        let fulfillment = worked + carry_in;
        let owed = target - fulfillment;
        let carry_out = fulfillment - target;

        out.push(WeekAccounting {
            week: current,
            target,
            carry_in,
            worked,
            fulfillment,
            owed,
            carry_out,
        });

        if current == through {
            break;
        }
        carry_in = carry_out;
        current = current.next();
    }

    out
}

/// One week's accounting, computed by walking the full chain up to it.
pub fn week_accounting(
    week: WeekId,
    data: &impl WeekData,
    targets: &impl WeekTargets,
) -> WeekAccounting {
    week_series(week, data, targets)
        .pop()
        .expect("week_series is never empty")
}

/// The `status`-only daily pace hint: the week's target divided by 5,
/// floored to the minute (SPEC §2.4 / §7.1, NOTES decision 17).
///
/// Purely derived from the target — no override, no storage, no
/// interaction with carry.
///
/// `div_euclid` rather than `/` on purpose: `/` truncates toward zero,
/// which is not flooring for negative inputs. The schema forbids a
/// negative `target_minutes`, so today the two agree — but §2.4 says
/// "floor", so this writes floor. Do not "simplify" it back to `/`.
pub fn daily_target_minutes(week_target_minutes: i64) -> i64 {
    week_target_minutes.div_euclid(5)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Duration, NaiveDate, Weekday};

    /// Test-only shorthand for a validated `WeekId`.
    fn wk(year: i32, week: u32) -> WeekId {
        WeekId::new(year, week, "test").expect("test fixture uses a real ISO week")
    }

    /// Number of ISO weeks in `year`, read off chrono rather than assumed.
    fn iso_weeks_in_year(year: i32) -> u32 {
        NaiveDate::from_ymd_opt(year, 12, 28)
            .expect("Dec 28 is always in the last ISO week of its year")
            .iso_week()
            .week()
    }

    // ---- §7.8 daily target derivation (SPEC §2.4 / §7.1, NOTES 17) ----
    //
    // Note for future refactors: `daily_target_minutes` takes the week's
    // target and *nothing else* — no carry, no fulfillment, no week id.
    // SPEC §2.4 ("purely derived from the week's target ... no interaction
    // with carry") is enforced by that signature, not by a runtime check.
    // Do not widen it.

    #[test]
    fn daily_target_is_a_fifth_of_the_week_target() {
        assert_eq!(daily_target_minutes(2400), 480); // §7.1's `08h 00m`
    }

    #[test]
    fn daily_target_floors_rather_than_rounding() {
        assert_eq!(daily_target_minutes(2002), 400); // 400.4 -> 400
        assert_eq!(daily_target_minutes(2004), 400); // 400.8 -> 400, not 401
        assert_eq!(daily_target_minutes(1), 0);
    }

    #[test]
    fn daily_target_of_a_zero_target_week_is_zero() {
        assert_eq!(daily_target_minutes(0), 0);
    }

    // ---- §7.9 WeekId::next() across year boundaries ----

    #[test]
    fn iso_year_lengths_are_what_the_plan_assumes() {
        assert_eq!(iso_weeks_in_year(2026), 53, "2026 is a 53-week ISO year");
        assert_eq!(iso_weeks_in_year(2025), 52, "2025 is a 52-week ISO year");
    }

    #[test]
    fn next_walks_through_a_53_week_year() {
        assert_eq!(wk(2026, 52).next(), wk(2026, 53));
        assert_eq!(wk(2026, 53).next(), wk(2027, 1));
    }

    #[test]
    fn next_rolls_a_52_week_year_straight_into_january() {
        assert_eq!(wk(2025, 52).next(), wk(2026, 1));
    }

    #[test]
    fn week_53_of_a_52_week_year_is_not_constructible() {
        assert!(WeekId::new(2025, 53, "test").is_err());
        assert!(WeekId::new(2026, 54, "test").is_err());
        assert!(WeekId::new(2026, 0, "test").is_err());
    }

    #[test]
    fn from_date_and_start_round_trip() {
        let monday = wk(2026, 3).start();
        assert_eq!(monday.weekday(), Weekday::Mon);
        assert_eq!(WeekId::from_date(monday), wk(2026, 3));
        assert_eq!(WeekId::from_date(monday + Duration::days(6)), wk(2026, 3));
    }

    #[test]
    fn from_date_uses_iso_year_not_calendar_year() {
        // 2027-01-01 belongs to ISO week 2026-53.
        let new_years_day = NaiveDate::from_ymd_opt(2027, 1, 1).unwrap();
        assert_eq!(WeekId::from_date(new_years_day), wk(2026, 53));
    }

    #[test]
    fn ord_is_chronological_across_a_year_boundary() {
        assert!(wk(2026, 53) < wk(2027, 1));
        assert!(wk(2026, 1) < wk(2026, 2));
    }

    // ---- §7.1 SPEC §5's four-week table, exactly (primary test) ----

    /// The fixture behind SPEC §5: 2200 / 2500 / 1950 / 2600 worked over
    /// `2026-01..=2026-04`, with a 2000 target override on `2026-03`.
    fn spec_section_5_ledger() -> WeekLedger {
        WeekLedger::new()
            .with_worked(wk(2026, 1), 2200)
            .with_worked(wk(2026, 2), 2500)
            .with_worked(wk(2026, 3), 1950)
            .with_worked(wk(2026, 4), 2600)
            .with_target(wk(2026, 3), 2000)
    }

    #[test]
    fn reproduces_spec_section_5_table_exactly() {
        let ledger = spec_section_5_ledger();
        let series = week_series(wk(2026, 4), &ledger, &ledger);

        let expected = vec![
            WeekAccounting {
                week: wk(2026, 1),
                target: 2400,
                carry_in: 0,
                worked: 2200,
                fulfillment: 2200,
                owed: 200,
                carry_out: -200,
            },
            WeekAccounting {
                week: wk(2026, 2),
                target: 2400,
                carry_in: -200,
                worked: 2500,
                fulfillment: 2300,
                owed: 100,
                carry_out: -100,
            },
            WeekAccounting {
                week: wk(2026, 3),
                target: 2000,
                carry_in: -100,
                worked: 1950,
                fulfillment: 1850,
                owed: 150,
                carry_out: -150,
            },
            WeekAccounting {
                week: wk(2026, 4),
                target: 2400,
                carry_in: -150,
                worked: 2600,
                fulfillment: 2450,
                owed: -50,
                carry_out: 50,
            },
        ];

        assert_eq!(series.len(), 4);
        assert_eq!(series, expected);
    }

    #[test]
    fn week_accounting_agrees_with_the_series_tail() {
        let ledger = spec_section_5_ledger();
        let series = week_series(wk(2026, 4), &ledger, &ledger);
        let one = week_accounting(wk(2026, 4), &ledger, &ledger);

        assert_eq!(one, *series.last().unwrap());
        assert_eq!(one.owed, -50);
        assert_eq!(one.carry_out, 50);
    }

    // ---- §7.2 first tracked week has carry_in = 0 (E12) ----

    #[test]
    fn first_tracked_week_starts_with_zero_carry_in() {
        let ledger = WeekLedger::new().with_worked(wk(2026, 1), 2200);
        let series = week_series(wk(2026, 1), &ledger, &ledger);

        // Exactly one row: the walk must not invent weeks before the
        // earliest data week.
        assert_eq!(series.len(), 1);
        assert_eq!(
            series[0],
            WeekAccounting {
                week: wk(2026, 1),
                target: 2400,
                carry_in: 0,
                worked: 2200,
                fulfillment: 2200,
                owed: 200,
                carry_out: -200,
            }
        );
    }

    // ---- §7.3 default target when unset (SPEC §6.2 / F6) ----

    #[test]
    fn week_with_no_override_uses_the_default_target() {
        let ledger = WeekLedger::new().with_worked(wk(2026, 1), 1000);
        let row = week_accounting(wk(2026, 1), &ledger, &ledger);

        assert_eq!(DEFAULT_WEEK_TARGET_MINUTES, 2400);
        assert_eq!(row.target, DEFAULT_WEEK_TARGET_MINUTES);
    }

    #[test]
    fn a_target_override_applies_only_to_its_own_week() {
        let ledger = WeekLedger::new()
            .with_worked(wk(2026, 1), 0)
            .with_worked(wk(2026, 2), 0)
            .with_target(wk(2026, 1), 2000);
        let series = week_series(wk(2026, 2), &ledger, &ledger);

        assert_eq!(series[0].target, 2000);
        assert_eq!(series[1].target, 2400, "overrides are not sticky");
    }

    // ---- §7.4 zero-target override (F7b / SPEC §2.3) ----

    #[test]
    fn zero_target_week_with_worked_time_ends_ahead() {
        let ledger = WeekLedger::new()
            .with_worked(wk(2026, 10), 600)
            .with_target(wk(2026, 10), 0);
        let row = week_accounting(wk(2026, 10), &ledger, &ledger);

        assert_eq!(row.carry_in, 0);
        assert_eq!(row.target, 0);
        assert_eq!(row.fulfillment, 600);
        assert_eq!(row.owed, -600, "ahead by 600, uncapped");
        assert_eq!(row.carry_out, 600);
    }

    #[test]
    fn zero_target_week_with_no_worked_time_accrues_no_deficit() {
        let ledger = WeekLedger::new()
            .with_worked(wk(2026, 10), 0)
            .with_target(wk(2026, 10), 0);
        let row = week_accounting(wk(2026, 10), &ledger, &ledger);

        assert_eq!(row.target, 0);
        assert_eq!(row.owed, 0, "a deliberate week off owes nothing");
        assert_eq!(row.carry_out, 0);
    }

    #[test]
    fn a_zero_override_is_not_the_same_as_no_override() {
        // The `unwrap_or(0)` trap: `Some(0)` and `None` must diverge.
        let with_zero = WeekLedger::new()
            .with_worked(wk(2026, 10), 0)
            .with_target(wk(2026, 10), 0);
        let without = WeekLedger::new().with_worked(wk(2026, 10), 0);

        assert_eq!(with_zero.target_override(wk(2026, 10)), Some(0));
        assert_eq!(without.target_override(wk(2026, 10)), None);

        assert_eq!(
            week_accounting(wk(2026, 10), &with_zero, &with_zero).target,
            0
        );
        assert_eq!(
            week_accounting(wk(2026, 10), &without, &without).target,
            2400
        );
    }

    // ---- §7.5 idle gap weeks carry through (F8 / F8b / SPEC §2.4) ----

    #[test]
    fn an_absent_week_is_still_walked_and_accrues_its_full_deficit() {
        // 2026-01 lands exactly on target so a bug cannot hide in a
        // nonzero carry_out.
        let ledger = WeekLedger::new()
            .with_worked(wk(2026, 1), 2400)
            .with_worked(wk(2026, 3), 2400);
        let series = week_series(wk(2026, 3), &ledger, &ledger);

        assert_eq!(series.len(), 3);
        assert_eq!(
            series[1],
            WeekAccounting {
                week: wk(2026, 2),
                target: 2400,
                carry_in: 0,
                worked: 0,
                fulfillment: 0,
                owed: 2400,
                carry_out: -2400,
            }
        );
        assert_eq!(series[2].carry_in, -2400);
        assert_eq!(series[2].owed, 2400);
    }

    #[test]
    fn an_absent_week_is_indistinguishable_from_an_explicit_zero_week() {
        let absent = WeekLedger::new()
            .with_worked(wk(2026, 1), 2400)
            .with_worked(wk(2026, 3), 2400);
        let explicit = WeekLedger::new()
            .with_worked(wk(2026, 1), 2400)
            .with_worked(wk(2026, 2), 0)
            .with_worked(wk(2026, 3), 2400);

        assert_eq!(
            week_series(wk(2026, 3), &absent, &absent),
            week_series(wk(2026, 3), &explicit, &explicit),
        );
    }

    #[test]
    fn two_consecutive_gap_weeks_both_accrue_deficit() {
        let ledger = WeekLedger::new()
            .with_worked(wk(2026, 1), 2400)
            .with_worked(wk(2026, 4), 2400);
        let series = week_series(wk(2026, 4), &ledger, &ledger);

        assert_eq!(series.len(), 4);
        assert_eq!(series[3].carry_in, -4800, "the walk loops, not looks back");
    }

    // ---- §7.6 never-touched weeks (E13) ----

    #[test]
    fn future_week_after_all_data_accumulates_the_whole_deficit_chain() {
        let ledger = WeekLedger::new().with_worked(wk(2026, 1), 2400);
        let series = week_series(wk(2026, 20), &ledger, &ledger);

        assert_eq!(series.len(), 20);
        let last = series.last().unwrap();
        assert_eq!(last.week, wk(2026, 20));
        assert_eq!(last.worked, 0);
        assert_eq!(last.target, 2400);
        // 2026-01 is on target; weeks 02..=19 are 18 empty weeks.
        assert_eq!(last.carry_in, -2400 * 18);
        assert_eq!(week_accounting(wk(2026, 20), &ledger, &ledger), *last);
    }

    #[test]
    fn past_week_before_all_data_is_a_single_zero_carry_row() {
        let ledger = WeekLedger::new().with_worked(wk(2026, 10), 2400);
        let series = week_series(wk(2026, 5), &ledger, &ledger);

        assert_eq!(series.len(), 1);
        assert_eq!(
            series[0],
            WeekAccounting {
                week: wk(2026, 5),
                target: 2400,
                carry_in: 0,
                worked: 0,
                fulfillment: 0,
                owed: 2400,
                carry_out: -2400,
            }
        );
    }

    #[test]
    fn completely_empty_ledger_still_yields_a_full_result() {
        let ledger = WeekLedger::new();
        assert_eq!(ledger.earliest_data_week(), None);

        let series = week_series(wk(2026, 7), &ledger, &ledger);
        assert_eq!(series.len(), 1);
        assert_eq!(
            series[0],
            WeekAccounting {
                week: wk(2026, 7),
                target: 2400,
                carry_in: 0,
                worked: 0,
                fulfillment: 0,
                owed: 2400,
                carry_out: -2400,
            }
        );
    }

    #[test]
    fn a_note_only_week_still_anchors_the_walk() {
        // Proves `earliest_data_week` is not derived from `worked.keys()`:
        // if it were, the series would be length 1 with carry_in 0.
        let ledger = WeekLedger::new()
            .with_data_week(wk(2026, 1))
            .with_worked(wk(2026, 2), 2400);
        let series = week_series(wk(2026, 2), &ledger, &ledger);

        assert_eq!(series.len(), 2);
        assert_eq!(series[0].week, wk(2026, 1));
        assert_eq!(series[0].worked, 0);
        assert_eq!(series[1].carry_in, -2400);
    }

    // ---- §7.9 walk-level: across a 53-week year boundary ----

    #[test]
    fn the_walk_spans_a_53_week_year_boundary_unbroken() {
        let ledger = WeekLedger::new().with_worked(wk(2026, 52), 2400);
        let series = week_series(wk(2027, 2), &ledger, &ledger);

        let weeks: Vec<WeekId> = series.iter().map(|r| r.week).collect();
        assert_eq!(
            weeks,
            vec![wk(2026, 52), wk(2026, 53), wk(2027, 1), wk(2027, 2)]
        );
        assert_eq!(series[3].carry_in, -2400 * 2);
    }

    // ---- §7.10 nothing is clamped anywhere (SPEC §5's closing note) ----

    #[test]
    fn a_surplus_survives_into_a_zero_worked_week() {
        // NOTES.md's description of the *old manual* process capped carry at
        // 40h with only shortfall spilling forward. Decision 3 / SPEC §1.1
        // replaced that with signed two-way carry. This pins the new
        // behavior; the old cap must not come back.
        let ledger = WeekLedger::new()
            .with_worked(wk(2026, 1), 5000)
            .with_worked(wk(2026, 2), 0);
        let series = week_series(wk(2026, 2), &ledger, &ledger);

        assert_eq!(series[0].carry_out, 2600);
        assert_eq!(series[1].carry_in, 2600);
        assert_eq!(series[1].fulfillment, 2600);
        assert_eq!(series[1].owed, -200);
    }

    #[test]
    fn a_long_deficit_chain_has_no_floor() {
        let ledger = WeekLedger::new().with_worked(wk(2026, 1), 2400);
        let series = week_series(wk(2026, 12), &ledger, &ledger);

        assert_eq!(series.len(), 12);
        // Ten empty weeks (02..=11) precede the requested week.
        assert_eq!(series[11].carry_in, -24000);

        for pair in series[1..].windows(2) {
            assert!(
                pair[1].carry_in < pair[0].carry_in,
                "carry must keep decreasing, never bottom out at zero"
            );
        }
    }
}
