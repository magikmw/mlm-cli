# Task 3 report: `status` per-date anomalies and the shared lookback

Worktree branch: `worktree-agent-a36256cd014b8216a`.
Commit: `9f77224`.

## What was implemented

`src/status.rs` now builds `render::DateAnomalies` for `target_date`
from a `DateContext` (the post-splice `day` plus the three raw
`prev_raw`/`day_raw`/`next_raw` classifications) and Task 1's two new
predicates, replacing `to_anomalies`/`render::Anomalies` entirely:

- `local_time(at_utc: DateTime<Utc>) -> NaiveTime` — one-line helper,
  also applied to `build_stint_lines`'s two existing conversions and
  the `receiving_end_time` computation, factoring out the repeated
  `.with_timezone(&Local).time()` pattern per the task plan.
- `DateContext` / `classify_date_context` — the shared raw+post-splice
  classification bundle `resolve()` now builds once per target date.
- `anomaly_facts_for_date` — builds one `render::DateAnomalies`,
  filling `stale_open_start` from `stint::stale_open_start(&ctx.day,
  ..)` and both `refused_join_with_*` fields from
  `stint::is_refused_join` against the raw pairs.
- `group_punches_by_date`, `lookback_finding_for_date`, and the
  published `pub fn lookback(conn, today, now_utc) ->
  anyhow::Result<Vec<LookbackFinding>>` — the ages-2-through-14 scan,
  oldest date first, reading one `punches_in_range` call.
- `resolve()`: replaces the old two-line anomaly section with `let ctx
  = classify_date_context(..); let facts = anomaly_facts_for_date(..);
  let mut anomaly_lines = facts.detail_lines();` plus, when `is_today`,
  appending each `lookback(..)` finding's `detail_lines()`.
- `pub use crate::render::{LookbackFinding, LookbackKind};` — so Task 4
  can name these types via `crate::status::` without importing
  `render` directly.

`tests/status_cli.rs`'s `t14_status_with_anomalies_exits_zero_and_prints_to_stdout`
now asserts the new E7 wording plus both fix/or lines, per Task 2's
handoff note.

All signatures match the task plan's Published Interface verbatim,
including the exact doc comment's `lookback` signature.

## TDD sequence

Wrote the full new/changed test set (18 new `resolve()`/`lookback()`
tests in `src/status.rs`, plus the two-line `tests/status_cli.rs`
change) against the pre-Task-3 `status.rs`. `cargo build --tests`
failed with `error[E0425]: cannot find function 'lookback' in this
scope` — confirmed red. Implemented `local_time`, `DateContext`,
`classify_date_context`, `anomaly_facts_for_date`,
`group_punches_by_date`, `lookback_finding_for_date`, `lookback`, and
rewired `resolve()`; reran. Two pre-existing tests then failed on
`anomaly_lines.len()` assertions written against the old (fix-line-free)
wording — both were already flagged by the plan's own re-alignment
note as needing updates (its line numbers ~1631/~1793 correspond to
`resolve_e7_multi_open_and_e8_orphaned_end_via_anomalies_only` and
`resolve_midnight_boundary_still_flags_non_1to1_shapes`). Updated their
length assertions (2->6 and 1->4 respectively) and added an assertion
for the now-expected refused-join line on the midnight-boundary test's
non-1:1 shape (a real, correct new behavior, not a wording nit — a
1:1-failed splice is a refused join under §3.2). One of my own new
tests (`resolve_fix_lines_carry_the_real_date_for_every_anomaly_kind`)
also failed once, self-inflicted: its four anomaly-kind fixtures used
adjacent dates on one shared connection, so the E8 fixture's orphaned
end silently spliced against the stale fixture's open start on the
neighboring date. Fixed by giving each kind its own connection. All
green after that.

## Deviations from the plan

1. **Worktree was not based on the `open-punch-visibility` branch's
   tip at task start.** The worktree's branch
   (`worktree-agent-a36256cd014b8216a`) was rooted on an unrelated,
   much later branch (tip `f76ad3b`, a `status-wording-fixes`-era
   release commit), not on `open-punch-visibility`'s tip (`fb80968`).
   Per instructions, ran `git rebase open-punch-visibility` before any
   other work; clean rebase, nothing of the worktree's own to replay
   (`git merge-base --is-ancestor open-punch-visibility HEAD` confirmed
   true afterward). Worktree-setup issue only, not a plan or scope
   problem.
2. **`static EMPTY: Vec<Punch>` was not attempted; went straight to the
   plan's own stated fallback**, `let empty = Vec::new();` inside
   `lookback_finding_for_date`. The plan explicitly sanctioned this as
   an equally-valid alternative ("either compiles the same logic"), so
   this is a implementation choice within the plan's own stated
   latitude, not a deviation from it.

No published signature differs from the plan's "Published interface"
section. No file outside `src/status.rs`/`tests/status_cli.rs` was
touched.

## Final public signature (verbatim, for Task 4)

```rust
pub use crate::render::{LookbackFinding, LookbackKind};

pub fn lookback(
    conn: &rusqlite::Connection,
    today: chrono::NaiveDate,
    now_utc: chrono::DateTime<chrono::Utc>,
) -> anyhow::Result<Vec<LookbackFinding>>
```

Unchanged from the plan.

## Verification (all green)

- `cargo fmt` — clean.
- `cargo build` / `cargo build --tests` — clean.
- `cargo test` — 499 lib tests + 5 `status_cli` tests + all other
  integration suites pass.
- `cargo clippy --all-targets -- -D warnings` — clean.
- Pre-commit hook (fmt / cognitive complexity / coverage ratchet) —
  passed; coverage improved (99.085% -> 99.100%), baseline
  auto-ratcheted and included in the commit.
- Hand check: `MLM_DB_PATH=/tmp/mlm-task3-check.db cargo run -- start
  09:00 --date 2026-09-21` (5 days before the run date) then `... --
  status` shows the lookback block:
  ```
  [!] unclosed start on Mon 2026-09-21 at 09:00 (5 days ago)
      fix: mlm stop HH:MM --date 2026-09-21  (a time after 09:00)
       or: mlm delete punch --date 2026-09-21  (lists punches; rerun with the start's number)
  ```
  and `... status 2026-09-21` shows the stale-open-start line:
  ```
  [!] unclosed start at 09:00, 5 days old (no stop recorded)
      fix: mlm stop HH:MM --date 2026-09-21  (a time after 09:00)
       or: mlm delete punch --date 2026-09-21  (lists punches; rerun with the start's number)
  ```
  Both match spec §3.4 exactly.

## Open questions for the reviewer

None. The plan's own risks section (cognitive complexity of `resolve()`,
the `Punch: Sync` question) both resolved cleanly: `resolve()` and every
new helper pass clippy's complexity gate with no further splitting
needed, and the `let empty = Vec::new()` fallback sufficed without
needing to test `static EMPTY`'s `Sync` requirement at all.
