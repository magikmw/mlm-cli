# Milestone 6 — Week accounting (target, carry, fulfillment)

Implementation plan for a TDD subagent. Tests first, implement to green.
Everything below is concrete enough to start writing tests immediately;
where a decision is genuinely open it is called out in §8 (Risks) with a
default already chosen so nothing blocks.

**Spec basis**: SPEC.md §1.1 (carry adjusts fulfillment, never target),
§1.3 (target/fulfillment/carry terms), §2.3 (`week_targets` sparse
overrides, default 40h = 2400), §2.4 (walk *every* week from the earliest
data week, including idle gaps; daily-target derivation), §5 (worked
example + formulas), §6.2 (unset target is not an error).
**PLAN.md basis**: wave-1 fixture-driven milestone; interface contracts
3 ("week accounting result shape") and 6 ("now as an injected value").
**NOTES.md basis**: decisions 3, 17, 37, 38.

**Hard boundary**: no CLI, no rendering, no SQL, no `chrono::Local::now()`
anywhere in this milestone. Pure functions over injected data. This
milestone must compile and pass its whole test suite with Milestones
4 and 5 still unimplemented.

---

## 1. Module layout

New file `src/week.rs`, declared as `mod week;` in `src/main.rs`.
(Do **not** touch `cli.rs`, `db.rs`, or `time.rs` — Milestones 1/2/3 own
those and are in flight in parallel worktrees. `main.rs` gets exactly one
added `mod` line; if even that conflicts, declare the module and leave
`main`'s body untouched.)

Unit tests live in a `#[cfg(test)] mod tests` at the bottom of
`src/week.rs` (the §5 table is small; no `tests/` integration file needed
at this stage — the end-to-end version of these assertions is Milestone
11's job).

Everything public in this module is `pub` so Milestones 9/10/11 can
consume it.

Minutes are `i64` everywhere. Not `u32`, not `i32`, not `Duration`:
carry, owed and fulfillment are all legitimately signed (§5), and `i64`
removes any overflow worry at personal-use scale. Conversion to
`chrono::Duration` (if the renderer wants it) happens at the display
edge in Milestone 1's formatter, not here.

---

## 2. Types this milestone needs as *input* (contracts)

### 2.1 `WeekId` — Milestone 2's actual type, used as-is (contract 9)

**Cross-plan fix**: this milestone originally hand-rolled its own
`WeekId { pub year: i32, pub week: u32 }` with public fields and its
own `monday()`/`from_date()`/`next()` methods, flagged as tentative
pending Milestone 2's real design. Milestone 2 has since landed with a
materially different, validated design (private fields, a checked
constructor, methods named `start()` not `monday()`, and accessors
`iso_year()`/`week()`). Per PLAN.md contract 9, **this milestone
consumes Milestone 2's `WeekId` directly — no local re-declaration.**

Adjust every reference below accordingly:
- `WeekId { year, week }` literal construction in tests → use
  Milestone 2's validated constructor instead (test fixtures cannot
  build an invalid `WeekId` by construction, which is a strict
  improvement over the original plan's unchecked public fields).
- `.monday()` → `.start()`.
- `.year` / `.week` field access → `.iso_year()` / `.week()` accessor
  calls.
- `from_date()` and `next()` are unchanged in name and behavior.
- The derived-`Ord`-gives-chronological-order property, and `next()`'s
  date-based implementation (next paragraph), both still hold — they
  were correct calls, just attached to the wrong type before.

Three helpers this milestone needs, now **Milestone 2's**, not
reimplemented here: `start()` (Monday of the week, a `NaiveDate`),
`from_date(date: NaiveDate) -> WeekId` (the ISO week containing a
date), and `next(self) -> WeekId` (the next ISO week in sequence).

`next()` must be implemented as **`WeekId::from_date(self.start() + Duration::days(7))`**,
never as "week + 1, and if week > 52 then year + 1, week = 1". The
naive arithmetic version silently corrupts every 53-week year (2020,
2026, 2032, …). This is the single most likely correctness bug in the
milestone and must have its own test (§7.9).

`monday()` uses `NaiveDate::from_isoywd_opt(year, week, Weekday::Mon)`;
`from_date()` uses `date.iso_week()` → `(.year(), .week())`.

### 2.2 The worked-minutes source

Per SPEC.md §2.4 and NOTES.md decisions 37/38, the minutes this source
reports are **completed stints only**: no live minutes from a currently
open stint, no contribution from an orphaned `end`. That rule is the
*source's* responsibility (Milestone 5 pairing + Milestone 4 queries) —
this milestone consumes whatever number it is handed and never inspects
stints itself. State that assumption in a doc comment on the trait so the
wave-3 integrator cannot miss it.

Two things are needed from the source, and the second is the one that is
easy to overlook: the walk cannot start without knowing **the earliest
week that has any data at all** (§2.4: "starting from the earliest week
with any punch/note data").

```rust
/// A source of per-week completed-stint totals.
///
/// Implementations MUST report completed stints only (SPEC §2.4,
/// NOTES decisions 37/38): open stints' live minutes and orphaned
/// `end`s contribute nothing here.
pub trait WeekData {
    /// The earliest ISO week with any punch or note data, or `None`
    /// when the database is completely empty.
    fn earliest_data_week(&self) -> Option<WeekId>;

    /// Total completed-stint minutes recorded in `week`.
    /// Returns 0 for a week with no data — never an error, never
    /// an `Option` (SPEC §6.2: an empty week is valid, not an error).
    fn worked_minutes(&self, week: WeekId) -> i64;
}

/// A source of sparse per-week target overrides (`week_targets`).
pub trait WeekTargets {
    /// `Some(minutes)` when the week has an override row (including a
    /// legal `0`), `None` when it has none (SPEC §2.3/§6.2 — the
    /// caller then applies `DEFAULT_WEEK_TARGET_MINUTES`).
    fn target_override(&self, week: WeekId) -> Option<i64>;
}
```

Note carefully: `target_override` returns `Option<i64>` and `Some(0)` is
**not** the same as `None`. A zero override is a deliberate week off
(§2.3, F7b); collapsing the two would make F7b indistinguishable from a
default 40h week. Do not use `unwrap_or(0)` anywhere near this.

**Why two traits rather than one**: `week_targets` is a separate table
with genuinely different access (a small sparse map, trivially loadable
whole) from the punch aggregation. Keeping them separate lets Milestone
8's `week target` writer and Milestone 4's punch reader land
independently. Callers that have both can pass the same struct twice if
it implements both traits.

**Fixture implementation** (used by every test in this milestone, and
reusable by Milestones 9/10/11's tests):

```rust
/// In-memory `WeekData` + `WeekTargets`, used by tests now and by the
/// real code path later (Milestone 4 loads one query's worth of
/// GROUP BY rows straight into it).
#[derive(Debug, Default, Clone)]
pub struct WeekLedger {
    worked: BTreeMap<WeekId, i64>,   // sparse: absent == 0 worked
    data_weeks: BTreeSet<WeekId>,    // weeks with ANY punch/note data
    targets: BTreeMap<WeekId, i64>,  // sparse: absent == default target
}

impl WeekLedger {
    pub fn new() -> Self;
    /// Record a week's worked minutes AND mark it a data week.
    pub fn with_worked(self, week: WeekId, minutes: i64) -> Self;
    /// Mark a week as having data (notes, or punches summing to zero)
    /// without adding worked minutes — needed for a note-only week.
    pub fn with_data_week(self, week: WeekId) -> Self;
    /// Record a target override (0 is legal and meaningful).
    pub fn with_target(self, week: WeekId, minutes: i64) -> Self;
}
```

Builder style (`self` by value, returns `Self`) so a four-week fixture is
four chained calls and the §5 table test reads like the §5 table.

`earliest_data_week()` = `self.data_weeks.iter().next().copied()` — this
is why `data_weeks` is a `BTreeSet` and why `WeekId`'s `Ord` must be
year-then-week.

**`data_weeks` is deliberately separate from `worked`**, not derived from
`worked.keys()`. A week can have data but zero worked minutes: a
note-only week (F4's shape at week scale), or a week whose only punches
are an unmatched `start` and an orphaned `end`. Such a week must still
anchor the walk's start. Deriving the earliest data week from
`worked` alone would silently mis-start the chain. (`with_worked` inserts
into both maps, so the common case stays one call.)

### 2.3 "Now"

Per PLAN.md contract 6, this milestone never reads a clock. The only
thing it needs "now" for is the `is_current_week` flag, and the only
part of "now" that matters is the **local calendar date**. So:

```rust
today: NaiveDate   // the caller's local calendar date (§2.1: local, not UTC)
```

passed as a plain parameter. Not `DateTime<Local>`, not `Utc::now()`.
The caller (Milestone 10/11) is responsible for deriving the local date
per §2.1's per-instant rule; this milestone just receives it.

---

## 3. The result type this milestone *produces*

Honors PLAN.md interface contract 3 exactly — the six signed-minute
fields plus the explicit current-week boolean, so Milestone 9's headline
decision is a field read rather than a recomputation.

```rust
/// One week's fully-computed accounting. All minute fields are signed:
/// nothing is clamped anywhere (SPEC §5's closing note).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeekAccounting {
    /// Which week this row describes.
    pub week: WeekId,
    /// Override if set (including 0), else DEFAULT_WEEK_TARGET_MINUTES.
    pub target: i64,
    /// Previous week's `carry_out`; 0 for the first week of the walk.
    pub carry_in: i64,
    /// Completed-stint minutes for this week (NOTES 37/38). Always >= 0.
    pub worked: i64,
    /// `worked + carry_in`.
    pub fulfillment: i64,
    /// `target - fulfillment`. Negative == ahead of target.
    pub owed: i64,
    /// `fulfillment - target`; becomes the next week's `carry_in`.
    /// (Exactly `-owed`; both are stored because contract 3 names both
    /// and both are rendered by §7.2's output.)
    pub carry_out: i64,
    /// True iff this week is the week containing the caller's `today`.
    /// Milestone 9 reads this instead of recomputing the comparison
    /// (PLAN.md contract 3).
    pub is_current_week: bool,
}
```

`#[derive(PartialEq, Eq)]` is load-bearing: it lets the §5 table test
assert whole rows in one `assert_eq!` against a literal, which is how
that test stays readable.

Constant:

```rust
/// SPEC §2.3 / §6.2: 40h, used when a week has no `week_targets` row.
pub const DEFAULT_WEEK_TARGET_MINUTES: i64 = 2400;
```

Deliberately **not** returning a `Result`: there is no error case in this
milestone. Every input combination — empty database, week before all
data, week far in the future, zero target, huge carry — produces a valid
result (§6.2, E13). Any `Result` here would be a design smell.

---

## 4. Public API

```rust
/// Compute one week's accounting by walking the full chain up to it.
pub fn week_accounting(
    week: WeekId,
    data: &impl WeekData,
    targets: &impl WeekTargets,
    today: NaiveDate,
) -> WeekAccounting;

/// The full walk: every week from the walk start through `through`,
/// in chronological order, inclusive at both ends. Never empty.
/// `week_accounting` is `week_series(..).last()`.
pub fn week_series(
    through: WeekId,
    data: &impl WeekData,
    targets: &impl WeekTargets,
    today: NaiveDate,
) -> Vec<WeekAccounting>;

/// SPEC §2.4 / §7.1 / NOTES 17: a `status`-only pace hint.
/// `week target / 5`, floored to the minute. No override, no storage,
/// no interaction with carry.
pub fn daily_target_minutes(week_target_minutes: i64) -> i64;
```

`week_series` is the real implementation; `week_accounting` is a thin
wrapper over its last element. Exposing the series is worth it: the §5
table test asserts all four rows from one call, and Milestone 11 may want
the chain for debugging. It also makes the "did the gap week actually get
walked" test (§7.5) assertable directly rather than inferred.

Resolve the target once, in one place:

```rust
fn resolve_target(week: WeekId, targets: &impl WeekTargets) -> i64 {
    targets.target_override(week).unwrap_or(DEFAULT_WEEK_TARGET_MINUTES)
}
```

`daily_target_minutes` must use **`div_euclid(5)`**, not `/ 5`. For the
non-negative targets the schema permits (`target_minutes >= 0`) the two
agree, but `/` truncates toward zero rather than flooring, so a plain `/`
would be silently wrong the day anything negative reaches it. §2.4 says
"floor"; write floor. Add a comment saying exactly this so a reviewer
doesn't "simplify" it back.

---

## 5. The week-walk algorithm

### 5.1 Determining the walk's start week

```
start = match data.earliest_data_week() {
    Some(earliest) => min(earliest, through),
    None           => through,
}
```

Three cases, all required:

1. **Normal**: data exists and `through` is at or after the earliest data
   week → start at the earliest data week and walk forward. This is the
   §2.4 rule verbatim.
2. **`through` precedes all data** (asking for a week before tracking
   began — a legal E13 case for a past week): `min` picks `through`, so
   the walk is a single week with `carry_in = 0`. Walking forward from a
   *later* earliest week would never reach `through` and would loop
   forever or return empty; walking *backwards* is not a thing (carry
   only flows forward). `min` is what makes this case terminate.
3. **Empty database** (`None`): start at `through`. Single week, all
   zeros, default target, `carry_in = 0`. This is E13's other half and
   must not error.

The walk is unbounded above only by `through`, which is caller-supplied,
so it always terminates. Add a defensive note (not an assert) that the
loop is `O(weeks between start and through)` — §2.4 explicitly accepts
this cost.

### 5.2 The fold

```
carry = 0
current = start
out = Vec::new()
loop {
    target      = resolve_target(current, targets)
    worked      = data.worked_minutes(current)     // 0 for a gap week
    fulfillment = worked + carry
    owed        = target - fulfillment
    carry_out   = fulfillment - target
    out.push(WeekAccounting {
        week: current, target, carry_in: carry, worked,
        fulfillment, owed, carry_out,
        is_current_week: current == WeekId::from_date(today),
    })
    if current == through { break }
    carry = carry_out
    current = current.next()
}
out
```

Points a reviewer will check, and that the tests must pin:

- **Every week in the range is visited**, including ones with zero data.
  There is no "skip if empty" branch, no `if worked == 0 { continue }`.
  This is §2.4's explicit fix and F8b's whole point. A gap week goes
  through the identical arithmetic with `worked = 0`, which is precisely
  how it accrues its full `-target` deficit.
- **Carry is threaded from `carry_out` to the next `carry_in`** — no
  separate accumulator, no re-deriving from `owed`.
- **First week's `carry_in` is 0** (E12) — it falls out of `carry = 0`
  before the loop; no special-casing inside it.
- **Nothing is clamped** (§5's closing note): no `max(0, ..)`, no
  `saturating_*`, no `abs()`. `owed` may be negative, `carry_in` may be
  positive or negative, `fulfillment` may exceed `target` in either
  direction. Reviewers should grep the implementation for `max(`, `min(`
  (outside §5.1's start selection), `abs` and `clamp` and find none.
- **`is_current_week` is computed per row** against the same injected
  `today`, so past rows in the series are correctly `false`. Hoist
  `WeekId::from_date(today)` out of the loop (compute it once before it)
  — it's the only clock-adjacent value in the function.
- `carry_out == -owed` identically. Both are stored per contract 3; do
  **not** "optimize" one away, and do not compute one from the other by
  negation — compute each from its own §5 formula so a sign error in one
  cannot hide.

### 5.3 §5 reproduced

With `2026-01..2026-04` worked = 2200 / 2500 / 1950 / 2600 and a 2000
override on `2026-03`, the fold produces (verified by hand against
SPEC.md §5):

| week | target | carry_in | worked | fulfillment | owed | carry_out |
|---|---|---|---|---|---|---|
| `2026-01` | 2400 | 0 | 2200 | 2200 | 200 | -200 |
| `2026-02` | 2400 | -200 | 2500 | 2300 | 100 | -100 |
| `2026-03` | 2000 | -100 | 1950 | 1850 | 150 | -150 |
| `2026-04` | 2400 | -150 | 2600 | 2450 | -50 | 50 |

---

## 6. Ordering of work (TDD)

1. `WeekId` + `monday`/`from_date`/`next`, with §7.9's 53-week test
   first. Nothing else works if `next()` is wrong.
2. `daily_target_minutes` (§7.8) — trivial, gets it out of the way.
3. `WeekLedger` fixture builder + the two trait impls.
4. `week_series`'s fold, driven by §7.1's §5 table test.
5. The remaining edge-case tests (§7.2–§7.7), each of which should pass
   without new production code if the fold is right — if one requires a
   new branch, that branch is a bug in the fold, not a missing feature.

---

## 7. Test cases (exhaustive list — write all of these)

All tests construct `WeekId` literals and a `WeekLedger`; none touch
SQLite, the clock, or string parsing. `today` is a `NaiveDate` literal.
Pick a `today` far from the fixture weeks (e.g. `2026-06-15`) in every
test where `is_current_week` isn't the subject, so no test accidentally
depends on it.

### 7.1 §5's four-week table, exactly (primary test)

- Fixture: `2026-01` 2200, `2026-02` 2500, `2026-03` 1950 with a 2000
  target override, `2026-04` 2600. No other overrides.
- Call `week_series(WeekId{2026,4}, ..)`.
- Assert the vec has length 4 and each element equals the literal
  `WeekAccounting` from the table in §5.3 — all seven numeric fields, per
  row, not just the last week's `owed`.
- Also assert `week_accounting(WeekId{2026,4}, ..)` equals row 4, i.e.
  the wrapper agrees with the series' tail.
- Covers: F8, the `2026-03` override case, the negative-`owed` /
  positive-`carry_out` week (`2026-04`), and the no-clamping rule in one
  go.

### 7.2 First tracked week has `carry_in = 0` (E12)

- Fixture: a single data week `2026-01` with 2200 worked.
- `week_accounting(WeekId{2026,1})` → `carry_in == 0`, `worked == 2200`,
  `fulfillment == 2200`, `target == 2400`, `owed == 200`,
  `carry_out == -200`.
- Series length is exactly 1 — assert this, to prove the walk did not
  invent weeks before the earliest data week.

### 7.3 Default target when unset (§6.2 / F6)

- Fixture: one data week, no override rows at all.
- `target == DEFAULT_WEEK_TARGET_MINUTES == 2400`, and the call returns
  normally (no panic, no `Result` to unwrap).
- Second case: a week with an override of 2000 and the *following* week
  with none → the override applies only to its own week; the next week
  is back to 2400. Pins that overrides are per-week and non-sticky.

### 7.4 Zero-target override (F7b / §2.3)

- Fixture: `2026-10` with a `0` override and 600 worked minutes;
  `carry_in` 0 (make it the first data week).
- Assert `target == 0`, `fulfillment == 600`, `owed == -600` (ahead by
  600 — negative, uncapped), `carry_out == 600`.
- Second case: **zero target with zero worked** → `owed == 0`,
  `carry_out == 0`, i.e. a zero-target week accrues *no* deficit, which
  is exactly the point of §2.3's "deliberate week off".
- Third case, the trap: a week with a `0` override must behave
  differently from a week with *no* override. Assert side by side that
  the `Some(0)` week gets `target == 0` while an otherwise-identical
  no-override week gets `target == 2400`. This is the test that catches
  an `unwrap_or(0)`.

### 7.5 Idle gap week carries through (F8 / F8b / §2.4 — the fix)

- Fixture: `2026-01` worked 2400 (exactly on target, so `carry_out` is 0
  and cannot mask a bug), `2026-02` **absent entirely** from the ledger
  (not present with a zero — genuinely no entry, no data-week mark),
  `2026-03` worked 2400.
- `week_series(WeekId{2026,3})` has length **3**, and element 1 is the
  gap week: `week == 2026-02`, `worked == 0`, `carry_in == 0`,
  `target == 2400`, `fulfillment == 0`, `owed == 2400`,
  `carry_out == -2400`.
- `2026-03`'s `carry_in == -2400` and its `owed == 2400`.
- Equivalence assertion (F8b's exact wording): build a second ledger
  where `2026-02` is explicitly present with 0 worked minutes and is
  marked a data week, and assert the `2026-03` row is **identical** in
  both. "Skipping zero-data weeks" must be unobservable because it never
  happens.
- Two-gap variant: `2026-01` data, `2026-02` and `2026-03` both absent,
  `2026-04` data → `2026-04`'s `carry_in == -4800`. Proves the walk is a
  loop, not a one-week lookback.

### 7.6 Never-touched week (E13)

Four sub-cases, all must return a full result rather than an error:

a. **Future week, after the data**: data in `2026-01` only; request
   `2026-20`. Series spans `2026-01..=2026-20` (assert length 20 —
   catches an off-by-one at either end), the requested row has
   `worked == 0`, `target == 2400`, and a `carry_in` equal to the
   accumulated deficit of the 19 preceding weeks.
b. **Past week, before all data**: data in `2026-10` only; request
   `2026-05`. Series length 1, `carry_in == 0`, `worked == 0`,
   `owed == 2400`. (This is §5.1's `min` branch; without it this case
   hangs or returns empty.)
c. **Completely empty ledger**: no data anywhere; request any week.
   Series length 1, all-zero worked, `carry_in == 0`, default target,
   `owed == 2400`. No panic, no error.
d. **Note-only earliest week**: ledger where `2026-01` is marked a data
   week via `with_data_week` (no worked minutes) and `2026-02` has 2400
   worked. Request `2026-02` → series length **2** and `2026-02`'s
   `carry_in == -2400`. This is the test that proves `earliest_data_week`
   is not derived from `worked.keys()`; if it were, the series would be
   length 1 and `carry_in` would be 0.

### 7.7 `is_current_week`

- Data spanning `2026-01..2026-04`; `today` = a `NaiveDate` that falls
  inside ISO week `2026-03` (e.g. `2026-01-15` — the implementer should
  confirm the ISO week of whatever literal they pick with
  `NaiveDate::iso_week`, not assume it).
- In the returned series, exactly one row has `is_current_week == true`
  and it is the `2026-03` row; all others are `false`.
- Separate case: `today` inside a week *outside* the series range → every
  row is `false`.
- Boundary pair: `today` = that week's Monday and `today` = that week's
  Sunday both mark the same week current (catches an off-by-one in
  `from_date`/Monday-start handling).
- Cross-year boundary case: a `today` in very late December or very early
  January whose ISO week belongs to the *other* ISO year (e.g.
  `2027-01-01`, which is ISO week `2026-53`) still matches the right
  `WeekId` — this is the case a naive `today.year()` implementation gets
  wrong.

### 7.8 Daily target derivation (§2.4 / §7.1 / NOTES 17)

- `daily_target_minutes(2400) == 480` (the §7.1 example's `08h 00m`).
- Non-round, flooring case: `daily_target_minutes(2002) == 400`
  (2002/5 = 400.4 → 400), and `daily_target_minutes(2004) == 400`
  (400.8 → 400, i.e. it floors, never rounds to nearest).
- `daily_target_minutes(0) == 0` (a zero-target week, F7b).
- `daily_target_minutes(1) == 0` (floors below one minute).
- Assert it is a pure function of the target only — no carry, no
  fulfillment, no week id in the signature at all (§2.4: "purely derived
  from the week's target … no interaction with carry"). This is a
  signature-level assertion, i.e. it is enforced by the API shape in §4
  rather than by a runtime check; call it out in the test module's doc
  comment so a later refactor doesn't quietly widen the signature.

### 7.9 `WeekId::next()` across year boundaries

Not in PLAN.md's acceptance criteria, but the walk is wrong without it.

- 2026 is a **53-week** ISO year: `next(WeekId{2026,52}) == WeekId{2026,53}`
  and `next(WeekId{2026,53}) == WeekId{2027,1}`.
- 2025 is a **52-week** ISO year: `next(WeekId{2025,52}) == WeekId{2026,1}`
  (not `2025-53`).
- The implementer must verify these two year lengths against `chrono`
  (e.g. the ISO week of Dec 31) rather than trusting this document —
  assert the year lengths in the test itself so a wrong assumption fails
  loudly instead of hiding.
- Walk-level consequence: a `week_series` whose range spans a 53-week
  year boundary (data in `2026-52`, request `2027-02`) has the expected
  length (4: 52, 53, 2027-01, 2027-02) and the carry chain is unbroken.

### 7.10 No clamping anywhere

- Large positive carry: a week with 5000 worked against a 2400 target,
  followed by a week with 0 worked → second week's `carry_in == 2600`,
  `fulfillment == 2600`, `owed == -200`. Surplus survives into a
  zero-worked week and is not zeroed, capped, or "expired".
  (Note for reviewers: NOTES.md's *current manual process* section
  describes a 40h cap where only shortfall spills forward. Decision 3 and
  §1.1 explicitly replaced that with signed two-way carry. This test
  pins the new behavior; the old cap must not be reintroduced.)
- Long deficit chain: 10 consecutive empty weeks after one data week →
  `carry_in == -24000`, monotonically decreasing, no floor at zero.

---

## 8. Ambiguities, risks, and disagreements

### 8.1 CONTRACT POINT — the shape of the worked-minutes input (highest risk)

Milestone 4 is being designed in a parallel worktree and **may not
produce the shape assumed above**. What this milestone assumes:

- Worked minutes arrive **already aggregated per ISO week**, as signed
  integer minutes, completed stints only.
- The source can answer **"which is the earliest week with any data"** —
  including data that produces zero worked minutes (a note-only week, a
  week with only an unmatched `start`).
- Weeks are addressed by `(iso_year, iso_week)`, not by a date range.

Plausible mismatches to check with whoever owns Milestone 4 **before
wave-3 integration**:

- Milestone 4 may naturally produce **per-*date* totals** (it stores a
  local `date` column; `GROUP BY date` is the obvious query, and
  Milestone 11's contract 4 needs per-date rows anyway). If so, the
  date→week bucketing has to live *somewhere*. **Recommendation**: it
  lives in the adapter that builds a `WeekLedger`, not inside this
  milestone — `WeekLedger` gains a `from_daily_totals(impl
  IntoIterator<Item = (NaiveDate, i64)>)` constructor at integration
  time. That keeps this milestone's fold untouched by the change and is
  why the traits are narrow.
- **Earliest-data-week may be awkward to answer efficiently.** It is a
  `SELECT min(date)` over `punches` UNION `notes` — cheap, but it is a
  *second* query, and a Milestone 4 that only exposes "give me punches
  for date X" cannot answer it at all. Flag this explicitly: this
  milestone **cannot** function with a date-at-a-time reader. It needs
  either a full-history load or an explicit earliest-date query.
- **Does a week with only a `week_targets` override, and no punches or
  notes, count as a "data week" for starting the walk?** §2.4 says
  "earliest week with any punch/note data" — overrides are not mentioned.
  **Assumption taken here: no** — overrides alone do not start the chain.
  Practically harmless (an override on a week before any tracking would
  just be ignored until data exists), but it is a real reading choice.
  Flagged for confirmation; if it flips, it is a one-line change in the
  adapter that builds `data_weeks`, not in the fold.
- **Live-open-stint exclusion is the source's job, not this milestone's.**
  If Milestone 4/5's aggregate helpfully includes open-stint live minutes,
  every carry number silently becomes wrong and no test here would catch
  it. Whoever wires wave 3 must assert NOTES 37/38 at the seam — add an
  integration test there, not here.

### 8.2 `owed` and `carry_out` are algebraically redundant

`carry_out == -owed`, always. Contract 3 names both, and §7.2's rendered
output shows carry-in while the headline shows owed, so both stay. Noting
it so a reviewer doesn't file it as a bug, and so nobody "simplifies" one
into the other — computing each from its own §5 formula is the cheap
insurance.

### 8.3 `is_current_week` and "now"

Contract 3 says the accounting result carries this boolean; contract 6
says "now" is always injected. Together they force `today` into this
milestone's signature even though *nothing else* in the accounting math
needs it. This is a mild wart (a pure arithmetic function taking a date
purely to set a flag) but it is exactly what PLAN.md asks for and it does
save Milestone 9 from recomputing the comparison. Accepted as specified.

One subtlety for Milestone 9's author: `is_current_week` is about the
*week*, not about the date being displayed. §7.1/§3.5 require that when
`status DATE` targets a non-today date that still falls inside the
current week (F11), the deadline framing keyed to **today's** weekday is
used. `is_current_week` as defined here (week containing `today`) is the
right input for that; the *weekday* in the phrase comes from `today`
separately, in Milestone 9. This milestone deliberately exposes no
weekday.

### 8.4 Performance

§2.4 accepts an O(n-weeks) walk. Worth naming the pathological case
anyway: a user who tracks one week in 2020 and then asks for a week in
2030 walks ~520 iterations of pure integer arithmetic — microseconds.
No memoization, no caching, no early exit. If `week_series` ever shows up
in a profile, the fix is a materialized carry table, which §2.4
explicitly declines for MVP. Do not pre-optimize.

### 8.5 Disagreements with PLAN.md — none substantive

Two small additions beyond PLAN.md's stated Milestone 6 acceptance
criteria, both defensible:

- `WeekId::next()`'s 53-week-year handling (§7.9) is not listed in
  PLAN.md's criteria but is a genuine correctness prerequisite for the
  walk. If Milestone 2 ships `next()`, this milestone drops its copy and
  keeps the test.
- `week_series` (the whole chain) is exposed, where PLAN.md's scope
  literally describes only computing "that week's" figures. Exposing the
  series costs nothing, makes F8b directly assertable rather than
  inferred, and is the natural implementation anyway.

Neither changes any spec-defined behavior.

### 8.6 Note for the reviewer

The one thing most worth adversarially probing in the resulting code:
**that the walk has no empty-week shortcut**. The tempting optimization
("iterate the data weeks we have rather than every week in between") is
precisely the bug §2.4 was amended to prevent. Read the loop body for any
`continue`, any iteration over `worked.keys()`/`data_weeks`, or any
`filter` — the loop must iterate weeks, not entries.
