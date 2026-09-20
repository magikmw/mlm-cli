# Final review — boundary stint pairing (`v0.3.2` → `boundary-stint-pairing`)

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.


**Reviewed**: `git diff v0.3.2...boundary-stint-pairing`, every changed
line, plus a by-hand build-and-run of the release binary against the
spec's worked examples.

**Spec under review**: `docs/dev/specs/2026-09-19-boundary-stint-pairing.md`
(locked, two adversarial rounds folded in).

**Verdict**: **ship-with-followups** — 6 findings, none blocking. The
shipped code implements §3.2, §4.1, §4.2 and §4.3 correctly; the
load-bearing tests are genuinely load-bearing (verified by mutation, not
by reading); the manual transcripts match the spec verbatim. The
followups are the still-pending §6 doc sweep plus four small quality /
portability nits.

---

## 1. Scope of the changeset

```
 .github/workflows/ci.yml                           |   28 +
 coverage-baseline.json                             |    2 +-
 docs/dev/NOTES.md                                  |   87 ++
 docs/dev/plans/…  (spec, plan, task plans, reports) | 3606 ++
 src/status.rs                                      |  100 +-
 src/stint.rs                                       |  507 +++-
 src/week_view.rs                                   |  187 +-
```

Six commits: spec/plans, Task 1 (`stint.rs` algorithm) + merge, Task 2
(call-site rewiring) + report + merge. No stray edits outside the
spec's stated surface.

---

## 2. Correctness

### 2.1 §3.2 tied-instant-group scan reorder — correct

`classify()`'s flat LIFO loop is replaced by a group-walk
(`src/stint.rs:140-153`) that finds each maximal run of equal `at_utc`
and hands it to `close_group` (`src/stint.rs:177-232`). `close_group`
implements the three phases exactly as §3.2 specifies:

1. every `End` in the group pops the stack carried in from *strictly
   earlier* groups, before any of the group's own `Start`s exist on it;
2. the group's `Start`s are pushed;
3. the `End`s that found nothing in phase 1 pop again — against only
   this group's own `Start`s — producing the E14 zero-length pair or an
   ordinary orphan.

The "strictly earlier" property is structural rather than asserted: the
phase-1 loop runs to completion before the phase-2 push loop begins, so
the stack it sees provably contains no same-instant `Start`. That is the
right way to get this guarantee.

Ordering invariants hold. `group` is a contiguous slice of `classify`'s
own `sorted` vector, so within a group all `Start`s precede all `End`s
(`PunchKind::Start` < `PunchKind::End`, `src/storage.rs:54-57`) and each
kind's sub-sequence is in ascending `id` order — which is exactly what
the two `.filter()` passes rely on, and the doc comment states the
precondition explicitly. `completed` stays ascending by end instant
(phases 1 and 3 emit only same-instant ends); `open` stays ascending by
start instant (the stack is only ever pushed in ascending order).

The degenerate-group reduction §3.2 claims is real: a singleton `Start`
group does nothing in phase 1, pushes in phase 2, nothing in phase 3; a
singleton `End` group pops in phase 1 or falls through phases 2/3 to the
same pop/orphan decision the old code made. Empirically confirmed — all
425 pre-existing lib tests pass unmodified apart from the one
deliberately-rewritten `b2` (§4 below).

### 2.2 §4.1 splice rule — correct, all three gates present

`splice_candidate` (`src/stint.rs:306-318`) enforces exactly the three
conditions:

```rust
if earlier_open_count != 1 || later.orphaned_ends.len() != 1 {
    return None;
}
let mut sorted_later: Vec<Punch> = later_punches.to_vec();
sorted_later.sort_by_key(|q| (q.at_utc, q.kind, q.id));
let first = sorted_later.first()?;
(first.id == later.orphaned_ends[0].punch.id).then_some(0)
```

The third gate (chronologically-first-punch) is implemented the way
§4.2 prescribes: sort a local copy with the *same* `(at_utc, kind, id)`
key `classify()` uses, compare `id`s — not re-derived from `DayStints`.

`classify_at` (`src/stint.rs:227-296`) applies the rule in both
directions:

