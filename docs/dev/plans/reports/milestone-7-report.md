# Milestone 7 completion report — `start`, `stop`, `note` commands

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.


## What was implemented

- `src/cli.rs`: replaced the scaffold's `Command::Start { note }`,
  `Command::Stop { note }`, and `Command::Log` with `Command::Start(PunchArgs)`,
  `Command::Stop(PunchArgs)`, `Command::Note(NoteArgs)` per the plan's §1.2
  shapes. `Log` is gone outright (regression test `log_subcommand_is_gone`).
  Added the crate-wide `--verbose`/`-v` global flag to `Cli` (PLAN contract 12).
  `Command::Week(WeekArgs)` and its `WeekArgs`/`WeekAction`/`WeekTargetArgs`
  supporting types are untouched.
- `src/commands.rs` (new): `start`, `stop`, `note` handlers, each
  `anyhow::Result<()>`, taking an injected `now: DateTime<Local>` and never
  reading the clock themselves. `start`/`stop` share a `punch()` helper
  parameterized by `PunchKind`. All three delegate the actual write to
  Milestone 4's `storage::insert_punch_with_note` / `storage::insert_note`,
  which already implement: TIME-before-NOTE validation ordering, the
  empty/whitespace-note rejection, and the punch+note atomic transaction with
  rollback-on-failure (§6.1/§4.1 of the plan). `commands.rs` itself only
  resolves `TIME` (via `time::parse_time`) vs. "now", joins the note tokens,
  and calls into storage.
- `src/main.rs`: `run() -> i32` / `process::exit`, no more `.expect()` on
  `db::connect()`. `--verbose` initializes `env_logger` before any dispatch.
  `dispatch()` adds `Start`/`Stop`/`Note` arms; `Command::Week` is left as the
  pre-existing `todo!()` (see deviation below).

## Deviations from the plan (with justification)

1. **No local `EmptyNoteError` type.** The plan pre-dated Milestone 4 landing
   and proposed a milestone-local `EmptyNoteError`. Milestone 4's real
   `storage::StorageError::EmptyNote` already exists, already implements
   `Display`/`Error`, and is what `insert_note`/`insert_punch_with_note`
   actually return. Introducing a second, redundant "note is empty" type would
   just duplicate the concept. Tests downcast to `storage::StorageError` instead.
2. **Atomicity/validation-ordering logic lives in `storage.rs`, not `commands.rs`.**
   The plan (written before M4 landed) expected M7 to open the
   `rusqlite::Transaction` and call `insert_punch`/`insert_note` in sequence
   itself. The real M4 API already exposes `insert_punch_with_note`, which does
   exactly this (validate note body before opening a transaction, punch then
   note inside one transaction, rollback via `Drop` on early return). Milestone
   7's handlers are therefore thinner than planned — they resolve `TIME`/`now`
   and hand off to that one call. This is a straightforward "use the real API"
   substitution, not a behavior change; the underlying guarantees (E1 order, E5
   no-orphan, exit 0 on success) are all still true and tested.
3. **Tests live inline in `src/commands.rs` and `src/cli.rs` (`#[cfg(test)] mod
   tests`), not in a new `tests/write_commands.rs`.** The crate has no `lib.rs`
   target (binary-only), so a `tests/` integration test cannot see
   `crate::commands`/`crate::cli` items at all. Every other milestone
   (`storage.rs`, `db.rs`, `time.rs`, `week_target.rs`, `cli.rs` itself) already
   tests in-module for this reason — followed that established convention
   instead of the plan's guess.
4. **T22 (transactional-rollback mechanism test) not duplicated.** Milestone
   4's own test suite already exercises this exact mechanism directly against
   `insert_punch_with_note` (`insert_punch_with_note_rejects_empty_note_leaving_no_orphaned_punch`,
   `insert_punch_with_note_commits_both_rows_together` in `src/storage.rs`).
   Since M7 calls that function verbatim with no additional transaction logic
   of its own, re-deriving the same mechanism test at the M7 layer would be
   redundant; M7's own suite instead asserts the outcome (T18/T19/T20) at the
   handler level.
5. **`Command::Week` dispatch left as the pre-existing `todo!()`.** Milestone
   8's report explicitly left it as a stub pending Milestone 7/11's wiring
   pass; the match still compiles fine with `Start`/`Stop`/`Note` arms added
   alongside it, so there was no need to give it a real implementation (that's
   Milestone 11's `week`-view scope, and `week target`'s scope wiring — neither
   is this milestone's job per the task's explicit instructions). `week`/`week
   target` remain unreachable via the CLI in this branch, unchanged from
   Milestone 8's state.
6. **TDD workflow note:** given the scope and the fact that Milestone 4 already
   provides fully-formed, transaction-safe storage primitives, the handler
   logic in `commands.rs` ended up thin enough that tests and implementation
   were authored together in one pass per function rather than as strict
   isolated red/green cycles per test case. Every test was run and observed
   passing only after the corresponding implementation existed; no test was
   left unexercised.

## Test / build / lint results

- `cargo build`: clean.
- `cargo test`: **218 passed, 0 failed, 0 ignored** (full workspace, including
  all pre-existing milestone suites plus this milestone's new tests in
  `src/commands.rs` and `src/cli.rs`).
- `cargo clippy --all-targets -- -D warnings`: clean, no warnings.
- `cargo fmt`: applied.
- Manual smoke test against a temp `MLM_DB_PATH`: `start 9:05 "note"`,
  `stop 17:30`, `note "standalone"` all exit 0 with silent stdout (per SPEC
  §7.4); `start abc` and `start 9:05 "   "` exit 1 with a one-line stderr
  message; `note` with no argument exits 2 (clap); `log` is an unrecognized
  subcommand (exit 2). Verified rows land correctly via `sqlite3`.

## Open questions / risks for the reviewer

- Plan §8.1's known gap stands as documented: `mlm start "some note"` (no
  TIME) is not expressible — it hard-errors as a malformed TIME, per the
  plan's own recommended resolution. Not changed here; flagged as a possible
  future spec amendment, not implemented.
- Plan §8.3 (success output) was resolved as **total silence on success**
  (nothing printed to stdout, exit 0), matching SPEC §7.4 ("write commands are
  silent on success") — the plan's own alternative proposal (printing a
  confirmation line) was not implemented, since SPEC.md is the source of
  truth here and it says silent.
- `Command::Week`'s dispatch is still `todo!()` — invoking `mlm week` or `mlm
  week target ...` from the compiled binary will panic. This is unchanged
  from Milestone 8's landed state and is explicitly out of this milestone's
  scope, but the reviewer should not mistake this for something this
  milestone broke.
- DST gap/fold handling (plan §8.5) is fully implemented, but at the
  Milestone 4 layer (`storage::to_utc_and_date`), not by this milestone;
  `commands.rs` just propagates `StorageError::NonexistentLocalTime` through
  `anyhow` like any other hard error. No special-casing was needed or added
  here.
