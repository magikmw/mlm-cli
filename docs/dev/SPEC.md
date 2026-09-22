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

Deliberately out of scope: future features not committed to, not
things known to be broken or confusing today. See §1.2a for that.

- Editing a punch/note after entry (deleting is implemented — `mlm
  delete note|punch`; the correction path is delete-then-recreate, not
  in-place edit).
- Project tagging on notes/stints.
- Terminal dashboard (ratatui) — deps are in, UI is not.
- Shell prompt integration.
- Non-ISO week conventions.
- 12h (AM/PM) time input — MVP is `HH:MM`/`HHMM`/`HH`, 24h only.
- `+N`/`-N` relative week notation for `week`'s week-id argument.
  (`status`'s `DATE` argument's own `-N` shorthand, and logging a
  punch/note against a date other than today, are no longer non-goals —
  both are implemented.)

Stints spanning midnight, and a same-instant `end`/`start` boundary
between two real stints, were both non-goals through this point in the
project's history — both are now fixed; see §4.3.

### 1.2a Known issues to revisit

Shipped, working-as-designed behavior that's rough or confusing in a
way worth fixing later. Kept separate from §1.2 because a "known
issue" reads very differently to a user hitting it than a "non-goal"
does: one is "we haven't built this yet," the other is "this works,
but expect a rough edge here." Originally surfaced by a first-time-user
UX pass run against the boundary-stint-pairing changeset (several
predated that changeset and were simply never written down before);
the boundary-context-cues changeset fixed four of those bullets (see
`docs/dev/plans/reports/boundary-context-cues-*`) and its own
fresh-eyes UX check surfaced five more, listed below.

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
- `status`/`week`'s output for a past (closed) date/week uses a
  different sentence shape (`Total still owed: Xh Ym`, no
  fulfillment/target breakdown) than the current-week form the README
  only ever shows examples of (`X left by end of <weekday>
  (fulfillment.../target...)`) — a user who has only read the README's
  examples hits an undocumented format the first time they check a
  past date/week.
- `est. EOD HH:MM` (§7.1) can point at tomorrow with no date shown —
  "EOD" reads as "later today," but the shown clock time can require
  working through the night into the next calendar date, and nothing
  in the string distinguishes the two.
