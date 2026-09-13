# Cross-cutting verification report

Wave-5 verification-only pass over the merged `mlm` codebase (branch
`cross-cutting-verification`). No new behavior was added; findings below
per PLAN.md's "Cross-cutting concerns" section.

## 1. DST-safe per-instant conversion (§2.1) — end-to-end through `status`

**Did not previously exist at this scope.** Milestone 4's `src/storage.rs`
already covers the storage layer (`dst_spring_forward_different_dates_
use_own_offsets`, `dst_order_holds_across_transition_within_one_date`),
but nothing exercised the *display* path through `status`.

Added `dst_transition_is_shown_correctly_end_to_end_via_status` in
`tests/status_cli.rs`. It seeds two completed stints directly via SQL
(the CLI can only write "today", so historical dates must be seeded
below the CLI) straddling Europe/Warsaw's real 2026 spring-forward
(2026-03-28, still CET/UTC+1, vs. 2026-03-30, already CEST/UTC+2), then
invokes the compiled `mlm status` binary with `TZ=Europe/Warsaw` set on
the child process (chrono's `Local` reads `TZ` once per process, so a
fresh subprocess per invocation is what makes this deterministic) and
asserts both dates render the same local wall-clock stint
(`12:00-13:00`, `01h 00m`) despite the hour of UTC-offset difference.
Passes.

## 2. Two-tier error handling (§6) — hard errors vs. anomalies

**Holds, spot-checked across all six commands.** In every handler
(`commands::punch`/`note`, `week_target::run`, `status::run`,
`week_view::run`), all parsing/validation that can hard-error (`TIME`,
`DATE`, `WEEK_ID`, `DURATION`, empty/whitespace note bodies) happens
before any write, and writes that do happen (`insert_punch_with_note`)
are wrapped in one transaction so a note-validation failure can't leave
an orphaned punch. Anomalies (multi-open stints, orphaned ends) are
never checked against on the write path at all — `insert_punch` just
inserts — so they can never block a write, and the read path
(`status`/`week`) renders them as extra output rather than an error.
No local error-tier reimplementation found; no fix needed.

## 3. E6 end-to-end (DB open/migration failure through a real command)

**Gap confirmed and fixed.** Only `src/db.rs`'s schema-layer unit tests
(`connect_at_errors_when_parent_path_is_a_regular_file`, etc.) exercised
this; nothing drove it through an actual command invocation.

Added `tests/e6_db_failure.rs` with two binary-level tests
(`e6_db_open_failure_surfaces_through_start`,
`..._through_status`), each pointing `MLM_DB_PATH` at
`<file>/sub/mlm.db` (a file where a directory is expected — mirrors the
existing unit test's setup). Both confirm: nonzero exit, a clean process
exit (no signal/panic), a non-empty stderr message, empty stdout, and
(for `start`) no write occurs. Both pass.

## 4. Exit codes (§6.3)

**Holds**, spot-checked with the real binary for `week` and `week
target` (the last commands to land) in addition to the existing
`status`/`week` unit- and binary-level coverage:

```
mlm week bogus                    -> exit 1, stderr message
mlm week target 2026-07 -5h       -> exit 1, stderr message
mlm week 2026-07 (empty week)     -> exit 0
mlm week target 2026-07 10h       -> exit 0
```

`main::exit_code` is the single dispatch point (`Ok -> 0`, `Err -> 1`)
and every command handler returns `anyhow::Result<()>` through it — no
command calls `std::process::exit` itself. No fix needed.

## 5. Plain-ASCII output (§7) — dedicated sweep

**Did not previously exist at the "both commands together, full
realistic output" scope.** Existing tests (`src/status.rs`'s
`t13_plain_ascii_output_and_non_ascii_notes_pass_through`,
`src/week_view.rs`'s `a11_plain_ascii_audit` / `c12_plain_ascii_over_
real_output`) each byte-sweep one command's *own* curated fixtures.

Added `tests/ascii_sweep.rs`: seeds a realistic week (an ordinary
completed stint, a multi-open anomaly, an orphaned-end anomaly, an
ASCII note, a non-ASCII note, a non-default week target), drives
`status` over four dates and `week` over the whole week through the
real binary, concatenates all stdout, and sweeps every byte for
`0x20..=0x7E` or `\n` — with the one known non-ASCII, user-typed note
body (SPEC.md explicitly permits these) carved out first and separately
asserted to appear verbatim rather than silently exempting the whole
sweep. Passes.

## 6. Duration formatting reuse (§4.2)

**Holds.** Grepped every call site that renders a duration
(`src/render.rs`, `src/status.rs`, `src/week_view.rs`) — all go through
`crate::time::format_minutes`. The one other `Hh`/`MMm`-shaped check
found (`src/status.rs`'s `assert_durations_well_formed` test helper) is
a *test assertion* that a rendered string matches the canonical format,
not a second formatter. No reimplementation found; no fix needed.

## 7. "Now" injectability

**Holds.** `grep -n "Utc::now()\|Local::now()" src/*.rs` finds exactly
one call, in `main.rs` (plus two doc-comments referencing the rule).
Every command handler and rendering function takes `now`/`today` as a
parameter. No fix needed.

## Test count / pass-fail

- `cargo test`: 307 (lib unit tests) + 4 (`tests/status_cli.rs`,
  including the new DST test) + 2 (`tests/e6_db_failure.rs`, new) + 1
  (`tests/ascii_sweep.rs`, new) = **314 tests, all passing**.
- `cargo build`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean, no warnings.
- `cargo fmt`: applied, no diffs left outstanding.

## Flagged for the reviewer

Nothing rises to the level of a design ambiguity or defect. One minor,
non-blocking observation: `src/week_target.rs` still carries a
module-level `#![allow(dead_code)]` and a doc comment saying it's
"Wired into `main`'s dispatch by a later milestone... in the meantime"
— but `main.rs` already dispatches to it (`Command::Week` ->
`WeekAction::Target` -> `week_target::run`), so the comment is stale.
Left as-is since it's cosmetic (a stale comment, not a behavior gap) and
outside this pass's "small, surgical fixes only" mandate for anything
beyond the 7 listed items — worth a one-line cleanup whenever that file
is next touched.
