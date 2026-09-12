# Design notes

Running log of background/story before formal spec exists. Not a spec —
decisions here can (will) change. Formal spec comes later as its own doc.

## Workflow note (process, not product)

When we get to implementation: use subagents for TDD (write tests first,
implement to green) and a separate independent adversarial-review
subagent pass before merging — not the same agent that wrote the code.

At final spec review (before implementation starts): give §6 (error
handling & validation) particular scrutiny by mapping out concrete
user flows end-to-end — tests will be written directly against those
flows/assumptions, so gaps there become gaps in test coverage.

## Current manual process (plain text file)

Each work day:

- Line per stint: wall-clock start time, then end time gets appended to
  the same line once known, plus that stint's summed duration (h+m).
  Sums usually happen incrementally, not all at EOD.
- A running work log above the day's time lines: short free-text notes
  on anything substantial done that day. Sometimes prefixed with a
  project name if specific enough (project tracking = stretch goal,
  not MVP).
- Below the stint lines: a day total (sum of stints).
- At EOD, a week-status block, updated daily:
  - hours worked this week so far
  - hours spilled over from last week (cap: 40h/week is the max that
    counts as "regular" — nothing above 40 becomes overtime, so only
    a *shortfall* spills forward, not surplus)
  - hours still owed by end of current day, given the above
  - a spillover/deficit note
- Dashed line delimits the start of a new week.

Pain point: summing time in your head is tedious, especially
incremental resumming through the day. Motivating the tool.

## MVP scope (from conversation)

- Internal time unit: minutes. Keep it simple.
- CLI to add start/end times for stints, **not necessarily
  chronologically** (need to handle out-of-order entry/edits).
- CLI to attach notes (day work log entries), separate from stints.
- Output (read-only, MVP):
  - today's status: all of today's stints, today's notes, day sum,
    week sum so far
  - a separate whole-week view
- Schema: normalized, designed to be easy to extend (migrations
  support from day one, since schema/data will evolve with features).

## Deferred / stretch

- Project tagging per note/stint.
- Terminal dashboard (ratatui, see README) — bar/sparkline views.
- Shell prompt integration (fast, side-effect-free status query).

## Decisions (from Q&A)

1. **Week boundary**: ISO week (Mon start) for MVP. Design should not
   hardcode this so deeply that other week-start conventions become
   impossible later — but supporting them is a stretch goal, not MVP.
2. **Week identity**: weeks are identified by an **(ISO year, ISO week
   number) tuple**, not by a start date. Robust across year boundaries
   (ISO week 1 of a year can include late-December dates and vice
   versa — `chrono` gives us `iso_week()` for this).
3. **Surplus/deficit carry**: the **target stays fixed at 40h by
   default** (or an explicit override, decision 7) — carry does not
   shift the target. Instead, carry-in is folded into the week's
   *fulfillment* sum, like an extra virtual day that itself worked a
   (possibly negative) number of minutes: `fulfillment = sum(this
   week's stints) + carry_in`. Owed = `target - fulfillment` (can
   already be negative, i.e. ahead of target from minute one of the
   week if carry-in is a surplus). Carry-out for the next week =
   `fulfillment - target` at week's end (signed: positive surplus
   *and* negative deficit both propagate). "No overtime" just means
   surplus is never paid out specially in the current week beyond
   being counted plainly toward fulfillment — it's not a bonus, just
   arithmetic.
4. **Stints from point-in-time punches, not stored ranges**: stints
   are *derived* like matched parentheses from a sequence of
   start/end time points, not stored as a single row with two
   columns. This is what makes non-chronological entry natural: you
   insert a point (start or end) at any time value, points get sorted
   by time, then paired sequentially (start, end, start, end, ...) to
   produce stints. Precomputing/caching paired stints in the DB is
   possible later but not worth it for MVP — compute at read time.
5. **Editing/deleting past entries**: deferred, not in MVP. Add-only.
6. **Timezone**: no explicit open question raised as blocking; assume
   single-machine local wall-clock time unless it comes up again.

## More decisions (from Q&A round 2)

7. **Target-override mechanism**: absolute value only for MVP (no
   delta/relative adjustment). Command takes a week id and a value;
   week id typing defaults to current (year, week) when omitted, and
   the id is written without the ISO `W` prefix (e.g. `2026-07`, not
   `2026-W07`) — KISS on input format.
8. **Dangling/open punch display**: shown as ongoing, duration
   computed live against current time, updates on each view.
9. **Punch surrogate id**: yes — `id INTEGER PRIMARY KEY
   AUTOINCREMENT` on the punch table, independent of `(date, time)`.
   Rationale: a natural `(date, time)` key breaks the moment you edit
   a punch's time (the key itself would be what's changing) and can't
   disambiguate two punches at an identical timestamp. Free in
   SQLite, standard practice, avoids an awkward migration once editing
   lands.

## More decisions (from Q&A round 3)

11. **Time zone**: store UTC internally, convert to/from the user's
    local (system) timezone at input/output only. Local calendar date
    for a punch is therefore computed app-side, not derivable from the
    stored UTC instant by plain string slicing (see SPEC.md §2.1).
12. **Punch time input formats**: `HH:MM`, `HHMM`, `HH` (minute
    defaults to `:00`), 24h only for MVP. 12h AM/PM format is a
    stretch goal.
