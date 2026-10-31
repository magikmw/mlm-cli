# Task 4 report: `start` stale-start warning + `render::Anomalies` cleanup

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

Worktree branch: `worktree-agent-a2eb9012afd3a0808`.
Commit: none — the pre-commit coverage-ratchet gate blocked the commit
(see "Blocked" below). All changes are staged but uncommitted.

## Worktree setup

The worktree's branch was rooted on an old, unrelated tip (`f76ad3b`,
a `status-wording-fixes`-era release commit), not on
`open-punch-visibility`'s tip (`914ba4a`). Ran `git rebase
open-punch-visibility` before any other work — a clean fast-forward,
nothing of the worktree's own to replay. `git merge-base
--is-ancestor open-punch-visibility HEAD` confirmed true afterward.

## What was implemented

### Job 1 — `src/commands.rs`: §3.6 stale-start warning

- `punch()`'s return type changed from `anyhow::Result<()>` to
  `anyhow::Result<NaiveDate>`, returning the resolved `target_date` it
  already computed. `stop()` discards it; `start()` uses it.
- `start()` now runs the warning as a strictly-after, best-effort step
  once the write has succeeded:
  ```rust
  pub fn start(conn: &mut Connection, now: DateTime<Local>, args: &PunchArgs) -> anyhow::Result<()> {
      let written_date = punch(conn, now, PunchKind::Start, args)?;
      warn_stale_starts(
          conn,
          now.date_naive(),
          now.with_timezone(&Utc),
          written_date,
          &mut std::io::stderr(),
      );
      Ok(())
  }
  ```
- New private helpers, exactly per the plan's split:
  ```rust
  fn warn_stale_starts<W: std::io::Write>(
      conn: &Connection,
      today: chrono::NaiveDate,
      now_utc: DateTime<Utc>,
      written_date: chrono::NaiveDate,
      out: &mut W,
  );

  fn emit_warnings<W: std::io::Write>(
      findings: Vec<render::LookbackFinding>,
      written_date: chrono::NaiveDate,
      out: &mut W,
  );
  ```
  `warn_stale_starts` calls `status::lookback(conn, today, now_utc)`,
  swallowing an `Err` via a `let ... else { return }`; `emit_warnings`
  filters out the one written-date/`SingleOpenStart` case and writes
  every other finding's `finding.stderr_line()`.

Both new top-level imports added: `use crate::render;` and `use crate::status;`.

### Job 2 — `src/render.rs`: delete the superseded `Anomalies` type

Grepped `\bAnomalies\b` across `src/` and `tests/` first. Every hit
outside `render.rs` itself was a stale doc comment in `src/status.rs`
(lines 22, 24, 26, 148) naming the old type/a function (`to_anomalies`)
that no longer exists in that file — not a real usage. Per this task's
scope, `status.rs` is off-limits, so those comments were left as-is;
flagged below as an open item for whoever next touches `status.rs`.

Deleted from `src/render.rs`:
- `pub struct Anomalies { open_stint_count, orphaned_end_times }` and
  `impl Anomalies { has_any, detail_lines, row_marker }`.
- Tests `t16`-`t32` (all of the "4.8 anomaly detail lines" / "4.9
  has_any / row_marker" / "4.10 no-divergence invariant" / "4.11 row
  composition sanity" sections) and the `fixtures() -> Vec<Anomalies>`
  helper — all existed only to exercise the deleted type.
- `t33_every_produced_string_is_ascii`'s `for a in fixtures() { ... }`
  loop, since `fixtures()` no longer exists; the headline-ASCII
  assertions in the same test are untouched (that test wasn't
  dedicated to `Anomalies` alone, so it was edited rather than
  deleted).
- Three doc-comment mentions of "the old `Anomalies`" on
  `DateAnomalies`'s fields/`row_marker` (now a nonexistent type, so the
  comparison no longer means anything) — reworded to drop the
  now-dangling reference, keeping the rest of each comment.
- The now-orphaned `// ---- 4.12 plain-ASCII audit ----` section-header
  comment, which previously sat directly above `t33` but, after the
  `t16`-`t32` deletion, ended up sitting above the unrelated
  `DateAnomalies`/`LookbackFinding` section instead (that section
  already has its own header comment).

