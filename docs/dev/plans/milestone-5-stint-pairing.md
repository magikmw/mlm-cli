# Milestone 5 — Stint pairing (implementation plan)

**Spec basis**: SPEC.md §4.3 (nearest-match/LIFO pairing + named edge
cases), §1.3 (stint / open stint definitions), §2.4 (orphaned `end`
contributes nothing to totals; open-stint live minutes never enter
totals), §6.2 (anomalies are surfaced, never rejected), §8 flows F1,
F3, F9 (open-stint half), E7, E8, E11, E14.

**PLAN.md basis**: wave-1, fixture-driven. Interface contracts 1
(punch value shape), 2 (stint classification result shape), 6 ("now"
injection), 8 (`Punch`/`PunchKind` ownership — resolved to Milestone
4, see §1/§7.2 below). This milestone's *logic* must not depend on
Milestone 4's behavior (fixtures stand in for real reads), but it does
have a **compile-time** dependency on `storage.rs`'s `Punch`/
`PunchKind` type definitions existing (even as an early, functionally
empty stub) — genuine wave-1 concurrency means both worktrees agree on
those two type declarations before either starts, not that Milestone
5 can build in total isolation from Milestone 4's crate module.

**Deliverable**: a new module `src/stint.rs` (declared `mod stint;` in
`src/main.rs`), containing the algorithm in §3 and the unit tests in
§6 (the punch/result types themselves now live in `src/storage.rs`
and `src/render.rs`'s consumers respectively, per §1/contract 8 and
contract 2). No rendering, no DB, no clock reads, no CLI. Nothing
outside `src/stint.rs` + the one `mod` line in `src/main.rs` changes.

---

## 1. The punch value shape (interface contract 1) — resolved: import, don't redeclare

**Cross-plan fix**: this milestone originally declared its own
`Punch`/`PunchKind` here as a *fixture contract* for Milestone 4 to be
checked against. Cross-plan review found Milestone 4 independently
designed the field-for-field identical shape (including the same
kind-then-id tiebreak ordering) in `src/storage.rs` — so per PLAN.md
contract 8, **Milestone 4 is the sole owner**. This milestone imports
`Punch`/`PunchKind` from `storage.rs` (`use crate::storage::{Punch,
PunchKind};`) rather than declaring its own copy.

