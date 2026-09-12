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

### 1.1 Goals (MVP)

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

### 1.2 Non-goals (MVP — deferred/stretch, see `NOTES.md`)

- Editing or deleting punches/notes after entry.
- Project tagging on notes/stints.
- Terminal dashboard (ratatui) — deps are in, UI is not.
- Shell prompt integration.
- Non-ISO week conventions.
- 12h (AM/PM) time input — MVP is `HH:MM`/`HHMM`/`HH`, 24h only.
- `+N`/`-N` relative day/week notation for `status`'s date argument
  and `week`'s week-id argument.
- Punches for anything but *today* — no way to log a forgotten punch
  against yesterday in MVP (no editing means no fixing a wrong date
  either, so this stays add-only-for-today until editing lands).
- Stints spanning midnight: pairing is strictly per calendar `date`
  (§4.3), so a session like `start 23:30` / `stop 00:45` the next day
  splits into two anomalies (a permanently-open `start` on day one, an
  orphaned `end` on day two) rather than one clean overnight stint.
  Accepted as a known MVP limitation — fixing it means pairing across
  date boundaries, a real complexity jump for a rare case.

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
  `2026-07`. Always Monday-start for MVP.
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

### 2.2 Migrations

A `schema_migrations` table (or equivalent) tracks applied versions.
Uses the [`rusqlite_migration`](https://docs.rs/rusqlite_migration)
crate — ordered SQL migrations embedded in the binary, applies
whatever's pending on `connect()` — instead of a hand-rolled runner.

Single-user, single-machine tool — concurrent access from two `mlm`
invocations at once is out of scope for MVP; SQLite's default locking
is trusted to fail safely (as a §6.1 DB error) rather than corrupt
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
| `body` | `TEXT NOT NULL` | free text, trimmed of leading/trailing whitespace before storage (§6.1); project-name prefix stays *in* the text for MVP (no `project` column — that's the deferred stretch, adding it later is a plain migration); no length cap or charset restriction — the plain-ASCII rule in §7 is about layout characters in *rendered* output, not what a user can type into a note |
| `created_at_utc` | `TEXT NOT NULL` | insertion-order tiebreaker for same-day notes |

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
  **completed stints only** — a currently-open stint's live minutes
  are never folded into any total/carry/owed computation (consistent
  with §7.1's day-total rule); the open stint contributes only to its
  own displayed line and the estimated-EOD figure, nowhere else. An
  **orphaned `end`** (§4.3) contributes nothing to any total either —
  it has no paired `start` to derive a duration from, so it shows up
  only as its own flagged anomaly line, never as time in a day/week
  sum.
- **Daily target**: `status`-only pace hint, `today's week target ÷
  5` (floor to the minute). Purely derived from the week's target —
  no override, no storage, no interaction with carry. Used for the
  "X left to daily target" and estimated-EOD figures (§7.1).

## 3. CLI surface

All commands operate on the local machine's current date/time unless
a command explicitly takes a date/week argument. Typed times/dates are
interpreted in local time and converted to UTC on write (§2.1); output
converts back to local.

### 3.1 Time input format

Accepted for any `TIME` argument, 24h only (§1.2):

| form | example | meaning |
|---|---|---|
| `HH:MM` | `9:05`, `17:30` | hour and minute |
| `HHMM` | `0905`, `1730` | hour and minute, no separator |
| `HH` | `9`, `17` | hour, minute defaults to `:00` |

### 3.2 `mlm start [TIME] [NOTE]`

Insert a `start` punch for today.

- `TIME` optional (§3.1). Defaults to now.
- `NOTE` optional, free text — if given, also inserts a note row for
  today in the same call (convenience for "starting work on X").
- No chronology requirement: a start punch can be inserted at any
  time value relative to existing punches for the day.

### 3.3 `mlm stop [TIME] [NOTE]`

Same shape as `start`, inserts an `end` punch.

### 3.4 `mlm note NOTE`

Insert a work-log note for today, independent of punches — for
end-of-day or next-day notes with no punch attached.

### 3.5 `mlm status [DATE]`

Today's (or `DATE`'s, `YYYY-MM-DD`) view:

- that date's stints (derived), each as `start–end (duration)`, with
  an open stint shown as `start–now (duration, ongoing)`
- that date's notes, in insertion order
- date total
- **the week containing `DATE`**'s fulfillment so far, target, and
  time still owed — not always "this week": a `status` for a date two
  weeks ago shows *that* week's numbers next to it, not today's.
  (Querying a past date's own stints next to today's unrelated week
  totals was the original, confusing behavior — fixed here.)
- the daily-target pace hint and estimated-EOD (§7.1) appear only
  when `DATE` is actually today — both depend on "day total so far"
  and "now," which are meaningless for any other date
- any pairing anomalies for that date (§4.3), if present

### 3.6 `mlm week [WEEK_ID]`

Current (or `WEEK_ID`) week's view:

- one line per date in the week with that date's total
- week total, carry-in, target, fulfillment, still owed

`WEEK_ID` accepts either a full id (`YYYY-WW`, e.g. `2026-07`) or a
bare week number (`WW`, e.g. `7`), which defaults the year to the
current one.

### 3.7 `mlm week target [WEEK_ID] DURATION`

Set an absolute target override for a week. `WEEK_ID` (same accepted
forms as §3.6) defaults to the current week when omitted. `DURATION`
uses the human duration format (§4.2), not raw minutes — e.g. `20h`,
`33h30m` — so setting a target has the same friction as everything
else here: no mental-math input.

## 4. Time/duration semantics

### 4.1 Parsing and conversion

A `TIME` argument (§3.1) is parsed as a local wall-clock hour/minute,
combined with local *today* (the only date `start`/`stop` ever target
in MVP, §1.2) to form a local datetime, then converted to UTC for
storage per-instant (§2.1). There is no seconds precision anywhere —
everything is minute-granular, per NOTES.md's "keep time in minutes"
call.

### 4.2 Duration formatting

One display rule everywhere, tabular or standalone alike: `HHh MMm`,
both sides zero-padded, hour part never dropped even at zero (`07h
45m`, `00h 20m`) — this is what keeps a column of these lined up, and
there's no reason for status output to look different from the week
table right next to it. A negative value (ahead-of-target `owed`, a
surplus `carry_out`) keeps the same padding with the sign in front:
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

1. Sort the date's punches by `at_utc` (ties broken by `id`, i.e.
   insertion order).
2. Scan in that order keeping a stack of unmatched `start`s: a
   `start` pushes; an `end` pops the *most recently pushed* unmatched
   `start` and pairs with it, forming a stint.

Worked example (matches the motivating case): starts entered at
`09:00` and `14:00`, ends entered at `18:00` and `13:00`, in that
entry order. Sorted by time: `09:00(start), 13:00(end), 14:00(start),
18:00(end)`. Scanning: `09:00` pushes; `13:00` pops it → stint
`09:00–13:00`; `14:00` pushes; `18:00` pops it → stint `14:00–18:00`.
Entry order never mattered, only time order.

This also correctly handles legitimately nested entry (e.g. two
`start`s before either `end`: the second `start` pairs with the
*first* subsequent `end`, nearest first) without treating it as
malformed.

Edge cases, still just **detected and surfaced in that date's
summary** (§3.5/§3.6 output), not silently fixed and not rejected at
insert time (no editing/validation in MVP — see §1.2):

- **One unmatched trailing `start`** (stack has exactly one item once
  the date's punches are exhausted): the normal open/ongoing stint
  (§1.3), not an anomaly.
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
  matched-parentheses pairing doesn't care about ordering *within* a
  tie, only that one exists (§4.3 step 1's tie-break by `id` still
  applies to the sort, but doesn't change that both punches pair up
  cleanly).

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

A negative `owed` for the *current, still-open* week just means
already-ahead-of-pace — nothing is capped or clamped anywhere in this
table; every value can legitimately go negative or exceed target in
either direction.

`status`'s "time still owed" (§3.5) is exactly this week's `owed`
value evaluated as of now — no separate day-by-day pacing formula
(e.g. spreading target evenly across weekdays); the original
process's "how much I should still put in today" and this week-level
`owed` are the same question once fulfillment is tracked continuously.

## 6. Error handling & validation

Two tiers, kept distinct on purpose: **hard errors** reject the
command outright (nonzero exit, nothing written, message on stderr);
**anomalies** (§4.3) are accepted, stored, and surfaced later in
`status`/`week` output instead — because MVP has no editing, refusing
to store a punch that merely produces a weird pairing would leave the
user with no way to fix it.

### 6.1 Hard errors (reject, no write)

- Malformed `TIME` (§3.1): doesn't match `HH:MM`/`HHMM`/`HH`, or an
  out-of-range hour/minute (`25:00`, `9:75`).
- Malformed `DATE` (`YYYY-MM-DD`): wrong shape or an invalid calendar
  date (`2026-02-30`).
- Malformed `WEEK_ID` (§3.6): not a bare week number or `YYYY-WW`
  shape, or a week number that isn't a valid ISO week for its year
  (most years have 52, some have 53 — `2027-53` is invalid if 2027
  only has 52; this is a calendar-validity check, not a flat
  `1..=53` range test). An unpadded number is accepted and normalized
  (`2026-7` parses the same as `2026-07`), consistent with how
  `TIME`/`DURATION` input is lenient about padding elsewhere.
- Malformed `DURATION` (§3.7/§4.2): doesn't match the input grammar,
  or parses to a negative number of minutes. Zero is legal (a
  deliberate week off, see §2.3's `week_targets` note) — only
  negative is rejected.
- Malformed `TIME` boundary: `24:00` is rejected, not accepted as a
  next-day-midnight alias — valid range is `00:00` through `23:59`.
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

### 6.2 Not errors

- A `status`/`week` target (date or week) with no punches/notes at
  all: valid, just renders as empty for that scope.
- Any §4.3 pairing anomaly (extra open stints, an orphaned `end`): the
  punch is still inserted; the anomaly shows up in that date's next
  `status`/`week` output, not at insert time.
- A week with no `week_targets` row: valid, uses the default target
  (40h) rather than erroring for "unset".

### 6.3 Exit codes

`0` on success (including a status view that reports anomalies — the
anomaly is data to look at, not a tool failure). Nonzero on any §6.1
hard error.

## 7. Output layout

Plain ASCII throughout (no box-drawing/unicode dashes) — matches the
portability goal behind the ratatui pick (README) even though this is
plain `println!` output, not a TUI screen.

### 7.1 `mlm status`

`DATE` omitted (today), so the daily-target/est.-EOD lines apply and
the week shown is the actual current week:

```
Thu 2026-02-12

Day total:     07h 25m (+ ongoing), 00h 35m left to 08h 00m daily target, est. EOD 18:35
Week 2026-07:  10h 45m left by end of Thursday (fulfillment 29h 15m / target 40h 00m)

  09:00-13:00  (04h 00m)
  14:05-17:30  (03h 25m)
  17:45-now    (00h 15m, ongoing)

Notes:
  - fixed migration runner bug
  - started punch pairing tests
```

- Header: `<weekday abbrev> <YYYY-MM-DD>`.
- **Day total** and **week** lines lead the output, ahead of the
  stint list — the "how much is left" figures are the point of a
  status check, not something to hunt for at the bottom.
- Day total is a tabular-format sum; ongoing time isn't folded into
  it live (avoids the total silently changing mid-read) — `(+
  ongoing)` just flags that an open stint isn't counted yet.
- "X left to `<daily target>`" is a **display-only pace hint**, not a
  stored/independent target: `daily target = today's week target ÷ 5`
  (floor to the minute), always derived, never overridden on its own
  (§2.4). Negative once the day total already meets/exceeds it — shown
  the same signed way as any other summary value (§4.2).
- **Estimated EOD** (`est. EOD HH:MM`) appears only when today has an
  open stint: it's `now + (daily target − day total)`, i.e. "if you
  keep going from right now, this is the clock time you'd hit today's
  quota." Omitted entirely when there's no open stint (nothing to
  project from) or replaced with `target already met` when the gap is
  already zero or negative.
- Week line reports the week containing `DATE` (§3.5), and its
  framing follows the same ongoing-vs-not split as `mlm week` (§7.2):
  "`<owed>` left by end of `<weekday>`" when that week is the actual
  currently-ongoing week (using *today's* weekday, even if `DATE`
  itself is some other day within that same week — the deadline is
  always about today, not about which day's stints you're viewing),
  or the plain `Total still owed`/`Total ahead` form when `DATE`'s
  week is a past or future one.
- Stint lines: `HH:MM-HH:MM  (duration)`, ongoing stint's end is the
  literal word `now`. Tabular duration format (§4.2). Section omitted
  entirely when the date has zero stints (a note-only day, or a fully
  empty one) — same treatment as Notes below, no empty-list
  placeholder either way.
- Notes render only if any exist for the date (section omitted
  otherwise, not shown empty).
- Anomalies (§4.3), if any, print between the two lead lines and the
  stint list — see §7.3.

`mlm status 2026-01-05` (a past date, different week, "today" is
still `2026-02-12`) — no daily-target/est.-EOD lines, and the week
line uses the plain closed-week form since `2026-01-05`'s week has
already ended:

```
Mon 2026-01-05

Day total:     06h 15m
Week 2026-02:  Total still owed: 01h 40m

  08:30-14:45  (06h 15m)
```

### 7.2 `mlm week`

Current (ongoing) week — this example, `2026-07`, is the week
containing "today" (`status`'s `2026-02-12`, a Thursday):

```
Week 2026-07 (2026-02-09 - 2026-02-15)

10h 45m left by end of Thursday

  Mon 2026-02-09   08h 10m
  Tue 2026-02-10   07h 50m
  Wed 2026-02-11   08h 00m
  Thu 2026-02-12   07h 25m (ongoing)
  Fri 2026-02-13   00h 00m
  Sat 2026-02-14   00h 00m
  Sun 2026-02-15   00h 00m

Carry-in:      -02h 10m
Worked:        31h 25m
Fulfillment:   29h 15m
Target:        40h 00m
```

A **past or future** week has no "today" to frame a deadline against,
so its headline is a plain total instead — everything else is
identical in shape, same per-day rows and the same
carry-in/worked/fulfillment/target block:

```
Week 2026-06 (2026-02-02 - 2026-02-08)

Total still owed: 03h 10m

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

- Header names the week id and its Mon-Sun date span.
- Headline is the same "owed" figure `status`'s week line shows
  (§7.1), just leading its own output here instead of being inline —
  worded with the weekday-deadline framing only for the current week;
  a closed or not-yet-started week gets the plain `Total still owed`/
  `Total ahead` form (NOTES.md decision, this round).
- One row per calendar date in the week, always all 7 even if some
  are empty (`00h 00m`) — consistent shape, easy to scan for gaps.
  `(ongoing)` marks a date with a currently-open stint (only possible
  on today's row, and only when the requested week is the current
  one).
- Field order below the headline is fixed: carry-in, worked,
  fulfillment, target — `still owed` isn't repeated down here since
  the headline already states it plainly.
- Any date in the row list with a §4.3 anomaly gets a marker (§7.3)
  appended to its row rather than a separate block, since the week
  view is already one-line-per-date.

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
  33h30m`, then `status`/`week` during that week reflect the new
  target in fulfillment/owed math.
- **F7b** — Target override of `0`: legal, that week's target is
  `00h 00m` and the week ends however far ahead its full worked total
  puts it (no deficit possible against a zero target).
- **F8** — Multi-week carry: seed two consecutive weeks of data,
  confirm `week`'s carry-in/fulfillment/owed on the second matches
  the §5 formula by hand.
- **F8b** — Carry across an idle gap week (tests the §2.4 fix): week
  N has data, week N+1 has *zero* punches, week N+2 has data —
  confirm week N+2's `carry_in` reflects week N+1's full deficit, not
  a skip straight from N to N+2.
- **F9** — Estimated EOD, all three states: open stint with quota
  remaining (shows a clock time), open stint with quota already met
  (shows `target already met`), no open stint (line omitted).
- **F10** — `status DATE` for a past date in a different, closed week:
  no daily-target/est.-EOD lines, week line uses the plain `Total
  still owed`/`Total ahead` form for *that* week, not today's (§3.5,
  §7.1's second example).
- **F11** — `status DATE` for a different day *within the current,
  still-open week* (e.g. viewing Monday's status on Thursday): week
  line still uses the deadline framing, keyed to today's actual
  weekday, not `DATE`'s.
- **F12** — Punches either side of a DST transition on different
  dates: each displays in the local offset that applied on its own
  date (§2.1) — a naive "single current offset" implementation should
  fail this.

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
  open/ongoing stint and flags the multi-open anomaly.
- **E8** — Orphaned `end` (empty-stack case, §4.3): the punch is
  stored but produces no stint-list line of its own — it only
  surfaces as an anomaly line naming its timestamp (§7.3). Two or
  more orphaned ends on the same date each get their own line.
- **E9** — `week target` with a negative `DURATION` → hard error;
  zero is not an error (F7b).
- **E10** — `week target` missing the `DURATION` argument entirely →
  clap-level missing-argument error.
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
  the next day): produces two anomalies, not one clean stint — an
  open `start` on day one, an orphaned `end` on day two. Accepted
  MVP limitation (§1.2), not a bug to fix.

---

*Spec complete through MVP scope, flow-mapped and adversarially
reviewed per NOTES.md's final-review note. Ready for TDD
implementation + independent adversarial code review, per the
workflow note at the top of NOTES.md.*
