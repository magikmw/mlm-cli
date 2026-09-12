# Independent adversarial review: cross-cutting verification pass (wave 5)

Reviewed branch `cross-cutting-verification` at commit `2240c53` against
`plans/reports/cross-cutting-verification-report.md`'s claims for the 7
cross-cutting properties in `PLAN.md`. Verification performed independently
(not by re-reading the implementer's prose as fact): builds, tests, clippy
run directly; grep sweeps re-run from scratch; the two riskiest new tests
(DST end-to-end, E6 end-to-end) read in full and their premises checked
against real facts (actual CET/CEST offsets for the claimed 2026 dates,
actual DB-connection call graph).

## Findings

No blocking or significant issues found. All 7 claims independently
confirmed; scope is verification-only as framed.

1. `src/week_target.rs:1-6` — stale doc comment ("Wired into `main`'s
   dispatch by a later milestone... in the meantime") contradicts the
   fact that `main.rs` already dispatches to it. Confirmed present, and
   confirmed cosmetic: `#![allow(dead_code)]` is a repo-wide convention
   present in 7 of 8 top-level modules (`week.rs`, `time.rs`, `stint.rs`,
   `render.rs`, `date.rs`, `storage.rs`, `week_target.rs`), not something
   specific to this file being genuinely unwired. No functional impact.
   Severity: minor (matches the implementer's own "flagged for reviewer"
   note; not a new finding, just confirmed).

## Independent verification detail (for the record)

- **Item 1 (DST end-to-end)**: read `tests/status_cli.rs`'s
  `dst_transition_is_shown_correctly_end_to_end_via_status` in full. It
  is a genuine end-to-end test, not a superficial one: it seeds raw UTC
  instants via direct SQL (bypassing the CLI, which can only write
  "today"), invokes the real compiled `mlm` binary as a fresh subprocess
  per call with `TZ=Europe/Warsaw` set (subprocess boundary matters
  because `chrono::Local` caches the timezone offset for the process
  lifetime), and asserts the rendered local wall-clock time and duration
  match across the transition. Independently confirmed the historical
  premise is factually correct by querying the system tz database
  directly: `TZ=Europe/Warsaw date -d "2026-03-28 12:00"` → CET +0100,
  `TZ=Europe/Warsaw date -d "2026-03-30 12:00"` → CEST +0200. The test
  is exercising a real hour-offset discontinuity, not a coincidence of
  matching offsets. This is not a mocked or superficial test.

- **Item 3 (E6 end-to-end)**: read `tests/e6_db_failure.rs` in full. Both
  tests invoke the real compiled binary (`env!("CARGO_BIN_EXE_mlm")`)
  against a genuinely unopenable path (`<regular-file>/sub/mlm.db`,
  where the parent segment is a file, not a directory) — no mocking of
  `db::connect`/`connect_at`, the real code path runs. Confirmed via
  `grep` that `main.rs` has exactly one call site for `db::connect()`
  (in `dispatch`, shared by all six commands), so exercising `start` and
  `status` genuinely covers the real, shared connection path used by
  every command, not a special-cased one. Assertions check nonzero
  exit, clean process exit (no signal), non-empty stderr, empty stdout,
  and (for `start`) that the blocker file wasn't touched/turned into a
  directory. This is a real end-to-end test of the real code path.

- **Item 2 (two-tier errors)**: confirmed independently by reading
  `src/storage.rs`'s `insert_punch_with_note` (validates the note body
  before opening a transaction; both rows commit together via
  `conn.transaction()?` at line 317) and `src/commands.rs`,
  `src/status.rs`, `src/week_view.rs`, `src/week_target.rs` for anomaly
  handling — anomalies (multi-open stints, orphaned ends) are never
  checked on any write path, confirmed by grep; they surface only as
  rendered markers on the read path. Holds.

- **Item 4 (exit codes)**: confirmed `std::process::exit` has exactly one
  call site in the whole `src/` tree (`main.rs:19`), and every command
  handler returns `anyhow::Result<()>`, funneled through `main.rs`'s
  `exit_code` (`Ok` → 0, `Err` → 1). Holds.

- **Item 5 (plain-ASCII sweep)**: read `tests/ascii_sweep.rs` in full.
  Seeds a realistic mixed week (ordinary stint, multi-open anomaly,
  orphaned-end anomaly, ASCII note, non-ASCII note, non-default week
  target) through real SQL + the real binary for both `status` (4 dates)
  and `week`, concatenates output, carves out only the one legitimately
  non-ASCII note body (asserted present verbatim, not just excluded),
  and sweeps every remaining byte for `0x20..=0x7E` or `\n`. Genuine
  test, not a tautology — it also sanity-asserts the anomaly marker,
  note text, and "Target:" line are actually present, so it can't pass
  on an accidentally-empty sweep.

- **Item 6 (duration formatting)**: re-ran the grep myself —
  `format_minutes` is the only call site producing `Hh MMm`-shaped
  output across `render.rs`, `status.rs`, `week_view.rs`; the only other
  hand-written `h`/`m`-shaped pattern is a test assertion comparing
  against the canonical format, not a second formatter. Holds.

- **Item 7 (now injectability)**: re-ran
  `grep -rn "Utc::now()\|Local::now()\|Local::today()" src/` myself.
  Exactly one live call, `src/main.rs:42` (`Local::now()`); the other
  two matches are doc comments in `render.rs` and `storage.rs`
  explicitly documenting the rule, not violations. Holds.

- **Build/test/lint**: ran directly, not taken on faith.
  - `cargo build --all-targets`: clean.
  - `cargo test`: 307 (lib) + 1 (`ascii_sweep.rs`) + 2 (`e6_db_failure.rs`)
    + 4 (`status_cli.rs`, including the new DST test) = **314 passed, 0
    failed**, matching the claimed count exactly.
  - `cargo clippy --all-targets -- -D warnings`: clean (re-verified after
    `touch src/main.rs` to force a real recompile rather than trust a
    cached "Finished").

- **Scope discipline**: `git diff` across the wave-5 commit
  (`3ffda1e..HEAD`) touches zero files under `src/` — only 3 new test
  files (`tests/ascii_sweep.rs`, `tests/e6_db_failure.rs`, additions to
  `tests/status_cli.rs`) plus the report. No new dependencies added
  (`tempfile`, `rusqlite` were already project dependencies). This
  matches the "verification + tests only, no production-code changes"
  framing exactly — arguably even more conservative than "small surgical
  fixes," since no fixes were needed at all.

- **DB-connection / `MLM_DB_PATH` consistency (independent hunt beyond
  the 7-item list)**: checked whether the six commands might each open
  the DB differently. They don't — `dispatch` in `main.rs` calls
  `db::connect()` exactly once, before matching on the command, and
  every command handler receives the already-open `&mut Connection`.
  There is no per-command connection logic to be inconsistent. No
  cross-cutting gap found here.

## Verdict

**APPROVE** — all 7 cross-cutting properties independently confirmed to
hold (two via genuine new end-to-end tests that exercise the real
subprocess/binary/connection path, not mocks; five via re-run greps and
direct code reading); build, full test suite (314/314), and clippy all
pass as claimed; the diff is strictly test-and-report, no production code
touched, consistent with a verification-only pass.