Post-deletion grep confirms zero remaining `Anomalies` (struct/impl)
references anywhere in `src/`/`tests/`, `cargo build`/`cargo test`
both clean.

## TDD sequence (job 1)

Added the full test set (`stale_warning_lines_are_oldest_first_and_match_stderr_line`,
`start_stdout_stays_empty_and_exit_is_zero`,
`no_warning_for_just_written_date_with_single_open_start`,
`multi_open_warning_on_written_date_still_prints`,
`no_warning_when_window_is_clean`,
`read_failure_after_write_is_best_effort`) to `src/commands.rs`'s
existing test module before writing `warn_stale_starts`/`emit_warnings`.
`cargo build --tests` failed with 5x `error[E0425]: cannot find
function 'emit_warnings'/'warn_stale_starts' in this scope` —
confirmed red. Implemented the two functions and the `start()`/`punch()`
signature changes; reran — all new tests passed first try, and every
pre-existing test in `commands.rs` still passed unchanged (confirms
`punch()`'s return-type change didn't disturb `stop()`'s or any other
caller's behavior).

The plan's own anticipated risk — whether the read-failure test would
survive to the real implementation — resolved in the test's favor:
`status::lookback` propagates `storage::punches_in_range`'s error via
`?`, so `DROP TABLE punches` on the in-memory test connection produces
a genuine `Err`, no fallback comment-only path needed.

## Deviations from the plan

None in the implemented logic — every signature matches the plan's
"Design" section verbatim (`warn_stale_starts`, `emit_warnings`, the
`punch()`/`start()`/`stop()` changes). The two deviations below are
process/tooling, not scope or design:

1. **Worktree rebase**, described above under "Worktree setup".
2. **`render.rs` cleanup touched three doc comments and one
   section-header comment beyond the plan's literal "delete the
   struct/impl/tests" instruction.** The plan's own steps (1-4) only
   name the struct, impl, and dedicated tests. Left as originally
   written, the doc comments on `DateAnomalies` would reference a type
   (`Anomalies`) that no longer exists anywhere in the crate, and the
   orphaned section-header comment would sit above unrelated content.
   Both are direct, minimal fallout of the deletion this task's scope
   explicitly assigns to `render.rs`, not an unrelated change — no
   other `render.rs` content was touched.

## Final public signatures (verbatim)

```rust
pub fn start(conn: &mut Connection, now: DateTime<Local>, args: &PunchArgs) -> anyhow::Result<()>
pub fn stop(conn: &mut Connection, now: DateTime<Local>, args: &PunchArgs) -> anyhow::Result<()>

fn punch(conn: &mut Connection, now: DateTime<Local>, kind: PunchKind, args: &PunchArgs) -> anyhow::Result<chrono::NaiveDate>

fn warn_stale_starts<W: std::io::Write>(
    conn: &Connection,
    today: chrono::NaiveDate,
    now_utc: DateTime<Utc>,
    written_date: chrono::NaiveDate,
    out: &mut W,
)

fn emit_warnings<W: std::io::Write>(
    findings: Vec<render::LookbackFinding>,
    written_date: chrono::NaiveDate,
    out: &mut W,
)
```

`start`'s and `stop`'s public signatures are unchanged from before this
task (no `main.rs` call-site edit needed or made).

## Verification

- `cargo fmt` — clean (`cargo fmt --check` passes).
- `cargo build` / `cargo build --tests` — clean.
- `cargo test` — all 494 lib tests + all integration suites
  (`ascii_sweep`, `e6_db_failure`, `status_cli`) pass; `main.rs`'s own
  2 tests pass.
- `cargo clippy --all-targets -- -D warnings` — clean.
- Hand check, `MLM_DB_PATH=/tmp/mlm-task4-check.db`:
  - `cargo run -- start "09:00" --date 2026-09-21` (5 days before the
    run date 2026-09-26) — exit 0, no warning (nothing stale yet on
    that first write).
  - `cargo run -- start` (today, no args) — exit 0, stdout empty,
    stderr:
    ```
    warning: unclosed start on Mon 2026-09-21 at 09:00 (5 days ago); fix: mlm stop HH:MM --date 2026-09-21 (a time after 09:00)
    ```
    Matches spec §3.6's worked example shape exactly (single-line
    `warning: ...; fix: ...` form, correct weekday/date/age/time).

