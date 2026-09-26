# Task 6 plan — Docs (open-punch-visibility)

## What I read

- `docs/dev/plans/open-punch-visibility-plan.md` (full) — changeset
  plan, Task 6's own section, the global constraints, and the
  ordering/dependency graph.
- `docs/dev/specs/2026-09-26-open-punch-visibility.md` (full, locked
  design spec) — all of §1-§5, especially §3.4's exact line shapes and
  §1.2/§4 (the docs list this task implements).
- Sibling task plans: `ls docs/dev/plans/` shows no
  `open-punch-visibility-task-{1,2,3,4,5}-*.md` files exist yet. All
  five code-task plans are missing (being written in parallel, per the
  dispatch note). I drafted this plan from the changeset plan and
  locked spec alone, since those already pin every literal string and
  behavior this task needs (Contracts A/B in the changeset plan exist
  precisely so Task 6 doesn't need the sibling plans' internal detail).
- `docs/dev/SPEC.md` (full, 987 lines) — current text of every section
  this task touches: §1.2a, §4.3, §4.3.1, §7.1-§7.4, §8.2.
- `README.md` (full, 399 lines) — current text of Known issues,
  `start`, `status`, and `week` sections.

## Scope

**This task owns, exclusively**: `docs/dev/SPEC.md`, `README.md`.

