# Review round 2: 2026-09-19-boundary-stint-pairing.md

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.


Adversarial re-review against `v0.3.2` source (`src/stint.rs`, `src/status.rs`,
`src/week_view.rs`, `src/storage.rs`, `docs/dev/SPEC.md` §1.2/§4.3,
`README.md`, `docs/dev/NOTES.md` decisions 55-56), after round 1's 7
findings were folded into the spec text directly.

## Prior-findings verification

1. **Wrong citation ("NOTES.md decision 4")** — **addressed correctly**.
   Spec §2 (line ~33) now reads "Per SPEC.md §4.3's nearest-match (LIFO)
   algorithm: fix only what's mechanically forced..." — cites SPEC.md §4.3
   directly, not NOTES.md. `docs/dev/SPEC.md` §4.3 (lines 337-340, "Genuine
   nearest-match (LIFO) parentheses matching, per NOTES.md decision 4...")
   does describe the LIFO algorithm itself, so the new citation target is
   correct and supports the claim.

2. **"Three" vs four call sites** — **addressed correctly**. Spec §4.3
   (line 233) now reads "All four current `classify()` call sites move to
   `classify_at`" followed by the same four bullets (`resolve`,
   `build_ledger`, `build_week_view`, `DbWeekData::worked_minutes`). Count
   matches `grep -n "stint::classify(" src/status.rs src/week_view.rs`
   (status.rs:300,339; week_view.rs:136,191).

3. **README.md omitted from §6** — **addressed correctly**. Spec §6 (lines
   403-410) now has a fourth bullet covering `README.md`'s "### Known
   limitations" section (`README.md:44-50`), quoting both bullets verbatim
   and citing the `e2ad62b` precedent, matching round 1's ask exactly.

4. **Missing `classify_at` debug_assert** — **addressed incompletely, and
   the added text introduces a new bug**. Spec §4.2 (lines 217-229) does
   add self-check language, but as literally specified it will panic in
   debug/test builds on a routine, common input shape. See New Finding 1
   below — this is not a superficial gap, it's a load-bearing correctness
   defect in exactly the code round 1 asked for.

5. **No test for §3×§4 interaction at the boundary** — **addressed
   correctly**. Spec §5.2's final bullet ("§3 × §4 interaction, at the
   boundary itself", lines 343-358) now specifies both the tied
   `End`+`Start` case (zero-pairs internally, never reaches the orphan
   check) and the mirror tied-2-`End`s case, matching round 1's requested
   fixture and the by-hand trace round 1 did.

6. **Residual case not spelled out** — **addressed correctly**. Spec §4.1
   (lines 155-165, "Residual case worth naming") now states explicitly that
   a genuine carried-over orphan sharing its date with one unrelated stray
   orphan still fails the `orphaned_ends.len() == 1` gate and does not
   splice, and flags that §6's SPEC.md rewrite must state this rather than
   imply total coverage. §6 (lines 396-397) references it.

7. **`worked_minutes` perf justification gap** — **addressed correctly**.
   Spec §4.3's `worked_minutes` bullet (lines 254-267) now carries its own
   justification paragraph ("Same reasoning as `build_ledger` above
   applies... not worth a rolling-window optimization here either"),
   explicitly naming the multi-week walk-back tests
   (`c7_never_touched_week_carries_forward`,
   `c8_multi_week_idle_gap_carry_end_to_end`, `src/week_view.rs:851-887`) as
   evidence of call frequency. Confirmed those tests exist at exactly that
   location.

**Summary**: 6 of 7 addressed correctly; 1 (finding 4) addressed
incompletely — the fix as written creates a new, more severe defect than
the gap it was meant to close.

## New findings

### 1. `classify_at`'s prescribed date-consistency check indexes `punches[0]` unconditionally — will panic on the single most common input shape — **high** (error-handling gap / factual error re: Rust semantics)

- **Claim**: `docs/dev/specs/2026-09-19-boundary-stint-pairing.md:223-229` —
  "`classify_at` needs the equivalent self-checks: each of
  `prev_punches`/`next_punches` internally shares one date (same assertion
  as `classify()`, once per non-empty slice), and — since each slice
  already carries its own `date` field per punch — that
  `prev_punches[0].date == punches[0].date.pred_opt()` and
  `next_punches[0].date == punches[0].date.succ_opt()` **whenever those
  slices are non-empty**." (emphasis on the guard clause, which only
  covers `prev_punches`/`next_punches`.)