- `(prev, day)`: on a hit, `day.orphaned_ends.remove(0)` only. The
  completed stint is deliberately **not** added — it belongs to `prev`'s
  own view. Correct, and the comment says so.
- `(day, next)`: on a hit, `day.open.remove(0)` and a new `Stint` pushed
  onto `day.completed`, minutes = `end.at_utc - open.start.at_utc`.

Three properties I checked independently and found sound:

- **Symmetry / no double-attribution.** The `(prev, day)` check calls
  `splice_candidate(classify(prev_punches).open.len(), &day, punches)`.
  `prev`'s own `classify_at` call evaluates its `(day, next)` direction
  as `splice_candidate(classify(prev_punches).open.len(), &day, punches)`
  — literally the same three arguments. The two dates therefore *always*
  agree on whether the splice fires, with no shared state and no
  ordering dependency, exactly as §4.2 requires. And because opens are
  only ever consumed in the forward direction, `prev.open.len()` can
  never have been reduced by anything `classify_at` did elsewhere, so
  the raw `classify()` value is the right one to gate on.
- **Both directions are conflict-free.** They touch `orphaned_ends` and
  `open` respectively; the `(day, next)` gate reads `day.open`, which
  the `(prev, day)` branch never mutates. Order of the two `if` blocks
  is therefore irrelevant.
- **Emission order of the appended stint.** `day.completed.push(...)`
  appends. The spliced stint's `end` is on `day+1`, later than every
  end already in `day.completed`, so `completed`'s documented
  "ascending by end instant" invariant survives.

`has_anomaly` is recomputed as `open.len() > 1 || !orphaned_ends.is_empty()`
(`src/stint.rs:294`), matching `classify()`'s own definition
(`src/stint.rs:167`). See finding 5 for the DRY nit.

The `.expect()` at `src/stint.rs:301` is provably unreachable
(`splice_candidate` returning `Some` implies `orphaned_ends.len() == 1`)
and is documented as such.

### 2.3 §4.2 panic-safety guards — correct

All five `debug_assert!`s are empty-safe:

- the three "shares one date" checks use
  `Some(q.date) == slice.first().map(|f| f.date)` inside `.iter().all()`
  — never evaluated for an empty slice, and never a bare `[0]`;
- both adjacency checks are wrapped in
  `if let (Some(a), Some(b)) = (x.first(), y.first())`, so an empty
  `punches` bordering a non-empty neighbor skips the check entirely.

There is **no unguarded `punches[0]`/`prev_punches[0]`/`next_punches[0]`
anywhere in `classify_at`.** I grepped and read the whole function; the
only `[0]` indexes are `later.orphaned_ends[0]` (gated by the
`len() != 1` early return) and `day.orphaned_ends.remove(0)` /
`day.open.remove(0)` (gated by `splice_candidate` having returned
`Some`). Round 2's finding 1 is genuinely fixed, and
`classify_at_empty_punches_bordering_nonempty_{prev,next}_no_panic` pin
it (mutation-verified, §3 below).

*Note (finding 6)*: the spec's stated *rationale* for that finding — "a
bare `punches[0]` in the assert condition is a plain slice index, which
panics in every build profile — not compiled out in release" — is
technically inaccurate. `debug_assert!(cond)` expands to
`if cfg!(debug_assertions) { assert!(cond) }`, so the whole condition,
index included, is behind a compile-time-false branch in release and
never evaluated. The defect was still real for debug and `cargo test`
builds (which is where it mattered), and the shipped fix is correct and
strictly better regardless; only the justifying sentence in the locked
spec is wrong.

### 2.4 §4.3 call-site changes — all four done, `worked_minutes` fix correct

| Site | Change | Verdict |
|---|---|---|
| `status.rs::build_ledger` (`:297-303`) | fetches `date.pred_opt().unwrap()` / `.succ_opt().unwrap()`, calls `classify_at` | ✅ |
| `status.rs::resolve` (`:337-344`) | same, three fetches then `classify_at` | ✅ |
| `week_view.rs::build_week_view` (`:242`) | `punches_in_range(start.pred_opt().unwrap(), end.succ_opt().unwrap())` | ✅ |
| `week_view.rs::DbWeekData::worked_minutes` (`:194-217`) | padded fetch **and** the required `week.dates()` rewrite | ✅ |

