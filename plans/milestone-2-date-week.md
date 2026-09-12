# Milestone 2 — Calendar date and week-id parsing/formatting

**Implementation plan.** Detailed enough for a TDD subagent to start writing
tests with no further clarification. Unlike `PLAN.md`, concrete types and
signatures *are* in scope here.

**Spec basis**: SPEC.md §1.3 (week = ISO year + week tuple), §2.3
(`week_targets.week_id` shape), §3.5 (`DATE` argument), §3.6 (`WEEK_ID`
argument forms), §6.1 (DATE/WEEK_ID hard errors), §7.1/§7.2 (worked
examples that pin real week↔date mappings). Flow ids: **E2**, **E3**;
indirectly F6, F10, F11, F8b.

**PLAN.md basis**: Milestone 2 (wave 1), interface contracts 6 and 7.

---

## 0. Summary of decisions

| # | Decision | Status |
|---|---|---|
| D1 | New module `src/date.rs`; `src/time.rs` is left entirely to Milestone 1 | proposed |
| D2 | `DATE` is `chrono::NaiveDate`, no newtype | proposed |
| D3 | `WEEK_ID` is a validated newtype `WeekId(i32, u8)` with private fields | proposed |
| D4 | `DATE` parsing is **strict** about zero-padding (`2026-2-12` rejected) | **ambiguity — see §7.1** |
| D5 | Bare `WW` defaults to *today's ISO year*, not today's calendar year | **ambiguity — see §7.2** |
| D6 | ISO week count derived from the Dec-28 rule, not `from_isoywd_opt`'s `None` | proposed |
| D7 | Local `DateWeekError` in `src/date.rs` (not shared with Milestone 1) — a concrete completion of PLAN contract 7, later reconciled to wave-1-local error types | **resolved — see §3** |
| D8 | Week *iteration* (`next`/`prev`/`week_range`) is owned by this milestone, not Milestone 6 | **scope addition — see §7.4** |
| D9 | `format_date_with_weekday` lives here, not in Milestone 9/10/11 | **scope addition — see §7.5** |
| D10 | "current year" is injected as a `today: NaiveDate` parameter (extends contract 6 backwards to M2) | proposed |

---

## 1. Module layout

```
src/date.rs    — NEW. Everything this milestone owns, including its
                 own local DateWeekError (§3) — no shared error file.
```

Rationale for a separate `date.rs` rather than extending `time.rs`:
Milestone 1 and Milestone 2 are both wave-1 and are expected to run in
**parallel worktrees**. `time.rs` is rewritten wholesale by Milestone 1;
touching it here guarantees a merge conflict for zero benefit. No file is
shared between the two milestones — each owns its own error type in its
own module (contract 7, final form), which is exactly what keeps the two
worktrees from needing any coordination beyond `main.rs`'s `mod` list.

`src/main.rs` needs `mod date;` added. This is a one-line edit that **every**
wave-1 milestone also makes to the same `mod` block — expect a trivial
conflict there and resolve by union. Do not restructure `main.rs` here.

---

## 2. Type design

### 2.1 `DATE` → `chrono::NaiveDate` (no newtype)

```rust
use chrono::NaiveDate;
```

**Why no newtype.** A `DATE` carries no invariant beyond "is a real calendar
date", which `NaiveDate` already enforces at construction. It is also the
exact type every downstream consumer wants: Milestone 4 stores
`date.format("%Y-%m-%d")` into `punches.date` / `notes.date`, Milestone 5
never sees it, Milestone 10 formats it, and `WeekId::from_date` consumes it.
Wrapping it would force unwrapping at every one of those boundaries. §2.1's
"local calendar date is its own concept" is about *storage*, not about
needing a distinct Rust type.

The *string form* used for storage is `%Y-%m-%d`, identical to the input
form, so one function covers both directions.

### 2.2 `WEEK_ID` → validated newtype `WeekId`

```rust
/// An ISO-8601 week: (ISO year, week number). Always Monday-start (§1.3).
///
/// Invariant: `week` is a real ISO week for `iso_year`
/// (i.e. `1 <= week <= iso_weeks_in_year(iso_year)`), and
/// `1000 <= iso_year <= 9999`. Enforced by every constructor; fields are
/// private so the invariant cannot be bypassed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WeekId {
    iso_year: i32,
    week: u8,
}
```

**Why a newtype and not a bare `(i32, u32)` tuple or a `String`.**

- `§6.1` makes "is this a real ISO week for that year" a *hard error*, i.e. a
  validation gate. A newtype with private fields is the standard way to make
  "validated once, trusted everywhere" a compile-time fact. Milestones 6, 8,
  9 and 11 then never re-check.
- A `String` week id (which is what lands in SQLite) would push
  re-parsing/re-validating into Milestone 6's hot walk and would make
  `next()`/`prev()` rollover logic (52 vs 53 weeks) a string operation.

**Why the derives, individually.**

- `Copy`: 8 bytes, passed by value everywhere. Milestone 6's walk copies it
  per iteration; `&WeekId` would be noise.
- `PartialOrd`/`Ord`: derived lexicographically over `(iso_year, week)`,
  which — because the invariant guarantees `week` is in range — is **exactly
  chronological order**. Milestone 6 needs `while cursor <= target` for the
  week walk; Milestone 9 needs past/current/future comparisons. Deriving
  this rather than hand-writing it is only correct *because* of the
  invariant, so it is worth an explicit test (§5, T79–T80).
