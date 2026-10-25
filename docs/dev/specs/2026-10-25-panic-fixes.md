# Panic fixes — design spec

**Baseline**: written against `main` at `dd91952` (`v0.4.0` + `orphan-stop-warning` + `week-notes` + `export-notes`, unreleased).
**Status**: draft.
**Input**: `export-notes` final review F1 and the unwrap audit
(`docs/dev/plans/reports/unwrap-audit-review.md`, F1-F4). Decisions:
NOTES.md entries 138 and 139.

## 1. Summary

Three ways a real user can crash `mlm` today:

- Any command run in the repeated hour when clocks go back in autumn
  panics (exit `101`).
- A huge week target (`mlm week target 99999999999h`) makes `status`
  panic while a stint is open, and makes week totals overflow.
- Any command whose stdout closes early (`mlm week | true`) or fails
  (disk full under `> file`) panics.

After this changeset none of these panic.

## 1.1 Non-goals

- No change to the three panics the audit found unreachable (a corrupt
  date row, the seed tool's flag parsing and huge `--weeks`).
- No change to targets already stored. The cap applies when a target
  is set.
- No user-set target cap. A future user config may raise it, with the
  understanding that a very large value can break totals.
- No change to `parse_date`'s accepted years or to week ranges near
  year 9999.

## 1.2 Known issues

None.

## 2. Prerequisites

None.

## 3. Current time in the repeated hour

- `now` is the current instant, truncated to the whole minute (SPEC
  §4.1). In the repeated autumn hour it keeps the offset that applies
  at that instant, so the earlier pass and the later pass are two
  different instants.
- Every command works in that hour as in any other.
- Typed `TIME` input is unchanged. A typed time in the repeated hour
  still resolves to the earlier instant (SPEC §2.1).

## 4. Week target cap

- `mlm week target` rejects a `DURATION` above `168h` (every hour in a
  week). `168h` itself is accepted.
- Error, exit `1`, nothing written:

  `error: target cannot exceed 168h 00m (got 200h 00m)`

  The `got` value is the parsed duration in the standard format (SPEC
  §4.2).
- The check runs after the existing `DURATION` checks, so a malformed
  or negative value keeps its current message.

## 5. Output write failures

- **Stdout closed by its reader** (broken pipe): the command stops
  writing and exits `141` with nothing on stderr. `141` is what a shell
  reports for a process killed by `SIGPIPE`, so `set -o pipefail`
  scripts see the same code as from other tools.
- **Any other stdout write failure**: one line on stderr, exit `1`.

  `error: cannot write output: <reason>`

  `<reason>` is the system's message for the failure.
- Lines written before the failure stay written.
- A failed write to stderr is ignored. The exit code is the one the
  command would have returned.
- A write command (`start`, `stop`, `note`, `delete`, `week target`)
  that already committed its change keeps it. Only the output is lost.

## 6. Testing

- a command in the repeated autumn hour exits `0` and stores the
  correct UTC instant; the earlier and the later pass store different
  instants
- `week target 168h` accepted; `168h1m` and `200h` rejected with the §4
  message, exit `1`, no target written; `-5h` and `10x` keep their
  messages
- `status` with an open stint and a `168h` target exits `0`
- stdout closed before output (`mlm week`, `mlm export notes`,
  `mlm status`) → exit `141`, empty stderr
- stdout write failing with another error (`/dev/full` on Linux) →
  the §5 message, exit `1`
- a write command with stdout closed keeps its write

## 7. User-visible surface

- `README.md`: the `week target` section names the `168h` cap. One
  short paragraph on piping says a closed pipe exits `141` quietly and
  any other write failure prints an `error:` line.
- `mlm week target --help`, `DURATION` line: `Target to set: 20h,
  33h30m or 45m, up to 168h (human format, never raw minutes)`

## Interaction with the project spec

- §2.1 and §4.1 state how `now` is taken in the repeated hour.
- §3.7 gains the cap.
- §6.1 gains the cap error and the write-failure error.
- §6.3 gains exit `141`.
- §8.2 gains error flows for the cap and write failures.
