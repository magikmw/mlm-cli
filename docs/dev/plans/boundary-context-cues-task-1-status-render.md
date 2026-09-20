# Task 1 low-level plan: `status`/`render` — span suffix, header suffix, backdated caption, Day total match, carry-in wiring

Changeset: `boundary-context-cues`. This changeset has exactly one task
— no siblings to hand work to or receive it from.

## What was read (in full)

- `docs/dev/plans/boundary-context-cues-plan.md` (284 lines) — the
  changeset plan, single task, four rounds of adversarial review
  folded in (most recently: the 2-task split was collapsed to 1 task
  because `status_week_line`'s call-site coupling makes a
  `render.rs`-only intermediate not compile).
- `docs/dev/specs/2026-09-20-boundary-context-cues.md` (185 lines) —
  the locked spec, §§1-8.
- `src/status.rs` (1317 lines, full file) — `StintLine`, `StatusView`,
  `stint_line()`, `day_total_line()`, `render()`, `week_line()`,
  `build_stint_lines()`, `resolve()`, and the full existing test
  module (golden tests T1-T13 plus the `resolve_*`/splice tests).
- `src/render.rs` (589 lines, full file) — `status_week_line`,
  `week_headline`, `WeekFraming`/`week_framing`, `Anomalies`, and the
  full test module including `t33_every_produced_string_is_ascii`.
