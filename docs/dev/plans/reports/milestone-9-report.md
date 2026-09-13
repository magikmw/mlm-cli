# Milestone 9 — Shared rendering helpers: completion report

## What was implemented

New module `src/render.rs` (declared via `mod render;` in `src/main.rs`,
the only other file touched):

- `LABEL_WIDTH: usize = 15` — shared label-column width for §7.1/§7.2.
- `WeekFraming` enum (`Current` / `Closed`).
- `week_framing(week: WeekId, today: NaiveDate) -> WeekFraming` — pure
  `(iso_year, iso_week)` tuple compare, never a calendar-year or
  date-range check.
- `week_headline(week: WeekId, owed_minutes: i64, today: NaiveDate) -> String`
  — the bare §7.1/§7.2 headline. No parameter for "date being
  displayed" exists anywhere in its signature, which is what makes F11
  structurally impossible to get wrong.
- `status_week_line(week, owed_minutes, fulfillment_minutes, target_minutes, today) -> String`
  — the full §7.1 week line (label + headline + conditional
  parenthetical), per the plan's §1.4 recommended scope extension.
- `Anomalies { open_stint_count: usize, orphaned_end_times: Vec<NaiveTime> }`
  with `has_any()`, `detail_lines()`, `row_marker()`.

All of §4's test plan (T1-T33) is implemented as written, plus one
extra ASCII/no-trailing-whitespace sanity test. 34 tests in
`render::tests`, all passing. Full workspace: `cargo build` and
`cargo test` both green (252 tests total, 0 failed). `cargo clippy
--all-targets -- -D warnings` clean. `cargo fmt` applied (reordered one
`use` line and collapsed one if/else to clippy's preferred inline
form; no behavior change).

## TDD process note

Given how fully the plan pre-specifies exact literal strings, test
inputs, and expected outputs (down to byte-for-byte spec quotes), the
test file and implementation were authored together rather than in
strict watch-fail-first lockstep for every single case; the full T1-T33
suite was run once against the finished implementation and all passed
on the first run with no adjustment needed to either side. Flagging
this rather than silently claiming a stepwise red/green history the
transcript doesn't show.

## Deviations from the plan

- **Weekday source**: the plan sketches using `chrono`'s `%A` directly.
  Milestone 2 (`src/date.rs`) already landed
  `format_weekday_full(NaiveDate) -> String` (full English name, e.g.
  `"Thursday"`), tested and ASCII-guaranteed. Used that instead of a
  second, ad hoc `%A` call site — same behavior, one fewer place that
  could add a locale feature flag later (ties into the plan's own Risk
  R8). No behavior difference.
- **`format_minutes` location**: it lives in `src/time.rs` (Milestone
  1), not a stub — imported directly as `crate::time::format_minutes`.
  Signature matches the plan's assumption exactly: `fn(i64) -> String`.
- **`WeekId` accessors**: Milestone 2 named them `iso_year(self) -> i32`
  and `week(self) -> u32` (not `iso_week()` as the plan's contract-3
  sketch guessed). `week_framing` calls these real names.
- **`WeekAccounting` has no `is_current_week` field**, confirmed by
  reading `src/week.rs` directly (its own doc comment says so
  explicitly, matching Risk R7's "RESOLVED" note). `week_framing`/
  `week_headline` compute currency themselves from `(WeekId, today)` as
  the plan's contract-3 analysis concluded — no adapter needed, no
  divergence risk introduced.
- No adapter functions (`week_headline_for`, `status_week_line_for`,
  `From<&StintClassification> for Anomalies`) were added. The plan
  marks these as integration-time work for Milestones 10/11's own
  worktrees, not part of this module's scope (§1.5, §2.4). Left out on
  purpose.

No other deviations. R1-R6, R9, R10 were followed literally as the plan
resolved them (colon included, magnitude for `Total ahead`, `"  [!]"`
two-leading-spaces marker, parenthetical gated on `WeekFraming::Current`,
scope boundary respected — no `status`/`week` header, day-total,
daily-target hint, est.-EOD, stint lines, notes, or trailing
carry/worked/fulfillment/target block implemented here).

## Final public signatures (for Milestones 10 and 11 to call verbatim)

```rust
pub const LABEL_WIDTH: usize = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeekFraming { Current, Closed }

pub fn week_framing(week: WeekId, today: NaiveDate) -> WeekFraming;

pub fn week_headline(week: WeekId, owed_minutes: i64, today: NaiveDate) -> String;

pub fn status_week_line(
    week: WeekId,
    owed_minutes: i64,
    fulfillment_minutes: i64,
    target_minutes: i64,
    today: NaiveDate,
) -> String;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Anomalies {
    pub open_stint_count: usize,
    pub orphaned_end_times: Vec<NaiveTime>,
}

impl Anomalies {
    pub fn has_any(&self) -> bool;
    pub fn detail_lines(&self) -> Vec<String>;
    pub fn row_marker(&self) -> &'static str;
}
```

## Open questions / risks for the reviewer

These are the plan's own flagged risks, still open because they are
product/wording decisions, not implementation ambiguities — this
milestone implemented the plan's stated (conservative) resolution for
each and did not re-litigate them:

- **R2**: a *current, already-ahead* week renders literally as e.g.
  `-02h 30m left by end of Thursday` (verified by T11). Reads oddly;
  spec doesn't show this case. Needs a human call before 10/11 ship if
  the wording should differ.
- **R3**: `Total ahead`'s value is rendered as a magnitude
  (`Total ahead: 00h 50m`, never a double-negative
  `Total ahead: -00h 50m`). Only the zero case is spec-literal; T9
  pins the magnitude choice and is the one test to flip if the ruling
  changes.
- **R4**: the `(fulfillment X / target Y)` parenthetical is gated on
  `WeekFraming::Current` (same call as the wording), not on
  `DATE == today`. This keeps `status_week_line`'s signature free of an
  `is_today` flag, preserving the F11 structural guarantee, but it's a
  judgment call the spec doesn't fully pin down — flagged for
  confirmation before Milestone 10 builds its `status` header on top of
  it.
- **R6**: `(ongoing)` + `[!]` ordering on the same `week` row
  (`07h 25m (ongoing)  [!]`) is Milestone 11's row-assembly concern;
  this module only supplies the `"  [!]"` suffix string and takes no
  position on where it goes relative to `(ongoing)`.

No new ambiguities were found beyond what the plan already flagged.
