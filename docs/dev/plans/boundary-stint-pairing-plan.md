# Plan: boundary stint pairing

> **Archived — historical planning record.** Not authoritative. Current spec: `docs/dev/SPEC.md`. Design/process history: `docs/dev/NOTES.md`.


**Spec**: `docs/dev/specs/2026-09-19-boundary-stint-pairing.md` (locked,
two adversarial review rounds folded in). This plan is scope-of-work
only — no algorithm detail beyond what the spec already pins.

## Goal

Fix two related, previously-deferred defects in stint pairing
(`src/stint.rs::classify`, SPEC.md §4.3): a same-instant `stop`/`start`
boundary that currently zero-pairs and silently drops a real stint
instead of closing the `start` that was already open, and a
midnight-spanning session that currently renders as two disconnected,
partially-invisible halves instead of one clean stint. Both are fixed
by (1) reordering `classify()`'s internal same-instant scan so a tied
`end` preferentially closes an earlier open `start` before pairing
with a same-instant `start`, and (2) a new `classify_at` wrapper that
looks one calendar day in each direction and splices an unambiguous
(1:1, first-punch) boundary pair into one completed stint — landing on
the day it started. Anything with more than one plausible reading
stays exactly as flagged today; this is a mechanical fix, not a new
heuristic.

## Architecture (prose)

`classify()` itself stays the pure, single-date primitive; its public
signature is untouched, and its internal step-2 scan changes shape
(process same-instant punches as a group, letting an `end` reach
backward into the carried-in stack before same-group `start`s are
pushed) rather than as a flat linear scan. This is a strict
generalization — every non-tied group reduces to today's exact
behavior — so it carries no migration risk for the bulk of existing
data or tests.

A new function, `classify_at`, sits one layer above `classify()` and
owns the cross-date concern entirely: it classifies three adjacent
dates independently and stitches together at most two boundary pairs
(yesterday→today, today→tomorrow) under a strict 1:1-or-nothing gate.
It is deliberately *not* a new pairing algorithm — it's `classify()`
called three times plus a narrow, mechanical post-processing rule.
Every current call site that classifies one calendar date in a context
where "yesterday" and "tomorrow" are meaningful concepts (i.e., all
four of them) moves from `classify()` to `classify_at`, fetching one
extra day of punches on each side. This is why the change touches four
call sites across two files in addition to the algorithm itself: the
fix isn't complete until every place that currently produces a
per-date view is capable of seeing across the boundary.

One call site — `DbWeekData::worked_minutes` — requires more than a
wider fetch: its current minutes sum iterates every bucket a `HashMap`
happens to contain, which is only safe today because the fetched range
is scoped exactly to the week. Widening that fetch without also fixing
the iteration to walk the week's own 7 dates explicitly would leak an
adjacent week's padding-day stint into this week's total and
double-count it in the neighbor's own total. This is a required
correctness fix riding alongside the widening, not an optional
cleanup — round 2 review flagged it as a defect, not a style
preference.

## Global constraints (every task must hold to)

1. `classify(punches: &[Punch], now: DateTime<Utc>) -> DayStints` keeps
   its exact signature (spec §3.3). Only its internal step-2 scan
   changes; nothing outside the function's body changes, and no
   existing non-tied-group test may be edited to pass.
2. Pairing (both the tie-break and the splice) never looks further
   than the literal adjacent calendar date. No "skip empty days and
   find the next date with data" logic anywhere (spec §2, §4.1).
3. A spliced boundary stint is a normal completed stint once resolved
   — no new anomaly kind, no new render marker (spec §2, §7).
4. `classify_at`'s debug-assert self-checks must never index
   `punches[0]`, `prev_punches[0]`, or `next_punches[0]` unguarded —
   every such comparison is gated on `.first()`/an empty check first.
   An empty `punches` bordering a non-empty neighbor is the ordinary
   shape for every idle calendar date once `classify_at` replaces
   `classify()` everywhere, not a rare edge case, and it must never
   panic in debug or release (spec §4.2, round 2 finding 1 — this is
   the one correctness property every reviewer must re-verify by
   hand, not by reading the prose).
