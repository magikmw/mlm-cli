# Orphaned-stop warning — design spec

**Baseline**: written against `main` at `4c451d7` (`v0.4.0`).
**Status**: locked 2026-10-24 after two adversarial review rounds
(`docs/dev/plans/reports/orphan-stop-warning-spec-review.md`, 13
findings, NOTES.md entry 112;
`docs/dev/plans/reports/orphan-stop-warning-spec-round-2-review.md`, 13
findings, NOTES.md entry 113).
**Input**: fresh-eyes finding FE5 from `overnight-visibility` (NOTES.md
entry 108). Decisions: NOTES.md entries 111-113.

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

This changeset makes `stop` warn on stderr in both cases, and when a
`stop` breaks an existing splice. It also fixes two hints that lead into
these states: the E8 `fix:` hint gains a time window, and the yesterday
hint stops offering a bare `mlm stop` once the start is 12h old. The
write, stdout and the exit code are unchanged.

| Case | Before | After |
|---|---|---|
| After the write, no orphan, refused join or long splice on `D` or `D+1` | silent | silent |
| An orphaned end or refused join on `D` or `D+1` (§3.1) | silent | date warnings, then lookback warnings |
| FE5: an orphan on today, a start 2+ days old open | silent | orphan warning, then the stale-start warning |
| A splice of 12h or more into `D` or `D+1` (§3.2) | silent | long-splice warning, then lookback warnings |
| `status` E8 block, with an earlier punch on the date (§5) | `(a time before T)` | `(a time between P and T)` |
| `status` E8 block, another end at the orphan's minute (§5) | `fix: mlm start ...` | delete line only |
| `status` yesterday hint, start 12h+ old (§6) | `or: mlm stop` line | no `or:` line |

## 1.1 Non-goals

- `stop` never refuses a write and never changes the date it writes to.
  Inserts stay unvalidated (SPEC §1.2, §4.3).
- No change to pairing, the splice, totals, or `mlm week`.
- A `stop` that triggers no warning (§3) prints nothing, even when the
  lookback would find stale starts. Only `start` warns unconditionally.
- No warning for an overlap (E21), or a wrong start left open on an E7
  date, that a `stop` creates.
- No change to `start`'s warnings or to `note`.
- No `--quiet` flag.

## 1.2 Known issues

- Backfilling stop-then-start warns once. `mlm stop 17:00 --date -1`
  followed by `mlm start 09:00 --date -1` is legal (SPEC §3.2). The
  first command prints an orphan warning whose `or:` line suggests
  deleting a correct punch. When the day before holds a lone open start,
  it prints a long-splice warning instead. The second command clears
  either.
- A warning that was already true before the write repeats on every
  `stop` to `D` or `D-1` until it is fixed. The warnings do not say
  which of them this `stop` caused, and they never name the punch just
  typed.
- A backdated `stop` gets today's lookback, not the written date's. A
  stale start beside an old written date, outside the 14-day window,
  does not show.
- `stop` now writes to stderr. A script that treats any stderr from
  `mlm stop` as failure, or logs `2>&1`, sees new output.
- The 12h long-splice threshold is a fixed value. A genuine overnight
  session of 12h or more warns, and the yesterday hint drops its
  `or: mlm stop` line for it.
- An E8 window can be empty. An orphan at `12:00` followed by one at
  `12:01` gives `(a time between 12:00 and 12:01)`, which no `HH:MM`
  satisfies. The hint text does not show whether `P`'s minute itself
  works (§5).

## 2. Prerequisites

None.

## 3. Triggers

After a successful `stop` write to date `D`, `stop` checks `D` and
`D+1`. `D+1` is skipped when `D` is today. Each date gets the checks
below, and one `stop` can print several warnings.

### 3.1 Orphaned ends and refused joins

`stop` classifies each date with the splice applied, as `status` does
(SPEC §4.3.1). It warns once for every orphaned end on the date, and
once for every refused join the date's `status` page shows.

- `D` covers an end the new punch displaced. Given `start 09:00` and
  `stop 17:00`, a new `stop 12:00` pairs with `09:00` and leaves `17:00`
  orphaned.
- `D+1` covers a splice the write broke by closing `D`'s start. Given
  yesterday `start 22:00` and today `stop 01:00`, spliced, a new
  `mlm stop 23:00 --date -1` leaves today's `01:00` orphaned.
- A refused join covers a splice the write broke from the other side.
  Given the same splice, a second `mlm stop` today leaves two orphans,
  so the join is refused and yesterday's start reopens uncounted. The
  refused-join line says so.
- An end that splices with the previous date's open start is not
  orphaned.

