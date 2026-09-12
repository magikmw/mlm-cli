# mlm — specification

Formal spec, built incrementally from `NOTES.md` decisions. Sections
land one at a time for review — this doc grows over several passes,
not all at once.

Status: **draft, section 1 of N**.

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
  an absolute value per week, and report hours still owed against it.
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

### 1.3 Terminology

- **Punch**: a single timestamped event, either `start` or `end`, on a
  given date.
- **Stint**: a derived (start, end) time range, computed by pairing a
  day's punches in chronological order like matched parentheses — not
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

---

*Next up: §2 data model (tables, columns, migrations approach).
Flag anything above before I continue.*