13. **Note-only entry**: `mlm note NOTE` — a separate command that
    attaches a work-log note to today without touching punches, for
    EOD/next-day notes with nothing to punch.
14. **Week id input shorthand**: accepts a full id (`YYYY-WW`) or a
    bare week number, defaulting the year to the current one.
15. **Relative day/week stretch**: `+N`/`-N` notation for `status`'s
    date argument and `week`'s week-id argument, relative to
    today/this-week. Stretch goal, not MVP.

## More decisions (from Q&A round 4, spec §7)

17. **Daily pace hint on `status`**: not a new independent target —
    a display-only "X left to daily target" figure, where daily
    target = the current week's target ÷ 5 (floor), always derived,
    never overridden on its own.
18. **Week owed framing**: `status`'s week line and `week`'s headline
    both lead with the owed figure. For the *current* week it's
    framed as "`X` left by end of `<weekday>`"; a closed or
    not-yet-started week (no "today" inside it to frame against) uses
    a plain "Total still owed `X`" / "Total ahead `X`" form instead —
    closed weeks are done, they just report their final number.
19. **Estimated EOD**: shown on `status` only when today has an open
    stint — `now + (daily target − day total)`, i.e. "the clock time
    you'd hit today's quota if you kept going from right now."
    Omitted with no open stint; replaced with "target already met"
    once the gap is ≤ 0.

## More decisions (from flow-mapping + adversarial spec review, round 5)

Flow-mapping SPEC.md's §6 against concrete user flows surfaced gaps
G1-G4 (now resolved below); a separate independent adversarial review
pass over the whole spec then found one self-contradiction (blocker)
and several real underspecified-behavior gaps. All folded directly
into SPEC.md; logged here for the record.

21. **G1 — empty-day rendering**: a date with zero stints omits the
    stint-list section entirely (same treatment as Notes with none).
22. **G2 — zero-target weeks**: `0h` is a legal target override (an
    explicit week off), distinct from silently letting a normal
    week's deficit accrue. Only negative durations are rejected.
23. **G3 — note trimming**: note `body` is trimmed of leading/trailing
    whitespace before storage (emptiness is still checked pre-trim).
24. **G4 — backdated entry**: confirmed intentional — `start`/`stop`/
    `note` only ever target today in MVP, no exceptions.
25. **Blocker fix — §7.2 closed-week example**: was contradicting its
    own prose (bare one-liner vs. claimed full breakdown). Resolved:
    a closed/future week gets the same full per-day table and
    carry-in/worked/fulfillment/target block as the current week —
    only the headline wording differs (plain total vs.
    weekday-deadline framing).
26. **`status DATE` week scope**: fixed to show the week *containing*
    `DATE`, not always "today's" week — the old behavior (a past
    date's stints shown next to today's unrelated week totals) read
    as a bug, not a feature. The deadline-vs-plain-total headline
    split from decision 25 applies here too, keyed off whether
    `DATE`'s week is the actual currently-ongoing one (using today's
    weekday for the deadline phrase even if `DATE` itself is a
    different day in that same week) or a past/future one. The
    daily-target and estimated-EOD lines only ever appear when `DATE`
    is literally today, since both depend on "day total so far" and
    "now."
27. **Overnight stints — accepted limitation**: a session crossing
    midnight splits into two anomalies (permanently-open `start` on
    day one, orphaned `end` on day two) since pairing is strictly
    per-`date`. Deliberately not fixed for MVP — pairing across date
    boundaries is real complexity for a rare case.
28. **DST-safe conversion**: every UTC↔local conversion must use the
    offset that applied *at that specific instant*, never a single
    "current" offset reused across punches — otherwise punches either
    side of a DST transition would display incorrectly.
29. **`WEEK_ID` validation**: unpadded week numbers (`2026-7`) are
    accepted and normalized, matching the lenient-input style used
    for `TIME`/`DURATION` elsewhere. The out-of-range check is a real
    per-year ISO-week-count check (52 vs. 53), not a flat `1..=53`.
30. **`TIME` boundary**: `24:00` is rejected outright, not treated as
    a next-day-midnight alias. Valid range is `00:00`-`23:59`.
31. **Zero-length stints**: a `start`/`end` pair at the identical
    instant is legal (`00h 00m`, not an anomaly) — matched-parentheses
    pairing doesn't care about same-instant ordering.
32. **Multiple orphaned `end`s**: each gets its own flagged line
    (§7.3), never coalesced into a single count.
33. **First run vs. DB failure**: a missing app-data directory/db file
    bootstraps silently (already how `src/db.rs`'s scaffold works);
    only a genuinely broken DB (bad permissions, corruption, a failed
    migration) is a hard error.
34. **Concurrency**: explicitly out of scope for MVP — single-user,
    single-machine tool, SQLite's default locking trusted to fail
    safely rather than corrupt data, no WAL/busy-timeout tuning
    planned.
35. **Note content policy**: no length cap, no charset restriction —
    §7's plain-ASCII commitment is about rendered layout characters,
    not what a user can type into a note body.

## Open questions (still need answers)

None currently — all resolved.

36. **Multi-week-format stretch**: deferred entirely. Not designing
    for it now; revisit only if it becomes a real ask.
