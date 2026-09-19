# Task 1 low-level plan: `src/stint.rs` — tie-break reorder + `classify_at`

## What was read for this plan

- `docs/dev/plans/boundary-stint-pairing-plan.md` (full) — changeset
  scope-of-work plan, Task 1/Task 2 split, global constraints,
  interface contract.
- `docs/dev/specs/2026-09-19-boundary-stint-pairing.md` (full) — locked
  design spec, two adversarial review rounds folded in.
- `src/stint.rs` (full, 641 lines, including its entire `#[cfg(test)]`
  module) — current implementation, as of this writing.
- `Cargo.toml` — crate metadata, dependency versions (`chrono 0.4.45`,
  edition 2024, no clippy-specific `[lints]` table).
- `clippy.toml` — `cognitive-complexity-threshold = 15` (stricter than
  clippy's own default of 25).
- `.githooks/pre-commit` — quality gate: `cargo fmt` check (blocking),
  `cargo clippy --all-targets --quiet -- -W clippy::cognitive_complexity`
  scored against the `clippy.toml` threshold per function (blocking),
  and a `cargo-llvm-cov` line-coverage-regression check against
  `coverage-baseline.json` (degrades to SKIP if tooling missing, never
  hard-fails on a clean machine without it — but must not regress on a
  machine that has it).
- `.github/workflows/ci.yml` — CI runs
  `cargo clippy --all-targets --target <target> -- -D warnings` (all
  warnings deny, not just cognitive-complexity) plus `cargo fmt --check`
  and `cargo test`, matrixed across targets.

No sibling task plan exists yet (Task 2's plan, for `src/status.rs` /
`src/week_view.rs`, is being written in parallel by another agent). Per
the coordinator's instruction this plan does not wait for it — Task 2
depends only on the interface contract already pinned in the changeset
plan (§"Interface contract to pin before either task starts"), which is
reproduced and held to exactly below.

## Scope

**This plan owns `src/stint.rs` exclusively — including its
`#[cfg(test)]` module.** It does not touch, and makes no assumption
about the internals of, `src/status.rs` or `src/week_view.rs` — those
are Task 2's exclusive files (changeset plan §"File ownership summary").
No source has been edited to produce this document; it is a plan file
only. `cargo build`/`cargo test` for the crate as a whole will not
succeed after Task 1 lands in isolation, because `classify_at` has no
caller yet outside `stint.rs`'s own tests — that is expected per the
changeset plan (Task 1 acceptance criteria, last bullet) and is not
a defect to fix here.

## Facts gathered from the current file (baseline for the diff)

- `src/stint.rs` is 641 lines total; non-test code is lines 1-169,
  `#[cfg(test)] mod tests` is lines 171-641.
- Imports (lines 12, 14):
  ```rust
  use chrono::{DateTime, Utc};
  use crate::storage::{Punch, PunchKind};
  ```
  `Punch` (from `crate::storage`) has fields `id: i64`, `at_utc:
  DateTime<Utc>`, `date: NaiveDate`, `kind: PunchKind` (inferred from
  test helpers `p()`/`p_on()`, lines 203-220, and the existing
  `debug_assert!` at line 117 comparing `q.date`). `PunchKind` is
  `Copy`, orders `Start` before `End` (comment at line 125: "`Start`
  before `End`, which is `PunchKind`'s own `Ord`").
- `classify`'s current body (lines 115-169):
  - Line 116-119: `debug_assert!(punches.iter().all(|q| q.date ==
    punches[0].date), ...)`. This is safe today because
    `.iter().all()` short-circuits to `true` on an empty iterator
    without the closure ever running, so `punches[0]` is never
    evaluated when `punches` is empty — this is the exact pattern spec
    §4.2 says to mirror for `classify_at`'s own guards.
  - Lines 121-128 (step 1): clones punches into `sorted: Vec<Punch>`,
    stable-sorts by `(q.at_utc, q.kind, q.id)`.
  - Lines 130-147 (step 2, **the code this task reorders**): a single
    flat `for punch in sorted` loop — `Start` pushes to `stack: Vec<Punch>`,
    `End` pops the stack (`Some` → push to `completed`, `None` → push
    to `orphaned_ends`). This is the exact loop §3.2 replaces with a
    two-phase, tied-group-aware scan.
  - Lines 149-159: builds `open: Vec<OpenStint>` from whatever remains
    on `stack`, computing `minutes_so_far` against the injected `now`,
    clamped at 0 via `.max(0)`.
  - Line 161: `let has_anomaly = open.len() > 1 || !orphaned_ends.is_empty();`
  - Lines 163-168: constructs `DayStints { completed, open,
    orphaned_ends, has_anomaly }` — a plain struct literal, callable
    only from inside `stint.rs` because `has_anomaly` is a private
    field (line 68) and there is no public constructor. This is the
    fact that determines the mechanism section below.
- `DayStints` (lines 56-69) has four fields: `completed: Vec<Stint>`,
  `open: Vec<OpenStint>`, `orphaned_ends: Vec<OrphanedEnd>`, and the
  private `has_anomaly: bool`. Its only public accessors are
  `has_anomaly()` (line 73, returns the stored bool — does **not**
  recompute), `anomalies()` (lines 82-95, recomputes an `Anomaly` list
  live from `open`/`orphaned_ends`), `completed_minutes()` (lines
  99-101), `is_ongoing()` (lines 104-106).
- `Stint { start: Punch, end: Punch, minutes: i64 }` (lines 17-24,
  `Copy`). `OpenStint { start: Punch, minutes_so_far: i64 }` (lines
  27-34, `Copy`). `OrphanedEnd { punch: Punch }` (lines 37-41, `Copy`).
  All three are plain public structs with public fields — freely
  constructible from within the same module (and in fact from anywhere,
  since their fields are `pub`), which matters for the splice: taking
  an `OpenStint` out of `open` and an `OrphanedEnd` out of
  `orphaned_ends` and turning them into a `Stint` needs only their
  already-public `.start`/`.punch` fields.
- Module-level `#![allow(dead_code)]` (line 10) — `classify_at` being
  uncalled outside tests during Task 1 will not trigger a dead-code
  warning; this allow already covers it, consistent with the
  changeset plan's expectation that the crate doesn't compile
  end-to-end until Task 2.
- Test module facts: `TEST_DATE = (2026, 2, 12)` (line 178), helpers
  `at(hhmm)` (same-date instant), `at_on(d, hhmm)` (explicit-date
  instant, already exists — used by the existing cross-midnight test),
  `p(id, hhmm, kind)` (same-date punch), `p_on(id, d, hhmm, kind)`
  (explicit-date punch, already exists), `completed_spans(&DayStints)`
  helper (lines 225-230). The existing `cross_midnight_halves_are_two_
  separate_anomalies` test (lines 578-593) already builds two adjacent
  dates (`day1`/`day2` via `NaiveDate::from_ymd_opt`) and is the closest
  existing precedent for `classify_at` test fixtures — it stays
  unmodified (it exercises plain `classify()`, not `classify_at`, and
  its assertions about "two separate anomalies" are exactly what §5.1's
  regression sweep protects).

## Part 1 — §3.2 tied-instant-group scan (reorders step 2 only)

### Constraint recap

- `classify()`'s signature is byte-for-byte unchanged (line 115 itself
  does not change).
- Steps 1 (sort, lines 121-128) and the post-loop open-stint build
  (lines 149-159) and `has_anomaly` computation (line 161) and struct
  literal (163-168) are **untouched** — only the loop body at lines
  130-147 changes shape.
- Every singleton group (no tie) must reduce to exactly today's
  push/pop order — this is what keeps every existing non-tied-group
  test passing unmodified with zero test edits.

### Algorithm, translated from spec §3.2 into the existing sorted-Vec shape

`sorted` is already grouped by construant: since it's sorted by
`(at_utc, kind, id)`, all punches sharing one `at_utc` are contiguous.
The new scan processes `sorted` in contiguous same-`at_utc` chunks
(`chunk_by`/`slice::group_by`-style, or a manual index walk — see
below for the concrete choice) instead of one punch at a time:

1. For each group (ascending `at_utc`):
   a. **Step 1 of §3.2**: for each `End` in the group, in ascending
      `id` order (which — because the sort key is `(at_utc, kind, id)`
      and `Start < End` — means: within the group's `End`s specifically,
      already in ascending-`id` order as a sub-sequence of `sorted`,
      *but* the group as sorted has all `Start`s before all `End`s, so
      this "in `id` order" needs its own explicit sort/partition — see
      pitfall note below): if `stack` (carried in from strictly earlier
      groups) is non-empty, pop and close (push to `completed`).
   b. **Step 2 of §3.2**: push the group's own `Start`s onto `stack`,
      in ascending `id` order (already satisfied: within the group,
      `sorted`'s stable sort already orders same-kind punches by `id`,
      and all `Start`s in a group sort before all `End`s under
      `(at_utc, kind, id)` — but that ordering is irrelevant to step 2a
      because step 2a must run *before* any of this group's `Start`s
      are pushed, not interleaved with them).
   c. **Step 3 of §3.2**: any `End` in the group not yet closed in (a)
      pops against `stack`, which now holds only this group's own
      pushed `Start`s (LIFO) — closing them nearest-first, or falling
      to `orphaned_ends` if the group's own `Start`s are also
      exhausted.

**Pitfall worth calling out explicitly**: `sorted` already has, within
one `at_utc` group, all `Start`s before all `End`s (because the sort
key is `(at_utc, kind, id)` and `PunchKind::Start < PunchKind::End`).
That means a plain `for punch in group { ... }` loop over the group as
laid out in `sorted` visits `Start`s first — which is exactly the
*old*, buggy order for step 2a's purposes (it would push this group's
`Start`s before any `End` in the group gets a chance to look at the
pre-group stack). The new scan must **not** just iterate the group
once in `sorted`'s existing order; it must explicitly separate a
group's `End`s and `Start`s and run the `End`s-against-pre-group-stack
pass (2a) before pushing any `Start`s (2b), which is exactly the
generalization spec §3.2 describes. Concretely, within a group already
sliced out of `sorted`, `Start`s are contiguous at the front and `End`s
at the back (given the sort key), so `group.iter().filter(|q| q.kind ==
PunchKind::End)` for phase (a) and `group.iter().filter(|q| q.kind ==
PunchKind::Start)` for phase (b)/(c)'s residual push is sufficient —
no re-sort needed, since each sub-filter is already in ascending-`id`
order as a consequence of the stable sort.