- `Hash` + `Eq`: Milestone 6 wants `HashMap<WeekId, i64>` for worked minutes
  and `HashMap<WeekId, i64>` for target overrides, keyed without
  stringification.
- `Debug`: test assertion readability.

**Why `u8` for the week but `u32` at the API edge.** `1..=53` fits a `u8`
and keeps the struct at 8 bytes with a niche-free layout; but `chrono`'s
`IsoWeek::week()` and `NaiveDate::from_isoywd_opt` both speak `u32`, so
accessors and constructors take/return `u32` and convert internally. This
keeps zero casts in caller code.

**Why `i32` for the ISO year.** Matches `chrono`'s year type exactly;
avoids casts in `from_isoywd_opt` / `iso_week().year()`. The parser
narrows it to `1000..=9999` anyway (§7.6).

### 2.3 Week iteration type

```rust
/// Inclusive iterator over consecutive ISO weeks. Empty if `from > to`.
/// Handles 52↔53-week year rollover.
#[derive(Debug, Clone)]
pub struct WeekRange { cursor: Option<WeekId>, end: WeekId }

impl Iterator for WeekRange { type Item = WeekId; /* ... */ }
```

`Option<WeekId>` for the cursor (rather than a `done: bool`) makes the
"already emitted `end`" state unrepresentable-in-two-ways. `Clone` so
Milestone 6 can count then re-walk.

---

## 3. Error type — completing PLAN contract 7

PLAN.md contract 7 says only: *"every parsing/storage function that can
hard-error (Milestones 1, 2, 4) needs an agreed error shape so Milestones 7
and 8's CLI wiring … doesn't need rework."* It does **not** specify the
shape. **Superseded by cross-plan reconciliation**: contract 7 was later
settled as "each wave-1 milestone owns its own local error enum, in its
own module — no shared error file across wave-1 worktrees" (the shared
`src/error.rs` proposed below is exactly the kind of cross-worktree
coupling that caused divergence when tried literally). This milestone's
error type moves entirely into `src/date.rs`, local to Milestone 2, not
shared with Milestone 1's `TimeParseError`/`DurationParseError`. The
struct-with-enum-cause *design* below is otherwise unchanged and still a
good fit — only its module location and its `ArgKind` variant set change
(no `Time`/`Duration` variants here; those belong to Milestone 1's own
type). At the CLI boundary (Milestone 7 onward), `anyhow::Result<()>`
unifies every wave-1 error type via `?` — no shared enum needed for that
either (PLAN.md contract 7, final form).

```rust
// src/date.rs — local to Milestone 2, not shared with any other module.

/// Which of this milestone's two argument kinds failed to parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgKind { Date, WeekId }

/// Machine-inspectable cause. Tests assert on this, not on message text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cause {
    /// Did not match the argument's grammar at all.
    Shape,
    /// Matched the grammar but a component is outside its legal range
    /// (week `0`).
    OutOfRange,
    /// Well-shaped `YYYY-MM-DD` that is not a real calendar date
    /// (`2026-02-30`).
    NoSuchCalendarDate,
    /// Well-shaped `YYYY-WW` whose week does not exist in that ISO year.
    /// Carries the year's real week count so the message can say so.
    NoSuchIsoWeek { weeks_in_year: u32 },
}

/// A §6.1 hard error originating at the input edge, for this milestone's
/// two argument kinds only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateWeekError {
    pub arg: ArgKind,
    /// The user's input, verbatim and untruncated, for the stderr message.
    pub input: String,
    pub cause: Cause,
}

impl DateWeekError {
    pub fn new(arg: ArgKind, input: &str, cause: Cause) -> Self;
}

impl std::fmt::Display for DateWeekError { /* see wording below */ }
impl std::error::Error for DateWeekError {}
```

**Display wording** (single line, plain ASCII per §7, no trailing period —
Milestones 7/8/10/11 propagate it via `anyhow` and print it to stderr as-is,
exit nonzero per §6.3):

| cause | rendered |
|---|---|
| `Date` + `Shape` | `invalid DATE "13/02/2026": expected YYYY-MM-DD` |
| `Date` + `NoSuchCalendarDate` | `invalid DATE "2026-02-30": not a real calendar date` |
| `WeekId` + `Shape` | `invalid WEEK_ID "abcd": expected YYYY-WW or WW` |
| `WeekId` + `OutOfRange` | `invalid WEEK_ID "0": week number must be 1 or greater` |
| `WeekId` + `NoSuchIsoWeek{52}` | `invalid WEEK_ID "2027-53": 2027 has only 52 ISO weeks` |

**Why a struct-with-enum-cause rather than a flat enum per argument kind.**
Tests want to assert *why* it failed without string-matching, and one
struct covering both of this milestone's argument kinds avoids two nearly
identical types. A flat `enum DateError { BadShape, BadDate, ... }` plus a
separate `WeekIdError` would just duplicate the `Display`/`Error` boilerplate
for no benefit, since both stay internal to this one module either way.

**No `thiserror` dependency for this type.** `DateWeekError`'s hand-written
`Display`/`Error` impls are ~15 lines; `anyhow` (now a project dependency,
per PLAN.md contract 7) is for the CLI boundary, not for replacing this
milestone's own precise error type.

**Explicitly not in scope here**: DB errors (Milestone 3/4 own those) and
the `main()` error→exit-code plumbing (Milestones 7/8/10/11).

---

## 4. Public API — full signatures

