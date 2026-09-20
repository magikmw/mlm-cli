# Review: 2026-09-19-boundary-stint-pairing.md

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.


Adversarial review against v0.3.2 source (`src/stint.rs`, `src/status.rs`,
`src/week_view.rs`, `src/storage.rs`, `docs/dev/SPEC.md`, `docs/dev/NOTES.md`,
`README.md`).

## Findings

### 1. Wrong citation: NOTES.md decision 4 does not say "nearest-match, not sequential pairing" — **high** (factual error / internal inconsistency)

- **Claim**: `docs/dev/specs/2026-09-19-boundary-stint-pairing.md:34` — "Per
  NOTES.md decision 4 (nearest-match, not sequential pairing): fix only
  what's mechanically forced..." — this is the stated justification for
  the whole §2 scope guardrail the rest of the spec leans on.
- **Evidence**: `docs/dev/NOTES.md:82-89`, decision 4 is titled "Stints
  from point-in-time punches, not stored ranges" and its body literally
  says the opposite of what's cited: "you insert a point (start or end)
  at any time value, points get sorted by time, then paired
  **sequentially** (start, end, start, end, ...) to produce stints."
  There is no other mention of "nearest-match" or "LIFO" anywhere in
  `NOTES.md` (`grep -n "nearest-match\|LIFO\|sequential" docs/dev/NOTES.md`
  returns only that one line). The actual nearest-match/LIFO algorithm
  lives in `SPEC.md §4.3` (`docs/dev/SPEC.md:337-340`) and
  `src/stint.rs:130-147`, not in a NOTES.md decision at all.
- Not fatal to the design itself (the scope-discipline argument in §2
  can stand on its own merits), but the spec misattributes its own
  rationale to a document section that says something different — an
  implementer or reviewer checking the citation will find it doesn't
  support the claim.

### 2. "All three current classify() call sites" — there are four, and the spec's own next paragraph lists four — **medium** (internal inconsistency / factual error)

- **Claim**: `docs/dev/specs/2026-09-19-boundary-stint-pairing.md:207` —
  "All three current `classify()` call sites move to `classify_at`,
  fetching one extra day of punches on each side:" followed immediately
  by four bullets (`resolve`, `build_ledger`, `build_week_view`,
  `DbWeekData::worked_minutes`) at lines 210-232.
- **Evidence**: `grep -n "stint::classify(" src/status.rs src/week_view.rs`
  confirms four real non-test call sites: `src/status.rs:300`
  (`build_ledger`), `src/status.rs:339` (`resolve`), `src/week_view.rs:136`
  (`build_rows`, called from `build_week_view`), `src/week_view.rs:191`
  (`DbWeekData::worked_minutes`). The count in the lead sentence ("three")
  contradicts both the source and the spec's own enumeration two lines
  later. Low practical risk since the list itself is complete and
  correct, but it's a clean internal-consistency defect worth fixing
  before this doc is used as an implementation checklist.

### 3. §6's doc-update list omits README.md, which documents both defects as "Known limitations" — **medium** (scope gap)

- **Claim**: `docs/dev/specs/2026-09-19-boundary-stint-pairing.md:335-349`
  (§6) lists only `SPEC.md §1.2`, `SPEC.md §4.3`, and `NOTES.md` as
  needing updates once this ships.
- **Evidence**: `README.md:45-49` ("### Known limitations") currently
  states, verbatim: "A session spanning midnight splits into two pieces
  instead of one clean stint (pairing is strictly per calendar date)"
  and "A `stop`/`start` typed at the exact same instant, back-to-back
  between two real stints, can mis-pair (tracked, not yet fixed — see
  `docs/dev/SPEC.md` §1.2/§4.3)" — these are exactly the two defects
  this spec fixes. Updating `README.md` alongside `SPEC.md`/`NOTES.md`
  when a spec ships is the project's established convention: commit
  `e2ad62b docs: reflect that backdated punches are implemented`
  (`git log --oneline -- README.md`) did exactly this for the prior
  backdated-punches spec. This spec's §6 will leave `README.md` stale
  and actively wrong (claiming a limitation that no longer exists)
  unless a maintainer catches it outside the spec's own instructions.