### 3.2 Long splice

`stop` warns once for every splice of 12h or more whose end lies on `D`
or `D+1`. A splice shorter than 12h is an ordinary overnight session and
stays silent. The splice into `D` covers today's bare `stop` after a
forgotten start. The splice into `D+1` covers a backdated `stop` that
closes one of `D`'s two open starts, so `D`'s other start now joins
`D+1`'s first end.

## 4. Output

All warnings go to stderr. Stdout stays empty and the exit code stays
`0`. Warnings for `D` print first, then `D+1`. Within a date the order
follows `status` (SPEC §7.3): orphaned ends ascending by instant, then a
refused join with the previous date, then the next date. The long-splice
blocks print after the date warnings, `D` first. The lookback lines
print last.

### 4.1 Orphan block

Three lines per orphaned end. They match `status`'s E8 block (SPEC
§7.3, with §5's changes) word for word:

```
warning: orphaned end on Mon 2026-10-05 at 14:16 (no matching start)
    fix: mlm start HH:MM --date 2026-10-05  (a time before 14:16)
     or: mlm delete punch --date 2026-10-05  (lists punches; rerun with the end's number)
```

When another end shares the orphan's minute, the block has two lines
(§5):

```
warning: orphaned end on Mon 2026-10-05 at 17:00 (no matching start)
    fix: mlm delete punch --date 2026-10-05  (lists punches; rerun with the end's number)
```

### 4.2 Refused-join line

One line, `status`'s refused-join text (SPEC §7.3) with `warning:` in
place of `[!]`:

```
warning: not joined to Sun 2026-10-04 across midnight (a join needs exactly one open start on Sun 2026-10-04 and exactly one unmatched end, as Mon 2026-10-05's first punch)
```

### 4.3 Long-splice block

Three lines. `S` is the start's time on the earlier date `A`. `E` is the
end's time on the later date `A+1`. The `fix:` hint follows the
stop-hint rule (SPEC §7.3), so it names a window when a punch on `A`
follows `S`:

```
warning: end on Mon 2026-10-05 at 14:16 joins the start on Sun 2026-10-04 at 09:00 (29h 16m, counted on Sun 2026-10-04)
    fix: mlm stop HH:MM --date 2026-10-04  (a time after 09:00)
   then: mlm status 2026-10-05  (the 14:16 end shows as orphaned, with its fixes)
```

Stopping `A`'s start first keeps the end on `A+1`. It becomes an
orphan, and its own E8 block offers a start before it or a delete. The
duration uses the one formatter (SPEC §4.2).

### 4.4 Lookback lines

When any warning above prints, `stop` then prints the 14-day lookback,
oldest first, in the stderr forms `start` prints (SPEC §7.4). The
lookback runs from today, not from `D`. It is filtered as today's
`status` page filters it (SPEC §7.1). The yesterday warning prints only
when today's `status` would print the yesterday hint. So a refused join
never yields a yesterday `fix:` that would pair the overnight start with
a same-day stop. `stop` suppresses nothing else, because its own punch
is an end and never appears as a lookback start. `stop` prints no
carry-in marker.

### 4.5 Examples

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
warning: end on Mon 2026-10-05 at 14:16 joins the start on Sun 2026-10-04 at 09:00 (29h 16m, counted on Sun 2026-10-04)
    fix: mlm stop HH:MM --date 2026-10-04  (a time after 09:00)
   then: mlm status 2026-10-05  (the 14:16 end shows as orphaned, with its fixes)
