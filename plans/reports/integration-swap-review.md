# Independent adversarial review: wave-3 integration swap (Milestones 5/6)

Reviewed commit `f8a01c9` on branch `integration-swap`, against
`plans/reports/integration-swap-report.md`, `PLAN.md` contracts 8/9,
`src/storage.rs`, `src/date.rs`, `src/stint.rs`, `src/week.rs`.

## Findings

- No blocker or significant findings. This is a clean, correctly-scoped
  mechanical swap.

- `src/stint.rs`: swap is exactly as claimed — the local `Punch`/`PunchKind`
  stand-in (and its `TODO(integration)` comment) is deleted, replaced by
  `use crate::storage::{Punch, PunchKind};`. `storage::Punch`'s fields
  (`id: i64`, `at_utc: DateTime<Utc>`, `date: NaiveDate`, `kind: PunchKind`)
  and `storage::PunchKind`'s variants/derives (`PartialOrd, Ord` with
  `Start < End`, needed by `classify()`'s `(at_utc, kind, id)` sort key at
  stint.rs:128) match contract 8 field-for-field. No leftover references to
  a local stand-in type anywhere in `src/stint.rs`. Severity: n/a (clean).

- `src/week.rs`: swap is exactly as claimed — the local `WeekId` stand-in
  (struct, `new`/`iso_year`/`week`/`start`/`from_date`/`next`, and its
  `TODO(integration)` comment) is deleted, replaced by
  `use crate::date::WeekId;`. `date::WeekId` has private fields, `start()`,
  `from_date()`, `next()`, `iso_year()`/`week()`, and `Copy + Ord` with
  year-then-week chronological order — matching contract 9 exactly except
  for `new()`'s signature, which the report explicitly flags. Severity: n/a
  (clean).

- Report's characterization of the `WeekId::new` signature difference is
  accurate, verified directly against `src/date.rs:181`
  (`pub fn new(iso_year: i32, week: u32, input: &str) -> Result<Self,
  DateWeekError>`). Confirmed by grep that no production code path in
  `src/week.rs` calls `WeekId::new` — the only production constructors used
  are `from_date()` (infallible) and `next()` (infallible, built on
  `from_date`). Only two test call sites needed adaptation: the `wk()` test
  helper (week.rs:235-237) and `week_53_of_a_52_week_year_is_not_constructible`
  (week.rs:292-296). This matches the report's claim precisely.

- Test-weakening check: the adapted test
  `week_53_of_a_52_week_year_is_not_constructible` still asserts all three
  original invalid constructions (`(2025, 53)`, `(2026, 54)`, `(2026, 0)`)
  are rejected, just via `.is_err()` instead of `== None`. Same three cases,
  same coverage, no premise lost. No test was deleted or silently narrowed.
  This is a faithful, honest adaptation, not a workaround. Severity: n/a
  (confirmed correct).

- Design-mismatch question (does `week.rs` anywhere assume `WeekId`
  construction can't fail in a way that matters?): no. `week.rs`'s only
  production-path construction is via `from_date()`/`next()`, both
  infallible by construction (they derive from an already-valid `NaiveDate`
  via `iso_week()`, which cannot produce an out-of-range week/year pair).
  `week.rs` has no error-handling path around `WeekId` construction at all
  today (correctly — it never constructs one from raw ISO numbers). The
  design mismatch the report flags is real but inert until a later
  milestone (9/10/11, e.g. a CLI `--week` flag) adds a caller that builds a
  `WeekId` from user-typed numbers; at that point the caller will need to
  supply a real `input` string and propagate/convert `DateWeekError` rather
  than assume an `Option`. Flagging this as a forward-looking note, not a
  defect in the current diff.

- Minor / worth a follow-up glance (not a defect in this change): the
  `input: &str` parameter on `WeekId::new` exists to populate
  `DateWeekError::input` for user-facing messages (per `date.rs:180`'s own
  doc comment). The two test call sites now pass the literal `"test"` as
  that input. This is harmless (tests never inspect the error's `input`
  field) but is worth noting for whoever writes Milestone 9/10/11's CLI
  parsing tests later — they should pass the actual user-typed string, not
  copy the `"test"` placeholder pattern from `week.rs`'s tests. Severity:
  minor (informational only, no action needed in this diff).

- Scope check: `git show --stat f8a01c9` shows only three files touched —
  `plans/reports/integration-swap-report.md` (new), `src/stint.rs`, and
  `src/week.rs`. No changes to `src/storage.rs`, `src/date.rs`,
  `src/main.rs`, `Cargo.toml`, or any other module. Both files' own
  `#[cfg(test)]` chrono-import reshuffling (moving `NaiveDate` into
  stint.rs's test module; moving `Datelike`/`Duration`/`NaiveDate`/`Weekday`
  into week.rs's test module) is confined to each file's own test module
  and is the minimal fallout of deleting the stand-in types, not scope
  creep. `#![allow(dead_code)]` and its `TODO(integration): drop this once
  Milestones 9/10/11 consume the module` comment are correctly left in
  place in both files (verified no other module references `stint::` or
  `week::` items yet, matching the report's claim). No `TODO(integration)`
  comments referring to *this* swap (the Punch/PunchKind/WeekId stand-ins)
  remain anywhere in `src/`.

## Verification performed independently

- `cargo build`: clean, 0 warnings.
- `cargo test`: **191 passed; 0 failed; 0 ignored** — matches the report's
  claim exactly.
- `cargo clippy --all-targets -- -D warnings`: clean, 0 warnings.
- `cargo fmt -- --check`: clean, no diff.
- `git show --stat f8a01c9`: confirms only `src/stint.rs`, `src/week.rs`,
  and the new report file changed.

## Verdict

APPROVE — mergeable as-is. The swap is clean, correctly scoped, the
report's characterization of the one judgment call (`WeekId::new`'s extra
`input` parameter and `Result` vs `Option`) is accurate and its
"production code never calls `new()` directly" claim holds under
inspection, the one adapted test preserves its original coverage rather
than being weakened, and build/test/clippy/fmt all pass as claimed.