- `src/stint.rs` (1131 lines, full file) — `classify`, `classify_at`,
  `splice_candidate`, `DayStints`, `Stint`/`OpenStint`/`OrphanedEnd`,
  and the full test module (including the `splice_*` tests pinning
  `splice_candidate`'s three-part gate).
- `clippy.toml` — one setting,
  `cognitive-complexity-threshold = 15` (stricter than clippy's
  default 25).
- `.githooks/pre-commit` — gates a commit on (1) `cargo fmt --check`,
  (2) no function over the clippy.toml cognitive-complexity threshold
  (via `clippy::cognitive_complexity`), (3) no regression in
  `cargo-llvm-cov` line coverage vs. `coverage-baseline.json` (bootstrap
  or improve-only; missing tooling degrades to SKIP, never a hard
  failure).
- `AGENTS.md` "Verifying changes" section: `cargo fmt`, `cargo build`,
  `cargo test`, `cargo clippy --all-targets -- -D warnings`, then two
  manual smoke-test `cargo run` invocations against a scratch DB
  (`MLM_DB_PATH=/tmp/mlm-check.db`).
- `src/week.rs` (grepped, not full read — out of this task's file
  scope): confirmed `WeekAccounting` has `pub carry_in: i64` (line 66)
  and `fulfillment = worked + carry_in` is enforced at construction
  (line 180), matching the changeset plan's claim verbatim. Not
  touched by this task.

## Scope

This task owns `src/render.rs` and `src/status.rs` exclusively, per
the changeset plan's file-ownership table. There are no sibling tasks
in this changeset — nothing to hand off, nothing incoming. The one
permitted edit outside those two files is `src/stint.rs`'s single
visibility keyword on `splice_candidate` (changeset plan's named
exception, "no new public API" clause) — call sites for it live in
`src/status.rs`, inside this task's own scope.

## Current facts (exact, cited to source)

### `src/render.rs`

- `pub const LABEL_WIDTH: usize = 15;` (line 24).
- `pub fn status_week_line(week: WeekId, owed_minutes: i64, fulfillment_minutes: i64, target_minutes: i64, today: NaiveDate) -> String` (lines 76-82). Body (lines 83-100): builds `label`, `headline` via `week_headline`, and a `parenthetical` that is `" (fulfillment {} / target {})"` under `WeekFraming::Current` and `String::new()` under `WeekFraming::Closed`, then `format!("{:<w$}{}{}", label, headline, parenthetical, w = LABEL_WIDTH)`.
- `pub fn week_headline(week: WeekId, owed_minutes: i64, today: NaiveDate) -> String` (line 60) — untouched by this task.
- `pub fn week_framing(week: WeekId, today: NaiveDate) -> WeekFraming` (line 44) — untouched.
- `format_minutes` imported from `crate::time` (line 19) — sign convention: `-` for negative, nothing for zero/positive (confirmed by `t12_negative_zero_never_renders_with_a_minus_sign`, lines 294-298, and `t9_negative_owed_renders_as_a_magnitude_under_total_ahead`).
- `t33_every_produced_string_is_ascii` (lines 569-588): builds an array of `headlines` (5 `week_headline` calls, 2 `status_week_line` calls) and asserts `.is_ascii()` on each, plus every `Anomalies::detail_lines()`/`row_marker()` result from `fixtures()`.

### `src/status.rs`

- `pub enum StintEnd { At(NaiveTime), Now }` (lines 68-71).
- `pub struct StintLine { pub start: NaiveTime, pub end: StintEnd, pub duration_minutes: i64 }` (lines 75-79), `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`.
- `pub struct StatusView` (lines 85-111): `header: String`, `day_total_minutes: i64`, `has_open_stint: bool`, `daily_target: Option<DailyTargetHint>`, `weekday_name: Option<String>`, `eod: Option<EodState>`, `week_line: String`, `anomaly_lines: Vec<String>`, `stints: Vec<StintLine>`, `notes: Vec<String>`. `#[derive(Debug, Clone, PartialEq, Eq)]` — no `Copy` (holds `String`/`Vec`).
- `fn day_total_line(view: &StatusView) -> String` (lines 122-152): builds the `"Day total:"` line, appends `" (+ ongoing)"` when `view.has_open_stint` (lines 129-131), then the daily-target/EOD segments.
- `fn stint_line(line: &StintLine) -> String` (lines 155-167): builds `range`, sets `ongoing = matches!(line.end, StintEnd::Now)`, returns `format!("  {:<11}  ({}{})", range, format_minutes(line.duration_minutes), if ongoing { ", ongoing" } else { "" })`.
- `pub fn render(view: &StatusView) -> String` (lines 172-199): assembles `header`, blank, `day_total_line`, `week_line`, then conditionally anomaly/stint/notes sections.
- `fn week_line(acct: &WeekAccounting, today: NaiveDate) -> String` (lines 211-213): `render::status_week_line(acct.week, acct.owed, acct.fulfillment, acct.target, today)` — 5 positional args, no `carry_in` passed today.
- `fn build_stint_lines(day: &DayStints) -> Vec<StintLine>` (lines 238-256): one parameter, iterates `day.completed` then `day.open`, sorts by `start`.
- `pub fn resolve(date_arg: Option<&str>, now: DateTime<Local>, conn: &Connection) -> anyhow::Result<StatusView>` (lines 326-396). Exact order relevant to this task:
  - `today` bound line 331, `target_date` line 332-335, `is_today` line 336.
  - `punches`/`notes`/`prev_punches`/`next_punches` fetched lines 338-341.
  - `now_utc` line 343.
  - `let day = stint::classify_at(&prev_punches, &punches, &next_punches, now_utc);` — line 344. This is the "existing `classify_at` call" the changeset plan's pitfall warns not to reuse for the §3 fresh-`classify` check.
  - `day_total_minutes`/`has_open_stint` lines 346-347.
  - `week_line_str = week_line(&acct, today)` line 352 (after `acct` built line 351).
  - `stints = build_stint_lines(&day)` line 381 — called near the end, well after `today`/`target_date`/`is_today` are all bound.
  - `StatusView { ... }` struct literal built lines 384-395.
- Existing golden tests: `t1`..`t13` (render-level, lines 502-1006) and `resolve_*` (I/O-level, lines 1012 onward), including the midnight-splice tests (`resolve_midnight_splice_earlier_date_shows_completed_stint_no_anomaly`, `resolve_midnight_splice_later_date_shows_no_orphan_anomaly`, `resolve_midnight_boundary_still_flags_non_1to1_shapes`, `resolve_midnight_splice_reflected_in_week_line`) that already exercise the exact splice shapes §3 needs, but assert only on `anomaly_lines`/`stints`/`day_total_minutes`/`week_line`, never on any header text — confirming no existing fixture already depends on a header suffix.
- `week_line_matches_status_week_line_field_for_field` (lines 1309-1315) is the second call site the changeset plan's "Ladder-altitude check" names as the reason a 2-task split doesn't compile: it calls `render::status_week_line(acct.week, acct.owed, acct.fulfillment, acct.target, today())` directly with the current 5-arg signature.

### `src/stint.rs`

- `pub fn classify(punches: &[Punch], now: DateTime<Utc>) -> DayStints` (line 124) — public already; no change.
- `pub fn classify_at(prev_punches: &[Punch], punches: &[Punch], next_punches: &[Punch], now: DateTime<Utc>) -> DayStints` (line 256) — public already; no change. Its (prev, day) branch (lines 297-303) only ever calls `day.orphaned_ends.remove(0)`, never touches `day.open` — confirms the changeset plan's "Facts gathered" claim. Its (day, next) branch (lines 306-319) is the one that mutates `day.open`.
- `fn splice_candidate(earlier_open_count: usize, later: &DayStints, later_punches: &[Punch]) -> bool` — **line 334**, currently module-private (no `pub` keyword at all). Body (lines 335-341): returns `false` unless `earlier_open_count == 1 && later.orphaned_ends.len() == 1`, then checks whether `later_punches`'s `(at_utc, kind, id)`-minimal punch's `id` equals `later.orphaned_ends[0].punch.id`.
  - **The visibility edit**: line 334, `fn splice_candidate(...)` → `pub(crate) fn splice_candidate(...)`. Nothing else on that line or in the function body changes.
- `DayStints::compute_has_anomaly` (line 77, private `fn`, not exported) — the module's own precedent the changeset plan cites for "share the gate function instead of re-deriving the three conditions in `status.rs`." Confirms the convention exists, but this is informational only — no edit needed here.
- `classify_at`'s two `splice_candidate` call sites (line 301: `splice_candidate(prev.open.len(), &day, punches)`; line 307: `splice_candidate(day.open.len(), &next, next_punches)`) are untouched — this task calls `splice_candidate` independently from `status.rs`, not through `classify_at`.

## Implementation plan

### 1. `status_week_line`'s new `carry_in_minutes: i64` parameter (`src/render.rs`)

New signature:

```rust
pub fn status_week_line(
    week: WeekId,
    owed_minutes: i64,
    fulfillment_minutes: i64,
    target_minutes: i64,
    carry_in_minutes: i64,
    today: NaiveDate,
) -> String {
```

Parameter placement: immediately after `target_minutes`, before `today`
— keeps the existing four minute-typed params contiguous (matching
their existing adjacency, already flagged as a transposition risk in
this module's own doc comment) and puts the one new minute param
next to them rather than splitting them across `today`. `today` stays
last since every other function in this module (`week_headline`,
`week_framing`) also takes it last — no reason to break that
convention for one new parameter.

Body change — only the `WeekFraming::Current` arm of the
`parenthetical` match:

```rust
let parenthetical = match week_framing(week, today) {
    WeekFraming::Current => {
        let worked_minutes = fulfillment_minutes - carry_in_minutes;
        if carry_in_minutes == 0 {
            format!(
                " (fulfillment {} / target {})",
                format_minutes(fulfillment_minutes),
                format_minutes(target_minutes)
            )
        } else {
            format!(
                " (fulfillment {} = worked {} + carry-in {} / target {})",
                format_minutes(fulfillment_minutes),
                format_minutes(worked_minutes),
                format_minutes(carry_in_minutes),
                format_minutes(target_minutes)
            )
        }
    }
    WeekFraming::Closed => String::new(),
};
```

- `worked_minutes = fulfillment_minutes - carry_in_minutes`: this is
  the changeset plan's required derivation (§4.2's `worked = fulfillment
  - carry_in`), computed unconditionally (cheap, and needed only in
  the non-zero branch, but computing it once above the `if` keeps the
  two arms symmetric and avoids a duplicate subtraction expression).
  Do NOT accept `worked_minutes` as a new parameter — the changeset
  plan is explicit that this must be derived, not passed, so it can
  never diverge from `WeekAccounting.worked` by construction (enforced
  in `src/week.rs`, not this file — out of scope here).
- Each of `format_minutes(fulfillment_minutes)`,
  `format_minutes(worked_minutes)`, `format_minutes(carry_in_minutes)`
  is called independently on its own signed `i64` — `format_minutes`
  already applies "`-` for negative, nothing for zero/positive" per
  its existing use everywhere in this file, so no new sign-handling
  logic is written. This is what makes the three spec worked examples
  (deficit, surplus, fulfillment-goes-negative) fall out automatically:
  - Deficit example (spec §5, `carry_in = -02h 10m = -130`,
    `fulfillment = 29h 15m = 1755`, `worked = 1755 - (-130) = 1885 =
    31h25m`): `worked_minutes` computes to exactly `31h 25m` and
    `format_minutes(-130)` prints `-02h 10m` — matches
    `"fulfillment 29h 15m = worked 31h 25m + carry-in -02h 10m / target 40h 00m"`
    verbatim.
  - Fulfillment-negative example (spec §5 second block:
    `carry_in = -05h 00m = -300`, `worked = 03h 00m = 180`,
    `fulfillment = worked + carry_in = -120 = -02h 00m`): passed in as
    `fulfillment_minutes = -120`; `format_minutes(-120)` prints
    `-02h 00m` — matches `"fulfillment -02h 00m = worked 03h 00m +
    carry-in -05h 00m / target 40h 00m"` verbatim. No special-casing
    needed for `fulfillment < 0` — it's just another `i64` through the
    same `format_minutes` call.
  - Surplus example (`carry_in = 00h 50m = 50`, positive,
    `worked = 39h 10m = 2350`, `fulfillment = 2400 = 40h 00m`):
    `format_minutes(50)` prints `00h 50m` with no `+` — matches
    `"fulfillment 40h 00m = worked 39h 10m + carry-in 00h 50m / target
    40h 00m"` verbatim (no `+` sign is ever printed by
    `format_minutes`, confirmed by the existing `t9`/`t11`/`t12` tests
    in this file's own test module — this task adds no new sign logic).
- `carry_in_minutes == 0` takes the exact pre-existing one-line
  branch, byte-identical to today's output — this is the "zero unchanged"
  half of acceptance criterion §5.
- This whole change is confined to the `WeekFraming::Current` arm.
  `WeekFraming::Closed` arm (`String::new()`) is untouched, so a
  past/future week's plain line gains no parenthetical regardless of
  `carry_in_minutes` — matches spec §5's closing paragraph ("gains none
  here").

### 2. `splice_candidate`'s visibility change (`src/stint.rs`)

Line 334, change

```rust
fn splice_candidate(earlier_open_count: usize, later: &DayStints, later_punches: &[Punch]) -> bool {
```

to

```rust
pub(crate) fn splice_candidate(earlier_open_count: usize, later: &DayStints, later_punches: &[Punch]) -> bool {
```

No other change to this function — same three parameters, same body,
same three-part gate. `stint.rs`'s own two call sites (lines 301, 307)
need no edit since `pub(crate)` is a strict widening (private code can
still call a `pub(crate)` item in the same crate). Existing `stint.rs`
tests (`splice_prev_two_opens_stays_unspliced`,
`splice_day_two_orphans_stays_unspliced`,
`splice_orphan_not_first_punch_of_day_stays_unspliced`, etc.) exercise
`classify_at`, not `splice_candidate` directly, and are unaffected by
a visibility-only change — no `stint.rs` test needs editing for this
step.

### 3. The two new `classify()` calls and the header-suffix gate (`resolve()`, `src/status.rs`)

Insert this block in `resolve()` immediately after line 344 (the
existing `let day = stint::classify_at(...)` call), i.e. between the
existing `day` binding and the `day_total_minutes`/`has_open_stint`
lines that follow it:

```rust
let receiving_end_time = {
    let prev_classified = stint::classify(&prev_punches, now_utc);
    let fresh_target_classified = stint::classify(&punches, now_utc);
    stint::splice_candidate(prev_classified.open.len(), &fresh_target_classified, &punches).then(
        || {
            fresh_target_classified.orphaned_ends[0]
                .punch
                .at_utc
                .with_timezone(&Local)
                .time()
        },
    )
};
```

Exact requirements this satisfies, called out explicitly:

- **Two fresh `classify()` calls, not `classify_at`, not the `day`
  variable**: `prev_classified = stint::classify(&prev_punches,
  now_utc)` and `fresh_target_classified = stint::classify(&punches,
  now_utc)`. Neither reuses `day` (bound at line 344 by the pre-existing
  `classify_at` call) — this is the exact pitfall the changeset plan's
  "Facts gathered" section calls out: `day` has already had its
  matching orphan removed by `classify_at`'s (prev, day) branch
  whenever the splice condition holds, so testing against `day` would
  make the header suffix never fire. `fresh_target_classified` is an
  entirely separate `DayStints` value, never assigned to or read from
  `day`.
- **`splice_candidate`'s three arguments, in order**:
  `prev_classified.open.len()` (prev's own open count — "did the
  previous date have exactly one open stint"), `&fresh_target_classified`
  (the fresh target-date classification, for its `orphaned_ends`), and
  `&punches` (the raw target-date punches — `splice_candidate` needs the
  raw slice, not just the `DayStints`, to compute the
  chronologically-first punch itself, per its own doc comment at
  `src/stint.rs` lines 330-333).
- **At most one suffix**: `splice_candidate` returns a single `bool`,
  gating a single `Option<NaiveTime>` — structurally impossible to
  produce more than one suffix, matching §4.3.1's 1:1 guarantee.
- **The suffix's `HH:MM`** is `fresh_target_classified.orphaned_ends[0].punch.at_utc`
  converted to local time — this is safe to index `[0]` unconditionally
  only inside the `.then(...)` closure, which only runs when
  `splice_candidate` returned `true`, which requires
  `fresh_target_classified.orphaned_ends.len() == 1` (checked inside
  `splice_candidate` itself, `src/stint.rs` line 335) — so the index
  never panics.
- Uses `then(|| ...)` (not `then_some(...)`) because the closure needs
  a value computed only when the condition holds and indexing is
  guarded by that condition — `then_some` would evaluate its argument
  eagerly regardless of the boolean, which would panic on the
  index whenever the gate is false and `fresh_target_classified`
  happens to have zero orphans (the common case).

`StatusView` gains one new field (see step 6) carrying the pre-rendered
suffix string (not just the `Option<NaiveTime>`) — built in `resolve()`
from `receiving_end_time`:

```rust
let receiving_date_suffix = receiving_end_time
    .map(|t| format!("  ({} continues previous day's stint)", t.format("%H:%M")));
```

`render()`'s header assembly then simply appends this when `Some`
(step 6 below) — no bracket marker, no extra line, matching spec §3's
`Fri 2026-02-13  (00:45 continues previous day's stint)` example
verbatim (two spaces before the opening paren, matching the spec's
example spacing).

### 4. `StintLine`'s new fields and `stint_line()`'s new branches (`src/status.rs`)

New `StintLine`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StintLine {
    pub start: NaiveTime,
    pub end: StintEnd,
    pub duration_minutes: i64,
    /// True for a completed stint whose `start.date != end.date`
    /// (§2). Always `false` for an open stint (`StintEnd::Now`) —
    /// §2's span cue applies to completed stints only.
    pub spans_to_next_day: bool,
    /// Only meaningful when `end == StintEnd::Now`; `None` for a
    /// completed stint. §4's three-way open-stint age classification.
    pub open_stint_age: Option<OpenStintAge>,
}
```

Stays `Copy` — both new fields (`bool`, `Option<OpenStintAge>` where
`OpenStintAge` is a plain fieldless enum, see below) are themselves
`Copy`, so the existing `#[derive(..., Copy, ...)]` needs no removal.

New enum (placed near `StintEnd`, above `StintLine`):

```rust
/// §4's three-way classification of how stale an open stint's date is,
/// relative to `today`. Threaded into every `StintLine` via
/// `build_stint_lines`'s new parameter, and stored again, unchanged, on
/// `StatusView` for `day_total_line()` to read directly (§4's Day-total
/// match requirement) — see the "Facts gathered" note in the changeset
/// plan: `StintLine`s are built before `StatusView` exists, so this is
/// not a copy *from* `StatusView`, it flows the other way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenStintAge {
    /// `target_date == today` — today's own open stint (§4, unchanged
    /// rendering).
    Today,
    /// `target_date == today - 1 day` — keeps its duration, gains a
    /// caption.
    Yesterday,
    /// `target_date <= today - 2 days` — duration dropped, `(unclosed)`.
    TwoOrMoreDaysBack,
}
```

`stint_line()`'s new body:

```rust
fn stint_line(line: &StintLine) -> String {
    let range = match line.end {
        StintEnd::At(end) => format!("{}-{}", line.start.format("%H:%M"), end.format("%H:%M")),
        StintEnd::Now => format!("{}-now", line.start.format("%H:%M")),
    };
    let suffix = match (line.end, line.open_stint_age) {
        (StintEnd::At(_), _) => {
            if line.spans_to_next_day {
                ", spans to next day".to_string()
            } else {
                String::new()
            }
        }
        (StintEnd::Now, Some(OpenStintAge::Today)) | (StintEnd::Now, None) => ", ongoing".to_string(),
        (StintEnd::Now, Some(OpenStintAge::Yesterday)) => {
            ", ongoing - elapsed since now, not a running total".to_string()
        }
        (StintEnd::Now, Some(OpenStintAge::TwoOrMoreDaysBack)) => String::new(), // unused: see below
    };
    if matches!(
        (line.end, line.open_stint_age),
        (StintEnd::Now, Some(OpenStintAge::TwoOrMoreDaysBack))
    ) {
        format!("  {:<11}  (unclosed)", range)
    } else {
        format!(
            "  {:<11}  ({}{})",
            range,
            format_minutes(line.duration_minutes),
            suffix
        )
    }
}
```

Notes on this shape:

- The `spans_to_next_day` branch reuses the exact same slot the
  existing `, ongoing` suffix occupies (`format!("  {:<11}  ({}{})",
  ...)`), per spec §2's explicit instruction ("same slot `, ongoing`
  already occupies") and the changeset plan's "duration parenthetical
  slot" wording. A completed stint can never also be `ongoing` (the
  match is on `StintEnd`), so there is no possibility of both suffixes
  appearing on one line — matches acceptance criterion §2's "a
  same-date completed stint and any open stint are unaffected by this
  specifically."
  Also note the exact literal is `, spans to next day` (leading comma
  and space, matching `, ongoing`'s own formatting) — spec §2's example
  `(01h 15m, spans to next day)` confirms this.
- `(StintEnd::Now, None)` is kept as a defensive fallback mapping to
  `", ongoing"` (today's current unconditional behavior) rather than
  making `open_stint_age` non-optional on `StintLine` — see the
  "Ambiguities" section below for why `Option` was chosen over a bare
  `OpenStintAge` field.
- The `TwoOrMoreDaysBack` case bypasses the normal
  `format_minutes(line.duration_minutes)` call entirely — spec §4 is
  explicit that "the stint's duration figure is dropped entirely" and
  acceptance criterion §4 requires "no minutes number anywhere in that
  line." Writing this as a separate `if`/`else` on the whole format
  string (rather than trying to make `duration_minutes` render as
  empty inside the shared template) is the only way to guarantee no
  digit-bearing token appears — a `format_minutes` call would always
  produce at least `"00h 00m"`.
- `is_today`/`OpenStintAge::Today`'s branch renders byte-identical to
  today's current `", ongoing"` output — satisfies acceptance criterion
  §4's "byte-identical to today's current output" for `target_date ==
  today`.

### 5. `build_stint_lines`'s new parameter and where `resolve()` computes it

New signature:

```rust
fn build_stint_lines(day: &DayStints, open_stint_age: OpenStintAge) -> Vec<StintLine> {
    let mut rows: Vec<StintLine> = Vec::new();
    for s in &day.completed {
        rows.push(StintLine {
            start: s.start.at_utc.with_timezone(&Local).time(),
            end: StintEnd::At(s.end.at_utc.with_timezone(&Local).time()),
            duration_minutes: s.minutes,
            spans_to_next_day: s.start.date != s.end.date,
            open_stint_age: None,
        });
    }
    for o in &day.open {
        rows.push(StintLine {
            start: o.start.at_utc.with_timezone(&Local).time(),
            end: StintEnd::Now,
            duration_minutes: o.minutes_so_far,
            spans_to_next_day: false,
            open_stint_age: Some(open_stint_age),
        });
    }
    rows.sort_by_key(|r| r.start);
    rows
}
```

- `spans_to_next_day: s.start.date != s.end.date` — `storage::Punch`
  already carries a `date` field (confirmed via `Punch { id, at_utc,
  date, kind }` in `stint.rs`'s own test fixtures, e.g. `p_on`, line
  376-383) — no new data fetch, exactly as the changeset plan states.
  Completed-stint branch only; open stints get `spans_to_next_day:
  false` unconditionally (§4 governs open-stint rendering, not §2).
- Every open `StintLine` built in one call shares the same
  `open_stint_age` value — the changeset plan's "deliberate, harmless
  duplication in the rare multi-open (E7) case" (every open stint on
  one date is the same age, since the age is a property of the *date*,
  not the individual stint).

Where `resolve()` computes the local: immediately after `is_today` is
bound (line 336), since it needs both `today` and `target_date`, both
already available there and unchanged by the intervening splice-check
work from step 3 above. Insert:

```rust
let open_stint_age = if target_date == today {
    OpenStintAge::Today
} else if target_date == today - ChronoDuration::days(1) {
    OpenStintAge::Yesterday
} else {
    OpenStintAge::TwoOrMoreDaysBack
};
```

placed directly after `let is_today = target_date == today;` (line
336) — before the punches/notes fetch, so it's available at every
later point in `resolve()` without reordering anything else.
`target_date <= today - 2 days` in spec/plan wording is implemented as
the `else` arm of a three-way if/else-if chain keyed on equality
checks against `today` and `today - 1 day`, which is exactly
equivalent for any `target_date <= today` and also correctly falls
into `TwoOrMoreDaysBack` for any `target_date > today` (a future date
can never have an open stint from "today" or "yesterday" relative to
itself in the failure sense the spec cares about, and no test in this
module constructs a future date with an open stint, so this branch is
inert in practice — see Risks). `date::resolve_date`'s existing
shorthand/absolute parsing is untouched; this is a pure comparison of
two already-bound `NaiveDate`s.

`build_stint_lines`'s call site (currently line 381,
`let stints = build_stint_lines(&day);`) becomes:

```rust
let stints = build_stint_lines(&day, open_stint_age);
```

No reordering needed — `open_stint_age` is bound at line ~337 (right
after `is_today`), well before line 381.

### 6. `StatusView`'s new fields and where `render()`/`day_total_line()` read them

New fields on `StatusView`:

```rust
pub struct StatusView {
    pub header: String,
    /// §3 header suffix, pre-rendered, e.g. `"  (00:45 continues
    /// previous day's stint)"`. `None` when no splice consumed
    /// `target_date`'s first punch.
    pub receiving_date_suffix: Option<String>,
    pub day_total_minutes: i64,
    pub has_open_stint: bool,
    pub daily_target: Option<DailyTargetHint>,
    pub weekday_name: Option<String>,
    pub eod: Option<EodState>,
    pub week_line: String,
    pub anomaly_lines: Vec<String>,
    pub stints: Vec<StintLine>,
    pub notes: Vec<String>,
    /// §4's three-way open-stint age, stored unchanged from the same
    /// local `resolve()` computes for `build_stint_lines` — read
    /// directly by `day_total_line()` since it is a per-date fact, not
    /// per-stint. NOT derived from `stints` — it is set even when
    /// `has_open_stint` is false, in which case `day_total_line()`
    /// never reads it (its two call sites below are both gated on
    /// `has_open_stint`).
    pub open_stint_age: OpenStintAge,
}
```

Field placement: `receiving_date_suffix` placed directly after
`header` (both are header-line concerns, keeps the struct's existing
top-to-bottom "header, then day-total-ish, then week, then detail"
grouping intact); `open_stint_age` placed at the end, near `stints`,
since it is conceptually adjacent to `has_open_stint`. Exact position
within the struct has no functional effect (named-field struct
literal, no positional construction anywhere in the codebase); this
placement is a readability choice, not a requirement.

`render()`'s header assembly (currently `view.header.clone()` at line
174) becomes:

```rust
let mut header_line = view.header.clone();
if let Some(suffix) = &view.receiving_date_suffix {
    header_line.push_str(suffix);
}
let mut lines: Vec<String> = vec![
    header_line,
    String::new(),
    day_total_line(view),
    view.week_line.clone(),
];
```

`day_total_line()`'s existing `(+ ongoing)` branch (lines 129-131):

```rust
if view.has_open_stint {
    s.push_str(" (+ ongoing)");
}
```

becomes:

```rust
if view.has_open_stint {
    match view.open_stint_age {
        OpenStintAge::Today | OpenStintAge::Yesterday => s.push_str(" (+ ongoing)"),
        OpenStintAge::TwoOrMoreDaysBack => s.push_str(" (+ unclosed)"),
    }
}
```

This is the exact "Day total match" requirement: today's/yesterday's
open stint leaves Day total's suffix unchanged (`(+ ongoing)`); a
two-or-more-days-back open stint changes it to `(+ unclosed)`, gated
on the same `view.open_stint_age` field the stint line itself was
built from — reading `StatusView`'s field, not deriving it again from
`view.stints` (there could be multiple open `StintLine`s in the E7
case, all sharing the same age, so re-deriving from `stints` would
work too, but reading the single `StatusView` field is simpler and is
what the changeset plan specifies: "`day_total_line()` reads the
`StatusView` field directly (it isn't per-stint)").

`resolve()`'s `StatusView { ... }` literal (lines 384-395) gains the
two new fields:

```rust
Ok(StatusView {
    header: date::format_date_with_weekday(target_date),
    receiving_date_suffix,
    day_total_minutes,
    has_open_stint,
    daily_target,
    weekday_name,
    eod,
    week_line: week_line_str,
    anomaly_lines,
    stints,
    notes: note_bodies,
    open_stint_age,
})
```

(`receiving_date_suffix` bound in step 3; `open_stint_age` bound in
step 5 — both already in scope by this point.)

### 7. `week_line()`'s one-line change (`src/status.rs`)

Current:

```rust
fn week_line(acct: &WeekAccounting, today: NaiveDate) -> String {
    render::status_week_line(acct.week, acct.owed, acct.fulfillment, acct.target, today)
}
```

New:

```rust
fn week_line(acct: &WeekAccounting, today: NaiveDate) -> String {
    render::status_week_line(
        acct.week,
        acct.owed,
        acct.fulfillment,
        acct.target,
        acct.carry_in,
        today,
    )
}
```

`acct.carry_in` already exists on `WeekAccounting` (`src/week.rs` line
66) and is already computed by `week::week_accounting` — no new
arithmetic, exactly the changeset plan's "just plumbing an already-
computed value one call further." This is the one call site inside
`status.rs`'s non-test code; the second call site the changeset plan's
"Ladder-altitude check" names,
`week_line_matches_status_week_line_field_for_field` (test module,
line ~1312), must be updated in the same commit (see Test plan §5
below) — this is exactly why the 2-task split was rejected: this
line's compile depends on `status_week_line`'s new signature landing
atomically with this call site and the test call site together.

## Test plan

One subsection per acceptance criterion in the changeset plan
(itself keyed to spec sections).

### §2 — span suffix (`status.rs`/`render.rs` interaction, tested at the render level)

- New test in `status.rs`'s test module, alongside `t2`/`t3`: a
  `StintLine` with `end: StintEnd::At(_)` and `spans_to_next_day:
  true`, `open_stint_age: None`, asserting the rendered stint line
  contains exactly `"(01h 15m, spans to next day)"` for a 75-minute
  duration (spec §2's own example numbers) and that `stint_line()`'s
  parenthetical has no other suffix text.
- Companion negative case: same duration, `spans_to_next_day: false`
  — asserts the line is byte-identical to the pre-existing `t2`-style
  `"(HH:MM)"` form, no `, spans to next day` substring anywhere in the
  output.
- Companion negative case for open stints: an open `StintLine`
  (`StintEnd::Now`, `open_stint_age: Some(OpenStintAge::Today)`) never
  contains `spans to next day` regardless of any hypothetical
  `spans_to_next_day` value — since `build_stint_lines` always sets
  `spans_to_next_day: false` for opens, add a direct unit assertion on
  `build_stint_lines`'s output (not just `stint_line`'s rendering) that
  every open row has `spans_to_next_day == false`, closing the gap
  between "the field is always false for opens" (a `build_stint_lines`
  fact) and "the suffix never appears for opens" (a `stint_line` fact)
  — two different functions, two different tests.
- `resolve()`-level: extend
  `resolve_midnight_splice_earlier_date_shows_completed_stint_no_anomaly`
  (or add a sibling test using the same fixture) to assert
  `view.stints[0].spans_to_next_day == true` for the existing
  23:30-start/00:45-end fixture (`d1 = 2026-02-09`, `d2 =
  2026-02-10`), and that `render(&view)` contains `", spans to next
  day"`.

### §3 — header suffix (`resolve()`, `render()`)

- Positive case (fires): using the existing midnight-splice fixture
  (`d1` start 23:30, `d2` end 00:45), resolve `d1` (Tue 2026-02-09) —
  wait: the suffix belongs to the *receiving* date, `d2` — resolve
  `Some("2026-02-10")` and assert `view.receiving_date_suffix ==
  Some("  (00:45 continues previous day's stint)".to_string())`, and
  that `render(&view)` contains the header line
  `"Tue 2026-02-10  (00:45 continues previous day's stint)"` (weekday
  computed via `date::format_date_with_weekday`, not hardcoded —
  assert via `.contains(&format!("{}  (00:45 continues previous day's
  stint)", date::format_date_with_weekday(d2)))` to avoid a
  hand-typed weekday string going stale). This directly satisfies the
  changeset plan's explicit "cover this with a test asserting the
  positive case actually fires, not just that the negative case stays
  silent."
- Negative case (no splice): `resolve_e7_multi_open_and_e8_orphaned_end_via_anomalies_only`'s
  existing fixture (or any ordinary non-boundary day) — assert
  `view.receiving_date_suffix.is_none()`.
- Negative case (prev has 2 opens, E7 shape): reuse
  `resolve_midnight_boundary_still_flags_non_1to1_shapes`'s fixture
  (`d1` has two starts, `d2` has one end at 00:30) — assert
  `receiving_date_suffix.is_none()` for `d2`, confirming the fresh
  `classify(prev_punches, now_utc).open.len() != 1` gate correctly
  blocks the suffix in the same shape that already blocks the splice
  itself in `stint.rs`.
- At-most-one case: not separately testable at this call site beyond
  the type itself (`Option<String>` structurally admits at most one) —
  note this in the test as a comment rather than writing a redundant
  assertion, per §4.3.1's 1:1 guarantee already being enforced inside
  `splice_candidate`.
- Only-punch-of-the-day case: new `resolve()` test — `d1` has a single
  trailing `Start` at 23:30, `d2` has exactly one punch (`End` at
  00:45, and nothing else). Assert: `view.stints.is_empty()` (stint
  list omitted per SPEC.md §7.1, unaffected by this task), `view.
  receiving_date_suffix.is_some()` (header suffix still renders),
  `view.day_total_minutes` and `view.week_line` still reflect the
  spliced 75 minutes (reuse the existing
  `resolve_midnight_splice_reflected_in_week_line` assertions as the
  template) — directly covering the changeset plan's explicitly named
  case.

### §4 — backdated caption / Day total match

- Today (`OpenStintAge::Today`): assert byte-identical output to the
  current `t1`/`t6a` fixtures — no new literal in either the stint
  line or the `Day total:` line. Concretely: build a `StatusView` with
  `open_stint_age: OpenStintAge::Today` and diff its `render()` output
  character-for-character against `t1_f1_single_open_stint_no_completed`'s
  existing expected substrings.
- Yesterday (`OpenStintAge::Yesterday`): a `StintLine` with `end:
  StintEnd::Now`, nonzero `duration_minutes`, `open_stint_age:
  Some(OpenStintAge::Yesterday)` — assert the rendered line contains
  both the duration figure (e.g. `10h 35m`, spec §4's own example) and
  the exact literal `, ongoing - elapsed since now, not a running
  total`, and that `Day total:` still shows `(+ ongoing)` (unchanged)
  on a `StatusView` with `open_stint_age: OpenStintAge::Yesterday`.
- Two-or-more-days-back (`OpenStintAge::TwoOrMoreDaysBack`): assert
  the rendered stint line is exactly `"  09:00-now    (unclosed)"` for
  a 09:00 start (spec §4's own example), with **no digit-bearing
  duration token anywhere in that line** — assert via a substring
  search for any of `"h "`/`"m)"`-shaped tokens being absent from that
  specific line, or more simply, assert the line does not contain
  `duration_minutes`'s formatted form for a nonzero fixture value
  (e.g. set `duration_minutes: 4321` — an implausible value that would
  be very visible if it leaked through — and assert `"43"` does not
  appear in the rendered stint line). Also assert `Day total:` shows
  `(+ unclosed)`, not `(+ ongoing)`, on a `StatusView` with
  `open_stint_age: OpenStintAge::TwoOrMoreDaysBack`.
- `resolve()`-level: three `resolve()` tests, one per age bucket, each
  seeding one open `Start` punch on `today`/`today - 1`/`today - 3`
  respectively and asserting `view.open_stint_age` matches and
  `view.stints[0].open_stint_age` matches (both must agree — this is
  the one place a mismatch between the `StintLine`-level field and the
  `StatusView`-level field could silently appear if `build_stint_lines`
  and the `StatusView` literal were ever passed two different values
  by accident).
- Boundary tightness: a dedicated test asserting `today - ChronoDuration::days(2)`
  classifies as `TwoOrMoreDaysBack` (not a fourth, unwritten bucket) —
  guards against an off-by-one in the `else`-chain.

### §5 — carry-in wiring

- `carry_in == 0`: reuse every existing `status.rs` test that builds a
  `WeekAccounting` via `current_week_acct`/`closed_week_acct` (both
  fixtures already hardcode `carry_in: 0`, confirmed by reading
  `src/status.rs` lines 453-475) — assert they still produce
  byte-identical output. Since `week_line()`'s new call passes
  `acct.carry_in` through unconditionally, and both fixtures already
  set it to 0, this is mechanically guaranteed by
  `status_week_line`'s `carry_in_minutes == 0` branch (step 1) — but
  add one explicit `assert_eq!` pinning the exact unchanged string for
  at least one fixture (e.g. `t11`'s full golden output) as a
  regression trip-wire.
- Deficit example: `render::status_week_line(wk(2026,7), 645, 1755,
  2400, -130, TODAY())` (deficit: `carry_in = -130` = `-02h10m`) ==
  `"Week 2026-07:  10h 45m left by end of Thursday (fulfillment 29h
  15m = worked 31h 25m + carry-in -02h 10m / target 40h 00m)"` — spec
  §5's first worked example, verbatim.
- Fulfillment-negative example: `status_week_line(wk(2026,9), <owed
  for 42h00m-left>, -120, 2400, -300, TODAY())` == `"...  (fulfillment
  -02h 00m = worked 03h 00m + carry-in -05h 00m / target 40h 00m)"` —
  spec §5's second example, verbatim (the `owed_minutes`/headline
  numbers are independent of the parenthetical and can be taken
  directly from the spec's `42h 00m left by end of Monday` text if a
  full headline match is wanted, or omitted from the assertion if only
  the parenthetical is being pinned).
- Surplus example: `status_week_line(wk(2026,10), 0, 2400, 2400, 50,
  TODAY())` == `"...  (fulfillment 40h 00m = worked 39h 10m + carry-in
  00h 50m / target 40h 00m)"` — spec §5's third example, verbatim.
- End-to-end through `resolve()`/`render()`: at least one `resolve()`
  test that seeds a prior week's data such that `WeekLedger`'s walk
  produces a nonzero `carry_in` for the target week (mirroring the
  existing `resolve_f9b_large_carry_in_negative_pace_on_day_one`
  fixture's week-target-zeroing technique), then asserts `render(&
  view)` contains the `worked {} + carry-in {}` substring — confirms
  the wiring survives the full `build_ledger` → `week_accounting` →
  `week_line` → `status_week_line` chain, not just the direct
  `status_week_line` unit calls above.
- `week_line_matches_status_week_line_field_for_field` (existing test,
  line ~1310): update its direct `render::status_week_line(...)` call
  to add `acct.carry_in` as the fifth argument, matching `week_line`'s
  new call — this is the second half of the atomic-signature-change
  requirement from the changeset plan's "Ladder-altitude check."

### Regression

- Run the full existing test suite (`cargo test`) after the change and
  confirm every currently-passing test in `status.rs`/`render.rs`
  still passes unmodified, per the changeset plan's per-fixture
  inspection claim that none of the current fixtures fall into any of
  the four new conditions. The one exception requiring a mechanical
  (not behavioral) edit is
  `week_line_matches_status_week_line_field_for_field`, whose call
  must gain the new argument to keep compiling (not because its
  behavior changes — `carry_in: 0` in every fixture keeps its
  assertion identical).
- Every `StatusView`/`StintLine` struct literal in the existing test
  module needs the two/three new fields added (`receiving_date_suffix:
  None`, `open_stint_age: OpenStintAge::Today` as the additive-only
  default on `StatusView`; `spans_to_next_day: false, open_stint_age:
  None` on every existing `StintLine` literal) — purely mechanical,
  required for the crate to compile, with no assertion changes needed
  since these are exactly today's implicit defaults.

### Plain-ASCII

- Extend `t13_plain_ascii_output_and_non_ascii_notes_pass_through`
  (`status.rs`) to build one `ascii_view` variant covering each of the
  five `status.rs`-produced literals in one pass (or several small
  variants, each producing one literal, all still ASCII-checked): `,
  spans to next day`; `, ongoing - elapsed since now, not a running
  total`; `(unclosed)`; `continues previous day's stint` (via a
  non-`None` `receiving_date_suffix`); `(+ unclosed)`. Each variant
  goes through the existing `.bytes().all(|b| b == b'\n' ||
  (0x20..=0x7E).contains(&b))` assertion already in that test.
- Extend `t33_every_produced_string_is_ascii` (`render.rs`) to add one
  `status_week_line(..., carry_in_minutes: <nonzero>, ...)` call
  (e.g. reusing the deficit example's arguments) to the `headlines`
  array already iterated by that test — covers the one new literal
  `render.rs` itself produces, `carry-in`.

## Risks, ambiguities, and disagreements

- **`OpenStintAge` as `Option<OpenStintAge>` on `StintLine`, not a
  bare field.** The changeset plan's own wording ("day-age
  enum/classification" as a new field, singular) doesn't explicitly
  settle whether a *completed* `StintLine` needs a value at all. This
  plan chooses `Option<OpenStintAge>`, `None` for completed stints,
  because a completed stint has no "age" concept at all (§4 is
  scoped to open stints only) and `None` makes that structurally
  explicit rather than forcing an arbitrary default (e.g. always
  `Today`) onto rows the value is meaningless for. The cost is one
  extra `None` arm in `stint_line()`'s match that can never actually
  be reached with a *different* value for the same stint (a completed
  `StintLine` is never built with `Some(_)`) — a small, acceptable
  redundancy given the alternative (an always-populated field with an
  ignored value on completed rows) is more likely to be silently
  misread by a future maintainer as "this row's age" rather than
  "unused."
- **`(StintEnd::Now, None)` fallback branch in `stint_line()` is dead
  code by construction**, since `build_stint_lines` always sets
  `Some(open_stint_age)` for every open row it builds, and no other
  caller constructs a `StintLine` outside tests. It is kept anyway (a)
  as a defensive default matching today's unconditional `, ongoing`
  behavior, so a hypothetical future caller that forgets to set
  `open_stint_age` degrades to today's behavior instead of panicking
  or silently doing nothing, and (b) because `clippy`'s cognitive-
  complexity gate (`.githooks/pre-commit`, threshold 15) rewards a
  flat match with an explicit arm over an `.unwrap_or(OpenStintAge::
  Today)` that would need its own justifying comment anyway. This is a
  judgment call, not something the four review rounds settled either
  way — flagging it explicitly rather than silently picking one.
- **Future-dated open stint falls into `TwoOrMoreDaysBack`.** Spec §4
  only discusses `today` / `today - 1` / `today - 2-or-more` — it
  never discusses `target_date > today`. `resolve_future_absolute_date_still_succeeds_and_renders_empty`
  (existing test) shows future dates are accepted and render empty
  (no punches seeded), so there is no existing fixture that would
  exercise a future date *with* an open stint, and the spec's own
  worked examples never construct one either. This plan's three-way
  if/else-if (`== today` / `== today - 1` / `else`) sends a future
  date to `TwoOrMoreDaysBack`, which is defensible (a future open
  stint's "age" relative to today is nonsensical either way, and
  `(unclosed)` at least avoids printing a nonsensical negative-looking
  duration) but is an extrapolation beyond what four rounds of review
  actually pinned down. Recommend flagging this explicitly in the PR
  description if it comes up in review, rather than treating it as
  settled — it is technically outside the acceptance criteria's
  explicit language (`target_date <= today - 2 days`), which never
  says what happens when neither that nor the first two conditions
  hold in the increasing-date direction. This plan's choice is
  internally consistent and requires no code change if a reviewer
  disagrees (only the `else` branch's target enum variant would need
  to change), so it is not a blocking ambiguity, just a documented
  extrapolation.
- **`day_total_line`'s cognitive complexity.** `.githooks/pre-commit`
  hard-fails a commit if any function exceeds `clippy.toml`'s
  `cognitive-complexity-threshold = 15`. `day_total_line` already has
  three sequential conditional segments before this task's change
  (the `(+ ongoing)` check, the `daily_target` block, the `eod` match);
  this task adds one more small `match` inside the existing `if
  view.has_open_stint` block, which is a bounded, shallow addition
  (2-arm match, no new nesting depth) and should not meaningfully move
  the needle, but this plan does not have a real clippy run to confirm
  headroom — call `cargo clippy --all-targets --quiet -- -W
  clippy::cognitive_complexity` as part of implementation, before
  relying on `git commit`'s hook to catch it, since a hook failure
  mid-implementation is more expensive to unwind than a preemptive
  check.
- **No disagreement found with the changeset plan or spec's
  substance** after reading both in full against the current source:
  every exact claim checked against source (the `splice_candidate`
  signature and its private-vs-`pub(crate)` status, the (prev,day)
  vs (day,next) `classify_at` branch behavior, `WeekAccounting.
  carry_in`'s existence and the `fulfillment = worked + carry_in`
  invariant, the `Punch.date` field's existence, the `format_minutes`
  sign convention, the two `status_week_line` call sites, and the
  existing test fixtures' `carry_in: 0`/no-boundary-marker shapes)
  checked out exactly as stated. No correctness problem the four
  review rounds missed was found.
