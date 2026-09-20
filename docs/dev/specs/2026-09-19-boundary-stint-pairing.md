# Boundary stint pairing — design spec

> **Archived — historical changeset spec.** Superseded by `docs/dev/SPEC.md`, which folds in the decisions made here.


**Baseline**: written against `v0.3.2`.

**Status**: locked — two adversarial review rounds folded in:
`docs/dev/plans/reports/boundary-stint-pairing-review.md` (round 1,
ship-with-followups, 7 findings) and
`docs/dev/plans/reports/boundary-stint-pairing-review-round2.md`
(round 2, needs-rework, 2 findings — a `debug_assert` panic-on-empty-
input bug and a `worked_minutes` double-counting risk, both real
correctness defects, now fixed in the text below).

## 1. Summary

SPEC.md §1.2/§4.3 carries two accepted-but-deferred defects in stint
pairing (`src/stint.rs::classify`), both really one problem: the
nearest-match (LIFO) algorithm only ever looks at a single calendar
date's punches, and its same-instant tie-break was only ever patched
for the isolated zero-length case (E14), not the boundary case. This
spec fixes both in one changeset:

- **Same-instant boundary mis-pairing** (§4.3's documented "known
  defect"): `stop 09:00` then `start 09:00` back-to-back currently
  zero-pairs the tied instant instead of closing the `start` that was
  already open before it, silently dropping that stint's minutes with
  no anomaly.
- **Midnight-spanning stints** (§1.2 non-goal): a session like `start
  23:30` / `stop 00:45` the next day currently renders as two
  unrelated halves — day one's plain non-anomalous open stint (wrong:
  it never closes) and day two's flagged orphaned-`end` anomaly — with
  the time silently missing from both days' totals in the meantime
  (`week`'s per-day sum only counts `completed`, never `open`).

Both fixes together remove SPEC.md §1.2's "stints spanning midnight"
and "same-instant `end`/`start` boundary" bullets — see §6.

## 2. Design principle (scope guardrail)

Per SPEC.md §4.3's nearest-match (LIFO) algorithm: fix only what's
mechanically forced — a boundary that has exactly one possible
reading — and leave everything with more than one plausible reading
exactly as flagged today, for the user to sort out by hand.
Concretely:

- Pairing stays scoped to looking at, at most, the literal adjacent
  calendar date — never "the next date with any punches," never
  further than one day out.
- No new heuristics that guess among multiple candidates. Both fixes
  below apply **only** when the match is 1:1 unambiguous; anything
  else (2+ open stints, 2+ orphaned ends) is untouched, still flagged
  exactly as today.
- A spliced boundary stint is a normal completed stint once resolved —
  not a new anomaly kind, not a new render marker. It either fully
  resolves (clean stint, no flag) or it doesn't apply at all (existing
  anomaly stands, unchanged).

## 3. Same-instant tie-break fix

### 3.1 Current behavior (§4.3 step 1, exact)

`classify()` sorts a date's punches once — `(at_utc, kind, id)`, `Start`
before `End` — then does a single flat LIFO scan. For a tied instant
group containing a `start` and an `end`, the sort always visits the
`start` first, so it gets pushed and immediately popped by the tied
`end` — a zero-length pair — regardless of whatever was already open
on the stack before that instant.

### 3.2 New behavior

Within each *group* of punches sharing one `at_utc` (processed in
ascending-instant order, exactly as today), before any of the group's
own `start`s are pushed:

1. For each `end` in the group (in `id` order, for determinism), if
   the stack (carried in from strictly earlier instants) is
   non-empty, pop it and close that stint — **this is the fix**: an
   `end` preferentially closes an already-open `start` from before
   this instant over pairing with a same-instant `start`.
2. Push the group's `start`s onto the stack (LIFO, `id` order).
3. Any `end` from the group that didn't close anything in step 1 (the
   stack was already empty when it was its turn) pops against the
   stack now — which at this point holds only this group's own
   `start`s (or is empty). This is what still produces the isolated
   E14 zero-length pair when nothing was open beforehand, and an
   ordinary orphaned-`end` anomaly (§4.3, unchanged) when a group has
   more `end`s than there are `start`s to pair against, in it or
   before it.

