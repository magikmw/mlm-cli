# Task 1 completion report: `src/stint.rs` — tie-break reorder + `classify_at`

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.


**Branch**: `worktree-agent-a735aac3dcc2ebf66`, rebased onto `boundary-stint-pairing`'s
tip (`d24e2de`) before starting — the worktree was created slightly behind
that branch (at `50e3e6a`, its parent), so a `git rebase boundary-stint-pairing`
was run first (fast-forward, no conflicts, no other change) to get onto the
current tip. Noting this explicitly per the pre-flight instruction.

**Plans/spec read in full**: `docs/dev/plans/boundary-stint-pairing-task-1-stint-algorithm.md`,
`docs/dev/plans/boundary-stint-pairing-plan.md`, `docs/dev/specs/2026-09-19-boundary-stint-pairing.md`
(§3, §4 especially, but read in full).

## What was implemented

1. **`classify()`'s step-2 scan reordered** (spec §3.2). The old flat
   `for punch in sorted { match punch.kind { ... } }` loop (previously
   `src/stint.rs:135-147`) is replaced with a group-at-a-time scan: `sorted`
   is walked in contiguous same-`at_utc` chunks, and each chunk is handed to
   a new private helper, `close_group`, which does the three-phase (a)
   close-against-pre-group-stack / (b) push-group-starts / (c)
   close-residual-ends-against-just-pushed-starts dance from spec §3.2.
   `classify()`'s public signature is unchanged. `close_group` was extracted
   as a private helper from the start (not added reactively after a clippy
   failure) — see "Deviation 1" below.
2. **`classify_at` added**, placed directly after `classify()`, exactly the
   pinned signature (see below), with five `debug_assert!` guards (three
   single-slice "shares one date" checks, two cross-slice adjacency checks),
   all gated on `.first()` so an empty `punches`/`prev_punches`/`next_punches`
   never triggers an unguarded `[0]` index in any build profile. A private
   helper `splice_candidate(earlier_open_count: usize, later: &DayStints,
   later_punches: &[Punch]) -> Option<usize>` implements the three-part §4.1
   gate (exactly one open, exactly one orphan, orphan is `later_punches`'s
   chronologically-first punch under the `(at_utc, kind, id)` sort) and is
   called twice — once for the `(prev, day)` pair, once for `(day, next)` —
   operating only on already-computed `DayStints` values (no redundant
   `classify()` call, no clock read), matching the plan's final corrected
   sketch rather than its flagged first-draft dead end.

## Test-first evidence (strict TDD)