All in `src/date.rs` unless noted.

### 4.1 DATE

```rust
/// Parse a `DATE` argument (§3.5): strictly `YYYY-MM-DD`, zero-padded.
/// Rejects wrong shape and non-existent calendar dates (§6.1, E2).
pub fn parse_date(s: &str) -> Result<NaiveDate, DateWeekError>;

/// Canonical storage/display form: `YYYY-MM-DD`.
/// Used by Milestone 4 for `punches.date`/`notes.date`.
pub fn format_date(d: NaiveDate) -> String;

/// §7.1's status header / §7.2's week-row form: `Thu 2026-02-12`,
/// `Mon 2026-02-09`. Always ASCII, always English (chrono's `%a` is
/// locale-independent), never a locale lookup (§7 plain-ASCII rule).
pub fn format_date_with_weekday(d: NaiveDate) -> String;

/// `Thursday` — §7.1/§7.2's "left by end of <weekday>" headline word.
/// Consumed by Milestone 9. Full name, not the abbreviation.
pub fn format_weekday_full(d: NaiveDate) -> String;
```

**Implementation note for `parse_date`:** do **not** rely on
`NaiveDate::parse_from_str(s, "%Y-%m-%d")` alone — chrono's `%Y`/`%m`/`%d`
accept unpadded and, for `%Y`, signed/5-digit years, which would silently
accept `2026-2-12` and `+2026-02-12`, violating D4. Validate the shape
first with an explicit byte check (`len == 10`, `b[4] == b'-'`,
`b[7] == b'-'`, all other bytes ASCII digits) → `Cause::Shape`; then
`NaiveDate::from_ymd_opt` → `Cause::NoSuchCalendarDate` on `None`. This
also cleanly separates the two `Cause` values the tests assert on.

### 4.2 WEEK_ID construction & validation

```rust
/// Number of ISO weeks in an ISO year: always 52 or 53 (§6.1).
///
/// Implemented via the ISO invariant that **December 28 always falls in the
/// last ISO week of its own year**:
///   `NaiveDate::from_ymd_opt(y, 12, 28).unwrap().iso_week().week()`
/// This is preferred over probing `from_isoywd_opt(y, 53, Mon).is_some()`
/// because it does not depend on chrono's out-of-range behaviour for that
/// constructor (D6).
pub fn iso_weeks_in_year(iso_year: i32) -> u32;

impl WeekId {
    /// Validated constructor. `Cause::OutOfRange` for `week == 0`;
    /// `Cause::NoSuchIsoWeek` for a week above the year's real count;
    /// `Cause::OutOfRange` for a year outside `1000..=9999`.
    /// `input` is threaded through only to populate `DateWeekError::input`.
    pub fn new(iso_year: i32, week: u32, input: &str) -> Result<Self, DateWeekError>;

    /// Infallible: every real date belongs to exactly one ISO week.
    pub fn from_date(date: NaiveDate) -> Self;

    /// The ISO week containing `today`. Sugar for `from_date`, named for
    /// intent at the three call sites that mean "current week"
    /// (§3.6, §3.7 defaults; Milestone 9's current-week check).
    pub fn current(today: NaiveDate) -> Self;

    pub fn iso_year(self) -> i32;
    pub fn week(self) -> u32;
}

/// Parse a `WEEK_ID` argument (§3.6): full `YYYY-WW` **or** a bare `WW`
/// whose year defaults to `today`'s **ISO** year (D5). Unpadded week
/// numbers are accepted and normalized (`2026-7` == `2026-07`, E3).
/// A leading `W` is *not* accepted (§2.3: "no `W`").
///
/// `today` is an injected parameter, never a hidden `Local::now()` read —
/// extending PLAN contract 6's convention back to this milestone (D10) so
/// the bare-`WW` tests are deterministic.
pub fn parse_week_id(s: &str, today: NaiveDate) -> Result<WeekId, DateWeekError>;
```

### 4.3 WEEK_ID formatting & storage round-trip

```rust
impl std::fmt::Display for WeekId {
    /// `YYYY-WW`, week zero-padded to two digits: `2026-07`, `2026-53`.
    /// This is simultaneously the §2.3 `week_targets.week_id` primary-key
    /// form and the §7.1/§7.2 display form — one format, deliberately.
}

impl WeekId {
    /// Explicit name for the storage form (== `to_string()`), so Milestones
    /// 3/8 read as intentional rather than incidentally stringifying.
    pub fn to_key(self) -> String;

    /// Strict inverse of `to_key` for reading `week_targets` rows back.
    /// Accepts **only** zero-padded `YYYY-WW` — no bare `WW`, no unpadded
    /// form, no `today` dependency. Anything else is a corrupt/foreign row,
    /// not lenient user input.
    pub fn from_key(s: &str) -> Result<Self, DateWeekError>;
}
```

**Why the lenient/strict split.** §3.6's leniency is a *user-input* affordance.
Storage must be canonical or `week_targets`'s `TEXT PRIMARY KEY` would admit
`2026-7` and `2026-07` as two rows for the same week, silently breaking
Milestone 8's upsert and Milestone 6's lookup. Keeping the strict reader
separate makes that impossible to get wrong by reaching for the wrong
function. (Also: zero-padding is what makes lexicographic ordering of the
stored keys agree with chronological order — a `ORDER BY week_id` in any
future query depends on it.)

