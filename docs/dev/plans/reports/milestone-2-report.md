# Milestone 2 — completion report

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.


**Branch**: `milestone-2`. **Files touched**: `src/date.rs` (new),
`src/main.rs` (added `mod date;` only). No other module touched.

## What was implemented

`src/date.rs`, per `plans/milestone-2-date-week.md`:

- **Errors** (local to this module, PLAN contract 7 final form):
  `ArgKind {Date, WeekId}`, `Cause {Shape, OutOfRange,
  NoSuchCalendarDate, NoSuchIsoWeek {iso_year, weeks_in_year}}`,
  `DateWeekError {arg, input, cause}` + hand-written `Display` (single
  ASCII line, no trailing period/newline) and `std::error::Error`.
- **DATE**: `parse_date` (strict zero-padded `YYYY-MM-DD` via an explicit
  byte shape check, then `from_ymd_opt` — deliberately not
  `parse_from_str`), `format_date`, `format_date_with_weekday`
  (`Thu 2026-02-12`), `format_weekday_full` (`Thursday`).
- **`iso_weeks_in_year(i32) -> u32`** via the Dec-28 ISO invariant (D6).
- **`WeekId`** (private `iso_year: i32`, `week: u8`;
  `Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash`):
  `new(iso_year, week, input)`, `from_date`, `current`, `iso_year()`,
  `week()`, `to_key()`, `from_key()` (strict), `start()`, `end()`,
  `span()`, `dates() -> [NaiveDate; 7]`, `contains()`, `next()`,
  `prev()`, `Display` = `YYYY-WW` zero-padded.
- **`parse_week_id(s, today)`** — full `YYYY-WW` (year exactly 4 digits,
  week 1–2 digits, unpadded accepted) or bare `WW`; bare form defaults to
  `today.iso_week().year()` (D5).
- **`WeekRange` + `week_range(from, to)`** — inclusive, empty when
  `from > to`, correct 52↔53 rollover.

## Verification (all run in this worktree)

- `cargo build` — clean (0 errors). 3 pre-existing `dead_code` warnings in
  `src/time.rs`, untouched by this milestone.
- `cargo test` — **42 test functions, 42 passed, 0 failed**. They cover
  the plan's full T1–T125 case list (cases grouped per behaviour and
  annotated with their T numbers), including the 200-year property checks
  (T38/T39) and the date-sweep properties (T24, T81, T92/T93, T111/T112).
- `cargo clippy --all-targets -- -D warnings` — **zero findings in
  `src/date.rs`**. The command as written still *fails*, but only on three
  pre-existing `dead_code` errors in `src/time.rs` (`parse_hm`, `between`,
  `format_duration`) that are present on the base commit before this
  milestone and belong to Milestone 1. With `-A dead_code` the whole
  workspace is clean. `src/date.rs` carries a module-level
  `#![allow(dead_code)]` (with a comment) because its consumers land in
  Milestones 4/6/8/9/10/11.
- `cargo fmt --check` — clean.

## TDD note (honest process record)

Tests and implementation were first written in one pass, which is not the
prescribed order. To recover a real RED step, the entire implementation was
then replaced with `todo!()` bodies and the suite run: **41 of 42 tests
failed with "not yet implemented"**; the single pass was T124, the
compile-time `std::error::Error` trait assertion, which correctly needs no
body. The implementation was then restored and taken to green. Two genuine
test bugs were caught by that cycle (a bogus leap-year expression, and the
`Display` defect below), so the red step did real work.

## Deviations from the plan

1. **`Cause::NoSuchIsoWeek` carries `iso_year` in addition to
   `weeks_in_year`** (plan §3 specified only `weeks_in_year`). Reason: the
   plan's own message wording (`2027 has only 52 ISO weeks`) needs the
   year, and for a bare `WW` input the year is *not* in the input string —
   recovering it by re-parsing `input` produced the wrong message
   (`"53" has only 52 ISO weeks` → rendered `53 has only 52 ISO weeks`).
   A test now pins the bare-form message:
   `invalid WEEK_ID "53": 2027 has only 52 ISO weeks`.
   Downstream impact: only code that *constructs or matches* this variant
   (nobody outside this module today).