- Part 1: added `tied_end_and_start_closes_earlier_open_start` and
  `tied_group_two_ends_one_start_nothing_open_before` to the existing test
  module *before* touching `classify()`'s body. Ran `cargo test --lib
  stint::` — `tied_end_and_start_closes_earlier_open_start` **failed**
  (`left: [(09:00, 09:00, 0)], right: [(08:00, 09:00, 60)]` — the exact old
  zero-pairing bug the spec describes), confirming the test actually
  exercises the defect. `tied_group_two_ends_one_start_nothing_open_before`
  passed even under the old code — traced by hand and confirmed correct:
  with nothing open before the tied group, the old flat scan and the new
  phased scan produce the same result for that specific fixture, so it's a
  valid regression test that happens not to discriminate old-vs-new
  behavior on its own (AC4's point still stands: the *other* new test does
  discriminate).
- Reordered `classify()`'s step 2 → both tests, plus all 21 pre-existing
  tests, green with zero edits to any pre-existing test.
- Part 2: added all `classify_at`/`splice_candidate` tests to the test
  module before `classify_at` existed at all. `cargo test --lib stint::`
  failed to **compile** (`cannot find function classify_at in this scope`,
  13 errors) — the TDD "watch it fail" step for a not-yet-existing function.
  Implemented `classify_at` + `splice_candidate`; all 36 `stint::` tests
  (23 pre-existing/Part-1 + 13 new `classify_at` tests) passed on the first
  attempt — every hand-trace in the test-writing step matched the shipped
  algorithm's actual output.

## Final test list (new, beyond the two Part-1 tests above)

- `splice_next_direction_closes_open_into_completed` — clean splice,
  `(day, next)` direction.
- `splice_prev_direction_clears_orphan_without_adding_stint` — clean splice,
  `(prev, day)` direction; confirms the stint does *not* appear in `day`'s
  own `completed`.
- `splice_both_directions_fire_independently` — both directions fire on the
  same centered date at once.
- `splice_prev_two_opens_stays_unspliced` — E7 on `prev` blocks the splice.
- `splice_day_two_orphans_stays_unspliced` — 2 orphans on `day` blocks it.
- `splice_orphan_not_first_punch_of_day_stays_unspliced` — orphan exists but
  isn't punch index 0.
- `splice_genuine_gap_two_dates_out_no_splice` — empty middle day, no
  "skip empty days" behavior.
- `classify_at_empty_neighbors_matches_classify` — regression sweep, 8
  fixtures, `classify_at(&[], p, &[], now) == classify(p, now)`.
- `classify_at_empty_punches_bordering_nonempty_prev_no_panic` /
  `..._nonempty_next_no_panic` — round-2 finding 1's no-panic property,
  both directions.
- `tied_end_start_at_next_boundary_never_reaches_orphan_check` — §3×§4
  interaction, tied `End`+`Start` at `next`'s first instant zero-pairs
  internally, never becomes a splice-eligible orphan.
- `tied_group_two_ends_at_next_boundary_multi_orphan_stays_unspliced` —
  mirror case, tied group of 2 `End`s, both become orphans,
  `orphaned_ends.len() == 2` blocks the gate (asserted directly via a
  separate `classify()` call in the test body, to pin down *why*).
- `single_end_at_next_first_instant_is_splice_eligible` — the positive
  control the task plan flagged as missing from the spec's literal two
  bullets: a lone `End` (no tie) at `next`'s first instant *does* reach and
  pass the splice gate.

All 13 added beyond Part 1, matching the task plan's AC5-AC9 list, with the
tied-group interaction tests' exact fixtures independently re-derived (not
copy-pasted from the plan) and confirmed correct against the shipped
`close_group` implementation.

## Deviations from the task plan, with reasons

1. **`close_group` extracted as a private helper from the start**, not
   inlined into `classify()` first and split out only if clippy flagged
   cognitive complexity. The task plan flagged this as a "build-time
   decision point, not a certainty" and suggested trying the inline sketch
   first. I judged that the inline version's shape (a `while` loop, a
   `.position()` closure, two `.filter()` closures, nested `match` arms) was
   obviously going to be at or near the threshold-15 ceiling, and that
   extracting a clearly-named helper up front is strictly better structure
   regardless of whether it was *required* to pass clippy — so I wrote it
   extracted from the first pass rather than deliberately writing then
   fixing an over-complex version. Confirmed after the fact:
   `cargo clippy --all-targets -- -D warnings` is clean with this shape, so
   the extraction was sufficient (and, in hindsight, was probably not
   strictly necessary to clear the threshold — but it costs nothing and
   documents the group-closing logic's contract with its own doc comment).
2. **`classify_at`'s body was written directly in its final (non-redundant,
   non-clock-reading) shape**, skipping the task plan's first draft
   (`splice_candidate` re-calling `classify()` on `later_punches` and
   reading `Utc::now()`). The plan itself flags that first draft as a
   documented dead end, not something to actually implement — so
   implementing only the corrected final sketch is not a deviation from the
   plan's *intent*, just skipping transcribing an intermediate step the plan
   itself says not to ship.
3. **No `apply_prev_splice`/`apply_next_splice` extraction** for
   `classify_at`'s two `if splice_candidate(...) { ... }` blocks — the plan
   flagged this as a possible fallback if `classify_at` tripped the
   cognitive-complexity lint. It didn't (`cargo clippy --all-targets -- -D
   warnings` is clean for the whole crate), so the two blocks stayed inline
   as written in the plan's final sketch.
4. **Test names**: `splice_next_direction_closes_open_into_completed`,
   `splice_prev_direction_clears_orphan_without_adding_stint`, and the two
   no-panic tests (`..._nonempty_prev_no_panic` /
   `..._nonempty_next_no_panic`) match the task plan's own naming exactly.
   The three §3×§4 interaction tests
   (`tied_end_start_at_next_boundary_never_reaches_orphan_check`,
   `tied_group_two_ends_at_next_boundary_multi_orphan_stays_unspliced`,
   `single_end_at_next_first_instant_is_splice_eligible`) use names I chose
   independently (close to, but not identical to, the plan's suggested
   names, e.g. the plan's draft name for the second one didn't settle on a
   final form in the plan text itself) — same coverage, same fixtures
   (re-derived, not copied), different exact identifier.
5. Everything else (signature, doc comment content, guard shape, splice
   logic, `has_anomaly` direct-field-mutate mechanism) matches the task
   plan's final sketches as written — no other substantive deviation.

## `classify_at`'s final public signature, verbatim

```rust
pub fn classify_at(
    prev_punches: &[Punch],
    punches: &[Punch],
    next_punches: &[Punch],
    now: DateTime<Utc>,
) -> DayStints
```

Placement order in the shipped `src/stint.rs` is: `classify()` →
`close_group` (classify's own helper) → `classify_at` → `splice_candidate`
(classify_at's own helper) → `#[cfg(test)] mod tests`. Exported (`pub fn`),
same module, no relocation.
Doc comment states the panic-safety contract explicitly (never panics for
any combination of empty/non-empty inputs; all self-checks gated on
`.first()`; reduces to `classify(punches, now)` with both neighbors empty)
— Task 2 can rely on this without re-deriving it.

