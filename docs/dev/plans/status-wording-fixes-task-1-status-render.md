# Task 1 plan — code: `render.rs` + `status.rs`

## What I read

- `docs/dev/plans/status-wording-fixes-plan.md` (locked, one review round folded in) — full.
- `docs/dev/specs/2026-09-23-status-wording-fixes.md` (locked, two review rounds folded in) — full.
- `src/render.rs` — full (646 lines).
- `src/status.rs` — full (1826 lines, read in two chunks).
- `src/week.rs` — `daily_target_minutes` and its doc comment/tests (lines ~220-270), for the `div_euclid(5)` sibling constant Fix C's constraint section references.
- `src/week_view.rs` — the three `Total still owed` test sites (lines 355, 386, 455, 476) and their surrounding fixtures/goldens.
- `src/commands.rs` — the two `Total still owed` test-assertion sites (lines 1128, 1156) and their surrounding end-to-end test.
- `clippy.toml` — sets `cognitive-complexity-threshold = 15` (stricter than clippy's default 25).
- `.github/workflows/ci.yml` — the `test` job's exact gate: `cargo fmt --check`, `cargo build --target <t>`, `cargo test --target <t>`, `cargo clippy --all-targets --target <t> -- -D warnings`, then a multi-platform e2e smoke test running the built binary. Confirmed clippy runs with `-D warnings`, which is what makes `.date()`'s deprecation lint fatal (spec §2, plan's global constraints).
- `AGENTS.md`'s "Verifying changes" section (lines 69-78) — the exact local-dev command sequence, ending in two `cargo run` smoke-check lines Task 1 must keep behaving sanely.

## Scope

**This task owns, and edits directly:**
- `src/render.rs` — production code (`week_headline`'s `Closed` branch) and its `#[cfg(test)]` module.
- `src/status.rs` — production code (`EodState`, `DailyTargetHint`, `day_total_line`, `resolve`) and its `#[cfg(test)]` module.

**This task owns for test-assertion edits only — no production-code change:**
- `src/week_view.rs` — four `"Total still owed"` literals inside `#[cfg(test)]` fixtures/goldens (lines 355, 386, 455, 476).
- `src/commands.rs` — two `"Total still owed"` literals inside one `#[cfg(test)]` end-to-end test (lines 1128, 1156).

**Not touched by this task, at all:** `README.md`, `docs/dev/SPEC.md` — Task 2's exclusive scope, running in parallel. Nothing in Task 2's work constrains this task; the plan's "Interface contract to pin before opening worktrees" section already pins the three exact string shapes both tasks need, so this task can proceed without reading Task 2's output.

This document is a plan only. No source file is edited to produce it.

## Fix A — `"Total behind"` rename

### Production change

`src/render.rs:67-69`, inside `week_headline`'s `WeekFraming::Closed if owed_minutes > 0` arm:

```rust
// before
WeekFraming::Closed if owed_minutes > 0 => {
    format!("Total still owed: {}", format_minutes(owed_minutes))
}

// after
WeekFraming::Closed if owed_minutes > 0 => {
    format!("Total behind: {}", format_minutes(owed_minutes))
}
```

Line 70 (`WeekFraming::Closed => format!("Total ahead: {}", ...)`) is untouched, byte-identical.

### Full inventory of test-string sites (re-grepped, not trusted from the plan)

`grep -rn "Total still owed" src/` currently returns 12 hits across 3 files:

**`src/render.rs`** (7 hits, all in `#[cfg(test)] mod tests`):
- line 202 — `t2_past_week_headline_plain_total`: `"Total still owed: 03h 10m"` → `"Total behind: 03h 10m"`
- line 210 — `t3_status_first_example_past_week`: `"Total still owed: 01h 40m"` → `"Total behind: 01h 40m"`
- line 276 — `t34_carry_in_zero_is_byte_identical_to_current_output`: `"Week 2026-02:  Total still owed: 01h 40m"` → `"Week 2026-02:  Total behind: 01h 40m"`
- line 285 — `t6_f10_status_week_line_past_week_is_plain_total`: same string, same replacement. Note this test also asserts `!line.contains("left by end of")` and iterates all 7 weekday names asserting absence — those assertions are unaffected and stay as-is; "Total behind" contains no weekday name either.
- line 310 — `t7_future_week_gets_the_same_plain_form_as_closed`: `"Total still owed: 40h 00m"` → `"Total behind: 40h 00m"`
- line 336 — `t10_one_minute_owed_is_still_owed_the_boundary_above_t8`: `"Total still owed: 00h 01m"` → `"Total behind: 00h 01m"`. The test's own *name* (`..._is_still_owed_...`) is stale prose but not a string assertion — leave the identifier as-is; renaming test function identifiers is out of scope for a presentation-only string fix and risks an unrelated diff. (Flagged under Ambiguities below in case the reviewer disagrees.)

**`src/status.rs`** (1 hit, in `#[cfg(test)] mod tests`, part of the `t7_f10_golden_second_spec_example` golden string):
- line 1042 — inside the `expected` multi-line literal: `"Week 2026-02:  Total still owed: 01h 40m\n\"` → `"Week 2026-02:  Total behind: 01h 40m\n\"`. This is the same golden block Fix C's own required test update (below) does NOT touch — `t7_f10_golden_second_spec_example` pins a past date with no `daily_target`/`weekday_name`/`eod`, so it is unaffected by Fixes B/C. Only the literal string changes here.

**`src/week_view.rs`** (4 hits, all in `#[cfg(test)] mod tests`):
- line 355 — `example_b()` fixture's `headline` field: `"Total still owed: 03h 10m".to_string()` → `"Total behind: 03h 10m".to_string()`
- line 386 — `EXAMPLE_B_GOLDEN` const, the headline line inside the multi-line raw string: `Total still owed: 03h 10m` → `Total behind: 03h 10m`
- line 455 — `a4_future_week_all_zero_rows_full_block_present`'s inline `WeekView.headline`: `"Total still owed: 40h 00m".to_string()` → `"Total behind: 40h 00m".to_string()`
- line 476 — `a5_never_touched_week_seven_zero_rows`'s inline `WeekView.headline`: `"Total still owed: 43h 12m".to_string()` → `"Total behind: 43h 12m".to_string()`

Note `example_a()` (the *current*-week fixture around line 320, not shown above) has no `"Total still owed"` literal — it uses the `Current` framing's `"... left by end of ..."` form, untouched by Fix A.

**`src/commands.rs`** (2 hits, both in `backdated_punch_retroactively_changes_a_later_closed_weeks_owed`, an end-to-end test against `crate::status::resolve`):
- line 1128 — `before.week_line.contains("Total still owed: 76h 00m")` → `before.week_line.contains("Total behind: 76h 00m")`
- line 1156 — `after.week_line.contains("Total still owed: 74h 00m")` → `after.week_line.contains("Total behind: 74h 00m")`

Both assertions' surrounding comments describe *why* the week is closed and owes that figure (carry-forward arithmetic across weeks 2026-05/2026-06) — that prose is correct and untouched; only the literal inside `.contains(...)` changes.

**Total: 12 call sites across 4 files** (7 render.rs + 1 status.rs + 4 week_view.rs, plus 2 commands.rs = 12; the grep above returns exactly these 12, confirming no other file references the old literal). Re-run the grep after editing to confirm zero remaining hits before calling Fix A done.

## Fix B — `EodState` gains a `(tomorrow)` bool

### Type change

`src/status.rs:46-52`:

```rust
// before
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EodState {
    /// `est. EOD HH:MM` -- quota not yet met, `now + gap`.
    At(NaiveTime),
    /// `target already met` -- quota already met or exceeded (gap <= 0).
    TargetAlreadyMet,
}

// after
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EodState {
    /// `est. EOD HH:MM` -- quota not yet met, `now + gap`. The bool is
    /// true exactly when the estimate falls on a later calendar date
    /// than `today`, rendered as a `(tomorrow)` suffix.
    At(NaiveTime, bool),
    /// `target already met` -- quota already met or exceeded (gap <= 0).
    TargetAlreadyMet,
}
```

No new variant (spec §1.1 non-goal, plan global constraint) — the existing `At` variant grows a second tuple field.

### Construction site

`src/status.rs:470-476`, inside `resolve()`'s `is_today` branch, where `has_open_stint` and `gap_minutes > 0`:

```rust
// before
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

// after
let eod = if has_open_stint {
    if gap_minutes > 0 {
        let eod_datetime = now + ChronoDuration::minutes(gap_minutes);
        let eod_time = eod_datetime.time();
        let is_tomorrow = eod_datetime.date_naive() != today;
        Some(EodState::At(eod_time, is_tomorrow))
    } else {
        Some(EodState::TargetAlreadyMet)
    }
} else {
    None
};
```

`now` is `DateTime<Local>` (the `resolve()` parameter); `now + ChronoDuration::minutes(gap_minutes)` is still `DateTime<Local>`. Call `.date_naive()` on that `DateTime<Local>` — **never `.date()`**, which is deprecated since chrono 0.4.23, returns `Date<Local>` (not `NaiveDate`), and would fail to type-compare against `today: NaiveDate` (bound at `status.rs:409`, in scope at this call site) even before considering that its deprecation lint is fatal under this repo's `cargo clippy --all-targets -- -D warnings` gate (confirmed live at `.github/workflows/ci.yml:50`). `.date_naive()` returns `NaiveDate` directly, comparable to `today` with plain `!=`.

`today` is already in scope in this function body (bound at line 409, `let today = now.date_naive();`), so no new parameter or import is needed — `chrono::Datelike` is already imported (used elsewhere in this file), and `DateTime<Local>::date_naive()` is an inherent method, not a trait method, so no new `use` is required for it either.

### Rendering change

`src/status.rs:184-188`, `day_total_line`'s match on `view.eod`:

```rust
// before
match &view.eod {
    Some(EodState::At(t)) => s.push_str(&format!(", est. EOD {}", t.format("%H:%M"))),
    Some(EodState::TargetAlreadyMet) => s.push_str(", target already met"),
    None => {}
}

// after
match &view.eod {
    Some(EodState::At(t, false)) => s.push_str(&format!(", est. EOD {}", t.format("%H:%M"))),
    Some(EodState::At(t, true)) => {
        s.push_str(&format!(", est. EOD {} (tomorrow)", t.format("%H:%M")))
    }
    Some(EodState::TargetAlreadyMet) => s.push_str(", target already met"),
    None => {}
}
```

Matching on `&view.eod` means `t` binds as `&NaiveTime` and the bool as a literal pattern (`false`/`true`) works directly on the referenced tuple field — no explicit deref needed since `NaiveTime: Copy` and the match ergonomics here mirror the existing `Some(EodState::At(t))` pattern, which already worked the same way. This exhaustively covers both bool values, so no wildcard arm is needed and clippy's exhaustiveness check stays satisfied.

### Every existing single-argument `EodState::At(t)` construction

Re-grepped live (`grep -n "EodState::At(" src/status.rs`), confirming the plan's list is accurate and complete:

| Line | Context | Before | After |
|---|---|---|---|
| 185 | production, `day_total_line` match arm | `Some(EodState::At(t))` | replaced by the two-arm match above, not a mechanical edit |
| 473 | production, `resolve()` construction | `Some(EodState::At(eod_time))` | replaced by the construction-site rewrite above |
| 658 | test `t1_f1_single_open_stint_no_completed` | `EodState::At(t(18, 0) + ChronoDuration::minutes(480))` | `EodState::At(t(18, 0) + ChronoDuration::minutes(480), false)` |
| 950 | test `t6a_f9_open_stint_with_gap_shows_est_eod` | `EodState::At(t(20, 45))` | `EodState::At(t(20, 45), false)` |
| 1158 | test `t11_golden_full_first_spec_example` | `EodState::At(t(20, 45))` | `EodState::At(t(20, 45), false)` |
| 1256 | test `t12_every_duration_matches_the_canonical_format` | `EodState::At(t(20, 45))` | `EodState::At(t(20, 45), false)` |
| 1320 | test `t13_plain_ascii_output_and_non_ascii_notes_pass_through` | `EodState::At(t(20, 45))` | `EodState::At(t(20, 45), false)` |

Five test sites, matching the plan's enumeration exactly (`t1`, `t6a`, `t11`, `t12`, `t13`) — confirmed by direct grep, not trusted from the plan's prose. `base_view()` (line 583-598) sets `eod: None` and has no `EodState::At` construction, confirming the plan's correction that it is not a sixth site. All five existing sites use `false` (same calendar day) since none of their fixtures cross midnight — none of these five tests assert anything about the `(tomorrow)` marker, so `false` preserves their exact current behavior.

### New coverage for the `(tomorrow)` branch

Fix B needs at least one new test exercising `Some(EodState::At(t, true))` end-to-end through `day_total_line`/`render`, and ideally one through `resolve()` confirming the `.date_naive() != today` computation itself (not just the rendering branch, which the render-level test alone wouldn't exercise).

**Render-level test** (add near `t6a`/`t6b`, in the `render` golden-test section):

```rust
// New -- Fix B: est. EOD carries a (tomorrow) suffix when the estimate
// falls on a later calendar date than today.
#[test]
fn fix_b_eod_estimate_tomorrow_gets_suffix() {
    let mut view = base_view();
    view.day_total_minutes = 60;
    view.has_open_stint = true;
    view.daily_target = Some(DailyTargetHint {
        required_minutes: 480,
        gap_minutes: 420,
    });
    view.weekday_name = Some("Thursday".to_string());
    view.eod = Some(EodState::At(t(1, 30), true));
    let out = render(&view);
    assert!(out.contains(", est. EOD 01:30 (tomorrow)"), "{out}");
}

// Companion negative case: same clock time, false bool, no suffix --
// confirms the two arms are actually distinct, not that "true" always
// wins on any input.
#[test]
fn fix_b_eod_estimate_same_day_has_no_tomorrow_suffix() {
    let mut view = base_view();
    view.day_total_minutes = 60;
    view.has_open_stint = true;
    view.daily_target = Some(DailyTargetHint {
        required_minutes: 480,
        gap_minutes: 420,
    });
    view.weekday_name = Some("Thursday".to_string());
    view.eod = Some(EodState::At(t(1, 30), false));
    let out = render(&view);
    assert!(out.contains(", est. EOD 01:30"), "{out}");
    assert!(!out.contains("(tomorrow)"), "{out}");
}
```

**`resolve()`-level test** (add near `resolve_f9b_large_carry_in_negative_pace_on_day_one`, in the F9b section) — needs a `gap_minutes` large enough that `now + gap_minutes` crosses midnight. `now_thu_1800()` is Thursday 2026-02-12 18:00; a gap of, say, 400 minutes (6h 40m) pushes past midnight to 00:40 Friday:

```rust
// New -- Fix B, resolve()-level: a gap_minutes large enough to push
// `now + gap` past midnight sets EodState::At's bool to true and
// day_total_line renders the (tomorrow) suffix.
#[test]
fn resolve_fix_b_eod_estimate_crossing_midnight_is_marked_tomorrow() {
    let conn = test_db();
    // No punches at all today: day_total_minutes stays 0, has_open_stint
    // is false by default -- so first give the day an open stint to
    // satisfy the `has_open_stint` precondition for `eod` to be `Some`.
    storage::insert_punch(&conn, PunchKind::Start, today(), t(9, 0), &Local).expect("insert");
    // now_thu_1800() is 18:00; required_minutes for Thursday (weekday 4)
    // is daily_target_minutes(2400) * 4 = 480 * 4 = 1920 (32h 00m).
    // fulfillment is near 0 (9h open stint counts toward day_total, not
    // week fulfillment, since fulfillment comes from the week ledger,
    // which has no completed stints yet) -- gap_minutes is therefore
    // large and positive, comfortably past midnight from 18:00.
    let view = resolve(None, now_thu_1800(), &conn).expect("resolve");
    let hint = view.daily_target.expect("is_today");
    assert!(hint.gap_minutes > 360, "gap must push well past midnight: {hint:?}");
    match view.eod {
        Some(EodState::At(_, is_tomorrow)) => assert!(is_tomorrow, "expected tomorrow marker"),
        other => panic!("expected EodState::At, got {other:?}"),
    }
    let out = render(&view);
    assert!(out.contains("(tomorrow)"), "{out}");
}
```

This test's exact `gap_minutes` figure should be verified against the real `week::week_accounting` output during implementation (the comment's arithmetic is a sketch, not a pre-verified number) — if `fulfillment` isn't what this sketch assumes, adjust the seeded punches until `gap_minutes` is confirmed > 360 by the assertion itself, which will fail loudly and cheaply if the setup doesn't achieve that.

### Edge case (documented, not tested further)

Per spec §2's edge case: a gap crossing two midnights still renders only `(tomorrow)`, not a more precise marker. This is accepted behavior, not a bug — no test needs to assert the "two midnights" case distinctly, since the implementation has no special-case branch for it (the `!=` comparison is binary regardless of how many days away the date is). One test asserting a two-midnight gap still says `(tomorrow)` and nothing more precise would be a reasonable regression guard but is not required by the acceptance criteria; recommend adding it only if the implementer wants extra confidence, not required for this task to pass review.

## Fix C — `DailyTargetHint` gains `day_reaches_week_cap: bool`

### Type change

`src/status.rs:57-64`:

```rust
// before
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DailyTargetHint {
    /// `daily target x min(today's ISO weekday number, 5)` (§2.4).
    pub required_minutes: i64,
    /// Signed: `required_minutes - fulfillment` (`carry_in + worked`,
    /// §2.4/§5) -- NOT `day_total_minutes`.
    pub gap_minutes: i64,
}

// after
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DailyTargetHint {
    /// `daily target x min(today's ISO weekday number, 5)` (§2.4).
    pub required_minutes: i64,
    /// Signed: `required_minutes - fulfillment` (`carry_in + worked`,
    /// §2.4/§5) -- NOT `day_total_minutes`.
    pub gap_minutes: i64,
    /// True when `weekday_number.min(5) == 5` -- i.e. `required_minutes`
    /// has plateaued at the week's full target (Friday, Saturday, and
    /// Sunday alike under the default 5-day work week). Drives
    /// `day_total_line`'s "required today" vs "required by end of
    /// {weekday}" choice; computed once in `resolve()`, where `today`
    /// and `weekday_number` are in scope -- `day_total_line` itself
    /// never sees either.
    pub day_reaches_week_cap: bool,
}
```

### Computation site

`src/status.rs:461-469`, inside `resolve()`'s `is_today` branch:

```rust
// before
let daily_target_minutes = week::daily_target_minutes(acct.target);
let weekday_number = i64::from(today.weekday().number_from_monday());
let required_minutes = daily_target_minutes * weekday_number.min(5);
let gap_minutes = required_minutes - acct.fulfillment;
let hint = DailyTargetHint {
    required_minutes,
    gap_minutes,
};

// after
let daily_target_minutes = week::daily_target_minutes(acct.target);
let weekday_number = i64::from(today.weekday().number_from_monday());
let required_minutes = daily_target_minutes * weekday_number.min(5);
let gap_minutes = required_minutes - acct.fulfillment;
let day_reaches_week_cap = weekday_number.min(5) == 5;
let hint = DailyTargetHint {
    required_minutes,
    gap_minutes,
    day_reaches_week_cap,
};
```

`weekday_number.min(5)` is computed twice (once for `required_minutes`, once for `day_reaches_week_cap`) rather than bound to a local and reused — this is a one-line, already-cheap `i64::min` call with no side effects, so the small duplication reads more clearly than introducing a new local purely to avoid it; either is acceptable, but the sketch above keeps `required_minutes`'s existing line untouched to minimize diff noise around Fix A/B's surrounding edits. If the implementer prefers `let capped_weekday = weekday_number.min(5);` reused in both places, that's a reasonable simplification-pass call and doesn't change behavior.

### Rendering change

`src/status.rs:170-183`, `day_total_line`'s daily-target clause:

```rust
// before
if let Some(hint) = &view.daily_target {
    let (word, magnitude) = if hint.gap_minutes >= 0 {
        ("left to", hint.gap_minutes)
    } else {
        ("over", -hint.gap_minutes)
    };
    s.push_str(&format!(
        ", {} {} {} required by end of {}",
        format_minutes(magnitude),
        word,
        format_minutes(hint.required_minutes),
        view.weekday_name.as_deref().unwrap_or_default()
    ));
}

// after
if let Some(hint) = &view.daily_target {
    let (word, magnitude) = if hint.gap_minutes >= 0 {
        ("left to", hint.gap_minutes)
    } else {
        ("over", -hint.gap_minutes)
    };
    if hint.day_reaches_week_cap {
        s.push_str(&format!(
            ", {} {} {} required today",
            format_minutes(magnitude),
            word,
            format_minutes(hint.required_minutes),
        ));
    } else {
        s.push_str(&format!(
            ", {} {} {} required by end of {}",
            format_minutes(magnitude),
            word,
            format_minutes(hint.required_minutes),
            view.weekday_name.as_deref().unwrap_or_default()
        ));
    }
}
```

No padding on the shorter `required today` form (spec §3's resolved layout constraint, plan global constraint) — it renders exactly as written.

### Every `DailyTargetHint { ... }` construction needs the new field

Since `DailyTargetHint` is not `#[non_exhaustive]` and derives no `Default`, every existing struct-literal construction across both files' test modules fails to compile without the new field. Re-grepping `grep -n "DailyTargetHint {" src/status.rs` at implementation time is the authoritative list; based on this read, the existing sites needing `day_reaches_week_cap: false` added (none of the current fixtures pin a Friday/Saturday/Sunday `today`, so `false` preserves current behavior for every one of them) are at minimum: lines 653, 685, 945, 964, 1001, 1153, 1251, 1315 (the T1/T2/T6a/T6b/T6c/T11/T12/T13 render-golden fixtures) plus the two `resolve_f9b_*` tests' expectations reference `view.daily_target` but construct it via `resolve()`, not a literal, so those need no direct edit to the struct literal (their assertions on `hint.required_minutes` also don't need a `day_reaches_week_cap` addition, since they read the field rather than construct it). Confirm the exact literal-construction line count via grep before editing, since Fix A/B's edits landing first will shift some of these line numbers.

### New test coverage (Fix C)

**Friday case** — new render-level test:

```rust
// New -- Fix C: Friday reaches the week cap, day-total clause says
// "required today", not "required by end of Friday".
#[test]
fn fix_c_friday_reaches_week_cap_says_required_today() {
    let mut view = base_view();
    view.day_total_minutes = 445;
    view.daily_target = Some(DailyTargetHint {
        required_minutes: 2400,
        gap_minutes: 100,
        day_reaches_week_cap: true,
    });
    view.weekday_name = Some("Friday".to_string());
    let out = render(&view);
    assert!(out.contains("required today"), "{out}");
    assert!(!out.contains("required by end of Friday"), "{out}");
    assert!(out.contains("01h 40m left to 40h 00m required today"), "{out}");
}
```

**Weekend case** (Saturday, per spec's own choice of Saturday for the analogous SPEC.md example Task 2 will mirror — using Saturday here keeps Task 1's own coverage consistent with what Task 2 quotes):

```rust
// New -- Fix C: Saturday also reaches the week cap (not "the one day" --
// spec §3's correction).
#[test]
fn fix_c_saturday_reaches_week_cap_says_required_today() {
    let mut view = base_view();
    view.day_total_minutes = 0;
    view.daily_target = Some(DailyTargetHint {
        required_minutes: 2400,
        gap_minutes: 2400,
        day_reaches_week_cap: true,
    });
    view.weekday_name = Some("Saturday".to_string());
    let out = render(&view);
    assert!(out.contains("required today"), "{out}");
    assert!(!out.contains("required by end of Saturday"), "{out}");
    // Same required_minutes figure as the adjacent Friday case above --
    // both plateau at the full week target.
    assert!(out.contains("40h 00m required today"), "{out}");
}
```

**Mon-Thu unchanged case**: the existing `t2_f2_ordinary_day_no_eod_line_without_open_stint` (Thursday, `weekday_name: "Thursday"`) already covers this once its `DailyTargetHint` literal gains `day_reaches_week_cap: false` — its existing assertion `out.contains("... required by end of Thursday")` stays a correct, unchanged assertion. No new test is needed for this leg; the mechanical field addition to the existing literal, with `false`, is itself the regression guard (spec's "Monday-through-Thursday case (the existing Thursday fixture, today pinned at 2026-02-12) still renders required by end of {weekday} unchanged").

**`resolve()`-level coverage**: `resolve_f9b_saturday_pin_multiple_of_five_target` (status.rs:1739-1757) already pins a Saturday and checks `hint.required_minutes`/`weekday_name` but does not call `render()`, so it won't break under this fix (confirmed by spec §3) — but it's a natural place to extend with a `day_reaches_week_cap` assertion:

```rust
// Extend resolve_f9b_saturday_pin_multiple_of_five_target with:
assert!(hint.day_reaches_week_cap, "Saturday reaches the week cap");
```

Add this single assertion line to that existing test rather than duplicating the whole test body.

### The one existing test that WILL BREAK

`resolve_f9b_sunday_pin_non_multiple_of_five_target_override` (`status.rs:1759-1780`) pins Sunday and currently asserts, via `render()`:

```rust
assert!(out.contains("33h 30m required by end of Sunday"));
```

This assertion is false once Fix C lands — Sunday is `weekday_number.min(5) == 5`, so `day_reaches_week_cap` is `true` and the real rendered clause becomes `"33h 30m required today"`. Update in place as part of this fix, not as incidental fallout discovered later (spec's explicit instruction):

```rust
// before
assert!(out.contains("33h 30m required by end of Sunday"));

// after
assert!(out.contains("33h 30m required today"));
```

Recommend also adding, immediately after, a companion negative assertion for symmetry with the Friday/Saturday tests above:

```rust
assert!(!out.contains("required by end of Sunday"));
assert!(hint.day_reaches_week_cap, "Sunday reaches the week cap");
```

(`hint` is already bound earlier in this test body as `let hint = view.daily_target.expect("is_today");` — the extra assertion is a one-line addition, not a restructure.)

## Combined mechanical note: Fix B + Fix C both touch `day_total_line` and `DailyTargetHint`/`EodState` construction sites

Since almost every test fixture that constructs a `DailyTargetHint` literal or an `EodState::At(..)` also needs updating for whichever of B/C it touches (some need both — e.g. `t6a`, `t11`, `t12`, `t13` all set both `eod: Some(EodState::At(...))` and `daily_target: Some(DailyTargetHint { ... })`), the practical implementation order that minimizes churn is:

1. Land the two type changes (`EodState`, `DailyTargetHint`) together — the compiler will then flag every call site that needs updating as a compile error, which is a more reliable inventory than any grep-based list (including this plan's own).
2. Fix each flagged site: add `, false` to `EodState::At(t)` sites (all five existing ones use `false`, per the audit above) and add `day_reaches_week_cap: false` to `DailyTargetHint` literals (all existing ones use `false`, since none currently pins a Fri/Sat/Sun `today`), *except* the one known-breaking assertion in `resolve_f9b_sunday_pin_non_multiple_of_five_target_override`, which needs its *assertion string* changed, not a struct literal it doesn't construct.
3. Then add the new tests (Fix B's tomorrow-marker tests, Fix C's Friday/weekend tests) on top of a codebase that already compiles and passes.
4. Apply Fix A's literal rename across all four files last, or first — it's independent of B/C and touches disjoint lines within `render.rs`/`status.rs`, so ordering relative to B/C doesn't matter; ordering it last is slightly safer since it touches the fewest lines that could conflict with B/C's edits to the same functions.

This ordering note is implementation guidance, not a new acceptance criterion — the compiler-driven inventory in step 1 should be treated as authoritative over both this plan's line-number tables and the changeset plan's, since both are snapshots that could have drifted.

## Test plan, by acceptance criterion

### Fix A (spec §4)

- **AC**: `week_headline`'s `Closed if owed_minutes > 0` branch renders `"Total behind: {}"`.
  - Covered by: `t2_past_week_headline_plain_total`, `t3_status_first_example_past_week`, `t7_future_week_gets_the_same_plain_form_as_closed`, `t10_one_minute_owed_is_still_owed_the_boundary_above_t8` (all in `render.rs`), each updated to assert the new literal.
- **AC**: the `owed_minutes <= 0` sibling branch is byte-identical.
  - Covered by: `t8_zero_owed_on_a_closed_week_is_total_ahead`, `t9_negative_owed_renders_as_a_magnitude_under_total_ahead`, `t11_current_week_already_ahead_passes_sign_through_verbatim` — none of these three assert `"Total still owed"`/`"Total behind"` at all, and none needs editing; their continued pass, unedited, is the byte-identical guarantee.
- **AC**: every test in `render.rs`/`status.rs`/`week_view.rs`/`commands.rs` asserting the old literal is updated.
  - Covered by: the 12-site inventory above; re-run `grep -rn "Total still owed" src/` after editing and confirm zero hits.

### Fix B (spec §2)

- **AC**: `EodState::At` becomes `(NaiveTime, bool)`; bool true iff `(now + gap_minutes).date_naive() != today`.
  - Covered by: `resolve_fix_b_eod_estimate_crossing_midnight_is_marked_tomorrow` (new, `resolve()`-level, asserts the bool via pattern match) and the five existing sites (`t1`, `t6a`, `t11`, `t12`, `t13`) continuing to pass with `false`, confirming the non-crossing case stays `false`.
- **AC**: `day_total_line` renders `, est. EOD {t}` when false, `, est. EOD {t} (tomorrow)` when true.
  - Covered by: `fix_b_eod_estimate_tomorrow_gets_suffix` (new) and `fix_b_eod_estimate_same_day_has_no_tomorrow_suffix` (new), both render-level, both directly asserting the exact suffix presence/absence.
- **AC**: every existing `EodState::At(t)` site updated to two-argument form.
  - Covered by: the five-site table above, cross-checked against `grep -n "EodState::At(" src/status.rs` re-run at implementation time.
- **AC** (edge case, not a hard requirement): gap crossing two midnights still renders only `(tomorrow)`.
  - Not separately tested per this plan (see Fix B section) — accepted as documented, non-blocking behavior; add only if the implementer wants belt-and-suspenders coverage.

### Fix C (spec §3)

- **AC**: `DailyTargetHint` gains `day_reaches_week_cap: bool`, computed as `weekday_number.min(5) == 5`, threaded through unchanged.
  - Covered by: the computation-site sketch above (direct code inspection during review) plus every test that sets the field explicitly (Friday/Saturday cases) or via `resolve()` (the extended `resolve_f9b_saturday_pin_multiple_of_five_target` assertion and the corrected `resolve_f9b_sunday_pin_non_multiple_of_five_target_override`).
- **AC**: `day_total_line` renders `required by end of {weekday}` when false, `required today` when true, no padding.
  - Covered by: `fix_c_friday_reaches_week_cap_says_required_today`, `fix_c_saturday_reaches_week_cap_says_required_today` (both new), and the existing `t2_f2_ordinary_day_no_eod_line_without_open_stint` / `t6a`/`t6b`/`t6c` Thursday-pinned tests (updated only with `day_reaches_week_cap: false`, assertions otherwise unchanged) confirming the `false` branch's wording and lack of padding stays byte-identical to today.
- **AC**: Friday case and one weekend case each assert `required today` and the *same* `required_minutes` figure as the adjacent weekday case.
  - Covered by: `fix_c_friday_reaches_week_cap_says_required_today` and `fix_c_saturday_reaches_week_cap_says_required_today` both use `required_minutes: 2400` (`40h 00m`), matching each other — satisfying "same figure" directly within this task's own two new tests, without needing a third comparison fixture.
- **AC**: Mon-Thu case (existing Thursday fixture, today pinned 2026-02-12) unchanged.
  - Covered by: every existing Thursday-pinned test (`TODAY`/`today()` throughout `status.rs`'s test module is 2026-02-12, a Thursday) continuing to assert `required by end of Thursday`, now via an explicit `day_reaches_week_cap: false` field rather than the field's absence.
- **AC**: `resolve_f9b_sunday_pin_non_multiple_of_five_target_override` updates from `"33h 30m required by end of Sunday"` to `"33h 30m required today"` as part of Fix C, not incidental fallout.
  - Covered by: the direct edit specified above, plus the recommended companion negative assertion and `day_reaches_week_cap` check.

### Regression (plan's own AC)

- **AC**: every existing test not touched by the three fixes passes unchanged.
  - No dedicated new test — this is the existing suite's continued green status after the mechanical edits above, verified by `cargo test` in the verification gate.

### Verification gate (plan's own AC)

- **AC**: `cargo fmt`, `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings` all pass.
  - Run all four locally in that order per `AGENTS.md`; additionally run the two `cargo run` smoke-check lines from the same section (`MLM_DB_PATH=/tmp/mlm-check.db cargo run -- start "9:00" "note"` then `... cargo run -- status`) to confirm the real binary's `status` output still renders sanely end-to-end, since this task touches `status`'s render path directly (plan's Task-1-specific addition to the global verification set). CI (`ci.yml`) additionally runs a cross-platform e2e smoke test that greps for `"Day total:"` and `"Target:"` in real `status`/`week` output — neither grep target is a string this changeset touches, so CI's smoke assertions should continue passing unmodified, but the implementer should watch CI's full run (not just local `cargo test`) before calling this task done, since CI matrixes across 5 OS/arch targets local dev doesn't cover.

## Risks and ambiguities

- **`t10`'s stale test name**: `t10_one_minute_owed_is_still_owed_the_boundary_above_t8` (render.rs:333) has "still_owed" baked into its *identifier*, not just its assertion string. This plan leaves the identifier unchanged (only the assertion literal updates) on the grounds that renaming test function names is a separate, unscoped cleanup — but a reviewer could reasonably want the name updated too for consistency with the new wording. Flagging rather than deciding unilaterally, since the changeset plan doesn't mention test *identifiers* anywhere, only string assertions.
- **Fix C's `weekday_number.min(5)` double-computation**: the sketch above computes `weekday_number.min(5)` twice (once inline for `required_minutes`, once for `day_reaches_week_cap`) rather than binding it to a local once. This is a stylistic call, not a behavior question — flagged in the Fix C section itself with the alternative spelled out, deferring to whichever the implementer/reviewer prefers without blocking on it.
- **New test count**: this plan adds roughly 8 new tests (2 for Fix B's tomorrow marker, 2 for Fix C's Friday/weekend cases, plus small assertion additions to 2 existing tests) on top of the ~15 existing tests needing mechanical field/argument updates. None of these new tests are strictly required beyond "Coverage: a Friday case and one weekend case" and "every existing `EodState::At(t)` construction... updated" from the acceptance criteria — the render-level pair for each fix could in principle be collapsed to one test each if the implementer wants fewer, more densely-asserting tests instead. This plan errs toward one assertion-focused test per behavior for clearer failure attribution, not because the spec demands this exact count.
- **`resolve_fix_b_eod_estimate_crossing_midnight_is_marked_tomorrow`'s exact `gap_minutes` figure is a sketch, not a verified number.** The comment inside that test's code sketch says so explicitly — the implementer must run it and adjust the seeded fixture (or the assertion threshold) if the actual `gap_minutes` computed by `resolve()` doesn't land where the sketch assumes. This is flagged deliberately rather than presented as exact, since deriving it precisely requires tracing `week::week_accounting`'s carry-in/fulfillment arithmetic against a specific empty-ledger scenario, which is more reliably done by running the test than by hand-computing it in this plan.
- **No disagreement with the locked plan or spec.** Both went through one adversarial review round each and this task's read of them found no gaps, contradictions, or infeasibilities against the current source — the constructions, field additions, and rendering branches described in both documents map cleanly onto the actual code at every cited line number (modulo the expected minor drift the spec/plan themselves already flag as a risk, which this plan's own re-greps corrected where found — e.g. confirming the EodState five-site list and the 12-site Fix A inventory independently rather than trusting either document's numbers).