**Not in scope:** `rusqlite::ToSql`/`FromSql` impls for `WeekId`. They would
be cheap and are a reasonable follow-up, but they couple `date.rs` to
`rusqlite` and belong to whichever of Milestone 3/8 wants them. Flagged in
§7.7.

### 4.4 Span derivation and iteration

```rust
impl WeekId {
    /// Monday of this week.
    pub fn start(self) -> NaiveDate;
    /// Sunday of this week (== `start() + 6 days`).
    pub fn end(self) -> NaiveDate;
    /// `(Monday, Sunday)` — §7.2's header span `(2026-02-09 - 2026-02-15)`.
    pub fn span(self) -> (NaiveDate, NaiveDate);
    /// All seven dates, Monday first. Fixed-size array because §7.2
    /// mandates exactly 7 rows, always — Milestone 11 gets that guarantee
    /// from the type rather than from a runtime length check.
    pub fn dates(self) -> [NaiveDate; 7];

    /// Does this week contain `date`? (== `WeekId::from_date(date) == self`)
    pub fn contains(self, date: NaiveDate) -> bool;

    /// Next/previous ISO week, rolling 52↔53 correctly
    /// (`2026-53.next() == 2027-01`; `2027-52.next() == 2028-01`).
    pub fn next(self) -> Self;
    pub fn prev(self) -> Self;
}

/// Inclusive week sequence, `from..=to`. Empty when `from > to`.
/// This is Milestone 6's week-walk driver (§2.4: walk *every* week
/// including idle ones).
pub fn week_range(from: WeekId, to: WeekId) -> WeekRange;
```

`start()`/`end()` are implemented with
`NaiveDate::from_isoywd_opt(iso_year, week, Weekday::Mon)` — safe to
`expect()` *only* because the type invariant guarantees the week exists;
the expect message should say so.

`next()`/`prev()` are implemented arithmetically on `(year, week)` using
`iso_weeks_in_year`, **not** via `start() + 7 days → from_date`. Both are
correct; the arithmetic version is the one whose 52/53 rollover the tests
target directly, and the date-arithmetic version is a good cross-check
property test (T74).

`next()`/`prev()` saturate at the `1000..=9999` year bounds — or, simpler
and preferred: they `panic!` there, since the CLI can never produce a week
near those bounds. Pick one and document it; tests do not exercise it.

---

## 5. Test plan

Unit tests in `#[cfg(test)] mod tests` at the bottom of `src/date.rs`.
Fixed "today" for every bare-`WW` test is **`2026-02-12`** (a Thursday, ISO
week `2026-07`) — the same date §7.1 uses, so the tests and the spec's
worked examples reinforce each other.

Every `Err` expectation asserts on `(arg, input, cause)`, never on the
`Display` string, except the three dedicated message tests (T82–T84).

### 5.1 `parse_date` — §3.5, §6.1, **E2**

| # | input → expected |
|---|---|
| T1 | `"2026-02-12"` → `NaiveDate 2026-02-12` |
| T2 | `"2026-01-01"` → ok |
| T3 | `"2026-12-31"` → ok |
| T4 | `"2024-02-29"` → ok (real leap day) |
| T5 | `"2023-02-29"` → `Err(Date, NoSuchCalendarDate)` |
| T6 | `"2026-02-30"` → `Err(Date, NoSuchCalendarDate)` — **E2, spec-named** |
| T7 | `"13/02/2026"` → `Err(Date, Shape)` — **E2, spec-named** |
| T8 | `"2026-13-01"` → `Err(Date, NoSuchCalendarDate)` (month 13) |
| T9 | `"2026-00-10"` → `Err(Date, NoSuchCalendarDate)` (month 0) |
| T10 | `"2026-02-00"` → `Err(Date, NoSuchCalendarDate)` (day 0) |
| T11 | `""` → `Err(Date, Shape)` |
| T12 | `"abc"` → `Err(Date, Shape)` |
| T13 | `"2026-2-12"` → `Err(Date, Shape)` — **D4, see §7.1** |
| T14 | `"2026-02-2"` → `Err(Date, Shape)` — D4 |
| T15 | `"26-02-12"` → `Err(Date, Shape)` |
| T16 | `"2026-02-12T09:00"` → `Err(Date, Shape)` (trailing garbage) |
| T17 | `"2026-02-121"` → `Err(Date, Shape)` |
| T18 | `" 2026-02-12"` / `"2026-02-12 "` → `Err(Date, Shape)` (no implicit trim) |
| T19 | `"+2026-02-12"` → `Err(Date, Shape)` (guards against chrono `%Y` leniency) |
| T20 | `"2026_02_12"` → `Err(Date, Shape)` |
| T21 | `"2026-02-1２"` (non-ASCII digit) → `Err(Date, Shape)`, no panic |

### 5.2 `format_date` / weekday formatting — §7.1, §7.2, §2.3

| # | input → expected |
|---|---|
| T22 | `format_date(2026-02-12)` → `"2026-02-12"` |
| T23 | `format_date(2026-01-05)` → `"2026-01-05"` (zero-padded month & day) |
| T24 | round-trip `parse_date(format_date(d)) == d` over a sampled date range |
| T25 | `format_date_with_weekday(2026-02-12)` → `"Thu 2026-02-12"` (§7.1 header) |
| T26 | `format_date_with_weekday(2026-01-05)` → `"Mon 2026-01-05"` (§7.1 2nd example) |
| T27 | `format_date_with_weekday(2026-02-09)` → `"Mon 2026-02-09"` (§7.2 row) |
| T28 | `format_weekday_full(2026-02-12)` → `"Thursday"` (§7.2 headline) |
| T29 | all seven weekdays of `2026-02-09..2026-02-15` render ASCII-only (`is_ascii()`), 3-char abbrevs (§7) |

