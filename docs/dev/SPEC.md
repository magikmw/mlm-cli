# mlm — specification

Formal spec, built incrementally from `NOTES.md` decisions. Sections
land one at a time for review — this doc grows over several passes,
not all at once.

Status: **draft, complete through section 8, post-adversarial-review
fixes applied**.

## 1. Overview

`mlm` is a CLI time tracker. It replaces a plain-text daily log
(wall-clock stint times, a work-log note, day/week sums, and a
week-over-week hours-owed calculation) with a tool that does the
arithmetic and lets entries land in any order.

### 1.1 Goals

- Record start/end time punches for the current date, entered in any
  order.
- Record short free-text work-log notes for the current date,
  independent of punches.
- Show a status view for today: today's stints (derived from punches),
  today's notes, today's total, and this week's total so far.
- Show a status view for a full week: same shape, week-scoped.
- Track a per-week target, fixed at 40h by default and overridable to
  an absolute value per week, and report time still owed against it.
- Carry a week's variance into the next week as a signed adjustment to
  that week's *fulfillment* (worked-time sum), not to its target —
  like an extra virtual day that itself worked a positive or negative
  number of minutes. A deficit week can start already behind; a
  surplus week can start already ahead.
- Persist everything in SQLite, normalized, in the platform app-data
  directory, with schema migrations from the first release.

### 1.2 Non-goals (deferred/stretch, see `NOTES.md`)

Deliberately out of scope: future features not committed to, not
things known to be broken or confusing today. See §1.2a for that.

- Editing a punch/note after entry (deleting is implemented — `mlm
  delete note|punch`; the correction path is delete-then-recreate, not
  in-place edit).
- Project tagging on notes/stints.
- Terminal dashboard (ratatui) — deps are in, UI is not.
- Shell prompt integration.
- Non-ISO week conventions.
- 12h (AM/PM) time input — accepted input is `HH:MM`/`HHMM`/`HH`, 24h only.
- `+N`/`-N` relative week notation for `week`'s week-id argument. (This
  is distinct from `status`'s `DATE` argument and `start`/`stop`/
  `note`'s `--date` flag, both of which already accept a `-N` shorthand
  — §3.4a, §3.5.)
- Other `mlm export` formats, such as JSON or CSV, and an `--output`
  file. `mlm export notes` prints plain text to stdout, and the user
  redirects it (§3.8).

Stints spanning midnight, and a same-instant `end`/`start` boundary
between two real stints, were both non-goals through this point in the
project's history — both are now fixed; see §4.3.

### 1.2a Possible issues to revisit

Shipped, working-as-designed behavior that's rough or confusing, not
committed to a fix. Kept separate from §1.2: a non-goal isn't built
yet, a possible issue works but has a rough edge.

- Entering a stop before its start warns once. `mlm stop 17:00` then
  `mlm start 09:00` is legal on any date (§3.2), but the first command
  prints an orphan warning (§7.4). The second command clears it.

### 1.3 Terminology

- **Punch**: a single timestamped event, either `start` or `end`, on a
  given date.
- **Stint**: a derived (start, end) time range, computed by pairing a
  day's punches like matched parentheses (nearest-match, §4.3) — not
  stored directly.
- **Open stint**: a stint whose punch pairing leaves a trailing,
  unmatched `start` (i.e. still ongoing). Duration is computed live
  against the current time when displayed.
- **Note**: a short free-text work-log entry for a date, independent
  of punches.
- **Week**: identified by an (ISO year, ISO week number) tuple, e.g.
  `2026-07`. Always Monday-start.
- **Target**: the number of minutes a week is expected to reach.
  Fixed at 40h by default; can be overridden per week to an absolute
  value. Not affected by carry (see **Fulfillment**).
- **Fulfillment**: a week's worked-time sum plus that week's
  **carry-in** (a signed minute adjustment from the prior week's
  variance against target — negative if the prior week fell short,
  positive if it exceeded target). Owed = target − fulfillment.
  **Carry-out** to the next week = fulfillment − target at week's end.

## 2. Data model

### 2.1 Time zone

All stored instants are **UTC**. The user's local (system) timezone is
used only at the edges: converting typed input to UTC on write, and
converting back to local for display. This means a punch's *calendar
date* is a local-time concept, not derivable from the UTC instant by
plain string slicing — so it's stored as its own app-computed column
rather than a SQL-generated one (see `punches.date` below).

Every conversion is **per-instant**, using whatever UTC offset applied
*at that instant's own date*, not a single offset grabbed once (e.g.
"now"'s offset) and reused. This matters concretely across a DST
transition: a punch made before the transition and one made after both
convert correctly using their own moment's offset, rather than one of
them silently displaying an hour off. `chrono`'s timezone-aware
conversion (via the system tz database) does this correctly as long as
every conversion call passes the specific instant, not a cached offset.

Two DST edge cases this raises, both resolved explicitly rather than
left to whatever the conversion library happens to do by default:

- **Spring-forward gap**: a typed local `TIME` that doesn't correspond
  to any real local instant on today's date (the hour skipped when
  clocks jump forward) is a hard error (§6.1) — nothing written, same
  treatment as the `24:00` boundary case.
- **Fall-back ambiguity**: a typed local `TIME` that occurs *twice*
  (the repeated hour when clocks fall back) resolves to the **earlier**
  of the two real instants.

### 2.2 Migrations

