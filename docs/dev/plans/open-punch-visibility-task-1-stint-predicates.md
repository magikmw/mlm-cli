# Task 1 plan: Anomaly predicates (`src/stint.rs`)

Changeset: open-punch-visibility. Parent plan:
`docs/dev/plans/open-punch-visibility-plan.md`. Spec:
`docs/dev/specs/2026-09-26-open-punch-visibility.md` (locked).

## What was read

- `docs/dev/plans/open-punch-visibility-plan.md` (full) — goal,
  architecture, global constraints, Contract A, Contract B, ordering,
  and this task's own section ("Task 1 — Anomaly predicates").
- `docs/dev/specs/2026-09-26-open-punch-visibility.md` (full) — §1
  summary table, §1.1 non-goals, §1.2 residual gaps, §2 terms (age,
  stale open start, lookback window, refused join), §3.1-§3.7
  behavior, §4 docs list, §5 test plan.
- `docs/dev/plans/open-punch-visibility-task-*.md` — none exist yet.
  This changeset has only the top-level plan on disk so far; Tasks
  2-6's plans are being written in parallel by other agents and are
  not available to cross-check. Contract A and Contract B (quoted
  above from the parent plan) are the only agreed interface text this
  plan can rely on.
- `src/stint.rs` (full, all 1135 lines) — `Stint`, `OpenStint`,
  `OrphanedEnd`, `Anomaly`, `DayStints` (incl. `compute_has_anomaly`,
  `has_anomaly`, `anomalies`, `completed_minutes`, `is_ongoing`),
  `classify`, `close_group`, `classify_at`, `splice_candidate`, and
  the full test module.
