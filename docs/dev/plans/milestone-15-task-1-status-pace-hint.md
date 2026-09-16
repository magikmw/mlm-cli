# Milestone 15, Task 1 — low-level plan: carry-inclusive required-by-day in `status`

## What was read

- `docs/dev/plans/milestone-15-required-by-day.md` (full) — the changeset
  plan, its one task's acceptance criteria (1-8) and out-of-scope list.
- `docs/dev/SPEC.md` §2.4 ("Daily target"/"Required-by-day" bullets,
  ~lines 197-213), §5's explanatory note (~415-436), §7.1 (sample output
  ~503-548, pace-hint/est.-EOD bullets), §7.1 test-case list F9/F9b/F10/F11
  (~727-748).
- `docs/dev/NOTES.md` decisions 52 (~313-330) and 53 (~333-340).
- `docs/dev/plans/reports/required-by-day-spec-review.md` (full) —
  adversarial review, verdict `needs-rework`, findings 1-4.
- `src/status.rs` in full (1091 lines) — types, `render`, `resolve`, and
  every existing test.
- `src/week.rs`: `WeekAccounting` (lines 63-79), `daily_target_minutes`
  (223-225), `week_series`'s per-week formula (the `fulfillment =
  worked + carry_in`, `owed = target - fulfillment`, `carry_out =
  fulfillment - target` block).
- `src/date.rs`: `format_weekday_full` (179-190, `Weekday -> "Monday"`
  etc., no cross-reference needed — it's a plain match).
- `src/time.rs::format_minutes` (109-114) — the §4.2 signed formatter
  (`"-25h 20m"` style, `unsigned_abs` so `i64::MIN` can't overflow).
- `src/week_target.rs::set_week_target` (21-33) — used to seed target
  overrides in new tests.
- `clippy.toml` — `cognitive-complexity-threshold = 15` (stricter than
  clippy's default 25; relevant because `resolve()`'s daily-target/EOD
  block must stay simple enough to clear this under the pre-commit gate).
- Verified independently, not just trusted: the changeset plan's ISO
  weekday numbers for 2026-02-09/12/14/15 (Mon/Thu/Sat/Sun, all ISO week
  2026-07) and 2026-02-02 (Mon, ISO week 2026-06) via
  `datetime.date(...).isocalendar()`; the `33h31m → 402 → 2010 = 33h30m`
  arithmetic (2011 min, `div_euclid(5) = 402`, `402*5 = 2010 = 33h30m`).

## Scope

This task owns **`src/status.rs` exclusively**. Per the changeset plan
there are no sibling tasks — nothing in `week.rs`, `week_target.rs`,
`render.rs`, or the CLI/storage layers changes. `daily_target_minutes`
and `WeekAccounting`/`acct.fulfillment` are consumed as-is.

---

## 1. Current state, field-by-field

### `DailyTargetHint` (src/status.rs:56-61)

```rust
pub struct DailyTargetHint {
    pub target_minutes: i64,
    /// Signed: `target_minutes - day_total_minutes`.
    pub gap_minutes: i64,
}
```

Becomes:

```rust
/// The §7.1 "X left to <required> required by end of <weekday>" hint.
/// `None` at the `StatusView` level whenever `DATE` is not today (§3.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DailyTargetHint {
    /// `daily target × min(today's ISO weekday number, 5)` (§2.4).
    pub required_minutes: i64,
    /// Signed: `required_minutes - fulfillment` (`carry_in + worked`,
    /// §2.4/§5) — NOT `day_total_minutes`.
    pub gap_minutes: i64,
}
```

`target_minutes` is renamed `required_minutes` (field rename, not an
addition) because it no longer holds `daily_target_minutes(target)`
directly — it holds that value already multiplied by
`min(weekday, 5)`. Keeping the old name on the new value would be a
silent lie in every future reader of this struct.

The weekday word ("Thursday") is **not** stored on `DailyTargetHint`.
`day_total_line` already receives the full `&StatusView`, which already
carries `header: String` (e.g. `"Thu 2026-02-12"`) — but that's the
abbreviated form, not `format_weekday_full`'s `"Thursday"`, and deriving
one from the other by string surgery would be fragile. Instead, thread
the weekday word through as a second `Option<String>`-shaped field
alongside `daily_target` at the `StatusView` level (not inside
`DailyTargetHint` itself), because:
- it's `Some` under exactly the same condition as `daily_target`
  (`is_today`) and `None` under exactly the same condition, so a sibling
  `Option` field carries no risk of the two disagreeing that a single
  combined `Option` wouldn't already carry, and
- `DailyTargetHint` is a small numeric-only value type used identically
  by both `t6a`/`t6b`/`t6c` render-level tests (constructed directly,
  no dates involved) and `resolve()`; putting a `String` on it would
  force every one of those existing render-level tests to also supply a
  throwaway weekday string, for no benefit.

Concretely, add one field to `StatusView` (src/status.rs:82-104):

```rust
/// Today's full weekday name (`date::format_weekday_full`), e.g.
/// `"Thursday"` — used only by the day-total pace-hint line. `Some`
/// under exactly the same condition as `daily_target` (`is_today`).
pub weekday_name: Option<String>,
```

Placed directly after `pub daily_target: Option<DailyTargetHint>,` in
the struct, so the two `Option`s that must agree sit next to each other
textually.

### `day_total_line` (src/status.rs:115-138)

Current pace-hint segment (lines 125-131):

```rust
if let Some(hint) = &view.daily_target {
    s.push_str(&format!(
        ", {} left to {} daily target",
        format_minutes(hint.gap_minutes),
        format_minutes(hint.target_minutes)
    ));
}
```

Becomes:

```rust
if let Some(hint) = &view.daily_target {
    s.push_str(&format!(
        ", {} left to {} required by end of {}",
        format_minutes(hint.gap_minutes),
        format_minutes(hint.required_minutes),
        view.weekday_name.as_deref().unwrap_or_default()
    ));
}
```

`unwrap_or_default()` is defensive only — `weekday_name` is always
`Some` exactly when `daily_target` is `Some` (both set together in
`resolve()`, both `None` together), so the empty-string branch is
unreachable in practice, not a real code path this task needs to design
around. (Alternative considered and rejected: destructure both options
together with `if let (Some(hint), Some(weekday)) = (...)`. Rejected
because it reads as if the two could legitimately disagree, when the
invariant is "always both or neither" — the simpler single-`if let`
plus a defensive default states that invariant more directly than a
tuple match would.)

The EOD segment (lines 132-136) is untouched — it only reads
`view.eod`, which is unaffected in shape.

### `resolve()`'s daily-target/EOD block (src/status.rs:338-355)

Current:

```rust
let (daily_target, eod) = if is_today {
    let daily_target_minutes = week::daily_target_minutes(acct.target);
    let gap_minutes = daily_target_minutes - day_total_minutes;
    let hint = DailyTargetHint {
        target_minutes: daily_target_minutes,
        gap_minutes,
    };
    let eod = if has_open_stint {
        if gap_minutes > 0 {
            let eod_time = (now + ChronoDuration::minutes(gap_minutes)).time();
            Some(EodState::At(eod_time))
        } else {
            Some(EodState::TargetAlreadyMet)
        }
    } else {
        None
    };
    (Some(hint), eod)
} else {
    (None, None)
};
```

Becomes (only the two `let` lines feeding `gap_minutes`/`hint` change;
the `eod` derivation below is untouched — it already only depends on
`gap_minutes` and `has_open_stint`, exactly as the changeset plan says):

```rust
let (daily_target, eod, weekday_name) = if is_today {
    let daily_target_minutes = week::daily_target_minutes(acct.target);
    let weekday_number = i64::from(today.weekday().number_from_monday());
    let required_minutes = daily_target_minutes * weekday_number.min(5);
    let gap_minutes = required_minutes - acct.fulfillment;
    let hint = DailyTargetHint {
        required_minutes,
        gap_minutes,
    };
    let eod = if has_open_stint {
        if gap_minutes > 0 {
            let eod_time = (now + ChronoDuration::minutes(gap_minutes)).time();
            Some(EodState::At(eod_time))
        } else {
            Some(EodState::TargetAlreadyMet)
        }
    } else {
        None
    };
    (Some(hint), eod, Some(date::format_weekday_full(today)))
} else {
    (None, None, None)
};
```

And in the final `StatusView` construction (src/status.rs:363-373), add
`weekday_name,` as a field (shorthand, since the local binding has the
same name).

Notes on this specific wiring, since it's the crux of the whole task:

- `today.weekday()` — `today` is already bound at the top of `resolve()`
  (`let today = now.date_naive();`, line 314) and is a `NaiveDate`, which
  has `.weekday() -> chrono::Weekday` via the already-imported
  `Datelike` trait (src/status.rs:28, already `use chrono::{...,
  Datelike, ...}` — no new import needed). `is_today` guarantees
  `target_date == today`, so using `today` rather than `target_date`
  here is deliberately redundant-but-correct (they're equal on this
  branch); `today` reads slightly clearer next to `now` at the call
  site and matches what `week_line(&acct, today)` already does two
  lines above.
- `.number_from_monday()` returns `u32` (Mon=1…Sun=7); cast to `i64`
  before multiplying, since `daily_target_minutes` is `i64` and mixing
  `i64 * u32` doesn't compile. `.min(5)` after the cast, on the `i64`,
  not on the `u32` — either order is numerically identical here (5 fits
  in both types) but casting first keeps the whole expression in one
  type end to end.
- No `saturating_*`/`clamp` anywhere in this block, matching
  `week.rs`'s own house style (its doc comments explicitly call out
  "nothing here is ever clamped") and SPEC.md §5's "nothing is capped
  or clamped anywhere in this table" — `gap_minutes` legitimately goes
  deeply negative (F9b) and that's the correct, un-clamped answer.
- `week.rs`'s own `#![allow(dead_code)]` note (line 10-11, "TODO
  (integration): drop this once Milestones 9/10/11 consume the module")
  is stale trivia from before `status.rs` started consuming it — not
  this task's concern to fix (out of scope: don't touch `week.rs`), but
  worth knowing it's already dead-code-annotated so no new warning
  should appear from this task's read of `acct.fulfillment`, which was
  already being read at line 333 before this change (via `week_line`).