### Code sketch (replaces lines 130-147 only; everything before line
130 and after line 147 is untouched)

```rust
// Step 2: nearest-match (LIFO) scan, processed one tied-instant group
// at a time (SPEC.md §4.3 step 2, boundary-fix §3.2). Every group of
// size 1 (no tie) reduces to exactly the old flat scan: a lone `End`
// has nothing to skip in phase (a) besides itself, and a lone `Start`
// only ever gets pushed in phase (b).
let mut stack: Vec<Punch> = Vec::new();
let mut completed: Vec<Stint> = Vec::new();
let mut orphaned_ends: Vec<OrphanedEnd> = Vec::new();

let mut i = 0;
while i < sorted.len() {
    let group_at = sorted[i].at_utc;
    let group_end = sorted[i..]
        .iter()
        .position(|q| q.at_utc != group_at)
        .map_or(sorted.len(), |off| i + off);
    let group = &sorted[i..group_end];

    // §3.2 step 1: each `End` in the group, in ascending id order
    // (guaranteed by the stable (at_utc, kind, id) sort), preferentially
    // closes a `Start` carried in from a strictly earlier group — BEFORE
    // any of this group's own `Start`s are pushed. This is the fix: an
    // `End` reaches backward past same-instant `Start`s.
    let mut unclosed_ends: Vec<Punch> = Vec::new();
    for punch in group.iter().filter(|q| q.kind == PunchKind::End) {
        match stack.pop() {
            Some(start) => completed.push(Stint {
                start,
                end: *punch,
                minutes: (punch.at_utc - start.at_utc).num_minutes(),
            }),
            None => unclosed_ends.push(*punch),
        }
    }

    // §3.2 step 2: push the group's own Starts (LIFO, id order —
    // already the group's natural order under the sort key).
    for punch in group.iter().filter(|q| q.kind == PunchKind::Start) {
        stack.push(*punch);
    }

    // §3.2 step 3: any End that found nothing open before this group
    // now pops against the stack again, which holds only this group's
    // own just-pushed Starts (or is empty) — same-instant E14 pairing,
    // or an ordinary orphaned-end anomaly.
    for punch in unclosed_ends {
        match stack.pop() {
            Some(start) => completed.push(Stint {
                start,
                end: punch,
                minutes: (punch.at_utc - start.at_utc).num_minutes(),
            }),
            None => orphaned_ends.push(OrphanedEnd { punch }),
        }
    }

    i = group_end;
}
```