A `schema_migrations` table (or equivalent) tracks applied versions.
Uses the [`rusqlite_migration`](https://docs.rs/rusqlite_migration)
crate — ordered SQL migrations embedded in the binary, applies
whatever's pending on `connect()` — instead of a hand-rolled runner.

Single-user, single-machine tool — concurrent access from two `mlm`
invocations at once is out of scope; SQLite's default locking is
trusted to fail safely (as a §6.1 DB error) rather than corrupt
anything, but no explicit WAL/busy-timeout tuning is planned.

### 2.3 Tables

**`punches`** — one row per timestamped start/end event.

| column | type | notes |
|---|---|---|
| `id` | `INTEGER PRIMARY KEY AUTOINCREMENT` | surrogate id, see NOTES.md decision 9 |
| `at_utc` | `TEXT NOT NULL` | UTC instant, RFC 3339 (`2026-09-12T13:05:00Z`), sortable lexically |
| `date` | `TEXT NOT NULL` | local calendar date, `YYYY-MM-DD`, computed app-side at insert from `at_utc` + local timezone — not a generated column (see §2.1) |
| `kind` | `TEXT NOT NULL CHECK (kind IN ('start', 'end'))` | |

Index on `date` (and probably `at_utc` for ordering within a date).

**`notes`** — one row per work-log entry.

| column | type | notes |
|---|---|---|
| `id` | `INTEGER PRIMARY KEY AUTOINCREMENT` | |
| `date` | `TEXT NOT NULL` | local calendar date, `YYYY-MM-DD` — a note is attached to a day, not an instant, so no `at_utc` here |
| `body` | `TEXT NOT NULL` | free text, trimmed of leading/trailing whitespace before storage (§6.1); embedded `\r`/`\n` are collapsed to a single space rather than preserved verbatim, so a stored body is always exactly one line (this collapse never affects whether a body counts as empty — that check happens first, against the pre-normalization text, and rejects a whitespace-only body, including one that's only newlines, before either step touches it, §6.1); project-name prefix stays *in* the text (no `project` column — that's the deferred stretch, adding it later is a plain migration); no length cap or charset restriction beyond that newline collapse — the plain-ASCII rule in §7 is about layout characters in *rendered* output, not what a user can type into a note |
| `created_at_utc` | `TEXT NOT NULL` | minute-granular like every other stored instant (§4.1) — not a source of sub-minute precision. Two notes inserted in the same minute are disambiguated by `id ASC` as the actual tiebreaker; this column orders coarsely, `id` breaks remaining ties |

**`week_targets`** — sparse overrides only; a week with no row uses
the default target (40h = 2400 minutes).

| column | type | notes |
|---|---|---|
| `week_id` | `TEXT PRIMARY KEY` | `YYYY-WW`, ISO year + zero-padded ISO week, no `W` (e.g. `2026-07`) |
| `target_minutes` | `INTEGER NOT NULL CHECK (target_minutes >= 0)` | absolute override; `0` is legal (a deliberate week off — an explicit "nothing owed this week" is meaningfully different from silently letting a normal week's deficit accrue) |

### 2.4 Derived, not stored

- **Stints**: computed at read time from a date's punches via the
  LIFO nearest-match algorithm in §4.3, not stored.
- **Fulfillment / carry**: computed at read time, not materialized —
  walk **every** ISO week in sequence, starting from the earliest week
  with any punch/note data, up through the requested week, folding
  each week's `(worked_minutes + carry_in) - target` into the next
  week's `carry_in`. This must include weeks with zero recorded time
  in between — a fully idle week (sick, vacation, simply not tracked)
  still accrues a deficit against its target and has to carry forward
  like any other week; skipping zero-data weeks would silently break
  the chain across any gap. Cost scales with total week count,
  negligible at personal-use scale. `worked_minutes` is the sum of
  completed stints. For a past or future week that is all of it. For
  the **current week only**, the running stint's live minutes (§7.1)
  join `worked_minutes` before carry is walked, so fulfillment and
  worked count the running stint, and a future week's `carry_in`
  reflects that live-inclusive fulfillment. Past weeks never contain
  live minutes, so their carry is unchanged. Nothing live-inclusive is
  stored. Live minutes come from now, so every figure that includes
  them changes minute by minute while a stint runs: the day total,
  `Worked`, `Fulfillment`, the week line, and a future week's
  `Carry-in`. A plain `stop` at the current time does not move the week
  line, because those minutes already counted. A backdated `stop HH:MM`
  raises the `left to` figure by the minutes it cuts off, and a
  backdated `start HH:MM` lowers it by the minutes it adds. A stint open
  across midnight gets no live credit (§7.1). `est. EOD` is computed from fulfillment without live and
  stays display-only. An
  **orphaned `end`** (§4.3) contributes nothing to any total either —
  it has no paired `start` to derive a duration from, so it shows up
  only as its own flagged anomaly line, never as time in a day/week
  sum. Every storage read behind this walk (the earliest data week, the
  target overrides, the punches) is a hard error when it fails (§6.1).
  No read falls back to a default target or to "no earlier weeks".
- **Daily target**: `today's week target ÷ 5` (floor to the minute).
  Purely derived from the week's target — no override, no storage.
  Used only as the per-weekday increment for **required-by-day**
  below; never displayed or compared on its own.
- **Required-by-day**: the slice of the target due by end of today,
  by today's weekday. Monday through Thursday it is `daily target ×
  weekday number` (Mon=1 … Thu=4, same numbering as the ISO week
  machinery already in use elsewhere, e.g. `week.rs`). Friday through
  Sunday it is the week's full target, because the week has no more
  workdays to spread the target over. That exact target can sit up to 4
  minutes above `5 × daily target` when a target override is not a
  multiple of 5 minutes. `required` always uses today's weekday, even
  when `status DATE` shows another date, and it is computed for every
  `status` of the current week. Compared against the week's
  **fulfillment** (`carry_in + worked`, live-inclusive for the current
  week, §2.4 above), it gives `gap = required − fulfillment`, the
  figure the current week's headline shows in both `status` and `mlm
  week` (§7.1, §7.2). Carry-in counts here, measured against a smaller,
  day-scoped slice of the target instead of the whole week. The same
  `required` feeds the estimated-EOD figure (§7.1).

## 3. CLI surface

All commands operate on the local machine's current date/time unless
a command explicitly takes a date/week argument. Typed times/dates are
interpreted in local time and converted to UTC on write (§2.1); output
converts back to local.

The global `-v`/`--verbose` flag turns on debug log lines on stderr.
Without it only warnings and errors are logged, so a run that creates
and migrates a new database prints nothing. The one exception is a
first `mlm stop`, which warns about its orphaned end (§7.4).

### 3.1 Time input format

Accepted for any `TIME` argument, 24h only (§1.2):

| form | example | meaning |
|---|---|---|
| `HH:MM` | `9:05`, `17:30` | hour and minute |
| `HHMM` | `0905`, `1730` | hour and minute, no separator |
| `HH` | `9`, `17` | hour, minute defaults to `:00` |

### 3.2 `mlm start [TIME] [NOTE] [--date DATE]`

Insert a `start` punch, for today unless `--date` says otherwise
(§3.4a).

- `TIME` optional (§3.1). Defaults to now.
- `NOTE` optional, free text — if given, also inserts a note row for
  the target date in the same call (convenience for "starting work on
  X").
- Positional order is strict and never sniffed: `TIME`, when given, is
  always the first positional argument. A value in that position that
  fails to parse as `TIME` is a hard error (§6.1, E1) — it is never
  silently reinterpreted as `NOTE` text. A note-only invocation with
  no `TIME` is `mlm note` (§3.4), not a single positional guessed at.
- No chronology requirement: a start punch can be inserted at any
  time value relative to existing punches for the day.

### 3.3 `mlm stop [TIME] [NOTE] [--date DATE]`

Same shape as `start`, inserts an `end` punch. After the write it warns
on stderr about each orphaned `end` on the date it wrote to (§7.4).

### 3.4 `mlm note NOTE [--date DATE]`

Insert a work-log note, for today unless `--date` says otherwise
(§3.4a), independent of punches — for end-of-day or next-day notes
with no punch attached.

### 3.4a Backdating with `--date`/`-d`

`start`, `stop`, and `note` all take an optional `--date`/`-d DATE`
flag targeting a date other than today. Same `DATE` grammar as
`status` (§3.5): an absolute `YYYY-MM-DD`, or `-N` for N days before
today (`-1` = yesterday, N a positive integer, `-0` rejected).

- `--date`/`-d` must come before `NOTE` text on the command line —
  after `NOTE` starts, everything remaining is note text, so a
  trailing `--date` is absorbed into the note body instead of parsed
  as the flag.
- Malformed `--date`, or one resolving to a future date: hard error
  (§6.1).

### 3.5 `mlm status [DATE]`

Today's (or `DATE`'s, `YYYY-MM-DD`) view. `DATE` must not be after today
(§6.1). `mlm week WEEK_ID` is the way to look at a future week (§7.2).
The future-date error says so: `invalid DATE "2099-01-01": date is in the
future; to view a future week, run mlm week WEEK_ID`.

- that date's stints (derived), each as `start–end (duration)`, with
  an open stint shown as `start–now (duration, ongoing)`
- that date's notes, in insertion order
- date total
- **the week containing `DATE`**'s fulfillment so far, target, and
  headline figure — for the current week the point-in-time gap
  (`required − fulfillment`, §2.4, §7.1), for any other week the plain
  total behind or ahead. Not always "this week": a `status` for a date
  two weeks ago shows *that* week's numbers next to it, not today's.
  (Querying a past date's own stints next to today's unrelated week
  totals was the original, confusing behavior — fixed here.)
- the estimated-EOD (§7.1) appears only when `DATE` is actually today
  — it depends on the running stint and "now," which are meaningless
  for any other date. The current week's week line prints whether or
  not `DATE` is today, and always describes now.
- any pairing anomalies for that date (§4.3), if present, including
  overlapping stints
- for today only, lookback lines for open starts from earlier dates, an
  `[i]` hint for a start still open from yesterday, and a stint line for
  that start ahead of today's own stints (§7.1)
- a carry-in marker line when an earlier week has an anomaly (§7.1)

`mlm status --help` and `mlm week --help` end with a short glossary of
the terms the week line and the markers use. Short `-h` does not carry
it, and its help line gains clap's `(see more with '--help')` suffix.
Three entries precede the `[!], [i]` one. The first is `so far, started
yesterday`, for the overnight stint line (§7.1): a start still open
from yesterday, listed on today's page and not counted until stopped.
The second is `spans to next day, continues previous day's stint`, for
the §4.3.1 suffixes: a stint stopped after midnight is counted on the
day it started, `spans to next day` ends the start date's stint line
and `continues previous day's stint` ends the next date's header. The
third is `- <note>`, for one of a date's notes in insertion order.
`mlm status` lists them under `Notes:` (§7.1), and `mlm week --notes`
lists them under the date's row (§7.2). Both commands share the
glossary, so `mlm status --help` lists it too. The `[i]` entry, last,
ends ``It marks yesterday's row in `mlm week` until that start is stopped.``

### 3.6 `mlm week [WEEK_ID] [--notes|-n]`

Current (or `WEEK_ID`) week's view:

- the week's headline figure (§7.1/§7.2). For the current week it is
  the point-in-time gap, the same line `status` shows. For any other
  week it is the plain total behind or ahead.
- one line per date in the week with that date's total
- yesterday's row ends with `[i]` while a start from yesterday is still
  open and unjoined, with an `[i]` footer line, and on a Monday the
  current week's view names Sunday's stint in that footer line (§7.2)
- with `--notes`/`-n`, each date's notes on the lines directly under
  its row (§7.2)
- a trailing block, in this order: carry-in, worked, fulfillment,
  target (§7.2) — the headline figure appears only in the headline, not
  repeated here

`WEEK_ID` accepts either a full id (`YYYY-WW`, e.g. `2026-07`) or a
bare week number (`WW`, e.g. `7`), which defaults the year to the
current one.

`--notes`/`-n` works with or without `WEEK_ID`, in either order (`mlm
week -n 43`, `mlm week 43 -n`). `mlm week --help` lists it as `Show
each date's notes under its row`. The flag belongs to the view only,
so `week target` (§3.7) does not take it. After `target`, `--notes`
and `-n` are ordinary tokens in target's own argument list. `mlm week
target 40h --notes` fails as an invalid `WEEK_ID` or `DURATION` with
exit `1`, and `mlm week target 43 40h --notes` is a usage error with
exit `2`. Neither writes a target.

### 3.7 `mlm week target [WEEK_ID] DURATION`

Set an absolute target override for a week. `WEEK_ID` (same accepted
forms as §3.6) defaults to the current week when omitted. `DURATION`
uses the human duration format (§4.2), not raw minutes — e.g. `20h`,
`33h30m` — so setting a target has the same friction as everything
else here: no mental-math input.

On success it prints one line to stdout, with the week id it wrote to
(the current week when omitted) and the stored target in the standard
duration format (§4.2):

```
Week 2026-41 target: 20h 00m
```

`note` stays silent. `start` and `stop` print nothing to stdout, and
print only warnings to stderr (§7.4).

`WEEK_ID` goes after `target`. `mlm week WEEK_ID target DURATION` fails
with one line on stderr and exit `2`, the same code as parser errors:

```
error: put WEEK_ID after target: mlm week target 2026-41 20h
```

The line echoes the week id and duration as typed, always spells `mlm
week target` even when the user typed the `w` alias, and prints no
`Usage:` block. The success echo above uses the normalized week id, so
`mlm week target 2026-7 33h30m` prints `Week 2026-07 target: 33h 30m`.
`mlm week target --help` shows the usage `mlm week target [OPTIONS]
[WEEK_ID] <DURATION>` and an Arguments section naming both arguments. No
parser error for this subcommand mentions `<ARGS>`.

### 3.8 `mlm export notes [--date DATE | --week WEEK_ID | --month MONTH]`

Print the notes of one date, one ISO week or one calendar month to
stdout as plain text, one line per note (§7.5), for pasting into a
report or piping into another tool. `export` has the alias `ex` and
`notes` the alias `n`, so `mlm ex n -w 43` works. `export` requires a
subcommand, and `notes` is the only one.

- no range flag: the current ISO week, Monday to Sunday
- `--date`/`-d DATE`: one date, in the `DATE` forms of §3.4a
  (`YYYY-MM-DD`, or `-N` for N days before today), with no future-date
  check
- `--week`/`-w WEEK_ID`: Monday to Sunday of one ISO week, in the
  `WEEK_ID` forms of §3.6, with a bare `WW` in the current ISO year
- `--month`/`-m MONTH`: the first to the last calendar day of one
  month. `MONTH` is `YYYY-MM` (e.g. `2026-03`), `YYYY-M` (`2026-3`, the
  same month) or a bare `M`/`MM` (e.g. `3`) in the current calendar
  year. The calendar year is not the ISO year, so on 2025-12-30 `-m 1`
  is `2025-01`. The year is exactly four digits, and any value from
  `0000` to `9999` is accepted. February runs to the 28th or the 29th
  by the calendar.

At most one range flag is given. The range covers every date from its
first to its last day, inclusive. A note belongs to its stored `date`,
so punches play no part. The command makes one storage read, the
ranged note read over the range, and touches no punch, total or
target.

All three flags accept values that start with `-`. So `-d -1` is
yesterday, and `-w -1` and `-m -1` reach their own `WEEK_ID` and
`MONTH` errors with exit `1` (§6.1) instead of a parser error. A flag
followed by another flag takes that flag as its value, so `-m -d` fails
as a bad `MONTH`.

A future date, week or month is not an error. It has no notes, so the
command prints nothing and exits `0` (§6.2).

These are usage errors from the argument parser, with exit `2` (§6.3):

- bare `mlm export`, whose output lists `notes`
- two or more range flags
- one range flag given twice
- a range flag with no value as the last token

`mlm --help` lists `export` as `Print stored data as plain text`, and
`mlm export --help` lists `notes`. `mlm export notes --help` shows:

- command: `Print notes for a date, week or month, one per line (defaults to the current week)`
- `-d, --date <DATE>`: ``Date to export: YYYY-MM-DD, or `-N` for N days before today (e.g. `-1` = yesterday)``
- `-w, --week <WEEK_ID>`: `Week to export: YYYY-WW (e.g. 2026-07) or a bare week number (e.g. 7)`
- `-m, --month <MONTH>`: `Month to export: YYYY-MM (e.g. 2026-03) or a bare month number (e.g. 3)`

## 4. Time/duration semantics

### 4.1 Parsing and conversion

A `TIME` argument (§3.1) is parsed as a local wall-clock hour/minute,
combined with the target date (local *today* by default) to form a
local datetime, then converted to UTC for storage per-instant (§2.1).
There is no seconds precision anywhere —
everything is minute-granular, per NOTES.md's "keep time in minutes"
call.

### 4.2 Duration formatting

One display rule everywhere, tabular or standalone alike: `HHh MMm`,
both sides zero-padded, hour part never dropped even at zero (`07h
45m`, `00h 20m`) — this is what keeps a column of these lined up, and
there's no reason for status output to look different from the week
table right next to it. A negative value (a surplus `carry_out`, a
negative `Carry-in:` or `fulfillment`) keeps the same padding with the
sign in front:
`-00h 50m`, `-03h 20m`.

`DURATION` arguments (§3.7) accept a looser *input* grammar than the
display format — no padding, and hours-only or minutes-only are both
fine on their own:

| form | example | meaning |
|---|---|---|
| `Hh` | `20h` | whole hours |
| `HhMMm` | `33h30m` | hours and minutes |
| `MMm` | `45m` | minutes only, no hour part needed |

Parsed input converts straight to an integer minute count for storage
(`week_targets.target_minutes`, §2.3) — the same internal unit as
everywhere else, just never exposed raw at either the input or output
edge.

### 4.3 Stint pairing

Genuine nearest-match (LIFO) parentheses matching, per NOTES.md
decision 4 — not a plain "sort then pair sequentially by index",
since that only agrees with nearest-matching on already-alternating
data and gives wrong answers otherwise. Algorithm:

1. Sort the date's punches by `at_utc`; ties at an identical instant
   are broken first by kind (`start` before `end`), then by `id`
   (insertion order). The kind tiebreak matters for E14 below — pure
   `id`-order would let a `stop` entered before a same-instant `start`
   produce an orphan and a dangling open stint instead of the clean
   zero-length pairing E14 requires.
2. Scan the sorted punches **one same-instant group at a time**, not
   as one flat pass: within a group sharing an `at_utc`, every `end`
   in the group first tries to pop the stack as carried in from
   *strictly earlier* groups (closing a `start` that was already
   open, before any of this group's own `start`s exist on the stack);
   only then does the group's `start`s get pushed, and only then do
   any `end`s left over from the first step (nothing was open before
   them) pop against those same-group `start`s — producing the E14
   zero-length pair, or an ordinary orphaned `end` if the group has
   more `end`s than `start`s to pair against. This is what lets a
   genuine boundary between two real stints — `stop 09:00` then
   `start 09:00` back to back, no gap — close the *preceding* open
   stint correctly instead of zero-pairing the tied instant and
   silently dropping that stint's time (every group of size 1, i.e.
   no tie at all, reduces to the plain LIFO scan below unchanged).
3. Outside a tied group, an `end` simply pops the *most recently
   pushed* unmatched `start` and pairs with it, forming a stint.

Worked example (matches the motivating case): starts entered at
`09:00` and `14:00`, ends entered at `18:00` and `13:00`, in that
entry order. Sorted by time: `09:00(start), 13:00(end), 14:00(start),
18:00(end)`. Scanning: `09:00` pushes; `13:00` pops it → stint
`09:00–13:00`; `14:00` pushes; `18:00` pops it → stint `14:00–18:00`.
Entry order never mattered, only time order.

This also correctly handles legitimately nested entry (e.g. two
`start`s before either `end`: the second `start` pairs with the
*first* subsequent `end`, nearest first) without treating the pairing as
malformed. The nested stints overlap, though, and an overlap is flagged
(below) because the overlapping minutes count twice.

Edge cases, still just **detected and surfaced in that date's
summary** (§3.5/§3.6 output), not silently fixed and not rejected at
insert time (no editing/validation — see §1.2). `start` and `stop`
print stderr warnings at insert time (§7.4), and the punch is stored
either way. The edge cases are:

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
- **An `end` with an empty stack** (no unmatched `start` to pop):
  unpaired, the summary flags it as an orphaned `end` rather than
  silently dropping it or guessing which `start` it belonged to. Two
  or more orphaned `end`s on the same date each get their own flagged
  line (§7.3) — never coalesced into one count.
- **A `start` and its paired `end` at the identical instant**: legal,
  produces a zero-length stint (`00h 00m`), not itself an anomaly —
  this is exactly why step 1's tie-break sorts `start` before `end`
  at a shared instant, regardless of entry order, so the pair always
  matches cleanly rather than depending on which was typed first.
- **Overlapping stints**: two stints on one date overlap when each
  starts before the other ends (`a.start < b.end` and `b.start <
  a.end`) and both have nonzero length. Identical ranges and nests that
  share one endpoint overlap. Stints that only touch at an endpoint do
  not. The check covers the stints the date's page shows. These are
  closed stints and, for today only, the most recent open stint taken up
  to now. A past date's open stint counts zero minutes and is left out. A
  spliced cross-midnight stint (§4.3.1) takes part on each date with the
  part that falls on that date. Each overlapping pair is flagged (§7.3).
  Pairing and totals do not change, so the overlapping minutes still
  count twice in the day total.

### 4.3.1 Boundary splice (cross-midnight)

Pairing above is scoped to one calendar date, by design — the
algorithm never looks past the literal adjacent date. For two literal
adjacent calendar dates `A` and `A+1`: if `A`'s own
classification leaves exactly one trailing open `start`, and `A+1`'s
own classification has exactly one orphaned `end` that is also `A+1`'s
chronologically first punch of the date (sorted by the same
`(at_utc, kind, id)` order step 1 above uses for tie-breaks — literally
punch index 0 for that date, not merely "the date's only orphan"),
they're spliced into one completed stint — `A`'s `start` paired with
`A+1`'s `end`, its minutes landing on `A` (the day the stint started),
not `A+1`. Neither date shows an anomaly for it once spliced.

`classify()` itself stays the pure, single-date primitive. A second
function implements the splice:

```rust
pub fn classify_at(
    prev_punches: &[Punch],
    punches: &[Punch],
    next_punches: &[Punch],
    now: DateTime<Utc>,
) -> DayStints
```

Classifies `punches` (one calendar date) same as `classify()`, then
resolves an unambiguous midnight-spanning stint against its immediate
neighbors per the rule above. `prev_punches`/`next_punches` are the
literal adjacent calendar dates' punches (pass `&[]` when a neighbor
has no data — an empty slice already classifies correctly as "nothing
open, nothing orphaned").

Anything short of that exact 1:1, first-punch shape is left completely
alone, rendered exactly as an ordinary open stint / orphaned end today
— never guessed at, never partially resolved. In particular: **a
genuinely carried-over orphan sharing its date with one unrelated
stray orphan elsewhere that day still doesn't splice** (the
`orphaned_ends.len() == 1` gate fails), so this section's fix isn't an
unconditional guarantee against every possible day's data, only the
ordinary case. This residual case is the known rough edge here. When
the gate rejects a pair — the earlier date has an open start and the
later date an orphaned end, but not in the exact 1:1, first-punch
shape above — both dates now flag a refused-join anomaly naming each
other (§7.3), so the rejection is no longer silent.

A spliced stint is not indistinguishable from an ordinary same-date
one: the earlier date's stint line gets a `, spans to next day` suffix
(§7.1), and the receiving date's header gets a `(HH:MM continues
previous day's stint)` suffix (§7.1).

## 5. Week accounting — worked example

Table values are raw minutes for arithmetic clarity; on screen these
render in the uniform duration format (§4.2) — e.g. `2026-01`'s `owed`
of `200` displays as `03h 20m`, `2026-04`'s `owed` of `-50` as `-00h
50m`. `fulfillment = worked + carry_in`; `owed = target -
fulfillment`; `carry_out = fulfillment - target` (becomes next week's
`carry_in`). First tracked week starts with `carry_in = 0`.

| week | target | carry_in | worked | fulfillment | owed | carry_out |
|---|---|---|---|---|---|---|
| `2026-01` | 2400 | 0 | 2200 | 2200 | 200 | −200 |
| `2026-02` | 2400 | −200 | 2500 | 2300 | 100 | −100 |
| `2026-03`* | 2000 | −100 | 1950 | 1850 | 150 | −150 |
| `2026-04` | 2400 | −150 | 2600 | 2450 | −50 | 50 |

\* `2026-03`'s target was overridden to 2000 (`mlm week target 2026-03
33h20m`) — e.g. a half day planned off.

Reading `2026-04`: despite the incoming deficit, working 2600 (over
the 2400 target on its own) both clears the debt and ends the week
**ahead** — `owed` goes negative (already met target with 50 minutes
to spare), and that surplus becomes `2026-05`'s `carry_in`, lowering
its effective target rather than paying out as overtime now (§1.1).

Nothing is capped or clamped anywhere in this table; every value can
legitimately go negative or exceed target in either direction.

`owed` drives carry and the closed-week `Total behind` / `Total ahead`
headline. The current, still-open week does not print `owed`. Its
headline shows `gap = required − fulfillment` (§2.4, §7.1), measured
against the slice of the target due by end of today instead of the
whole week's target. `owed` and `gap` share the fulfillment number but
not the target they are measured against, so the current week's `gap`
can read `over` while `owed` is still positive, e.g. comfortably under
the week target while already past today's slice. `owed` is only truly
"due" at week's end, while `gap` answers the original manual process's
"how much I should still put in today" (NOTES.md). Both are correct
answers to different questions, not a contradiction. Once the week
closes, the headline switches to `owed`.

## 6. Error handling & validation

Two tiers, kept distinct on purpose: **hard errors** reject the
command outright (nonzero exit, nothing written, message on stderr);
**anomalies** (§4.3) are accepted, stored, and surfaced later in
`status`/`week` output instead — because there's no in-place editing
(§1.2), refusing to store a punch that merely produces a weird pairing
would leave the user with no way to fix it. `stop` also surfaces an
orphaned `end` right away, as a stderr warning after the write (§7.4),
and still exits `0`.

### 6.1 Hard errors (reject, no write)

- Malformed `TIME` (§3.1): doesn't match `HH:MM`/`HHMM`/`HH`, or an
  out-of-range hour/minute (`25:00`, `9:75`).
- Malformed `DATE` (`YYYY-MM-DD`, or a `-N` shorthand where accepted):
  wrong shape, an invalid calendar date (`2026-02-30`), or a malformed
  `-N` (non-integer, `-0`).
- `--date`/`-d` on `start`/`stop`/`note` resolving to a date later than
  today (§3.4a), or `status`'s `DATE` argument (§3.5) resolving to one.
  The first three never write into the future, and `status` is no
  preview of a future date. `status DATE` uses the same message shape and
  exit code as the `--date` case, and adds `; to view a future week, run
  mlm week WEEK_ID` to it. `mlm week WEEK_ID` for a future week is
  not an error (§7.2). `mlm export notes` checks no range against today,
  so its `--date`, `--week` or `--month` may name a future range
  (§3.8, §6.2).
- Malformed `WEEK_ID` (§3.6): not a bare week number or `YYYY-WW`
  shape, or a week number that isn't a valid ISO week for its year
  (most years have 52, some have 53 — `2027-53` is invalid if 2027
  only has 52; this is a calendar-validity check, not a flat
  `1..=53` range test). An unpadded number is accepted and normalized
  (`2026-7` parses the same as `2026-07`), consistent with how
  `TIME`/`DURATION` input is lenient about padding elsewhere.
- Malformed `MONTH` (§3.8). A value that is not `YYYY-MM`, `YYYY-M` or
  a bare `M`/`MM` prints this:

  `invalid MONTH "<input>": expected YYYY-MM or MM`

  A month number of `0` or above `12` prints this:

  `invalid MONTH "<input>": month must be 1-12`

  `<input>` is the value as typed. The year is exactly four digits, and
  any four-digit year is accepted.
- Malformed `DURATION` (§3.7/§4.2): doesn't match the input grammar,
  or parses to a negative number of minutes. Zero is legal (a
  deliberate week off, see §2.3's `week_targets` note) — only
  negative is rejected.
- Malformed `TIME` boundary: `24:00` is rejected, not accepted as a
  next-day-midnight alias — valid range is `00:00` through `23:59`.
- A `TIME` that falls in a DST spring-forward gap (§2.1) — no real
  local instant exists for it on today's date.
- Empty `NOTE`/note `body`: whitespace-only text is rejected rather
  than stored as a blank log line (checked *before* the trim in §2.3
  — a note that's nothing but whitespace has nothing left to trim to).
- Database open/migration failure (missing permissions, corrupt file,
  a migration erroring partway): fatal, the command aborts — there's
  no reasonable partial-success state to fall back to. This is
  distinct from first run: a missing app-data directory or db file is
  expected and handled silently (`db.rs` already creates the
  directory and runs migrations from empty — see `src/db.rs` in the
  scaffold), not a failure case.
- A storage read failure in `status`, `mlm week` or `mlm export notes`,
  on any of the reads they make (the earliest data week, the target
  overrides, the punches behind the carry walk, for `mlm week --notes`
  the shown week's notes, and for `mlm export notes` its one ranged
  note read). The command prints `error:` and the storage error text to
  stderr, prints nothing to stdout, and exits `1`. `mlm week` never
  falls back to the default target or to "no earlier weeks", and
  neither `--notes` nor `mlm export notes` falls back to an empty note
  list. Plain `mlm week` makes no note read.
- `mlm week WEEK_ID target DURATION`, the week id before `target`
  (§3.7). It is rejected with the one-line message there and exit `2`.

### 6.2 Not errors

- A `status`/`week` target (date or week) with no punches/notes at
  all: valid, just renders as empty for that scope.
- Any §4.3 pairing anomaly (extra open stints, an orphaned `end`): the
  punch is still inserted, and the anomaly shows up in that date's next
  `status`/`week` output. A `stop` to a date that holds an orphaned
  `end` also warns on stderr right after the write (§7.4), and still
  exits `0`.
- A week with no `week_targets` row: valid, uses the default target
  (40h) rather than erroring for "unset".
- `mlm export notes` over a range with no notes: prints nothing and
  exits `0` (§7.5).
- `mlm export notes` for a future date, week or month: not checked
  against today (§3.8). It has no notes, so it prints nothing and exits
  `0`.

### 6.3 Exit codes

`0` on success (including a status view that reports anomalies, since
the anomaly is data to look at and not a tool failure). Nonzero on any
§6.1 hard error. A runtime error (a bad value, a storage failure) exits
`1`. A usage error from the argument parser (an unknown flag, a missing
argument, the `week WEEK_ID target DURATION` shape) exits `2`.

## 7. Output layout

Plain ASCII throughout (no box-drawing/unicode dashes) — matches the
portability goal behind the ratatui pick (README) even though this is
plain `println!` output, not a TUI screen.

### 7.1 `mlm status`

`DATE` omitted (today), so the est.-EOD figure applies and the week
shown is the actual current week. Thursday, now 18:05, target 40h,
carry-in +06h 55m, Mon-Wed closed 13h 40m, and today's stints
09:00-13:00, 13:30-16:55 and 17:25-now:

```
Thu 2026-10-01

Day total:     08h 05m (incl. ongoing), est. EOD 21:25
Week 2026-40:  03h 20m left to 32h 00m by end of Thursday (fulfillment 28h 40m = worked 21h 45m + carry-in 06h 55m / target 40h 00m)

  09:00-13:00  (04h 00m)
  13:30-16:55  (03h 25m)
  17:25-now    (00h 40m, ongoing)

Notes:
  - fixed migration runner bug
  - started punch pairing tests
```

The arithmetic for this sample is `live` = 40m, today's closed stints
= 07h 25m, day total = 08h 05m, `worked` = 13h 40m + 07h 25m + 40m =
21h 45m, `fulfillment` = 21h 45m + 06h 55m = 28h 40m, `required` = 4 ×
08h 00m = 32h 00m, `gap` = 03h 20m, and `est. EOD` = 17:25 + (32h 00m −
28h 00m) = 21:25.

`2026-40` carries in a +06h 55m surplus from the previous week, so the
fulfillment parenthetical here is the **expanded** form, `fulfillment
F = worked W + carry-in C / target T`, not the shorter `fulfillment F /
target T` form — that shorter form only appears when the current week's
carry-in is exactly zero.

- Header: `<weekday abbrev> <YYYY-MM-DD>`, with a suffix
  `  (HH:MM continues previous day's stint)` when this date's
  chronologically-first punch was consumed by a §4.3.1 splice onto the
  previous date — e.g. `Fri 2026-02-13  (00:45 continues previous
  day's stint)`. No bracket marker, no extra line. `status`-only:
  `week`'s row for the same date shows no equivalent marker (§7.2) —
  an accepted asymmetry, not a bug.
- **Day total** and **week** lines lead the output, ahead of the
  stint list — the "how much is left" figures are the point of a
  status check, not something to hunt for at the bottom.
- Day total is a tabular-format sum. For today it includes the running
  stint's live minutes, so it changes minute by minute while a stint
  runs, and the suffix `(incl. ongoing)` flags that. The suffix appears
  for any open start dated today, including one typed for a time later
  than now (`live` 0, nothing added). On an earlier date, an open stint
  is not counted and the suffix is `(+ ongoing)`, or `(+ unclosed)` when
  that open stint is two or more calendar days old, matching the stint
  line's own wording change below — a stale, probably-forgotten punch
  reads differently from a live one. The Day total line carries no
  due-today clause. That figure lives on the week line.
- `live` is the whole minutes from the most recent open start dated
  today to now, clamped at 0, and 0 when no stint is open today. An open
  start from an earlier date never counts, and with two or more open
  starts today only the most recent one counts (summing them would
  count the same minutes twice). An open start two or more calendar
  days old is stale and gets no live credit. `live` always comes from
  today's punches, whatever date `status` shows, classified the same way
  `status` for today classifies them. `live` enters the current week's
  `worked` and `fulfillment` (§2.4) and today's day total, and is never
  stored.
- `required` is the slice of the target due by end of today (§2.4),
  `daily target × weekday number` Monday through Thursday and the full
  target Friday through Sunday, always by today's weekday. `gap =
  required − fulfillment`, with `fulfillment = carry_in + worked`
  live-inclusive. `X` is always `gap`'s absolute value. Which word
  prints carries the sign — a word switch, not an inline `-` (matching
  decision 18's closed-week "Total behind" / "Total ahead" split, §7.2,
  rather than §4.2's bare-signed-number convention used elsewhere).
  `gap == 0` counts as "left to" (X = `00h 00m`), matching every other
  "reached exactly" case in this spec. The gap is driven by
  *fulfillment*, not by the "Day total" figure printed above it, so the
  two can point opposite directions mid-day (e.g. day total still
  climbing while the week line already reads `over` because of a large
  carry-in), and that is expected: day total is "today, in isolation,"
  the week line is "today's slice of the whole week's math."
- **Estimated EOD** (`est. EOD HH:MM`) appears only when today has an
  open stint: it's `open start + (required − fulfillment without the
  running stint's live minutes)`, equivalently `now + gap`. It is the
  clock time at which this open stint closes the gap, so it stays the
  same on every read while the stint runs and moves only when a punch is
  added or removed. For a start at or before now this equals `now +
  gap`, with `gap` the live-inclusive figure above. The open start is the
  most recent open start dated today. A start typed for a time later
  than now has `live` 0, so the week line equals the no-stint figure, and
  `est. EOD` projects from that start's own time. It therefore sits
  further ahead of the clock than the week line's `left to` figure, and
  the week line does not count down. Omitted entirely when
  there's no open stint (nothing to project from) or replaced with
  `done for today` when the gap is already zero or negative. When
  the projected time lands on a later calendar date than today, the line
  gains a day suffix after the clock time, so the time can't be misread
  as landing before midnight tonight. The suffix depends on how many
  days later it lands, and the weekday abbreviation matches the
  header's:
  - today: no suffix, e.g. `est. EOD 17:30`
  - 1 day later: ` (tomorrow)`, e.g. `est. EOD 01:15 (tomorrow)`
  - 2–6 days later: ` (<weekday abbrev>)`, e.g. `est. EOD 06:20
    (Mon)`
  - 7 or more days later: ` (<weekday abbrev> <YYYY-MM-DD>)`, e.g.
    `est. EOD 06:20 (Mon 2026-10-12)`

  `est. EOD` is fixed per stint, so its day suffix shows from the first
  `status` after the `start` and does not change while the stint runs. A
  break (`stop`, then `start`) moves it later by the break's length. From
  Friday on `required` is the full target, so with little worked it can
  land days out, e.g. `est. EOD 06:00 (Mon)`. It sits on the Day total
  line, one line above the week line it is computed from. Monday through
  Thursday, `done for today` means today's slice is covered, not the
  target the week line's parenthetical names. At `gap == 0` the week
  line still reads `00h 00m left to ...` beside `done for today`, and
  `done for today` can show next to `(incl. ongoing)` while a stint
  runs, or with little or no time worked when carry-in already covers
  what is due. Two `status` runs with no punch between them can print
  different week-line figures, because live minutes grow with the clock.
  When the running stint closes the gap, the line flips from `left to`
  to `over` and `est. EOD` becomes `done for today`.
- Week line reports the week containing `DATE` (§3.5), and its framing
  follows the same current-vs-not split as `mlm week` (§7.2). For the
  actual current week the format is `Week <id>:  <headline>
  (<parenthetical>)`, and the headline depends on `gap` and today's
  weekday (even if `DATE` itself is some other day within that same
  week — the figure is always about now, not about which day's stints
  you're viewing). The line prints whether or not `DATE` is today.

  | | `gap >= 0` | `gap < 0` |
  |---|---|---|
  | Monday through Thursday | `X left to <required> by end of <Weekday>` | `X over <required> by end of <Weekday>` |
  | Friday through Sunday | `X left to <target> target` | `X over <target> target` |

  `<Weekday>` is today's full weekday name. From Friday on `required`
  equals the target (§2.4), so the line names the target alone. A
  zero-target week follows the same rule, e.g. `00h 00m left to 00h
  00m by end of Monday` with nothing worked, or `02h 00m over 00h 00m
  by end of Monday` after two hours. A past week gets the plain `Total
  behind`/`Total ahead` form when `DATE`'s week isn't the current one. A
  future week is viewable only through `mlm week WEEK_ID` (§7.2),
  because `status DATE` rejects a future date (§6.1). When the current week's `carry_in` is non-zero, the
  `(fulfillment .../target ...)` parenthetical expands to
  `(fulfillment F = worked W + carry-in C / target T)`, each figure
  signed independently per §4.2 (a carry-in deficit can drive
  `fulfillment` itself negative — that's expected, not an error) — when
  `carry_in` is zero, the shorter one-term form is used. `F` and `W` are
  the live-inclusive figures. The past-week `Total behind`/`Total
  ahead` form never gets this parenthetical, zero or non-zero carry-in
  alike. On a Friday with fulfillment 41h 10m including live, the line
  reads:

  ```
  Week 2026-40:  01h 10m over 40h 00m target (fulfillment 41h 10m = worked 34h 15m + carry-in 06h 55m / target 40h 00m)
  ```

  The week line changes form at Thursday-to-Friday midnight, from `left
  to 32h 00m by end of Thursday` to `left to 40h 00m target`. Monday
  through Thursday, `over` means today's slice is met while the week's
  target may still be far off. The current week prints no whole-week
  remainder. It is `target - fulfillment`, which the parenthetical lets
  a reader compute, so Monday through Thursday `mlm week`'s `Target:`
  and the headline's `left to` figure name different amounts. A week
  that starts in debt can read more left than the target. With a
  carry-in of `-28h 00m` and 17h 27m worked on a Friday, the headline
  reads `50h 33m left to 40h 00m target`, because `gap` is `required -
  fulfillment` and fulfillment includes the debt. The same zero reads
  differently by framing. A current week at `gap == 0` reads `00h 00m
  left to ...`, and the same week once closed reads `Total ahead: 00h
  00m` (§7.2). For an earlier date in the current week, the week line
  counts today's live minutes beside a day total for `DATE` that has
  none, so the day total stays fixed while the week line moves. `mlm
  week` moves only today's row.
- Stint lines: `HH:MM-HH:MM  (duration)`, ongoing stint's end is the
  literal word `now`. Tabular duration format (§4.2). Section omitted
  entirely when the date has zero stints and no overnight stint line
  (below) (a note-only day, or a fully empty one) — same treatment as
  Notes below, no empty-list placeholder either way.
  - A **completed** stint whose end falls on a later calendar date
    than its start (a §4.3.1 splice) gets `, spans to next day` added
    inside the duration parenthetical, same slot the `, ongoing` suffix
    below occupies: `23:30-00:45  (01h 15m, spans to next day)`.
  - An **open** stint's rendering depends on how old its date is,
    relative to today: today's own open stint is unchanged
    (`17:45-now  (00h 15m, ongoing)`); one dated yesterday keeps its
    live duration and gains a caption
    (`23:10-now  (10h 35m, ongoing - duration as of right now, not a
    running total)`); one dated two or more days back drops the
    duration figure entirely — at that age the number is noise, not
    information — and renders as `09:00-?  (unclosed)` instead, with `?`
    in place of `now` so an old start does not read as running. That
    same date now also carries a `[!]` stale-open-start anomaly
    (§7.3), and, while it's within the 14-day lookback window,
    today's own `status` surfaces it too (the lookback bullet below).
- Notes render only if any exist for the date (section omitted
  otherwise, not shown empty).
- Anomalies (§4.3), if any, print between the two lead lines and the
  stint list — see §7.3 for the line shapes, including the
  stale-open-start and refused-join kinds.
- For today only (`DATE` omitted, or today's own date given
  explicitly), the anomaly block also lists dates that still have open
  starts, oldest first, each as its own `[!]` line with `fix:`/`or:`
  hints (§7.3). A date 2 to 14 days old with one open start gives the
  unclosed-start line. Yesterday with two or more open starts gives the
  open-stints line with `(yesterday)`. A start open for more than 14
  days drops out of today's `status` and out of `mlm start`'s warnings.
  It stays on its own date's `status` and its own `week` row, and the
  carry-in marker below flags its week. `status DATE` for any other date
  runs no lookback and no hint.
- Yesterday hint, today's page only. When yesterday has exactly one
  open start and it stays unjoined once today's punches are taken into
  account (§4.3.1), `status` prints an `[i]` hint after the anomaly
  block and the lookback lines (line shape in §7.3). A `stop` typed today
  as today's first punch joins the start and clears the hint. There is no
  hint when today's page already shows a refused-join line naming
  yesterday. The `or:` line prints only when no punch follows the open
  start on yesterday's date and today has no punches yet.
  The hint is not an anomaly. It never feeds `[!]`, the `[!]` footer, the
  carry-in marker or `has_any`, and it never shows on `status DATE` for
  another date. `status -1` shows yesterday's open start as a stint line
  only. The hint, the overnight stint line below and the `[i]` row mark
  in `mlm week` (§7.2) all fire on the same condition, so they appear
  and disappear together. Call that start the overnight stint.
- Overnight stint line, today's page only. While the hint prints, the
  stint list opens with one extra line for the overnight stint, ahead of
  today's own stints:

  ```
    22:00-now    (15h 30m so far, started yesterday, not counted until stopped)
  ```

  The range uses the same column layout as other stint lines. The
  duration is whole minutes from the start to now in the standard
  formatter (§4.2), so it can pass 24h. The line is rendered on its own,
  outside today's pairing and sorting, so the list is not in clock order
  when it prints. It prints even when today has punches of its own, and
  it is the only line in the list whose minutes are not in the Day total.
  With no punches today the list holds just this line under `Day total:
  00h 00m`. With an open stint of today's own, both `-now` lines print,
  the overnight one first, and only today's own stint counts and carries
  `(incl. ongoing)` and `est. EOD`. `status DATE` for any other date,
  `status -1` included, keeps its own wording and prints no such line.
  The line goes away together with the hint: when a `stop` joins the
  start to today, when a refused-join line names yesterday, or when
  yesterday has two or more open starts. When the start turns two days
  old, the line and every `[i]` mark disappear and the `[!]` stale
  lookback line and week row marker take over, though the stint is still
  open.
- Overnight stint accounting. The overnight stint gets no live credit
  and no `est. EOD`. Its minutes stay out of the Day total, `Worked`,
  `Fulfillment` and the week line until `stop`, and `stop` returns them to
  yesterday. The Day total shows no `(incl. ongoing)` for it. At 00:00
  the week line jumps, because `required` steps up a weekday and the
  stint is not counted. Past Sunday midnight the closed week's `Worked`
  leaves the stint out until `stop` (§7.2).
- Carry-in marker, any `DATE`. The shown week's carry-in comes from
  every earlier week, from the earliest week with any punch up to the
  week before the shown one. When at least one date in those weeks would
  carry `[!]` on a `mlm week` row (E7, stale start, E8, overlap, refused
  join), `status` prints one marker line after the anomaly block, the
  lookback lines and the hint (§7.3). The 14-day lookback window does not
  limit it. It uses the same per-date check as the row marker, so the two
  never disagree. It looks only at weeks before the shown week, so the
  anomalous week's own page shows `[!]` rows and no marker, and a week
  with no earlier data or no earlier anomaly prints nothing. It counts
  every anomaly kind, including an orphaned end that adds no minutes, so
  it can name a week whose figures are right. It stays on every later
  week until the old anomaly is fixed, and it applies to stored history,
  so an old anomaly shows it on the first run with no new punch.
- `status DATE` with `DATE` after today is a hard error (§6.1).

`mlm status` on Monday 2026-10-05 at 13:30, no punches today, with a
start at 22:00 still open from Sunday and a zero carry-in (week 2026-41
runs Monday to Sunday). The line is listed under the Day total and is
not part of it:

```
Mon 2026-10-05

Day total:     00h 00m
Week 2026-41:  08h 00m left to 08h 00m by end of Monday (fulfillment 00h 00m / target 40h 00m)

[i] start on Sun 2026-10-04 at 22:00 still open (yesterday; not counted until stopped)
    fix: mlm stop HH:MM --date 2026-10-04  (a time after 22:00)
     or: mlm stop  (still working: ends it now, across midnight)

  22:00-now    (15h 30m so far, started yesterday, not counted until stopped)
```

`mlm status 2026-01-05` (a past date, different week, "today" is
still `2026-10-01`) — no est.-EOD figure, and the week
line uses the plain closed-week form since `2026-01-05`'s week has
already ended:

```
Mon 2026-01-05

Day total:     06h 15m
Week 2026-02:  Total behind: 01h 40m

  08:30-14:45  (06h 15m)
```

### 7.2 `mlm week`

Current (ongoing) week — this example, `2026-40`, is the week
containing "today" (`status`'s `2026-10-01`, a Thursday, now 18:05):

```
Week 2026-40 (2026-09-28 - 2026-10-04)

03h 20m left to 32h 00m by end of Thursday

  Mon 2026-09-28   04h 30m
  Tue 2026-09-29   04h 40m
  Wed 2026-09-30   04h 30m
  Thu 2026-10-01   08h 05m (incl. ongoing)
  Fri 2026-10-02   00h 00m
  Sat 2026-10-03   00h 00m
  Sun 2026-10-04   00h 00m

Carry-in:      06h 55m
Worked:        21h 45m
Fulfillment:   28h 40m
Target:        40h 00m
```

A **past or future** week has no "today" to frame a deadline against,
so its headline is a plain total instead — everything else is
identical in shape, same per-day rows and the same
carry-in/worked/fulfillment/target block:

```
Week 2026-06 (2026-02-02 - 2026-02-08)

Total behind: 03h 10m

  Mon 2026-02-02   07h 30m
  Tue 2026-02-03   08h 00m
  Wed 2026-02-04   07h 45m
  Thu 2026-02-05   08h 10m
  Fri 2026-02-06   05h 25m
  Sat 2026-02-07   00h 00m
  Sun 2026-02-08   00h 00m

Carry-in:      00h 00m
Worked:        36h 50m
Fulfillment:   36h 50m
Target:        40h 00m
```

(or `Total ahead: 00h 00m` etc. for a week that ended at/above target
— same signed summary formatting, §4.2, just without a weekday tied
to it.)

A future week has not started, so with no carry-in its headline reads
`Total behind` for the whole target. `mlm week WEEK_ID` is the way to
view a future week, because `status DATE` rejects a future date (§6.1).

- Header names the week id and its Mon-Sun date span.
- Headline for the current week is the same text `status`'s week line
  shows (§7.1), just leading its own output here instead of being
  inline. The headline logic is shared, so the two commands cannot
  drift: `X left to <required> by end of <Weekday>` or `X over
  <required> by end of <Weekday>` Monday through Thursday, `X left to
  <target> target` or `X over <target> target` from Friday on. A closed
  or not-yet-started week gets the plain `Total behind`/`Total ahead`
  form regardless (NOTES.md decision, this round).
- `mlm week` shows no `est. EOD`.
- One row per calendar date in the week, always all 7 even if some
  are empty (`00h 00m`) — consistent shape, easy to scan for gaps.
  `(incl. ongoing)` marks a date whose open stint's date equals today,
  and only when the requested week is the current one — not just "any
  date with an open stint." That row's total includes the running
  stint's live minutes, and `Worked:` and `Fulfillment:` show the
  live-inclusive figures, so the rows still sum to `Worked:`. A future
  week's `Carry-in:` reflects the current week's live-inclusive
  fulfillment (§2.4). `Carry-in:` and `Target:` of the current week and
  all past-week output are unchanged. This distinction still matters for the
  §4.3.1 residual case: when a cross-midnight session's earlier date
  *doesn't* qualify for the boundary splice (not the ordinary 1:1,
  first-punch shape — e.g. two dangling starts that date), the join is
  refused. Both dates flag a `[!]` refused-join line naming each other
  (§7.3/E17), at any age, alongside whatever E7/E8 anomaly the shape
  itself produces (two dangling starts is E7 on the earlier date; an
  extra orphaned end is E8 on the later one). Neither marker is
  `(incl. ongoing)` — that stays reserved for a clean open stint dated today.
  The ordinary case (a clean cross-midnight session) splices into one
  completed stint on the earlier date instead of reaching this
  fallback at all.
  `status`'s own rendering of a refused-join date does show the open
  stint's age-based caption/`(unclosed)` form (§7.1); `week`'s row
  shows the total and `[!]` but no equivalent caption, another
  instance of the same status/week asymmetry the header suffix (§7.1)
  also has.
- Field order below the headline is fixed: carry-in, worked,
  fulfillment, target — the headline figure isn't repeated down here
  since the headline already states it plainly.
- Any date in the row list with an anomaly (§4.3, plus the
  stale-open-start, overlap and refused-join anomalies) gets a marker (§7.3)
  appended to its row rather than a separate block, since the week
  view is already one-line-per-date.
- When at least one row carries `[!]`, one footer line follows the
  table after a blank line:

```
[!] needs a look: run mlm status DATE for details and fixes
```

  No footer when no row is marked. `week` runs no lookback and prints no
  hint block. Its anomaly output has four parts: the row marker, the
  footer, the carry-in marker below, and the `[i]` row mark with its
  footer (below).
- Overnight stint mark. While today's `status` prints the `[i]` hint
  (§7.1), the row for yesterday ends with `  [i]`, after the duration and
  in the same column as `  [!]`. A row with both an anomaly and the
  overnight stint ends `  [!] [i]`, and both footers print. The mark shows
  in whichever shown week contains yesterday, the current week or a past
  week requested by `WEEK_ID`. No other row is marked, and a future week
  shows nothing. The mark follows the hint's own trigger, so it is absent
  when the start is joined to today, when a refused-join line names
  yesterday, when yesterday has two or more open starts, and when the
  start is two days old. When any row shows `[i]`, one footer line
  prints. It names the marked row's weekday and date, here a Sat:

```
[i] open stint on Sat 2026-10-03: mlm status shows how to stop it
```

  The Sunday row of last week's view, on a Monday:

```
  Sat 2026-10-03   00h 00m
  Sun 2026-10-04   00h 00m  [i]

[i] open stint on Sun 2026-10-04: mlm status shows how to stop it
```

  The `00h 00m` is correct because the overnight stint is not counted.
  On a Monday, yesterday is last week's Sunday. The current week's view
  (the week containing today) has no marked row, so the `[i]` footer slot
  names the stint instead:

```
[i] open stint on Sun 2026-10-04 (last week): mlm status shows how to stop it
```

  The weekday and date are the stint's own, in the row display form. This
  line prints only in the view of the week containing today and only when
  the stint's date lies in the previous week, so a past week viewed by
  `WEEK_ID` never prints it and marks the Sunday row of last week's view
  instead. The plain footer and the Monday line never print together. Both
  depend only on the overnight trigger, never on whether a row also shows
  `[!]`. The Monday line still prints when last week's Sunday row carries
  `[!]`. `[i]` never feeds `[!]`, the `[!]` footer, the carry-in marker,
  `has_any` or any figure. A week with only an `[i]` row prints no `[!]`
  footer. `Worked`, `Fulfillment`, `Target` and the headline are
  unchanged by the mark, and past Sunday midnight the closed week's
  `Worked` leaves the stint out until `stop` (§7.1).
- Carry-in marker. When an earlier week has an anomaly (the same
  condition as in §7.1), the Carry-in figure gets a trailing marker and
  the line `status` prints for the same week sits above the Carry-in
  block. The layout is the day rows, a blank line, the `[!]` footer if any
  row is marked, the `[i]` footer or Monday line if it applies, the
  marker line directly under the last footer printed (no blank line
  between), one blank line, then the Carry-in block. With no footer line
  the marker line takes the footers' place:

```
  Sun 2026-10-11   00h 00m

[!] needs a look: run mlm status DATE for details and fixes
[!] carry-in includes anomalies from 2 earlier weeks, oldest 2026-38 (see mlm week 2026-38)

Carry-in:      -80h 00m  [!]
```

  With one week the line reads `from 1 earlier week, 2026-38 (see mlm
  week 2026-38)`. Nothing prints for a week with no earlier data or no
  earlier anomaly. It prints for any shown week, future and current
  included, and never counts the shown week's own anomalies. The blank
  line before the Carry-in block prints exactly when any footer or
  marker line printed.
- Notes (`--notes`/`-n`, §3.6). Each of a date's notes prints on its
  own line directly under that date's row, before the next row, as six
  spaces, `- `, then the body as stored. No `Notes:` header and no blank
  line surround a row's notes. A date shows the notes `status DATE`
  lists, in the same insertion order (`created_at_utc ASC, id ASC`,
  §2.3). Only the layout differs, since `status` prints a `Notes:`
  header and two-space bullets (§7.1). A note belongs to its stored
  `date`, so stint pairing and the §4.3.1 splice never move it. Row
  markers stay at the end of the row, so a marked row reads marker
  first, notes below. Blank lines, footers, the carry-in marker line and
  the Carry-in block are unchanged. An illustrative current week, today
  Sat 2026-10-24, one marked row, `mlm week --notes`:

```
Week 2026-43 (2026-10-19 - 2026-10-25)

05h 16m over 40h 00m target

  Mon 2026-10-19   08h 27m
      - fixed migration runner bug
      - started punch pairing tests
  Tue 2026-10-20   07h 12m
  Wed 2026-10-21   07h 24m  [!]
      - code review for #41
  Thu 2026-10-22   08h 52m
  Fri 2026-10-23   08h 40m
  Sat 2026-10-24   08h 13m
  Sun 2026-10-25   00h 00m

[!] needs a look: run mlm status DATE for details and fixes

Carry-in:      -03h 32m
Worked:        48h 48m
Fulfillment:   45h 16m
Target:        40h 00m
```

  A note-only date keeps its `00h 00m` row, with its notes under it.
  Notes never change a total, a marker, a footer, the headline or the
  Carry-in block. A week with no notes prints exactly what plain `mlm
  week` prints, and plain `mlm week` prints no note lines. Note bodies
  pass through unchanged, non-ASCII included, as in `status` (§2.3,
  §7). With the flag, `mlm week` makes one ranged note read over the
  shown week's seven dates, Monday to Sunday, and a failed read is a
  hard error like every other `week` read (next bullet, §6.1). Without
  the flag it makes no note read.
- Every storage read `mlm week` makes is a hard error, as in `status`
  (§6.1). The storage error goes to stderr as `error: ...`, stdout stays
  empty, and the exit code is `1`. `mlm week` never falls back to the
  default 40h target or to "no earlier weeks".

### 7.3 Anomaly rendering

Plain-ASCII marker, no color/unicode dependency: prefix `[!] `. `[i] `
marks a hint that is not an anomaly. Most
`status` anomaly lines also carry one or two indented `fix:`/`or:`
lines naming a command to run — there's no automatic repair, the user
runs the named command themselves. The refused-join line is the one
exception: it carries no `fix:` lines of its own, since the E7/E8/stale
lines already on the two dates carry them. Below, `D` stands for the
anomaly's own date (`YYYY-MM-DD`), and `HH:MM` inside a `fix:`/`or:`
line is a literal placeholder the user replaces with a real time.

Lines for one date print in this fixed order: multiple open stints
(E7), stale open start, orphaned ends (E8, ascending by instant),
overlapping stints (one per pair), a refused join with the previous date
then the next date, and, for today only, one lookback line per earlier
date with open starts (oldest first, §7.1), then the yesterday hint. The
carry-in marker line comes last (§7.1).

Two fix-command details shape the wording throughout: `mlm delete
punch --date D` only lists and numbers that date's punches (§7.4) —
deleting takes a second run naming the number, so the hint says so.
`mlm stop HH:MM --date D` pairs by time (§4.3): an end before the open
start becomes a new orphaned end, and with several open starts it
closes the latest one before it, so the stop hint names the start
time the new end must follow.

A stop hint targets one open start `S`. Let `N` be the first punch on
that date after `S`. A stop between `S` and `N` pairs with `S` and
nothing else, and a stop after `N` pairs with a later start or nests `S`
around later stints. With no punch after `S` the hint reads `(a time
after S)`. With one it reads `(a time between S and N)`. The window
includes `N`'s minute, because a stop at the same instant as a start
pairs with `S` (§4.3, step 2). Punches at the same minute as `S` do not
count as after `S`. An open start at `23:59` with nothing after it keeps
`(a time after 23:59)`, which no `HH:MM` satisfies (a same-instant stop
pairs zero-length, E14). The rule applies to the stale-start line, the
E7 line, the lookback lines, the yesterday hint and `mlm start`'s
warnings.

E7, multiple open stints (`14:00` below is the latest open start on
the date):

```
[!] 2 open stints for this date (only one can be open at a time)
    fix: mlm delete punch --date D  (lists punches; rerun with the extra start's number)
     or: mlm stop HH:MM --date D  (a time after 14:00 closes the 14:00 start)
```

With a later punch after the 14:00 start, the stop hint names a window:

```
     or: mlm stop HH:MM --date D  (a time between 14:00 and 15:00 closes the 14:00 start)
```

The hint names only the latest open start, but a `stop` between two open
starts also works. It pairs with the earlier start and leaves the later
one open. The stint list shows every open stint with its own duration,
while the week line and `est. EOD` credit only the most recent start
(§7.1), so fixing the anomaly makes both jump.

Stale open start (§4.3) — a lone open start on a date 2 or more
calendar days old:

```
[!] unclosed start at 09:00, 5 days old (no stop recorded)
    fix: mlm stop HH:MM --date D  (a time after 09:00)
     or: mlm delete punch --date D  (lists punches; rerun with the start's number)
```

With a later punch the hint names a window:

```
    fix: mlm stop HH:MM --date D  (a time between 09:00 and 10:30)
```

E8, orphaned end (line text unchanged, fix lines new):

```
[!] orphaned end at 18:00 (no matching start)
    fix: mlm start HH:MM --date D  (a time before 18:00)
     or: mlm delete punch --date D  (lists punches; rerun with the end's number)
```

Overlapping stints (§4.3), one line per overlapping pair:

```
[!] overlapping stints 09:00-12:00 and 10:00-11:00 (the overlap counts twice)
    fix: mlm delete punch --date D  (lists punches; rerun with the number of the punch to remove)
```

An open stint prints as `09:00-now`. A spliced cross-midnight stint
(§4.3.1) is labelled with its real start and end times, as in the stint
list (e.g. `23:30-00:45`), on both dates. Pairs sort by the first
stint's start, then the second's. The date's `week` row gets `[!]` and
the date counts toward the footer. The overlap still counts twice in the
day and week totals. The line flags the double count and does not fix
it.

Refused join (§4.3.1) — no `fix:` lines of its own, since the
E7/E8/stale lines on the two dates already carry them; the reason
text is the same on both dates:

```
[!] not joined to Tue 2026-09-22 across midnight (a join needs exactly one open start on Mon 2026-09-21 and exactly one unmatched end, as Tue 2026-09-22's first punch)
```

The other date's line names the first date instead: `not joined to
Mon 2026-09-21 across midnight`, same parenthetical.

Lookback lines (today's `status` only, §7.1) — one open start on the
window date:

```
[!] unclosed start on Mon 2026-09-21 at 09:00 (5 days ago)
    fix: mlm stop HH:MM --date 2026-09-21  (a time after 09:00)
     or: mlm delete punch --date 2026-09-21  (lists punches; rerun with the start's number)
```

With a later punch on that date, the hint names a window (see the
stop-hint rule above):

```
    fix: mlm stop HH:MM --date 2026-09-21  (a time between 09:00 and 10:30)
```

Two or more open starts on the window date:

```
[!] 2 open stints on Mon 2026-09-21 (5 days ago)
    fix: see mlm status 2026-09-21
```

Yesterday with two or more open starts uses `(yesterday)` in place of
`(N days ago)`:

```
[!] 2 open stints on Fri 2026-10-02 (yesterday)
    fix: see mlm status 2026-10-02
```

Yesterday with exactly one unjoined open start gets an `[i]` hint
instead of an anomaly line (§7.1). `[i]` is the quiet marker. It never
feeds `[!]`, the `[!]` footer, the carry-in marker or `has_any`. It also
marks yesterday's `week` row and adds the `[i]` footer there (§7.2),
and today's stint list gets a line for the same start (§7.1):

```
[i] start on Fri 2026-10-02 at 18:02 still open (yesterday; not counted until stopped)
    fix: mlm stop HH:MM --date 2026-10-02  (a time after 18:02)
     or: mlm stop  (still working: ends it now, across midnight)
```

The `or:` line prints only when no punch follows the open start on
yesterday's date and today has no punches yet. A `stop` now joins the
start only when it becomes today's first punch. A later punch yesterday
means the start is probably a forgotten stop, and joining it would
nest it over that later stint.

The carry-in marker (§7.1, §7.2) is one `[!]` line, last on the page:

```
[!] carry-in includes anomalies from 2 earlier weeks, oldest 2026-38 (see mlm week 2026-38)
```

In `week`, appended inline to the affected date's row:

```
  Wed 2026-02-11   08h 00m  [!]
```

(detail deferred to that date's own `status` output rather than
repeated in the week table). When at least one row carries `[!]`, one
footer line follows the table (§7.2). The overnight stint's row ends
with `  [i]` in the same column, or `  [!] [i]` when it also carries an
anomaly, and prints the `[i]` footer (§7.2).

### 7.4 Write-command output

`start`, `stop` and `note` print nothing on success, apart from the
stderr warnings `start` and `stop` print (below). They are silent,
Unix-conventional, and exit `0` (§6.3) is the only signal. A hard error
(§6.1) still prints its message to stderr as usual. `status`, `week`
and `export notes` produce stdout output (§7.1, §7.2, §7.5). `week
target` prints one line (§3.7). `delete note`/`delete punch` are a
narrow, deliberate exception to the write-command-silence rule above
rather than a repeal of it. List mode prints that date's numbered
entries (or `nothing to delete for <date>.` when there are none), and a
successful delete prints a ready-to-run recreate command.

`start` gains one more narrow exception, on stderr only. After a
successful write, it runs the same 14-day lookback `status` uses for
today (§7.1) and prints one warning per stale date found, oldest
first — stdout stays silent and exit stays `0` regardless. The date
just written to draws no warning only when its single open start is
the exact punch this command just inserted. Two or more open starts
there (the new punch made an E7) still warn, and so does a single open
start that is an older punch — the new start paired off with an
existing orphaned end there, leaving the old start open.

A single open start warns with three lines, laid out like `status`'s
own lookback line (§7.3):

```
warning: unclosed start on Mon 2026-09-21 at 09:00 (5 days ago)
    fix: mlm stop HH:MM --date 2026-09-21  (a time after 09:00)
     or: mlm delete punch --date 2026-09-21  (lists punches; rerun with the start's number)
```

Two or more open starts on the date still warn in one line:

```
warning: 2 open stints on Mon 2026-09-21 (5 days ago); see mlm status 2026-09-21
```

A single unjoined open start dated yesterday warns too, with no `or:`
line because today now has a punch (§7.1):

```
warning: start on Fri 2026-10-02 at 18:02 still open (yesterday)
    fix: mlm stop HH:MM --date 2026-10-02  (a time after 18:02)
```

Yesterday with two or more open starts warns in the one-line form with
`(yesterday)`:

```
warning: 2 open stints on Fri 2026-10-02 (yesterday); see mlm status 2026-10-02
```

`start` repeats the yesterday warning on every call while that start
stays open. With a later punch on a warned date, the `fix:` time is a
window, `(a time between S and N)` (§7.3). `start` prints no carry-in
marker.

The check is best-effort: if the lookback read fails after the write
already committed, `start` still prints nothing extra and still exits
`0`.

`stop` has its own narrow exception, also on stderr only. After a
successful write to date `D`, it classifies `D` with the splice
applied, exactly as `D`'s `status` page does (§4.3.1). It prints one
warning block per orphaned end on `D`, ascending by instant, with no
blank line between blocks. That includes an end the new punch
displaced, and an orphan that was already on `D` before the write, so
every `stop` to `D` reports the orphans `D` holds. An end that splices
with the previous date's open start is not orphaned and draws no
warning. `--date` changes `D`, not the rule. `stop` checks no other
date and warns about nothing else, so an overlap or a second open start
it creates waits for `status`. Stdout stays empty and the exit code
stays `0`.

The header names the date, which `status`'s E8 header leaves to its
page. The two hint lines are `status`'s E8 lines (§7.3) word for word.
With today Mon 2026-10-05, a start from two days back is still open and
a bare `stop` lands on today at 14:16. The splice joins adjacent dates
only, so today's end stays orphaned:

```
$ mlm start 22:00 --date -2
$ mlm stop
warning: orphaned end on Mon 2026-10-05 at 14:16 (no matching start)
    fix: mlm start HH:MM --date 2026-10-05  (a time before 14:16)
     or: mlm delete punch --date 2026-10-05  (lists punches; rerun with the end's number)
```

`status` shows the stale start that the end failed to close. Entering a
stop before its start warns once (§1.2a).

This check is best-effort too. If any read after the committed write
fails, `stop` prints nothing at all and still exits `0`. A failed
neighbor read never counts as an empty date, since that would turn a
spliced end into a false orphan. A failed write to stderr is ignored
and never changes the exit code. A hard error before the write (§6.1)
is unchanged.

### 7.5 `mlm export notes`

One line per note: the date as `YYYY-MM-DD`, two spaces, then the body
as stored. No header, no blank line, no summary. Lines run ascending by
date, and within a date in insertion order (`created_at_utc ASC, id
ASC`, §2.3), the order `status` (§7.1) and `mlm week --notes` (§7.2)
use. A week with notes on three of its dates:

```
$ mlm export notes --week 2026-43
2026-10-19  fixed migration runner bug
2026-10-19  started punch pairing tests
2026-10-21  code review for #41
2026-10-24  refactored the parser
```

- An empty range prints nothing and exits `0`.
- Note bodies pass through unchanged, non-ASCII included, as in
  `status` (§2.3, §7). Every other byte is ASCII.

## 8. User flows (test basis)

Concrete flows mapped end-to-end against the spec, per NOTES.md's
final-review note, and revised after an independent adversarial pass
over the whole document (findings folded into the sections above; see
NOTES.md for the log). Each flow below is meant to become a test more
or less directly.

### 8.1 Happy paths

- **F1** — First punch of a fresh day: `start` only. `status` shows
  one open stint, day total `00h 00m`, no completed stints.
- **F2** — Ordinary day: `start`, `stop`. One stint, no anomalies.
- **F3** — Out-of-order entry (the motivating case, §4.3): `start
  09:00`, `start 14:00`, `stop 18:00`, `stop 13:00`, in that entry
  order → stints `09:00-13:00` and `14:00-18:00`.
- **F4** — Note-only day, no punches at all: just `mlm note "..."`.
  `status` shows the Notes block only, stint-list section fully
  omitted (§7.1).
- **F5** — `start`/`stop` with an inline `NOTE`: one command produces
  both a punch and a note row.
- **F6** — Week view, no override: `week` shows target `40h 00m`
  from an absent `week_targets` row.
- **F7** — Target override then read-back: `week target 2026-07
  33h30m` prints `Week 2026-07 target: 33h 30m`, then `status`/`week`
  during that week reflect the new
  target in the fulfillment, `required` and closed-week `owed` math.
- **F7b** — Target override of `0`: legal, that week's target is
  `00h 00m` and the week ends however far ahead its full worked total
  puts it (no deficit possible against a zero target).
- **F8** — Multi-week carry: seed two consecutive weeks of data,
  confirm `week`'s carry-in/fulfillment/closed-week `owed` on the second matches
  the §5 formula by hand.
- **F8b** — Carry across an idle gap week (tests the §2.4 fix): week
  N has data, week N+1 has *zero* punches, week N+2 has data —
  confirm week N+2's `carry_in` reflects week N+1's full deficit, not
  a skip straight from N to N+2.
- **F9** — Estimated EOD, all three states: open stint with quota
  remaining (shows a clock time), open stint with quota already met
  (shows `done for today`), no open stint (line omitted). Stability:
  with the same punches, two `status` runs at different `now`s (e.g. 30
  minutes apart) print the same `est. EOD` while the week line's figure
  drops by the minutes elapsed.
- **F9b** — Week line with a large carry-in: seed a week with a
  carry-in big enough that `required − fulfillment` is already
  negative on day 1 despite day total being small/zero; confirm the
  week line (and est. EOD, if an open stint) reads off *fulfillment*
  and switches to the "X over `<required>`" wording (or `done
  for today` for est. EOD) independently of day total, while the
  plain "Day total" figure right next to it is unaffected. Also cover
  the over-by-live case: an open stint whose live minutes exceed
  `required − fulfillment` flips the week line to the "X over" wording
  and est. EOD to `done for today` with no punch change.
  Also cover a Friday-through-Sunday `status`: `required` is the full
  target and the week line reads `X left to <target> target` or `X
  over <target> target`. Include a target override that isn't a
  multiple of 5 minutes (e.g. `33h 31m`) and confirm `required` is the
  override itself, not rounded down to a multiple of 5 minutes.
- **F10** — `status DATE` for a past date in a different, closed week:
  no est.-EOD figure, week line uses the plain `Total
  behind`/`Total ahead` form for *that* week, not today's (§3.5,
  §7.1's second example).
- **F11** — `status DATE` for a different day *within the current,
  still-open week* (e.g. viewing Monday's status on Thursday): week
  line still uses the current-week framing, keyed to today's actual
  weekday, not `DATE`'s, and counts today's live minutes while the Day
  total for `DATE` does not.
- **F12** — Punches either side of a DST transition on different
  dates: each displays in the local offset that applied on its own
  date (§2.1) — a naive "single current offset" implementation should
  fail this.
- **F13** - First run on a fresh database prints nothing on stderr,
  except that a first `mlm stop` warns about its orphaned end (§7.4,
  E27). With `--verbose` the migration line and debug lines appear (§3).
- **F14** - Week notes (§3.6, §7.2): `mlm note "fixed migration runner
  bug"`, then `mlm note "started punch pairing tests"` on one date, then
  `mlm week --notes` for its week prints both under that date's row, in
  that order, each as six spaces, `- ` and the body. Other rows stay one
  line, and every figure, marker and footer matches plain `mlm week`.
  `mlm week -n WEEK_ID` and `mlm week WEEK_ID --notes` print the same.
  Plain `mlm week` prints no note lines.
- **F15** - Export notes (§3.8, §7.5): two notes on 2026-10-19 in that
  entry order, one on 2026-10-21 and one on 2026-10-24, then `mlm
  export notes --week 2026-43` prints the four lines of §7.5's example.
  `--date 2026-10-19` prints that date's two lines, and `--month
  2026-10` prints all four. Notes dated the day before a range's first
  day or the day after its last day never appear. No range flag prints
  the current week's notes and none from the weeks either side. Two
  notes on one date whose later `id` has the earlier `created_at_utc`
  print in `created_at_utc` order. `mlm ex n -w 2026-43` prints the
  same as the long form, and a non-ASCII body passes through unchanged.

### 8.2 Error / edge paths (§6 direct mapping)

- **E1** — Malformed `TIME`: `25:00`, `24:00`, `9:75`, `abc` → hard
  error, no punch written.
- **E2** — Malformed `DATE`: `2026-02-30`, `13/02/2026` → hard error.
- **E3** — Malformed `WEEK_ID`: `0`, `abcd`, and a week number that
  doesn't exist for its year (e.g. `2027-53` if 2027 has only 52 ISO
  weeks) → hard error. `2026-7` (unpadded) is accepted and normalized
  to `2026-07`, not rejected.
- **E4** — Malformed `DURATION`: `10` (no unit), `-5h`, `10x` → hard
  error. `0h` is legal (F7b), only negative is rejected.
- **E5** — Empty/whitespace-only `NOTE` → hard error (checked before
  the trim, §2.3/§6.1). A padded-but-non-empty note (`"  did a
  thing  "`) is accepted and stored trimmed.
- **E6** — DB unreachable (bad permissions/corrupt file) on any
  command → fatal abort, nonzero exit, nothing written. Distinct from
  first run (missing app-data dir/db file), which bootstraps silently
  rather than erroring (§6.1).
- **E7** — Multiple dangling `start`s same day (fat-finger case,
  §4.3): punches all still stored; `status` lists each as its own
  open/ongoing stint and flags the multi-open anomaly, now with
  `fix:`/`or:` lines naming the delete/stop commands to resolve it
  (§7.3). The `or:` hint names a window when a punch follows the latest
  open start (§7.3), and the anomaly block and `status` list
  overlapping nested stints as their own anomaly (E21).
- **E8** — Orphaned `end` (empty-stack case, §4.3): the punch is
  stored but produces no stint-list line of its own — it only
  surfaces as an anomaly line naming its timestamp, now with `fix:`/
  `or:` lines naming the start/delete commands to resolve it (§7.3).
  Two or more orphaned ends on the same date each get their own line.
  A `stop` to that date also warns about each one on stderr (E27).
- **E9** — `week target` with a negative `DURATION` → hard error;
  zero is not an error (F7b).
- **E10** — `week target` missing the `DURATION` argument entirely →
  parser missing-argument error whose usage names `[WEEK_ID]
  <DURATION>` and never `<ARGS>`, exit `2`.
- **E11** — `status` for a date with zero punches *and* zero notes:
  both the stint-list and Notes sections omitted (F4's rule applied
  to both blocks at once).
- **E12** — First week ever tracked: `carry_in = 0`, no prior week to
  walk from (already covered by §5's "first tracked week" note).
- **E13** — `week` requested for a week with no rows at all (never
  touched, past or future): still renders all 7 dates at `00h 00m`,
  default target, and carry computed by walking the full sequence
  from the earliest data week through it (§2.4).
- **E14** — A `start`/`end` pair at the identical instant: legal,
  zero-length stint (`00h 00m`), not flagged as an anomaly (§4.3).
- **E15** — A session crossing midnight (`start 23:30`, `stop 00:45`
  the next day): splices into one clean completed stint on the earlier
  date (§4.3.1) when the shape is unambiguous (one open `start` that
  date, one orphaned `end` — the next date's first punch — on the
  next). The later date shows nothing for it: no stint, no anomaly, no
  footnote (§4.3.1). Outside
  that exact shape — e.g. two dangling starts on the earlier date —
  the join is refused: both dates flag a `[!]` line naming each other
  (§7.3/E17), at any age, alongside whatever E7/E8 anomaly the shape
  itself produces (two dangling starts is E7 on the earlier date; an
  extra orphaned end is E8 on the later one).
- **E16** — A lone open `start` at least 2 calendar days old (§4.3):
  flagged as a stale-open-start anomaly, distinct from the ordinary
  open-stint case at age 0-1, with `fix:`/`or:` lines pointing at
  `stop`/`delete punch` (§7.3). A date with two or more open starts
  still gets only the E7 line, never an extra stale line. With a later
  punch after the start, the `fix:` time is a window, `(a time between
  09:00 and 10:30)` (§7.3).
- **E17** — A refused join (§4.3.1): adjacent dates where the earlier
  has an open start and the later an orphaned end, but not in the
  exact 1:1, first-punch splice shape. Both dates flag a `[!]` line
  naming the other date (§7.3), with no `fix:` lines of their own,
  since the E7/E8/stale lines on those dates already carry them. A
  clean splice (E15) produces no refused join.
- **E18** — Today's lookback (§7.1): a plain `status` for today
  (`DATE` omitted, or given explicitly) additionally lists dates 2 to
  14 days old that still have an open start, and yesterday when it has
  two or more open starts, oldest first, each as its own `[!]` line
  with `fix:` lines. `status DATE` for any other date runs no lookback.
  A date whose open start is 15 or more days old falls outside the
  window and isn't listed (§7.1), though the carry-in marker still
  flags its week (E23).
- **E19** — `mlm start`'s stale-start warning (§7.4): a successful
  `start`, today or backdated, prints one warning per stale date found
  by the same 14-day lookback to stderr, oldest first; stdout stays
  silent and exit stays `0`. The date just written to draws no warning
  only when its single open start is the exact punch just inserted —
  two or more open starts there, or a single older open start left
  behind by the new punch pairing off an existing orphaned end, still
  warn. A single unjoined open start dated yesterday warns too, with no
  `or:` line (§7.4). `start` prints no carry-in marker.
- **E20** - `mlm week` read failure (§6.1, §7.2): an unreadable
  `week_targets` table, `notes` table, or punch range makes `mlm week`
  and `mlm week WEEK_ID` print `error: ...` to stderr, nothing to
  stdout, and exit `1`. `status` does the same. Successful output is
  unchanged. `mlm week --notes` also reads the shown week's notes, so a
  renamed `notes.body` column fails it the same way. Plain `mlm week`
  on the same database still prints, because it makes no note read.
- **E21** - Overlapping stints (§4.3, §7.3): `start 09:00`, `start
  10:00`, `stop 11:00`, `stop 12:00` pairs into 09:00-12:00 and
  10:00-11:00. The date's `status` shows the overlap line, its `week`
  row gets `[!]` and the footer prints once. The day and week totals
  still count the overlap twice. Stints that only touch at an endpoint
  and zero-length stints are not flagged.
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
- **E23** - Carry-in marker (§7.1, §7.2): an anomaly in a week older
  than 14 days puts the marker line on `status` and on `mlm week`, with
  `[!]` on the Carry-in figure. Several anomalous weeks give the plural
  form naming the oldest. The marker never counts the shown week's own
  anomalies and prints nothing when no earlier week has one.
- **E24** - Argument order (§3.7, §6.1): `mlm week 2026-41 target 20h`
  prints `error: put WEEK_ID after target: mlm week target 2026-41 20h`,
  no `Usage:` block, exit `2`. `mlm week 7 foo` keeps the parser's own
  error.
- **E25** - Future `status DATE` (§6.1): `status` with a date after
  today exits `1` with the future-date message. `mlm week` for a
  future week still prints.
- **E26** - Overnight stint in `week` (§7.2): while the hint prints,
  yesterday's row ends with `  [i]` and the `[i] open stint on DATE:
  mlm status shows how to stop it` footer prints once, after the `[!]` footer and
  before the carry-in marker line. A row with an anomaly too ends `  [!]
  [i]` and both footers print. A row with only `[i]` prints no `[!]`
  footer. On a Monday the current week's view has no marked row and
  prints `[i] open stint on Sun DATE (last week): mlm status shows how
  to stop it`, last week's view marks the Sunday row, and no other week prints
  anything. No `[i]` mark or footer appears when the start is joined,
  refused-join, doubled, or two days old.
- **E27** - `mlm stop`'s orphaned-end warning (§7.4): a successful
  `stop`, today or backdated, classifies the date it wrote to with the
  splice applied and prints one `warning: orphaned end on <Wdy
  YYYY-MM-DD> at <HH:MM> (no matching start)` block per orphaned end
  there, ascending, each with `status`'s E8 `fix:`/`or:` lines. Stdout
  stays silent and exit stays `0`. The FE5 case (a start two or more
  days old, then a bare `stop` today) warns. So does an end the new
  punch displaced (`start 09:00`, `stop 17:00`, `stop 12:00` warns for
  17:00) and an orphan already on the date. A clean pair and an end that
  splices with the previous date's open start stay silent. A read
  failure after the write prints nothing, and the punch is stored.
- **E28** - `mlm export notes` ranges (§3.8, §6.2): `--month 2024-02`
  covers the 1st to the 29th, `--month 2026-02` the 1st to the 28th,
  and a December runs to the 31st. `--month 2026-3` equals `--month
  2026-03`. A bare `WW` uses the current ISO year and a bare `M`/`MM`
  the current calendar year, so on 2025-12-30 `-m 1` is `2025-01`.
  `-d -1` is yesterday. A range with no notes, and a future date, week
  or month, print nothing and exit `0`.
- **E29** - `mlm export notes` value errors (§6.1): `-m 2026-003`,
  `-m 26-03`, `-m 2026-` and `-m -1` fail with the shape message, e.g.
  `-m abc` prints `error: invalid MONTH "abc": expected YYYY-MM or MM`.
  `-m 0`, `-m 13` and `-m 2026-13` fail with the range message, e.g.
  `-m 13` prints `error: invalid MONTH "13": month must be 1-12`. A bad
  `DATE` and a bad `WEEK_ID`, `-w -1` included, print their existing
  messages, and `-m -d` fails as a bad `MONTH`. Each exits `1` with
  nothing on stdout.
- **E30** - `mlm export notes` usage errors (§3.8, §6.3): bare `mlm
  export` (its output lists `notes`), two range flags such as `-d
  2026-10-19 -w 2026-43`, one flag given twice such as `-m 3 -m 4`, and
  a range flag with no value as the last token each exit `2` from the
  argument parser.
- **E31** - `mlm export notes` read failure (§6.1): with the
  `notes.body` column renamed, it prints `error: ...` to stderr,
  nothing to stdout, and exits `1`.

