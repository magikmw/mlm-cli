# Plan: panic-fixes

Spec: `docs/dev/specs/2026-10-25-panic-fixes.md` (locked). Review: `docs/dev/plans/reports/panic-fixes-spec-review.md` (folded, NOTES.md entry 140). Audit: `docs/dev/plans/reports/unwrap-audit-review.md` F1-F4. Decision log: NOTES.md entries 138-141.

**Altitude**: scope of work only. Task plans choose identifiers and mechanisms.

**Read for this plan**: the spec in full; NOTES.md entries 137-141; both review reports above; `docs/dev/plans/export-notes-plan.md` (format reference); `src/main.rs` (the `now` capture, dispatch, the error-line exit path); `src/cli.rs` (the `week target` help block, the swapped-target reminder, the parse-failure report, the duration-shape check behind the reminder); `src/time.rs` (DURATION parsing, its errors, the formatter); `src/week_target.rs` (parse-then-write order, the echo line); `src/commands.rs` (the untyped-`TIME` punch path, the `delete` list and recreate lines, the stderr warnings); the stdout sites in `src/status.rs`, `src/week_view.rs`, `src/export.rs`; `tests/week_target_cli.rs` (the `week target --help` golden); `Cargo.toml` (`rust-version = "1.85"`, `chrono-tz` dev-dependency); `.github/workflows/ci.yml` (Linux, macOS, Windows runners); `clippy.toml`; README.md's `week target` section; SPEC.md §2.1, §3.7, §4.1, §6, §6.1-§6.3; env_logger 0.11.11's logger (it already ignores stderr write errors).

## Goal

Three user-reachable panics stop panicking (spec §1). Any command run in the repeated autumn hour works, and an untyped `start`/`stop` there stores the first-pass instant, the same as a typed time does today (spec §3). `mlm week target` rejects a `DURATION` above `168h` with a fixed message and writes nothing, so newly set targets can no longer overflow `status`'s `est. EOD` or the week totals (spec §4). A stdout write failure no longer panics. A broken pipe exits `141` with nothing on stderr, any other failure prints one `error: cannot write output: <reason>` line and exits `1`, and a failed stderr write is ignored everywhere (spec §5). Stored targets, the storage of repeated-hour times, and clap's help and version output do not change (spec §1.1).

## Architecture

The `now` capture in `src/main.rs` truncates the current instant to the whole minute without rebuilding a local wall time, so it cannot hit an ambiguous local time. The audit's F1 shows the shape. Everything downstream is unchanged. The untyped punch path in `src/commands.rs` still passes today's wall time to the storage re-resolve, which already picks the earlier pass (spec §3, NOTES 140-141). So `src/storage.rs` does not change.

The cap is a check in the `week target` path. It runs after the existing DURATION parse succeeds, so malformed, negative and `i64`-overflowing values keep their current messages (spec §4). It runs before the upsert, which keeps the "both parses complete before the single write" rule in `src/week_target.rs`. The cap error is a new error distinct from the DURATION parse errors. The swapped-target reminder in `src/cli.rs` treats any well-shaped duration as a duration, and the cap must not change that.

Every stdout write moves off the panicking print macros onto a fallible write whose error reaches `src/main.rs`. Ten sites change, in `src/status.rs`, `src/week_view.rs`, `src/export.rs`, `src/week_target.rs` and `src/commands.rs` (the `delete` list lines, the "nothing to delete" lines and the recreate lines). The write error has to survive as a typed I/O error, so `src/main.rs` can tell a broken pipe from other failures when it maps the result to an exit code. Stdout is flushed explicitly before exit, because `std::process::exit` flushes the buffer itself and drops any error. The two remaining panicking stderr writes (`src/main.rs`'s `error:` line and `src/cli.rs`'s swapped-target reminder) ignore failure, as the start/stop warnings already do.

No schema change, no new dependency, no new top-level module, no MSRV change.

## Global constraints

