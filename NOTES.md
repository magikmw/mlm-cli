# Design notes

Running log of background/story before formal spec exists. Not a spec —
decisions here can (will) change. Formal spec comes later as its own doc.

## Workflow note (process, not product)

When we get to implementation: use subagents for TDD (write tests first,
implement to green) and a separate independent adversarial-review
subagent pass before merging — not the same agent that wrote the code.

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
3. **Deficit carry**: shortfall (worked <40h in a week) *does*
   compound into the next week's target by default. But a week's
   target must be **adjustable** (sick day, half day, planned time
   off) — so target-hours is a per-week, overridable value, not always
   a derived constant. Default target = 40h minus any carried
   deficit/surplus-cap-at-zero rule; user can override a given week's
   target explicitly.
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

## Open questions (still need answers)

None currently — all resolved.

10. **Multi-week-format stretch**: deferred entirely. Not designing
    for it now; revisit only if it becomes a real ask.