## Verification output

- `cargo fmt --check`: initially failed on 5 spots in the newly-added test
  code (multi-line `vec![...]` literals rustfmt preferred to collapse or
  expand differently than I'd typed them); ran `cargo fmt` once, then
  `cargo fmt --check` passed clean. No hand-editing needed.
- `cargo build`: clean, no warnings (the crate-wide `cargo build` succeeds
  even with `classify_at` uncalled outside tests, thanks to the existing
  module-level `#![allow(dead_code)]` — matches the task plan's expectation
  that this wouldn't block Task 1, though in practice it isn't even a
  warning here, just silently allowed).
- `cargo test` (full crate, not just `stint::`): **423 tests total, all
  green** — 418 lib tests (36 in `stint::`, the rest untouched across the
  rest of the crate), 2 `main.rs` tests, 1 ascii-sweep integration test, 2
  `e6_db_failure` integration tests, 5 `status_cli` integration tests, 0
  doctests. Zero failures, zero pre-existing test edited.
- `cargo clippy --all-targets -- -D warnings`: clean, zero warnings,
  including the cognitive-complexity lint specifically — no function in
  `src/stint.rs` needed threshold-relaxation or further splitting beyond
  the `close_group`/`splice_candidate` extractions already made.
- Pre-commit hook (`.githooks/pre-commit`): ran automatically on `git
  commit`; passed (fmt gate, complexity gate, coverage gate all green — see
  commit output for exact per-gate lines if needed).

## Open questions for the reviewer / for Task 2

- None block Task 2. `classify_at`'s signature, panic-safety contract, and
  "reduces to `classify()` with empty neighbors" property are all shipped
  and tested exactly as the interface contract in the changeset plan pins
  them.
- One thing worth a reviewer's eye, not a defect: `splice_candidate`
  returns `Option<usize>` (always `Some(0)` or `None`) rather than a plain
  `bool`, kept from the task plan's own sketch for a reason that no longer
  fully applies once `classify_at` only ever calls `.is_some()` on the
  result and always removes index `0` itself rather than using the
  returned index. It's harmless (dead information in the `Some` case,
  documented in the doc comment as "always 0"), but a reviewer might
  reasonably ask why it isn't just `bool`. I kept `Option<usize>` because
  it mirrors the plan's pinned signature shape and reads slightly more
  intentionally at the call site (`is_some()` reads as "gate check passed"
  rather than a bare boolean), not because the `usize` payload is used for
  anything beyond documentation. Not a functional issue either way.
- The task plan's own §4 "Risks" section flagged the cognitive-complexity
  threshold as the one real open question on paper; it resolved cleanly in
  practice (clean `clippy -D warnings` with the extractions made from the
  start), so there's nothing outstanding there.