This is a strict generalization of today's scan: every group of size 1
(the overwhelming majority of punches — no tie at all) reduces to
exactly the same push/pop behavior as before, so nothing needs a
migration path and every existing non-tied-group test in
`src/stint.rs` keeps passing unmodified. Only genuinely tied instants
change behavior, and only when something was already open before the
tie.

Worked example (the motivating case, §1.2): a `start` at `08:00`, then
a tied `stop`/`start` at `09:00`. Old behavior: `09:00` zero-pairs
with itself, `08:00` dangles open forever (silently wrong, no
anomaly). New behavior: the `09:00` `end` closes the `08:00` `start`
(a real `08:00`–`09:00` stint), then the `09:00` `start` is pushed
fresh — becomes the new trailing open stint, to be closed normally by
whatever `end` comes next. No anomaly either way; this was never
malformed data, just mis-paired.

### 3.3 No signature change

`classify(punches: &[Punch], now: DateTime<Utc>) -> DayStints` keeps
its exact signature. This is entirely an internal change to the
step-2 scan in `src/stint.rs`; nothing outside the function is
touched.

## 4. Midnight-spanning splice

### 4.1 Rule

For two literal adjacent calendar dates `A` and `A+1` (never further
apart — §2): if plain single-date `classify()` of `A`'s punches leaves
**exactly one** trailing open `start` (`open.len() == 1`), *and* plain
single-date `classify()` of `A+1`'s punches has **exactly one**
orphaned `end` (`orphaned_ends.len() == 1`), **and that orphaned `end`
is `A+1`'s chronologically first punch of the date** (sorted by the
same `(at_utc, kind, id)` order §3 uses — not merely "the date's only
orphan," but literally punch index 0 for that date), splice them into
one completed stint:

The third condition matters on its own: `orphaned_ends.len() == 1` is
not enough by itself. A date can have one orphan sitting *after* one
or more ordinary completed pairs earlier that day (e.g. `09:00 start,
10:00 end` completes cleanly, then an unrelated `13:00 end` pops an
empty stack) — that orphan has nothing to do with a midnight-spanning
stint and must never be spliced into yesterday's dangling open. A
genuine carried-over stint's `end` is always the very first thing that
happens on `A+1` — nothing on `A+1` precedes it — so requiring it to
be punch index 0 is what actually distinguishes "this closes
yesterday" from "this is today's own stray orphan."

- The stint's `start` is `A`'s open punch, its `end` is `A+1`'s
  orphaned punch. Minutes computed the same way as any other stint
  (`end.at_utc - start.at_utc`).
- **The stint belongs to `A`** — added to `A`'s `completed` list, its
  minutes count toward `A`'s day total and `A`'s week (per your
  direction: the workday is "until I go to sleep," so it lands on the
  day it started, not the day it happened to cross into). `A+1` never
  sees this stint in its own `completed` list.
- `A`'s open entry is consumed (no longer open, no longer "ongoing").
  `A+1`'s orphaned-`end` entry is consumed (no longer flagged as an
  anomaly). Neither date shows any anomaly for this pair once spliced.

Anything short of that exact 1:1 shape is left completely alone,
rendered exactly as it is today:

- `A` has 0 or 2+ trailing open starts.
- `A+1` has 0 or 2+ orphaned ends.
- `A` and `A+1` aren't literally adjacent calendar dates (there is no
  "skip empty days and find the next date with punches" case — a gap
  day with zero punches in between means no splice, full stop; `A`'s
  open stint and `A+1`'s — or whichever later date's — orphan both
  stay flagged independently).

Residual case worth naming: a genuinely carried-over orphan (punch
index 0, per §4.1) sharing `A+1` with one unrelated stray orphan
elsewhere that same date fails the `orphaned_ends.len() == 1` gate
just like any other 2-orphan date — so it does **not** splice, even
though the midnight-crossing half of it is exactly the case this spec
exists to fix. This is the "1:1 or not at all" guardrail (§2) doing
its job, not a bug — but it means §1's "both fixes together remove
SPEC.md §1.2's ... bullets" isn't an unconditional guarantee for every
possible day's data, only for the ordinary case. §6's SPEC.md rewrite
should state this residual case explicitly rather than implying total
coverage.

### 4.2 API shape

