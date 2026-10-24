# Task plan: orphan-stop-warning, Task 2 (docs)

Changeset plan: `docs/dev/plans/orphan-stop-warning-plan.md`, "Task 2: docs".
Spec: `docs/dev/specs/2026-10-24-orphan-stop-warning.md` (locked).

## Read for this plan

- `docs/dev/plans/orphan-stop-warning-plan.md` (all 132 lines)
- `docs/dev/specs/2026-10-24-orphan-stop-warning.md` (all 358 lines)
- `docs/dev/SPEC.md` (all 1593 lines)
- `README.md` (all 571 lines)
- `AGENTS.md` (all 165 lines), including "Documentation" (`AGENTS.md:59-72`)
- `docs/dev/README.md` (all 12 lines)
- `src/cli.rs:30-57` (the `status --help` glossary, to confirm the
  known-issue quote "Add a start before it, or delete it" at `src/cli.rs:42-43`)
- Sibling task plan `docs/dev/plans/orphan-stop-warning-task-1-code.md`
  did not exist when this plan was written. This task needs none of its
  signatures (see Dependencies).

## Scope

| File | Owner |
|---|---|
| `docs/dev/SPEC.md` | this task |
| `README.md` | this task |
| `AGENTS.md` | this task |
| `src/**`, `tests/**`, `coverage-baseline.json`, `--help` text in `src/cli.rs` | Task 1 (code) |
| `docs/dev/NOTES.md` | coordinator |
| the working spec and plans (archiving) | a later phase |

No source file is touched. No doc gains a pointer into `docs/dev/specs/`
or `docs/dev/plans/`.

## Dependencies

| Dependency | State | What this task uses | Stub today | Swap at integration |
|---|---|---|---|---|
| Task 1 output text | not landed | the exact stderr lines and E8 hint lines | the locked spec's §4.1-§4.5, §5, §6 text, copied verbatim below | none. If Task 1's completion report records an output-text deviation, the re-alignment pass edits the matching example here |
| Task 1 module name | pinned by changeset plan | `src/stop_warnings.rs` in AGENTS.md | the pinned name | none |
| Task 1 `src/lookback.rs` stderr-forms wording | pinned by changeset plan | AGENTS.md bullet names `stop` beside `start` | the pinned wording | none |

## Facts gathered (current text)

### SPEC.md

| Line(s) | Section | What it says now |
|---|---|---|
| 58-64 | §1.2a | Possible-issues intro, then "None currently." at 64 |
| 225-227 | §3 | `-v` off logs only warnings/errors, "so a run that creates and migrates a new database prints nothing" |
| 256-258 | §3.3 | "Same shape as `start`, inserts an `end` punch." |
| 356 | §3.7 | "`start`, `stop` and `note` stay silent (§7.4)." |
| 454-456 | §4.3 | edge-case lead-in, "not rejected at insert time (no editing/validation — see §1.2):" |
| 644-646 | §6.2 | orphan/E7 anomaly "shows up in that date's next `status`/`week` output, not at insert time." |
| 867-868 | §7.1 lookback bullet | "drops out of today's `status` and out of `mlm start`'s warnings." |
| 875-879 | §7.1 yesterday-hint bullet | "joins the start and clears the hint" sentence (875-876); `or:` condition (878-879) |
| 934-950 | §7.1 overnight example | lead-in at 934-937; `or:` line at 947 |
| 1187-1189 | §7.3 stop-hint rule | consumer list ends "and `mlm start`'s warnings." |
| 1227-1233 | §7.3 E8 | "E8, orphaned end (line text unchanged, fix lines new):" plus the three-line block |
| 1292-1302 | §7.3 yesterday `[i]` | lead-in and the `18:02` `[i]` block with its `or:` line |
| 1304-1308 | §7.3 | standalone `or:` condition sentence |
| 1330-1332 | §7.4 | "`start`, `stop` and `note` print nothing on success. They are silent, ..." |
| 1385-1387 | §7.4 | `start`'s best-effort paragraph, last lines of §7.4 |
| 1389 | §8 | `## 8. User flows (test basis)` heading |
| 1460-1461 | §8.1 F13 | "First run on a fresh database prints nothing on stderr." |
| 1488-1492 | §8.2 E8 | orphaned end flow |
| 1541-1549 | §8.2 E19 | `mlm start`'s warning flow; E20 follows at 1550 |
| 1561-1570 | §8.2 E22 | yesterday hint flow with the old `or:` condition |

SPEC's own section numbers used for retargeting the working spec's local
references:

| Working spec ref | SPEC.md target |
|---|---|
| §1.2 (known issues) | §1.2a |
| §3, §4, §4.1-§4.5, §7 (`stop` triggers, output, errors) | new §7.4.1 |
| §5 (E8 window) | §7.3 (E8 block) |
| §6 (yesterday hint) | §7.1 and §7.3 (yesterday hint) |
| SPEC §x | §x |
| FE5 | E27 |

### README.md

| Line(s) | What it says now |
|---|---|
| 43-48 | "### Possible issues", intro, "None currently." |
| 115-122 | "Every `start`/`stop`/`note` call is silent on success ... That holds on the first run too, when the database is created." |
| 176-184 | `mlm stop` section, one example `$ mlm stop 13:00`, no warning text |
| 253-265 | lookback and yesterday-hint lead-in, then the `18:02` `[i]` block with `or:` |
| 267-271 | `or:` condition sentence and "joins the start and clears the hint" sentence |
| 273-289 | overnight stint line lead-in and the `22:00-now (15h 30m so far ...)` example, `or:` at 286 |

### AGENTS.md

| Line(s) | What it says now |
|---|---|
| 43-44 | "`src/lookback.rs` — today's lookback scan (ages 1-14), the yesterday hint and `mlm start`'s stderr warnings" |
| 45 | `src/history.rs` bullet follows |

## Steps