- **Evidence**: the guard ("whenever those slices are non-empty") only
  protects indexing into `prev_punches[0]`/`next_punches[0]`. It says
  nothing about `punches` (the day being classified) being non-empty, yet
  the right-hand side of both comparisons unconditionally indexes
  `punches[0]`. Plain slice indexing (`punches[0]`) panics on
  out-of-bounds in **both debug and release** builds — it is not gated by
  `debug_assert!`'s "compiled out in release" behavior at all, unlike
  `classify()`'s own precedent at `src/stint.rs:116-119`
  (`debug_assert!(punches.iter().all(|q| q.date == punches[0].date), ...)`),
  where `punches[0]` sits inside a closure passed to `.iter().all()` and is
  therefore never evaluated when `punches` is empty (short-circuits on the
  empty iterator before the closure runs once) — genuinely safe today.
  The new check copies the *look* of that precedent but not its safety
  property, because it indexes `punches[0]` directly in the assert
  condition rather than through a closure over `punches.iter()`.
  A day with zero punches bordering a non-empty neighbor is not a rare
  edge case — it is the default case for every non-worked calendar date
  (weekends, days off) once `classify_at` fully replaces `classify()` at
  all four call sites per §4.3: `build_ledger`'s day-by-day walk
  (`src/status.rs:291-305`) calls `classify_at` for literally every date
  in a week span regardless of whether that date has punches, and
  `DbWeekData::worked_minutes`'s per-week-date loop (§4.3's own bullet,
  lines 254-267) does the same for all 7 dates of every week walked. The
  very first idle day adjacent to a worked day in the test suite (run in
  debug profile by default via `cargo test`) would panic. This directly
  contradicts §2's design principle that `classify_at`'s splice logic
  should be a purely mechanical, conservative addition — a defensive
  check meant to catch caller bugs instead introduces a crash on
  legitimate, everyday data.
- **Fix required**: guard the `punches[0]` access the same way
  `classify()` does — either wrap the whole per-neighbor check in
  `if let Some(first) = punches.first() { ... }` (skipping the check
  entirely when `punches` is empty, which is fine: an empty `punches` with
  a non-empty neighbor can never actually violate the adjacency invariant
  in a way this check is meant to catch, since there's no `punches[0].date`
  to compare against), or restructure as a closure-based `.iter().all()`
  pattern mirroring `classify()`'s own style. §5.2's test plan should add
  a case exercising this exact shape (`punches` empty, one neighbor
  non-empty) to pin the fix down — it is currently untested in either
  direction.

### 2. `DbWeekData::worked_minutes`'s widened fetch risks leaking an unrelated padding day's own stints into the wrong week's total, and no test targets this — **medium-high** (testing gap / implementation-risk)

- **Claim**: spec §4.3's `worked_minutes` bullet
  (`docs/dev/specs/2026-09-19-boundary-stint-pairing.md:254-267`) specifies
  "fetch `punches_in_range(start.pred_opt().unwrap(), end.succ_opt().unwrap())`,
  bucket by date, and for each of the week's own 7 dates call `classify_at`
  with that date's neighbors pulled from the same map" — i.e., the sum
  must iterate the week's own 7 `NaiveDate`s specifically, not every key
  present in the bucket map.