### 4. `classify_at` has no equivalent of `classify()`'s date-consistency `debug_assert!` — **medium** (missing edge case / error-handling gap)

- **Claim**: §4.2 (`docs/dev/specs/2026-09-19-boundary-stint-pairing.md:155-203`)
  specifies `classify_at`'s signature and behavior but says nothing
  about validating that `prev_punches`/`next_punches` are actually the
  literal adjacent-date punches the whole splice rule depends on.
- **Evidence**: `src/stint.rs:115-119` — `classify()` itself has
  `debug_assert!(punches.iter().all(|q| q.date == punches[0].date), "classify() expects all punches to share one date")`,
  an explicit self-check against caller mistakes. The entire correctness
  argument of §4.1's splice rule rests on `prev`/`next` being exactly
  `date.pred_opt()`/`date.succ_opt()` (§2: "never further than one day
  out"); a caller bug — an off-by-one in the date arithmetic at any of
  the four call sites in §4.3, or accidentally passing the same day
  twice — would silently produce an incorrect splice with no defensive
  check at all, in a function explicitly designed to be conservative
  everywhere else. The spec should require an analogous
  `debug_assert!` on `classify_at` (each of `prev_punches`/`next_punches`
  sharing one date, and — if cheaply checkable — that date being exactly
  `punches[0].date.pred_opt()`/`.succ_opt()`), and §5.2 should test it.

### 5. No test exercises the tied-instant-group fix (§3) interacting with the "chronologically first punch" splice check (§4) at the same boundary — **low-medium** (testing gap)

- **Claim**: §5.1/§5.2 test plans (`docs/dev/specs/2026-09-19-boundary-stint-pairing.md:251-307`)
  cover the two mechanisms independently but never combine them: no
  fixture has `next`'s very first instant be a tied group (e.g. an
  orphaned `end` sharing an `at_utc` with an unrelated `start`) that
  interacts with both the new step-1/step-3 tie-break *and* the
  "punch index 0" check in the same date.