Verification this reduces to the old scan for every singleton group:
a group of size 1 holding a lone `Start` — phase (a)'s filter yields no
`End`s, `unclosed_ends` stays empty, phase (b) pushes the one `Start`,
phase (c) has nothing to iterate. A group of size 1 holding a lone
`End` — phase (a)'s filter yields that one `End`; it pops `stack` (the
pre-group stack, which for a non-tied scan is the *entire* stack built
so far, identical to what the old flat loop would have popped at that
point) exactly as the old code did at that same position in `sorted`.
No behavior changes for any group of size 1. `Copy`-ness of `Punch`
(derived in line-17-24-adjacent structs, and `Punch` itself must be
`Copy` too for the existing code at line 137/141 to compile — confirmed
by the existing `*` derefs implied by `stack.push(punch)` moving
`Punch` by value in the current code) means `*punch`/`punch` copies are
free and match the existing code's style (the current loop already
moves `Punch` by value into `stack`/`completed`/`orphaned_ends`).

`cognitive-complexity-threshold = 15` (clippy.toml) is a real
constraint here: the new step-2 block adds a `while` loop, a
`.position()` closure, two `.filter()` closures, and two more `match`
arms nested inside — meaningfully more branching than the old flat
`for`/`match`. If clippy's cognitive-complexity lint (enforced via
`-W clippy::cognitive_complexity` in the pre-commit hook, scored
against this threshold, and folded into `-D warnings` in CI) flags
`classify()` after this change, the fix is to extract the group-close
logic (phases a/b/c) into a small private helper function — e.g.
`fn close_group(group: &[Punch], stack: &mut Vec<Punch>, completed:
&mut Vec<Stint>, orphaned_ends: &mut Vec<OrphanedEnd>)` — rather than
raising the threshold in `clippy.toml`, since the threshold is a
project-level convention this task should not unilaterally relax.
Whether this split is actually needed can only be confirmed by running
`cargo clippy --all-targets -- -D warnings` once the code exists;
flag this as a build-time decision point, not a certainty, in Risks
below.

## Part 2 — `classify_at`

### Signature (pinned by the changeset plan's interface contract — reproduced verbatim, not re-derived)

```rust
pub fn classify_at(
    prev_punches: &[Punch],
    punches: &[Punch],
    next_punches: &[Punch],
    now: DateTime<Utc>,
) -> DayStints
```

Placed directly after `classify()` (i.e. after line 169, before the
`#[cfg(test)]` module) in `src/stint.rs`.

### Doc comment (must state the panic-safety contract explicitly — Task 1 acceptance criterion)

