# Backdated punches — design spec

**Baseline**: written against `v0.1.5`.

**Status**: implemented — see
`docs/dev/plans/milestone-12-backdated-punches.md` for the executed
plan and the `backdated-punches` branch/commits for the change itself.

## 1. Summary

`start`, `stop` and `note` currently always operate on "today"
(`now.date_naive()`). This adds a `-d, --date <DATE>` option to all
three so entries can be recorded for a past date (e.g. forgotten
punches, corrections). `status`'s existing bare positional `DATE` arg
gains the same shorthand support for consistency.

## 2. CLI surface

- `start`, `stop`, `note` each gain `-d, --date <DATE>`, declared with
  `allow_hyphen_values = true` (required for clap to accept a `-N`
  shorthand as the flag's *value* instead of treating it as an unknown
  flag — see §4).
- `DATE` accepts two forms, both handled by a shared resolver (see §4):
  - Absolute: `YYYY-MM-DD` (existing `date::parse_date` grammar,
    unchanged).
  - Relative shorthand: `-` followed by exactly one or more ASCII
    digits, no sign, no decimal point, no surrounding whitespace,
    parsed as a positive integer N (≥ 1; `-0` is rejected), meaning N
    days before today (`-1` = yesterday, `-2` = the day before, …).
    Leading zeros are tolerated (`-01` == `-1`); anything else not
    matching this shape is a shape error (§3).
- `status`'s existing positional `DATE` argument gains the same `-N`
  shorthand (it already accepts absolute `YYYY-MM-DD`), but **not**
  the future-date restriction — see §3's note on that.
- `week` is unaffected — out of scope for this spec.
- Omitting `--date` on `start`/`stop`/`note` keeps today's existing
  behavior exactly as-is.

### 2.1 Required token order (clap footgun — confirmed empirically)

`PunchArgs.note` and `NoteArgs.body` are `trailing_var_arg = true,
allow_hyphen_values = true`. Once clap starts matching the trailing
positional, it stops re-scanning later tokens for named flags — so
**`--date`/`-d` must appear before any `NOTE` text**, or it is silently
absorbed into the note body instead of being parsed as the date flag
(verified against clap 4.6.6: `start 9:00 did a thing --date -1`
yields a note body of `"did a thing --date -1"` and `date: None`, with
no error at all). `--date` may appear before or after `TIME` — only
its position relative to `NOTE` matters:

- OK: `mlm start --date -1 9:00 kicked off migration`
- OK: `mlm start 9:00 --date -1 kicked off migration`
- **Silently wrong**: `mlm start 9:00 kicked off migration --date -1`
  (no error; `--date` becomes part of the note text)

This is a known limitation, not something this spec fixes structurally
(restructuring `NOTE` capture is out of scope — see §6). It must be
documented in both this spec and the `--date` help text (§4), and
locked down by an explicit test (§5) so a future clap upgrade or
refactor that changes this behavior gets caught rather than silently
shipped.

## 3. Validation & error handling

New hard errors (§6.1-tier in SPEC.md — reject outright, nothing
written, message on stderr, nonzero exit):

