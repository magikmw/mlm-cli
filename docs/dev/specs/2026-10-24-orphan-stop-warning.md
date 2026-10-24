# Orphaned-stop warning — design spec

**Baseline**: written against `main` at `4c451d7` (`v0.4.0`).
**Status**: draft.
**Input**: fresh-eyes finding FE5 from `overnight-visibility` (NOTES.md
entry 108). Decisions: NOTES.md entry 111.

## 1. Summary

A `stop` whose end finds no start to pair with is stored as an orphaned
end (E8, SPEC §4.3). `stop` prints nothing and exits `0`. The user
learns about it only on the next `status`.

The common trigger is FE5. A start from two or more days ago is still
open, and a bare `mlm stop` lands on today. The splice (SPEC §4.3.1)
joins adjacent dates only, so today's end stays orphaned and the old
start stays open.

After this changeset, `stop` warns on stderr whenever the date it wrote
to holds an orphaned end. The warning carries the same fixes `status`
prints for that end. Any open start the 14-day lookback finds follows,
in the form `start` already uses (SPEC §7.4). The write, stdout and
the exit code are unchanged.

| Case | Before | After |
|---|---|---|
| `stop` pairs with a start, or splices with yesterday's | silent | silent |
| `stop` orphans its end, no open start in the lookback | silent | orphan warning |
| FE5: `stop` orphans its end, a start 2+ days old is open | silent | orphan warning, then the stale-start warning |

## 1.1 Non-goals

- `stop` never refuses a write and never changes the date it writes to.
  Inserts stay unvalidated (SPEC §1.2, §4.3).
- No change to pairing, the splice, totals, `status`, or `mlm week`.
- A `stop` that writes no orphan prints nothing, even when the lookback
  would find stale starts. Only `start` warns unconditionally.
- No change to `start`'s warnings or to `note`.

## 1.2 Known issues

Filled at phase 4.

## 2. Prerequisites

None.

## 3. Trigger

After a successful `stop` write, `stop` classifies the written date
with the splice applied, as `status` does for that date (SPEC §4.3.1).
It warns once for every orphaned end on that date.

- This includes an end the new punch displaced. For example, given
  `start 09:00` and `stop 17:00`, a new `stop 12:00` pairs with `09:00`
  and leaves `17:00` orphaned.
- This includes an orphaned end that was already there before the
  write. It warns on every `stop` to that date until the orphan is
  fixed, as `start` repeats its yesterday warning (SPEC §7.4).
- An end that splices with the previous date's open start is not
  orphaned and draws no warning.
- `--date` changes the written date, not the rule.

## 4. Output

All warnings go to stderr. Stdout stays empty and the exit code stays
`0`.

Each orphaned end, ascending by instant, prints three lines. The
`fix:`/`or:` lines match `status`'s E8 block (SPEC §7.3) word for word:

```
warning: orphaned end on Mon 2026-10-05 at 14:16 (no matching start)
    fix: mlm start HH:MM --date 2026-10-05  (a time before 14:16)
     or: mlm delete punch --date 2026-10-05  (lists punches; rerun with the end's number)
```

After the orphan blocks, `stop` prints every warning the 14-day
lookback yields, oldest first, in exactly the forms `start` prints
(SPEC §7.4). The lookback runs from today, not from the written date.
`stop` suppresses nothing, because its own punch is an end and never
appears as a lookback start.

FE5, with today Mon 2026-10-05:

```
$ mlm start 22:00 --date -2
$ mlm stop
warning: orphaned end on Mon 2026-10-05 at 14:16 (no matching start)
    fix: mlm start HH:MM --date 2026-10-05  (a time before 14:16)
     or: mlm delete punch --date 2026-10-05  (lists punches; rerun with the end's number)
warning: unclosed start on Sat 2026-10-03 at 22:00 (2 days ago)
    fix: mlm stop HH:MM --date 2026-10-03  (a time after 22:00)
     or: mlm delete punch --date 2026-10-03  (lists punches; rerun with the start's number)
```

The user deletes today's stray end and stops Saturday's start with
`--date`.

## 5. Errors

The check is best-effort, as `start`'s is (SPEC §7.4). If any read
after the committed write fails, `stop` prints nothing extra and still
exits `0`. A hard error before the write (SPEC §6.1) is unchanged.

## 6. User-visible surface

- `README.md`, under `mlm stop`, describes the warning with the FE5
  example.
- `mlm stop --help` is unchanged. `start --help` does not mention its
  warnings either.

## Interaction with the project spec

- §7.4: `stop` gains the orphan warning above and the conditional
  lookback warnings. The "write commands print nothing" rule gains this
  exception.
- §3.3: gains a pointer to §7.4's `stop` warning.
- §8: the E8 flow (§8.2) gains the `stop` warning, and a new happy-path
  flow covers FE5.
