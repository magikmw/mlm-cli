# Plan: orphan-stop-warning

> **Archived — historical planning record.** Not authoritative. Current spec: `docs/dev/SPEC.md`. Design/process history: `docs/dev/NOTES.md`.

Spec: `docs/dev/specs/2026-10-24-orphan-stop-warning.md` (locked, rescoped). Reviews: `docs/dev/plans/reports/orphan-stop-warning-rescope-review.md`, `docs/dev/plans/reports/orphan-stop-warning-rescope-plan-review.md` (folded, NOTES.md entry 119). Decision log: NOTES.md entries 111, 117, 118, and the "Triaging review findings" process section.

This file replaces the plan for the superseded larger scope (entries 112-116). Its task plans were deleted in `44f8537`.

**Altitude**: scope of work only. Task plans choose identifiers. The one new file name below is pinned for ownership.

## Goal

After a successful `stop` write to date D, `stop` classifies D the way D's `status` page does, splice included, and prints one stderr warning block per orphaned end on D, ascending by instant (spec §3, §4). Each block is a header that names the date, followed by `status`'s two E8 hint lines word for word. Stdout stays empty and the exit code stays `0`. Any read failure after the write suppresses the whole warning (spec §5). Nothing else changes. Pairing, the splice, totals, `status`, `mlm week`, `start` and `note` all stay as they are (spec §1.1).

## Architecture

The `stop` handler in `src/commands.rs` currently writes the punch through the shared start/stop helper and returns. That helper already hands back the target date. `start` follows its write with a best-effort warning step whose writer is injected, so tests can capture its output. `stop` gains the same shape. It runs one post-write step that reads D's punches together with its two calendar neighbours, using the existing ranged punch read in `src/storage.rs` and the existing three-date bucket view in `src/history.rs`. It then builds D's anomaly facts through the existing per-date fact builder in `src/anomalies.rs`, which already applies the splice and lists orphaned-end times ascending. Finally it writes one block per orphaned end.

The two hint lines must not drift from `status`'s. Today they are built inside the E8 renderer in `src/anomalies.rs`, which is private to that module. The code task makes them reusable, so `status`'s E8 block and `stop`'s warning come from one source. `status`'s output must stay byte-identical.

`stop` does not run `start`'s 14-day lookback and adds no new storage query. It adds no module. AGENTS.md's `src/anomalies.rs` bullet, which lists its callers, gains `stop` (Task 2).

## Global constraints