- `src/status.rs` lines 1-45 and 425-444 — `OpenStintAge`'s three-way
  split and the exact existing staleness/age computation:
  `target_date == today - ChronoDuration::days(1)` for yesterday,
  else (when not `today`) `TwoOrMoreDaysBack`. This is the age-2
  boundary this task's staleness predicate must reuse the reasoning
  of (per the parent plan's global constraints).
- `src/storage.rs` lines 54-110 — `PunchKind` (`Start`/`End`) and
  `Punch` (`id: i64`, `at_utc: DateTime<Utc>`, `date: NaiveDate`,
  `kind: PunchKind`) field shapes, confirmed as already `pub`.
- `clippy.toml` — `cognitive-complexity-threshold = 15`.
- `.githooks/pre-commit` — confirms the three gates (fmt, cognitive
  complexity via clippy, coverage-vs-baseline via `cargo llvm-cov`)
  and that all three degrade to SKIP rather than fail when tooling is
  missing; no behavior this plan needs to route around, since the two
  new functions are each a handful of lines.
- `render.rs` — skimmed for the existing `Anomalies` struct location
  (lines ~123-135) only to confirm this task's predicates hand back
  plain data, never a `render` type. Not read in full: it is Task 2's
  file and out of scope here.

## Scope

**This task owns, exclusively**: `src/stint.rs`.

**Must not touch**:

- `src/render.rs` — Task 2. This task never imports it or constructs
  any of its types; the two new predicates return plain `stint.rs`
  types (`bool`, `i64`, `NaiveDate`, or a small new struct/enum
  defined in `stint.rs` itself).
- `src/status.rs` — Task 3. Consumes this task's predicates; not
  touched here.
- `src/week_view.rs` — Task 5. Also consumes this task's predicates;
  not touched here.
- `src/commands.rs` — Task 4.
- `docs/dev/SPEC.md`, `README.md` — Task 6.
- `src/storage.rs`, `src/main.rs` — the parent plan states neither
  needs a task for this changeset.

No change to `classify`, `classify_at`, or `splice_candidate` — their
logic, signatures, and every existing test's expected value are
frozen (parent plan's global constraints; also this task's own "out
of scope" list). The two new predicates are added beside them, in the
same file, as new pure functions.

## Published interface

Both functions are pure, take no clock and no database handle (every
date and "today" is caller-supplied), and live in `src/stint.rs`
beside `splice_candidate`. Visibility is `pub(crate)`, matching
`splice_candidate`'s own visibility — Tasks 3 and 5 are both
in-crate callers (`status.rs`, `week_view.rs`), and nothing outside
the crate needs either predicate.

There is no separate bool-only "is stale" function: every real caller
(Task 3's per-date fact builder, Task 5's per-row marker) needs the
time/age fields anyway to build the fact structures Contract A
describes, so a caller that only wants the yes/no answer writes
`stint::stale_open_start(&day, date, today).is_some()` inline.

There is also no separate "which side is which" function for the
refused join. Contract B's own wording — "It is true when [conditions]"
— describes a plain `bool`. The "names the other specifically"
requirement in the parent plan's Task 1 acceptance criteria is
satisfiable by the caller for free: Task 3/Task 5 already hold both
dates (they are the ones walking the calendar to obtain each
`DayStints`) and call `is_refused_join(earlier, later, ...)` with a
fixed, known argument order, so nothing about "which side" needs to
come back out of `stint.rs`. See "Refused-join direction" under
Risks/ambiguities for the one way a sibling author could read this
differently.

**Published interface** (what Task 1 adds to `src/stint.rs`,
`pub(crate)`, immediately below `splice_candidate`):

```rust
/// The stale open start's own instant and age, when `day`'s classification
/// carries exactly one open start whose age (`today - date`, in whole
/// calendar days) is >= 2 -- spec §2/§3.1. `None` for zero or >= 2 open
/// starts (>= 2 is E7's own anomaly, already covered by
/// `Anomaly::MultipleOpenStints`) and `None` at age 0 or 1. `age_days`
/// keeps growing past 14 when `Some` -- the 14-day lookback-window
/// cutoff is Task 3's concern, not this predicate's (spec §3.1's own
/// wording: the rule doesn't stop being true past the window).
pub(crate) struct StaleOpenStart {
    pub at: DateTime<Utc>,
    pub age_days: i64,
}

pub(crate) fn stale_open_start(
    day: &DayStints,
    date: NaiveDate,
    today: NaiveDate,
) -> Option<StaleOpenStart>

/// True when `earlier` (classified for some date `A`) and `later`
/// (classified for `A`'s very next calendar date, `A+1`) form a
/// *refused join* (spec §2/§3.2): `earlier` has >= 1 open start,
/// `later` has >= 1 orphaned end, and `splice_candidate` rejects the
/// pair. Both `DayStints` must be raw single-date classifications
/// (plain `classify()` output), never `classify_at`-resolved -- see
/// the "Refused-join inputs" note below.
pub(crate) fn is_refused_join(
    earlier: &DayStints,
    later: &DayStints,
    later_punches: &[Punch],
) -> bool
```

Both functions require no new imports beyond what `stint.rs` already
has (`chrono::{DateTime, Utc}` is imported; `NaiveDate` needs adding
to that `use` line since today's file only imports `DateTime, Utc`
from `chrono` at line 12 — `chrono::NaiveDate` becomes a new import).
`Punch` is already imported (line 14).

## Dependencies

| Dependency | Status | Stub to test against today | Import to swap at integration |
|---|---|---|---|
| `splice_candidate(earlier_open_count, later, later_punches) -> bool` | Shipped (`src/stint.rs:334`, `pub(crate)`) | N/A — call directly | N/A, already in this file |
| Contract A's per-date fact shape (Task 2, `render.rs`) | Not landed | This task's own unit tests construct `StaleOpenStart`/`bool` values directly and assert on them; no dependency on Task 2's shape to test Task 1 in isolation | Not applicable to this task — Task 3/5 do the wiring into Task 2's shape, not Task 1 |
| Task 3's lookback walker (`status.rs`) | Not landed | N/A — Task 1 is a pure-predicate leaf, called BY Task 3, not calling it | N/A |

Task 1 has no real dependency on any unlanded sibling: it only calls
the already-shipped `splice_candidate`, and its own tests are fully
self-contained (construct `DayStints` via `classify()`, feed the two
new functions, assert). Contract A/B exist to let Tasks 2/3/5 build
against Task 1's signatures above without waiting for this file to
merge, not the reverse.

## Design

### `stale_open_start`

Mirrors `status.rs`'s existing age split (line 437-443) but expressed
as a pure day-count rather than a three-way enum, since spec §2
defines "age" as a plain integer (`today - D` in whole calendar
days), and the stale threshold is `>= 2` — a single numeric
comparison, not a three-way match.

```rust
pub(crate) fn stale_open_start(
    day: &DayStints,
    date: NaiveDate,
    today: NaiveDate,
) -> Option<StaleOpenStart> {
    let [only_open] = day.open.as_slice() else {
        return None;
    };
    let age_days = (today - date).num_days();
    if age_days < 2 {
        return None;
    }
    Some(StaleOpenStart {
        at: only_open.start.at_utc,
        age_days,
    })
}
```

`today - date` on two `NaiveDate`s produces a `chrono::TimeDelta`
(`Duration`) directly (`NaiveDate` implements `Sub<NaiveDate>`), so
`.num_days()` needs no extra conversion — same mechanism
`status.rs` already uses for `today - ChronoDuration::days(1)`,
just subtracting the other direction. `[only_open] = day.open.as_slice()
else { ... }` is a slice pattern that only matches a
length-exactly-1 slice; zero or >= 2 elements fall to the `else`
arm and return `None` before the age check runs, which directly
encodes "never fires for two or more opens" (acceptance criterion 2)
without a separate `.len() == 1` check.

Note: negative `age_days` (a `date` after `today`, e.g. a clock skew
or a caller bug) also falls under `age_days < 2` and returns `None` —
correct by construction, no separate guard needed, and not a
reachable case in practice since every caller derives `date` by
walking backward from `today` (Task 3) or from an already-stored
punch's date (Task 5), never forward.

### `is_refused_join`

```rust
pub(crate) fn is_refused_join(
    earlier: &DayStints,
    later: &DayStints,
    later_punches: &[Punch],
) -> bool {
    !earlier.open.is_empty()
        && !later.orphaned_ends.is_empty()
        && !splice_candidate(earlier.open.len(), later, later_punches)
}
```

This is a direct, literal translation of spec §2's definition and
Contract B's restated version: "true when the earlier date has at
least one open start, the later date has at least one orphaned end,
and the existing splice gate rejects the pair." `splice_candidate`
itself already returns `false` whenever `earlier_open_count != 1` or
`later.orphaned_ends.len() != 1`, so the leading `!earlier.open.is_empty()`
/`!later.orphaned_ends.is_empty()` guards are not strictly needed for
correctness (if either is empty, `splice_candidate` already answers
`false`, making the whole expression `true` — wrong!). They are
therefore **required**, not defensive redundancy: without them, a
date with zero open starts and a neighbor with zero orphaned ends
would read as "refused join" (`splice_candidate` returns `false` on
empty inputs, and `!false` is `true`). The explicit emptiness checks
are what makes "at least one" the real precondition instead of
accidentally falling out of `splice_candidate`'s own multi-reason
`false`.

Both functions are pure, take no clock, and read no database —
`today`/dates arrive as parameters (per Contract B and the parent
plan's global constraints).

## Cognitive complexity

Both functions are single early-return-then-boolean-expression
bodies, well under the threshold of 15 — no split needed. This task
does not touch `classify`/`classify_at`, which already sit under the
threshold today (unchanged).

## Test plan

Tests live in `src/stint.rs`'s existing `#[cfg(test)] mod tests`,
alongside the existing `splice_candidate` tests, using the module's
existing `p()`/`p_on()`/`at()`/`at_on()`/`date()` helpers. No new test
file — this module has never split its tests out.

### AC1 — stale boundary ages 0/1/2/14/15 (spec §5 "Stale boundary")

```rust
#[test]
fn stale_open_start_not_stale_at_age_zero_or_one() {
    let d = classify(&[p(1, "09:00", Start)], at("10:00"));
    assert!(stale_open_start(&d, date(), date()).is_none()); // age 0
    assert!(stale_open_start(&d, date(), date().succ_opt().unwrap()).is_none()); // age 1
}

#[test]
fn stale_open_start_fires_from_age_two_through_arbitrarily_old() {
    let d = classify(&[p(1, "09:00", Start)], at("10:00"));
    for age in [2, 14, 15, 100] {
        let today = date() + ChronoDuration::days(age);
        let s = stale_open_start(&d, date(), today).expect("stale at age {age}");
        assert_eq!(s.age_days, age);
        assert_eq!(s.at, at("09:00"));
    }
}
```

Covers "age 0 or 1 is not stale; age 2 through 14 is stale; age 15+
is still stale by the same rule" verbatim (plan AC1, spec §5 first
bullet).

### AC2 — two or more opens never reported stale (spec §5 second bullet)

```rust
#[test]
fn stale_open_start_none_for_two_or_more_opens_even_when_old() {
    let d = classify(&[p(1, "09:00", Start), p(2, "11:00", Start)], at("12:00"));
    let today = date() + ChronoDuration::days(5);
    assert!(stale_open_start(&d, date(), today).is_none());
}
```

### AC3/AC4 — refused join fires and names each side distinctly

`is_refused_join` itself only returns `bool` — naming "which side is
which" (plan AC4) is not this function's job by the Published
Interface above (see "Refused-join direction" in Risks): the caller
(Task 3/5) already knows which `DayStints` is earlier and which is
later, since it is the one walking the calendar to obtain them. This
task tests the boolean at every enumerated shape and both orderings
of the call are exercised so a caller relying on argument order (not
a return value) is proven correct:

```rust
#[test]
fn refused_join_fires_for_two_opens_on_earlier_date() {
    let day1 = date();
    let day2 = day1.succ_opt().unwrap();
    let earlier = classify(&[p_on(1, day1, "09:00", Start), p_on(2, day1, "11:00", Start)], at_on(day2, "01:00"));
    let later_punches = vec![p_on(3, day2, "00:45", End)];
    let later = classify(&later_punches, at_on(day2, "01:00"));
    assert!(is_refused_join(&earlier, &later, &later_punches));
}

#[test]
fn refused_join_fires_when_orphan_not_first_on_later_date() {
    let day1 = date();
    let day2 = day1.succ_opt().unwrap();
    let earlier = classify(&[p_on(1, day1, "23:00", Start)], at_on(day2, "14:00"));
    let later_punches = vec![
        p_on(2, day2, "09:00", Start),
        p_on(3, day2, "10:00", End),
        p_on(4, day2, "13:00", End),
    ];
    let later = classify(&later_punches, at_on(day2, "14:00"));
    assert!(is_refused_join(&earlier, &later, &later_punches));
}

#[test]
fn refused_join_fires_for_two_orphans_on_later_date() {
    let day1 = date();
    let day2 = day1.succ_opt().unwrap();
    let earlier = classify(&[p_on(1, day1, "23:00", Start)], at_on(day2, "02:00"));
    let later_punches = vec![p_on(2, day2, "00:45", End), p_on(3, day2, "01:00", End)];
    let later = classify(&later_punches, at_on(day2, "02:00"));
    assert!(is_refused_join(&earlier, &later, &later_punches));
}

#[test]
fn refused_join_at_yesterday_today_boundary() {
    // Same shapes as above, just confirming the predicate carries no
    // age gate itself (spec §3.2: "at every age, including
    // yesterday-today").
    let yesterday = date();
    let today = yesterday.succ_opt().unwrap();
    let earlier = classify(&[p_on(1, yesterday, "23:00", Start), p_on(2, yesterday, "23:30", Start)], at_on(today, "01:00"));
    let later_punches = vec![p_on(3, today, "00:15", End)];
    let later = classify(&later_punches, at_on(today, "01:00"));
    assert!(is_refused_join(&earlier, &later, &later_punches));
}
```

### AC5 — clean splice produces no refused join

```rust
#[test]
fn clean_splice_shape_is_not_a_refused_join() {
    let day1 = date();
    let day2 = day1.succ_opt().unwrap();
    let earlier = classify(&[p_on(1, day1, "23:30", Start)], at_on(day2, "01:00"));
    let later_punches = vec![p_on(2, day2, "00:45", End)];
    let later = classify(&later_punches, at_on(day2, "01:00"));
    assert!(!is_refused_join(&earlier, &later, &later_punches));
}

#[test]
fn no_open_and_no_orphan_is_not_a_refused_join() {
    // Regression guard for the "guards are load-bearing" note in
    // Design: without the explicit emptiness checks this would
    // wrongly read true, since splice_candidate(0, ..) is false too.
    let earlier = classify(&[], at("10:00"));
    let day2 = date().succ_opt().unwrap();
    let later_punches: Vec<Punch> = vec![];
    let later = classify(&later_punches, at_on(day2, "10:00"));
    assert!(!is_refused_join(&earlier, &later, &later_punches));
}
```

### AC6 — both predicates unit-tested independent of `status.rs`/`week_view.rs`

Satisfied structurally: every test above lives in `stint.rs`'s own
test module and constructs its inputs via `classify()` plus the
module's existing helpers, with no dependency on `status`/`week_view`
compiling or existing.

## Verifying this task

```sh
cargo fmt
cargo build
cargo test stint::
cargo clippy --all-targets -- -D warnings
```

`cargo test stint::` scopes to this module so the task is checkable
before Tasks 3/5 exist to call the new functions (both are currently
unused outside tests, so `#![allow(dead_code)]` at the top of
`stint.rs`, already present at line 10, keeps clippy/rustc quiet about
that until Task 3/5 land — no change needed there).

## Risks, ambiguities, and disagreement

- **No sibling task plans exist yet.** Tasks 2-6's plan files are not
  on disk (only the top-level changeset plan). This plan relies
  entirely on Contract A and Contract B's text, quoted verbatim from
  the parent plan, for the shape Tasks 3/5 will expect. If a sibling
  planner reads Contract B differently, notably around whether the
  refused-join predicate should itself report *which* side is which
  (see next point), that is a real risk of rework here since there is
  no live sibling doc to diff against yet.
- **Refused-join direction.** Contract B says: "the refused-join
  predicate takes the two dates' raw single-date classifications...
  It is true when [conditions]." That sentence describes a `bool`
  return, not a "which side" return. Separately, plan AC4 says "The
  refused-join predicate reports which date is 'the one with the open
  start' and which is 'the one with the orphaned end' distinctly,
  since spec §3.4 requires each side's line to name the other
  specifically." I read these together as: the **predicate** is a
  bool (Contract B is explicit about this), and the "which is which"
  requirement is satisfiable by the caller for free, since the caller
  (Task 3/5) is the one holding both dates and calling
  `is_refused_join(earlier, later, ...)` with a fixed, known
  argument order — nothing about "which side" needs to come back out
  of `stint.rs` when the caller put it in. I have **not** added a
  second "named-sides" return type for this reason, but flag this as
  a disagreement-risk: if Task 3 or Task 5's author reads AC4 as
  requiring `stint.rs` itself to return an enum like `RefusedJoin {
  EarlierHasOpen, LaterHasOrphan }` (redundant with argument order but
  perhaps wanted for defensiveness or symmetry with `Anomaly`'s own
  enum style), that is a one-function addition compatible with
  everything else in this plan and can be bolted on without touching
  `is_refused_join`'s existing signature. I judge the plain `bool` to
  be the correct, minimal reading of Contract B and recommend sibling
  authors call `is_refused_join(earlier, later, later_punches)`
  keeping their own already-known `(earlier_date, later_date)` pair on
  the calling side, but note this explicitly since it is the one place
  this plan makes a judgment call Contract B doesn't fully pin down.
- **`StaleOpenStart` as a struct vs. a tuple.** The parent plan and
  spec don't pin an exact Rust shape, only "whether its one open start
  is stale and if so its time and age in days" (Contract A, describing
  Task 2's input facts, not this task's return type directly). I chose
  a small named struct (`at`, `age_days`) over a tuple for the same
  reason every other multi-field return in this module already uses a
  named struct (`Stint`, `OpenStint`, `OrphanedEnd`) — consistent with
  existing style, and self-documenting at every call site without a
  comment. If Task 3/5's authors independently assumed a raw tuple or
  different field names, that is a one-line adaptation on their end,
  not a logic change.
- **`age_days` type is `i64`**, matching `OpenStint::minutes_so_far`'s
  existing `i64` and `chrono::TimeDelta::num_days()`'s own return
  type — not `u32`/`usize`, even though it is never negative once
  `Some`. Keeps the module's existing convention of `i64` for every
  duration-shaped field (`Stint::minutes`, `OpenStint::minutes_so_far`)
  and avoids a cast at the one call site each in Task 3/Task 5.
- **Coverage gate**: every new branch in both functions (0 opens, 1
  open, 2+ opens; age < 2, age >= 2; each of the three refused-join
  trigger shapes; the clean-splice negative; the both-empty negative)
  has a dedicated test above, so the pre-commit coverage ratchet
  should not regress on this file. Not independently verified by a
  local `cargo llvm-cov` run in the course of writing this plan, since
  no code has been written or committed yet (plan-only deliverable, as
  instructed).
- I have no disagreement with the spec or parent plan's substance —
  the two predicates as specified are a small, direct extension of
  `splice_candidate`'s own existing shape, and the "raw classification,
  not `classify_at`-resolved" input constraint in Contract B is, on
  inspection of `classify_at`'s own body (lines 293-321), clearly
  necessary: `classify_at` mutates `day.orphaned_ends`/`day.open` in
  place when a splice fires, so evaluating `is_refused_join` against
  its output would indeed silently hide genuine refusals the way
  Contract B warns against.
