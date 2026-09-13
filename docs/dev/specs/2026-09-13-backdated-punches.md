# Backdated punches — design spec

**Baseline**: written against `v0.1.5`.

**Status**: approved design, not yet implemented.

## 1. Summary

`start`, `stop` and `note` currently always operate on "today"
(`now.date_naive()`). This adds a `-d, --date <DATE>` option to all
three so entries can be recorded for a past date (e.g. forgotten
punches, corrections). `status`'s existing bare positional `DATE` arg
gains the same shorthand support for consistency.

## 2. CLI surface

- `start`, `stop`, `note` each gain `-d, --date <DATE>`.
- `DATE` accepts two forms, both already or newly supported by a
  shared resolver (see §4):
  - Absolute: `YYYY-MM-DD` (existing `date::parse_date` grammar,
    unchanged).
  - Relative shorthand: `-N` where N is an integer ≥ 1, meaning N days
    before today (`-1` = yesterday, `-2` = the day before, …).
- `status`'s existing positional `DATE` argument gains the same `-N`
  shorthand (it already accepts absolute `YYYY-MM-DD`).
- `week` is unaffected — out of scope for this spec.
- Omitting `--date` on `start`/`stop`/`note` keeps today's existing
  behavior exactly as-is.

## 3. Validation & error handling

New hard errors (§6.1-tier in SPEC.md — reject outright, nothing
written, message on stderr, nonzero exit):

- Malformed `--date`/`-d` (or `status`'s positional `DATE`) value: not
  `YYYY-MM-DD` shape, not a real calendar date, or not a valid `-N`
  shorthand (N must be a positive integer; `-0`, `-1.5`, `-abc` etc.
  are all shape errors).
- Resolved date is in the future (later than today, local calendar
  date). Applies to both absolute and shorthand forms.
- **`TIME` becomes required on `start`/`stop` when `--date`/`-d`
  resolves to a date other than today.** Defaulting `TIME` to "now's
  clock time" makes no sense for a different calendar day, so omitting
  `TIME` in that case is a hard error. When `--date` is omitted, or
  explicitly resolves to today, `TIME` keeps its current
  default-to-now behavior.

Not new validation (existing rules apply unchanged, just against the
resolved date instead of always-today):

- Start/end pairing and anomaly detection (SPEC §4.3): unaffected.
  `storage::insert_punch_with_note` already takes the target `date` as
  a parameter and does its ordering/anomaly bookkeeping per-date, so a
  backdated date flows through the exact same logic as today's date
  does now. A backdated day with an odd pairing surfaces as an anomaly
  in that day's `status`/`week` output later, exactly like it would
  for today.
- Empty/whitespace-only `NOTE` body rejection (SPEC §6.1): unaffected.

`created_at_utc` (the audit-trail insert timestamp on both `punches`
and `notes`) always stays the real wall-clock instant the command was
run — it is never backdated, regardless of `--date`. Only the punch's
`date` and time-of-day (and a note's `date`) reflect the resolved
target date.

## 4. Implementation touch points

- **`src/date.rs`**: new resolver, e.g.
  `resolve_date(s: &str, today: NaiveDate) -> Result<NaiveDate, DateWeekError>`.
  Tries `-N` shorthand first (integer ≥ 1, subtract from `today`), else
  falls through to the existing `parse_date` grammar. Rejects a
  resolved date `> today` (new `Cause` variant, e.g. `Cause::Future`).
  `parse_date` itself stays untouched (pure, absolute-only) since it
  has no `today` to compare against and other callers (`storage.rs`
  round-tripping stored dates) must stay unaffected by this feature.
- **`src/cli.rs`**: add `-d, --date <DATE>` (`Option<String>`) to
  `PunchArgs` and `NoteArgs`.
- **`src/commands.rs`**: in the shared `punch()` helper and in `note()`,
  resolve `args.date` via `date::resolve_date` (falling back to
  `now.date_naive()` when absent) before doing anything else; enforce
  the TIME-required-when-backdated rule in `punch()` right after date
  resolution, before the existing `parse_time`/default-to-now branch.
- **`src/status.rs`**: swap the existing `date::parse_date(s)` call at
  the `resolve()` date-arg branch for `date::resolve_date(s, today)`.

No `storage.rs` signature changes needed — `insert_punch_with_note`,
`insert_note`, `punches_for_date`, `notes_for_date` already take an
explicit `date: NaiveDate` parameter.

## 5. Testing plan

- **`date.rs`** unit tests for `resolve_date`: absolute date passthrough,
  valid `-N` shorthand (several N values), `-0` rejected, non-integer
  offset rejected, resolved date in the future rejected (both absolute
  and shorthand forms), resolved date equal to today accepted.
- **`commands.rs`** unit tests for `start`/`stop`: successful backdated
  punch with explicit TIME, missing TIME with a backdated `--date`
  rejected (nothing written), future `--date` rejected (nothing
  written), `--date` omitted behaves exactly as before (regression
  guard), an anomaly-producing pairing on a backdated date still
  surfaces the same as it would for today.
- **`commands.rs`** unit test for `note`: backdated note stored against
  the resolved date, `created_at_utc` still reflects real now rather
  than the backdated date.
- **`status.rs`**: a couple of passthrough tests confirming `-N`
  shorthand resolves the same target date as the equivalent absolute
  `YYYY-MM-DD` would.

No new coverage/complexity work beyond what naturally follows from
exercising each new branch — falls under the existing CRAP-ish
pre-commit gate (`AGENTS.md`), no separate boundary list needed.

## 6. Out of scope

- `week` command / `WEEK_ID` argument: untouched.
- Editing or deleting existing punches/notes: still no editing story,
  per MVP (this spec only adds *where* a new punch/note can land, not
  a way to fix ones already recorded).
- Timezone changes, DST edge cases beyond what already applies to
  today's punches: unaffected by this feature.