## Blocked: pre-commit coverage-ratchet gate

`git commit` was refused by `.githooks/pre-commit`'s coverage check:

```
==> quality gate: cargo fmt
    OK
==> quality gate: cognitive complexity
    OK (no functions exceed the complexity threshold)
==> quality gate: test coverage (vs. baseline)
    FAIL: coverage would drop from 99.1140125903474% to 99.10356216088701%
```

Diagnosis: this is the exact "deleting tested code can shift coverage"
scenario the task brief names, not an undertested feature. Comparing
`cargo llvm-cov`'s per-file report before/after:

- `render.rs` shrank from 1234 to 936 lines (deleted the fully-covered
  `Anomalies` struct/impl/tests, ~298 lines) — still 100.00% line
  coverage after deletion (0 missed lines), so nothing here is
  undertested.
- `commands.rs` grew from 1692 to 1844 lines (net +152, this task's new
  code). The uncovered-line count in `commands.rs` (18 missed lines)
  is identical before and after — all 18 are pre-existing, unreachable
  `other => panic!(...)` match arms inside test-fixture helpers
  (`punch_args`, `stop_args`, `note_args`, etc. — 14 such arms,
  `grep -c "other => panic"` is 14 both before and after this task's
  edit). None of this task's new production code
  (`warn_stale_starts`/`emit_warnings`/the `start`/`punch` signature
  changes) appears among the missed lines; all of it is exercised by
  the new tests.
- Net effect: total crate line count dropped (1234+1692 -> 936+1844,
  -146 lines overall) while the same fixed number of pre-existing dead
  branches remained uncovered, so their share of the total denominator
  went up slightly, moving the ratio from 99.1140% to 99.1036%
  (confirmed deterministic on two separate `cargo llvm-cov` runs, not
  measurement noise).

This is a real, if narrow, coverage regression under the ratchet's own
arithmetic, caused entirely by shrinking a well-tested file rather than
by anything undertested in this task's own new code. Per the task
brief's explicit instruction ("if the ratchet blocks, report it rather
than bypassing with --no-verify"), the commit was not forced through.

**Left for the reviewer to decide**: either accept the baseline's small
downward adjustment for this specific, understood cause (the ratchet's
own design intends to allow this via manual review, since it "never
lowers automatically" but a human can), or have this task add coverage
elsewhere in `commands.rs` to compensate — which would mean deleting or
restructuring the pre-existing unreachable `other => panic!` arms in
the test helpers themselves (out of this task's stated scope, and not
a `render.rs`/`commands.rs`-production-code change either way, since
those arms live in `#[cfg(test)]` fixture code, not the functions this
task was asked to touch).

## Open questions for the reviewer

1. The coverage-ratchet block above — how to proceed (see "Left for
   the reviewer to decide").
2. `src/status.rs`'s four stale doc-comment mentions of
   `render::Anomalies`/`to_anomalies` (lines 22, 24, 26, 148) — both
   name things that no longer exist in the crate. Out of this task's
   file scope (`status.rs` belongs to Task 3), flagged for whoever
   next touches that file's module doc comment.