### 5.3 `iso_weeks_in_year` — §6.1's "most years have 52, some have 53"

Candidate years derived from the ISO rule (*53 weeks iff Jan 1 is Thursday,
or it is a leap year and Jan 1 is Wednesday* — equivalently, the year ends
on a Thursday, or is a leap year ending on a Friday). Verified against the
system calendar rather than assumed:

| # | input → expected | why this year |
|---|---|---|
| T30 | `2026` → `53` | Jan 1 2026 is a **Thursday** (and Dec 31 is a Thursday). The spec's own examples live in 2026 — so `2026-53` must be *accepted* |
| T31 | `2032` → `53` | leap year, Jan 1 is a **Thursday** |
| T32 | `2020` → `53` | **leap** year, Jan 1 is a **Wednesday** / Dec 31 a Thursday — the second, easy-to-miss branch of the rule |
| T33 | `2027` → `52` | Jan 1 is a Friday — **the spec's own `2027-53` counterexample (§6.1, E3)** |
| T34 | `2021` → `52` | Jan 1 is a Friday |
| T35 | `2025` → `52` | Jan 1 is a Wednesday, **not** a leap year — the near-miss of T32's branch |
| T36 | `2019` → `52` | Jan 1 is a Tuesday |
| T37 | `2024` → `52` | leap year that is *not* 53 weeks (guards "leap ⇒ 53") |
| T38 | property: for every `y` in `1900..=2100`, result ∈ `{52, 53}` **and** equals `NaiveDate(y,12,28).iso_week().week()` **and** equals `(NaiveDate(y,12,31).ordinal() - NaiveDate(y,12,31).weekday().num_days_from_monday() + 3) / 7`-style independent derivation, or at minimum equals the count of Mondays-that-start-an-ISO-week — one *independent* formula, not the same one twice |
| T39 | property: over `1900..=2100`, exactly the years whose Dec 31 is a Thursday, plus the leap years whose Dec 31 is a Friday, return 53 |

### 5.4 `parse_week_id` — §3.6, §6.1, **E3** (today = `2026-02-12`)