2. **`next()`/`prev()` panic at the `1000..=9999` year bounds** rather than
   saturating — the plan explicitly said pick one and document it; it is
   documented on both methods. Unreachable via any CLI path.
3. `parse_date` validates shape by byte inspection rather than `chrono`'s
   `parse_from_str`, as the plan's §4.1 implementation note instructs (not
   a deviation, restated because it is the part a reviewer would question).
4. Test functions are grouped (42 functions covering 125 planned cases)
   instead of one function per T-number; every case is present and tagged
   with its T id in a comment.

## For downstream milestones (6, 8, 9, 11) — final `WeekId` API

```rust
WeekId::new(iso_year: i32, week: u32, input: &str) -> Result<WeekId, DateWeekError>
WeekId::from_date(date: NaiveDate) -> WeekId          // infallible
WeekId::current(today: NaiveDate) -> WeekId
w.iso_year() -> i32        w.week() -> u32            // by value, Copy
w.start() -> NaiveDate     w.end() -> NaiveDate       // Mon, Sun
w.span() -> (NaiveDate, NaiveDate)
w.dates() -> [NaiveDate; 7]                           // Mon-first, always 7
w.contains(date: NaiveDate) -> bool
w.next() -> WeekId         w.prev() -> WeekId
w.to_key() -> String       WeekId::from_key(&str) -> Result<WeekId, DateWeekError>
impl Display for WeekId    // "2026-07"
week_range(from: WeekId, to: WeekId) -> WeekRange     // inclusive, Iterator<Item = WeekId>
iso_weeks_in_year(iso_year: i32) -> u32
```

Notes that matter to them:

- Contract 9 names (`start`, `from_date`, `next`, `iso_year`, `week`) are
  exactly as specified. Fields are private; every accessor takes `self` by
  value.
- `Ord` is derived over `(iso_year, week)` and is chronological (pinned by
  a test against sorting by `start()`), so M6 can use
  `while cursor <= target` / `.min()` directly.
- **M8 must persist `to_key()`, never the raw user string** — `2026-7` and
  `2026-07` would otherwise be two `week_targets` rows for one week.
  `from_key` is strict on purpose; `parse_week_id` is the lenient
  user-input path and needs `today`.
- M9's "is this the current week" is `WeekId::current(today) == week` and
  nothing else (contract 3 note in the plan).
- M11's 7-row guarantee comes from `dates()`'s array type.

## Open questions / risks for the reviewer

1. **D4 (strict `DATE` zero-padding)** is still an unresolved spec
   ambiguity — §6.1 grants padding leniency to `WEEK_ID`/`TIME`/`DURATION`
   and is silent on `DATE`. Implemented strict. If overruled, only
   `parse_date_rejects_wrong_shapes`' `"2026-2-12"`/`"2026-02-2"` entries
   and `has_ymd_shape` change.
2. **D5 (bare `WW` defaults to today's *ISO* year)** likewise needs a human
   ruling; implemented as ISO year, pinned by two tests. One-line change
   (`today.iso_week().year()` → `today.year()`) if overruled.
3. `WeekId::from_date` does not enforce the `1000..=9999` year bound (only
   `new`/`from_key` do), since `NaiveDate` can hold years outside it. No
   MVP path reaches such a date (plan §7.6).
4. `ToSql`/`FromSql` for `WeekId` deliberately omitted (plan §7.7) — M3/M8
   should decide deliberately rather than adding it ad hoc.
5. Plan §7.8's `src/error.rs` merge hazard is moot: no shared error file
   was created, per contract 7's final form.
6. `cargo clippy --all-targets -- -D warnings` will keep failing repo-wide
   until Milestone 1 gives `src/time.rs`'s functions call sites (or an
   allow). Not fixed here to avoid a cross-worktree conflict.
