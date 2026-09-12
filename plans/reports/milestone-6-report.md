# Milestone 6 — Week accounting: completion report

Branch: `milestone-6`. Scope touched: new `src/week.rs` + one `mod week;`
line in `src/main.rs`. Nothing else.

## What was implemented

`src/week.rs`, all public to the crate, TDD (red verified before each
green step):

- **`WeekId`** — a local, clearly-marked stand-in for Milestone 2's type
  (`// TODO(integration): replace with use crate::date::WeekId;`).
  Private fields, checked `new(iso_year, week) -> Option<Self>`,
  `iso_year()` / `week()` accessors, `start()` (Monday `NaiveDate`),
  `from_date()`, `next()`. `Copy + Ord`, field order year-then-week so
  derived `Ord` is chronological. `next()` is
  `from_date(start() + Duration::days(7))` — never naive week+1.
- **`DEFAULT_WEEK_TARGET_MINUTES: i64 = 2400`**.
- **`trait WeekData`** — `earliest_data_week() -> Option<WeekId>`,
  `worked_minutes(WeekId) -> i64`. Doc comment states the
  completed-stints-only contract (SPEC §2.4, NOTES 37/38) and that
  asserting it belongs at the wave-3 seam.
- **`trait WeekTargets`** — `target_override(WeekId) -> Option<i64>`,
  with `Some(0) != None` spelled out. No `unwrap_or(0)` anywhere.
- **`WeekLedger`** — in-memory fixture implementing both traits, builder
  style (`with_worked`, `with_data_week`, `with_target`). `data_weeks`
  is a separate `BTreeSet`, not derived from `worked.keys()`.
- **`week_series(through, data, targets) -> Vec<WeekAccounting>`** — the
  real implementation. Start week = `min(earliest_data_week, through)`,
  or `through` when the ledger is empty. Loop visits *every* ISO week in
  range; no `continue`, no `filter`, no iteration over data entries.
  Each of `fulfillment` / `owed` / `carry_out` computed from its own §5
  formula. No `max`/`saturating_*`/`abs`/`clamp` (the single `min` is
  §5.1's start selection).
- **`week_accounting(week, data, targets) -> WeekAccounting`** — thin
  wrapper over the series' last element.
- **`daily_target_minutes(week_target_minutes: i64) -> i64`** —
  `div_euclid(5)`, with a comment forbidding a "simplification" to `/`.

## Result type shape (for Milestones 9/10/11 to cross-check)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeekAccounting {
    pub week: WeekId,
    pub target: i64,
    pub carry_in: i64,
    pub worked: i64,
    pub fulfillment: i64,
    pub owed: i64,
    pub carry_out: i64,
}
```

**Confirmed: there is no `is_current_week` field.** Per PLAN.md contract
3, that comparison is Milestone 9's sole job, computed from `week` plus
an injected "now".

## Verification

- `cargo build` — passes.
- `cargo test` — **28 tests, 28 passed, 0 failed.**
- `cargo fmt` — applied, clean.
- `cargo clippy --all-targets` — **zero findings in `src/week.rs`.**
  `cargo clippy --all-targets -- -D warnings` currently **fails on a
  pre-existing condition unrelated to this milestone**: three
  `dead_code` warnings for `parse_hm` / `between` / `format_duration` in
  `src/time.rs`. Verified pre-existing by stashing this milestone's work
  and re-running clippy on the clean branch — identical three errors.
  `src/time.rs` is Milestone 1's module and outside this milestone's
  scope (and Milestone 1 replaces its contents wholesale), so it was
  left untouched rather than patched into a merge conflict. Milestone 1
  landing resolves it.
- `src/week.rs` carries a module-scoped `#![allow(dead_code)]` with a
  `TODO(integration)` comment, since nothing in the binary references it
  until Milestones 9/10/11 do. Without it this module would add the same
  class of warning it is reporting above.

## Deviations from the plan (and why)

1. **No `today: NaiveDate` parameter and no `is_current_week` field.**
   The plan file still contains the pre-correction text in §2.3, §3,
   §5.2, §7.7 and §8.3. PLAN.md contract 3 is authoritative and
   explicitly says "Deliberately **no** 'is this the current week'
   boolean here ... Milestone 6 has no other reason to know 'now'".
   With the flag gone, `today` had no remaining consumer, so it was
   dropped from `week_series` / `week_accounting` entirely rather than
   left as an unused parameter. Signatures are therefore:
   `week_series(through, data, targets)` and
   `week_accounting(week, data, targets)`.
   **Plan §7.7's four `is_current_week` test cases were not written** —
   they belong to Milestone 9. The underlying correctness they were
   guarding (ISO-year-vs-calendar-year in `from_date`, Monday/Sunday
   boundaries) is still covered by
   `from_date_uses_iso_year_not_calendar_year` and
   `from_date_and_start_round_trip`.
