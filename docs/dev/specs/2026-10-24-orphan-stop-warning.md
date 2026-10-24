# Orphaned-stop warning — design spec

**Baseline**: written against `main` at `4c451d7` (`v0.4.0`).
**Status**: draft, adversarial review folded
(`docs/dev/plans/reports/orphan-stop-warning-spec-review.md`, 13
findings — NOTES.md entry 112).
**Input**: fresh-eyes finding FE5 from `overnight-visibility` (NOTES.md
entry 108). Decisions: NOTES.md entries 111-112.

## 1. Summary

A `stop` whose end finds no start to pair with is stored as an orphaned
end (E8, SPEC §4.3). `stop` prints nothing and exits `0`. The user
learns about it only on the next `status`.

The common trigger is FE5. A start from two or more days ago is still
open, and a bare `mlm stop` lands on today. The splice (SPEC §4.3.1)
joins adjacent dates only, so today's end stays orphaned and the old
start stays open. One day younger, the same mistake goes the other way.
A start forgotten yesterday splices with today's bare `stop` into a
stint of a day or more, and nothing says so.

This changeset makes `stop` warn on stderr in both cases. It also gives
the E8 `fix:` hint a time window, so the start it suggests pairs with
the orphan and nothing else. The write, stdout and the exit code are
unchanged.

| Case | Before | After |
|---|---|---|
| After the write, no orphaned end on the written date or the day after, and no long splice | silent | silent |
| An orphaned end on the written date or the day after (§3.1) | silent | orphan warning, then lookback warnings |
| FE5: as above, with a start 2+ days old open | silent | orphan warning, then the stale-start warning |
| The new end splices into a stint of 12h or more (§3.2) | silent | long-splice warning, then lookback warnings |
| `status` E8 block, with an earlier punch on the date (§5) | `(a time before T)` | `(a time between P and T)` |

## 1.1 Non-goals

- `stop` never refuses a write and never changes the date it writes to.
  Inserts stay unvalidated (SPEC §1.2, §4.3).
- No change to pairing, the splice, totals, or `mlm week`.
- A `stop` that triggers no warning (§3) prints nothing, even when the
  lookback would find stale starts. Only `start` warns unconditionally.
- No warning for other anomalies a `stop` creates: an overlap (E21), or
  a wrong start left open on an E7 date.
- No change to `start`'s warnings or to `note`.
- No `--quiet` flag.

## 1.2 Known issues

- Backfilling stop-then-start warns once. `mlm stop 17:00 --date -1`
  followed by `mlm start 09:00 --date -1` is legal (SPEC §3.2), but the
  first command prints an orphan warning whose `or:` line suggests
  deleting a correct punch. The second command clears the orphan.
- An orphan already on a date repeats its warning on every `stop` to
  that date, or to the day before, until it is fixed.
- A backdated `stop` gets today's lookback, not the written date's. A
  stale start beside an old written date, outside the 14-day window,
  does not show.
- `stop` now writes to stderr. A script that treats any stderr from
  `mlm stop` as failure, or logs `2>&1`, sees new output.
- The 12h long-splice threshold is a fixed value. A genuine overnight
  session of 12h or more warns too.

## 2. Prerequisites

None.

## 3. Triggers

After a successful `stop` write to date `D`, `stop` checks two things.
Each warns independently, so one `stop` can print both.

### 3.1 Orphaned ends

`stop` classifies `D` and `D+1` with the splice applied, as `status`
does for each date (SPEC §4.3.1). It warns once for every orphaned end
on either date. `D+1` is skipped when `D` is today.

- `D` covers an end the new punch displaced. Given `start 09:00` and
  `stop 17:00`, a new `stop 12:00` pairs with `09:00` and leaves `17:00`
  orphaned.
- `D+1` covers a broken splice. Given yesterday `start 22:00` and today
  `stop 01:00`, spliced, a new `mlm stop 23:00 --date -1` closes
  yesterday's start, so today's `01:00` becomes an orphan.
- An orphaned end that was already there before the write warns too
  (§1.2).
- An end that splices with the previous date's open start is not
  orphaned.

### 3.2 Long splice

`stop` warns when its own end splices with `D-1`'s open start into a
stint of 12h or more. A splice shorter than 12h is an ordinary
overnight session and stays silent. Only the end this `stop` wrote
counts. An older spliced end never triggers it.

## 4. Output

All warnings go to stderr. Stdout stays empty and the exit code stays
`0`. The order is: orphan blocks for `D`, then for `D+1`, each ascending
by instant; the long-splice block; then the lookback lines.

### 4.1 Orphan block

Three lines per orphaned end. The `fix:`/`or:` lines match `status`'s E8
block (SPEC §7.3) word for word, including the window from §5:

```
warning: orphaned end on Mon 2026-10-05 at 14:16 (no matching start)
    fix: mlm start HH:MM --date 2026-10-05  (a time before 14:16)
     or: mlm delete punch --date 2026-10-05  (lists punches; rerun with the end's number)
```

### 4.2 Long-splice block

