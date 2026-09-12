# Wave-3 integration swap: Milestones 5 and 6

Swapped the wave-1 test-only fixture stand-ins for the real wave-2 types,
per PLAN.md contracts 8 (Punch/PunchKind) and 9 (WeekId).

## Task A — `src/stint.rs` (Milestone 5)

Replaced the local `Punch`/`PunchKind` stand-in with
`use crate::storage::{Punch, PunchKind};` (both `pub` in `src/storage.rs`).

**Clean swap.** `storage::Punch`'s fields (`id: i64`, `at_utc: DateTime<Utc>`,
`date: NaiveDate`, `kind: PunchKind`) and `storage::PunchKind`'s variants
(`Start`, `End`) match the stand-in field-for-field, exactly as PLAN.md
contract 8 promised. `storage::PunchKind` additionally derives `Hash`
(harmless superset) and comes with `as_str`/`from_str` helpers `stint.rs`
doesn't use. No logic in `stint.rs` changed.

Only follow-up needed: the top-level `use chrono::{DateTime, NaiveDate, Utc}`
lost its only non-test use of `NaiveDate` once the local `Punch` definition
was deleted, so `NaiveDate` was dropped from the top-level import and added
to the `#[cfg(test)]` module's own `chrono` import instead (the tests still
build `NaiveDate` values directly for cross-midnight fixtures).

## Task B — `src/week.rs` (Milestone 6)

Replaced the local `WeekId` stand-in with `use crate::date::WeekId;`
(`pub` in `src/date.rs`).

**Needed adaptation — not a pure mechanical swap.** The stand-in's own
comment described the real type's constructor as `pub fn new(iso_year: i32,
week: u32) -> Option<Self>`. The real `date::WeekId::new` is actually:

```rust
pub fn new(iso_year: i32, week: u32, input: &str) -> Result<Self, DateWeekError>
```

— a third `input: &str` parameter (populates `DateWeekError::input` for a
user-facing message) and `Result` instead of `Option`. Everything else lines
up with the stand-in exactly: private fields, `start()`, `from_date()`,
`next()`, `iso_year()`/`week()` accessors, `Copy + Ord` with chronological
(year-then-week) order.

Judgment call: I treated this as a trivial adaptation rather than a design
mismatch worth stopping over, because:
- No production code in `week.rs` calls `WeekId::new` directly — only test
  fixtures do (`week.rs`'s own `wk()` helper and one negative-construction
  test). The production path only ever builds a `WeekId` via `from_date()`
  and `next()`, both of which match the stand-in's signatures exactly.
- The extra `input` parameter and `Result`-with-error (vs `Option`) is
  `date.rs`'s own established pattern for surfacing `§6.1` hard errors with
  the offending input text — a deliberate, documented design choice in
  Milestone 2, not an accidental drift from contract 9's shape.

Adapted two things in the test module only:
- `wk(year, week)` test helper now calls `WeekId::new(year, week, "test")`
  and still `.expect(...)`s (mirrors `date.rs`'s own test helper of the same
  name/shape).
- `week_53_of_a_52_week_year_is_not_constructible` asserted
  `WeekId::new(...) == None`; changed to `.is_err()` since the real type
  returns `Result`, not `Option`. This preserves the exact same coverage
  (three invalid constructions all rejected) without depending on the
  stand-in's `Option` API.

No other test was deleted or had its premise invalidated — all of
`week.rs`'s 27 pre-existing tests still make sense unchanged against the
real, validated `WeekId` and continue to pass.

Also moved `Datelike`/`Duration`/`NaiveDate`/`Weekday` chrono imports that
were only used by the now-deleted stand-in (or only by tests) down into the
`#[cfg(test)]` module; `BTreeMap`/`BTreeSet` remain a top-level import
(still used by `WeekLedger`).

## Build / test / lint results

- `cargo build`: clean, no warnings, after both swaps.
- `cargo test`: **191 passed; 0 failed; 0 ignored** (full suite, all modules).
- `cargo clippy --all-targets -- -D warnings`: clean, no warnings.
- `cargo fmt -- --check`: clean (no diff needed after `cargo fmt`).

## `#![allow(dead_code)]`

Left in place in both `src/stint.rs` and `src/week.rs`. Verified with a
grep across `src/*.rs` that nothing outside those two files' own modules
references `stint::` or `week::` items yet — `main.rs` declares both
modules but nothing calls into them until later milestones (9/10/11), so
the allow is still genuinely load-bearing and removing it would reintroduce
dead-code warnings under `-D warnings`.

## Flag for review

The one judgment call in this task is the `WeekId::new` signature/return-type
mismatch in Task B, described above. It is not a rename-only fix (a third
argument was added and `Option` became `Result`), but I judged it a benign,
intentional refinement of contract 9 rather than a contract violation,
because production code in `week.rs` never called the fallible constructor
directly. Worth a second pair of eyes if `WeekId::new` is expected to gain
callers elsewhere in `week.rs` in a later milestone — at that point the
caller will need to supply a real `input` string and handle `DateWeekError`
rather than `Option::None`.