- Malformed `--date`/`-d` (or `status`'s positional `DATE`) value: not
  `YYYY-MM-DD` shape, not a real calendar date, or not a valid `-N`
  shorthand per §2's shape rule. `Cause::Shape`, `ArgKind::Date` (same
  as today's malformed-`DATE` error) — no new `Cause` variant needed
  for this part.
- `-N` resolves to a date chrono cannot represent (an absurdly large
  N): **must not panic**. `NaiveDate`'s `Sub<Days>` operator panics on
  overflow — the resolver must use `NaiveDate::checked_sub_days`
  instead and map `None` to a hard error (`Cause::OutOfRange`, reusing
  the existing variant — same shape as its current "week number/year
  out of range" use). Covered by an explicit "N absurdly large, no
  panic" test (§5), mirroring `date.rs`'s existing
  `junk_input_never_panics`-style convention.
- Resolved date is in the future (later than today, local calendar
  date) — **applies to `start`/`stop`/`note` only, not to `status`**.
  New `Cause::Future` variant, always paired with `ArgKind::Date`;
  `Display` text: `"date is in the future"`. `status` keeps its
  current behavior of accepting any resolved date, including future
  ones (SPEC §6.2 — a target date with no data just renders empty);
  this spec only adds `-N` shorthand support to `status`, it does not
  newly restrict what `status` already accepts. Concretely: the
  resolver splits into a core `resolve()` (absolute or `-N`, no future
  check) used by `status`, and a thin wrapper used by
  `start`/`stop`/`note` that adds the future-date check on top (§4).
- **`TIME` becomes required on `start`/`stop` when `--date`/`-d`
  resolves to a date other than today.** Defaulting `TIME` to "now's
  clock time" makes no sense for a different calendar day, so omitting
  `TIME` in that case is a hard error. When `--date` is omitted, or
  explicitly resolves to today, `TIME` keeps its current
  default-to-now behavior. New `TimeParseError::Required` variant
  (`src/time.rs` — TIME-shaped errors already live there, and this is
  "no TIME given where one was required," not a date-parsing failure);
  `Display` text: `"TIME is required when --date targets a day other
  than today"`.
- **Error precedence**, extending the existing TIME-before-NOTE rule
  (`time_error_precedes_note_error`): date resolution happens first,
  before any TIME handling (§4) — a malformed/future `--date` is
  reported even if `TIME`/`NOTE` are also invalid. Order is therefore
  **date errors → TIME errors (including the new "required" case) →
  NOTE errors**. Covered by an explicit precedence test (§5).

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

### 3.1 Accepted consequence: retroactive week figures

`owed`/`carry` are computed live, not materialized (SPEC §2.4) —
inserting a backdated punch into an already-"closed" past week changes
that week's, and every subsequent week's, figures on the next
`status`/`week` view. This is intentional and follows directly from
the existing live-computation model; called out here explicitly since
it's the whole point of the feature (fixing a forgotten punch should
retroactively correct the numbers), not a side effect to guard
against.

## 4. Implementation touch points

- **`src/date.rs`**:
  - `resolve_date(s: &str, today: NaiveDate) -> Result<NaiveDate, DateWeekError>`
    — the core resolver, no future check. Tries the `-N` shorthand
    first (shape per §2: `-` + digits only, parse the digits as a
    `u64`, reject `0`; subtract via `today.checked_sub_days(Days::new(n))`,
    mapping `None` — offset out of chrono's representable range — to
    `Cause::OutOfRange`, never letting the panicking `Sub<Days>`
    operator run on user input). Falls through to the existing
    `parse_date` grammar when the input isn't `-N`-shaped. Used
    directly by `status`.
  - `resolve_future_checked_date(s: &str, today: NaiveDate) -> Result<NaiveDate, DateWeekError>`
    (naming placeholder — pick whatever reads best at implementation
    time) — thin wrapper around `resolve_date` adding the `> today` →
    `Cause::Future` check. Used by `start`/`stop`/`note`.
  - `parse_date` itself stays untouched (pure, absolute-only): it has
    no `today` to compare against, and — corrected from an earlier
    draft of this spec — the reason to keep it separate is *not* that
    `storage.rs` calls it (it doesn't; `storage.rs` has its own
    private, unrelated `parse_date(s, column)` built directly on
    `NaiveDate::parse_from_str`). The actual reason: keeping the bare
    absolute-only grammar available as a primitive, independent of
    `-N`/future-check semantics, in case a future caller (e.g. `week`)
    wants strict absolute parsing without either.
- **`src/time.rs`**: add `TimeParseError::Required` (§3) alongside the
  existing `InvalidFormat`/`OutOfRange` variants, with its `Display`
  arm.
- **`src/cli.rs`**:
  - Add `-d, --date <DATE>` (`Option<String>`, `allow_hyphen_values =
    true`) to `PunchArgs` and `NoteArgs`.
  - Update `PunchArgs.time`'s doc comment: current text ("Defaults to
    now.") becomes conditionally true post-spec — reword to something
    like "Defaults to now when recording for today; required when
    `--date` targets another day." so `--help` doesn't mislead.
  - Document the §2.1 ordering footgun on the new `--date` field's doc
    comment (e.g. "Must come before NOTE text — see docs/dev/specs/…").
- **`src/commands.rs`**: in the shared `punch()` helper and in `note()`,
  resolve `args.date` via `date::resolve_future_checked_date` (falling
  back to `now.date_naive()` when absent) **before doing anything
  else** — including before the existing `parse_time`/default-to-now
  branch, per §3's error-precedence rule. Enforce the
  TIME-required-when-backdated rule (returning
  `TimeParseError::Required`) immediately after date resolution, still
  ahead of the existing NOTE validation.
- **`src/status.rs`**: swap the existing `date::parse_date(s)` call at
  the `resolve()` date-arg branch for `date::resolve_date(s, today)`
  (the core resolver — no future check, preserving `status`'s current
  permissive behavior per §3).

No `storage.rs` signature changes needed — `insert_punch_with_note`,
`insert_note`, `punches_for_date`, `notes_for_date` already take an
explicit `date: NaiveDate` parameter. (Verified directly against
current signatures, not assumed.)

## 5. Testing plan

- **`date.rs`** unit tests for `resolve_date` (core resolver):
  absolute date passthrough, valid `-N` shorthand (several N values,
  including a leap-year-crossing one, e.g. `today = 2027-03-01, -1 ⇒
  2026-02-28`), zero-padded `-N` accepted (`-01` == `-1`), `-0`
  rejected, non-integer/malformed offset rejected (`-1.5`, `-abc`,
  leading `+`, embedded whitespace), an absurdly large N rejected via
  `Cause::OutOfRange` **without panicking** (the checked-arithmetic
  fix from §3), resolved date equal to today accepted, resolved date
  in the future accepted (core resolver has no future check).
- **`date.rs`** unit tests for the future-checked wrapper: resolved
  date in the future rejected (both absolute and shorthand forms),
  resolved date equal to today or in the past accepted.
- **`commands.rs`** unit tests for `start`/`stop`: successful backdated
  punch with explicit TIME, missing TIME with a backdated `--date`
  rejected (nothing written), future `--date` rejected (nothing
  written), `--date` omitted behaves exactly as before (regression
  guard), an anomaly-producing pairing on a backdated date still
  surfaces the same as it would for today, an inline `NOTE` given
  alongside a backdated punch lands on the *resolved* date (not
  today) in the `notes` table, a simultaneous bad `--date` + bad/missing
  `TIME` reports the date error (§3 precedence).
- **`commands.rs`** unit test for `note`: backdated note stored against
  the resolved date, `created_at_utc` still reflects real now rather
  than the backdated date.
- **`cli.rs`** parse-level tests (`Cli::try_parse_from`, mirroring the
  existing hyphen-value pattern used for `WeekTargetArgs`/duration
  parsing): `--date -1` and `-d -1` both parse as the flag taking `-1`
  as its value (not as an unknown-flag error); the §2.1 ordering
  footgun itself gets one locked-down test asserting that `--date`
  placed after `NOTE` tokens is absorbed into the note body rather
  than parsed as the date flag — so a future clap upgrade or arg
  refactor that silently changes this is caught instead of shipped.
- **`status.rs`**: a couple of passthrough tests confirming `-N`
  shorthand resolves the same target date as the equivalent absolute
  `YYYY-MM-DD` would, plus a regression test that a future absolute
  date still succeeds (renders empty) exactly as it does today —
  proving the future-date restriction was *not* accidentally extended
  to `status`.

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
- Restructuring `NOTE`'s `trailing_var_arg` capture to make `--date`
  safe in any position (§2.1): accepted as a documented limitation for
  this spec rather than fixed structurally — reopens argument-parsing
  behavior well beyond this feature's footprint.
