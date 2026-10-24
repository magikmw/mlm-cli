# Orphaned-stop warning — design spec

**Baseline**: written against `main` at `4c451d7` (`v0.4.0`).
**Status**: draft, rescoped 2026-10-24 (NOTES.md entry 117).
**Input**: fresh-eyes finding FE5 from `overnight-visibility` (NOTES.md
entry 108). Decisions: NOTES.md entries 111 and 117.

## 1. Summary

A `stop` whose end finds no start to pair with is stored as an orphaned
end (E8, SPEC §4.3). `stop` prints nothing and exits `0`. The user
learns about it only on the next `status`.

The common trigger is FE5. A start from two or more days ago is still
open, and a bare `mlm stop` lands on today. The splice (SPEC §4.3.1)
joins adjacent dates only, so today's end stays orphaned.

After this changeset, `stop` warns on stderr when the date it wrote to
holds an orphaned end. The warning carries the fixes `status` already
prints for it. `start` already warns about the open starts it leaves
behind (SPEC §7.4), so each write command now reports the unmatched
punch it leaves. The write, stdout and the exit code are unchanged.

## 1.1 Non-goals

- `stop` never refuses a write and never changes the date it writes to.
  Inserts stay unvalidated (SPEC §1.2, §4.3).
- No change to pairing, the splice, totals, `status`, `mlm week`,
  `start` or `note`.
- `stop` checks only the date it wrote to.

## 1.2 Known issues

- Backfilling stop-then-start warns once. `mlm stop 17:00 --date -1`
  followed by `mlm start 09:00 --date -1` is legal (SPEC §3.2), but the
  first command prints an orphan warning. The second command clears it.

## 2. Prerequisites

None.

## 3. Trigger

After a successful `stop` write to date `D`, `stop` classifies `D` with
the splice applied, exactly as `D`'s `status` page does (SPEC §4.3.1).
It warns once for every orphaned end on `D`.

- This includes an end the new punch displaced, and an orphan that was
  already on `D` before the write. Every `stop` to `D` reports the
  orphans `D` holds.
- An end that splices with the previous date's open start is not
  orphaned and draws no warning.
- `--date` changes `D`, not the rule.

## 4. Output

Warnings go to stderr, one block per orphaned end, ascending by
instant. Stdout stays empty and the exit code stays `0`.

The header names the date, which `status`'s E8 header leaves to its
page. The two hint lines are `status`'s E8 lines (SPEC §7.3) word for
word.

FE5, with today Mon 2026-10-05:

```
$ mlm start 22:00 --date -2
$ mlm stop
warning: orphaned end on Mon 2026-10-05 at 14:16 (no matching start)
    fix: mlm start HH:MM --date 2026-10-05  (a time before 14:16)
     or: mlm delete punch --date 2026-10-05  (lists punches; rerun with the end's number)
```

`status` shows the stale start that the end failed to close.

## 5. Errors

The check is best-effort, as `start`'s is (SPEC §7.4). If the read
after the committed write fails, `stop` prints nothing extra and still
exits `0`. A hard error before the write (SPEC §6.1) is unchanged.

## 6. Testing

Each case asserts exact stderr, empty stdout and exit `0`:

- FE5 → one orphan block
- clean pair → silent
- splice with yesterday's open start → silent
- displaced end (`start 09:00`, `stop 17:00`, then `stop 12:00`) →
  block for `17:00`
- pre-existing orphan, then a clean pair on the same date → block for
  the old orphan
- two orphans on one date → two blocks, ascending
- `--date` to a past date → block names that date
- read failure after the write → nothing extra, punch stored

## 7. User-visible surface

- `README.md`:
  - the line "Every `start`/`stop`/`note` call is silent on success"
    names the `start` and `stop` warnings
  - the first-run sentence ("That holds on the first run too") notes
    that a first `mlm stop` warns about its orphan
  - `mlm stop` gains the warning and the FE5 example
  - Possible issues mirrors the new SPEC §1.2a item
- `mlm stop --help` is unchanged. `start --help` does not mention its
  warnings either.

## Interaction with the project spec

- §1.2a gains the known issue in §1.2 here.
- §3's "a run that creates and migrates a new database prints nothing"
  and flow F13 note that a first `mlm stop` warns about its orphan.
- §3.3 points to §7.4's `stop` warning.
- §3.7's "`start`, `stop` and `note` stay silent" names the `stop`
  exception.
- §4.3's "not rejected at insert time" lead-in and §6.2's "not at insert
  time" sentence name the `stop` warning.
- §7.4 gains `stop`'s trigger, block and best-effort rule. The "write
  commands print nothing" rule gains the exception.
- §8.2 gains E27, `stop`'s warning, next to E19.
