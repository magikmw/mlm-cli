# Boundary stint pairing — fix pass report

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.


Small, bounded fix pass addressing findings 3, 4, and 5 from
`docs/dev/plans/reports/final-review.md`, plus one stale comment found
during a follow-up `SPEC.md` doc sweep (originally flagged in finding 2
but not itself fixed there).

## Worktree base

Worktree started stale relative to `boundary-stint-pairing`'s tip (it was
sitting on an older `Release v0.3.2` history). Rebased cleanly onto
`boundary-stint-pairing` (`aa83dff`) before starting; no conflicts, no
local commits were dropped.

## Changes

### 1. `.github/workflows/ci.yml` (final review finding 3)

`grep -qE "^\s*00:45"` used the GNU-only `\s` extension, which BSD grep
(the macOS runners) reads literally — turning the pattern into
`^s*00:45` and making the check vacuously pass on those runners.

Simplified to `grep -q "00:45"`, matching the surrounding smoke block's
own style for negative-absence checks (e.g. the adjacent
`grep -q '\[!\]' ...` checks two lines above use plain substring
matching, not an anchored character class). The check's intent — "the
string `00:45` should not appear anywhere in this date's status output"
— doesn't need the `^` anchor or whitespace class at all, since `00:45`
never appears elsewhere in that output.

Validated well-formedness with a YAML parse (`python3 -c "import yaml;
yaml.safe_load(open('.github/workflows/ci.yml'))"` — OK). Did not run
the workflow.

### 2. `src/stint.rs` — `splice_candidate` (final review finding 4)

- Changed return type from `Option<usize>` (always either `None` or
  `Some(0)`) to `bool`. Updated both call sites in `classify_at`
  (`if splice_candidate(...).is_some()` → `if splice_candidate(...)`)
  and the function's doc comment.
- Replaced the clone-and-fully-sort of `later_punches` (just to read its
  minimum by `(at_utc, kind, id)`) with
  `later_punches.iter().min_by_key(|q| (q.at_utc, q.kind, q.id))` —
  allocation-free, O(n) instead of O(n log n). Preserved the empty-slice
  behavior (`None` from `min_by_key` on an empty slice still yields
  "no splice", matching the old `sorted_later.first()?` early return).

Read the function fully before touching it, per the instruction that
this logic is load-bearing for the boundary-splice correctness gate. The
gating logic itself (`earlier_open_count != 1 || later.orphaned_ends.len()
!= 1`, then compare `first.id` to the orphan's `punch.id`) is unchanged;
only the return-type ceremony and the sort-vs-min strategy changed.

### 3. `src/stint.rs` — `has_anomaly` duplication (final review finding 5)

Added a private associated function `DayStints::compute_has_anomaly(open:
&[OpenStint], orphaned_ends: &[OrphanedEnd]) -> bool` encoding the one
definition of "has an anomaly" (`open.len() > 1 ||
!orphaned_ends.is_empty()`). Both `classify()`'s initial computation and
`classify_at()`'s post-splice recomputation now call it, instead of each
carrying its own copy of the expression.

### 4. `src/stint.rs:788` — stale T10 comment (doc-sweep finding)

Old text:

```rust
// --- T10: cross-midnight is two separate dates (accepted limitation) -
```

The "accepted limitation" framing predates `classify_at` (added by this
same changeset), which now resolves cross-midnight pairing via the
boundary-splice rule — so calling it an accepted limitation is no longer
accurate at the crate level. The T10 test itself (`
cross_midnight_halves_are_two_separate_anomalies`) is still correct and
unchanged: it exercises plain `classify()` on each date in isolation,
whose per-date behavior (an open stint on day 1, an orphaned end on day
2, each un-spliced) genuinely doesn't change under this changeset. Reworded
to make that scope explicit and point at `classify_at`'s own tests for
the actual cross-date resolution:

```rust
// --- T10: plain classify() treats cross-midnight halves as two
// independent dates, each with its own anomaly signal in isolation.
// Resolving them into one splice pair is classify_at()'s job (see its
// own tests below); this test only pins classify()'s per-date behavior,
// unchanged by this changeset. ---
```

## Verification

- `cargo fmt --check` — clean, no diff.
- `cargo build` — exit 0.
- `cargo test` — 425 lib tests + 2 (main) + 1 (ascii_sweep) + 2
  (e6_db_failure) + 5 (status_cli) = 435 passed, 0 failed. Same total the
  final review recorded as baseline; no regression from the
  `splice_candidate`/`has_anomaly` refactor.
- `cargo clippy --all-targets -- -D warnings` — clean.
- YAML parse check on `.github/workflows/ci.yml` — OK.
- Local pre-commit quality gate (`.githooks/pre-commit`: fmt, cognitive
  complexity, coverage-vs-baseline) ran automatically on commit and
  passed.

## Deviations

- **`splice_candidate`'s "no first punch" path, reworked for the
  coverage gate.** First attempt used a `let Some(first) = ... else {
  return false; }` for the `min_by_key` result (mirroring the old
  `sorted_later.first()?` early return). The pre-commit coverage gate
  caught that this turned an always-unreachable branch (an orphaned end
  in `later.orphaned_ends` implies `later_punches` is non-empty, so
  `min_by_key` can never return `None` once the earlier guard passes)
  into its own never-executed source line, dropping line coverage from
  98.9556% to 98.9411% — a real regression the gate is designed to catch,
  not a flaky threshold. Rewrote as a single expression,
  `later_punches.iter().min_by_key(...).is_some_and(|first| ...)`,
  which keeps the dead branch as an inlined default rather than a
  standalone statement — same shape as the original `?`-operator version,
  and coverage came back exactly to baseline (98.9556135770235%,
  unchanged) on the second commit attempt. Behavior is identical to the
  `let-else` version; this was a coverage-instrumentation-shape issue,
  not a logic change.

Otherwise no deviations. All four fixes landed as scoped; no additional
findings surfaced during the pass beyond the stale-comment one already
named in the task.

## Status

Committed locally on top of the rebased `boundary-stint-pairing` tip.
Not merged, not pushed, per instructions.