- **Evidence**: the *current* implementation
  (`src/week_view.rs:182-193`) does exactly the wrong thing for the new,
  widened fetch: it sums over `by_date.values()` — every bucket the
  `HashMap` happens to contain — not over the week's own 7 dates:
  ```rust
  by_date.values()
      .map(|bucket| stint::classify(bucket, self.now_utc).completed_minutes())
      .sum()
  ```
  Today this is harmless because `punches_in_range(start, end)` is scoped
  exactly to the week's 7 dates, so `by_date` can only ever contain those
  7 keys. Once the fetch widens by one day on each side (9-day span,
  `start.pred_opt()`..`end.succ_opt()`), `by_date` will additionally
  contain up to 2 more buckets — the day before `start` and the day after
  `end`, both belonging to *adjacent weeks*. If the `.values().sum()`
  pattern is not deliberately replaced with an explicit iteration over
  `week.dates()` (as the spec's prose says but does not flag as a
  required, easy-to-miss behavior change from the existing code), any
  ordinary, unrelated completed stint on either padding day would be
  silently added to the current week's `worked_minutes` total — and, since
  the same padding day is *also* one of the 7 own-dates for the adjacent
  week's own `worked_minutes` call, its minutes would be double-counted
  across two weeks' totals.
  The existing regression tests that exercise `worked_minutes` across
  multiple weeks — `c7_never_touched_week_carries_forward` and
  `c8_multi_week_idle_gap_carry_end_to_end` (`src/week_view.rs:852-887`),
  which the spec itself cites as evidence for the perf note (round 1
  finding 7, now addressed) — do not catch this: both seed fully idle
  weeks around the seeded week (`week N+1 (2026-02) intentionally empty`,
  comment at `src/week_view.rs:874`), so the widened fetch's padding days
  are themselves empty in those fixtures and the leak never triggers. §5.3
  (lines 371-375) only tests that `build_week_view`'s padded-range fetch
  doesn't pollute **rows** ("without pulling that neighbor date's own
  unrelated stints into the wrong week's rows") — `build_rows` is
  structurally safe already (it only ever produces `WeekRow`s for the
  fixed 7-element `dates` array, `src/week_view.rs:133-134`), so that test
  exercises the code path that was never actually at risk. No test in §5.3
  targets `worked_minutes`/`week_accounting`'s figures (`worked`,
  `fulfillment`, `owed`) against a padding day carrying a real, unrelated
  stint — which is the one call site where the widening genuinely changes
  the iteration semantics needed for correctness.
- **Fix required**: §4.3's `worked_minutes` bullet should say explicitly
  that the existing `by_date.values().sum()` pattern must be replaced with
  iteration over `week.dates()` (matching `build_rows`'s existing
  per-date-array pattern), not merely describe the desired end
  behavior in prose. §5.3 should add a case: a week whose immediately
  preceding or following calendar date (just outside the week span, but
  inside the padded fetch) has its own ordinary, unrelated completed
  stint — assert that date's minutes appear in *only* its own week's
  `worked`/`fulfillment` figures, not the neighboring week's.

## Verdict

**needs-rework** — the algorithmic design in §3/§4 remains sound (this
round re-traced the tied-group scan, the splice gate, the "chronologically
first punch" rejection case, and the §3×§4 interaction case by hand against
`src/stint.rs` and found no defect beyond what's noted above), and 6 of
round 1's 7 findings were folded in cleanly. But round 1's finding 4 — the
missing `debug_assert!` — was "fixed" with text that, read literally,
crashes on the ordinary case of a punchless day next to a punched one
(New Finding 1), which is worse than having no check at all if implemented
as written, since it turns "defensive check that never fires in
production" into "runtime panic on routine data in every debug/test
build." Combined with New Finding 2 (a real double-counting/leak risk in
`worked_minutes` that no test in the plan would currently catch), this
spec should not move to implementation until §4.2's self-check text and
§4.3's `worked_minutes` bullet are corrected and §5.2/§5.3 gain the two
missing test cases named above.

VERDICT: findings(2)
FILE: docs/dev/plans/reports/boundary-stint-pairing-review-round2.md
1. [high] classify_at's prescribed debug_assert indexes punches[0] unconditionally, panicking (in debug/test builds, unconditional slice-index panic, not just debug_assert) whenever the day itself has zero punches and a neighbor is non-empty — the default shape for every idle day (spec:223-229 vs stint.rs:116-119's safe closure-based precedent).
2. [medium-high] worked_minutes's widened 9-day fetch risks leaking/double-counting an unrelated padding day's completed stint into the wrong week's worked total unless the current by_date.values().sum() pattern is explicitly replaced with iteration over week.dates(); no test in §5.3 exercises this (spec:254-267 vs week_view.rs:182-193, existing c7/c8 tests use fully-idle padding so don't catch it).
