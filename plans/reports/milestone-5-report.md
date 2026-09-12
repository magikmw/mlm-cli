# Milestone 5 — Stint pairing: completion report

**Branch**: `milestone-5` · **Files touched**: `src/stint.rs` (new),
`src/main.rs` (one `mod stint;` line). Nothing else.

## What was implemented

`src/stint.rs` — pure stint-pairing logic per SPEC.md §4.3, built
test-first (RED→GREEN per case, see "TDD trail" below):

- `classify(punches: &[Punch], now: DateTime<Utc>) -> DayStints` — the
  single entry point. Sorts an internal copy by `(at_utc, kind, id)`
  (stable), then runs the nearest-match LIFO scan: `Start` pushes, `End`
  pops the most recent unmatched start, an `End` on an empty stack becomes
  an orphan, whatever remains on the stack becomes open stints.
- `debug_assert!` on the single-date precondition (§3.0). It is safe on an
  empty slice — `Iterator::all` never evaluates `punches[0]` for an empty
  iterator, which T9 covers.
- No `Utc::now()` anywhere outside `#[cfg(test)]` (contract 6); `now` is a
  plain positional `DateTime<Utc>` parameter.
- `minutes_so_far` is clamped at 0 for a future-dated open start (plan §3.4 /
  §7.3), covered by its own test.
- `has_anomaly` is a private precomputed field
  (`open.len() > 1 || !orphaned_ends.is_empty()`), read via `has_anomaly()`;
  `DayStints` has no public constructor, so the invariant cannot be violated
  from outside.

## Verification (all run in this worktree)

- `cargo build` — **pass** (3 pre-existing `dead_code` warnings in
  `src/time.rs`, none from `stint.rs`).
- `cargo test` — **21 passed, 0 failed**.
- `cargo fmt` — applied, clean.
- `cargo clippy --all-targets -- -D warnings` — **fails only on
  pre-existing findings outside this milestone's scope**:
  `time.rs::parse_hm`, `time.rs::between`, `time.rs::format_duration` are
  "never used". This failure reproduces on the untouched baseline commit
  (verified before any edit) and `time.rs` is out of scope, so it was left
  alone. With those excluded (`-A dead_code`), clippy is clean — including
  under `-D clippy::pedantic`, where the only hit is a pre-existing
  `doc_markdown` nit in `db.rs:1`.

## Stint classification result shape (for Milestones 9/10/11 to cross-check)

Matches the plan §2 / PLAN.md contract 2 exactly, with no field renames:

```rust
pub enum PunchKind { Start, End }              // Copy + Ord, Start < End
pub struct Punch { pub id: i64, pub at_utc: DateTime<Utc>,
                   pub date: NaiveDate, pub kind: PunchKind }   // Copy

pub struct Stint      { pub start: Punch, pub end: Punch, pub minutes: i64 }
pub struct OpenStint  { pub start: Punch, pub minutes_so_far: i64 }
pub struct OrphanedEnd { pub punch: Punch }

pub enum Anomaly {
    MultipleOpenStints { count: usize },
    OrphanedEnd { punch: Punch },
}

pub struct DayStints {
    pub completed: Vec<Stint>,          // ascending by END instant
    pub open: Vec<OpenStint>,           // ascending by start instant
    pub orphaned_ends: Vec<OrphanedEnd>,// ascending by instant
    has_anomaly: bool,                  // PRIVATE
}
impl DayStints {
    pub fn has_anomaly(&self) -> bool;      // O(1), stored bool
    pub fn anomalies(&self) -> Vec<Anomaly>;// MultipleOpenStints first, then orphans
    pub fn completed_minutes(&self) -> i64;
    pub fn is_ongoing(&self) -> bool;       // !open.is_empty()
}

pub fn classify(punches: &[Punch], now: DateTime<Utc>) -> DayStints;
```

Derives: `Stint`/`OpenStint`/`OrphanedEnd`/`Anomaly` are
`Debug + Clone + Copy + PartialEq + Eq`; `DayStints` is
`Debug + Clone + PartialEq + Eq` (not `Copy` — it owns `Vec`s).

**Pinned semantics downstream must not re-derive:**
- `completed` is in **end-instant order**, not start order (differs from
  start order only in the nested case — see `nested_entry_pairs_nearest_not_outermost`).
- One open stint is **not** an anomaly; `is_ongoing()` and `has_anomaly()`
  are independent signals.
- Multi-open yields exactly **one** `MultipleOpenStints` per date; orphaned
  ends yield **one anomaly per orphan**, never coalesced. This asymmetry is
  from §7.3 and is deliberate.
- `completed_minutes()` excludes open-stint live time and orphans entirely.

## Tests (21, all in `#[cfg(test)] mod tests` in `src/stint.rs`)

Full plan §6 table, mapped to test names:

