# Panic fixes — design spec

**Baseline**: written against `main` at `dd91952` (`v0.4.0` + `orphan-stop-warning` + `week-notes` + `export-notes`, unreleased).
**Status**: locked 2026-10-25 after one review
(`docs/dev/plans/reports/panic-fixes-spec-review.md`, folded in NOTES.md
entry 140; lock in entry 141).
**Input**: `export-notes` final review F1 and the unwrap audit
(`docs/dev/plans/reports/unwrap-audit-review.md`, F1-F4). Decisions:
NOTES.md entries 138-141.

## 1. Summary

Three ways a real user can crash `mlm` today:

- Any command run in the repeated hour when clocks go back in autumn
  panics (exit `101`).
- A huge week target (`mlm week target 99999999999h`) makes `status`
  panic while a stint is open, and makes week totals overflow.
- Any command whose stdout closes early (`mlm week | true`) or fails
  (disk full under `> file`) panics.

After this changeset none of these panic, for any target set after
it.

## 1.1 Non-goals

- No change to the three panics the audit found unreachable (a corrupt
  date row, the seed tool's flag parsing and huge `--weeks`).
- No change to targets already stored. The cap applies when a target
  is set, so an oversized target stored before this changeset can
  still panic `status` and overflow totals.
- No way for the user to raise the cap. A future user config may allow
  it, with the understanding that a very large value can break totals.
- No change to how a time in the repeated hour is stored (§3).
- No change to `--help` and `--version` output. Clap prints them.
- No change to `parse_date`'s accepted years or to week ranges near
  year 9999.

## 1.2 Known issues

None.

## 2. Prerequisites

None.

## 3. Current time in the repeated hour

- Taking the current time never fails. `now` is the current instant
  truncated to the whole minute (SPEC §4.1), in the repeated autumn
  hour as in any other.
- Every command works in that hour.
- Storage is unchanged. A `start` or `stop` without `TIME` stores
  today's local clock time, as it does today, and a local time in the repeated
  hour resolves to the earlier pass (SPEC §2.1), typed or not. So a
  punch made in the second pass stores the first-pass instant, as a
  typed one does today.

## 4. Week target cap

- `mlm week target` rejects a `DURATION` above `168h` (7 x 24h).
  `168h` itself is accepted.
- Error, exit `1`, nothing written:

  `error: target cannot exceed 168h 00m (got 200h 00m)`

  The `got` value is the parsed duration in the standard format (SPEC
  §4.2).
- The check runs after the existing `DURATION` checks, so a malformed
  or negative value keeps its current message. A value too large to
  parse at all keeps the format message. The `week WEEK_ID target`
  reminder (SPEC §3.7, exit `2`) still comes first.

## 5. Output write failures

- **Stdout closed by its reader** (broken pipe): the command stops
  writing and exits `141` with nothing on stderr. `141` is what a shell
  reports for a process killed by `SIGPIPE`, so `set -o pipefail`
  scripts see the same code as from other tools.
- **Any other stdout write failure**: one line on stderr, exit `1`.

  `error: cannot write output: <reason>`

  `<reason>` is the I/O error's standard text, `(os error N)` suffix
  included, e.g. `No space left on device (os error 28)`.
- Lines written before the failure stay written.
- A failed write to stderr is ignored, at every stderr write site
  (warnings, `error:` lines, the `week WEEK_ID target` reminder). The
  exit code is the one the command would have returned.
- A write command that already committed its change keeps it, and
  exits per the two rules above. Only the output is lost. This is a
  runtime error, not a §6.1 "reject, no write" error.

## 6. Testing

- Unix only (`TZ` is ignored on Windows): with a `TZ` rule that puts
  the real clock inside a repeated hour, `status` exits `0` and
  untyped `start` stores the first-pass instant
- `week target 168h` accepted; `168h1m` and `200h` rejected with the §4
  message, exit `1`, no target written; `-5h` and `10x` keep their
  messages
- `status` with an open stint and a `168h` target exits `0`
- Unix only: stdout a pipe whose read end is closed before the process
  starts (`mlm week`, `mlm export notes`, `mlm status`) → exit `141`,
  empty stderr
- Linux only: stdout `/dev/full` → the §5 message, exit `1`
- `week target 40h` with stdout closed keeps its target, exit `141`

## 7. User-visible surface

- `README.md`: the `week target` section names the `168h` cap. One
  short paragraph on piping says a closed pipe exits `141` quietly and
  any other write failure prints an `error:` line.
- `mlm week target --help`, `DURATION` line: `Target to set: 20h,
  33h30m or 45m, up to 168h (human format, never raw minutes)`

## Interaction with the project spec

- §2.1 and §4.1 state how `now` is taken in the repeated hour.
- §1.2a gains stored oversized targets as a possible issue.
- §3.7 gains the cap.
- §6.1 gains the cap error.
- §6.3 gains the write-failure error and exit `141`.
- §8.2 gains error flows for the cap and write failures.