- Verification set: `cargo fmt`, `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- The pre-commit hook also enforces cognitive complexity of 15 or less per function (`clippy.toml`) and a coverage ratchet against `coverage-baseline.json`. The read-failure path needs a test, or coverage drops.
- No new runtime dependency and no schema change.
- `stop`'s write path, validation order and hard errors (SPEC §6.1) stay unchanged. The warning step runs only after the write commits.
- A failed read never falls back to an empty date (spec §5).
- ASCII-only output (`tests/ascii_sweep.rs`).

## Ordering and parallelization

The two tasks run in parallel in separate worktrees. They share no file, and the contract below is all they share. There is nothing to sequence.

A finer split buys nothing. The code change is one handler step plus one small extraction, and splitting tests from code would put two tasks on the same behavior with no file to divide.

## Interface contracts (pinned before worktrees open)

1. **Warning block text.** Each orphaned end at local time T on date D prints three lines to stderr:
   - `warning: orphaned end on <Wdy YYYY-MM-DD> at <HH:MM> (no matching start)`
   - `    fix: mlm start HH:MM --date <YYYY-MM-DD>  (a time before <HH:MM>)`
   - `     or: mlm delete punch --date <YYYY-MM-DD>  (lists punches; rerun with the end's number)`

   `fix:` has four leading spaces and `or:` has five. Two spaces come before each parenthetical. The weekday-date form is the same one `start`'s warnings use. The two hint lines equal `status`'s E8 lines (SPEC §7.3).
2. **Several orphans.** One block per orphaned end, ascending by instant, with no blank line between blocks. This matches `status`'s E8 rendering and `start`'s warnings.
3. **Silence.** No orphaned end on D after the write means nothing on stderr. A read failure after the write also means nothing on stderr. Either way stdout is empty and the exit code is `0`. A failed write to stderr is ignored, as `start` ignores it, and never changes the exit code.
4. **Doc example.** The FE5 example is spec §4's block, verbatim: today Mon 2026-10-05, `mlm start 22:00 --date -2`, then a bare `mlm stop` that lands at 14:16.
5. **Flow id.** The new §8.2 flow is E27.

## Task 1: code and tests

**Spec**: §3 trigger, §4 output, §5 errors, §6 testing; non-goals §1.1.

`stop` gains its post-write warning step, and the E8 hint lines become shared between `status` and `stop`. The step reads D-1 through D+1 in one ranged read and classifies D with the splice. It prints contract 1's block for each orphaned end and swallows any read failure without printing anything. The writer is injected the same way as `start`'s, so unit tests capture output. The CLI tests go in a new test file, driven through the binary with `TZ=UTC`, seeding raw rows the way `tests/start_hint_cli.rs` does.

Acceptance criteria. Each CLI case asserts exact stderr, empty stdout and exit `0`. Every case passes an explicit TIME, because a bare `stop` makes the minute in the text depend on the clock.
- FE5: a start two days back, then a stop today. Stderr holds one block naming today.
- Clean pair (`start 09:00`, `stop 17:00`): stderr is empty.
- Splice: yesterday has one open start, and today's stop is today's first punch. Stderr is empty.
- Displaced end (`start 09:00`, `stop 17:00`, then `stop 12:00`): one block for 17:00.
- Pre-existing orphan: a seeded orphaned end on D at 08:00, then `start 09:00` and `stop 17:00` on D, so the new start cannot pair with the old orphan. The final `stop` prints the old orphan's block.
- Two orphans on one date, inserted out of time order: two blocks, ascending.
- `--date` to a past date: the block names that date, weekday included.
- Read failure: a malformed row on D-1 (for example an unparseable `at_utc`, seeded raw). The `stop` to D prints nothing and exits `0`, and the punch is stored.
- `status`'s E8 output stays byte-identical. The existing anomaly and status tests pass unchanged.
- `stop`'s stderr is covered by the existing ASCII sweep, or by an ASCII assertion in the new test file.

**Out of scope**: refusing or retargeting a write. Warnings for overlaps, E7, stale starts or refused joins. Checking D+1 or any date other than D. Changes to `start`, `note`, `status` output, `mlm week`, pairing or the splice. `--help` text. Any doc file.

**Owns exclusively**: `src/commands.rs`, `src/anomalies.rs`, and the new `tests/orphan_stop_cli.rs`. It reads `src/storage.rs`, `src/history.rs` and `src/stint.rs` without editing them. Any change to another file needs a coordinator decision first.

## Task 2: docs

**Spec**: §3 trigger, §4 output, §5 errors (for SPEC §7.4's text), §7 user-visible surface, "Interaction with the project spec", §1.2 known issue.

This task folds the behavior into the live docs so SPEC.md and README.md stand alone, with no pointer out to the working spec.

Acceptance criteria:
- SPEC §1.2a gains the stop-before-start known issue (spec §1.2).
- SPEC §3's sentence that a run creating and migrating a new database prints nothing, and flow F13, both note that a first `mlm stop` warns about its orphan.
- SPEC §3.3 points to §7.4's `stop` warning.
- SPEC §3.7's "`start`, `stop` and `note` stay silent" names the `stop` exception.
- SPEC §4.3's "not rejected at insert time" lead-in, §6's "surfaced later" lead-in and §6.2's "not at insert time" sentence each name the `stop` warning.
- SPEC §7.4 states the trigger, gives the block (contract 1, with the FE5 example) and the best-effort rule, and adds the `stop` exception to the "write commands print nothing" rule.
- SPEC §8.2 gains E27 next to E19, and E8 names it.
- README.md: the "silent on success" line names both the `start` and `stop` warnings. The "That holds on the first run too" sentence notes that a first `mlm stop` warns. The `mlm stop` section gains the warning and the FE5 example. Possible issues mirrors the new §1.2a item.
- AGENTS.md's `src/anomalies.rs` bullet names `stop` among its callers.
- Every quoted warning matches contract 1 byte for byte.

**Out of scope**: `--help` text (spec §7). NOTES.md. Archiving the working spec. Any source or test file.

**Owns exclusively**: `docs/dev/SPEC.md`, `README.md`, `AGENTS.md`. NOTES.md and the spec archive banner stay with the coordinator.

## Risks and ambiguities

- **Blank lines between blocks.** The spec does not say whether blocks are separated. Contract 2 pins no separator, to match `status`'s E8 and `start`'s warnings.
- **The D+1 read.** Orphaned ends on D do not depend on D+1. Reading D-1 through D+1 matches "exactly as D's `status` page does" (spec §3) at the cost of one extra date in the same ranged read. The flip side is that a malformed row on D+1 also suppresses the warning. That is consistent with spec §5's "any read".
- **The existing test `other_writers_stay_silent`** in `tests/week_target_cli.rs` runs `start 9:00` and then a bare `stop`. Before 09:00 UTC that `stop` now warns on stderr. The test checks only stdout, so it still passes, and Task 1 leaves it alone.