| plan | test |
|---|---|
| T1 | `single_start_is_one_open_stint_not_an_anomaly` |
| T2 | `ordinary_pair_is_one_completed_stint` |
| T3 | `out_of_order_entry_pairs_by_time_not_entry_order` |
| T3b | `entry_order_does_not_change_result` |
| T4 | `open_stint_duration_is_measured_against_supplied_now` |
| T4b | `open_stint_duration_changes_only_with_now` |
| T5 | `two_dangling_starts_are_two_open_stints_plus_multi_open_anomaly` |
| T5b | `three_dangling_starts_report_count_three` |
| T5c | `mixed_completed_and_multi_open` |
| T6 | `orphaned_end_produces_anomaly_and_no_stint` |
| T6b | `two_orphaned_ends_are_never_coalesced` |
| T6c | `orphaned_end_before_a_clean_pair` |
| T6d | `orphan_and_multi_open_emit_both_anomalies_in_order` |
| T7 | `same_instant_pair_is_a_legal_zero_length_stint` |
| T7b | `same_instant_pair_still_pairs_when_end_entered_first` |
| T8 | `nested_entry_pairs_nearest_not_outermost` |
| T9 | `no_punches_produces_empty_classification` |
| T10 | `cross_midnight_halves_are_two_separate_anomalies` |
| T11 | `has_anomaly_matches_anomalies_nonempty` (table-driven, 7 inputs) |
| T12 | `completed_minutes_excludes_open_and_orphans` |
| (added) | `open_stint_in_the_future_clamps_to_zero_minutes` — plan §3.4/§7.3's clamp had no test in §6's table; added one |

Fixture helpers (test-only): `p(id, "HH:MM", kind)` on `TEST_DATE =
2026-02-12`, plus `p_on`/`at_on` for T10's second date. All fixtures use a
zero UTC offset, so a wall clock in a test name equals the UTC instant.

**TDD trail** (the two red steps worth recording): T7b was written while the
sort key was still `(at_utc, id)` and failed with
`left: [] right: [(09:00, 09:00, 0)]` — the exact degradation into
orphan + open stint the corrected §4.3 tie-break exists to prevent; adding
`kind` to the key turned it green. Likewise the clamp test was written
against an unclamped implementation and failed with `-840`.

## Deviations from the plan (2, both forced, both mechanical to undo)

1. **`Punch`/`PunchKind` are declared locally rather than imported.**
   Milestone 4's `src/storage.rs` does not exist in this worktree, so the
   plan's `use crate::storage::{Punch, PunchKind};` cannot compile. Both
   types are declared in `src/stint.rs` at the exact agreed shape (field
   names, types, `Copy`, `PunchKind: Ord` with `Start < End`) under a
   `// TODO(integration):` comment naming the one-line replacement.
   **Integration step: delete those two declarations, add that `use` line.
   Nothing else in the module changes** — the algorithm only uses
   `id`/`at_utc`/`date`/`kind` and `PunchKind`'s `Ord`.
   Sub-deviation: they are declared `pub`, not private as the task brief
   suggested, because they appear in public fields of `Stint`/`OpenStint`/
   `OrphanedEnd`/`Anomaly` — a private type there trips the
   `private_interfaces` lint and would fail `-D warnings`. In a binary crate
   `pub` in a non-root module is crate-visible only, so nothing leaks.
2. **`#![allow(dead_code)]` at the top of `src/stint.rs`.** Milestone 5 ships
   ahead of every consumer, so its whole public surface is uncalled in the
   `bin` target and `-D warnings` would reject it. Remove once Milestone 10/11
   call in.

## Open questions / risks for the reviewer

- **Pre-existing clippy failure in `time.rs`** (dead code) blocks a clean
  `cargo clippy --all-targets -- -D warnings` at repo level. Not introduced
  here and out of scope; whoever owns Milestone 1's integration should
  resolve it (likely the same `allow` or the first real caller).
- **Nested stints double-count wall clock by design** (T8: 240 minutes across
  180 minutes of clock) and do **not** raise `has_anomaly()`. Spec-conformant
  per §4.3 + §2.4 — flagged because it is the most likely thing to be
  mistaken for a bug, and because a fat-fingered double `start` can silently
  inflate a week total. Changing it is a SPEC.md change (a new overlap
  anomaly), not a Milestone 5 fix.
- **Sub-minute components truncate.** `num_minutes()` truncates toward zero;
  §4.1 says no seconds precision exists anywhere, but nothing here enforces
  it. Zeroing seconds/nanos when constructing `at_utc` is Milestone 4's
  responsibility (plan §7.7) — neither milestone should assume the other does
  it.
- **`MultipleOpenStints.count` is redundant** with `open.len()`. Kept so
  Milestone 9 can render §7.3's line from the `Anomaly` value alone; droppable
  if Milestone 9 takes `&DayStints` instead.
- **No genuine ambiguity or defect blocked the work.** SPEC.md §4.3 step 1 in
  this worktree already carries the corrected kind-before-id tie-break, so
  T7b's expectation is spec-backed rather than a unilateral call.