The shape (now defined in Milestone 4's plan, reproduced here only for
reference since this milestone's algorithm depends on it directly):
`Punch { id: i64, at_utc: DateTime<Utc>, date: NaiveDate, kind:
PunchKind }`, `Punch: Copy`, `PunchKind: Copy + Ord` with `Start`
ordered before `End`. Every design point below this milestone
originally argued for (full row not a lighter value, `Copy` for
stack-friendly pairing, `date` carried rather than re-derived,
`PunchKind` as a validated enum not a raw string) held up unchanged —
only the ownership/location moved.

**Fixture helper** (test-only, in `#[cfg(test)]`), so every test case in
§6 is one readable line:

```rust
/// `p(1, "09:00", Start)` — builds a Punch on a fixed test date
/// (`TEST_DATE` = 2026-02-12) at that local wall-clock HH:MM,
/// converted to UTC under a fixed test offset (UTC+00:00 for all
/// fixtures — no DST interaction is in this milestone's scope).
fn p(id: i64, hhmm: &str, kind: PunchKind) -> Punch;
```

Because every fixture uses a fixed UTC offset of zero, the local wall
clock in a test name equals the UTC instant — so `09:00` in a test
reads as `09:00` in the assertion. DST/offset correctness is Milestone
4's and Milestone 10's problem (PLAN.md cross-cutting concerns), not
this one's.

---

## 2. The classification result shape (interface contract 2)

```rust
/// A completed (start, end) pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stint {
    pub start: Punch,
    pub end: Punch,
    /// Whole minutes from `start.at_utc` to `end.at_utc`.
    /// Always >= 0 (guaranteed by the sort, §3.1). 0 for E14.
    pub minutes: i64,
}

/// A trailing unmatched `start`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenStint {
    pub start: Punch,
    /// Whole minutes from `start.at_utc` to the supplied `now`,
    /// clamped at 0 (§3.4). Live figure — SPEC.md §2.4 forbids this
    /// value from entering ANY day/week/carry total.
    pub minutes_so_far: i64,
}

/// An `end` that had no unmatched `start` to pair with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrphanedEnd {
    /// The offending punch itself — its `at_utc` is what §7.3's
    /// "orphaned end at HH:MM" line renders. Never coalesced with
    /// another orphan (E8): one value per orphaned punch.
    pub punch: Punch,
}

/// A renderable anomaly, in the two shapes §7.3 defines.
/// Milestone 5 produces the DATA; Milestone 9 produces the TEXT.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anomaly {
    /// More than one trailing unmatched start (§4.3, E7).
    /// `count` == `DayStints::open.len()`, always >= 2 when present.
    MultipleOpenStints { count: usize },
    /// One per orphaned end (§4.3, E8) — emitted once per orphan,
    /// in ascending instant order, never merged.
    OrphanedEnd { punch: Punch },
}

/// Everything Milestone 5 produces for ONE calendar date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayStints {
    /// Completed stints, ascending by `end` instant (emission order,
    /// §3.2). The ONLY source of `worked_minutes` (§2.4, NOTES 37/38).
    pub completed: Vec<Stint>,
    /// Trailing unmatched starts, ascending by start instant.
    /// len 0 = nothing open; len 1 = the normal open stint (§1.3, F1,
    /// NOT an anomaly); len >= 2 = the E7 multi-open anomaly.
    pub open: Vec<OpenStint>,
    /// One entry per orphaned end, ascending by instant (E8).
    pub orphaned_ends: Vec<OrphanedEnd>,
    /// PRECOMPUTED at construction. `open.len() > 1 || !orphaned_ends.is_empty()`.
    /// Private field + `has_anomaly()` accessor so there is exactly one
    /// definition of "has an anomaly" and Milestone 11 pays O(1) per row.
    has_anomaly: bool,
}

impl DayStints {
    /// Cheap O(1) signal for Milestone 11's per-row `[!]` marker
    /// (contract 2's "has-any-anomaly" half). Reads a stored bool.
    pub fn has_anomaly(&self) -> bool { self.has_anomaly }

    /// Renderable-detail form for Milestone 10's status output
    /// (contract 2's other half). Allocates; called once per rendered
    /// status page, never per week row.
    /// Emission order is FIXED: `MultipleOpenStints` first (when
    /// `open.len() > 1`), then one `OrphanedEnd` per orphan in
    /// ascending-instant order — matching §7.3's example block.
    pub fn anomalies(&self) -> Vec<Anomaly> { /* ... */ }

    /// Sum of `completed[..].minutes`. The day total for §7.1's
    /// "Day total" line and Milestone 6's per-week worked minutes.
    /// Excludes open-stint live time and orphans by construction
    /// (NOTES.md decisions 37/38).
    pub fn completed_minutes(&self) -> i64 { /* ... */ }

    /// True when at least one stint is open — drives §7.1's
    /// `(+ ongoing)`, the est.-EOD gate (F9), and §7.2's `(ongoing)`
    /// row marker. `!open.is_empty()`, NOT `open.len() == 1`.
    pub fn is_ongoing(&self) -> bool { !self.open.is_empty() }
}
```

Why `has_anomaly` is a stored field rather than a method body computing
`open.len() > 1 || !orphaned_ends.is_empty()`: contract 2 demands the
cheap boolean not be "derived expensively from" the detail form. The
expensive derivation the contract is guarding against is
`!self.anomalies().is_empty()` (which allocates a `Vec` per week row —
7 allocations per `mlm week` invocation, and a second definition of
"anomalous" that can drift). Storing the bool makes the constructor the
single place the rule is written, and `anomalies()` is then guaranteed
non-empty exactly when `has_anomaly()` is true — an invariant §6's
T12 asserts directly.

**Deliberate non-goals of this shape** (belong to other milestones):
no formatted strings anywhere (Milestone 9 §7.3), no `HHh MMm`
rendering (Milestone 1's formatter, fed `minutes`), no week rollup
(contract 4 / Milestone 11), no notes (Milestone 4).

---

## 3. The algorithm

Entry point:

```rust
/// Pair one calendar date's punches into stints (SPEC.md §4.3).
/// `punches` — all punches for a single date, in any order.
/// `now` — the injected current instant (contract 6, §4 below).
pub fn classify(punches: &[Punch], now: chrono::DateTime<chrono::Utc>) -> DayStints
```

### 3.0 Preconditions

- All `punches` share one `date`. Enforced by a
  `debug_assert!(punches.iter().all(|q| q.date == punches[0].date))`
  when non-empty — a debug assert, not a runtime error, because the
  caller (Milestone 10/11) queries by date and a violation is a caller
  bug, not user data. No anomaly type exists for it, and §6 does not
  test it.
- Input order is irrelevant: `classify` sorts internally (§3.1) and
  does not trust Milestone 4's read order. This is intentional
  redundancy — PLAN.md contract 1 lets Milestone 4 either sort or
  document that the caller must, so Milestone 5 owns the sort
  unconditionally per §4.3 step 1.
- `punches` is borrowed, not consumed; the sort happens on an internal
  `Vec<Punch>` copy (cheap — `Punch: Copy`).

### 3.1 Step 1 — the sort

Sort the copied slice by the composite key, ascending:

```
(at_utc, kind_rank, id)      where kind_rank: Start => 0, End => 1
```

- **`at_utc` first** — §4.3 step 1's primary key. Note entry order is
  irrelevant here, which is exactly what makes F3 work.
- **`kind_rank` second — THIS IS A DELIBERATE ADDITION TO §4.3's
  LETTER; see §7.1 for the flag.** At an identical instant, `Start`
  sorts before `End`. Without it, a user who types `mlm stop 09:00`
  and then `mlm start 09:00` gets id order `end(id=1), start(id=2)`,
  which the scan would classify as an orphaned end plus an open stint
  — directly contradicting SPEC.md §4.3's own E14 bullet, which
  asserts such a pair "produces a zero-length stint... not itself an
  anomaly" and that "both punches pair up cleanly". Ranking kind ahead
  of id makes E14 hold for BOTH entry orders. It changes nothing for
  any pair of punches at distinct instants, and nothing for two
  punches of the same kind at the same instant.
- **`id` third** — §4.3's stated tiebreaker (insertion order). It is
  now only load-bearing between same-instant, same-kind punches, where
  it still yields a fully deterministic total order.

Use a **stable** sort (`sort_by_key` / `sort_by`) so that even if two
rows somehow shared an id, output stays deterministic.

### 3.2 Step 2 — the LIFO scan

```
stack: Vec<Punch> = []          // unmatched starts, most recent last
completed: Vec<Stint> = []
orphaned: Vec<OrphanedEnd> = []

for punch in sorted:
    match punch.kind:
        Start => stack.push(punch)
        End   => match stack.pop():                 // pop = NEAREST unmatched start
                     Some(start) => completed.push(Stint {
                         start,
                         end: punch,
                         minutes: (punch.at_utc - start.at_utc).num_minutes(),
                     })
                     None => orphaned.push(OrphanedEnd { punch })
```

After the loop, `stack` holds every trailing unmatched start, in
ascending instant order (push order == sort order). Map each to an
`OpenStint` with `minutes_so_far` per §3.4. `completed` is in ascending
**end**-instant order (emission order); leave it that way — it is the
order §7.1's stint list wants for the common non-nested case, and
Milestone 10 may re-sort by start if it prefers. **Pin this**:
`completed` is documented as end-order, so tests assert end-order and
Milestone 10 does not silently assume start-order. `orphaned` is in
ascending instant order by construction.

`minutes` uses `num_minutes()`, which truncates toward zero. Because
the sort guarantees `end.at_utc >= start.at_utc`, the value is always
>= 0 and truncation never surprises; storage is minute-granular anyway
(§4.1), so sub-minute components should never exist in real data.

Complexity: O(n log n) sort + O(n) scan, one `Vec` of at most n punches.
Fine at personal-use scale (a day has single-digit punches).

### 3.3 How each named §4.3 edge case falls out

1. **Single open stint (§4.3 bullet 1, F1)** — the last `Start` has no
   subsequent `End` to pop it, so the loop ends with `stack.len() == 1`.
   It becomes `open[0]`. `has_anomaly` is computed as
   `open.len() > 1 || !orphaned_ends.is_empty()`, so with one open and
   no orphans it is **false** — the normal ongoing case, explicitly not
   an anomaly. `anomalies()` returns empty. `is_ongoing()` is true,
   which is a *different* signal from `has_anomaly()` and must not be
   conflated (§7.1's `(+ ongoing)` vs §7.3's `[!]`).

2. **Multi-open (§4.3 bullet 2, E7)** — two or more `Start`s with fewer
   subsequent `End`s to pop them, so the loop ends with
   `stack.len() >= 2`. Each stack entry becomes its own `OpenStint`
   (each with its own live duration against the same `now`), so
   Milestone 10 can render one ongoing line per §E7's "lists each as
   its own open/ongoing stint". The `open.len() > 1` term flips
   `has_anomaly` to true, and `anomalies()` emits exactly ONE
   `MultipleOpenStints { count: open.len() }` — a single summary
   anomaly for the whole date, matching §7.3's `[!] 2 open stints for
   this date` line, not one per extra start. Note the asymmetry with
   orphans (which are per-punch): it is deliberate and comes straight
   from §7.3's two example lines.

3. **Orphaned end (§4.3 bullet 3, E8)** — an `End` arrives when
   `stack` is empty, so `pop()` returns `None`. The punch is pushed to
   `orphaned` carrying its own instant, and — critically — **nothing is
   pushed into `completed`**, so it contributes zero minutes to any
   total (§2.4, NOTES.md decision 38) and produces no stint line
   (E8's "produces no stint-list line of its own"). No guessing at
   which start it belonged to; no silent drop. Two orphans on one date
   produce two `OrphanedEnd` values and two `Anomaly::OrphanedEnd`
   entries, never coalesced into a count (E8 explicitly forbids that,
   which is also why `OrphanedEnd` has no `count` field and
   `MultipleOpenStints` does).

4. **Zero-length same-instant stint (§4.3 bullet 4, E14)** — the
   `Start` and `End` share an `at_utc`; the §3.1 sort places the
   `Start` first (kind_rank), so the scan pushes then immediately pops
   it. `minutes = (t - t).num_minutes() = 0`. It lands in `completed`
   like any other stint, adds `0` to the day total, and touches neither
   `open` nor `orphaned` — so `has_anomaly` stays false. This is the
   case that *requires* the kind_rank tiebreak; with pure id ordering
   it would degrade into case 3 + case 1 whenever the `stop` was typed
   first.

Two further behaviors worth stating because tests pin them:

5. **Nested entry (§4.3's "legitimately nested" note)** — with
   `S1 < S2 < E1 < E2` by instant, the scan pushes S1, pushes S2, then
   the first `End` pops **S2** (most recent), pairing `S2–E1`; the
   second `End` pops S1, pairing `S1–E2`. Nearest-match, not
   first-in-first-out — this is the whole reason §4.3 rejects "sort
   then pair sequentially by index". `completed` emits `S2–E1` before
   `S1–E2` (end order), and the outer stint fully contains the inner
   one, so their minutes double-count wall-clock time. That is what
   nearest-match means and the spec accepts it; **no overlap detection
   and no de-duplication is in scope here** (see §7.4).

6. **Zero punches (F4/E11 at this layer)** — the sort and scan both
   no-op; `completed`, `open`, `orphaned_ends` are all empty,
   `has_anomaly` false, `completed_minutes()` 0, `is_ongoing()` false.
   No `Option`, no error — an empty `DayStints`, which is what lets
   Milestone 10 omit sections rather than special-casing.

### 3.4 Open-stint live duration

`minutes_so_far = max(0, (now - start.at_utc).num_minutes())`.

The `max(0, ..)` clamp covers `now < start.at_utc`, which is reachable
today: §3.2 lets a user type `mlm start 23:00` at 09:00 (no chronology
validation at insert, §3.2/F-none), leaving a future-dated open start.
SPEC.md does not name this case; clamping to `00h 00m` is chosen over a
negative duration because §4.2's signed format exists for
owed/carry values, and a negative *elapsed* time on a stint line would
read as a bug. **Flagged in §7.3.** All open stints on a date are
computed against the same single `now` value, so two open stints never
disagree about the current instant.

---

## 4. "Now" injection (interface contract 6)

```rust
pub fn classify(punches: &[Punch], now: chrono::DateTime<chrono::Utc>) -> DayStints
```

- A **plain positional parameter of type `DateTime<Utc>`** — not a
  `Clock` trait, not an `Option<DateTime<Utc>>` defaulting to
  `Utc::now()`, not a global.
- **UTC, not local**: `Punch::at_utc` is UTC, so the subtraction in
  §3.4 is offset-free and DST-proof by construction. Converting to
  local for display is Milestone 10's edge-of-the-program job (§2.1).
- `classify` **never calls `Utc::now()`**. The single real clock read
  lives in `main.rs` and is threaded down. Concretely: no `use` of
  `Utc::now` anywhere in `src/stint.rs` outside `#[cfg(test)]` — a
  grep-able rule an adversarial reviewer can check mechanically.
- It is required, not optional, even when no stint is open. Passing it
  unconditionally keeps the signature stable and avoids a caller
  branching on whether it thinks something is open (it can't know
  before calling).
- Rationale for rejecting a `Clock` trait: this milestone needs exactly
  one instant, once, with no sequencing — a trait would add a generic
  parameter or a `dyn` object to every downstream signature for no test
  power a literal `DateTime<Utc>` doesn't already give. If Milestones
  9/10/11 later want a shared clock abstraction, it wraps this
  parameter rather than replacing it.

---

## 5. Module/API surface summary

Public from `src/stint.rs`: `PunchKind`, `Punch`, `Stint`, `OpenStint`,
`OrphanedEnd`, `Anomaly`, `DayStints` (with `has_anomaly()`,
`anomalies()`, `completed_minutes()`, `is_ongoing()`), and `classify()`.
`DayStints::has_anomaly` is the only private field. No other
constructor is exposed — `DayStints` is built only by `classify`, so
the `has_anomaly` invariant cannot be violated from outside.

---

## 6. Test plan

All tests live in `#[cfg(test)] mod tests` in `src/stint.rs`, use the
`p(id, "HH:MM", kind)` fixture helper (§1), and a
`const NOW: DateTime<Utc>` set to `2026-02-12T17:45:00Z` unless a case
states otherwise. `TEST_DATE = 2026-02-12`. Punch ids in each case are
written in **entry order**, which is what makes the "entry order is
irrelevant" property visible.

| # | flow | name | input (entry order) | expected |
|---|---|---|---|---|
| T1 | F1 | `single_start_is_one_open_stint_not_an_anomaly` | `p(1,"09:00",Start)`; now=`09:30` | `completed` empty; `open` = 1 entry, start id 1, `minutes_so_far` 30; `orphaned_ends` empty; `has_anomaly()` **false**; `anomalies()` empty; `is_ongoing()` true; `completed_minutes()` 0 |
| T2 | F2 | `ordinary_pair_is_one_completed_stint` | `p(1,"09:00",Start)`, `p(2,"17:00",End)` | `completed` = [09:00–17:00, 480]; `open` empty; `orphaned_ends` empty; `has_anomaly()` false; `is_ongoing()` false; `completed_minutes()` 480 |
| T3 | F3 | `out_of_order_entry_pairs_by_time_not_entry_order` | `p(1,"09:00",Start)`, `p(2,"14:00",Start)`, `p(3,"18:00",End)`, `p(4,"13:00",End)` | `completed` = [09:00–13:00 (240), 14:00–18:00 (240)] **in that order** (end-instant order); `open`/`orphaned_ends` empty; `has_anomaly()` false; `completed_minutes()` 480. This is §4.3's worked example verbatim. |
| T3b | F3 | `entry_order_does_not_change_result` | same four punches as T3 but ids assigned in a different permutation (e.g. `p(1,"13:00",End)`, `p(2,"09:00",Start)`, `p(3,"18:00",End)`, `p(4,"14:00",Start)`) | identical `completed` pairs/minutes to T3 (ids differ; assert on instants+minutes) — proves the sort, not the input order, drives pairing |
| T4 | F9 (open half) | `open_stint_duration_is_measured_against_supplied_now` | `p(1,"09:00",Start)`, `p(2,"13:00",End)`, `p(3,"17:30",Start)`; now=`17:45` | `completed` = [09:00–13:00 (240)]; `open` = 1, `minutes_so_far` **15**; `completed_minutes()` **240** (the open 15 min is NOT folded in — §2.4/NOTES 37); `is_ongoing()` true; `has_anomaly()` false |
| T4b | F9/contract 6 | `open_stint_duration_changes_only_with_now` | same punches as T4, called twice with now=`17:45` and now=`18:45` | `minutes_so_far` 15 then 75; `completed` and `completed_minutes()` byte-identical between the two calls — proves no hidden clock and that only the open stint is time-sensitive |
| T5 | E7 | `two_dangling_starts_are_two_open_stints_plus_multi_open_anomaly` | `p(1,"09:00",Start)`, `p(2,"11:00",Start)`; now=`12:00` | `completed` empty; `open` = 2 entries **ascending by start** (09:00 → 180 min, 11:00 → 60 min); `orphaned_ends` empty; `has_anomaly()` **true**; `anomalies()` == `[MultipleOpenStints { count: 2 }]` — exactly one anomaly, not two; `completed_minutes()` 0 |
| T5b | E7 | `three_dangling_starts_report_count_three` | `p(1,"09:00",Start)`, `p(2,"10:00",Start)`, `p(3,"11:00",Start)` | `open.len()` 3; `anomalies()` == `[MultipleOpenStints { count: 3 }]` (still one anomaly value) |
| T5c | E7 | `mixed_completed_and_multi_open` | `p(1,"08:00",Start)`, `p(2,"09:00",End)`, `p(3,"10:00",Start)`, `p(4,"11:00",Start)`; now=`12:00` | `completed` = [08:00–09:00 (60)]; `open` = 2; `has_anomaly()` true; `completed_minutes()` 60 — anomaly does not suppress the good stint |
| T6 | E8 | `orphaned_end_produces_anomaly_and_no_stint` | `p(1,"18:00",End)` | `completed` empty; `open` empty; `orphaned_ends` = 1 entry at 18:00; `has_anomaly()` true; `anomalies()` == `[OrphanedEnd { punch @18:00 }]`; `completed_minutes()` **0**; `is_ongoing()` false |
| T6b | E8 | `two_orphaned_ends_are_never_coalesced` | `p(1,"12:00",End)`, `p(2,"18:00",End)` | `orphaned_ends.len()` **2**, ascending by instant (12:00 then 18:00); `anomalies()` == two separate `OrphanedEnd` values, in that order; no `MultipleOpenStints`; `completed_minutes()` 0 |
| T6c | E8 | `orphaned_end_before_a_clean_pair` | `p(1,"08:00",End)`, `p(2,"09:00",Start)`, `p(3,"10:00",End)` | `orphaned_ends` = [08:00]; `completed` = [09:00–10:00 (60)]; `has_anomaly()` true; `completed_minutes()` 60 — the orphan doesn't consume the later start |
| T6d | E8+E7 | `orphan_and_multi_open_emit_both_anomalies_in_order` | `p(1,"08:00",End)`, `p(2,"09:00",Start)`, `p(3,"10:00",Start)`; now=`11:00` | `anomalies()` == `[MultipleOpenStints{count:2}, OrphanedEnd{@08:00}]` — pins §2's fixed emission order (multi-open first, then orphans) |
| T7 | E14 | `same_instant_pair_is_a_legal_zero_length_stint` | `p(1,"09:00",Start)`, `p(2,"09:00",End)` | `completed` = one stint, `minutes` **0**; `open`/`orphaned_ends` empty; `has_anomaly()` **false**; `completed_minutes()` 0 |
| T7b | E14 (§7.1 risk) | `same_instant_pair_still_pairs_when_end_entered_first` | `p(1,"09:00",End)`, `p(2,"09:00",Start)` — end has the LOWER id | identical to T7: one zero-length completed stint, `has_anomaly()` false. **This is the test that fails under a pure `(at_utc, id)` sort and passes under §3.1's `(at_utc, kind_rank, id)`.** |
| T8 | §4.3 nested note | `nested_entry_pairs_nearest_not_outermost` | `p(1,"09:00",Start)`, `p(2,"10:00",Start)`, `p(3,"11:00",End)`, `p(4,"12:00",End)` | `completed` = [**10:00–11:00** (60), **09:00–12:00** (180)] in that order; assert explicitly that 10:00 paired with 11:00 and NOT 09:00 with 11:00; `open`/`orphaned_ends` empty; `has_anomaly()` false; `completed_minutes()` **240** (overlap double-counts by design, §3.3.5) |
| T9 | F4/E11 | `no_punches_produces_empty_classification` | `&[]`; any `now` | all three vecs empty; `has_anomaly()` false; `anomalies()` empty; `completed_minutes()` 0; `is_ongoing()` false; no panic (note: `classify` must not index `punches[0]` unguarded — see §3.0) |
| T10 | E15 (§1.2 limitation) | `cross_midnight_halves_are_two_separate_anomalies` | two separate `classify` calls: day 1 = `[p(1,"23:30",Start)]`, day 2 = `[p(2,"00:45",End)]` (different `TEST_DATE`s) | day 1: one open stint, `has_anomaly()` false (it's the ordinary open-stint case, per §4.3 bullet 1); day 2: one orphaned end, `has_anomaly()` true. Documents the accepted MVP limitation at this layer rather than leaving it to end-to-end tests |
| T11 | contract 2 | `has_anomaly_matches_anomalies_nonempty` | table-driven over T1/T2/T5/T6/T7/T9's inputs | for every case, `d.has_anomaly() == !d.anomalies().is_empty()` — the invariant that justifies storing the bool |
| T12 | §2.4 | `completed_minutes_excludes_open_and_orphans` | `p(1,"09:00",Start)`, `p(2,"10:00",End)`, `p(3,"11:00",End)`, `p(4,"12:00",Start)`; now=`13:00` | `completed_minutes()` == **60** exactly, while `open[0].minutes_so_far` == 60 and one orphan exists at 11:00 — a single assertion that neither leaks into the total (NOTES.md 37/38) |

Notes for the TDD agent:

- Write T1–T3 first (they define the happy path and force the sort +
  scan to exist), then T7b (forces the kind_rank tiebreak), then the
  anomaly cases.
- Assert on whole `DayStints` values where practical (derive
  `PartialEq`) rather than field-by-field, so an unexpected extra
  anomaly can't slip past a narrow assertion.
- Every duration assertion is an `i64` minute count. Do **not** assert
  on formatted `HHh MMm` strings anywhere in this milestone — that
  format belongs to Milestone 1 and asserting it here would create the
  duplicate formatting logic PLAN.md's cross-cutting section warns
  about.

---

## 7. Ambiguities, risks, and disagreements

### 7.1 RISK (highest) — the same-instant tiebreak contradicts §4.3's letter

SPEC.md §4.3 step 1 says ties are broken by `id`, and its E14 bullet
then asserts that a same-instant start/end pair "produces a zero-length
stint... not itself an anomaly" and that "both punches pair up
cleanly... step 1's tie-break by `id` still applies to the sort, but
doesn't change that both punches pair up cleanly." **Those two claims
are inconsistent.** If the user runs `mlm stop 09:00` before
`mlm start 09:00`, id order puts the `end` first, the scan sees an
empty stack, and the date yields an orphaned end + an open stint — two
anomalies, not a clean zero-length stint.

Decision taken here: sort by `(at_utc, kind_rank, id)` with
`Start` < `End`, honoring E14's *stated outcome* over step 1's literal
key. **Resolved, not just a unilateral call**: cross-plan adversarial
review confirmed this was a genuine SPEC.md defect and patched §4.3
step 1 directly to specify the kind-then-id tiebreak. T7b's expectation
(zero-length stint, no anomaly) is now the spec-correct one, not a
guess pending sign-off.

### 7.2 RESOLVED — punch-shape contract, per PLAN.md contract 8

Milestone 4 has since been designed and independently converged on the
identical field-for-field shape proposed here (including the same
`kind`-then-`id` tie-break). Per cross-plan review, **Milestone 4
(`src/storage.rs`) is the sole owner** of `Punch`/`PunchKind` — see §1
above, which now reflects this as an import rather than a local
declaration. The specific concerns originally raised here are all
settled: `id: i64` ✓, `at_utc` pre-parsed to `DateTime<Utc>` ✓, `date`
carried rather than re-derived ✓, `kind` as a validated enum ✓,
`Punch: Copy` ✓ (Milestone 4's plan was patched to add the derive).

### 7.3 AMBIGUITY — `now` earlier than an open start

Not covered by SPEC.md. Reachable because §3.2 imposes no chronology
requirement at insert. §3.4 clamps `minutes_so_far` at 0. Alternatives
rejected: a negative duration (reads as a bug on a stint line), or a
new anomaly kind (§4.3's anomaly list is closed, and inventing a fifth
one exceeds this milestone's mandate). Milestone 10 may separately
decide how `HH:MM-now (00h 00m, ongoing)` reads when the start is in
the future — a rendering question, not a pairing one.

### 7.4 NOTED, NOT A BUG — nested stints double-count

T8's nested case produces overlapping stints (09:00–12:00 contains
10:00–11:00) totaling 240 minutes of "worked" time across 180 minutes
of wall clock. §4.3 explicitly blesses nested entry as "not malformed",
and §2.4 defines worked minutes as the sum of completed stints with no
overlap rule. So this is spec-conformant and feeds Milestone 6 as-is.
Flagging it because it is the most plausible thing an adversarial
reviewer will mistake for a bug, and because it means a fat-fingered
double `start` can silently inflate a week total without ever raising
`has_anomaly()`. If that is undesirable, it is a SPEC.md change
(a new overlap anomaly), not a Milestone 5 fix.

### 7.5 AMBIGUITY — ordering of `completed`

§4.3 never says what order the resulting stints are presented in, and
§7.1's example only shows non-overlapping stints where start-order and
end-order coincide. §3.2 pins **end-instant order** (natural emission
order) and T3/T8 assert it. Milestone 10 must be told this explicitly;
if it wants start-order for the status list, that is a one-line sort at
the rendering layer, and T8's expectation is the case where the two
orders actually differ.

### 7.6 MINOR — `MultipleOpenStints.count` is redundant

It always equals `open.len()`. Carried anyway so Milestone 9 can render
§7.3's `[!] 2 open stints for this date` from the `Anomaly` value alone
without also needing the `DayStints` it came from — which is precisely
the "no adapter in 10 and 11" goal of contract 2. If Milestone 9's plan
prefers to take `&DayStints`, the field can be dropped.

### 7.7 MINOR — this milestone assumes minute-granular data but doesn't enforce it

§4.1 says there is no seconds precision anywhere, but `DateTime<Utc>`
can carry seconds and `num_minutes()` truncates. If a punch ever lands
with `:30` seconds, durations silently round down. Enforcement belongs
at the write edge (Milestone 4 should zero seconds/nanos when
constructing `at_utc`). Explicitly flagging it as Milestone 4's
responsibility so neither milestone assumes the other does it.