---

## 2. Render string, exact wording

SPEC.md §7.1's literal sample line (line ~503):

```
Day total:     07h 25m (+ ongoing), 02h 45m left to 32h 00m required by end of Thursday, est. EOD 20:45
```

Confirmed capitalization/spacing against this literal string: lowercase
`left to`, lowercase `required by end of`, capitalized weekday
(`Thursday`, from `format_weekday_full` verbatim — no case transform
needed, it already returns capitalized full names). Single spaces
throughout, comma-space between clauses, matching the existing
`", {} left to {} daily target"` template's punctuation exactly except
for the swapped-in words. The new format string, confirmed above:

```rust
", {} left to {} required by end of {}"
```

Negative gap rendering: `format_minutes` (src/time.rs:109-114) already
handles the sign — `format_minutes(-1520)` → `"-25h 20m"` (leading `-`,
no space before the sign, `unsigned_abs` so no overflow risk). No new
formatting path; this task calls `format_minutes(hint.gap_minutes)`
exactly as the old code did, just fed a different (still signed) `i64`.
This is the function named by §4.2 that the changeset plan's global
constraints require reusing verbatim.

---

## 3. Existing tests: exact current assertions and what they become

All of these are in `src/status.rs`'s `#[cfg(test)] mod tests`. Every
one below is a **render-level** test (constructs `StatusView`/
`DailyTargetHint` directly, no DB, no dates beyond the fixed `today()`
helper) unless marked "(resolve-level)".