- Verification set: `cargo fmt`, `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- The pre-commit hook enforces cognitive complexity of 15 or less per function (`clippy.toml`) and a coverage ratchet against `coverage-baseline.json`. Every new branch needs a test: the cap, the broken-pipe exit, the other-failure exit, and each stderr site's ignore path.
- Targets: Linux and Windows MSVC. CI also runs macOS. Tests that rely on `TZ`, on pipes or on `/dev/full` carry the platform gate spec §6 names (Unix, Unix, Linux).
- MSRV stays `1.85` (`Cargo.toml`). Test helpers must build on it, so `std::io::pipe` (stable in 1.87) is out.
- No new runtime dependency, no schema change, no new top-level module. A new module needs a coordinator decision first, because AGENTS.md's layout list would change.
- Existing output, messages and help stay byte-identical, apart from contract 3's help line. Existing tests keep their meaning.
- All new output is ASCII.

## Ordering and parallelization

Two tasks, run in parallel in separate worktrees. They share no file. The contracts below are all they share.

A finer code split buys nothing. The three fixes share `src/main.rs` (the `now` capture and the exit-code mapping), `src/week_target.rs` (the cap and a stdout site) and `src/cli.rs` (the help line and a stderr site). Splitting them would put two tasks on the same files.

## Interface contracts (pinned before worktrees open)

1. **Cap value.** `168h` (10080 minutes) is the largest accepted `DURATION` for `week target`. `168h`, `167h60m` and `10080m` are accepted. `168h1m` and `10081m` are rejected.
2. **Cap error** (spec §4), stderr, exit `1`, stdout empty, nothing written:

   `error: target cannot exceed 168h 00m (got 200h 00m)`

   The `got` value is the parsed minutes in the standard format (SPEC §4.2), so `168h1m` gives `(got 168h 01m)` and `99999999999h` gives `(got 99999999999h 00m)`. The order of checks is WEEK_ID, then the DURATION parse, then the cap. A malformed value (`10x`), a negative one (`-5h`) and one that overflows `i64` (`9999999999999999999h`) keep today's messages.
3. **`mlm week target --help` DURATION line** (spec §7), verbatim, two-space indent and two-space gap as today:

   `  <DURATION>  Target to set: 20h, 33h30m or 45m, up to 168h (human format, never raw minutes)`

   Every other line of that help is unchanged.
4. **Broken pipe** (spec §5). A stdout write that fails as a broken pipe stops the command. It prints nothing to stderr and exits `141`, on every platform.
5. **Other write failure** (spec §5). One stderr line, exit `1`:

   `error: cannot write output: <reason>`

   `<reason>` is the I/O error's standard display text. On Linux, stdout `/dev/full` gives `error: cannot write output: No space left on device (os error 28)`.
6. **Committed writes stay.** `week target` and `delete note|punch N` keep their change when the output fails, and exit per contracts 4 and 5. Output written before the failure stays written.
7. **Stderr failure.** A failed stderr write is ignored at every site, and the exit code is the one the command would have returned. The sites are the warnings, the `error:` line and the swapped-target reminder.
8. **Unchanged.** Clap's help, version and usage-error output keep their current exit codes and handling. The swapped-target reminder still fires for an over-cap duration (`mlm week 2026-41 target 200h` prints the reminder, exit `2`). Typed repeated-hour times and every stored value resolve as today.
9. **SPEC placement** (spec "Interaction"). The cap goes in §3.7 and §6.1. The write failure and exit `141` go in §6.3 as a runtime error, never in §6.1, whose heading promises no write. The repeated-hour `now` goes in §2.1 and §4.1. §8.2 gains the error flows.

## Task 1: code and tests

**Spec**: §3 repeated hour, §4 cap, §5 write failures, §6 testing, §7 help line; non-goals §1.1.

`src/main.rs` takes `now` without a local rebuild, ignores a failed stderr write on its `error:` line, flushes stdout before exit, and maps a write error to contracts 4 and 5. `src/week_target.rs` adds the cap check (contracts 1, 2) before the upsert. `src/cli.rs` carries contract 3's help line and ignores a failed stderr write on the swapped-target reminder. The ten stdout sites in `src/status.rs`, `src/week_view.rs`, `src/export.rs`, `src/week_target.rs` and `src/commands.rs` write through a fallible path whose error reaches `src/main.rs` as an I/O error. `tests/week_target_cli.rs`'s help golden takes the new line. The minute truncation gets a unit test over a fixed zone (the `chrono-tz` dev-dependency, `Europe/Warsaw` at a second-pass instant on 2026-10-25), so it runs on every platform. Process-level tests go in a new `tests/panic_fixes_cli.rs`, driven through the binary with `MLM_DB_PATH`. The repeated-hour test builds a POSIX `TZ` rule from the current time so that now sits in the second pass, and it reads the stored instant back from the database. The pipe tests close the read end before the binary starts, with a helper that builds on Rust 1.85.

Acceptance criteria. CLI cases assert exact stdout, stderr and exit code.
- The truncation unit test passes at a second-pass instant and keeps the instant, not the first-pass rebuild.
- Unix only: with now in the second pass of a computed `TZ` rule, `mlm status` exits `0`, and an untyped `mlm start` exits `0` and stores the first-pass instant of the current wall minute, an hour before the real now. The test tolerates a minute boundary between its clock reads and the binary's.
- `week target 168h` and `week target 10080m` succeed with today's echo line.
- `week target 168h1m`, `200h` and `99999999999h` print contract 2's message with the right `got` value, exit `1`, empty stdout, and leave no `week_targets` row. An existing row for that week keeps its old value.
- `week target -5h`, `10x` and `9999999999999999999h` keep their current messages and exit codes.
- `mlm week 2026-41 target 200h` prints the swapped-target reminder and exits `2`.
- `status` with an open stint today and a `168h` target exits `0` and shows an `est. EOD` line.
- `mlm week target --help` and `-h` print the golden with contract 3's line.
- Unix only: stdout a pipe whose read end is closed before spawn gives exit `141` and empty stderr for `mlm week`, `mlm export notes` and `mlm status`.
- Unix only: under the same pipe, `mlm week target 40h` exits `141` and the target is stored. `mlm delete punch 1` exits `141` and the punch is gone.
- Linux only: stdout `/dev/full` under `mlm week` prints contract 5's line on stderr and exits `1`.
- Unix only: stderr a closed-read-end pipe on an exit-`1` error (for example a malformed TIME) exits `1`, and on the swapped-target shape exits `2`.
- Existing tests pass unchanged, apart from the help golden.

**Out of scope**: stored oversized targets and any read-side clamp. A configurable cap. Storage of repeated-hour times, including the second-pass instant for untyped punches. `--help`/`--version` output. The audit's F5-F7, `parse_date`'s year range, week ranges near year 9999. The seed tool. Any `.md` file.

**Owns exclusively**: `src/main.rs`, `src/cli.rs`, `src/week_target.rs`, `src/commands.rs`, `src/status.rs`, `src/week_view.rs`, `src/export.rs`, `tests/week_target_cli.rs`, the new `tests/panic_fixes_cli.rs`, and `coverage-baseline.json`, which the pre-commit hook ratchets and stages. It reads `src/time.rs` and `src/storage.rs` without editing them. If the cap or the write path turns out to need an edit to `src/time.rs`, `src/render.rs` or `src/lib.rs`, that file also belongs to this task, because Task 2 owns no source file. Any other file needs a coordinator decision first.

## Task 2: docs

**Spec**: §3, §4, §5, §7 user-visible surface, "Interaction with the project spec"; non-goals §1.1.

This task folds the behavior into the live docs so SPEC.md and README.md stand alone, with no pointer out to the working spec.

Acceptance criteria:
- SPEC §2.1 and §4.1 say that taking the current time never fails, that `now` is the current instant truncated to the minute in the repeated hour too, and that an untyped `start`/`stop` there stores the first-pass instant, as a typed time does.
- SPEC §3.7 gains the `168h` cap (contract 1) and the order of checks (contract 2). If it quotes the help, it quotes contract 3's line.
- SPEC §6.1 gains the cap error (contract 2) under the DURATION bullet.
- SPEC §6.3 gains the write-failure runtime error (contracts 4-7): exit `141` with empty stderr for a broken pipe, contract 5's line and exit `1` for any other failure, committed writes kept, stderr failures ignored. §6.1's heading and the §6 intro stay true.
- SPEC §8.2 gains error flows for an over-cap target and for both write failures.
- README.md's `week target` section names the `168h` cap and shows contract 2's message. One short paragraph on piping says a closed pipe exits `141` quietly and any other write failure prints an `error:` line.
- AGENTS.md is checked and left alone unless a statement in it goes stale. With no new module, none is expected.
- Every quoted help line, message and exit code matches the contracts byte for byte.

**Out of scope**: `--help` text and module docs (Task 1). NOTES.md. Archiving the working spec. Any source or test file.

**Owns exclusively**: `docs/dev/SPEC.md`, `README.md`, `AGENTS.md`. NOTES.md and the spec archive banner stay with the coordinator.

## Risks and ambiguities

- **`<reason>` text.** The spec says "the system's message". Std's I/O error display adds ` (os error N)` to the OS text. Contract 5 pins std's display, since that is what every other `error:` line in `mlm` shows for an I/O error. Stripping the suffix would be a new formatting rule.
- **Swapped-target reminder and the cap.** The spec does not say whether `week 2026-41 target 200h` gets the reminder or the cap error. The reminder checks shape only, and a negative value already counts as a duration there. Contract 8 keeps that, so the parse layer's behavior does not change.
- **Stored oversized targets are undocumented.** Spec §1.1 keeps them able to panic `status` and overflow totals, but "Interaction" names no SPEC section for it. SPEC §1.2a ("Possible issues to revisit") is the natural home. The plan leaves it out of Task 2, so the coordinator decides whether to add it.
- **Stdout flush at exit.** `std::process::exit` flushes stdout and ignores the error. A write that sits in the buffer would fail silently and exit `0`. Task 1 must flush and check before exiting.
- **Deterministic closed pipe on 1.85.** Dropping the read end after spawn races the child's first write, and `std::io::pipe` needs 1.87. One 1.85-compatible route borrows a pipe from a short-lived helper process, whose stdin is the read end, and waits for it to exit before spawning `mlm`. Task 1's plan picks the mechanism.
- **Repeated-hour test near midnight.** A `TZ` rule built from the current time has to keep the shifted wall time on the same local date, or `start` writes to another date. Task 1's plan handles the boundary, for example by skipping the test in the minutes around midnight.
- **Windows exit `141`.** It carries no SIGPIPE meaning there (review item 14). The spec chose one code on every platform. No Windows test covers the broken pipe.