```

A double-tapped `stop` after an overnight splice:

```
$ mlm start 22:00 --date -1
$ mlm stop 01:00
$ mlm stop 01:00
warning: orphaned end on Mon 2026-10-05 at 01:00 (no matching start)
    fix: mlm delete punch --date 2026-10-05  (lists punches; rerun with the end's number)
warning: orphaned end on Mon 2026-10-05 at 01:00 (no matching start)
    fix: mlm delete punch --date 2026-10-05  (lists punches; rerun with the end's number)
warning: not joined to Sun 2026-10-04 across midnight (a join needs exactly one open start on Sun 2026-10-04 and exactly one unmatched end, as Mon 2026-10-05's first punch)
```

## 5. E8 fix hint

The E8 block changes everywhere it prints: `status`, and `stop`'s
orphan block.

An orphaned end at `T` needs a start that pairs with it and creates no
overlap.

- When another end on the date shares `T`'s minute, the block drops its
  `fix:` start line, and the delete line becomes `fix:`. That shape is a
  duplicate stop, and no start can pair with one copy without nesting
  around the other.
- Otherwise, let `P` be the latest punch on the date before `T`'s
  minute, in pairing order (SPEC §4.3, step 1). With no other end at
  `T`'s minute, the stack is empty at `T`, so `P` is always an end.
- With no `P`, the hint stays `(a time before T)`.
- With one, it reads `(a time between P and T)`. A start in the window
  leaves `T` paired and nothing overlapping.
- The window includes `P`'s minute when `P` closed a stint. A start at
  that instant either pairs with `T`, or pairs with `P` at zero length
  (E14) while `P`'s old start moves to `T`. Both are clean. When `P` is
  an orphaned end, a start at `P`'s minute pairs with `P` and leaves `T`
  orphaned, so the window excludes it.
- The window can be empty (§1.2).

The displaced example (`start 09:00`, `stop 12:00`, orphan `17:00`)
reads `(a time between 12:00 and 17:00)`.

## 6. Yesterday hint

The yesterday hint's `or: mlm stop` line (SPEC §7.1, §7.3) gains one
more condition. It prints only while the start is under 12h old at
`now`. From 12h on, a bare `stop` would draw the long-splice warning, so
the hint stops suggesting it. The `fix:` line is unchanged.

## 7. Errors

The check is best-effort, as `start`'s is (SPEC §7.4). `stop` does all
its reads first, then prints. If any read after the committed write
fails, `stop` prints nothing extra and still exits `0`. A hard error
before the write (SPEC §6.1) is unchanged.

## 8. Testing

`stop` warnings, each asserting exact stderr, empty stdout and exit `0`:

- orphan on `D`, empty lookback → orphan block only, `(a time before T)`
- FE5 → orphan block, then the stale-start lines
- clean pair, stale start in the window → silent
- displaced end → orphan block with `(a time between 12:00 and 17:00)`
- pre-existing orphan, clean pair on the same date → orphan block
- two orphans on one date at different minutes → two blocks, ascending
- duplicate same-minute stop → two-line blocks
- double-tapped `stop` after a splice → two-line blocks, refused-join line,
  no yesterday warning
- backdated `stop` breaking `D+1`'s splice → orphan block for `D+1`
- `D` is today → `D+1` not read
- splice of 11h 59m → silent
- splice of exactly 12h, and of 29h 16m → long-splice block
- long splice with a punch after `S` on `A` → `fix:` window
- backdated `stop` that clears `D`'s E7 and creates a 12h+ splice into
  `D+1` → long-splice block for `D+1`
- `D+1` orphan and a long-splice block from one `stop` → order per §4
- orphan with yesterday's lone unrefused start open → orphan block, then
  the yesterday warning
- `--date` older than 14 days → orphan block, lookback from today
- read failure after the write → nothing extra, exit `0`

`status`:

- E8 window: no earlier punch; earlier closed stint (minute included);
  earlier orphan (minute excluded); an E14 pair as `P`; another end at
  `T`'s minute (two-line block); empty window
- yesterday hint: start under 12h old → `or:` line; 12h or more → no
  `or:` line

## 9. User-visible surface

- `README.md`:
  - the write-commands line ("Every `start`/`stop`/`note` call is
    silent on success") names the `start` and `stop` warnings
  - `mlm stop` gains the warning text and the §4.5 examples
  - the "joins the start and clears the hint" sentence notes the 12h
    warning
  - the `[i]` hint example next to `22:00-now (15h 30m so far ...)`
    drops its `or:` line
  - Possible issues mirrors the new SPEC §1.2a items
- `mlm stop --help` is unchanged. `start --help` does not mention its
  warnings either.

## Interaction with the project spec

- §1.2a gains the known issues in §1.2 here.
- §3.3 points to §7.4's `stop` warnings.
- §3.7's "`start`, `stop` and `note` stay silent" names the `stop`
  exception.
- §6.2's "shows up in that date's next `status`/`week` output, not at
  insert time" names the `stop` warning.
- §7.1: the yesterday hint's `or:` condition gains the 12h rule (§6
  here). The overnight example at 15h 30m drops its `or:` line. The
  "joins the start and clears the hint" sentence notes the 12h warning.
  Text saying an old start drops out of "`mlm start`'s warnings" names
  `stop`'s too.
- §7.3: the E8 block gains §5's rules. The stop-hint rule's consumer
  list adds `mlm stop`'s long-splice block.
- §7.4 gains `stop`'s triggers, blocks, order and best-effort rule. The
  "write commands print nothing" rule gains the exception.
- §8.2 gains E27, `stop`'s warnings, next to E19. E8 and E22 change to
  match.
- `AGENTS.md` and `src/lookback.rs` docs that describe the stderr forms
  as `start`'s alone name `stop` too.