Three lines. `S` is the start's time on `D-1`. Nothing on `D-1` follows
`S`, because the splice needs `S` to be `D-1`'s only open start, so the
`then:` hint never needs a window:

```
warning: stop on Mon 2026-10-05 at 14:16 closed the start on Sun 2026-10-04 at 09:00 (29h 16m, counted on Sun 2026-10-04)
    fix: mlm delete punch --date 2026-10-05  (lists punches; rerun with the end's number)
   then: mlm stop HH:MM --date 2026-10-04  (a time after 09:00)
```

The duration uses the one formatter (SPEC §4.2).

### 4.3 Lookback lines

When any block above prints, `stop` then prints the 14-day lookback,
oldest first, in the stderr forms `start` prints (SPEC §7.4). The
lookback runs from today, not from `D`. It is filtered as today's
`status` page filters it (SPEC §7.1). The yesterday warning prints only
when today's `status` would print the yesterday hint. So a refused join
never yields a yesterday `fix:` that would pair the overnight start with
a same-day stop. `stop` suppresses nothing else, because its own punch
is an end and never appears as a lookback start. `stop` prints no
carry-in marker.

### 4.4 Examples

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

A start forgotten yesterday:

```
$ mlm start 09:00 --date -1
$ mlm stop
warning: stop on Mon 2026-10-05 at 14:16 closed the start on Sun 2026-10-04 at 09:00 (29h 16m, counted on Sun 2026-10-04)
    fix: mlm delete punch --date 2026-10-05  (lists punches; rerun with the end's number)
   then: mlm stop HH:MM --date 2026-10-04  (a time after 09:00)
```

## 5. E8 fix window

The E8 `fix:` hint gets a window, mirroring the stop-hint rule (SPEC
§7.3). The change applies everywhere the E8 block prints: `status`, and
`stop`'s orphan block.

An orphaned end at `T` needs a start that pairs with it and nothing
else. Let `P` be the latest punch on that date before `T`, in pairing
order (SPEC §4.3, step 1). A start between `P` and `T` pairs with `T`
only. A start before `P` pairs with `P` instead, or nests around later
stints and overlaps them.

- With no punch before `T`, the hint stays `(a time before T)`.
- With one, it reads `(a time between P and T)`.
- The window includes `P`'s minute only when `P` is an end that closed
  a stint on that date. A start at the same instant as such an end is
  pushed after the end pops (SPEC §4.3, step 2), so it pairs with `T`.
  When `P` is an orphaned end or the date's spliced first end, a start
  at `P`'s minute pairs with `P` at zero length (E14), so the window
  excludes it.
- Punches at the same minute as `T` do not count as before `T`.

The displaced example (`start 09:00`, `stop 12:00`, orphan `17:00`)
reads `(a time between 12:00 and 17:00)`.

## 6. Errors

The check is best-effort, as `start`'s is (SPEC §7.4). `stop` does all
its reads first, then prints. If any read after the committed write
fails, `stop` prints nothing extra and still exits `0`. A hard error
before the write (SPEC §6.1) is unchanged.

## 7. Testing

Unit and CLI tests cover each row:

- orphan on `D`, empty lookback → orphan block only
- FE5 → orphan block, then the stale-start lines
- clean pair, stale start in the window → silent
- splice under 12h → silent
- splice of exactly 12h, and of 29h 16m → long-splice block
- displaced end → orphan block for the end the user did not type
- pre-existing orphan, clean pair on the same date → orphan block
- two orphans on one date → two blocks, ascending
- backdated `stop` breaking `D+1`'s splice → orphan block for `D+1`
- `D` is today → `D+1` not read
- refused join with yesterday (second bare `stop` after a splice) →
  orphan blocks, no yesterday warning
- `--date` older than 14 days → orphan block, lookback from today
- read failure after the write → nothing extra, exit `0`
- stdout empty and exit `0` in every case above
- E8 window in `status`: no earlier punch, earlier closed stint (minute
  included), earlier orphan (minute excluded), spliced first end
  (minute excluded), same-minute punch at `T`

## 8. User-visible surface

- `README.md`:
  - the write-commands line ("Every `start`/`stop`/`note` call is
    silent on success") names the `start` and `stop` warnings
  - `mlm stop` gains the warning text and both §4.4 examples
  - any E8 example gains the window
- `mlm stop --help` is unchanged. `start --help` does not mention its
  warnings either.

## Interaction with the project spec

- §3.3 points to §7.4's `stop` warnings.
- §3.7's "`start`, `stop` and `note` stay silent" names the `stop`
  exception.
- §6.2's "shows up in that date's next `status`/`week` output, not at
  insert time" names the `stop` warning.
- §7.3's E8 block gains the window (§5 here). The stop-hint rule's
  consumer list adds `mlm stop`'s warnings.
- §7.4 gains `stop`'s triggers, blocks, order and best-effort rule. The
  "write commands print nothing" rule gains the exception.
- §8.2 gains E27, `stop`'s warnings, next to E19. E8 gains the window.
- §1.2a gains the known issues in §1.2 here.
- `AGENTS.md` and `src/lookback.rs` docs that describe the stderr forms
  as `start`'s alone name `stop` too.