The round-2 double-counting/leak fix is present and is the real thing,
not just a wider fetch:

```rust
week.dates()
    .into_iter()
    .map(|date| { … stint::classify_at(&prev_bucket, &bucket, &next_bucket, self.now_utc)
                        .completed_minutes() })
    .sum()
```

`by_date.values().sum()` is gone. I verified the fix is load-bearing by
mutation (§3, mutation 2) and end-to-end against the real binary (§4.3).

`build_ledger`'s fetch sits inside the existing
`if !day_punches.is_empty()` branch. Independently checked and agreed:
`classify_at` with empty `punches` can never produce a completed stint
(the `(day, next)` gate requires `day.open.len() == 1`, which requires at
least one punch), so the skipped iterations contribute 0 either way. No
minutes can be lost through that branch.

`build_rows`'s neighbour lookups come from the same padded `by_date` map
`build_week_view` now feeds it; the Monday row reaches the previous
week's Sunday and the Sunday row reaches the next week's Monday, both of
which the 9-day fetch supplies. `build_week_view` uses `punches` for
nothing but `build_rows`, so the widened fetch cannot contaminate
anything else in that function.

§4.4's stale `build_rows` doc comment ("…which is what keeps
cross-midnight pairing out of scope (E15)") is rewritten correctly.

---

## 3. Test strength — verified by mutation, not by reading

I mutated the implementation four ways and re-ran the suite each time,
restoring a clean tree afterwards (`git status --porcelain` → clean).

**Baseline**: `cargo test` → 425 + 2 + 1 + 2 + 5 passed, 0 failed.
`cargo clippy --all-targets --all-features -- -D warnings` → clean.

### Mutation 1 — drop the §4.1 chronologically-first-punch gate

```diff
-    let first = sorted_later.first()?;
-    (first.id == later.orphaned_ends[0].punch.id).then_some(0)
+    let _first = sorted_later.first()?;
+    Some(0)
```

```
test stint::tests::splice_orphan_not_first_punch_of_day_stays_unspliced ... FAILED
test stint::tests::tied_end_start_at_next_boundary_never_reaches_orphan_check ... FAILED
test result: FAILED. 423 passed; 2 failed
```

Both the dedicated first-punch test and the §3×§4 interaction test catch
it. **Load-bearing.**

### Mutation 2 — revert `worked_minutes` to enumerating every bucket