**Not touched by this task** (owned by siblings, per the changeset
plan's ordering graph):

- `src/stint.rs` — Task 1
- `src/render.rs`, `tests/status_cli.rs` — Task 2
- `src/status.rs` — Task 3
- `src/commands.rs` — Task 4
- `src/week_view.rs` — Task 5

This task runs last, after Tasks 1-5 merge, since it must quote their
final rendered text verbatim (changeset-plan Task 6 acceptance
criterion: "every quoted example string... matches Task 2's actual
rendered text exactly"). Below, every code-block example is drafted
from the locked design spec's §3.4/§3.6 (the wording Task 2 is
contracted to implement verbatim, per Contract A) and flagged
`[VERIFY AT MERGE]` — the implementer re-pulls the exact string from a
real `cargo run` against the merged binary before landing this doc
change, not from this plan or the design spec.

No file is edited by this plan document itself. It is a plan only.

## SPEC.md edits

### Edit 1 — §1.2a: drop four bullets, fold in the 14-day residual bullet

**Current** (lines 64-81, the first four bullets of §1.2a):

```
- Whether an unclosed stint reaching into the next day silently merges
  or gets flagged and left unmerged depends on an internal
  1:1-unambiguous gate (§4.3) the user has no way to observe — nothing
  in `--help`, `status`, or `week` explains why the same-looking
  situation sometimes resolves silently and sometimes doesn't.
- `[!]` anomaly flags (`status` and `week`) name the problem but give
  no remedy guidance — no pointer to `delete`, no suggested next step.
- Whether a lone unclosed `start` gets flagged depends on whether a
  *second* one also exists that date (one is the ordinary open-stint
  case, two-or-more is E7) — the same surface signal ("still open, no
  stop yet") is silent in one case and loudly flagged in the other,
  and the distinction isn't explained anywhere.
- A multi-day-old forgotten `stop` is invisible everywhere except the
  exact calendar date it started: `status` for today, `status` for any
  date in between, and `week`'s per-day table (that date's row just
  reads `00h 00m`) all show zero trace of it. Nothing says "you have an
  open punch from N days ago" anywhere except a `status` query against
  that exact date.
```

**Replacement** (all four bullets removed, replaced by one bullet
carrying the design spec's §1.2 first bullet, with its trailing
changelog clause dropped — see Risks below):

```
- An unclosed start more than 14 days old shows only on its own date's
  `status` and its own `week` row. Today's `status` and `mlm start`
  stop mentioning it once it leaves that 14-day window.
```

Everything from line 82 on (`status DATE` future-date bullet onward)
is untouched.

### Edit 2 — §4.3: add the stale-open-start edge case

**Current** (lines 428-433, the first two edge-case bullets):

```
- **One unmatched trailing `start`** (stack has exactly one item once
  the date's punches are exhausted): the normal open/ongoing stint
  (§1.3), not an anomaly.
- **More than one unmatched trailing `start`**: a data anomaly (LIFO
  matching keeps this to at most one in ordinary use) — each shown as
  open against now, and the summary flags that more than one is open.
```

**Replacement** (new bullet inserted between the two, nothing else in
the list changed):

```
- **One unmatched trailing `start`** (stack has exactly one item once
  the date's punches are exhausted): the normal open/ongoing stint
  (§1.3), not an anomaly.
- **A stale open start**: the case above, once its date turns 2 or
  more calendar days old. A lone open start dated today or yesterday
  stays the ordinary, non-anomalous case above — an overnight session
  may still be genuinely running (§7.1) — but from age 2 on it becomes
  a flagged `[!]` anomaly with its own fix suggestion (§7.3).
- **More than one unmatched trailing `start`**: a data anomaly (LIFO
  matching keeps this to at most one in ordinary use) — each shown as
  open against now, and the summary flags that more than one is open.
```

### Edit 3 — §4.3.1: replace the "undocumented gate" sentence

**Current** (lines 484-487, end of the paragraph after the worked
example of the splice fallback):

```
unconditional guarantee against every possible day's data, only the
ordinary case. This residual case is the known rough edge here — the
gate's own on/off condition is still undocumented in `--help`/`status`
(§1.2a).
```

**Replacement**:

```
unconditional guarantee against every possible day's data, only the
ordinary case. This residual case is the known rough edge here. When
the gate rejects a pair — the earlier date has an open start and the
later date an orphaned end, but not in the exact 1:1, first-punch
shape above — both dates now flag a refused-join anomaly naming each
other (§7.3), so the rejection is no longer silent.
```

### Edit 4 — §7.1: fix the `(unclosed)` cross-reference, add the lookback bullet

**Current** (lines 733-737, end of the open-stint-rendering bullet):

```
  one dated two or more days back drops the
    duration figure entirely — at that age the number is noise, not
    information — and renders as `09:00-now  (unclosed)` instead. See
    §1.2a for the remaining gap this doesn't close (nothing signals the
    stale punch's existence on any *other* date's `status`, or in
    `week`'s per-day table).
```

**Replacement**:

```
  one dated two or more days back drops the
    duration figure entirely — at that age the number is noise, not
    information — and renders as `09:00-now  (unclosed)` instead. That
    same date now also carries a `[!]` stale-open-start anomaly
    (§7.3), and, while it's within the last 14 days, today's own
    `status` surfaces it too (the lookback bullet below). See §1.2a
    for what's still not covered once a date passes the 14-day window.
```

**Current** (lines 740-741, the anomalies bullet, last bullet in
§7.1's list):

```
- Anomalies (§4.3), if any, print between the two lead lines and the
  stint list — see §7.3.
```

**Replacement**:

```
- Anomalies (§4.3), if any, print between the two lead lines and the
  stint list — see §7.3 for the line shapes, including the
  stale-open-start and refused-join kinds.
- For today only (`DATE` omitted, or today's own date given
  explicitly), the anomaly block also lists every date in the last 14
  days that still has an open start, oldest first, each as its own
  `[!]` line with `fix:`/`or:` hints (§7.3). `status DATE` for any
  other date runs no lookback.
```

### Edit 5 — §7.2: `[!]` now fires for stale/refused-join rows, add the footer

**Current** (lines 818-835, the `(ongoing)`/stale-date bullet):

```
- One row per calendar date in the week, always all 7 even if some
  are empty (`00h 00m`) — consistent shape, easy to scan for gaps.
  `(ongoing)` marks a date whose open stint's date equals today, and
  only when the requested week is the current one — not just "any
  date with an open stint." This distinction still matters for the
  §4.3.1 residual case: when a cross-midnight session's earlier date
  *doesn't* qualify for the boundary splice (not the ordinary 1:1,
  first-punch shape — e.g. two dangling starts that date), it's left
  with a stale, permanently-open `start`, an ordinary
  single-trailing-start under §4.3 (not an anomaly) but **not** what
  `(ongoing)` is for. That stale date renders as a plain total
  excluding the open stint's live minutes, with no `(ongoing)` and no
  `[!]` marker — an accepted, silent consequence of §4.3.1's "1:1 or
  nothing" scope, not a bug in this rule. The ordinary case (a clean
  cross-midnight session) now splices into one completed stint on the
  earlier date instead of reaching this fallback at all.
  `status`'s own rendering of that stale date does show the open
  stint's age-based caption/`(unclosed)` form (§7.1) — only `week`'s
  per-day row stays silent about it, another instance of the same
  status/week asymmetry the header suffix (§7.1) also has.
```

**Replacement**:

```
- One row per calendar date in the week, always all 7 even if some
  are empty (`00h 00m`) — consistent shape, easy to scan for gaps.
  `(ongoing)` marks a date whose open stint's date equals today, and
  only when the requested week is the current one — not just "any
  date with an open stint." This distinction still matters for the
  §4.3.1 residual case: when a cross-midnight session's earlier date
  *doesn't* qualify for the boundary splice (not the ordinary 1:1,
  first-punch shape — e.g. two dangling starts that date), it's left
  with a stale, permanently-open `start`, an ordinary
  single-trailing-start under §4.3 (not an anomaly on its own) but
  **not** what `(ongoing)` is for. That stale date renders as a plain
  total excluding the open stint's live minutes, with no `(ongoing)`.
  Once that date turns 2 or more calendar days old, its lone open
  start is itself a stale-open-start anomaly, so the row does carry
  `[!]` (§7.3) even without `(ongoing)` — the two markers answer
  different questions and aren't mutually exclusive. The ordinary case
  (a clean cross-midnight session) now splices into one completed
  stint on the earlier date instead of reaching this fallback at all.
  `status`'s own rendering of that stale date does show the open
  stint's age-based caption/`(unclosed)` form (§7.1); `week`'s row
  shows the total and `[!]` but no equivalent caption, another
  instance of the same status/week asymmetry the header suffix (§7.1)
  also has.
```

**Current** (lines 838-841, the last bullet in §7.2):

```
- Any date in the row list with a §4.3 anomaly gets a marker (§7.3)
  appended to its row rather than a separate block, since the week
  view is already one-line-per-date.
```

**Replacement**:

```
- Any date in the row list with an anomaly (§4.3, plus the
  stale-open-start and refused-join anomalies) gets a marker (§7.3)
  appended to its row rather than a separate block, since the week
  view is already one-line-per-date.
- When at least one row carries `[!]`, one footer line follows the
  table after a blank line:

\`\`\`
[!] needs a look: run mlm status DATE for details and fixes
\`\`\`

  No footer when no row is marked. `week` runs no lookback of any
  kind — the row marker and this one footer line are the only new
  things here.
```

(The escaped backtick fences above are how this plan renders a fenced
block inside a fenced block; the implementer writes a literal triple-
backtick fence in `SPEC.md`, matching every other example in the
file.)

### Edit 6 — §7.3: rewrite for the new line shapes, order, and fix lines

**Current** (lines 843-861, the entire section):

```
### 7.3 Anomaly rendering

Plain-ASCII marker, no color/unicode dependency: prefix `[!] `. In
`status`, one line per anomaly:

```
[!] 2 open stints for this date (unmatched starts)
[!] orphaned end at 18:00 (no matching start)
```

In `week`, appended inline to the affected date's row:

```
  Wed 2026-02-11   08h 00m  [!]
```

(detail deferred to that date's own `status` output rather than
repeated in the week table).
```

**Replacement** (full section rewrite; every code block is
`[VERIFY AT MERGE]` against Task 2's actual output before this lands):

```
### 7.3 Anomaly rendering

Plain-ASCII marker, no color/unicode dependency: prefix `[!] `. Every
`status` anomaly line also carries one or two indented `fix:`/`or:`
lines naming a command to run — there's no automatic repair, the user
runs the named command themselves. Below, `D` stands for the
anomaly's own date (`YYYY-MM-DD`), and `HH:MM` inside a `fix:`/`or:`
line is a literal placeholder the user replaces with a real time.

Lines for one date print in this fixed order: multiple open stints
(E7), stale open start, orphaned ends (E8, ascending by instant), a
refused join with the previous date then the next date, and, for
today only, one lookback line per stale date found in the last 14
days (oldest first, §7.1).

Two fix-command details shape the wording throughout: `mlm delete
punch --date D` only lists and numbers that date's punches (§7.4) —
deleting takes a second run naming the number, so the hint says so.
`mlm stop HH:MM --date D` pairs by time (§4.3): an end before the open
start becomes a new orphaned end, and with several open starts it
closes the latest one before it, so the stop hint names the start
time the new end must follow.

E7, multiple open stints (`14:00` below is the latest open start on
the date):

\`\`\`
[!] 2 open stints for this date (only one can be open at a time)
    fix: mlm delete punch --date D  (lists punches; rerun with the extra start's number)
     or: mlm stop HH:MM --date D  (a time after 14:00 closes the 14:00 start)
\`\`\`

Stale open start (§4.3) — a lone open start on a date 2 or more
calendar days old:

\`\`\`
[!] unclosed start at 09:00, 5 days old (no stop recorded)
    fix: mlm stop HH:MM --date D  (a time after 09:00)
     or: mlm delete punch --date D  (lists punches; rerun with the start's number)
\`\`\`

E8, orphaned end (line text unchanged, fix lines new):

\`\`\`
[!] orphaned end at 18:00 (no matching start)
    fix: mlm start HH:MM --date D  (a time before 18:00)
     or: mlm delete punch --date D  (lists punches; rerun with the end's number)
\`\`\`

Refused join (§4.3.1) — no `fix:` lines of its own, since the
E7/E8/stale lines on the two dates already carry them; the reason
text is the same on both dates:

\`\`\`
[!] not joined to Tue 2026-09-22 across midnight (a join needs exactly one open start on Mon 2026-09-21 and exactly one unmatched end, as Tue 2026-09-22's first punch)
\`\`\`

The other date's line names the first date instead: `not joined to
Mon 2026-09-21 across midnight`, same parenthetical.

Lookback lines (today's `status` only, §7.1) — one open start on the
window date:

\`\`\`
[!] unclosed start on Mon 2026-09-21 at 09:00 (5 days ago)
    fix: mlm stop HH:MM --date 2026-09-21  (a time after 09:00)
     or: mlm delete punch --date 2026-09-21  (lists punches; rerun with the start's number)
\`\`\`

Two or more open starts on the window date:

\`\`\`
[!] 2 open stints on Mon 2026-09-21 (5 days ago)
    fix: see mlm status 2026-09-21
\`\`\`

In `week`, appended inline to the affected date's row:

\`\`\`
  Wed 2026-02-11   08h 00m  [!]
\`\`\`

(detail deferred to that date's own `status` output rather than
repeated in the week table). When at least one row carries `[!]`, one
footer line follows the table (§7.2).
```

### Edit 7 — §7.4: add the `start` stderr-warning exception

**Current** (lines 862-871, the entire section):

```
### 7.4 Write-command output

`start`, `stop`, `note`, and `week target` print nothing on success —
silent, Unix-conventional, exit `0` (§6.3) is the only signal. A hard
error (§6.1) still prints its message to stderr as usual. `status` and
`week` produce stdout output (§7.1, §7.2); `delete note`/`delete
punch` are a narrow, deliberate exception to the write-command-silence
rule above rather than a repeal of it — list mode prints that date's
numbered entries (or `nothing to delete for <date>.` when there are
none), and a successful delete prints a ready-to-run recreate command.
```

**Replacement** (paragraph appended after the existing text, nothing
above it changed):

```
### 7.4 Write-command output

`start`, `stop`, `note`, and `week target` print nothing on success —
silent, Unix-conventional, exit `0` (§6.3) is the only signal. A hard
error (§6.1) still prints its message to stderr as usual. `status` and
`week` produce stdout output (§7.1, §7.2); `delete note`/`delete
punch` are a narrow, deliberate exception to the write-command-silence
rule above rather than a repeal of it — list mode prints that date's
numbered entries (or `nothing to delete for <date>.` when there are
none), and a successful delete prints a ready-to-run recreate command.

`start` gains one more narrow exception, on stderr only. After a
successful write, it runs the same 14-day lookback `status` uses for
today (§7.1) and prints one warning line per stale date found, oldest
first — stdout stays silent and exit stays `0` regardless. The date
just written to draws no warning unless the new punch left it with two
or more open starts (E7); a single open start there is the punch just
written.

\`\`\`
warning: unclosed start on Mon 2026-09-21 at 09:00 (5 days ago); fix: mlm stop HH:MM --date 2026-09-21 (a time after 09:00)
warning: 2 open stints on Mon 2026-09-21 (5 days ago); see mlm status 2026-09-21
\`\`\`

The check is best-effort: if the lookback read fails after the write
already committed, `start` still prints nothing extra and still exits
`0`.
```

### Edit 8 — §8.2: E7/E8 gain fix-line mentions, add E16-E19

**Current** (lines 955-957, E7):

```
- **E7** — Multiple dangling `start`s same day (fat-finger case,
  §4.3): punches all still stored; `status` lists each as its own
  open/ongoing stint and flags the multi-open anomaly.
```

**Replacement**:

```
- **E7** — Multiple dangling `start`s same day (fat-finger case,
  §4.3): punches all still stored; `status` lists each as its own
  open/ongoing stint and flags the multi-open anomaly, now with
  `fix:`/`or:` lines naming the delete/stop commands to resolve it
  (§7.3).
```

**Current** (lines 958-961, E8):

```
- **E8** — Orphaned `end` (empty-stack case, §4.3): the punch is
  stored but produces no stint-list line of its own — it only
  surfaces as an anomaly line naming its timestamp (§7.3). Two or
  more orphaned ends on the same date each get their own line.
```

**Replacement**:

```
- **E8** — Orphaned `end` (empty-stack case, §4.3): the punch is
  stored but produces no stint-list line of its own — it only
  surfaces as an anomaly line naming its timestamp, now with `fix:`/
  `or:` lines naming the start/delete commands to resolve it (§7.3).
  Two or more orphaned ends on the same date each get their own line.
```

**Current** (lines 985-987, end of E15, end of file):

```
  open `start` on day one, silent in `week`'s view (no `(ongoing)`, no
  `[!]`, §7.2), plus a flagged orphaned-`end` anomaly on day two.

```

**Replacement** (E15's text unchanged; four new bullets appended after
it, at the end of the file):

```
  open `start` on day one, silent in `week`'s view (no `(ongoing)`, no
  `[!]`, §7.2), plus a flagged orphaned-`end` anomaly on day two.
- **E16** — A lone open `start` at least 2 calendar days old (§4.3):
  flagged as a stale-open-start anomaly, distinct from the ordinary
  open-stint case at age 0-1, with `fix:`/`or:` lines pointing at
  `stop`/`delete punch` (§7.3). A date with two or more open starts
  still gets only the E7 line, never an extra stale line.
- **E17** — A refused join (§4.3.1): adjacent dates where the earlier
  has an open start and the later an orphaned end, but not in the
  exact 1:1, first-punch splice shape. Both dates flag a `[!]` line
  naming the other date (§7.3), with no `fix:` lines of their own,
  since the E7/E8/stale lines on those dates already carry them. A
  clean splice (E15) produces no refused join.
- **E18** — Today's lookback (§7.1): a plain `status` for today
  (`DATE` omitted, or given explicitly) additionally lists every date
  in the last 14 days that still has an open start, oldest first, each
  as its own `[!]` line with `fix:` lines. `status DATE` for any other
  date runs no lookback; a date whose open start is 15 or more days
  old falls outside the window and isn't listed (§1.2a).
- **E19** — `mlm start`'s stale-start warning (§7.4): a successful
  `start`, today or backdated, prints one line per stale date found by
  the same 14-day lookback to stderr, oldest first; stdout stays
  silent and exit stays `0`. The date just written to draws no warning
  unless the new punch left it with two or more open starts.
```

## README.md edits

### Edit 9 — Known issues: drop four bullets, add the 14-day one

**Current** (lines 47-56, the first four bullets):

```
- Whether a stint reaching into the next day auto-resolves or gets
  left flagged depends on an internal rule the output doesn't explain
- `[!]` anomaly flags describe the problem but not how to fix it
- Whether a lone unclosed `start` gets flagged depends on whether a
  *second* one also exists that date — the difference is just how
  many, but it isn't explained anywhere
- A forgotten `stop` from several days ago is invisible everywhere
  except a `status` query against the exact date it started — no
  warning on today's `status`, on any date in between, or in `week`'s
  per-day table
```

**Replacement**:

```
- An unclosed start more than 14 days old only shows up on its own
  date's `status` and `week` row — today's `status` and a fresh
  `start` both stop mentioning it once it's older than that
```

Every bullet from "`status DATE` silently accepts a future date..."
(current line 57) onward is untouched.

### Edit 10 — `start` section: document the stderr warning

**Current** (lines 167-172, end of the `start` section, right before
the `### mlm stop|e ...` header):

```
**Footgun**: `--date`/`-d` must come *before* the `NOTE` text on the
command line. `NOTE` is a trailing variadic that swallows everything
after it, including a later `--date` flag — `mlm start 09:00 wrapped
up --date -1` silently records `--date -1` as part of the note text
instead of parsing it as the date flag.

### `mlm stop|e [TIME] [NOTE...] [-d/--date DATE]`
```

**Replacement** (new paragraph and example inserted between the
footgun paragraph and the next header):

```
**Footgun**: `--date`/`-d` must come *before* the `NOTE` text on the
command line. `NOTE` is a trailing variadic that swallows everything
after it, including a later `--date` flag — `mlm start 09:00 wrapped
up --date -1` silently records `--date -1` as part of the note text
instead of parsing it as the date flag.

After a successful write, `start` also checks the last 14 days for
stale open starts (an unclosed `start` 2 or more days old) and prints
one `warning:` line per date found to stderr, oldest first — stdout
and the exit code are unaffected either way. The date you just wrote
to is never included unless the new punch left it with two or more
open starts.

```sh
$ mlm start 09:00
warning: unclosed start on Mon 2026-09-07 at 09:00 (5 days ago); fix: mlm stop HH:MM --date 2026-09-07 (a time after 09:00)
```

### `mlm stop|e [TIME] [NOTE...] [-d/--date DATE]`
```

### Edit 11 — `status` section: `[!]`/`fix:` lines, the lookback, overnight-join paragraph

**Current** (lines 213-218, the end of the first example and the start
of the next paragraph):

```
Notes:
  - reviewed open PRs
  - fixed migration runner bug
```

The required-by-day pace hint and estimated-EOD line only show up when
`DATE` is today (they need "now" to mean anything). The pace-hint
```

**Replacement** (three new paragraphs inserted between the example and
the existing "required-by-day" paragraph):

```
Notes:
  - reviewed open PRs
  - fixed migration runner bug
```

When a date has a pairing problem, `status` prints one or more `[!]`
lines between the day/week lines and the stint list, each with a
`fix:` (and sometimes an `or:`) line underneath naming the exact
command to run — nothing is fixed automatically:

```
[!] unclosed start at 09:00, 5 days old (no stop recorded)
    fix: mlm stop HH:MM --date 2026-09-07  (a time after 09:00)
     or: mlm delete punch --date 2026-09-07  (lists punches; rerun with the start's number)
```

Today's plain `status` (no `DATE`, or today's own date given
explicitly) also checks the last 14 days and appends one such line for
every date it finds with an unclosed start, oldest first — so a
forgotten `stop` from last week still shows up today, not just when
you query that exact date.

An overnight session (a `start` before midnight, `stop` after) joins
into one stint on the earlier date automatically, as long as that date
has exactly one open start and the next date's very first punch is the
matching `end`. Anything messier — a second stray start or end that
day — leaves both punches alone: an ordinary open start on the first
date, and a flagged, unjoined orphaned end on the second, with a `[!]`
line on both dates explaining why they didn't join.

The required-by-day pace hint and estimated-EOD line only show up when
`DATE` is today (they need "now" to mean anything). The pace-hint
```

(the rest of the `status` section, from "clause after the day total"
onward, is untouched)

### Edit 12 — `week` section: document the footer

**Current** (lines 290-296, end of the first `week` example, right
before the `### mlm week|w target ...` header):

```
Carry-in:      00h 00m
Worked:        44h 35m
Fulfillment:   44h 35m
Target:        40h 00m
```

### `mlm week|w target [WEEK_ID] DURATION`
```

**Replacement**:

```
Carry-in:      00h 00m
Worked:        44h 35m
Fulfillment:   44h 35m
Target:        40h 00m
```

When any row in the table above carries `[!]`, one line follows after
a blank line:

```
[!] needs a look: run mlm status DATE for details and fixes
```

No footer line prints when nothing in the week is flagged.

### `mlm week|w target [WEEK_ID] DURATION`
```

### `--help` check (no edit expected)

The design spec and the changeset plan both say `--help` needs no
change. This task verifies that claim rather than assuming it: run
`cargo run -- --help` and `cargo run -- start --help` (and `stop`/
`status`/`week` for completeness) against the merged binary, diff
against the pre-changeset output (`git show main:` the same commands
aren't recorded anywhere as golden text, so the check is "read it and
confirm no anomaly-related wording appears," not a byte diff). If any
`--help` text did change, that's a Task 1-5 scope violation to flag
back, not something this task papers over by documenting the new text.

## Check plan (verify docs against the merged binary)

1. `cargo build` on the fully merged branch (all of Tasks 1-5 landed).
2. For each SPEC.md/README.md code block touched above, reproduce the
   real scenario against a scratch DB and diff the actual output
   against the quoted block, character for character, indentation
   included:
   - E7: two `start`s same day, no `stop` → `status` that date.
   - Stale open start: one `start` backdated 5+ days, `status` that
     date, and today's plain `status` (lookback line, same finding,
     different wording).
   - E8: one `stop` with no matching `start` → `status` that date.
   - Refused join: two opens on date A, one orphan-first on date A+1
     → `status A` and `status A+1`, confirm each names the other.
   - Lookback multi-open: two `start`s on a date 5-14 days back, no
     `stop` → today's plain `status`, confirm the "2 open stints on
     ..." line and its single `fix:` line.
   - `start` stderr warning: seed a stale date, run `mlm start` for
     today, capture stderr, confirm wording and that stdout is empty
     and exit is `0` (`echo $?`).
   - `week` footer: one week with a flagged row, one without; confirm
     the footer's presence/absence and exact text.
3. Confirm the fix-command round trip once: run the stale-start hint's
   `mlm stop HH:MM --date D`, then `status D` again, confirm the `[!]`
   line is gone. Run a `delete punch` hint similarly and confirm the
   listed number matches the actual offending punch.
4. Run `cargo test` and confirm the golden-output tests Task 2/3/5 add
   (grep for `stale`, `refused`, `lookback`, `warning` in test names)
   pass — these are the same strings this doc now quotes, so a passing
   suite is corroborating evidence, not a substitute for step 2's
   direct diff.
5. Run the `--help` commands listed above and confirm no anomaly
   wording leaked in (see the `--help` check note).
6. Re-read both changed files end to end for the user's doc-style
   rules (plain declaratives, no colon-hinged sentences, no verbless
   fragments) and for AGENTS.md's self-containment rule (no pointer to
   `docs/dev/plans/` or `docs/dev/specs/` for behavior detail — this
   plan's own drafted text was checked against that rule while
   writing it, but it's worth a second pass after the verbatim swap in
   step 2 potentially changes wording).
7. Confirm `docs/dev/specs/2026-09-26-open-punch-visibility.md` gets
   archived per AGENTS.md's archive convention once this doc's content
   is folded in — process step, not a behavior-detail check, and
   explicitly out of scope for this task's acceptance criteria, but
   worth a nudge to whoever runs the changeset's final wrap-up.

## Risks, ambiguities, and disagreements

- **Disagreement with the changeset plan's "fold in verbatim"
  instruction (§1.2a edit).** The design spec's §1.2 first bullet
  ends "...Replaces the current §1.2a forgotten-`stop` bullet." That
  clause narrates the changeset's own edit to the document — it refers
  to "the current" bullet, which won't exist once this edit lands, and
  it's exactly the kind of changelog-in-the-document AGENTS.md's
  documentation section and the project's "no rejected alternatives in
  docs" convention both rule out for a self-contained spec. I dropped
  that clause in Edit 1's replacement text and keep only the
  substantive claim (the 14-day visibility gap). If a reviewer wants
  literal verbatim-with-clause instead, that's a one-line add-back, but
  I think dropping it is the correct call under AGENTS.md, not just a
  style preference.
- **`§7.3` cross-references, not design-spec section numbers.** The
  locked design spec uses its own §2/§3.x numbering (e.g. "stale open
  start" is its §2/§3.1). None of that numbering exists in SPEC.md.
  Every replacement above cross-references SPEC.md's own existing
  section numbers (§4.3, §4.3.1, §7.1, §7.3, §7.4, §1.2a) instead,
  consistent with how SPEC.md already cross-references itself
  elsewhere. Worth a second reviewer's eyes since it's easy to
  transpose a design-spec section number into SPEC.md by habit while
  actually implementing this.
- **`week`'s footer fenced-code-block escaping in this plan.** Because
  this plan document is itself Markdown with fenced code blocks
  containing other fenced code blocks (SPEC.md examples), I used
  escaped backticks (`\`\`\``) in two spots (Edits 5 and 6) to keep the
  outer/inner fences from colliding. The implementer writes a literal
  triple-backtick fence in the real file — this is a plan-authoring
  artifact only, not something to carry into `SPEC.md`/`README.md`.
- **Exact whitespace/indentation of `fix:`/`or:` lines is unverified
  against real code.** I copied the design spec's examples byte-for-
  byte (4-space indent under `[!] `, per §3.4's own statement of the
  rule), but Task 2 is the actual source of truth. Every code block in
  Edits 6, 7, 11, and 12 is `[VERIFY AT MERGE]` — don't hand-merge this
  plan's text without re-diffing against a real `cargo run`.
- **README's overnight-join paragraph placement is a judgment call.**
  The changeset plan just says "a short plain-language paragraph...",
  no fixed location. I placed it in the `status` section, right after
  the lookback explanation, since that's where a reader first
  encounters the refused-join `[!]` line and would want the plain-
  language "why" nearby. An equally defensible placement is right
  after `start`/`stop`'s own sections, or as part of Known issues. Not
  a correctness issue, just worth a second opinion.
- **No sibling task plans existed at drafting time.** I could not
  cross-check this plan's assumptions (e.g. the exact field names for
  a lookback finding, or whether `stint.rs`'s predicates end up named
  differently than the changeset plan describes) against Tasks 1/2's
  actual implementations, since neither had landed or even had a task
  plan yet. Nothing above depends on internal naming — every edit only
  depends on final rendered text and documented behavior, both of
  which the locked design spec already pins — but the check plan's
  step 2 is where any drift would surface, and it must not be skipped.