### `t1_f1_single_open_stint_no_completed` (lines 519-537)

No pace-hint assertion currently (`daily_target`/`gap` values are set
but never asserted in a string check — only `"Day total:     00h 00m (+
ongoing)"` is asserted, and the hint's rendered text isn't checked
here). Field names must still compile: change
`DailyTargetHint { target_minutes: 480, gap_minutes: 480 }` to
`DailyTargetHint { required_minutes: 480, gap_minutes: 480 }` and add
`view.weekday_name = Some("Thursday".to_string());` right after (needed
now that `day_total_line` reads `view.weekday_name` whenever
`daily_target` is `Some` — every existing test that sets `daily_target`
must also set `weekday_name`, or the rendered string will end in
`required by end of ` with a trailing space instead of a real weekday,
which would silently pass this test's specific assertions but is wrong
output). No assertion text changes here.

### `t2_f2_ordinary_day_no_eod_line_without_open_stint` (539-559)

Current:
```rust
view.daily_target = Some(DailyTargetHint { target_minutes: 480, gap_minutes: -30 });
...
assert!(out.contains("Day total:     08h 30m, -00h 30m left to 08h 00m daily target"));
```
Becomes:
```rust
view.daily_target = Some(DailyTargetHint { required_minutes: 480, gap_minutes: -30 });
view.weekday_name = Some("Thursday".to_string());
...
assert!(out.contains("Day total:     08h 30m, -00h 30m left to 08h 00m required by end of Thursday"));
```

### `t6a_f9_open_stint_with_gap_shows_est_eod` (587-600)

Current:
```rust
view.daily_target = Some(DailyTargetHint { target_minutes: 480, gap_minutes: 35 });
view.eod = Some(EodState::At(t(18, 35)));
...
assert!(out.contains(
    "Day total:     07h 25m (+ ongoing), 00h 35m left to 08h 00m daily target, est. EOD 18:35"
));
```
This test's own numbers (`day_total=445`, `gap=35`, `target=480`,
`EOD 18:35`) are the **pre-decision-52 formula** (day-total vs.
carry-free daily target), which this task retires entirely — this test
must be re-derived, not just re-worded, or it silently keeps testing
the old formula under a renamed field. Re-derive using the *same*
underlying week fixture already used elsewhere in this file
(`current_week_acct(645, 1755, 2400)`: target 2400, fulfillment 1755,
today = Thursday, weekday number 4): `daily_target_minutes(2400) =
480`, `required = 480*4 = 1920` (`32h 00m`), `gap = 1920 - 1755 = 165`
(`02h 45m`). Keep `day_total_minutes = 445` (`07h 25m`, unaffected by
this change per the global constraint) and `has_open_stint = true`.
`now = 18:00` per the `now_thu_1800()` fixture already in this file →
`est. EOD = 18:00 + 02h45m = 20:45`.
```rust
view.day_total_minutes = 445; // 07h 25m
view.has_open_stint = true;
view.daily_target = Some(DailyTargetHint { required_minutes: 1920, gap_minutes: 165 });
view.weekday_name = Some("Thursday".to_string());
view.eod = Some(EodState::At(t(20, 45)));
let out = render(&view);
assert!(out.contains(
    "Day total:     07h 25m (+ ongoing), 02h 45m left to 32h 00m required by end of Thursday, est. EOD 20:45"
));
```
This now matches SPEC.md §7.1's golden sample line exactly (same
numbers as `t11`, see below) — intentional, since this test and `t11`
are testing the same worked example from two different angles
(hand-built `DailyTargetHint` here vs. the full `StatusView` there).