`classify()` itself is untouched by this section — it stays the pure,
single-date primitive, now internally using §3's fixed scan. A new
function sits on top:

```rust
/// Classifies `punches` (one calendar date) same as `classify()`, then
/// resolves an unambiguous midnight-spanning stint against its
/// immediate neighbors per SPEC.md §4.3's boundary-splice rule.
/// `prev_punches`/`next_punches` are the literal adjacent calendar
/// dates' punches (pass `&[]` when a neighbor has no data — an empty
/// slice already classifies correctly as "nothing open, nothing
/// orphaned").
pub fn classify_at(
    prev_punches: &[Punch],
    punches: &[Punch],
    next_punches: &[Punch],
    now: DateTime<Utc>,
) -> DayStints
```

Implementation: call `classify()` on all three slices, then apply the
§4.1 rule twice — once treating (`prev`, `day`) as the (`A`, `A+1`)
pair (if it fires, `day` loses the matched orphan; the stint itself is
not added here, since it belongs to `prev`'s own view, produced by
*that* date's own `classify_at` call), and once treating (`day`,
`next`) as the pair (if it fires, `day` gains the completed stint and
loses the matched open entry). `has_anomaly` is recomputed from the
adjusted `open`/`orphaned_ends` after both checks. Each date's view is
computed independently and statelessly — no shared/mutated state
across dates, no ordering dependency between which date's `classify_at`
runs first. (Both directions can fire for the same `day` at once, e.g.
a date that both closes yesterday's dangling open *and* itself dangles
into tomorrow — they touch disjoint fields, `orphaned_ends` vs. `open`,
so there's no conflict between them.)

The "chronologically first punch of the date" check (§4.1) needs the
later date's punches sorted the same way `classify()` sorts them
internally — cheapest done by sorting `next_punches`/`punches` (a
local copy, same `(at_utc, kind, id)` key as §3) once per pairing check
and comparing its first element's `id` against the candidate orphan's
`id`, rather than re-deriving order from `DayStints` (which does not
expose punch order beyond the already-bucketed `completed`/`open`/
`orphaned_ends` fields).

`classify()` remains public and used directly wherever a caller
genuinely only has one date in hand with no meaningful neighbor
concept (none of today's callers qualify — see §4.3).

`classify()` already guards against a caller mistake with
`debug_assert!(punches.iter().all(|q| q.date == punches[0].date), ...)`
(`src/stint.rs:115-119`). `classify_at`'s entire splice rule depends
even more tightly on `prev_punches`/`next_punches` being exactly
`punches[0].date.pred_opt()`/`.succ_opt()` — an off-by-one at any call
site would silently produce a wrong splice with nothing catching it.
`classify_at` needs the equivalent self-checks: each of
`prev_punches`/`next_punches` internally shares one date (same
assertion as `classify()`, once per non-empty slice), and — since each
slice already carries its own `date` field per punch — that
`prev_punches[0].date == punches[0].date.pred_opt()` and
`next_punches[0].date == punches[0].date.succ_opt()` whenever *both*
sides of a given comparison are non-empty (`punches` itself, not only
the neighbor). A bare `punches[0]` in the assert condition is a plain
slice index, which panics in every build profile — not compiled out in
release the way the rest of a `debug_assert!`'s condition is — so
`punches` must never be indexed unguarded: an empty `punches` bordering
a non-empty neighbor is the ordinary shape for any idle calendar date
once `classify_at` replaces `classify()` everywhere (§4.3), not a rare
input. Mirror `classify()`'s own safety property at `src/stint.rs:115-119`,
where `punches[0]` sits inside a closure passed to `.iter().all()` and
is therefore never evaluated for an empty slice — e.g. gate each
comparison on `punches.first()` (skip the check entirely when `punches`
is empty; there is no `punches[0].date` to compare against, so there is
nothing that check could have caught in that case anyway).

### 4.3 Call-site changes

All four current `classify()` call sites move to `classify_at`,
fetching one extra day of punches on each side:

- **`src/status.rs::resolve`** (line ~339): fetch
  `storage::punches_for_date(conn, target_date.pred_opt().unwrap())`
  and `...succ_opt().unwrap()` alongside the existing
  `target_date` fetch, pass all three to `classify_at`.
- **`src/status.rs::build_ledger`** (line ~300, the day-by-day walk
  from `start_week.start()` to `through.end()`): fetch the previous
  and next calendar date's punches at each iteration the same way.
  (No rolling-window optimization — each iteration already does one
  `punches_for_date` call for its own day; two more per iteration is
  not worth the bookkeeping to avoid, given this runs once per `status`
  invocation over a bounded week span, not in a hot loop.)
- **`src/week_view.rs::build_week_view`** (line ~215): widen the
  fetched range by one day on each side —
  `storage::punches_in_range(conn, start.pred_opt().unwrap(),
  end.succ_opt().unwrap())` — then `build_rows` looks up
  `date.pred_opt()`/`date.succ_opt()` in the same `by_date` map it
  already builds (defaulting to `&[]` via the existing
  `.unwrap_or_default()` pattern when a neighbor date has no bucket).
- **`src/week_view.rs::DbWeekData::worked_minutes`** (line ~182):
  same widening — fetch `punches_in_range(start.pred_opt().unwrap(),
  end.succ_opt().unwrap())`, bucket by date, and for each of the
  week's own 7 dates call `classify_at` with that date's neighbors
  pulled from the same map. **Required behavior change, not just a
  wider fetch**: the current implementation sums
  `by_date.values().map(|bucket| classify(bucket, ...).completed_minutes()).sum()`
  — every bucket the map happens to contain. That's harmless today only
  because `punches_in_range(start, end)` is scoped exactly to the
  week's 7 dates, so the map can never hold anything else. Once the
  fetch widens to 9 days, the map also holds the day before `start` and
  the day after `end` — both belonging to *adjacent* weeks — and
  `.values().sum()` would silently fold an unrelated padding day's own
  stint into this week's `worked_minutes`, while that same day is
  *also* one of the adjacent week's own 7 dates in its own
  `worked_minutes` call, double-counting it across two weeks' totals.
  This must be rewritten to sum over `week.dates()` explicitly — the
  same fixed 7-element-array pattern `build_rows` already uses
  (`src/week_view.rs:133-134`), never over every key the padded map
  happens to contain. `worked_minutes` is called once per week visited
  by `week::week_accounting`'s carry-in walk-back — potentially several
  weeks per invocation (see `c7_never_touched_week_carries_forward`/
  `c8_multi_week_idle_gap_carry_end_to_end`, `src/week_view.rs:851-887`,
  both of which seed fully-idle padding weeks and so would not catch a
  regression here) — so this 9-day padded fetch and up to 7
  `classify_at` calls repeats per week walked, not just once. Same
  performance reasoning as `build_ledger` above applies regardless: a
  proportional scale-up of an already-accepted cost (one `status`/
  `week` invocation, not a hot loop), not a new algorithmic class — not
  worth a rolling-window optimization here either.

`NaiveDate::pred_opt()`/`succ_opt()` (not `-`/`+` `Days`, and not the
panicking `pred()`/`succ()`) — mirrors the existing
`checked_sub_days`-style overflow discipline elsewhere in this
codebase (`docs/dev/specs/2026-09-13-backdated-punches.md` §3). In
practice this can only run out of `NaiveDate`'s representable range at
the literal edge of what `chrono` can express, not at any date real
punch data will ever contain — `.unwrap()` is fine here, the same way
existing code already trusts stored dates to be in range.

### 4.4 `week_view.rs`'s stale doc comment

`build_rows`'s doc comment currently states: "one call per date, never
a whole week's punches in one call, which is what keeps cross-midnight
pairing out of scope (E15)." That sentence becomes false — update it
to describe the padded-neighbor lookup instead (§7 also sweeps for any
other doc claiming this is out of scope).

## 5. Testing plan

### 5.1 `src/stint.rs` — tie-break fix, still pure single-date `classify()`

- New: a `start` before a tied `end`+`start` group closes the earlier
  `start`, not itself — the exact worked example in §3.2
  (`08:00 start`, tied `09:00 end`+`start`) — asserting the completed
  stint is `08:00`–`09:00` and the `09:00 start` is the sole trailing
  open entry, no anomaly.
- New: same shape but the tied group's `end` has nothing to close
  (nothing open before it) and there's no same-group `start` either
  (a lone tied `end` sharing an instant with an unrelated `start`
  elsewhere isn't actually a tie in the relevant sense — pick a
  genuine multi-`end` tied group instead): tied group of 2 `end`s + 1
  `start`, nothing open before — one `end` pairs zero-length with the
  group's `start`, the other becomes an ordinary orphaned-`end`
  anomaly (§4.1's "falls out naturally" case).
- Existing `same_instant_pair_is_a_legal_zero_length_stint` and
  `same_instant_pair_still_pairs_when_end_entered_first`: unchanged,
  must still pass byte-for-byte (they're the "nothing open before the
  tie" case, which §3.2 explicitly reduces to today's behavior).
- Regression sweep: every other existing `stint.rs` test (no tied
  groups involved) must pass unmodified — they exercise only
  singleton groups, which §3.2 states reduce to the original scan
  exactly.

### 5.2 `src/stint.rs` — `classify_at` splice

- Clean splice: `prev` has one trailing open `start`, `day` has
  exactly one orphaned `end` at the day boundary — resolves to one
  completed stint on `prev`'s side and zero anomalies on `day`'s side.
  Mirror the same case checking `day`'s own trailing open against
  `next`'s single orphan.
- Both directions firing on the same `day` at once (closes yesterday's
  dangling open *and* itself dangles into tomorrow) — both resolve
  independently, no interference.
- Non-1:1 shapes stay unspliced and exactly as flagged today: `prev`
  has 2 trailing opens (E7) with `day` holding one orphan; `day` holds
  2 orphans (never coalesced, §4.3) with `prev` holding one trailing
  open; `prev`/`next` empty (`&[]`, no data at all).
- **`day`'s only orphan is not its first punch**: `prev` has one
  trailing open, `day` has exactly one orphaned `end` but it occurs
  after one or more of `day`'s own completed pairs earlier that date
  (e.g. `day` = `[09:00 start, 10:00 end, 13:00 end]` — the `13:00`
  is `orphaned_ends`'s only entry but punch index 0 is the `09:00
  start`, not it) — no splice; `prev`'s open and `day`'s orphan both
  stay flagged exactly as today. This is the case this revision's
  first-punch rule exists to reject.
- A genuine gap: `prev` has an open trailing start, but `day` itself
  has *no* orphaned end (the real matching orphan, if any, is two
  calendar dates out) — no splice, `prev`'s open stays open exactly as
  it renders today (§4.1's "no skip-empty-days" rule).
- `classify_at` called with empty `prev_punches`/`next_punches`
  reduces to plain `classify()`'s output for the day in between (a
  direct regression check against every existing `classify()` test
  fixture, called through `classify_at` with empty neighbors, same
  expected results).
- **§3 × §4 interaction, at the boundary itself**: `next`'s first
  instant is itself a tied group sharing an `End` with an unrelated
  `Start` (e.g. `next = [00:00 end, 00:00 start, 08:00 end]`, `prev`
  has one trailing open). Per §3.2, when nothing is open on `next`'s
  own stack before that instant, the tied `End`+`Start` zero-pairs
  internally (§3.2 step 3) before `orphaned_ends` is ever populated —
  so this shape never reaches the splice check as an orphan at all,
  and `prev`'s open stays open, unspliced. Assert that directly, since
  it's the least obvious interaction between the two mechanisms in
  this changeset and the two must not be assumed compatible without a
  fixture pinning it down. Also cover the mirror case where `next`'s
  first instant is a tied group of 2+ `End`s (no `Start` in the tied
  group) with nothing open before it: one `End` is left as the
  genuine orphan at index 0, correctly eligible for the splice check
  (`orphaned_ends.len()` must still be exactly 1 overall for the
  splice to fire — assert the multi-end case where it doesn't).
- **Empty `punches` bordering a non-empty neighbor** (§4.2's
  `debug_assert!` guard): `punches = &[]`, `prev_punches` or
  `next_punches` non-empty (e.g. `prev` has a trailing open). Must not
  panic — `classify_at` returns the same empty `DayStints` `classify(&[],
  now)` would, no splice (there is no `punches[0]` to compare against,
  so the adjacency check has nothing to validate and must be a no-op
  here, not a crash). Pins down round 2's finding 1 directly.

### 5.3 Integration — `status`, `week`

- `status` on the earlier of two midnight-spanning dates now shows the
  full stint completed (not "ongoing," not silently zero) and no
  anomaly; `status` on the later date shows the correct remaining
  punches for that date with no orphaned-`end` anomaly for the spliced
  one.
- `week`'s per-day total for the earlier date includes the spliced
  stint's minutes; the later date's total does not (§4.1: the stint
  belongs to the day it started). Week `worked`/`fulfillment`/`owed`
  figures reflect the now-recovered minutes.
- A splice landing exactly on a week boundary (`A` in one `WeekId`,
  `A+1` in the next): confirm the minutes land in `A`'s week only, and
  that `build_week_view`'s padded-range fetch (§4.3) correctly reaches
  one day outside the week span without pulling that neighbor date's
  own unrelated stints into the wrong week's rows.
- **`DbWeekData::worked_minutes` padding-day isolation** (round 2
  finding 2): a week whose calendar date immediately before `start` or
  immediately after `end` — outside the week's own 7 dates, but inside
  `worked_minutes`'s widened 9-day fetch — has its own ordinary,
  *unrelated* completed stint (no splice involved, just an ordinary
  punch pair on that padding day). Assert that padding day's minutes
  appear in *only* its own week's `worked`/`fulfillment`/`owed`
  figures, never added into the week under test's totals, and are not
  double-counted into both weeks. `c7`/`c8`'s existing fixtures
  (`src/week_view.rs:851-887`) seed fully-idle padding weeks and do not
  exercise this — this needs its own fixture with a real stint on the
  padding day specifically.
- CI e2e smoke (`.github/workflows/ci.yml`): add a scripted
  `start`/`stop` pair straddling midnight via two `-d`/`--date`-backed
  punches (mirroring the existing backdated-punch smoke block), then
  assert `status` on the start date shows the completed total and
  `status` on the end date shows no anomaly.

No new coverage/complexity work beyond what naturally follows from
exercising the new branches (same convention as
`docs/dev/specs/2026-09-13-backdated-punches.md` §5's closing note).

## 6. SPEC.md / NOTES.md / README.md updates

- §1.2: remove the "Stints spanning midnight" and "Same-instant
  `end`/`start` boundary between two real stints" bullets entirely
  (no longer non-goals — both fixed).
- §4.3: rewrite step 1's tie-break description per §3.2 above, replace
  the "Known defect... deferred" callout with a short note that it's
  fixed as of this spec (cross-reference this file), and add a new
  paragraph describing the §4.1 splice rule and its "exactly 1:1 or
  not at all" scope guardrail — including the §4.1 residual case (a
  legitimate carried-over orphan sharing its date with an unrelated
  second orphan still doesn't splice; see §4.1's closing paragraph).
- NOTES.md: append a decision entry recording the two-phase
  tied-group scan (§3.2) and the neighbor-padded `classify_at` shape
  (§4.2) as the chosen design, referencing this spec file — matching
  the convention in `docs/dev/specs/2026-09-13-backdated-punches.md`'s
  own NOTES.md entry.
- `README.md`'s "### Known limitations" section (lines 44-50) currently
  states both defects as unfixed, verbatim: "A session spanning
  midnight splits into two pieces instead of one clean stint" and "A
  `stop`/`start` typed at the exact same instant, back-to-back between
  two real stints, can mis-pair." Remove both bullets (or the whole
  section, if nothing else populates it) — matching the precedent set
  by commit `e2ad62b docs: reflect that backdated punches are
  implemented`, which did the same for the backdated-punches spec.

## 7. Out of scope

- Any splice spanning more than one calendar day out (§2 — this is the
  line between "mechanical, forced fix" and "heuristic guess," and the
  guardrail is deliberate, not a shortcut).
- Editing an existing punch/note (§1.2, still a non-goal, untouched by
  this spec).
- A new anomaly kind or render marker for "this stint was spliced
  across midnight" — per §2, a resolved splice is indistinguishable
  from any other ordinary completed stint once it's clean. (If a
  future spec wants that visibility back, it's additive, not blocked
  by anything here.)
- Reworking `week`'s or `status`'s existing per-day/per-week
  accounting model (§2.4's live-computation, non-materialized
  figures) — this spec only changes which stints `classify`/
  `classify_at` produce, not how completed minutes flow into weeks
  afterward.