Apply the steps bottom-up within each file (highest line first) so the
line numbers above stay valid while editing. Match by the quoted current
text, not by line number alone. Re-wrap prose at the file's existing
width (about 72 columns); code blocks keep their exact lines.

### SPEC.md

#### S1. §8.2 E22 (lines 1561-1570)

Current:

```text
- **E22** - Start still open from yesterday (§7.1, §7.4): today's
  `status` prints the `[i]` hint, with the `or:` line only when no punch
  follows the start on yesterday's date and today has no punches. The
  stint list opens with `HH:MM-now (DURATION so far, started yesterday,
  not counted until stopped)` ahead of today's own stints, and no figure
  changes. A `stop` as today's first punch joins the start and the hint
  and the line are gone. A refused-join line naming yesterday, or two
  open starts yesterday, suppresses both. At age two the line and hint
  go and the `[!]` stale lookback takes over. The hint never shows on
  `status -1`, which keeps its own open-stint line.
```

Replacement:

```text
- **E22** - Start still open from yesterday (§7.1, §7.4): today's
  `status` prints the `[i]` hint, with the `or:` line only when no punch
  follows the start on yesterday's date, today has no punches, and the
  start is under 12h old. A start 719 minutes old keeps the `or:` line
  and one 720 minutes old drops it, while the `fix:` line stays. The
  stint list opens with `HH:MM-now (DURATION so far, started yesterday,
  not counted until stopped)` ahead of today's own stints, and no figure
  changes. A `stop` as today's first punch joins the start and the hint
  and the line are gone. Once the start is 12h old, that `stop` also
  prints the long-splice warning (E27). A refused-join line naming
  yesterday, or two open starts yesterday, suppresses both. At age two
  the line and hint go and the `[!]` stale lookback takes over. The hint
  never shows on `status -1`, which keeps its own open-stint line.
```

#### S2. §8.2 E27, inserted after E19 (after line 1549, before E20 at 1550)

Insert, as a new bullet between E19 and E20:

```text
- **E27** - `mlm stop`'s warnings (§7.4.1): with today Mon 2026-10-05,
  `mlm start 22:00 --date -2` then a bare `mlm stop` prints the
  orphaned-end block for today's end, then the unclosed-start warning
  for Saturday's start, to stderr. Stdout stays empty and exit stays
  `0`. A start forgotten yesterday and a bare `stop` print the
  long-splice block. A double-tapped `stop` after an overnight splice
  prints two delete-only blocks and the refused-join line, and no
  yesterday warning. A displaced end prints its block with `(a time
  between 12:00 and 17:00)`. A backdated `stop` that breaks the next
  date's splice warns about the next date's orphan. A splice of 11h 59m
  is silent, and exactly 12h warns. A `stop` that leaves no orphaned
  end, refused join or long splice prints nothing, even with stale
  starts in the lookback. A read failure after the write prints nothing
  extra and still exits `0`.
```

#### S3. §8.2 E8 (lines 1488-1492)

Current:

```text
- **E8** — Orphaned `end` (empty-stack case, §4.3): the punch is
  stored but produces no stint-list line of its own — it only
  surfaces as an anomaly line naming its timestamp, now with `fix:`/
  `or:` lines naming the start/delete commands to resolve it (§7.3).
  Two or more orphaned ends on the same date each get their own line.
```

Replacement:

```text
- **E8** — Orphaned `end` (empty-stack case, §4.3): the punch is
  stored but produces no stint-list line of its own — it only
  surfaces as an anomaly line naming its timestamp, with `fix:`/`or:`
  lines naming the start/delete commands to resolve it (§7.3).
  Two or more orphaned ends on the same date each get their own line.
  With an earlier punch on the date the start hint names a window,
  `(a time between P and T)`. A start at `P`'s minute counts when `P`
  closed a stint and not when `P` is an orphaned end. Another end at the
  orphan's minute gives the two-line, delete-only block. A `stop` that
  leaves an orphaned end on its date or the next warns on stderr (E27).
```

#### S4. §8.1 F13 (lines 1460-1461)

Current:

```text
- **F13** - First run on a fresh database prints nothing on stderr. With
  `--verbose` the migration line and debug lines appear (§3).
```

Replacement:

```text
- **F13** - First run on a fresh database prints nothing on stderr,
  except that a first `mlm stop` warns about its orphaned end (§7.4.1).
  With `--verbose` the migration line and debug lines appear (§3).
```

#### S5. New §7.4.1, inserted after line 1387 (end of §7.4), before `## 8.` at 1389

Insert, with one blank line before and after. Every example block is
copied verbatim from spec §4.1-§4.5.

````text
### 7.4.1 `stop` warnings

`stop` has its own narrow exception, on stderr only. After a successful
write to date `D`, it checks `D` and `D+1`, and skips `D+1` when `D` is
today. One `stop` can print several warnings. Stdout stays empty and
exit stays `0`. A `stop` that triggers none of the checks below prints
nothing, even when the lookback would find stale starts. Only `start`
warns unconditionally.

`stop` classifies each checked date with the splice applied, as that
date's `status` does (§4.3.1), and warns about three things:

- Every orphaned end on the date, once each. An end that splices with
  the previous date's open start is not orphaned. Given `start 09:00`
  and `stop 17:00`, a new `stop 12:00` pairs with `09:00` and leaves
  `17:00` orphaned on `D`. Given yesterday `start 22:00` and today `stop
  01:00`, spliced, a new `mlm stop 23:00 --date -1` closes yesterday's
  start and leaves today's `01:00` orphaned on `D+1`.