5. `DbWeekData::worked_minutes` must sum over `week.dates()` (the same
   fixed 7-element pattern `build_rows` already uses), never over
   `by_date.values()` or any other structure that could contain a
   padding day pulled in by the widened fetch (spec §4.3, round 2
   finding 2).
6. Date arithmetic at every call site uses `NaiveDate::pred_opt()` /
   `succ_opt()` (never the panicking `pred()`/`succ()`, never `-`/`+
   Days`), matching the existing overflow discipline (spec §4.3).
7. No signature change to any of the four call sites' own public
   functions beyond what's needed to thread the two extra punch
   fetches through — this is a wiring change, not a refactor.

## Ordering and parallelization

This splits into two tasks with a real, not artificial, dependency:

- **Task 1 — `src/stint.rs`**: the tie-break reorder (spec §3) and the
  new `classify_at` function (spec §4.1, §4.2), entirely self-contained
  in one file plus its own test module.
- **Task 2 — call-site rewiring**: `src/status.rs` and
  `src/week_view.rs` (spec §4.3, §4.4), switching all four call sites
  from `classify()` to `classify_at`.

Task 2 calls `classify_at` by its exact shipped signature and relies
on its panic-safety contract (constraint 4) — it cannot be written,
let alone compiled, until Task 1's `classify_at` exists with that
signature. This is a genuine sequential dependency, not a scheduling
convenience: **Task 2 depends on Task 1's shipped `classify_at`.** Do
not parallelize them. Task 1 is the critical path; Task 2 starts only
after Task 1 lands and its own tests (§5.1/§5.2 below) pass.

Dependency graph:

```
Task 1 (src/stint.rs: tie-break + classify_at)
        │
        ▼
Task 2 (src/status.rs, src/week_view.rs: call-site rewiring)
```

No third task: doc updates (SPEC.md, NOTES.md, README.md) are owned by
the coordinator post-verification per standing convention (see below),
and the one piece of code-adjacent doc work that *isn't* coordinator
territory — `week_view.rs`'s stale "out of scope (E15)" doc comment on
`build_rows` (spec §4.4) — is in-repo source code, not one of the
three coordinator-owned docs, and sits in a file Task 2 already owns
exclusively. It's folded into Task 2, not split out.

## Interface contract to pin before either task starts

`classify_at`'s signature and panic-safety contract (spec §4.2) is the
one thing Task 2 calls without re-deriving:

```rust
pub fn classify_at(
    prev_punches: &[Punch],
    punches: &[Punch],
    next_punches: &[Punch],
    now: DateTime<Utc>,
) -> DayStints
```

- `prev_punches`/`next_punches` are the literal adjacent calendar
  dates' punches (§2: never further out); pass `&[]` when a neighbor
  has no data.
- Never panics, for any combination of empty/non-empty
  `prev_punches`/`punches`/`next_punches` — including `punches` empty
  with a non-empty neighbor, which is the common idle-day case, not an
  edge case (constraint 4).
- With both neighbors empty, reduces exactly to `classify(punches,
  now)`'s output (spec §5.2's direct regression requirement) — this is
  the property Task 2's callers implicitly lean on for every date that
  has no real boundary condition to resolve.
- `classify()` itself remains public, unchanged, and is no longer
  called from any of the four production call sites after Task 2
  lands (test code may still call it directly where it's exercising
  the pure single-date primitive).

## Task 1 — `src/stint.rs`: tie-break reorder + `classify_at`

**Spec citations**: §3 (tie-break fix), §4.1 (splice rule), §4.2 (API
shape and panic-safety contract), §5.1, §5.2 (test plans).

**Owns exclusively**: `src/stint.rs` (including its `#[cfg(test)]`
module).

**Acceptance criteria**:

- `classify()`'s public signature is byte-for-byte unchanged; every
  existing non-tied-group test in `src/stint.rs` passes unmodified,
  with no test edits.