- On the last weekday of the ISO week, the day-total pace hint
  ("... required by end of `<weekday>`") and the week line ("...
  left by end of `<weekday>`") quote the identical figure with
  different introductory phrasing — correct, but reads as a redundant
  repeated number on that one day.
- "`Total still owed`" (the closed-week/closed-date headline, §7.1/§7.2)
  reads as punitive/debt-like for a week that simply ended under
  target, inconsistent with the tool's otherwise neutral vocabulary
  ("fulfillment," "carry-in," "shortfall/surplus" per the README).

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
| `body` | `TEXT NOT NULL` | free text, trimmed of leading/trailing whitespace before storage (§6.1); embedded `\r`/`\n` are collapsed to a single space rather than preserved verbatim, so a stored body is always exactly one line (this collapse never affects whether a body counts as empty — that check happens first, against the pre-normalization text, and rejects a whitespace-only body, including one that's only newlines, before either step touches it, §6.1); project-name prefix stays *in* the text for MVP (no `project` column — that's the deferred stretch, adding it later is a plain migration); no length cap or charset restriction beyond that newline collapse — the plain-ASCII rule in §7 is about layout characters in *rendered* output, not what a user can type into a note |
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
  **completed stints only** — a currently-open stint's live minutes
  are never folded into any total/carry/owed computation (consistent
  with §7.1's day-total rule); the open stint contributes only to its
  own displayed line and the estimated-EOD figure, nowhere else. An
  **orphaned `end`** (§4.3) contributes nothing to any total either —
  it has no paired `start` to derive a duration from, so it shows up
  only as its own flagged anomaly line, never as time in a day/week
  sum.
- **Daily target**: `today's week target ÷ 5` (floor to the minute).
  Purely derived from the week's target — no override, no storage.
  Used only as the per-weekday increment for **required-by-day**
  below; never displayed or compared on its own.
- **Required-by-day**: `status`-only pace hint, `daily target ×
  min(today's ISO weekday number, 5)` — Mon=1 … Fri=5 (same numbering
  as the ISO week machinery already in use elsewhere, e.g. `week.rs`),
  Sat/Sun both pin to 5. That pin is `5 × daily target`, i.e. `5 ×
  floor(week target / 5)` — the week has no more workdays to spread
  the target over by then, so the requirement stops growing, but it
  is **not always exactly equal to** the week's own target: a target
  override that isn't a multiple of 5 minutes loses up to 4 minutes
  to the floor, same as any other day's `daily target`, so a Sat/Sun
  `required` can sit a few minutes under the full target in that case.
  Compared against the week's **fulfillment**
  (`carry_in + worked`, §2.4 above) as of now — i.e. carry-in *does*
  count here, same fulfillment figure the week-level `owed` uses, just
  measured against a smaller, day-scoped slice of the target instead
  of the whole week. Used for the "X left to/over `<required>`
  required by end of `<weekday>`" and estimated-EOD figures (§7.1).

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
- Positional order is strict and never sniffed: `TIME`, when given, is
  always the first positional argument. A value in that position that
  fails to parse as `TIME` is a hard error (§6.1, E1) — it is never
  silently reinterpreted as `NOTE` text. A note-only invocation with
  no `TIME` is `mlm note` (§3.4), not a single positional guessed at.
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

- the week's still-owed figure as headline framing (§7.1/§7.2)
- one line per date in the week with that date's total
- a trailing block, in this order: carry-in, worked, fulfillment,
  target (§7.2) — "still owed" appears only in the headline, not
  repeated here

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
  this is exactly why step 1's tie-break sorts `start` before `end`
  at a shared instant, regardless of entry order, so the pair always
  matches cleanly rather than depending on which was typed first.

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
ordinary case. This residual case is the known rough edge here — the
gate's own on/off condition is still undocumented in `--help`/`status`
(§1.2a).

A spliced stint is not indistinguishable from an ordinary same-date
one: the earlier date's stint line gets a `, spans to next day` suffix
(§7.1), and the receiving date's header gets a `(HH:MM continues
previous day's stint)` suffix (§7.1) — added by the boundary-context-cues
changeset, see `docs/dev/plans/reports/boundary-context-cues-*`.

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
value evaluated as of now, and is distinct from the required-by-day
pace hint (§2.4, §7.1): `owed` measures fulfillment against the
*whole* week's target and is only truly "due" at week's end, while
required-by-day measures the same fulfillment against a day-scoped
slice of that target — spreading it evenly across weekdays, per the
original manual process's "how much I should still put in today"
(NOTES.md). The two share the fulfillment number but not the target
they're measured against, so they can and do disagree mid-week (e.g.
comfortably under the week target while already behind today's
slice, or vice versa) — both are correct answers to different
questions, not a contradiction.

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

Day total:     07h 25m (+ ongoing), 02h 45m left to 32h 00m required by end of Thursday, est. EOD 20:45
Week 2026-07:  10h 45m left by end of Thursday (fulfillment 29h 15m / target 40h 00m)

  09:00-13:00  (04h 00m)
  14:05-17:30  (03h 25m)
  17:45-now    (00h 15m, ongoing)

Notes:
  - fixed migration runner bug
  - started punch pairing tests
```

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
- Day total is a tabular-format sum; ongoing time isn't folded into
  it live (avoids the total silently changing mid-read) — `(+
  ongoing)` just flags that an open stint isn't counted yet, unless
  that open stint is two or more calendar days old, in which case the
  suffix is `(+ unclosed)` instead, matching the stint line's own
  wording change below — a stale, probably-forgotten punch reads
  differently from a live one.
- "X left to `<required>` required by end of `<weekday>`" (fulfillment
  under `required`) or "X over `<required>` required by end of
  `<weekday>`" (fulfillment at/above it) is a **display-only pace
  hint** (§2.4), not a stored/independent target: `required = daily
  target × min(today's ISO weekday number, 5)`, always derived, never
  overridden on its own. `gap = required − fulfillment` (`carry_in +
  worked`, the same fulfillment the week line's `owed` uses, §2.4/§5)
  — carry-in counts here, unlike the old day-total figure it's
  compared next to. `X` is always `gap`'s absolute value; which of the
  two words prints is what carries the sign — a word switch, not an
  inline `-` (matching decision 18's closed-week "Total still owed" /
  "Total ahead" split, §7.2, rather than §4.2's bare-signed-number
  convention used elsewhere). `gap == 0` counts as "left to" (X =
  `00h 00m`), matching every other "reached exactly" case in this
  spec. This gap is driven by *fulfillment*, not by the "Day total"
  figure printed right before it on the same line —
  the two can point opposite directions mid-day (e.g. day total still
  climbing while the pace hint is already deep negative because of a
  large carry-in), and that's expected: day total is "today, in
  isolation," the pace hint is "today's slice of the whole week's
  math."
- **Estimated EOD** (`est. EOD HH:MM`) appears only when today has an
  open stint: it's `now + (required − fulfillment)`, i.e. "if you
  keep going from right now, this is the clock time you'd close out
  today's slice of the week's pacing." Fulfillment here still excludes
  the open stint's live minutes (§2.4), same as everywhere else.
  Omitted entirely when there's no open stint (nothing to project
  from) or replaced with `target already met` when the gap is already
  zero or negative.
- Week line reports the week containing `DATE` (§3.5), and its
  framing follows the same ongoing-vs-not split as `mlm week` (§7.2):
  "`<owed>` left by end of `<weekday>`" when that week is the actual
  currently-ongoing week (using *today's* weekday, even if `DATE`
  itself is some other day within that same week — the deadline is
  always about today, not about which day's stints you're viewing),
  or the plain `Total still owed`/`Total ahead` form when `DATE`'s
  week is a past or future one. When the current week's `carry_in` is
  non-zero, the `(fulfillment .../target ...)` parenthetical expands to
  `(fulfillment F = worked W + carry-in C / target T)`, each figure
  signed independently per §4.2 (a carry-in deficit can drive
  `fulfillment` itself negative — that's expected, not an error) —
  when `carry_in` is zero, the shorter one-term form is unchanged. The
  past/future `Total still owed`/`Total ahead` form never gets this
  parenthetical, zero or non-zero carry-in alike.
- Stint lines: `HH:MM-HH:MM  (duration)`, ongoing stint's end is the
  literal word `now`. Tabular duration format (§4.2). Section omitted
  entirely when the date has zero stints (a note-only day, or a fully
  empty one) — same treatment as Notes below, no empty-list
  placeholder either way.
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
    information — and renders as `09:00-now  (unclosed)` instead. See
    §1.2a for the remaining gap this doesn't close (nothing signals the
    stale punch's existence on any *other* date's `status`, or in
    `week`'s per-day table).
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

### 7.4 Write-command output

`start`, `stop`, `note`, and `week target` print nothing on success —
silent, Unix-conventional, exit `0` (§6.3) is the only signal. A hard
error (§6.1) still prints its message to stderr as usual. `status` and
`week` produce stdout output (§7.1, §7.2); `delete note`/`delete
punch` are a narrow, deliberate exception to the write-command-silence
rule above rather than a repeal of it — list mode prints that date's
numbered entries (or `nothing to delete for <date>.` when there are
none), and a successful delete prints a ready-to-run recreate command.

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
- **F9b** — Pace hint with a large carry-in: seed a week with a
  carry-in big enough that `required − fulfillment` is already
  negative on day 1 despite day total being small/zero; confirm the
  pace hint (and est. EOD, if an open stint) reads off *fulfillment*
  and switches to the "X over `<required>`" wording (or `target
  already met` for est. EOD) independently of day total, while the
  plain "Day total" figure right next to it is unaffected.
  Also cover a Sat/Sun `status`: `required` pins to `5 × daily
  target` (`min(weekday, 5)` = 5). Include a target override that
  isn't a multiple of 5 minutes (e.g. `33h 31m`) and confirm the
  Sat/Sun `required` is `5 × floor(target/5)`, a few minutes under
  the override itself, not silently rounded up to match it.
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
  the next day): splices into one clean completed stint on the earlier
  date (§4.3.1) when the shape is unambiguous (one open `start` that
  date, one orphaned `end` — the next date's first punch — on the
  next). The later date shows nothing for it: no stint, no anomaly, no
  footnote (§1.2a notes this as a known rough edge, not a bug). Outside
  that exact shape — e.g. two dangling starts on the earlier date —
  falls back to the pre-splice behavior: an ordinary (non-anomalous)
  open `start` on day one, silent in `week`'s view (no `(ongoing)`, no
  `[!]`, §7.2), plus a flagged orphaned-`end` anomaly on day two.

---

*Spec complete through MVP scope, flow-mapped and adversarially
reviewed per NOTES.md's final-review note. Ready for TDD
implementation + independent adversarial code review, per the
workflow note at the top of NOTES.md.*