- Every refused join the date's `status` page shows, once each. A
  refused join between `D` and `D+1` shows on both pages, but prints
  once, as `D`'s line naming `D+1`. Given the same splice, a second `mlm
  stop` today leaves two orphans, so the join is refused and yesterday's
  start reopens uncounted.
- Every splice of 12h or more whose end lies on `D` or `D+1`, once each.
  Exactly 12h warns. A splice under 12h is an ordinary overnight session
  and stays silent. The splice into `D` covers a bare `stop` today after
  a start forgotten yesterday. The splice into `D+1` covers a backdated
  `stop` that closes one of `D`'s two open starts, so `D`'s other start
  now joins `D+1`'s first end.

A `stop` that creates an overlap (§4.3), or leaves the wrong start open
on an E7 date, draws no warning.

Warnings for `D` print first, then `D+1`. Within a date the order
follows `status` (§7.3). Orphaned ends print ascending by instant, then
a refused join with the previous date, then one with the next date. The
long-splice blocks print after the date warnings, `D` first. The
lookback lines print last.

An orphaned end prints three lines. The header names the date, which
`status`'s E8 header leaves to its page. The lines after the header
match `status`'s E8 block (§7.3) word for word, window included:

```
warning: orphaned end on Mon 2026-10-05 at 14:16 (no matching start)
    fix: mlm start HH:MM --date 2026-10-05  (a time before 14:16)
     or: mlm delete punch --date 2026-10-05  (lists punches; rerun with the end's number)
```

When another end shares the orphan's minute, the block has two lines
(§7.3):

```
warning: orphaned end on Mon 2026-10-05 at 17:00 (no matching start)
    fix: mlm delete punch --date 2026-10-05  (lists punches; rerun with the end's number)
```

A refused join prints one line, `status`'s refused-join text (§7.3)
with `warning:` in place of `[!]`:

```
warning: not joined to Sun 2026-10-04 across midnight (a join needs exactly one open start on Sun 2026-10-04 and exactly one unmatched end, as Mon 2026-10-05's first punch)
```

A long splice prints three lines. `S` is the start's time on the
earlier date `A`, and `E` is the end's time on the later date `A+1`.
The `fix:` hint follows the stop-hint rule (§7.3), so it names a window
when a punch on `A` follows `S`:

```
warning: end on Mon 2026-10-05 at 14:16 joins the start on Sun 2026-10-04 at 09:00 (29h 16m, counted on Sun 2026-10-04)
    fix: mlm stop HH:MM --date 2026-10-04  (a time after 09:00)
   then: mlm status 2026-10-05  (the 14:16 end shows as orphaned, with its fixes)
```

Stopping `A`'s start first keeps the end on `A+1`. It becomes an
orphan, and its own E8 block offers a start before it or a delete. The
duration uses the one formatter (§4.2).

When any warning above prints, `stop` then prints the 14-day lookback,
oldest first, in the stderr forms `start` prints (§7.4). The yesterday
warning keeps that form's two lines, with no `or:` line. The lookback
runs from today, not from `D`. It is filtered as today's `status` page
filters it (§7.1). The yesterday warning prints only when today's
`status` would print the yesterday hint. So a refused join never yields
a yesterday `fix:` that would pair the overnight start with a same-day
stop. `stop` suppresses nothing else, because its own punch is an end
and never appears as a lookback start. `stop` prints no carry-in
marker.

A start from two days ago still open, and a bare `stop` today, with
today Mon 2026-10-05:

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

The check is best-effort, as `start`'s is (§7.4). `stop` does all its
reads first, then prints. If any read after the committed write fails,
`stop` prints nothing extra and still exits `0`. A hard error before
the write (§6.1) still rejects the command with nothing written.
````

#### S6. §7.4 opening sentence (lines 1330-1332)

Current:

```text
`start`, `stop` and `note` print nothing on success. They are silent,
Unix-conventional, and exit `0` (§6.3) is the only signal. A hard error
(§6.1) still prints its message to stderr as usual. `status` and `week`
```

Replacement:

```text
`start`, `stop` and `note` print nothing on success, apart from the
stderr warnings `start` (below) and `stop` (§7.4.1) print. They are
silent on stdout, Unix-conventional, and exit `0` (§6.3) is the only
signal. A hard error (§6.1) still prints its message to stderr as
usual. `status` and `week`
```

The rest of the paragraph (from "produce stdout output (§7.1, §7.2)."
on) is unchanged.

#### S7. §7.3 yesterday `or:` condition sentence (lines 1304-1308)

Current:

```text
The `or:` line prints only when no punch follows the open start on
yesterday's date and today has no punches yet. A `stop` now joins the
start only when it becomes today's first punch. A later punch yesterday
means the start is probably a forgotten stop, and joining it would
nest it over that later stint.
```

Replacement:

```text
The `or:` line prints only when no punch follows the open start on
yesterday's date, today has no punches yet, and the start is under 12h
old at `now`. A `stop` now joins the start only when it becomes today's
first punch. A later punch yesterday means the start is probably a
forgotten stop, and joining it would nest it over that later stint.
From 12h on, a bare `stop` would draw `mlm stop`'s long-splice warning
(§7.4.1), so the hint stops suggesting it. The `fix:` line prints at
any age.
```

#### S8. §7.3 yesterday `[i]` lead-in (lines 1292-1296)

Current:

```text
Yesterday with exactly one unjoined open start gets an `[i]` hint
instead of an anomaly line (§7.1). `[i]` is the quiet marker. It never
feeds `[!]`, the `[!]` footer, the carry-in marker or `has_any`. It also
marks yesterday's `week` row and adds the `[i]` footer there (§7.2),
and today's stint list gets a line for the same start (§7.1):
```

Replacement:

```text
Yesterday with exactly one unjoined open start gets an `[i]` hint
instead of an anomaly line (§7.1). `[i]` is the quiet marker. It never
feeds `[!]`, the `[!]` footer, the carry-in marker or `has_any`. It also
marks yesterday's `week` row and adds the `[i]` footer there (§7.2),
and today's stint list gets a line for the same start (§7.1). The
example assumes the `18:02` start is under 12h old and today has no
punches:
```

The `[i]` block at 1298-1302 is unchanged.

#### S9. §7.3 E8 block (lines 1227-1233)

Current:

````text
E8, orphaned end (line text unchanged, fix lines new):

```
[!] orphaned end at 18:00 (no matching start)
    fix: mlm start HH:MM --date D  (a time before 18:00)
     or: mlm delete punch --date D  (lists punches; rerun with the end's number)
```
````

Replacement:

````text
E8, orphaned end, with no punch before it on the date:

```
[!] orphaned end at 18:00 (no matching start)
    fix: mlm start HH:MM --date D  (a time before 18:00)
     or: mlm delete punch --date D  (lists punches; rerun with the end's number)
```

An orphaned end at `T` needs a start that pairs with it and creates no
overlap. Let `P` be the latest punch on the date before `T`'s minute,
in pairing order (§4.3, step 1). With no other end at `T`'s minute, the
stack is empty at `T`, so `P` is always an end. With no `P`, the hint
reads `(a time before T)`, as above. With one, it reads `(a time between
P and T)`. A start in that window leaves `T` paired and nothing
overlapping. Given `start 09:00`, `stop 12:00` and an orphan at
`17:00`:

```
    fix: mlm start HH:MM --date D  (a time between 12:00 and 17:00)
```

The window includes `P`'s minute when `P` closed a stint. A start at
that instant either pairs with `T`, or pairs with `P` at zero length
(E14) while `P`'s old start moves to `T`. Both are clean. When `P` is
an orphaned end, a start at `P`'s minute pairs with `P` and leaves `T`
orphaned, so the window excludes it. The hint text does not show which
case applies. The window can be empty. An orphan at `12:00` followed by
one at `12:01` gives `(a time between 12:00 and 12:01)`, which no
`HH:MM` satisfies (§1.2a).

When another end on the date shares `T`'s minute, the block drops its
`fix:` start line, and the delete line becomes `fix:`. That shape is a
duplicate stop, and no start can pair with one copy without nesting
around the other:

```
[!] orphaned end at 17:00 (no matching start)
    fix: mlm delete punch --date D  (lists punches; rerun with the end's number)
```

Both forms print the same way after the header of `mlm stop`'s orphan
warning (§7.4.1).
````

#### S10. §7.3 stop-hint consumer list (lines 1187-1189)

Current:

```text
pairs zero-length, E14). The rule applies to the stale-start line, the
E7 line, the lookback lines, the yesterday hint and `mlm start`'s
warnings.
```

Replacement:

```text
pairs zero-length, E14). The rule applies to the stale-start line, the
E7 line, the lookback lines, the yesterday hint, `mlm start`'s warnings,
and `mlm stop`'s lookback lines and long-splice block (§7.4.1).
```

#### S11. §7.1 overnight example (lines 934-950)

Current lead-in (934-937):

```text
`mlm status` on Monday 2026-10-05 at 13:30, no punches today, with a
start at 22:00 still open from Sunday and a zero carry-in (week 2026-41
runs Monday to Sunday). The line is listed under the Day total and is
not part of it:
```

Replacement lead-in:

```text
`mlm status` on Monday 2026-10-05 at 13:30, no punches today, with a
start at 22:00 still open from Sunday and a zero carry-in (week 2026-41
runs Monday to Sunday). The start is 15h 30m old, so the hint has no
`or:` line (§7.3). The line is listed under the Day total and is not
part of it:
```

In the block, delete line 947 exactly:

```text
     or: mlm stop  (still working: ends it now, across midnight)
```

The block then reads:

```text
Mon 2026-10-05

Day total:     00h 00m
Week 2026-41:  08h 00m left to 08h 00m by end of Monday (fulfillment 00h 00m / target 40h 00m)

[i] start on Sun 2026-10-04 at 22:00 still open (yesterday; not counted until stopped)
    fix: mlm stop HH:MM --date 2026-10-04  (a time after 22:00)

  22:00-now    (15h 30m so far, started yesterday, not counted until stopped)
```

#### S12. §7.1 yesterday-hint bullet (lines 875-879)

Current:

```text
block and the lookback lines (line shape in §7.3). A `stop` typed today
as today's first punch joins the start and clears the hint. There is no
hint when today's page already shows a refused-join line naming
yesterday. The `or:` line prints only when no punch follows the open
start on yesterday's date and today has no punches yet.
```

Replacement:

```text
block and the lookback lines (line shape in §7.3). A `stop` typed today
as today's first punch joins the start and clears the hint. Once the
start is 12h old, that `stop` also prints `mlm stop`'s long-splice
warning (§7.4.1). There is no hint when today's page already shows a
refused-join line naming yesterday. The `or:` line prints only when no
punch follows the open start on yesterday's date, today has no punches
yet, and the start is under 12h old at `now`.
```

#### S13. §7.1 lookback bullet (lines 867-868)

Current:

```text
  open-stints line with `(yesterday)`. A start open for more than 14
  days drops out of today's `status` and out of `mlm start`'s warnings.
```

Replacement:

```text
  open-stints line with `(yesterday)`. A start open for more than 14
  days drops out of today's `status` and out of `mlm start`'s and `mlm
  stop`'s warnings.
```

#### S14. §6.2 orphan bullet (lines 644-646)

Current:

```text
- Any §4.3 pairing anomaly (extra open stints, an orphaned `end`): the
  punch is still inserted; the anomaly shows up in that date's next
  `status`/`week` output, not at insert time.
```

Replacement:

```text
- Any §4.3 pairing anomaly (extra open stints, an orphaned `end`): the
  punch is still inserted, and the anomaly shows up in that date's next
  `status`/`week` output. At insert time only `mlm stop` warns, on
  stderr, about an orphaned end, refused join or long splice on its
  date or the next (§7.4.1). The punch is stored either way.
```

#### S15. §4.3 edge-case lead-in (lines 454-456)

Current:

```text
Edge cases, still just **detected and surfaced in that date's
summary** (§3.5/§3.6 output), not silently fixed and not rejected at
insert time (no editing/validation — see §1.2):
```

Replacement:

```text
Edge cases, still just **detected and surfaced in that date's
summary** (§3.5/§3.6 output), not silently fixed and not rejected at
insert time (no editing/validation — see §1.2). `mlm stop` warns on
stderr about an orphaned end or refused join it leaves, and still
stores the punch (§7.4.1):
```

#### S16. §3.7 silence sentence (line 356)

Current:

```text
`start`, `stop` and `note` stay silent (§7.4).
```

Replacement:

```text
`start`, `stop` and `note` print nothing to stdout. `start` and `stop`
can print warnings on stderr (§7.4, §7.4.1).
```

#### S17. §3.3 (lines 256-258)

Current:

```text
### 3.3 `mlm stop [TIME] [NOTE] [--date DATE]`

Same shape as `start`, inserts an `end` punch.
```

Replacement:

```text
### 3.3 `mlm stop [TIME] [NOTE] [--date DATE]`

Same shape as `start`, inserts an `end` punch. After the write it can
print warnings on stderr about orphaned ends, refused joins and long
splices (§7.4.1).
```

#### S18. §3 verbose paragraph (lines 225-227)

Current:

```text
The global `-v`/`--verbose` flag turns on debug log lines on stderr.
Without it only warnings and errors are logged, so a run that creates
and migrates a new database prints nothing.
```

Replacement:

```text
The global `-v`/`--verbose` flag turns on debug log lines on stderr.
Without it only warnings and errors are logged, so creating and
migrating a new database prints nothing. A first `mlm stop` on a new
database still warns about its orphaned end (§7.4.1).
```

#### S19. §1.2a (line 64)

Current:

```text
None currently.
```

Replacement:

```text
- Backfilling stop-then-start warns once. `mlm stop 17:00 --date -1`
  followed by `mlm start 09:00 --date -1` is legal (§3.2). The first
  command prints an orphan warning whose `or:` line suggests deleting a
  correct punch. When the day before holds a lone open start, it prints
  a long-splice warning instead. The second command clears either.
- A warning that was already true before the write repeats on every
  `stop` that writes to its date or the day before, until it is fixed.
  The warnings do not say which of them this `stop` caused, and they
  never name the punch just typed (§7.4.1).
- A backdated `stop` gets today's lookback, not the written date's. A
  stale start beside an old written date, outside the 14-day window,
  does not show (§7.4.1).
- `stop` writes to stderr when it warns. A script that treats any
  stderr from `mlm stop` as failure, or logs `2>&1`, sees the warning
  lines.
- The 12h long-splice threshold is a fixed value. A genuine overnight
  session of 12h or more warns, and the yesterday hint drops its `or:
  mlm stop` line for it (§7.1).
- A genuine overnight session of 12h or more re-warns on every later
  `stop` to its end date or the day before, and there is nothing to
  fix. Only changing correct data silences it.
- At the 12h boundary, `status` can still offer `or: mlm stop` at 11h
  59m, and a bare `stop` a minute later draws the long-splice warning.
- Past 12h, the yesterday hint's remaining `fix:` ends the session on
  yesterday's date. A user still working has no suggested way to end it
  now without the long-splice warning.
- A bare `stop` today against a start 15 or more days old (E27) prints
  only the orphan block. The lookback window ends at 14 days and `stop`
  prints no carry-in marker, so nothing names the old start.
- An E8 window can be empty. An orphan at `12:00` followed by one at
  `12:01` gives `(a time between 12:00 and 12:01)`, which no `HH:MM`
  satisfies. The hint text does not show whether `P`'s minute itself
  works (§7.3).
- The delete-only `fix:` for a duplicate stop says "rerun with the end's
  number", but `mlm delete punch` lists two identical `end` rows. Either
  number works.
- The `status --help` glossary still says an orphaned end means "Add a
  start before it, or delete it", while a duplicate-minute orphan's
  block offers only the delete.
- A first `mlm stop` on a fresh database writes an orphaned end, so a
  first run is not always silent on stderr (§3, F13).
```

### README.md

#### R1. Overnight example (lines 273-289)

Current lead-in (273-275):

```text
The stint list also opens with a line for that start, ahead of today's
own stints. The time is not in the day total, and the line goes away
together with the hint:
```

Replacement lead-in:

```text
The stint list also opens with a line for that start, ahead of today's
own stints. The time is not in the day total, and the line goes away
together with the hint. Here the start is 15h 30m old, so the hint has
no `or:` line:
```

In the block, delete line 286 exactly:

```text
     or: mlm stop  (still working: ends it now, across midnight)
```

The block then reads:

```text
$ mlm status
Mon 2026-10-05

Day total:     00h 00m
Week 2026-41:  08h 00m left to 08h 00m by end of Monday (fulfillment 00h 00m / target 40h 00m)

[i] start on Sun 2026-10-04 at 22:00 still open (yesterday; not counted until stopped)
    fix: mlm stop HH:MM --date 2026-10-04  (a time after 22:00)

  22:00-now    (15h 30m so far, started yesterday, not counted until stopped)
```

#### R2. `or:` condition and join sentence (lines 267-271)

Current:

```text
The `or:` line only shows when no punch follows the open start on
yesterday's date and today has no punches yet. A `stop` typed
as today's first punch joins the start and clears the hint. Yesterday
with two or more open starts prints the `[!]` line `2 open stints on
Fri 2026-10-02 (yesterday)` instead.
```

Replacement:

```text
The example assumes the start is under 12h old. The `or:` line only
shows when no punch follows the open start on yesterday's date, today
has no punches yet, and the start is under 12h old. A `stop` typed as
today's first punch joins the start and clears the hint. Once the start
is 12h old, that `stop` also prints a long-splice warning (see `mlm
stop` above), so the hint stops offering it. Yesterday with two or more
open starts prints the `[!]` line `2 open stints on Fri 2026-10-02
(yesterday)` instead.
```

#### R3. `mlm stop` section (lines 176-184)

Current:

````text
### `mlm stop|e [TIME] [NOTE...] [-d/--date DATE]`

Record an end punch for today, or another day with `--date`. Same
argument shape and TIME-required-when-backdated rule as `start`
(including the `--date`-before-`NOTE` footgun above).

```sh
$ mlm stop 13:00
```
````

Replacement:

````text
### `mlm stop|e [TIME] [NOTE...] [-d/--date DATE]`

Record an end punch for today, or another day with `--date`. Same
argument shape and TIME-required-when-backdated rule as `start`
(including the `--date`-before-`NOTE` footgun above).

```sh
$ mlm stop 13:00
```

After a successful write, `stop` checks the date it wrote to and the
next date, and skips the next date when you wrote to today. It prints a
`warning:` block to stderr for each orphaned end there (an end with no
start to pair with), each refused overnight join, and each overnight
join of 12h or more (a long splice). When any of those print, `stop`
then prints the same stale-start warnings `start` prints, from today's
14-day lookback. A `stop` that finds none of these prints nothing.
Stdout and the exit code are unaffected either way, and the punch is
stored.

A start from two days ago is still open, and a bare `stop` lands on
today. The overnight join reaches back one day only, so today's end
stays orphaned:

```sh
$ mlm start 22:00 --date -2
$ mlm stop
warning: orphaned end on Mon 2026-10-05 at 14:16 (no matching start)
    fix: mlm start HH:MM --date 2026-10-05  (a time before 14:16)
     or: mlm delete punch --date 2026-10-05  (lists punches; rerun with the end's number)
warning: unclosed start on Sat 2026-10-03 at 22:00 (2 days ago)
    fix: mlm stop HH:MM --date 2026-10-03  (a time after 22:00)
     or: mlm delete punch --date 2026-10-03  (lists punches; rerun with the start's number)
```

The fix is to delete today's stray end and stop Saturday's start with
`--date`. When an earlier punch on the date comes before the orphaned
end, the `fix:` time is a window, `(a time between 12:00 and 17:00)`.
`status` prints the same hint lines under `[!] orphaned end at 14:16
(no matching start)`.

A start forgotten yesterday joins today's bare `stop` into one long
stint:

```sh
$ mlm start 09:00 --date -1
$ mlm stop
warning: end on Mon 2026-10-05 at 14:16 joins the start on Sun 2026-10-04 at 09:00 (29h 16m, counted on Sun 2026-10-04)
    fix: mlm stop HH:MM --date 2026-10-04  (a time after 09:00)
   then: mlm status 2026-10-05  (the 14:16 end shows as orphaned, with its fixes)
```

Stopping the start first leaves today's end as an orphan, and `mlm
status` then shows its fixes. A join under 12h is an ordinary overnight
session and prints nothing.

A second `stop` at the same minute leaves two identical ends, so the
overnight join is refused. Each copy gets a delete-only block, because
no start can pair with one copy without nesting around the other:

```sh
$ mlm start 22:00 --date -1
$ mlm stop 01:00
$ mlm stop 01:00
warning: orphaned end on Mon 2026-10-05 at 01:00 (no matching start)
    fix: mlm delete punch --date 2026-10-05  (lists punches; rerun with the end's number)
warning: orphaned end on Mon 2026-10-05 at 01:00 (no matching start)
    fix: mlm delete punch --date 2026-10-05  (lists punches; rerun with the end's number)
warning: not joined to Sun 2026-10-04 across midnight (a join needs exactly one open start on Sun 2026-10-04 and exactly one unmatched end, as Mon 2026-10-05's first punch)
```
````

#### R4. Write-commands silence line (lines 115-117)

Current:

```text
Every `start`/`stop`/`note` call is silent on success, and nothing
prints unless something went wrong. That holds on the first run too,
when the database is created. `week target` prints one line confirming
```

Replacement:

```text
Every `start`/`stop`/`note` call prints nothing to stdout on success.
`start` and `stop` can print warnings to stderr (see below), and `note`
prints nothing unless something went wrong. Creating the database on
the first run prints nothing either, but a first `mlm stop` warns about
its orphaned end. `week target` prints one line confirming
```

The rest of the paragraph (from "the stored target." on) is unchanged.

#### R5. Possible issues (line 48)

Current:

```text
None currently.
```

Replacement:

```text
- Backfilling a day stop-first warns once. `mlm stop 17:00 --date -1`
  followed by `mlm start 09:00 --date -1` is fine, but the first command
  prints an orphaned-end warning whose `or:` line suggests deleting a
  correct punch. When the day before holds a lone open start, it prints
  a long-splice warning instead. The second command clears either.
- A warning that was already true before a `stop` repeats on every
  `stop` to that date or the day before, until it is fixed. The warnings
  do not say which of them this `stop` caused, and they never name the
  punch just typed.
- A backdated `stop` checks today's 14-day lookback, not the written
  date's. A stale start beside an old written date, outside the 14-day
  window, does not show.
- `stop` writes to stderr when it warns. A script that treats any
  stderr from `mlm stop` as failure, or logs `2>&1`, sees the warning
  lines.
- The 12h long-splice threshold is fixed. A genuine overnight session of
  12h or more warns, and the yesterday hint drops its `or: mlm stop`
  line for it.
- A genuine overnight session of 12h or more re-warns on every later
  `stop` to its end date or the day before, and there is nothing to
  fix. Only changing correct data silences it.
- At the 12h boundary, `status` can still offer `or: mlm stop` at 11h
  59m, and a bare `stop` a minute later draws the long-splice warning.
- Past 12h, the yesterday hint's remaining `fix:` ends the session on
  yesterday's date. A user still working has no suggested way to end it
  now without the long-splice warning.
- A bare `stop` against a start 15 or more days old prints only the
  orphaned-end warning. The lookback ends at 14 days and `stop` prints
  no carry-in marker, so nothing names the old start.
- An orphaned end's `fix:` window can be empty. An orphan at `12:00`
  followed by one at `12:01` gives `(a time between 12:00 and 12:01)`,
  which no `HH:MM` satisfies. The hint does not show whether the
  window's first minute itself works.
- The delete-only `fix:` for a duplicate stop says "rerun with the end's
  number", but `mlm delete punch` lists two identical `end` rows. Either
  number works.
- The `status --help` glossary still says an orphaned end means "Add a
  start before it, or delete it", while a duplicate stop's block offers
  only the delete.
- A first `mlm stop` on a fresh database writes an orphaned end, so it
  warns on stderr.
```

The intro at 45-46 ("Shipped behavior that's rough or confusing in a
way worth fixing later.") is unchanged.

### AGENTS.md

#### A1. Layout list (lines 43-44)

Current:

```text
- `src/lookback.rs` — today's lookback scan (ages 1-14), the yesterday
  hint and `mlm start`'s stderr warnings
```

Replacement:

```text
- `src/lookback.rs` — today's lookback scan (ages 1-14), the yesterday
  hint and the stderr warning forms `mlm start` and `mlm stop` print
- `src/stop_warnings.rs` — `stop`'s post-write warnings: orphaned ends,
  refused joins, long splices, then today's lookback
```

## Check plan

Run every grep from `/home/magikmw/projects/mlm-cli`. "Empty" means no
output. Greps over prose assume the wrapping shown in the steps. If the
implementer re-wraps a passage, run the same pattern over the section
joined into one line (`sed -n '<range>p' FILE | tr '\n' ' ' | grep -o
'<pattern>'`).

### C1. SPEC §1.2a carries every spec §1.2 item, retargeted

- `sed -n '/^### 1.2a/,/^### 1.3/p' docs/dev/SPEC.md | grep -c '^- '`
  prints `13` (the spec's §1.2 has 13 bullets).
- `grep -n 'None currently' docs/dev/SPEC.md README.md` is empty.
- Within §1.2a, `grep -nE 'SPEC §|FE5|\(§5\)|\(§6\)'` is empty (no
  working-spec-local refs survive).

### C2. SPEC §3.3 points to §7.4's `stop` warnings

- `sed -n '/^### 3.3/,/^### 3.4 /p' docs/dev/SPEC.md | grep -n '§7.4.1'`
  returns one line.

### C3. SPEC §3.7 names the `stop` exception

- `grep -n 'stay silent (§7.4)' docs/dev/SPEC.md` is empty.
- `sed -n '/^### 3.7/,/^## 4/p' docs/dev/SPEC.md | grep -n 'can print warnings on stderr'`
  returns one line.

### C4. SPEC §6.2 and §4.3 lead-in name the `stop` warning

- `grep -n 'output, not at insert time' docs/dev/SPEC.md` is empty.
- `sed -n '/^### 6.2/,/^### 6.3/p' docs/dev/SPEC.md | grep -n 'mlm stop'`
  returns a line.
- `sed -n '/^Edge cases, still just/,/^- \*\*One unmatched/p' docs/dev/SPEC.md | grep -n '§7.4.1'`
  returns a line.

### C5. SPEC §3 and F13 note the first `mlm stop` warning

- `grep -n 'and migrates a new database prints nothing' docs/dev/SPEC.md`
  is empty.
- `grep -n 'first `mlm stop`' docs/dev/SPEC.md` returns the §3 line,
  the F13 line and the §1.2a line.

### C6. SPEC §7.1 yesterday hint, overnight example, join sentence, `start`'s warnings

- `sed -n '/^### 7.1/,/^### 7.2/p' docs/dev/SPEC.md | grep -n 'under 12h old'`
  returns the hint bullet's condition line.
- `sed -n '/^### 7.1/,/^### 7.2/p' docs/dev/SPEC.md | grep -c 'or: mlm stop  (still working'`
  prints `0`.
- `grep -n "out of \`mlm start\`'s warnings" docs/dev/SPEC.md` is
  empty; `grep -n "stop\`'s warnings" docs/dev/SPEC.md` finds the
  lookback bullet.
- The join sentence in §7.1 is followed by the long-splice sentence:
  `grep -n 'that `stop` also prints' docs/dev/SPEC.md` returns a §7.1
  line.

### C7. SPEC §7.3 yesterday `or:` sentence and `18:02` example

- `grep -n 'has no punches yet\.' docs/dev/SPEC.md` is empty. It
  matches twice today (§7.1 line 879 and §7.3 line 1305), so empty
  means both old condition sentences are gone.
- `grep -n 'assumes the `18:02` start is under 12h old' docs/dev/SPEC.md`
  returns one line.

### C8. SPEC §7.3 E8 block rules and stop-hint consumer list

- `grep -n 'line text unchanged, fix lines new' docs/dev/SPEC.md` is
  empty.
- `grep -n '(a time between 12:00 and 17:00)' docs/dev/SPEC.md` returns
  the §7.3 window example (and E27).
- `grep -n '(a time between 12:00 and 12:01)' docs/dev/SPEC.md` returns
  §7.3 and §1.2a.
- `grep -n "window excludes it" docs/dev/SPEC.md` returns §7.3.
- `grep -n "long-splice block (§7.4.1)" docs/dev/SPEC.md` returns the
  consumer-list line.

### C9. SPEC §7.4 gains `stop`'s triggers, blocks, order, lookback, examples, best-effort rule

- `grep -n '^### 7.4.1 `stop` warnings' docs/dev/SPEC.md` returns one
  line, and the next `^## ` heading after it is `## 8.`.
- Each spec §4.5 example line appears verbatim. Extract the fenced
  blocks of spec §4.1-§4.5 and diff:
  `diff <(sed -n '155,246p' docs/dev/specs/2026-10-24-orphan-stop-warning.md | grep -E '^(warning|    fix|     or|   then|\$ )' | sort -u) <(sed -n '/^### 7.4.1/,/^## 8/p' docs/dev/SPEC.md | grep -E '^(warning|    fix|     or|   then|\$ )' | sort -u)`
  is empty.
- `sed -n '/^### 7.4.1/,/^## 8/p' docs/dev/SPEC.md` contains "12h or
  more", "Exactly 12h warns", "prints once", "reads first", "still
  exits `0`", "no carry-in marker".
- `grep -n 'print nothing on success\. They are silent,' docs/dev/SPEC.md`
  is empty (the write-command rule names the exception).
- `grep -n 'FE5' docs/dev/SPEC.md README.md` is empty.

### C10. SPEC §8.2 gains E27 next to E19; E8 and E22 match

- `grep -n '^- \*\*E\(19\|27\|20\)\*\*' docs/dev/SPEC.md` lists E19,
  E27, E20 in that order.
- `sed -n '/^- \*\*E8\*\*/,/^- \*\*E9\*\*/p' docs/dev/SPEC.md | grep -n 'between P and T'`
  returns a line, and `grep -n 'now with `fix:`/' docs/dev/SPEC.md` is
  empty.
- `sed -n '/^- \*\*E22\*\*/,/^- \*\*E23\*\*/p' docs/dev/SPEC.md | grep -n 'under 12h old'`
  returns a line.

### C11. README

- `grep -n 'silent on success' README.md` is empty;
  `grep -n 'can print warnings to stderr' README.md` returns one line.
- `grep -n 'first `mlm stop`' README.md` returns the usage paragraph
  and the Possible-issues bullet.
- The three spec §4.5 examples appear verbatim in the `mlm stop`
  section. Run the C9 `diff` with the left side narrowed to §4.5
  (`sed -n '207,246p'` of the working spec) and the right side
  `sed -n '/^### `mlm stop|e/,/^### `mlm note/p' README.md`. It prints
  only `> $ mlm stop 13:00`, the section's existing example.
- `grep -c 'or: mlm stop  (still working' README.md` prints `1` (only
  the `18:02` example keeps it).
- `grep -n 'under 12h old' README.md` returns the condition sentence.
- `grep -n 'that `stop` also prints a long-splice warning' README.md`
  returns one line.
- `sed -n '/^### Possible issues/,/^## Install/p' README.md | grep -c '^- '`
  prints `13`, and `grep -n '§' README.md` is empty.

### C12. AGENTS.md

- `grep -n 'src/stop_warnings.rs' AGENTS.md` returns one line, and
  `grep -n "mlm start\`'s stderr warnings" AGENTS.md` is empty.

### C13. No pointer into working docs

- `grep -nE 'docs/dev/(specs|plans)' docs/dev/SPEC.md README.md AGENTS.md`
  is empty.

### C14. Style and hygiene

- `git diff main -- docs/dev/SPEC.md README.md AGENTS.md | grep '^+' | grep -P '[^\x00-\x7F]'`
  shows only lines that kept an existing em dash (S3's E8 bullet and
  S15's lead-in). New sentences add none.
- Read each replaced passage once for colon-hinged sentences and
  verbless fragments (house style in `/home/magikmw/.claude/CLAUDE.md`).
  Colons appear only before code blocks and before a list.
- `git diff --stat main -- src tests` is empty for this task's commits.

## Risks, ambiguities, disagreements

1. **New subsection number.** The changeset plan says "SPEC §7.4 gains"
   the `stop` rules. This plan puts them under a new `### 7.4.1 \`stop\`
   warnings` so other sections can cite them precisely. SPEC already
   uses the pattern (`### 4.3.1`). All new cross-refs cite `§7.4.1`, and
   the §3.3 pointer criterion is met through it.
2. **FE5 is a working-spec id.** The spec §4.5 lead-in "FE5, with today
   Mon 2026-10-05:" cannot ship verbatim, because FE5 means nothing in
   SPEC.md. S5 uses "A start from two days ago still open, and a bare
   `stop` today, with today Mon 2026-10-05:" and §1.2a cites E27. The
   example lines themselves are verbatim.
3. **Yesterday stderr form under `stop`.** SPEC §7.4 (line 1365-1366)
   explains the stderr yesterday form's missing `or:` line as "because
   today now has a punch". A backdated `stop` can leave today empty, so
   that reason does not hold for `stop`. The plan leaves `start`'s
   sentence alone and states in §7.4.1 that the yesterday warning keeps
   its two-line form with no `or:` line. The changeset plan (Target
   shape 3) agrees that the stderr form is unchanged.
4. **README `18:02` example.** The changeset plan's README list does not
   name the `18:02` `[i]` example, but under the 12h rule it is only
   true before 06:02. R2 adds "The example assumes the start is under
   12h old," matching the SPEC §7.3 change. Small scope extension inside
   an owned file.
5. **E27 numbering out of order.** E27 sits between E19 and E20, as the
   interface contract pins. Readers scanning numerically may miss it;
   acceptable per the contract.
6. **Known-issue wording.** §1.2a and README rewrite two spec items to
   present state ("`stop` writes to stderr when it warns", "a first run
   is not always silent") instead of "now writes" / "no longer". Meaning
   is unchanged. The `D`/`D-1` wording is spelled out because SPEC's
   §1.2a has no local `D`.
7. **Output-text drift.** All example text comes from the locked spec.
   If Task 1 ships different text and records it, a re-alignment pass
   must update S5, S9 and R3. C9 and C11's diff checks catch drift
   against the spec, not against shipped output. Running the three
   §4.5 scenarios with `MLM_DB_PATH` after integration is the final
   check, but times (`14:16`) depend on the clock, so compare shape, not
   minutes.
8. **README "long splice" term.** README introduces "long splice" in the
   `mlm stop` section and uses it in later sections and Possible issues.
   That keeps one term across README and SPEC.