- I traced this by hand against the algorithm as specified: if `next`'s
  first instant has both an `End` and a `Start` tied together, §3's
  scan (stack empty at the start of `next`'s own scan) actually
  zero-pairs them internally (step 3 pops the just-pushed `Start`)
  before `orphaned_ends` is ever populated — so that specific shape
  never reaches the splice check as an orphan at all, and a tied group
  of 2+ `End`s at the first instant is separately excluded by the
  existing `orphaned_ends.len() == 1` gate. So the two mechanisms don't
  appear to conflict in the cases I checked — but the spec text never
  walks through this interaction, and given these are the two riskiest,
  least-intuitive pieces of the whole change landing in the same
  changeset, a fixture pinning this down (rather than relying on an
  informal hand-trace like this one) belongs in §5.1 or §5.2 explicitly.

### 6. Residual limitation not spelled out: a genuine cross-midnight stint sharing a day with an unrelated second orphan never gets spliced — **low** (design-boundary observation, not a bug)

- **Claim**: §4.1 (`docs/dev/specs/2026-09-19-boundary-stint-pairing.md:144-153`)
  requires `orphaned_ends.len() == 1` on the later date; §2 states the
  general "1:1 or not at all" guardrail.
- **Evidence**: a `A+1` date holding a genuine midnight-crossing orphan
  as punch index 0 *plus* one unrelated stray orphan later that same
  day (`orphaned_ends.len() == 2`) will — per the explicit "Anything
  short of that exact 1:1 shape is left completely alone" rule
  (line 144) — leave **both** the legitimate carry-over stint and the
  stray orphan flagged exactly as today, i.e. the motivating bug from
  §1 stays unfixed for that specific (rare but not exotic) combination.
  This is a natural, presumably intended consequence of the "1:1 or
  nothing" scope discipline the spec already states as a principle, so
  it isn't a defect in the spec's logic — but the spec doesn't call out
  this residual case anywhere despite §1 framing the fix as unqualified
  ("Both fixes together remove SPEC.md §1.2's ... bullets"), and §6's
  planned SPEC.md rewrite should make clear the fix is still probabilistic,
  not absolute, or a reader will reasonably expect zero coverage gaps
  left.

### 7. `DbWeekData::worked_minutes`'s widened fetch is not covered by the "not worth the bookkeeping" performance justification — **low** (scope/perf note)

- **Claim**: §4.3's `build_ledger` bullet (`docs/dev/specs/2026-09-19-boundary-stint-pairing.md:216-221`)
  justifies skipping a rolling-window optimization with "this runs once
  per `status` invocation over a bounded week span, not in a hot loop."
  No equivalent justification is given for the `worked_minutes` bullet
  immediately below it (lines 228-232).
- **Evidence**: `src/week_view.rs:182-193` (`DbWeekData::worked_minutes`)
  is called once per week visited by `week::week_accounting`'s carry-in
  walk-back — potentially many weeks (see existing tests
  `c7_never_touched_week_carries_forward` and
  `c8_multi_week_idle_gap_carry_end_to_end`,
  `src/week_view.rs:851-887`, which already exercise multi-week
  walk-back chains). Under this spec, every one of those calls now also
  does a 9-day padded `punches_in_range` fetch and up to 7
  `classify_at` calls, even for weeks with zero punches. This is a
  proportional scale-up of an already-accepted cost (not a new
  algorithmic class), so it's not blocking, but the spec's stated
  performance reasoning explicitly covers only `build_ledger`, leaving
  the reader to independently re-derive that the same reasoning applies
  to the (more frequently invoked) `worked_minutes` site.

## Verdict

**ship-with-followups** — the two algorithmic changes (§3's tied-group
reorder and §4's `classify_at` splice) are internally sound: I traced
the worked example, both existing same-instant tests, the multi-end
tied-group case, the "not first punch" rejection case, and the
week-boundary attribution case by hand against the actual `stint.rs`
source, and found no correctness defect in the algorithm as specified.
The problems are in the spec document itself: a wrong citation backing
its own scope rationale (#1), a self-contradicting call-site count
(#2), a missing README.md update in its own migration checklist (#3),
a missing defensive check whose absence undermines the "mechanical,
forced fix only" safety argument the whole design rests on (#4), and a
gap in the test plan at the exact intersection of the two new
mechanisms (#5). None of these require redesigning the algorithm; all
are fixable edits to the spec text/plan before implementation starts.

VERDICT: findings(7)
FILE: docs/dev/plans/reports/boundary-stint-pairing-review.md
1. [high] §2's "NOTES.md decision 4 (nearest-match, not sequential pairing)" citation is wrong — decision 4 says the opposite (spec:34 vs NOTES.md:82-89).
2. [medium] §4.3 says "three" classify() call sites, then lists and the codebase has four (spec:207 vs status.rs:300/339, week_view.rs:136/191).
3. [medium] §6's doc-update list omits README.md, which documents both defects as "Known limitations" and would go stale (spec:335-349 vs README.md:45-49).
4. [medium] classify_at has no debug_assert equivalent to classify()'s date-consistency check, despite correctness depending on exact adjacency (spec §4.2 vs stint.rs:115-119).
5. [low-medium] No test combines §3's tied-group fix with §4's first-punch splice check at the same boundary date (spec §5.1/§5.2 gap).
6. [low] A genuine cross-midnight stint sharing a day with an unrelated second orphan still never splices — true by design but not called out (spec §4.1/§1).
7. [low] worked_minutes's widened multi-week fetch isn't covered by the perf justification given only for build_ledger (spec §4.3 vs week_view.rs:182-193, 851-887).