2. **`WeekId` is a local stand-in, `pub` rather than private.** The task
   brief asked for a "private, test-only" stand-in, but it appears in
   the signatures of `pub trait WeekData` / `WeekTargets` and in
   `WeekAccounting`'s `week` field, so a private type would trip
   `private_interfaces`. It is `pub` *within* a module that is itself
   private to the binary crate, so its visibility is crate-internal
   either way. Marked with a `TODO(integration)` comment.
3. **`WeekLedger::with_*` take `mut self`, not `self`.** Same builder
   ergonomics as the plan's `self`-by-value signature; just avoids a
   redundant rebind.
4. **Added `WeekId::new` validation tests and an
   `iso_weeks_in_year` helper** that reads ISO year lengths off `chrono`
   (Dec 28) rather than trusting the plan's claim that 2026 has 53 weeks
   and 2025 has 52. Both confirmed. Per §7.9 these assertions are in the
   test suite so a wrong assumption fails loudly.
5. **Plan §7.6a's expected `carry_in`** is asserted as `-2400 * 18`, not
   19 weeks' worth: with `2026-01` exactly on target, the deficit accrues
   over weeks 02..=19, i.e. 18 empty weeks. The plan's prose ("the 19
   preceding weeks") counts weeks, not deficits; the arithmetic here
   matches §5's formulas.

## Test inventory (28)

`WeekId`: ISO year lengths verified against chrono; `next()` across a
53-week year (2026-52 → 2026-53 → 2027-01) and a 52-week year
(2025-52 → 2026-01); week 53/54/0 rejection; `from_date`/`start`
round-trip incl. Sunday; ISO-vs-calendar year (2027-01-01 → 2026-53);
chronological `Ord`.
`daily_target_minutes`: 2400→480, 2002→400, 2004→400, 1→0, 0→0.
Accounting: SPEC §5's four-row table asserted whole-row; wrapper agrees
with series tail; E12 first week `carry_in == 0` and series length 1;
default target when unset; override is non-sticky; F7b zero target with
and without worked time; `Some(0)` vs `None` side by side; F8 single gap
week whole-row; F8b absent-vs-explicit-zero series equality; two-gap
`carry_in == -4800`; E13 future week (length 20), past-before-all-data
(length 1), empty ledger, note-only anchoring week (length 2,
`carry_in == -2400`); walk across the 53-week boundary (2026-52 →
2027-02, length 4); no-clamping surplus survival (`carry_in == 2600`,
`owed == -200`); 10-week deficit chain (`carry_in == -24000`, strictly
monotonic).

## Open questions / risks for the reviewer

1. **The plan file itself is stale** on `is_current_week` (§2.3, §3,
   §5.2, §7.7, §8.3 all still describe it). It should be corrected so a
   later reader doesn't file this implementation as incomplete.
2. **Contract-9 integration debt.** When Milestone 2's `WeekId` lands,
   delete the stand-in and its `WeekId`-specific tests (keep §7.9's
   53-week-year tests, as the plan says) and re-point the `use`. Verify
   Milestone 2 names things `iso_year()`/`week()`/`start()`/`from_date()`
   /`next()` and that its `Ord` is chronological — `WeekLedger`'s
   `earliest_data_week()` depends on the `Ord` property, and the
   `BTreeSet` silently misbehaves if that changes.
3. **Plan §8.1's contract risks are untouched and still live**: whether
   Milestone 4 yields per-week or per-date totals (the plan's
   recommendation — bucket in the wave-3 adapter via a
   `WeekLedger::from_daily_totals` constructor — remains the right call
   and needs no change here), whether a `week_targets`-only week counts
   as a data week (assumed **no**, per §2.4's "punch/note data"), and
   above all that the aggregate the adapter feeds in excludes
   open-stint live minutes and orphaned `end`s. Nothing in this module
   can detect a violation of that last one; it needs an integration
   test at the seam.
4. **Pre-existing `-D warnings` failure in `src/time.rs`** (see
   Verification). Whoever integrates should confirm Milestone 1 cleared
   it rather than assume this branch introduced it.