### `t6b_f9_open_stint_gap_zero_or_negative_is_target_already_met` (602-622)

Current loop uses `gap in [0, -20]` with `target_minutes: 480` fixed,
and asserts `"left to 08h 00m daily target"` plus, for `gap < 0`, the
signed gap string. Rewire field name and wording only — the *values*
(`required_minutes: 480`, `gap` 0/-20) stay valid as arbitrary
render-level fixture numbers (this test isn't tied to a specific golden
example, unlike `t6a`/`t11`):
```rust
for gap in [0i64, -20i64] {
    let mut view = base_view();
    view.day_total_minutes = 480 - gap; // unchanged: still an arbitrary day-total fixture, no longer load-bearing for the gap math
    view.has_open_stint = true;
    view.daily_target = Some(DailyTargetHint { required_minutes: 480, gap_minutes: gap });
    view.weekday_name = Some("Thursday".to_string());
    view.eod = Some(EodState::TargetAlreadyMet);
    let out = render(&view);
    assert!(out.contains(", target already met"), "gap {gap}: {out}");
    assert!(!out.contains("est. EOD"), "gap {gap}: {out}");
    assert!(out.contains("left to 08h 00m required by end of Thursday"), "gap {gap}");
    if gap < 0 {
        assert!(out.contains(&format!("{} left to", format_minutes(gap))));
    }
}
```
Note: `view.day_total_minutes = 480 - gap` was already disconnected
from `gap_minutes` under the *old* formula too (the old code line 340
computed `gap_minutes` from `daily_target_minutes - day_total_minutes`
inside `resolve()`, but this is a render-level test that sets both
fields independently by hand) — so this line's continued presence is
not a new inconsistency introduced by this task, just an existing
render-level fixture quirk carried forward unchanged.

### `t6c_f9_no_open_stint_omits_eod_segment_entirely` (624-640)

Current: `target_minutes: 480, gap_minutes: 0`, asserts `"left to 08h
00m daily target"`. Becomes: `required_minutes: 480, gap_minutes: 0`,
add `weekday_name = Some("Thursday".to_string())`, assert `"left to 08h
00m required by end of Thursday"`.

### `t7_f10_golden_second_spec_example` (645-674)

No `daily_target`/`weekday_name` involved (`daily_target: None, eod:
None` — this is the F10 past-date-in-closed-week case). **No change**
beyond confirming it still compiles and still passes unmodified, per
acceptance criterion 6. Existing `assert!(!out.contains("daily
target"))` (line 669) should additionally gain a sibling assertion that
the new phrase is also absent, since "daily target" as a literal
substring will no longer appear anywhere in the renderer's vocabulary
after this change, but being explicit is cheap and documents intent:
add `assert!(!out.contains("required by end of"));` next to it.

### `t8_f11_different_day_in_current_week_has_no_daily_target_but_deadline_week_line` (679-689)

No `daily_target` set (`base_view()`'s default `None`). **No change**
beyond the same defensive addition: after
`assert!(!out.contains("daily target"));` add
`assert!(!out.contains("required by end of"));`. Per acceptance
criterion 7, confirm this test still passes unmodified otherwise.

### `t11_golden_full_first_spec_example` (759-809)

This is *the* golden test for SPEC.md §7.1's first worked example — the
one the adversarial review's finding #1 flagged as stale. Current:
```rust
daily_target: Some(DailyTargetHint { target_minutes: 480, gap_minutes: 35 }),
eod: Some(EodState::At(t(18, 35))),
...
let expected = "...\
    Day total:     07h 25m (+ ongoing), 00h 35m left to 08h 00m daily target, est. EOD 18:35\n\
    ...";
```
Becomes (using the same re-derivation as `t6a` above — this is the same
underlying example, `acct = current_week_acct(645, 1755, 2400)`, today
Thursday, weekday number 4):
```rust
daily_target: Some(DailyTargetHint { required_minutes: 1920, gap_minutes: 165 }),
eod: Some(EodState::At(t(20, 45))),
```
Add `weekday_name: Some("Thursday".to_string()),` to the `StatusView`
literal (needs a field addition at the construction site, not just a
value change). Expected string's day-total line becomes:
```
Day total:     07h 25m (+ ongoing), 02h 45m left to 32h 00m required by end of Thursday, est. EOD 20:45
```
This exactly reproduces SPEC.md §7.1's sample line, now with the
review's corrected `20:45` (not the stale `18:35`). The week line
directly below it (`"Week 2026-07:  10h 45m left by end of Thursday
(fulfillment 29h 15m / target 40h 00m)"`) is untouched — it's produced
by `render::status_week_line` via the unmodified `week_line` helper,
out of this task's scope, and its numbers (`owed 645` = `10h45m`,
`fulfillment 1755` = `29h15m`, `target 2400` = `40h00m`) are unaffected
by anything this task changes, so they stay as-is and don't need
re-deriving.