- The two existing same-instant tests
  (`same_instant_pair_is_a_legal_zero_length_stint`,
  `same_instant_pair_still_pairs_when_end_entered_first`) pass
  unmodified (spec §5.1: they're the "nothing open before the tie"
  case, which the fix explicitly reduces to today's behavior).
- New test: `08:00 start`, tied `09:00 end`+`start` → one completed
  `08:00`–`09:00` stint, the `09:00 start` the sole trailing open
  entry, no anomaly (spec §3.2's worked example, §5.1).
- New test: tied group of 2 `end`s + 1 `start`, nothing open before it
  → one zero-length pair against the group's own `start`, the other
  `end` an ordinary orphaned-end anomaly (spec §5.1).
- `classify_at` exists with exactly the signature in the interface
  contract above, is exported from `src/stint.rs`, and its doc comment
  states the panic-safety contract explicitly.
- New tests per spec §5.2, all present and passing: clean splice in
  each direction; both directions firing on the same day at once;
  every non-1:1 shape (2+ opens, 2+ orphans, both-empty neighbors)
  stays unspliced; the "orphan is not punch index 0" rejection case;
  the genuine-gap case (real orphan two dates out, no splice); the
  empty-neighbors-reduces-to-`classify()` regression sweep; the §3×§4
  boundary-interaction case (tied `End`+`Start` at `next`'s first
  instant never reaches the orphan check; tied group of 2+ `End`s at
  `next`'s first instant does, but only becomes an orphan when
  `orphaned_ends.len()` is still exactly 1 overall); the
  empty-`punches`-bordering-a-non-empty-neighbor no-panic case (round
  2 finding 1).
- `cargo test` and `cargo clippy` clean for `src/stint.rs` in
  isolation (the crate as a whole will not yet compile end-to-end
  until Task 2 lands, since `classify_at` has no caller yet — that's
  expected and not a Task 1 blocker).

**Out of scope**: touching `src/status.rs` or `src/week_view.rs` in
any way (including adding `classify_at` calls there — that's Task 2);
any splice spanning more than one calendar day; a new anomaly kind or
render marker for a spliced stint (spec §7).

## Task 2 — call-site rewiring: `src/status.rs`, `src/week_view.rs`

**Depends on**: Task 1 shipped and merged (`classify_at` exists with
the pinned signature and passes its own tests).

**Spec citations**: §4.3 (all four call sites, exact), §4.4 (stale doc
comment), §5.3 (integration test plan).

**Owns exclusively**: `src/status.rs`, `src/week_view.rs` (including
their `#[cfg(test)]` modules).

**Acceptance criteria**:

- `src/status.rs::resolve` (currently line ~339): fetches
  `target_date.pred_opt()`/`.succ_opt()` punches alongside the
  existing fetch, calls `classify_at` with all three.
- `src/status.rs::build_ledger` (currently lines ~291-305, the
  day-by-day walk): fetches the previous/next calendar date's punches
  at each iteration, calls `classify_at`. No rolling-window
  optimization (spec's explicit call — not worth the bookkeeping).
- `src/week_view.rs::build_rows`/`build_week_view` (currently lines
  ~118-157/~205-236): the punch fetch in `build_week_view` widens to
  `start.pred_opt().unwrap()`..`end.succ_opt().unwrap()`; `build_rows`
  looks up each date's neighbors in the same `by_date` map it already
  builds, defaulting to `&[]` via the existing
  `.unwrap_or_default()` pattern.
- `src/week_view.rs::DbWeekData::worked_minutes` (currently lines
  ~182-193): fetch widens the same way; the sum is rewritten to
  iterate `week.dates()` explicitly and call `classify_at` per date
  with that date's neighbors pulled from the padded map — the current
  `by_date.values().map(...).sum()` pattern is fully removed, not
  left alongside the fix (constraint 5, round 2 finding 2 — this is
  the one line in this task most likely to be gotten wrong by pattern-
  matching the old code instead of replacing it).
- `build_rows`'s stale doc comment ("one call per date, never a whole
  week's punches in one call, which is what keeps cross-midnight
  pairing out of scope (E15)") is rewritten to describe the
  padded-neighbor lookup instead (spec §4.4). A grep of both files for
  any other comment asserting cross-midnight/E15 is out of scope is
  done and any hit fixed the same way.
- All `NaiveDate` arithmetic at every new call site uses
  `pred_opt()`/`succ_opt()` with `.unwrap()`, never `pred()`/`succ()`
  or `Days` arithmetic (constraint 6).
- New/updated tests per spec §5.3, all present and passing: `status`
  on each half of a midnight-spanning pair shows the correct
  completed/no-anomaly state; `week`'s per-day and week-aggregate
  totals include the spliced stint's minutes on the correct
  (starting) day only; a splice landing exactly on a week boundary
  attributes minutes to the correct week only; the
  `worked_minutes`-padding-day-isolation case (round 2 finding 2:
  a real, unrelated stint on the day just outside the week's own 7
  dates but inside the widened 9-day fetch appears in only its own
  week's figures, never double-counted or leaked into the neighbor).
  `c7_never_touched_week_carries_forward`/
  `c8_multi_week_idle_gap_carry_end_to_end` (`src/week_view.rs:851-887`)
  are confirmed still passing but are not sufficient on their own for
  this case (they seed fully-idle padding weeks) — a new fixture with
  a real padding-day stint is required.
- CI e2e smoke (`.github/workflows/ci.yml`): a scripted `start`/`stop`
  pair straddling midnight via two `-d`/`--date`-backed punches,
  asserting `status` on the start date shows the completed total and
  `status` on the end date shows no anomaly.
- `cargo test`, `cargo clippy`, and the full CI e2e suite pass for the
  crate as a whole (this is the task that makes the crate compile
  end-to-end again with `classify_at` in use).

**Out of scope**: any change to `src/stint.rs` itself (Task 1's file,
already shipped by the time this task starts); editing SPEC.md,
NOTES.md, or README.md (coordinator-owned, see below); any change to
`week`'s or `status`'s accounting model beyond swapping which
`classify`/`classify_at` call feeds it (spec §7).

## Documentation ownership (explicit, not left ambiguous)

Per standing project convention, the coordinator — not any task —
directly edits `docs/dev/SPEC.md`, `README.md`, and `docs/dev/NOTES.md`
after Task 2 lands and is verified (spec §6):

- SPEC.md §1.2: remove the "Stints spanning midnight" and
  "Same-instant `end`/`start` boundary" non-goal bullets.
- SPEC.md §4.3: rewrite the tie-break description, replace the "known
  defect, deferred" callout, add the splice-rule paragraph including
  the §4.1 residual-case caveat (a legitimate carried-over orphan
  sharing its date with an unrelated second orphan still doesn't
  splice).
- NOTES.md: append a decision entry for the two-phase tied-group scan
  and the neighbor-padded `classify_at` shape (decisions 55-57 already
  record this spec's design history and adversarial-review findings;
  this new entry records the as-shipped implementation, once it's
  landed and verified — it does not duplicate 55-57).
- README.md "### Known limitations" (lines 44-50): remove both
  bullets, or the section entirely if nothing else populates it.

The one code-level doc item that is *not* coordinator territory is
`week_view.rs`'s stale `build_rows` comment (spec §4.4) — that's a
source-code comment inside a file Task 2 already owns and edits, not
one of the three coordinator-owned documents, so it's explicitly
folded into Task 2's acceptance criteria above. No task touches
SPEC.md, README.md, or NOTES.md.

## File ownership summary (no two tasks share a file)

| File | Owner |
|---|---|
| `src/stint.rs` | Task 1 |
| `src/status.rs` | Task 2 |
| `src/week_view.rs` | Task 2 |
| `docs/dev/SPEC.md` | coordinator (post-verification) |
| `README.md` | coordinator (post-verification) |
| `docs/dev/NOTES.md` | coordinator (post-verification) |