```rust
/// Classifies `punches` (one calendar date) same as `classify()`, then
/// resolves an unambiguous midnight-spanning stint against its immediate
/// neighbors (SPEC.md §4.3's boundary-splice rule, this changeset's
/// spec §4.1/§4.2).
///
/// `prev_punches`/`next_punches` must be the literal adjacent calendar
/// dates' punches — `punches[0].date.pred_opt()`/`.succ_opt()` — never
/// further out (pass `&[]` when a neighbor has no data; an empty slice
/// already classifies correctly as "nothing open, nothing orphaned").
///
/// Never panics for any combination of empty/non-empty
/// `prev_punches`/`punches`/`next_punches` — including `punches` empty
/// with a non-empty neighbor, the ordinary shape for every idle
/// calendar date. All internal self-checks are gated on `.first()`
/// before comparing any `[0]`-indexed date.
///
/// With both neighbors empty, reduces exactly to `classify(punches,
/// now)`'s output.
pub fn classify_at(
    prev_punches: &[Punch],
    punches: &[Punch],
    next_punches: &[Punch],
    now: DateTime<Utc>,
) -> DayStints {
    ...
}
```

### debug_assert guards (spec §4.2, changeset plan constraint 4 — the one property every reviewer re-verifies by hand)

Mirror `classify()`'s own guard (lines 116-119) once per non-empty
slice, plus the two cross-slice adjacency checks, every one gated on
`.first()`:

```rust
debug_assert!(
    prev_punches.iter().all(|q| Some(q.date) == prev_punches.first().map(|f| f.date)),
    "classify_at() expects prev_punches to share one date"
);
debug_assert!(
    punches.iter().all(|q| Some(q.date) == punches.first().map(|f| f.date)),
    "classify_at() expects punches to share one date"
);
debug_assert!(
    next_punches.iter().all(|q| Some(q.date) == next_punches.first().map(|f| f.date)),
    "classify_at() expects next_punches to share one date"
);
if let (Some(p), Some(d)) = (prev_punches.first(), punches.first()) {
    debug_assert!(
        Some(p.date) == d.date.pred_opt(),
        "classify_at() expects prev_punches' date to be punches' date minus one day"
    );
}
if let (Some(d), Some(n)) = (punches.first(), next_punches.first()) {
    debug_assert!(
        Some(n.date) == d.date.succ_opt(),
        "classify_at() expects next_punches' date to be punches' date plus one day"
    );
}
```

Note the first three guards use `.first().map(...)` rather than
`punches[0].date` even for the "one shared date" check — this is
*stricter* than mirroring `classify()`'s exact line 116-119 pattern
(which does index `punches[0]` inside the closure, safely, only
because `.all()` on an empty iterator short-circuits before the
closure runs at all). Reusing that exact `punches[0]`-inside-closure
pattern verbatim for `prev_punches`/`next_punches` would in fact still
be safe for the same short-circuit reason — but the two adjacency
checks below it *cannot* use that trick, because they compare across
two different slices and must run their comparison body even when one
side is empty and the other isn't (that's precisely the "idle day
bordering real data" case constraint 4 calls out) — so the
`.first()`-gated `if let` form is used uniformly for the two
cross-slice checks to avoid a mixed style, and reused for the
single-slice checks too for consistency, rather than having three
different idioms in one function. This is a plan-level judgment call,
not something the spec pins to one exact syntax — the spec's own
prose (§4.2) says "gate on `.first()`", not "match `classify()`'s
exact closure shape", so either is spec-compliant; the all-`.first()`
version is chosen here because it's the more uniform, more obviously-
correct-by-inspection option to write once and never revisit.

All five guards are `debug_assert!` — compiled out in release, exactly
like `classify()`'s existing guard — so they cost nothing in a release
build and exist purely to catch a caller's off-by-one during
development/tests, per spec §4.2's framing ("classify_at needs the
equivalent self-checks").

### Splice helper and full body

```rust
pub fn classify_at(
    prev_punches: &[Punch],
    punches: &[Punch],
    next_punches: &[Punch],
    now: DateTime<Utc>,
) -> DayStints {
    // ... five debug_assert! guards from above ...

    let prev = classify(prev_punches, now);
    let mut day = classify(punches, now);
    let next = classify(next_punches, now);

    // (prev, day) as the (A, A+1) pair: if it fires, `day` loses the
    // matched orphan. The completed stint itself belongs to `prev`'s
    // own view (produced by prev's own classify_at call at the
    // caller level) — never added here.
    if let Some(idx) = splice_candidate(&prev, punches) {
        day.orphaned_ends.remove(idx);
    }

    // (day, next) as the (A, A+1) pair: if it fires, `day` gains the
    // completed stint and loses the matched open entry.
    if let Some(idx) = splice_candidate(&day, next_punches) {
        let open = day.open.remove(idx);
        // next's matched orphan is next's own concern in next's own
        // classify_at call — only day's `open` entry is consumed here.
        let end = next
            .orphaned_ends
            .first()
            .expect("splice_candidate only returns Some when next has exactly one orphan")
            .punch;
        day.completed.push(Stint {
            start: open.start,
            end,
            minutes: (end.at_utc - open.start.at_utc).num_minutes(),
        });
    }

    day.has_anomaly = day.open.len() > 1 || !day.orphaned_ends.is_empty();
    day
}

/// Returns `Some(index into earlier.open)` when `earlier` (already
/// classified) has exactly one trailing open start, `later_punches`
/// (the next date's raw punches) classifies to exactly one orphaned
/// end, and that orphan is `later_punches`'s chronologically first
/// punch (SPEC.md §4.1's three-part gate). `None` otherwise — no
/// splice.
///
/// Takes `later_punches` raw (not a pre-built `DayStints`) because the
/// "chronologically first punch" check needs the same (at_utc, kind,
/// id) sort `classify()` uses internally, which `DayStints` does not
/// expose.
fn splice_candidate(earlier: &DayStints, later_punches: &[Punch]) -> Option<usize> {
    if earlier.open.len() != 1 {
        return None;
    }
    let later = classify(later_punches, /* now is irrelevant: only orphaned_ends is read */ Utc::now());
    if later.orphaned_ends.len() != 1 {
        return None;
    }
    let mut sorted_later: Vec<Punch> = later_punches.to_vec();
    sorted_later.sort_by_key(|q| (q.at_utc, q.kind, q.id));
    let first = sorted_later.first()?;
    if first.id != later.orphaned_ends[0].punch.id {
        return None;
    }
    Some(0) // earlier.open.len() == 1, so its only open entry is index 0
}
```

**Problem with the sketch above, flagged and resolved**: calling
`classify(later_punches, Utc::now())` inside `splice_candidate` reads
the system clock, which directly violates the module's own stated
contract ("no clock reads — the current instant is injected", line 6)
and would make `classify_at` non-deterministic/untestable (an
`OpenStint.minutes_so_far` computed against `Utc::now()` instead of the
injected `now` — though that specific field is never read by
`splice_candidate`, the violation is still real and is exactly the
kind of thing a reviewer would flag). **Fix**: `splice_candidate` must
take `now` as a parameter and thread it through to its internal
`classify()` call, even though the resulting `open`/`minutes_so_far`
values are unused — this keeps the "no clock reads inside this module"
invariant intact and avoids a second, redundant `classify()` call
(the caller already has `next`/`day` computed once each; better yet,
`splice_candidate` should take the *already-computed* `DayStints` for
`later` too, not re-classify). Revised, final shape:

```rust
fn splice_candidate(earlier_open_count: usize, later: &DayStints, later_punches: &[Punch]) -> Option<usize> {
    if earlier_open_count != 1 {
        return None;
    }
    if later.orphaned_ends.len() != 1 {
        return None;
    }
    let mut sorted_later: Vec<Punch> = later_punches.to_vec();
    sorted_later.sort_by_key(|q| (q.at_utc, q.kind, q.id));
    let first = sorted_later.first()?;
    (first.id == later.orphaned_ends[0].punch.id).then_some(0)
}
```

called as:

```rust
if splice_candidate(prev.open.len(), &day, punches).is_some() {
    day.orphaned_ends.remove(0); // index 0: splice_candidate only returns
                                  // Some when orphaned_ends.len() == 1
}
if splice_candidate(day.open.len(), &next, next_punches).is_some() {
    let open = day.open.remove(0); // same reasoning, on day's own open
    let end = next.orphaned_ends[0].punch;
    day.completed.push(Stint {
        start: open.start,
        end,
        minutes: (end.at_utc - open.start.at_utc).num_minutes(),
    });
}
```

This removes the redundant `classify()` call entirely (each of
`prev`/`day`/`next` is computed exactly once, matching spec §4.2:
"call `classify()` on all three slices" — singular, once each), and
`splice_candidate` becomes a pure predicate over already-computed
`DayStints` plus the one raw punch slice it needs for the ordering
check, taking no `now` at all — resolving the clock-read violation by
construction rather than by threading an unused parameter.

Both `if` blocks operate on disjoint fields (`orphaned_ends` vs.
`open`, per spec §4.2's closing parenthetical) — no interaction,
correct for "both directions fire on the same day at once."

### The `has_anomaly` recomputation problem — flagged plainly

**`DayStints::has_anomaly` is a private field** (line 68: `has_anomaly:
bool` with no `pub`), and `DayStints`'s only public constructor path is
`classify()`'s own struct literal at lines 163-168, which is private
to the module (not exposed as a `pub fn` — it's inline in `classify()`'s
body). `classify_at` lives in the **same module** (`src/stint.rs`), so
it has the same private-field access `classify()` does — **no new
public constructor and no internal restructuring is required**. The
sketch above (`day.has_anomaly = day.open.len() > 1 ||
!day.orphaned_ends.is_empty();`) compiles as ordinary same-module
private-field access, exactly like line 161 inside `classify()` itself.
This is a fact worth stating plainly because the task brief explicitly
asked to check for it: **there is no internal restructuring forced by
this** — the changeset plan and spec are silent on this point (neither
says "same module" out loud), but it is correct as a consequence of
where `classify_at` is placed (inside `stint.rs`, per the interface
contract's own placement, not in a new submodule or a different file),
and this plan places it there specifically so this direct-field-mutate
approach works without any other change to `DayStints`'s definition.
If a future task ever moved `classify_at` out of `stint.rs` (it must
not — Task 1 owns `stint.rs` exclusively and the interface contract
gives no indication of relocation), this mechanism would break and a
`pub(crate)` setter or a private constructor taking recomputed fields
would become necessary. Not needed here.

### Cognitive complexity

`classify_at`'s body — 5 guards, 3 `classify()` calls, 2 `if let
Some(...)` splice blocks, one field mutate — is comparably branchy to
the reordered `classify()`. Same caveat as Part 1: run `cargo clippy
--all-targets -- -D warnings` once written; if `classify_at` trips
`cognitive_complexity` at threshold 15, extract the two near-identical
`if splice_candidate(...) { ... }` blocks' bodies into two more tiny
private helpers (e.g. `apply_prev_splice`/`apply_next_splice`) rather
than raising the threshold.

## Test plan

One subsection per Task 1 acceptance-criterion bullet (changeset plan,
Task 1 section), each citing its spec case.

### AC1 — `classify()` signature unchanged, all existing tests pass unmodified

No new test. Verified by: zero edits to any existing `#[test]` fn in
`src/stint.rs`, and `cargo test --lib stint` (or full `cargo test`)
green after Part 1's edit. This is the regression gate for spec §5.1's
"every other existing `stint.rs` test... must pass unmodified" line.

### AC2 — the two existing same-instant tests pass unmodified

`same_instant_pair_is_a_legal_zero_length_stint` (line 315) and
`same_instant_pair_still_pairs_when_end_entered_first` (line 326):
zero edits. Both are singleton-group-adjacent (a tied group of exactly
one `Start` + one `End`, nothing open before it) — traced through the
new scan: phase (a) finds the `End`, pops the pre-group `stack`
(empty) → `unclosed_ends = [end]`; phase (b) pushes the `Start`; phase
(c) pops the `End` against the just-pushed `Start` → zero-length
`Stint`. Matches spec §5.1's explicit statement that this case
"reduces to today's behavior."

### AC3 — `08:00 start`, tied `09:00 end`+`start` (spec §3.2 worked example, §5.1)

New test: `tied_end_and_start_closes_earlier_open_start`.

```rust
#[test]
fn tied_end_and_start_closes_earlier_open_start() {
    let d = classify(
        &[
            p(1, "08:00", Start),
            p(2, "09:00", End),
            p(3, "09:00", Start),
        ],
        at("09:30"),
    );

    assert_eq!(completed_spans(&d), vec![(at("08:00"), at("09:00"), 60)]);
    assert_eq!(d.open.len(), 1);
    assert_eq!(d.open[0].start.id, 3);
    assert_eq!(d.open[0].start.at_utc, at("09:00"));
    assert!(d.orphaned_ends.is_empty());
    assert!(!d.has_anomaly());
}
```

Asserts: one completed `08:00`–`09:00` stint (60 min), the `09:00
start` (id 3) is the sole trailing open entry, no anomaly — matching
spec §3.2's worked example and §5.1's first bullet exactly.

### AC4 — tied group of 2 `End`s + 1 `Start`, nothing open before it (spec §5.1)

New test: `tied_group_two_ends_one_start_nothing_open_before`.

```rust
#[test]
fn tied_group_two_ends_one_start_nothing_open_before() {
    let d = classify(
        &[
            p(1, "09:00", End),
            p(2, "09:00", End),
            p(3, "09:00", Start),
        ],
        at("09:30"),
    );

    // One End pairs zero-length with the group's own Start (id 3);
    // id ordering among the two Ends (phase-a scan order) determines
    // which specific End pairs vs. orphans — assert the invariant the
    // spec actually pins (one pairs, one orphans), not a specific id,
    // since spec §5.1 doesn't pin which of the two ties the knot.
    assert_eq!(d.completed.len(), 1);
    assert_eq!(d.completed[0].start.id, 3);
    assert_eq!(d.completed[0].minutes, 0);
    assert!(d.open.is_empty());
    assert_eq!(d.orphaned_ends.len(), 1);
    assert!(d.has_anomaly());
}
```

Note: trace through the algorithm to pin down which `End` actually
orphans, so the test can assert it exactly rather than hedging. Group
sorted by `(at_utc, kind, id)`: `[End(1), End(2), Start(3)]` (`Start`
sorts after `End`? — **check**: line 125's comment says "`Start` before
`End`", meaning `PunchKind::Start < PunchKind::End`, so within a tied
group the sort places `Start`s *before* `End`s, i.e. the actual sorted
order here is `[Start(3), End(1), End(2)]`). Re-derive: phase (a)
filters the group for `End`s in the group's `sorted`-relative order,
which — since all `Start`s precede all `End`s in the sort but
`.filter()` preserves relative order of the *matching* elements only
— yields `End(1)` then `End(2)`, both popping against the pre-group
`stack` (empty, nothing open before) → both land in `unclosed_ends`.
Phase (b) pushes `Start(3)`. Phase (c) pops `unclosed_ends` in order:
`End(1)` pops `Start(3)` → completed zero-length stint id
`3`-`1`; `End(2)` finds `stack` empty → orphaned. So the test above
should assert `d.completed[0].end.id == 1` and
`d.orphaned_ends[0].punch.id == 2` explicitly:

```rust
    assert_eq!(d.completed[0].end.id, 1);
    assert_eq!(d.orphaned_ends[0].punch.id, 2);
```

(fold this into the test body above rather than leaving it hedged).

### AC5 — `classify_at` exists with the pinned signature, exported, doc comment states panic-safety

No behavioral test; verified by compilation (signature matches the
interface contract exactly — same argument names/types/order/return
type) and by the doc comment text itself (reviewed, not asserted in a
test — Rust doc comments aren't runtime-checkable content). Optionally
a `cargo doc` build could be added to CI verification but that's
outside this file's scope.

### AC6 — §5.2 splice tests, full list with names

- **`splice_clean_prev_direction`**: `prev` has one open `Start`
  (e.g. `23:30` on day1), `day` (day2) has exactly one orphaned `End`
  at `00:45` as `day`'s first punch. Assert `classify_at(prev_punches,
  day2_punches, &[], now)` returns one completed stint on... **wait**:
  per spec §4.1, the completed stint belongs to `prev`'s (day1's) own
  view, not `day`'s. So this test must call `classify_at` centered on
  **day1** (`prev_punches = &[]`, `punches = day1_punches`,
  `next_punches = day2_punches`) and assert `day1`'s result has the
  completed stint and zero opens/anomalies. Name:
  `splice_next_direction_closes_open_into_completed`.
  ```rust
  #[test]
  fn splice_next_direction_closes_open_into_completed() {
      let day1 = date();
      let day2 = date().succ_opt().unwrap();
      let prev_punches: Vec<Punch> = vec![];
      let punches = vec![p_on(1, day1, "23:30", Start)];
      let next_punches = vec![p_on(2, day2, "00:45", End)];

      let d = classify_at(&prev_punches, &punches, &next_punches, at_on(day2, "01:00"));

      assert_eq!(d.completed.len(), 1);
      assert_eq!(d.completed[0].start.id, 1);
      assert_eq!(d.completed[0].end.id, 2);
      assert_eq!(d.completed[0].minutes, 75);
      assert!(d.open.is_empty());
      assert!(d.orphaned_ends.is_empty());
      assert!(!d.has_anomaly());
  }
  ```
- **`splice_prev_direction_clears_orphan_without_adding_stint`**:
  mirror case, centered on day2 — `day` (day2) has the orphaned `End`
  that gets absorbed by `prev`'s (day1's) open `Start`; assert day2's
  own view shows *zero* completed stints (the stint belongs to day1's
  view, produced by day1's own `classify_at` call, per spec §4.2 — "the
  stint itself is not added here") but *also* zero anomalies (the
  orphan is consumed).
  ```rust
  #[test]
  fn splice_prev_direction_clears_orphan_without_adding_stint() {
      let day1 = date();
      let day2 = date().succ_opt().unwrap();
      let prev_punches = vec![p_on(1, day1, "23:30", Start)];
      let punches = vec![p_on(2, day2, "00:45", End)];
      let next_punches: Vec<Punch> = vec![];

      let d = classify_at(&prev_punches, &punches, &next_punches, at_on(day2, "01:00"));

      assert!(d.completed.is_empty());
      assert!(d.open.is_empty());
      assert!(d.orphaned_ends.is_empty());
      assert!(!d.has_anomaly());
  }
  ```
- **`splice_both_directions_fire_independently`**: `day` both closes
  `prev`'s dangling open (day's first punch is the matching orphan) and
  itself dangles into `next` (its own last punch is an unmatched open
  start matching `next`'s first-punch orphan). Assert both resolve:
  `day`'s `orphaned_ends` loses the prev-side match, `day`'s `open`
  loses the next-side match and a new completed stint appears for the
  next-side pair, no anomaly.
- **`splice_prev_two_opens_stays_unspliced`** (E7 case): `prev.open.len()
  == 2`, `day` has one first-punch orphan — assert no splice (`day`'s
  orphan and `prev`'s two opens both stay exactly as plain `classify()`
  would report them independently — check by comparing against
  `classify(prev_punches, now)`/`classify(punches, now)` directly).
- **`splice_day_two_orphans_stays_unspliced`**: `day.orphaned_ends.len()
  == 2` (never coalesced, §4.3), `prev` has one open — no splice.
- **`splice_both_neighbors_empty_no_splice`**: `prev_punches = &[]`,
  `next_punches = &[]` — folded into AC8's regression sweep, but worth
  a dedicated assertion too that no splice fires when there's nothing
  to splice against (trivially true, but pins the "empty neighbor
  never causes a spurious close" property distinctly from "reduces to
  classify()'s output").
- **`splice_orphan_not_first_punch_of_day_stays_unspliced`** (spec
  §5.2's "day's only orphan is not its first punch" bullet, §4.1's
  worked counter-example verbatim): `prev` has one open `Start`, `day`
  = `[09:00 start, 10:00 end, 13:00 end]` (day's own `13:00` is its
  only orphan but punch index 0 under the sort is the `09:00 start`).
  Assert no splice: `prev`'s open stays open, `day`'s `13:00` orphan
  stays flagged, `day`'s `09:00`–`10:00` pair stays completed
  untouched.
- **`splice_genuine_gap_two_dates_out_no_splice`** (spec §4.1's
  no-skip-empty-days rule, §5.2): `prev` has an open trailing start,
  `punches` (the immediate next date) is empty, the *real* matching
  orphan sits on the date after that (never passed to this
  `classify_at` call at all, since `classify_at` only ever sees one day
  in each direction). Assert `prev`'s open stays open when
  `classify_at`'s three slices are `(prev_punches, &[], &[], now)` —
  i.e. `next_punches` empty here represents "the literal next day had
  nothing," which correctly fails to splice even though a real orphan
  exists two days out (that far-out date is simply never passed in, by
  construction — the test demonstrates the API can't even see it, not
  that it sees and rejects it).
- **AC7 (interaction cases, own subsection below) also belong in this
  §5.2 group** but are large enough to list separately.
- **`classify_at_empty_neighbors_matches_classify`** (also covers AC8):
  see below.
- **`classify_at_empty_punches_bordering_nonempty_neighbor_no_panic`**:
  see below (round 2 finding 1, AC9).

### AC7 — §3×§4 interaction cases (spec §5.2, two bullets)

- **`tied_end_start_at_next_boundary_never_reaches_orphan_check`**:
  `next = [00:00 end, 00:00 start, 08:00 end]` (as raw punches on
  day2, ids e.g. 10, 11, 12), `prev` has one trailing open on day1. Per
  §3.2, `next`'s own `classify()` (called internally by `classify_at`)
  resolves the tied `00:00 end`+`start` group with nothing open before
  it → zero-length pair between them (§3.2 step 3), leaving `08:00 end`
  as the sole entry in `next.orphaned_ends` — but that surviving orphan
  is `next`'s *second* real event chronologically at the punch level
  (the `08:00 end`), and critically the *first punch of the date* under
  the sort is the `00:00` `Start` (index 0 is `Start`, kind-before-id),
  not an `End` at all — so `splice_candidate`'s "orphan is punch index
  0" check fails outright (the surviving orphan's `id` doesn't match
  `sorted_later.first().id`), and no splice fires regardless of
  `orphaned_ends.len()`. Assert `prev`'s open stays open, unspliced,
  and that `next`'s own `08:00` orphan (as seen were `next` classified
  standalone) still shows up correctly in `day`'s next-position — wait,
  this test is centered so that `next_punches` is passed as the
  `next_punches` argument to a `classify_at` call for day1 (`prev` =
  `&[]`, `punches` = day1, `next_punches` = the tied-group day). Assert
  `d.open.len() == 1` (unspliced) after the call.
- **`tied_group_of_ends_at_next_boundary_reaches_orphan_check`**
  (mirror, no `Start` in the tied group): `next = [00:00 end, 00:00
  end, 08:00 start]` — nothing open before the tied group on `next`'s
  own timeline, so phase (a)/(c) of `next`'s internal `classify()`
  leaves exactly one of the two `00:00 end`s as a genuine orphan at
  punch index 0 (the other zero-pairs against `next`'s own `08:00
  start`... wait, `08:00` sorts after `00:00`, so the `08:00 start`
  isn't in the same tied group — re-derive: group 1 = `[00:00 end,
  00:00 end]` (no `Start` in this group), nothing open before →
  phase(a) both fail to close (stack empty) → `unclosed_ends = [both]`
  → phase (b) pushes nothing (no `Start`s in group) → phase(c) both
  still fail (stack still empty) → both orphaned. That makes
  `orphaned_ends.len() == 2`, not 1 — which means the splice gate
  (`orphaned_ends.len() == 1`) correctly fails for a *different*
  reason than the first test. Assert this explicitly: `d.open.len() ==
  1` (unspliced) and separately (via a direct `classify(next_punches,
  now)` call in the test body) confirm `orphaned_ends.len() == 2` to
  pin down *why* it doesn't splice, matching spec §5.2's explicit ask
  to "assert the multi-end case where it doesn't [splice]." (If a
  single-orphan variant is wanted to more precisely test "reaches the
  orphan check, but at index 0" per the spec's literal wording, use
  `next = [00:00 end, 08:00 start]` instead — one `End` alone in its
  own group, nothing open before it, group size 1 → ordinary orphan at
  index 0 → *this* variant DOES become splice-eligible on the
  index-0/count-1 gate, and a third test,
  `single_end_at_next_first_instant_is_splice_eligible`, should assert
  it actually splices when `prev` has exactly one open — this is the
  positive control the spec's "correctly eligible for the splice
  check" phrase calls for, distinct from the multi-end negative
  control.)

### AC8 — empty-neighbors regression sweep (spec §5.2, §5.1 "direct regression requirement")

New test: `classify_at_empty_neighbors_matches_classify`, parametrized
over the same fixture list already used by
`has_anomaly_matches_anomalies_nonempty` (lines 598-611) plus a couple
more (the tied-group and nested-entry fixtures), asserting
`classify_at(&[], punches, &[], now) == classify(punches, now)` for
each — this requires `DayStints: PartialEq` (already derived, line 56)
and `Stint`/`OpenStint`/`OrphanedEnd: PartialEq` (all already derived).

```rust
#[test]
fn classify_at_empty_neighbors_matches_classify() {
    let cases: Vec<Vec<Punch>> = vec![
        vec![],
        vec![p(1, "09:00", Start)],
        vec![p(1, "09:00", Start), p(2, "17:00", End)],
        vec![p(1, "09:00", Start), p(2, "11:00", Start)],
        vec![p(1, "18:00", End)],
        vec![p(1, "09:00", Start), p(2, "09:00", End)],
        vec![p(1, "08:00", Start), p(2, "09:00", End), p(3, "09:00", Start)],
        vec![
            p(1, "09:00", Start),
            p(2, "10:00", Start),
            p(3, "11:00", End),
            p(4, "12:00", End),
        ],
    ];

    for punches in cases {
        let direct = classify(&punches, at("19:00"));
        let via_at = classify_at(&[], &punches, &[], at("19:00"));
        assert_eq!(direct, via_at, "mismatch for {punches:?}");
    }
}
```

### AC9 — empty `punches` bordering a non-empty neighbor, no panic (round 2 finding 1)

New test: `classify_at_empty_punches_bordering_nonempty_neighbor_no_panic`.

```rust
#[test]
fn classify_at_empty_punches_bordering_nonempty_neighbor_no_panic() {
    let prev_punches = vec![p(1, "09:00", Start)]; // trailing open on prev
    let punches: Vec<Punch> = vec![];
    let next_punches: Vec<Punch> = vec![];

    // Must not panic (this is the assertion — a panic fails the test
    // by itself; no unwrap/catch_unwind needed since the whole point
    // is that the debug_assert guards never fire and no [0]-index is
    // ever taken).
    let d = classify_at(&prev_punches, &punches, &next_punches, at("10:00"));

    assert!(d.completed.is_empty());
    assert!(d.open.is_empty());
    assert!(d.orphaned_ends.is_empty());
    assert!(!d.has_anomaly());
}

#[test]
fn classify_at_empty_punches_bordering_nonempty_next_no_panic() {
    let prev_punches: Vec<Punch> = vec![];
    let punches: Vec<Punch> = vec![];
    let next_punches = vec![p(1, "08:00", End)]; // orphan on next

    let d = classify_at(&prev_punches, &punches, &next_punches, at("10:00"));

    assert!(d.completed.is_empty());
    assert!(d.open.is_empty());
    assert!(d.orphaned_ends.is_empty());
    assert!(!d.has_anomaly());
}
```

Run this pair under `debug_assertions` (the default `cargo test`
profile already has them on) — that's precisely where an unguarded
`punches[0]` would panic, so a green `cargo test` on these two is the
actual proof of round 2 finding 1's fix, not just an inspection of the
guard code.

### AC10 — `cargo test`/`cargo clippy` clean for `src/stint.rs` in isolation

Verification steps, not new tests:
- `cargo test --lib` (or targeted `cargo test stint::`) — all tests
  above, plus every pre-existing test, green.
- `cargo clippy --all-targets -- -D warnings` — matches CI's exact
  invocation (`.github/workflows/ci.yml` line 50, minus the
  `--target` matrix flag which doesn't affect lint output). Per the
  cognitive-complexity notes in Parts 1 and 2 above, this is the step
  that determines whether either new function needs splitting.
- `cargo fmt --check` — the pre-commit hook's first gate; run `cargo
  fmt` (not `--check`) before committing to auto-fix.
- The pre-commit hook's coverage-regression check
  (`cargo-llvm-cov` against `coverage-baseline.json`) is expected to
  *improve* here (new tests, new covered branches) — not a risk, but
  worth noting the hook auto-ratchets the baseline upward on
  improvement and never needs a manual bump for this task.

## Risks, ambiguities, and disagreements

1. **Cognitive complexity is a real, not hypothetical, risk for both
   new/changed functions.** `clippy.toml`'s threshold (15) is stricter
   than clippy's default (25), and CI enforces `-D warnings` crate-wide
   — a cognitive-complexity violation is a hard CI failure, not a
   style nit. Neither the changeset plan nor the spec mentions this
   constraint at all, despite it being a project-level convention that
   directly constrains how the §3.2 scan and `classify_at`'s body must
   be shaped. This plan flags the extraction fallback (private helper
   functions) in both Part 1 and Part 2, but the actual pass/fail can
   only be known once the code is written and clippy is run — it is
   not something this plan can resolve on paper. Implementer should
   run `cargo clippy --all-targets -- -D warnings` early and often
   during Task 1, not just at the end.

2. **The `has_anomaly` private-field question, resolved but worth
   restating plainly per the task brief's explicit ask**: no
   restructuring of `DayStints` is needed. `classify_at` living in the
   same module as `classify()` gets ordinary private-field write access
   to `has_anomaly`, exactly like `classify()`'s own struct literal
   does. The changeset plan and spec are silent on this mechanism
   specifically — they only say "has_anomaly is recomputed" (spec
   §4.2) without saying *how*, given the field's privacy. This plan
   treats that silence as a gap the spec understated slightly (it's a
   one-line fact, not a design decision, but a careful implementer
   without this file could easily have gone looking for a nonexistent
   public setter or considered adding one unnecessarily).

3. **Ambiguity in spec §5.2's "one End pairs zero-length with the
   group's own Start, the other becomes an ordinary orphaned-End
   anomaly" (the tied-2-ends-1-start test)**: the spec doesn't say
   *which* of the two same-instant `End`s pairs vs. orphans — this plan
   worked through the algorithm by hand (see AC4 above) and pins it to
   "the `End` with the lower `id`, by construction of the stable sort
   and the filter's order-preservation" and writes the test to assert
   that specific outcome rather than leaving it a loose "one of the
   two" assertion, since a test that doesn't pin an exact result isn't
   really testing the tie-break's determinism (determinism-by-`id` is
   explicitly load-bearing elsewhere in this file, e.g. the sort key
   itself and the "entry order does not change result" test at line
   288). If the actual implementation's traced-through order differs
   from this plan's hand trace (easy to get wrong reasoning about
   `.filter()` over a slice with a non-uniform kind distribution),
   the implementer must re-derive it against the real code rather than
   trust this plan's arithmetic blindly — flagging this as a place
   worth double-checking by hand once the code exists, not just
   copy-pasting the test as written here.

4. **The §5.2 "next's first instant is a tied group" test fixtures
   required hand-tracing through the new algorithm to get right** (see
   AC7 above) — the spec's own prose description of the multi-end
   mirror case is correct but terse enough that a naive test author
   could easily write a fixture that doesn't actually exercise the
   claimed shape (e.g. accidentally using a fixture where the "genuine
   orphan... correctly eligible for the splice check" sub-case never
   gets its own positive-control test, only the multi-end negative
   control the spec explicitly asks for). This plan adds a third test,
   `single_end_at_next_first_instant_is_splice_eligible`, beyond
   spec §5.2's literal two bullets, because a negative control alone
   doesn't prove the boundary between "reaches the check" and
   "actually splices" is drawn where the spec claims — this is a
   judgment call, flagged as an addition beyond the letter of §5.2,
   not a deviation from it.

5. **No disagreement with the changeset plan's or spec's substance** —
   both are unusually precise for stint.rs's scope (worked examples,
   exact test names for the two must-pass-unmodified tests, an exact
   pinned signature). The one process gap is purely the
   cognitive-complexity/clippy-config fact neither document mentions
   (risk 1 above), which this codebase's own tooling enforces
   independently of anything spec/plan says.

6. **`splice_candidate`'s helper design went through one dead-end in
   this plan itself** (the first sketch called `classify()` a second,
   redundant time and read the system clock, violating the module's
   own no-clock-reads contract) — flagging this not as a risk in the
   shipped code (the final sketch fixes it) but as a note that the
   "operate on already-computed `DayStints`, take the raw punch slice
   only for the ordering check" shape is the one to implement, not the
   first draft above it in this document. Both drafts are left in this
   plan deliberately, in order, so the implementer sees why the second
   shape was chosen rather than just being handed the answer.