### `t12_every_duration_matches_the_canonical_format` (848-897)

Builds three `StatusView`s inline (mirroring `t11`, `t7`, `t6b`) purely
to feed `assert_durations_well_formed`, a formatting-shape check
(zero-padding, digit counts) that doesn't care about the *specific*
numbers, only their shape. Update the `t11`-shaped literal's
`daily_target`/`eod`/`weekday_name` fields to the same corrected values
as `t11` above (`required_minutes: 1920, gap_minutes: 165`, `t(20,
45)`, add `weekday_name: Some("Thursday".to_string())`) so it keeps
compiling; the `t7`-shaped and `t6b`-shaped literals are unaffected
(`daily_target: None` and `target_minutes`→`required_minutes: 480,
gap_minutes: -20` respectively — `t6b`'s literal here needs the same
field rename as everywhere else, plus a `weekday_name`, since it has
`has_open_stint = true` and a real `daily_target`). Since this test
only checks digit shape, the exact required/gap values chosen don't
need to match any golden example — reusing `t11`'s corrected numbers is
simplest and keeps one fewer set of "just some numbers" fixtures
floating around.

### `t13_plain_ascii_output_and_non_ascii_notes_pass_through` (901-936)

Same `t11`-shaped literal (`ascii_view`), same field-rename +
`weekday_name` addition, values updated to match `t11`'s corrected
numbers for consistency (again, this test only checks byte-range, not
specific numbers, so any valid fixture works — reuse `t11`'s).

### `week_line_matches_status_week_line_field_for_field` (1083-1089)

No `daily_target` involved at all — untouched.

### Resolve-level tests

- `resolve_f10_past_date_different_closed_week` (959-974): asserts
  `view.daily_target.is_none()`. Untouched — `is_today` is false for
  this fixture (`target_date = 2026-01-05`, `today = 2026-02-12`), so
  this branch is unaffected by anything in this task's diff.
- `resolve_f11_is_today_false_but_week_is_current` (980-1004): same —
  `daily_target.is_none()` asserted, untouched.
- All other `resolve_*` tests (`resolve_e7_...`,
  `resolve_malformed_date_is_a_hard_error`,
  `resolve_shorthand_matches_equivalent_absolute_date`,
  `resolve_future_absolute_date_still_succeeds_and_renders_empty`,
  `resolve_note_only_day_has_no_stint_section_but_has_notes`): none
  assert on `daily_target`/`eod` contents. Untouched, but
  `resolve_e7_multi_open_and_e8_orphaned_end_via_anomalies_only`
  (1007-1018) *does* call `resolve(None, now_thu_1800(), &conn)` with
  `date_arg: None`, i.e. `is_today = true` — it exercises the new
  formula path at runtime (just doesn't assert on it), so it's a
  reasonable place to additionally assert
  `view.weekday_name == Some("Thursday".to_string())` and that
  `view.daily_target.is_some()`, as a cheap smoke check that the new
  field actually gets populated through the real `resolve()` path, not
  just in hand-built `StatusView` literals. Add those two assertions;
  everything else in that test is unrelated (open-stint/orphaned-end
  anomaly counting) and stays as-is.

---

## 4. New tests for F9b (and the Sat/Sun/non-multiple-of-5 sub-cases)

All four new tests below are **resolve-level** (real `test_db()`,
real inserted punches/target overrides, real dates), because the thing
under test is the wiring inside `resolve()` itself — which weekday
number gets picked, whether `acct.fulfillment` (not `day_total_minutes`)
drives the gap, and the Sat/Sun pin — not just whether `render` prints
a struct's fields correctly (that's already covered by `t6a`/`t6b`/
`t6c` above). Render-level tests exercising the same *rendering* logic
already exist; these are deliberately at the layer the changeset plan's
review found the real risk (finding #2, a formula claim that's false
for real, unconstrained inputs).

All four live within ISO week 2026-07 (Mon 2026-02-09 … Sun 2026-02-15)
— the same week the existing `today()`/`now_thu_1800()` fixtures use —
except the carry-in test, which also seeds the prior week 2026-06 (Mon
2026-02-02 … Sun 2026-02-08) to produce a real `carry_in`. Both weeks'
ISO placement was verified independently (see "What was read" above),
not assumed.

### `resolve_f9b_large_carry_in_negative_pace_on_day_one`

Setup: week 2026-06 target overridden to `0` (F7b: legal, zero target),
worked 2000 minutes total across that week (five weekdays × 400
minutes each, e.g. `08:00-14:40` Mon-Fri 2026-02-02..06 — any split
summing to 2000 is fine; picking round 400-minute stints keeps the
fixture arithmetic checkable by eye). No punches in week 2026-07 before
`target_date`. `now` = Mon 2026-02-09, 09:00 (early in the day, no open
stint — keep this case simple: assert on `daily_target`, not `eod`).

```rust
#[test]
fn resolve_f9b_large_carry_in_negative_pace_on_day_one() {
    let conn = test_db();
    week_target::set_week_target(&conn, &wk(2026, 6), 0).expect("set target");
    for day in [2, 3, 4, 5, 6] {
        storage::insert_punch(&conn, PunchKind::Start, d(2026, 2, day), t(8, 0), &Local)
            .expect("insert");
        storage::insert_punch(&conn, PunchKind::End, d(2026, 2, day), t(14, 40), &Local)
            .expect("insert");
    }
    let monday_0900 = Local
        .from_local_datetime(&d(2026, 2, 9).and_hms_opt(9, 0, 0).unwrap())
        .unwrap();
    let view = resolve(None, monday_0900, &conn).expect("resolve");
    assert_eq!(view.header, "Mon 2026-02-09");
    assert_eq!(view.day_total_minutes, 0, "day total unaffected by carry-in");
    let hint = view.daily_target.expect("is_today");
    assert_eq!(hint.required_minutes, 480, "daily_target(2400) * min(1,5)");
    assert_eq!(hint.gap_minutes, -1520, "480 - fulfillment(2000)");
    assert_eq!(view.weekday_name, Some("Monday".to_string()));
    let out = render(&view);
    assert!(out.contains("Day total:     00h 00m,"), "day total line: {out}");
    assert!(out.contains("-25h 20m left to 08h 00m required by end of Monday"));
}
```

Hand-derivation, verified independently above: week 2026-06 target 0,
worked 2000 ⇒ `carry_out = 2000 - 0 = 2000` = week 2026-07's
`carry_in`. Week 2026-07 target defaults to 2400 (no override) ⇒
`daily_target_minutes(2400) = 480`. Monday = ISO weekday 1 ⇒
`required = 480 * min(1,5) = 480`. `worked` in week 2026-07 so far = 0
⇒ `fulfillment = 2000 + 0 = 2000`. `gap = 480 - 2000 = -1520` =
`format_minutes(-1520)` = `-25h 20m` (`1520/60=25`, `1520%60=20`).
Matches acceptance criterion 3's fixture numbers exactly.

### `resolve_f9b_saturday_pin_multiple_of_five_target`

Default target (2400, a multiple of 5) — confirms `required == target`
on Sat/Sun for this case only, per acceptance criterion 4.

```rust
#[test]
fn resolve_f9b_saturday_pin_multiple_of_five_target() {
    let conn = test_db();
    storage::insert_punch(&conn, PunchKind::Start, d(2026, 2, 14), t(9, 0), &Local)
        .expect("insert");
    storage::insert_punch(&conn, PunchKind::End, d(2026, 2, 14), t(11, 20), &Local)
        .expect("insert"); // 140 min, arbitrary day total, not load-bearing
    let saturday_1200 = Local
        .from_local_datetime(&d(2026, 2, 14).and_hms_opt(12, 0, 0).unwrap())
        .unwrap();
    let view = resolve(None, saturday_1200, &conn).expect("resolve");
    assert_eq!(view.header, "Sat 2026-02-14");
    let hint = view.daily_target.expect("is_today");
    assert_eq!(hint.required_minutes, 2400, "5 * daily_target_minutes(2400) == target itself here");
    assert_eq!(view.weekday_name, Some("Saturday".to_string()));
}
```

### `resolve_f9b_sunday_pin_non_multiple_of_five_target_override`

The changeset plan's own worked example, target `33h 31m` = 2011
minutes, verified independently above: `daily_target_minutes(2011) =
2011.div_euclid(5) = 402`; `5 * 402 = 2010` = `33h 30m` — one minute
under the raw override (2011), not equal to it. This is the case
finding #2 of the adversarial review said F9b was missing.

```rust
#[test]
fn resolve_f9b_sunday_pin_non_multiple_of_five_target_override() {
    let conn = test_db();
    week_target::set_week_target(&conn, &wk(2026, 7), 2011).expect("set target"); // 33h 31m
    let sunday_1200 = Local
        .from_local_datetime(&d(2026, 2, 15).and_hms_opt(12, 0, 0).unwrap())
        .unwrap();
    let view = resolve(None, sunday_1200, &conn).expect("resolve");
    assert_eq!(view.header, "Sun 2026-02-15");
    let hint = view.daily_target.expect("is_today");
    assert_eq!(hint.required_minutes, 2010, "5 * floor(2011/5) = 2010, NOT 2011");
    assert_ne!(hint.required_minutes, 2011, "must not silently round up to the raw override");
    assert_eq!(view.weekday_name, Some("Sunday".to_string()));
    let out = render(&view);
    assert!(out.contains("33h 30m required by end of Sunday"));
}
```

### (Covered by the above three, not a fourth test)

Acceptance criterion 3's "day total unaffected" assertion is folded
into `resolve_f9b_large_carry_in_negative_pace_on_day_one` above rather
than split into its own test — the whole point of that fixture is that
`day_total_minutes == 0` while `gap_minutes == -1520` simultaneously, so
asserting both in one test is what actually demonstrates the
independence claim; two separate tests asserting one fact each would
let either regress without the other's test catching that they'd
started correlating.

---

## 5. Test plan by acceptance criterion (spec cases)

- **F9 / golden first example (§7.1)** — `t6a_f9_open_stint_with_gap_shows_est_eod`
  and `t11_golden_full_first_spec_example`, both re-derived to
  `required=1920` (`32h 00m`), `gap=165` (`02h 45m`), `EOD 20:45`,
  reproducing SPEC.md §7.1's sample line verbatim.
- **F9, three EOD states** — `t6a` (state 1: `EodState::At`), `t6b`
  (state 2: `gap <= 0` → `TargetAlreadyMet`, both `gap=0` and `gap=-20`
  sub-cases), `t6c` (state 3: no open stint → `eod: None`, segment
  omitted). All three keep their existing structure, only field names/
  wording/weekday updated.
- **F9b, large carry-in, negative day-1 pace** —
  `resolve_f9b_large_carry_in_negative_pace_on_day_one` (new): carry_in
  2000, target 2400, Monday, `gap = -1520`, `day_total = 0` asserted
  unaffected in the same test.
- **F9b, Sat/Sun pin, multiple-of-5** —
  `resolve_f9b_saturday_pin_multiple_of_five_target` (new): target
  2400, `required == 2400 == target`.
- **F9b, Sat/Sun pin, non-multiple-of-5 override** —
  `resolve_f9b_sunday_pin_non_multiple_of_five_target_override` (new):
  target 2011 (`33h31m`), `required = 2010` (`33h30m`), explicitly
  asserted `!= 2011`. This is the case the adversarial review's
  finding #3 said F9b was missing before the spec fold; this task's
  test suite must not repeat that gap.
- **F10, past date in closed week** — `t7_f10_golden_second_spec_example`
  (unmodified except an added defensive assertion that "required by end
  of" is also absent, not just "daily target"); `daily_target`/`eod`
  stay `None` because `is_today` is false, logic untouched.
- **F11, different day within current week** —
  `t8_f11_different_day_in_current_week_has_no_daily_target_but_deadline_week_line`
  (unmodified except the same defensive addition); confirms
  `daily_target`/`eod` stay absent while the week line still keys its
  deadline framing to today's actual weekday — that's `render`/week-
  line territory, untouched by this task.
- **Wording change everywhere** (`"<gap> left to <required> required by
  end of <weekday>"`) — every render-level test above that sets
  `daily_target` (`t1`, `t2`, `t6a`, `t6b`, `t6c`, `t11`, `t12`, `t13`)
  now also sets `weekday_name` and asserts (where it asserts string
  content at all) the new phrase, never the old `"daily target"`
  wording.

---

## 6. Everything from the adversarial review this task must still address

The review found two real defects (findings 1 and 2) and one gap in
test coverage (finding 3); its verdict was `needs-rework`, but its own
text notes both are "cheap to fix... but both need a text change, not
just review sign-off." Checking each against what's already true of
SPEC.md/NOTES.md *as read for this plan* (not as of the review's own
timestamp):

- **Finding 1 (stale `est. EOD 18:35`)** — already fixed in the current
  SPEC.md (§7.1's sample now reads `est. EOD 20:45`, confirmed by direct
  read above) and called out as fixed by NOTES.md decision 53. This
  task's job is purely to make `t6a`/`t11`/`t12`/`t13`'s *source code*
  match that already-corrected spec text — done in section 3 above.
- **Finding 2 (Sat/Sun `required` isn't always `== target`)** — already
  fixed in the current SPEC.md text (§2.4's Required-by-day bullet now
  explicitly says the Sat/Sun pin "is **not always exactly equal to**
  the week's own target" for non-multiple-of-5 overrides) and in NOTES
  decision 53. The changeset plan's own acceptance criterion 5 and this
  plan's `resolve_f9b_sunday_pin_non_multiple_of_five_target_override`
  test implement exactly the corrected formula (`5 × floor(target/5)`,
  not `target` itself) — this task does not need to (and must not)
  "fix" SPEC.md further; it only needs its code and tests to match what
  SPEC.md already says.
- **Finding 3 (F9b didn't exercise the non-multiple-of-5 case)** —
  addressed by this plan's new
  `resolve_f9b_sunday_pin_non_multiple_of_five_target_override` test
  (section 4), which is exactly the missing case the review named.
- **Finding 4 (terminology note, explicitly "not a defect", not
  blocking)** — no action needed; the review itself says so.

Nothing else in the review's text names an uncovered case beyond these
four.

---

## Risks, ambiguities, and disagreements

1. **Weekday-word plumbing (`weekday_name` on `StatusView`) is an
   addition the changeset plan doesn't mention.** The changeset plan's
   "Architecture" section (lines 39-42) says `format_weekday_full` "is
   reused verbatim for the day line's new... wording", and describes
   the whole change (lines 54-61) as a rename/reshape of
   `DailyTargetHint` plus its one render call site — it doesn't say
   *where* the weekday string is threaded from render's `&StatusView`
   down to `day_total_line`. Two designs are possible: (a) a new
   `StatusView.weekday_name: Option<String>` field (what this plan
   picked, section 1 above), or (b) put the weekday string *inside*
   `DailyTargetHint` itself (e.g. `pub weekday: String`). This plan
   picked (a) because `DailyTargetHint` is reused byte-for-byte by
   `t6a`/`t6b`/`t6c` as a small numeric value type constructed directly
   in several tests with no date context, and folding a `String` into
   it would force every one of those to also invent a throwaway weekday
   string for no test-value gained. This is a real design choice this
   plan is making, not one the changeset plan settled — flagging it
   rather than treating it as obviously implied.
2. **`resolve_f9b_*` tests are resolve-level (DB-backed), not
   render-level.** The changeset plan's test list (line 63-67) names
   `t6a`/`t6b`/`t6c`/`t7_f10`/`t11_golden` as the tests needing
   correction and says new cases are "added alongside them" without
   specifying which layer. This plan chose resolve-level for the three
   new F9b tests specifically because the review's own finding #2 was a
   *formula* defect (real-input arithmetic), not a rendering defect —
   a render-level test that just hand-constructs a `DailyTargetHint`
   with pre-computed numbers would not have caught (and would not
   catch a regression of) the Sat/Sun floor-division bug the review
   found. If whoever executes this task disagrees and wants everything
   render-level for speed/simplicity, that's a legitimate alternative,
   but it would weaken exactly the coverage finding #2/#3 asked for.
3. **`t2_f2_ordinary_day_no_eod_line_without_open_stint`'s fixture
   (`gap = -30`, `required = 480`) is not tied to any real formula
   derivation** (it's a hand-picked render-level number, same as
   before this change) — left as-is deliberately; re-deriving it from
   a "real" week fixture would add churn with no test-value gain, since
   this test's job is checking the *no-EOD-without-open-stint* branch,
   not the gap formula (that's `t6a`/`t11`'s and the new F9b tests'
   job).
4. **No pure formula-extraction function was introduced** (e.g. a
   `required_by_day_minutes(daily_target, weekday) -> i64` helper) even
   though it would make the Sat/Sun/non-multiple-of-5 cases unit-
   testable without a DB. This plan deliberately followed the
   changeset plan's explicit instruction (out-of-scope bullet, lines
   200-203) against introducing "a new named 'ISO weekday number'
   helper/type" and, by the same minimalism the changeset plan argues
   for throughout ("this is a formula and wording correction... no new
   architecture"), extended that to not introducing a same-purpose
   formula-wrapper function either — using real calendar dates in
   resolve-level tests instead (section 4). If the implementer finds
   the resulting test fixtures (seeding a whole prior week for the
   carry-in case) too heavy, extracting such a helper and unit-testing
   it directly is the fallback, but it would be a scope addition beyond
   what the changeset plan describes, so this plan didn't default to
   it.
5. **Cognitive-complexity budget.** `clippy.toml` sets the threshold to
   15 (stricter than clippy's default 25). `resolve()`'s daily-target/
   EOD block gains one more line (`weekday_number`/`required_minutes`)
   inside the existing `if is_today { ... }` branch and one more
   3-tuple element threaded through. This is unlikely to push
   `resolve()` over threshold given it was already passing with the
   old, only-slightly-simpler version of the same branch, but it's
   worth the implementer running `cargo clippy` (read-only, no mutating
   flags) before considering the task done, since this plan can't
   execute that check itself.

VERDICT: green
FILE: docs/dev/plans/milestone-15-task-1-status-pace-hint.md
Plan covers the `DailyTargetHint`/`StatusView` field changes, the exact `resolve()` formula rewire, every existing test's corrected numbers (including the golden 20:45), and three new resolve-level F9b tests (carry-in, Sat/Sun multiple-of-5, Sat/Sun non-multiple-of-5); flags one real design choice (where the weekday string lives — `StatusView` vs. inside `DailyTargetHint`) and one layer choice (resolve-level vs. render-level F9b tests) as plan-level judgment calls the changeset plan left open, not spec ambiguities.