| # | input → expected |
|---|---|
| T40 | `"2026-07"` → `WeekId(2026, 7)` |
| T41 | `"2026-7"` → `WeekId(2026, 7)` — **E3 normalization, spec-named** |
| T42 | `"7"` → `WeekId(2026, 7)` — bare form, year from today |
| T43 | `"07"` → `WeekId(2026, 7)` |
| T44 | T40 == T41 == T42 == T43 (single assertion, the acceptance criterion literally) |
| T45 | `"2026-01"` → `WeekId(2026, 1)` |
| T46 | `"2026-53"` → ok (2026 genuinely has 53 weeks — T30) |
| T47 | `"2027-53"` → `Err(WeekId, NoSuchIsoWeek{52})` — **E3, spec-named** |
| T48 | `"2020-53"` → ok (53-week leap year — T32) |
| T49 | `"2021-53"` → `Err(WeekId, NoSuchIsoWeek{52})` |
| T50 | `"2026-54"` → `Err(WeekId, NoSuchIsoWeek{53})` (above 53 in *any* year) |
| T51 | `"0"` → `Err(WeekId, OutOfRange)` — **E3, spec-named** |
| T52 | `"2026-00"` → `Err(WeekId, OutOfRange)` |
| T53 | `"abcd"` → `Err(WeekId, Shape)` — **E3, spec-named** |
| T54 | `"2026-W07"` → `Err(WeekId, Shape)` (§2.3: "no `W`") |
| T55 | `"W07"` → `Err(WeekId, Shape)` |
| T56 | `"2026/07"` → `Err(WeekId, Shape)` |
| T57 | `"-5"` → `Err(WeekId, Shape)` |
| T58 | `"2026-"` → `Err(WeekId, Shape)` |
| T59 | `"-07"` → `Err(WeekId, Shape)` |
| T60 | `"2026-007"` → `Err(WeekId, Shape)` (week field is 1–2 digits) |
| T61 | `"999-07"` → `Err(WeekId, Shape)` (year field is exactly 4 digits) |
| T62 | `"20267"` → `Err(WeekId, Shape)` (bare form is 1–2 digits) |
| T63 | `""` → `Err(WeekId, Shape)` |
| T64 | `" 7 "` → `Err(WeekId, Shape)` (no implicit trim) |
| T65 | `"+1"` / `"-1"` → `Err(WeekId, Shape)` — §1.2 non-goal: relative notation is *not* accepted in MVP |
| T66 | `"53"` with `today = 2027-06-15` → `Err(WeekId, NoSuchIsoWeek{52})` — bare form is validated against the defaulted year, not skipped |
| T67 | `"7"` with `today = 2025-12-30` → `WeekId(2026, 7)` — **D5**: today's *ISO* year is 2026 though its calendar year is 2025. See §7.2 |
| T68 | `"7"` with `today = 2027-01-02` → `WeekId(2026, 7)` under D5 (2027-01-02's ISO year is 2026). Same flagged decision, mirrored |
| T69 | `"2026-07"` yields the identical result for `today` ∈ {2020-01-01, 2026-02-12, 2030-06-01} — the full form ignores `today` |

### 5.5 `WeekId::from_date` / `contains` — §1.3, Dec/Jan boundary

| # | input → expected |
|---|---|
| T70 | `2026-02-12` → `2026-07` (pins §7.1's "Thu 2026-02-12 / Week 2026-07") |
| T71 | `2026-01-05` → `2026-02` (pins §7.1's second worked example) |
| T72 | `2026-01-01` (Thu) → `2026-01` |
| T73 | `2025-12-29` (Mon) → `2026-01` — **Dec/Jan boundary: calendar year 2025, ISO year 2026** |
| T74 | `2025-12-28` (Sun) → `2025-52` — the day before, on the other side |
| T75 | `2026-01-04` (Sun) → `2026-01` |
| T76 | `2026-12-28` (Mon) → `2026-53` |
| T77 | `2027-01-03` (Sun) → `2026-53` — **Jan date belonging to the previous ISO year** |
| T78 | `2027-01-04` (Mon) → `2027-01` |
| T79 | `2021-01-03` (Sun) → `2020-53` |
| T80 | `2021-01-04` (Mon) → `2021-01` |
| T81 | property: for every date in `2019-01-01..=2032-12-31`, `from_date(d).contains(d)` and `start() <= d <= end()` |
| T82 | `WeekId(2026,1).contains(2025-12-29)` true; `.contains(2025-12-28)` false; `.contains(2026-01-05)` false |

### 5.6 `start` / `end` / `span` / `dates` — §7.2 header, Milestone 11's 7 rows

| # | input → expected |
|---|---|
| T83 | `2026-07.span()` → `(2026-02-09, 2026-02-15)` — **exactly §7.2's header span** |
| T84 | `2026-02.span()` → `(2026-01-05, 2026-01-11)` — consistent with §7.1's 2nd example |
| T85 | `2026-06.span()` → `(2026-02-02, 2026-02-08)` — **exactly §7.2's second worked example** |
| T86 | `2026-01.span()` → `(2025-12-29, 2026-01-04)` — **Dec/Jan crossing, the acceptance criterion** |
| T87 | `2026-53.span()` → `(2026-12-28, 2027-01-03)` — **Dec/Jan crossing in the other direction** |
| T88 | `2020-53.span()` → `(2020-12-28, 2021-01-03)` |
| T89 | `2027-01.span()` → `(2027-01-04, 2027-01-10)` |
| T90 | `dates()` for `2026-07` → the 7 dates `2026-02-09 .. 2026-02-15` in order (== §7.2's seven rows, in order) |
| T91 | `dates()` for `2026-01` → `2025-12-29 .. 2026-01-04` (crosses the year boundary mid-array) |
| T92 | property (sampled weeks incl. boundary ones): `dates()[0].weekday() == Mon`, `dates()[6].weekday() == Sun`, all 7 consecutive, `dates()[0] == start()`, `dates()[6] == end()`, `end() == start() + 6 days` |
| T93 | property: every date in `dates()` maps back through `from_date` to the same `WeekId` |

### 5.7 Display / `to_key` / `from_key` — §2.3 storage form

| # | input → expected |
|---|---|
| T94 | `WeekId(2026,7).to_string()` → `"2026-07"` (**zero-padded** — the normalization E3 requires) |
| T95 | `WeekId(2026,53).to_string()` → `"2026-53"` |
| T96 | `WeekId(2026,7).to_key()` == `to_string()` |
| T97 | `from_key("2026-07")` → `WeekId(2026,7)` |
| T98 | `from_key("2026-7")` → `Err(WeekId, Shape)` — strict, unlike `parse_week_id` |
| T99 | `from_key("7")` → `Err(WeekId, Shape)` — strict, no bare form |
| T100 | `from_key("2027-53")` → `Err(WeekId, NoSuchIsoWeek{52})` — validity still enforced on read-back |
| T101 | round-trip `from_key(w.to_key()) == w` over a sample incl. `2026-53`, `2020-53`, `2026-01` |
| T102 | `parse_week_id(w.to_string(), any_today) == w` for the same sample (lenient parser accepts the canonical form) |
| T103 | lexicographic ordering of `to_key()` strings matches `Ord` on `WeekId` for a shuffled sample — the property `week_targets`'s `TEXT PRIMARY KEY` relies on |

### 5.8 `next` / `prev` / `week_range` / `Ord` — Milestone 6's walk

| # | input → expected |
|---|---|
| T104 | `2026-07.next()` → `2026-08` |
| T105 | `2026-52.next()` → `2026-53` (2026 has 53 — no premature rollover) |
| T106 | `2026-53.next()` → `2027-01` |
| T107 | `2027-52.next()` → `2028-01` (52-week year rolls at 52) |
| T108 | `2027-01.prev()` → `2026-53` |
| T109 | `2026-01.prev()` → `2025-52` |
| T110 | `2021-01.prev()` → `2020-53` |
| T111 | property (sampled range incl. both rollover kinds): `w.next().prev() == w` and `w.prev().next() == w` |
| T112 | property: `w.next().start() == w.start() + 7 days` — independent cross-check of the arithmetic rollover against date arithmetic |
| T113 | `week_range(2026-52, 2027-02)` → `[2026-52, 2026-53, 2027-01, 2027-02]` (4 items, crosses a 53-week rollover) |
| T114 | `week_range(w, w)` → `[w]` (inclusive) |
| T115 | `week_range(2027-01, 2026-53)` → `[]` (from > to, empty, does not panic or loop forever) |
| T116 | `week_range(2026-01, 2026-53).count() == 53`; `week_range(2027-01, 2027-52).count() == 52` |
| T117 | `week_range` over a multi-year span emits strictly increasing, gapless weeks (no idle week skipped — the property §2.4/F8b depends on) |
| T118 | `2026-53 < 2027-01`, `2026-07 < 2026-08`, `2025-52 < 2026-01` |
| T119 | sorting a shuffled `Vec<WeekId>` (incl. boundary weeks) yields the same order as sorting by `start()` — validates the derived `Ord` |

### 5.9 Error shape — PLAN contract 7

| # | input → expected |
|---|---|
| T120 | every `parse_date` rejection carries `arg == ArgKind::Date` and `input` equal to the raw input verbatim |
| T121 | every `parse_week_id` / `from_key` rejection carries `arg == ArgKind::WeekId` and the verbatim input |
| T122 | `Display` of the `2027-53` error contains `"2027"`, `"52"`, and the verbatim input; is a single line; is ASCII-only (§7) |
| T123 | `Display` of the `13/02/2026` error contains the verbatim input and no `\n` |
| T124 | `DateWeekError` implements `std::error::Error` (compile-time assertion, e.g. a `fn assert_err<E: std::error::Error>()` call) |
| T125 | no parse entry point panics for a junk corpus: `["", " ", "\0", "-", "--", "999999999999", "2026-99999999999", "🙂", "2026-02-12\n", "𝟚𝟘𝟚𝟞-𝟘𝟚-𝟙𝟚"]` — each returns `Err`, none panics or overflows |

**Deliberately not tested here** (owned elsewhere, per PLAN): exit codes and
stderr plumbing (Milestones 7/8/10/11), the `Local::now()` read that supplies
`today` (the CLI layer), any DB interaction (Milestones 3/8).

---

## 6. Downstream consumers — does the shape actually fit?

| Consumer | What it needs | Provided by | Fits? |
|---|---|---|---|
| **M3** (schema) | nothing at compile time; `week_id TEXT PRIMARY KEY` must hold the canonical form | `to_key()` | yes — no code dependency, only a documented format |
| **M6** (week accounting) | walk *every* week from earliest-data week through target, incl. idle ones (§2.4, F8b) | `week_range(from, to)`, `next()`, `Ord` | yes. `week_range` is the whole walk driver; the 52/53 rollover it needs is already validated here rather than re-derived |
| **M6** | key worked-minutes and target-override maps by week | `Hash + Eq + Copy` | yes |
| **M6** | map a punch/note `date` to its week to bucket worked minutes | `WeekId::from_date(date)` | yes — infallible, so no error branch in the accounting loop |
| **M6** | find the earliest data week | `Ord` (`.min()` over the derived weeks) | yes |
| **M8** (`week target`) | parse optional `WEEK_ID`, default to current week; write the row | `parse_week_id(s, today)`, `WeekId::current(today)`, `to_key()` | yes. Upsert correctness depends on M8 writing `to_key()` and never the raw user string — **call this out in M8's plan** |
| **M8** | reject invalid week ids (E3) | `DateWeekError` + §3's Display, propagated via `anyhow` | yes — M8 prints `{e}` and exits nonzero, no per-module conversion |
| **M9** (shared rendering) | "is this the actual currently-ongoing week?" (contract 3/5) | `WeekId::current(now.date_naive()) == week` | yes — one expression, exactly the "don't reinvent it twice" goal |
| **M9** | `"<owed> left by end of <weekday>"` using **today's** weekday (F11) | `format_weekday_full(today)` | yes — takes `today`, structurally cannot accidentally use `DATE`'s weekday |
| **M10** (`status`) | header `Thu 2026-02-12` | `format_date_with_weekday` | yes |
| **M10** | the week *containing* `DATE` (§3.5's fix) | `WeekId::from_date(date)` | yes — and it is infallible, so `status` for any valid date always has a week |
| **M10** | parse `DATE` argument, E2 | `parse_date` | yes |
| **M11** (`week`) | header `Week 2026-07 (2026-02-09 - 2026-02-15)` | `Display` + `span()` + `format_date` | yes |
| **M11** | exactly 7 rows, always, Mon-first, even when empty (§7.2) | `dates() -> [NaiveDate; 7]` | yes — the array type enforces "always 7"; M11 zips it with the per-day rollup (contract 4) |
| **M11** | `(ongoing)` only on today's row, only in the current week | `dates()` + `WeekId::current(today)` | yes |
| **M11** | `WEEK_ID` accepts full id *and* bare number (F6/§3.6) | `parse_week_id` | yes |
| **M4** (storage) | `punches.date` / `notes.date` as `YYYY-MM-DD` | `format_date` | yes — though M4 may reasonably format inline; offering it here avoids a second format string |

**Contract-3 note for whoever pins the contracts:** PLAN contract 3 says the
week-accounting result carries "an explicit *is this the actual
currently-ongoing week* boolean". That boolean should be computed as
`WeekId::current(today) == week` and nothing else — flagged so M6 and M9 do
not each invent a date-range comparison.

---

## 7. Ambiguities, risks, and disagreements

### 7.1 `DATE` padding strictness is unspecified (D4) — **needs a ruling**

§6.1 requires rejecting "wrong shape", and explicitly grants padding
leniency to `WEEK_ID`, `TIME` and `DURATION` — but says nothing about
`DATE`. Two defensible readings:

- **Strict** (my proposal): the *enumerated* leniencies are exhaustive; a
  `DATE` is the same canonical `YYYY-MM-DD` string that goes into storage, so
  input and storage forms staying identical is a real simplification.
- **Lenient**: "everything else here is forgiving about padding" is a
  consistency argument, and `2026-2-12` is unambiguous.

I chose strict (T13/T14) because a lenient `DATE` requires the parser to
also decide about `26-02-12` and `2026-2-2`, none of which the spec covers.
**If the ruling flips, only T13/T14 change** and the rest of the plan stands
— the implementation note in §4.1 is written so that the shape check is one
isolated function.

### 7.2 Bare `WW` defaults to the ISO year, not the calendar year (D5) — **needs a ruling**

§3.6 says only "defaults the year to the current one." For 51 weeks of the
year these are the same. Around New Year they differ: on **2025-12-30**, the
calendar year is 2025 but the ISO year is **2026**. Under D5, `mlm week 7`
on that day means `2026-07`; under a calendar-year reading it means
`2025-07` — a week *ten months in the past*.

D5 (ISO year) is proposed because §1.3 defines a week purely in ISO terms,
and because "`mlm week` with no argument" (the current week, which on
2025-12-30 is `2026-01`) and "`mlm week 7`" should agree about what year
they are in. T67/T68 pin this. Flag it for the human; it is a one-line
change (`today.iso_week().year()` vs `today.year()`) if overruled.

### 7.3 `iso_weeks_in_year` should not lean on `from_isoywd_opt` (D6)

Probing `NaiveDate::from_isoywd_opt(y, 53, Mon).is_some()` makes the
52-vs-53 answer depend on chrono's out-of-range behaviour for that
constructor, which is an implementation detail of a dependency rather than a
property of the calendar. The Dec-28 rule is an ISO-8601 invariant and is
trivially checkable by hand. T38/T39 additionally cross-check it against an
independently-derived formula over 200 years, so a chrono upgrade cannot
silently move this.

Also note: `from_isoywd_opt` is used in `start()`, where the invariant makes
it total — so the dependency-behaviour risk does not reappear there.

### 7.4 Week iteration is a scope addition (D8)

PLAN's Milestone 2 scope lists parsing, normalization, validity, formatting,
and Mon-Sun span — not `next`/`prev`/`week_range`. But PLAN's Milestone 6
scope ("walk every ISO week in sequence … including weeks with zero data")
*requires* week iteration, and correct iteration requires exactly the
52/53-week knowledge this milestone owns. Leaving it to M6 would duplicate
`iso_weeks_in_year` in a second worktree, which is precisely the failure
mode the "Interface contracts" section warns about. I claim it here and
flag it so M6's planner knows not to build it.

### 7.5 Weekday/date *formatting* ownership overlaps M9/M10/M11 (D9)

`format_date_with_weekday` and `format_weekday_full` are rendering helpers,
which PLAN nominally puts in Milestones 9/10/11. But M10 (`status` header)
and M11 (`week` rows) both need the abbreviated form, and M9 needs the full
name — three consumers, one format string. Centralizing it here mirrors
PLAN's own reasoning for extracting Milestone 9. Low risk: if the human
prefers, these three functions move to M9 with no other change to this plan.

### 7.6 Year range clamped to `1000..=9999`

The `YYYY-WW` / `YYYY-MM-DD` shapes are exactly four digits, so parsing
cannot produce anything outside `1000..=9999` anyway. `WeekId::new` enforces
the same bound so that `Display`'s `{:04}` can never widen and break the
storage key format. `chrono::NaiveDate` supports a far wider range; that
extra range is unreachable through any MVP code path. Not tested beyond the
shape tests (T61).

### 7.7 `ToSql`/`FromSql` for `WeekId` deliberately omitted

Implementing them in `date.rs` would couple this module to `rusqlite` for
the benefit of one or two call sites in M8. `to_key()`/`from_key()` cover
those call sites at the cost of one `.to_key()` per bind. Flagged as a
reasonable, cheap follow-up for whichever of M3/M8 wants it — but it should
be a deliberate choice, not something M8 discovers it needs and adds ad hoc
in a third place.

### 7.8 `src/error.rs` is a shared file across two parallel worktrees

The single genuine merge hazard in this milestone. §3 specifies the file
completely — including the `Time` and `Duration` variants this milestone
never constructs — so Milestones 1 and 2 can each create it independently
and land byte-identical (or trivially union-mergeable) content. **Action
required before opening the worktrees: paste §3 verbatim into Milestone 1's
plan.** If that does not happen, expect two incompatible error enums and a
rework of M7/M8's CLI wiring — the exact outcome contract 7 exists to
prevent.

### 7.9 No disagreement with SPEC.md found

§6.1's "`2027-53` is invalid if 2027 only has 52 ISO weeks" checks out: 2027
begins on a Friday and is not a leap year, so it has **52** ISO weeks. The
spec's example is correct as written. Likewise §7.2's `Week 2026-07
(2026-02-09 - 2026-02-15)` and §7.1's `Mon 2026-01-05` / `Week 2026-02` are
both genuine ISO mappings — they are used as fixtures above (T83, T71, T84)
rather than merely as illustrations. Note that 2026 itself is a **53-week**
year, so `2026-53` must be *accepted* (T46) even though the neighbouring
spec example is about a rejection.