(keeping the widened 9-day fetch — i.e. exactly the "widen the fetch but
forget to fix the iteration" bug round 2 flagged)

```diff
-        week.dates()
-            .into_iter()
+        by_date.keys().copied().collect::<Vec<_>>().into_iter()
```

```
test week_view::tests::c14_worked_minutes_padding_day_isolation ... FAILED
test week_view::tests::c13_week_boundary_splice_end_to_end_lands_in_starting_weeks_view_only ... FAILED
test week_view::tests::c8_multi_week_idle_gap_carry_end_to_end ... FAILED
test result: FAILED. 422 passed; 3 failed
```

`c14` is precisely the guard round 2 asked for, and it fires. Task 2's
report notes `c14` "passed even before the implementation change" — that
is correct and not a test-quality problem: pre-Task-2 the fetch was
week-scoped, so the padding punches were never even loaded. Against the
*shipped* code's widened fetch, `c14` is the only thing standing between
the repo and a cross-week double-count, and it does stand. **Strongly
load-bearing.** Its three-week sum assertion
(`week_before + this_week + week_after == 120 + 480 + 90`) is a direct
anti-double-count check, not just a "no leak" check — good.

### Mutation 3 — restore the old same-instant tie-break, narrowly

Made phase 1 skip the pop only for genuinely tied mixed groups (so the
mutation isolates the §3.2 behaviour change rather than breaking the
whole scan):

```
test stint::tests::tied_end_and_start_closes_earlier_open_start ... FAILED
test result: FAILED. 424 passed; 1 failed
```

Exactly one test, exactly the intended one. Its assertions are specific
(`completed_spans == [(08:00, 09:00, 60)]`, `open[0].start.id == 3`),
not a weak "something completed" check. **Load-bearing.**

I also ran a broader mutation (pushing the group's `Start`s before
closing its `End`s unconditionally) which took down 10+ tests across
`stint`, `status` and `week_view` — confirming the group scan is
genuinely on the hot path of everything, not a dead branch.

### Mutation 4 — reintroduce the round-2 unguarded index

```diff
-    if let (Some(prev_first), Some(day_first)) = (prev_punches.first(), punches.first()) {
-        debug_assert!(Some(prev_first.date) == day_first.date.pred_opt(),
+    if let Some(prev_first) = prev_punches.first() {
+        debug_assert!(Some(prev_first.date) == punches[0].date.pred_opt(),
```

```
test stint::tests::classify_at_empty_punches_bordering_nonempty_prev_no_panic ... FAILED
test stint::tests::splice_genuine_gap_two_dates_out_no_splice ... FAILED
test week_view::tests::b2_bucketing_now_splices_across_midnight ... FAILED
test week_view::tests::b3_completed_stints_only_totals ... FAILED
… (and more)
```

Round 2's exact defect is caught by its named regression test *and* by
the whole `week_view` idle-day surface. **Load-bearing.**

### Other tests read for assertion strength (not mutated)

- `splice_prev_two_opens_stays_unspliced` / `splice_day_two_orphans_stays_unspliced`
  — assert the orphan is still present *by id* and `has_anomaly()`, not
  merely "no completed stint"; relaxing either `len()` gate breaks them.
- `single_end_at_next_first_instant_is_splice_eligible` — the positive
  control that keeps the negative tests honest (proves the negatives
  fail for the intended reason and not because the splice never fires).
- `tied_group_two_ends_at_next_boundary_multi_orphan_stays_unspliced`
  asserts `classify(next).orphaned_ends.len() == 2` *first*, pinning
  down **why** it doesn't splice rather than just that it doesn't. Good
  practice.
- `classify_at_empty_neighbors_matches_classify` compares whole
  `DayStints` values across 8 fixtures — a real equivalence check, not a
  field spot-check.
- `c13` asserts both `rows[6].minutes == 80` on week 7 **and**
  `rows[0].minutes == 0` / `worked_minutes == 0` on week 8, i.e. both
  halves of the attribution rule.

I found no new test that is merely decorative.

---

## 4. Manual verification against the spec's worked examples

Built `cargo build --release` (exit 0), drove `./target/release/mlm`
against scratch SQLite DBs via `MLM_DB_PATH`. Today's date in this
environment is 2026-09-19.

### 4.1 §3.2 worked example — `start 08:00`, tied `stop 09:00` / `start 09:00`

```console
$ export MLM_DB_PATH=$SCRATCH/tie.db
$ ./target/release/mlm start -d 2026-09-15 08:00
$ ./target/release/mlm stop  -d 2026-09-15 09:00
$ ./target/release/mlm start -d 2026-09-15 09:00
$ ./target/release/mlm status 2026-09-15
Tue 2026-09-15

Day total:     01h 00m (+ ongoing)
Week 2026-38:  39h 00m left by end of Saturday (fulfillment 01h 00m / target 40h 00m)

  08:00-09:00  (01h 00m)
  09:00-now    (106h 54m, ongoing)
```

Exactly §3.2: the `09:00` end closed the `08:00` start (a real 60-minute
stint), the `09:00` start is the sole trailing open, no anomaly. The old
behaviour would have shown a `09:00-09:00 (00h 00m)` zero pair with
`08:00` dangling open.

### 4.2 Midnight-spanning example — `start -d D 23:30`, `stop -d D+1 00:45`

```console
$ export MLM_DB_PATH=$SCRATCH/mid.db
$ ./target/release/mlm start -d 2026-09-15 23:30
$ ./target/release/mlm stop  -d 2026-09-16 00:45

$ ./target/release/mlm status 2026-09-15
Tue 2026-09-15

Day total:     01h 15m
Week 2026-38:  38h 45m left by end of Saturday (fulfillment 01h 15m / target 40h 00m)

  23:30-00:45  (01h 15m)

$ ./target/release/mlm status 2026-09-16
Wed 2026-09-16

Day total:     00h 00m
Week 2026-38:  38h 45m left by end of Saturday (fulfillment 01h 15m / target 40h 00m)

$ ./target/release/mlm week 2026-38
Week 2026-38 (2026-09-14 - 2026-09-20)

38h 45m left by end of Saturday

  Mon 2026-09-14   00h 00m
  Tue 2026-09-15   01h 15m
  Wed 2026-09-16   00h 00m
  Thu 2026-09-17   00h 00m
  Fri 2026-09-18   00h 00m
  Sat 2026-09-19   00h 00m
  Sun 2026-09-20   00h 00m

Carry-in:      00h 00m
Worked:        01h 15m
Fulfillment:   01h 15m
Target:        40h 00m
```

Matches §5.3 line by line: the earlier date shows the full completed
stint and no anomaly, the later date shows no orphaned-end anomaly and
contributes 0, and the week's per-day / `Worked` / `Fulfillment` figures
carry the recovered 75 minutes on the start date only.

### 4.3 Week-boundary splice + `worked_minutes` padding-day isolation, end to end

Sunday 2026-09-13 (end of week 37) → Monday 2026-09-14 (week 38), plus
an *unrelated* 2-hour stint on 2026-09-06 (Sunday of week 36 — the
padding day immediately before week 37's span):

```console
$ ./target/release/mlm start -d 2026-09-13 23:00
$ ./target/release/mlm stop  -d 2026-09-14 00:20
$ ./target/release/mlm start -d 2026-09-06 09:00
$ ./target/release/mlm stop  -d 2026-09-06 11:00

$ ./target/release/mlm week 2026-36 | tail -4
Carry-in:      00h 00m
Worked:        02h 00m          ← its own stint, undiminished
Fulfillment:   02h 00m
Target:        40h 00m

$ ./target/release/mlm week 2026-37
Week 2026-37 (2026-09-07 - 2026-09-13)
…
  Sun 2026-09-13   01h 20m       ← the spliced stint, on the day it started
…
Worked:        01h 20m          ← week 36's padding-day 2h did NOT leak in
Fulfillment:   -36h 40m

$ ./target/release/mlm week 2026-38 | tail -4
Carry-in:      -76h 40m
Worked:        00h 00m          ← the spliced stint did NOT land here
Fulfillment:   -76h 40m
Target:        40h 00m

$ ./target/release/mlm status 2026-09-13
Sun 2026-09-13
Day total:     01h 20m
Week 2026-37:  Total still owed: 76h 40m

  23:00-00:20  (01h 20m)

$ ./target/release/mlm status 2026-09-14
Mon 2026-09-14
Day total:     00h 00m
Week 2026-38:  116h 40m left by end of Saturday (fulfillment -76h 40m / target 40h 00m)
```

Three independent weeks, three correct totals, nothing double-counted —
the real-binary confirmation of `c14`/`c13`. Note also that `week 37`'s
per-day rows and its `Worked:` line agree (01h 20m both), i.e.
`build_rows` and `DbWeekData::worked_minutes` do not disagree at a
boundary.

### 4.4 Non-1:1 shapes still flagged exactly as before

```console
# two trailing opens on D, one orphan on D+1 (E7, not 1:1)
$ ./target/release/mlm start -d 2026-09-10 22:00
$ ./target/release/mlm start -d 2026-09-10 23:00
$ ./target/release/mlm stop  -d 2026-09-11 00:30
$ ./target/release/mlm status 2026-09-10
[!] 2 open stints for this date (unmatched starts)
  22:00-now    (213h 54m, ongoing)
  23:00-now    (212h 54m, ongoing)
$ ./target/release/mlm status 2026-09-11
[!] orphaned end at 00:30 (no matching start)

# D+1's only orphan is not its first punch (§4.1's third gate)
$ ./target/release/mlm start -d 2026-09-10 23:00
$ ./target/release/mlm start -d 2026-09-11 09:00
$ ./target/release/mlm stop  -d 2026-09-11 10:00
$ ./target/release/mlm stop  -d 2026-09-11 13:00
$ ./target/release/mlm status 2026-09-10
  23:00-now    (212h 54m, ongoing)          ← still open, not spliced
$ ./target/release/mlm status 2026-09-11
Day total:     01h 00m
[!] orphaned end at 13:00 (no matching start)
  09:00-10:00  (01h 00m)
```

Both residual shapes render exactly as §4.1 says they must. This also
confirms, against the real binary, the deviation discussed in §6.1.

---

## 5. Compliance with §6 (doc updates) — partially pending

| §6 item | State |
|---|---|
| `docs/dev/NOTES.md` decision entry | ✅ **Done** — decisions 55–58 added (design, round-1 fold, round-2 fold, task judgment calls), correctly cross-referencing the spec file. Matches the backdated-punches convention. |
| `docs/dev/SPEC.md` §1.2 — remove the two non-goal bullets | ❌ **Pending** — `docs/dev/SPEC.md:52` ("Stints spanning midnight: pairing is strictly per calendar `date`") and `:64-65` ("**Same-instant `end`/`start` boundary…** known defect in the current §4.3 tie-break, deferred…") are both still present verbatim. |
| `docs/dev/SPEC.md` §4.3 — rewrite step 1's tie-break, replace the "Known defect … deferred" callout, add the §4.1 splice rule + residual case | ❌ **Pending** — `docs/dev/SPEC.md:345` and `:349-351` still describe the old tie-break and carry the deferred-defect callout. |
| `README.md` "Known limitations" | ❌ **Pending** — `README.md:44-50` still states both defects verbatim, including "(tracked, not yet fixed — see `docs/dev/SPEC.md` §1.2/§4.3)". |
| §4.4 `build_rows` doc comment | ✅ **Done** (`src/week_view.rs:118-125`). |

This is the expected state — the coordinator planned to do these
directly after this review. Reporting it accurately: **the changeset is
not doc-complete as shipped**; a user reading `README.md` on this tip
would be told the bugs are unfixed. See finding 1.

§7's "sweep for any other doc claiming this is out of scope" is also
incomplete — see finding 2.

### Compliance with the changeset's own §3/§4

Every numbered requirement in §3.2, §3.3 (signature unchanged — yes,
`classify` keeps `(&[Punch], DateTime<Utc>) -> DayStints`), §4.1, §4.2
and §4.3 is implemented as written. The §5 testing plan's bullets each
have a corresponding test: §5.1 ×2 + the byte-for-byte-unchanged
existing tie tests (verified untouched in the diff); §5.2 ×10; §5.3 ×5
(`resolve_*` ×4 in `status.rs`, `b2`/`b9`/`c13`/`c14` in `week_view.rs`,
plus the CI smoke block). Nothing in §5 is unimplemented.

---

## 6. Code quality

Overall: idiomatic, well-commented, and the comments cite the spec
section they implement rather than restating the code. `close_group`'s
doc comment correctly documents its *precondition* (the slice must be
pre-sorted by `(at_utc, kind, id)`), which is the non-obvious thing a
future reader needs. No `unwrap()` on anything that can realistically
fail; no `clone()` in a hot path beyond what `classify` already did.

### 6.1 Deviation A — Task 2's corrected test expectation. **Holds up.**

`resolve_midnight_boundary_still_flags_non_1to1_shapes` expects `D+1` to
show an orphaned-end anomaly, where the task plan's prose had said "no
anomaly on that side either". My independent read agrees with the
correction, for two reasons the report gives and one it doesn't:

- Spec §4.1 is explicit: "Anything short of that exact 1:1 shape is left
  completely alone, rendered exactly as it is today." With `D` holding
  two opens, the gate never fires, so `D+1`'s `00:30` end *is* a genuine
  orphan by definition.
- `render::Anomalies` flags any orphaned end regardless of the
  neighbouring open count, as the pre-existing
  `resolve_e7_multi_open_and_e8_orphaned_end_via_anomalies_only`
  establishes for the same-date case.
- And I confirmed it against the real binary (§4.4 above):
  `status 2026-09-11` does print `[!] orphaned end at 00:30`.

The task plan's prose was simply wrong; the shipped test pins the
correct behaviour. No follow-up needed.

### 6.2 Deviation B — `unwrap_or_default()` vs `unwrap()` at neighbour lookups. **Holds up.**

```rust
let prev_bucket = date.pred_opt()
    .and_then(|d| by_date.get(&d).cloned())
    .unwrap_or_default();
```

The spec's §4.3 wording is "`build_rows` looks up `date.pred_opt()` /
`date.succ_opt()` in the same `by_date` map … (defaulting to `&[]` via
the existing `.unwrap_or_default()` pattern when a neighbor date has no
bucket)" — so the spec itself names `unwrap_or_default()` at exactly
these sites. The `.unwrap()` discipline in §4.3's closing paragraph is
about the *fetch-range* expressions, and those four sites do use
`.unwrap()` verbatim (`status.rs:300-301`, `status.rs:340-341`,
`week_view.rs:196-197`, `week_view.rs:243`). The only thing the chained
form gives up is distinguishing "chrono date-range edge" from "no bucket
for that date" — and at chrono's representable edge there could be no
such bucket anyway, so the two Nones are genuinely the same case here.
Strictly non-panicking, semantically identical. Agreed; no follow-up.

### 6.3 Remaining quality nits (findings 4 and 5)

- `splice_candidate -> Option<usize>` that can only ever be `Some(0)` is
  a dead abstraction; the doc comment says as much out loud. Every call
  site uses it as `.is_some()`. A `bool` return would say the same thing
  with less ceremony.
- `splice_candidate` clones and fully sorts `later_punches` just to
  obtain its minimum. `later_punches.iter().min_by_key(|q| (q.at_utc, q.kind, q.id))`
  is allocation-free and O(n) instead of O(n log n). Irrelevant at real
  data sizes (a day's punches), but it runs twice per `classify_at`,
  which itself runs up to 7× per week × N weeks walked. Cosmetic.
- `classify_at:294` re-derives `has_anomaly` with a copy of the
  expression at `classify:167`. If the anomaly definition ever changes,
  the two will silently drift. A tiny private helper (or a
  `DayStints::recompute_has_anomaly(&mut self)`) removes the risk.

None of these affect behaviour.

---

## 7. Security

The changeset touches no argument parsing, no SQL construction, no shell
invocation, and no rendering of user-controlled text into anything
replayable. `classify_at` is pure; the new `storage::punches_for_date` /
`punches_in_range` calls pass `NaiveDate` values derived from already
validated dates through the same parameterised-query helpers the code
already used.

**CI e2e smoke block** (`.github/workflows/ci.yml:255-281`): reviewed in
full. It runs under the step's existing `set -euo pipefail`, uses its
own dedicated `$RUNNER_TEMP` database so it cannot collide with the
other smoke blocks, quotes every variable expansion (`"$midnight_db"`,
`"$mlm_bin"`, `"${RUNNER_TEMP}/…"`), and introduces **no** `${{ }}`
template interpolation of its own (the pre-existing
`${{ matrix.target }}` on line 66 comes from the workflow's own matrix,
not from anything a PR author controls). All `mlm` arguments are
hard-coded literals. Nothing injectable, nothing replayable. Clean.

I did verify the greps match real output: `grep -qE "Day total: *01h 15m"`
matches `Day total:     01h 15m`, and `status 2020-01-02` prints no
stint line at all, so the negative check passes. One portability nit —
finding 3.

---

## 8. Findings

**1 — `docs/dev/SPEC.md` §1.2/§4.3 and `README.md` "Known limitations"
are still pending (medium).** Both documents still assert, verbatim,
that these two defects are unfixed — `README.md:44-50` even says
"tracked, not yet fixed". `NOTES.md` and the `build_rows` doc comment
*are* done. Expected-pending per the coordinator's stated plan; flagged
so it isn't lost before release. The §6 checklist also asks that the
§4.3 rewrite call out §4.1's residual case (a legitimate carried-over
orphan sharing its date with one unrelated stray orphan still doesn't
splice) — worth not dropping, since it's the one place §1's "removes
both bullets" claim isn't unconditional.

**2 — §7's doc sweep is incomplete (low).** Beyond the §6 targets,
`docs/dev/SPEC.md:652-653` still describes the "cross-midnight
limitation (§1.2, E15): a session split across midnight leaves a stale,
permanently-open `start` on the earlier…", and `:801` still defines
**E15** as an unfixed edge case. In source, `src/stint.rs:788`'s test
header reads `// --- T10: cross-midnight is two separate dates (accepted
limitation) -`. The T10 *test* itself is still valid (it exercises plain
single-date `classify`, whose behaviour is unchanged) — only the
"accepted limitation" framing in the comment is now wrong. Fold these
into the same doc pass as finding 1.

**3 — CI: `\s` in `grep -qE` is not portable to the macOS runners
(low).** `.github/workflows/ci.yml:278` uses
`grep -qE "^\s*00:45"`. `\s` is a GNU grep extension; BSD grep (both
`macos-14` and `macos-15-intel` in the matrix) does not implement it in
ERE and will read it as a literal `s`, turning the pattern into
`^s*00:45`. Since `status`'s stint lines are indented, the check then
silently matches nothing and passes vacuously — a false negative on two
of five runners, never a false failure. This is also the only `\s` in
the entire workflow; every other e2e grep avoids it. Fix:
`grep -qE "^[[:space:]]*00:45"`, or simply `grep -q "00:45"` (the string
should not appear anywhere on that date's output).

**4 — `splice_candidate`'s `Option<usize>` return is a dead abstraction
(low, quality).** It can only ever return `Some(0)`, both call sites use
it as a boolean, and the doc comment concedes "the only possible index".
Returning `bool` would be clearer. Same function clones and sorts the
whole slice where `.iter().min_by_key(…)` would do. Cosmetic only.

**5 — `has_anomaly` recomputation is duplicated (low, quality).**
`src/stint.rs:294` copies the expression at `src/stint.rs:167`
(`open.len() > 1 || !orphaned_ends.is_empty()`), against `DayStints`'s
own doc claim that the precomputed field exists so "there is exactly one
definition of 'has an anomaly'". Extract a helper so the two cannot
drift.

**6 — the locked spec's §4.2 rationale for round 2's finding 1 is
technically wrong (informational).** "A bare `punches[0]` in the assert
condition … panics in every build profile — not compiled out in release"
is inaccurate: `debug_assert!` gates its entire condition behind
`cfg!(debug_assertions)`, so the index is not evaluated in release
builds. The defect was nevertheless real for debug and `cargo test`
builds, the requirement it produced is right, and the shipped code is
correct — only the justifying sentence is. Worth correcting if the spec
text is ever revisited; not worth reopening the spec for on its own.

### Non-findings, explicitly checked and cleared

- A spliced stint renders as `23:30-00:45` on the start date with no
  next-day marker. Spec §7 rules a "this crossed midnight" marker
  explicitly out of scope, so this is intended, not a defect.
- Per-iteration triple `punches_for_date` in `build_ledger` and the
  9-day padded fetch in `worked_minutes` — explicitly accepted by §4.3's
  performance reasoning, which I agree with (bounded, once per
  invocation, not a hot loop).
- No `classify` → `classify_at` call site was missed: grepping the crate
  shows `stint::classify(` remains only inside `stint.rs`'s own tests and
  in `classify_at`'s own implementation.

---

## 9. Verdict

**ship-with-followups.**

The two defects the changeset set out to fix are fixed, correctly, in
exactly the mechanically-forced scope §2 defines, with the round-1 and
round-2 findings genuinely resolved rather than nominally addressed.
The load-bearing tests fail when the implementation regresses — verified
by four mutations, not by reading. The full suite (435 tests) and
`clippy -D warnings` are green, and the binary behaves against real
SQLite exactly as the spec's worked examples say it should, including at
week boundaries and in every non-1:1 residual shape.

The one thing standing between this and an unqualified ship is finding 1:
`docs/dev/SPEC.md` and `README.md` still tell the reader these bugs are
unfixed. That doc pass (plus findings 2 and 3, which are one-line edits
that belong in the same commit) should land before release. Findings 4–6
are optional cleanups.
