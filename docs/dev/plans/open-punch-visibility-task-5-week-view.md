# Task 5 plan: `week` row markers and footer (`src/week_view.rs`)

## What was read

- `docs/dev/plans/open-punch-visibility-plan.md` (full changeset plan,
  all six task sections, both interface contracts, ordering/
  parallelization section).
- `docs/dev/specs/2026-09-26-open-punch-visibility.md` (full spec,
  §1-§5, especially §2 terms, §3.2 refused join, §3.5 `week`, §3.7).
- Sibling task plan files under `docs/dev/plans/`: **none of
  `open-punch-visibility-task-{1,2,3,4,6}-*.md` exist yet** (checked
  via `ls`). Task 1's predicate signatures and Task 2's `Anomalies`/
  footer shapes are therefore not pinned by a written plan I can read
  — this plan assumes signatures consistent with the changeset plan's
  Contract A/B prose (quoted inline below) and flags every assumption
  in the Dependencies table for verification once those tasks land.
- `src/week_view.rs` (full file, current state: `render_week`,
  `WeekRow`, `WeekView`, `build_rows`, `DbWeekData`, `build_week_view`,
  `run`, and the full test module).
- `src/stint.rs` (full file): `DayStints`, `classify`, `classify_at`,
  `splice_candidate` (`pub(crate)`, line 334), and their tests —
  this is the file Task 1 will add two predicates to.
- `src/render.rs` lines 1-168: `Anomalies` (`open_stint_count`,
  `orphaned_end_times`, `has_any()`, `detail_lines()`, `row_marker()`)
  — the file Task 2 will extend.
- `src/status.rs` lines 1-130, 420-460: `OpenStintAge` enum (the
  existing age-2 threshold: `target_date <= today - 2 days` ==
  `TwoOrMoreDaysBack`) and `resolve()`'s age computation
  (`target_date == today - ChronoDuration::days(1)`), and `to_anomalies`
  (line 319-325: converts orphan times via
  `.at_utc.with_timezone(&Local).time()`).
- `src/storage.rs` lines 380-419: `punches_for_date`,
  `punches_in_range` (inclusive `[from, to]` span, same ordering
  contract).
- `src/date.rs` lines 228-320ish: `WeekId::span()`, `WeekId::dates()`.
- `clippy.toml` (`cognitive-complexity-threshold = 15`).
- `.githooks/pre-commit` (fmt / cognitive-complexity / coverage-ratchet
  gate; the complexity check runs `cargo clippy --all-targets` with
  `-W clippy::cognitive_complexity`).

## Scope

**This task owns exclusively**: `src/week_view.rs`. Nothing else is
edited.

Per the changeset plan's task table, other files this task's code
depends on but must **not** touch:

| File | Owned by | What it must land before this task runs |
|---|---|---|
| `src/stint.rs` | Task 1 | `stale_open_start` and a refused-join predicate (Contract B) |
| `src/render.rs` | Task 2 | extended `Anomalies` fact fields + `has_any()`, the `week` footer literal |
| `src/status.rs` | Task 3 | nothing this task calls — runs in a separate worktree in parallel, no shared symbol |
| `src/commands.rs` | Task 4 | nothing this task calls |
| `docs/dev/SPEC.md`, `README.md` | Task 6 | nothing this task calls |

Per the changeset plan's ordering diagram, Task 5 depends on Task 1
and Task 2 **landing first** (merged to the integration branch) before
this task's implementation begins; it then runs in parallel with
Task 3 against Contract B, in its own worktree, touching no file
Task 3 touches. By the time this task is actually implemented, Task
1's and Task 2's real code exists — the "dependency not landed" case
below is about *this planning session*, not about the implementation
window.

No handoff is owed to any other task: `week_view.rs` is a pure
consumer of Task 1/Task 2's public surface, and nothing here changes
`WeekArgs`, `main.rs`'s dispatch, or any type another task's file
imports.

## Two questions the dispatch asked, answered

**Does the refused-join check need neighbor dates outside the week?**
No new fetch is needed. `build_week_view` already calls
`storage::punches_in_range(conn, start.pred_opt().unwrap(),
end.succ_opt().unwrap())` (current line 244) — one day padding on
each side of the week's own 7 dates. `build_rows` already looks up
each row's `prev_bucket`/`next_bucket` from that same padded fetch via
`date.pred_opt()`/`date.succ_opt()` (current lines 140-147). A refused
join between date `D` and `D+1` needs only `D`'s and `D+1`'s own raw
per-date classifications (`stint::classify`, not `classify_at`) —
Contract B is explicit that the predicate takes "raw single-date
classifications... never `classify_at`-resolved ones," and `classify`
takes only one date's own punches, no neighbors of its own. So for
Monday's row, the "previous date" it checks against is the Sunday
just before the week (covered by `start.pred_opt()`), and for
Sunday's row the "next date" is the Monday just after
(`end.succ_opt()`) — both already inside the existing widened fetch.
No second layer of padding, no new storage call.

**Does the stale check need `today` passed in?** Yes, and it already
is: `build_week_view` computes `let today = now.date_naive();` (current
line 238) and threads it into `build_rows(dates, &punches, today,
now_utc)` (current line 245). `build_rows` already carries `today`
as a parameter used for `is_ongoing`; the new per-row anomaly
computation reuses that same parameter, no new plumbing.

## Dependencies (not yet landed — verify signatures at integration)

| Symbol (assumed) | Assumed signature | Stub to develop this task's tests against today | Swap at integration |
|---|---|---|---|
| `stint::stale_open_start` | `pub(crate) fn stale_open_start(day: &DayStints, date: NaiveDate, today: NaiveDate) -> bool` — per Contract B: "takes a date's classification plus that date and today, answers whether its lone open start is stale." Formula per spec §2/§3.1: `day.open.len() == 1 && (today - date).num_days() >= 2`. | Add a private `fn stub_stale_open_start(day: &DayStints, date: NaiveDate, today: NaiveDate) -> bool { day.open.len() == 1 && (today - date).num_days() >= 2 }` next to `row_anomalies` (below), used only until Task 1 lands. | Delete the stub; `use crate::stint::stale_open_start;` and call it in place of `stub_stale_open_start`. If Task 1's landed signature takes the age as a caller-supplied `i64` instead of computing it internally, adjust the one call site only. |
| `stint::refused_join` | `pub(crate) fn refused_join(earlier: &DayStints, later: &DayStints, later_punches: &[Punch]) -> bool` — mirrors `splice_candidate`'s parameter shape per Contract B: "the earlier date has at least one open start, the later date has at least one orphaned end, and the existing splice gate rejects the pair." Formula: `!earlier.open.is_empty() && !later.orphaned_ends.is_empty() && !splice_candidate(earlier.open.len(), later, later_punches)`. | Add a private `fn stub_refused_join(earlier: &DayStints, later: &DayStints, later_punches: &[Punch]) -> bool` with the formula above, calling the already-landed `stint::splice_candidate` (this one already exists, `pub(crate)`, line 334). | Delete the stub; `use crate::stint::refused_join;`. If Task 1 exposes this under a different name (e.g. `is_refused_join`) or bundles the "which side" naming into the return type instead of a bare `bool`, adjust `row_anomalies` accordingly — the direction (which date is `earlier`/`later`) is still decided by this task's two call sites either way. |
| `stint::age_in_days` (optional) | `pub(crate) fn age_in_days(date: NaiveDate, today: NaiveDate) -> i64` | Not stubbed separately — this task computes age inline as `(today - date).num_days()` wherever needed, since it is a one-line `chrono` expression. | If Task 1 lands a shared helper (likely, since Task 3's lookback needs the same age walk), swap the inline expression for the call — behaviorally identical, so this is a pure textual substitution, not a logic change. |
| `render::Anomalies` (extended) | New fields beyond today's `open_stint_count`/`orphaned_end_times`: `stale_open_start: Option<(NaiveTime, i64)>` (start time, age in days) and `refused_join_prev: Option<NaiveDate>` / `refused_join_next: Option<NaiveDate>` (per Contract A's fact list). `has_any()` extended to `open_stint_count > 1 \|\| !orphaned_end_times.is_empty() \|\| stale_open_start.is_some() \|\| refused_join_prev.is_some() \|\| refused_join_next.is_some()`. | Add a private test-only mirror struct `StubAnomalies` with the same fields and a `has_any()` matching the formula above, used only in this task's own unit tests until Task 2 lands (production code cannot compile against a struct that does not exist yet — see note below). | Once Task 2 lands, delete `StubAnomalies`; the production `row_anomalies` function is written directly against `render::Anomalies`'s real field names (whatever Task 2 actually calls them) — this is the one place in this task most likely to need a mechanical rename pass if Task 2's field names differ from the ones assumed here. |
| `render` footer literal | Assumed `pub const WEEK_ANOMALY_FOOTER: &str = "[!] needs a look: run mlm status DATE for details and fixes";` or an equivalent zero-argument function — spec §3.5's line is fixed text with no per-date substitution (`DATE` is literal, not a placeholder filled per row). | This task's own footer-presence tests can inline the literal string directly (spec-quoted) without importing anything, since the check is "does `render_week`'s output contain this exact line," not "does it call a specific Task 2 symbol." | At integration, `render_week` calls whatever Task 2 actually exports (constant or function) instead of a local `const` copy of the string, so the two files can never drift on this text. |

**Important compile-order note**: because `render::Anomalies` does not
yet have the new fields, this task's *production* code (not just
tests) cannot be written and compiled until Task 2 actually lands —
Contract A's fact-shape agreement is exactly what unblocks this. The
`StubAnomalies` row above is for drafting and unit-testing the
*predicate wiring logic* (which dates get checked, in which direction)
against Task 1's stub before Task 2 lands; the final `row_anomalies`
function this task ships is written directly against the real
`render::Anomalies`, not the stub. If Task 1 lands first and Task 2 is
still pending, this task can be fully implemented and tested for the
predicate-wiring half, with `row_anomalies`'s return type temporarily
`(bool /* has_anomaly */,)` or similar, then finished once Task 2
lands. If Task 2 lands first, the reverse: `row_anomalies` can be
written and tested against real `Anomalies` fields fed by the two
local stub predicates.

## Design

### Current structure (facts gathered above)

`build_rows` (`src/week_view.rs:126-169`) is a single closure body
inside `std::array::from_fn` that: buckets punches once, then per date
looks up `bucket`/`prev_bucket`/`next_bucket`, calls
`stint::classify_at`, builds an `Anomalies { open_stint_count:
day.open.len(), orphaned_end_times: ... }`, and constructs `WeekRow`.
`render_week` (`src/week_view.rs:72-116`) is a pure string builder:
header, headline, 7 row lines, blank line, 4 trailing label lines.

### New structure

1. Extract the per-row anomaly computation out of the `from_fn` closure
   into a named helper, `row_anomalies`, so `build_rows` stays a thin
   wiring function (keeps both functions well under the complexity-15
   budget — seeing the existing closure body already carries ~6
   statements plus a struct literal, adding 3-4 more branches inline
   risks the cognitive-complexity gate).

```rust
/// Builds the full anomaly fact set for one row's date (Contract A):
/// the existing E7/E8 facts plus (new) whether the date's lone open
/// start is stale, and whether it forms a refused join with either
/// calendar neighbor. Refused-join checks use each date's *raw*
/// single-date classification (`stint::classify`), never the
/// splice-resolved `day` passed in — matching Contract B exactly, so
/// `status` and `week` can never disagree on a date pair.
fn row_anomalies(
    day: &DayStints,
    prev_bucket: &[Punch],
    bucket: &[Punch],
    next_bucket: &[Punch],
    date: NaiveDate,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
) -> Anomalies {
    let raw_prev = stint::classify(prev_bucket, now_utc);
    let raw_day = stint::classify(bucket, now_utc);
    let raw_next = stint::classify(next_bucket, now_utc);

    let stale_open_start = stint::stale_open_start(day, date, today)
        .then(|| {
            let age = (today - date).num_days();
            (day.open[0].start.at_utc.with_timezone(&Local).time(), age)
        });

    let refused_join_prev = refused_join_neighbor(
        &raw_prev,
        &raw_day,
        bucket,
        date.pred_opt().expect("every calendar date has a predecessor"),
    );
    let refused_join_next = refused_join_neighbor(
        &raw_day,
        &raw_next,
        next_bucket,
        date.succ_opt().expect("every calendar date has a successor"),
    );

    Anomalies {
        open_stint_count: day.open.len(),
        orphaned_end_times: day
            .orphaned_ends
            .iter()
            .map(|o| o.punch.at_utc.with_timezone(&Local).time())
            .collect(),
        stale_open_start,
        refused_join_prev,
        refused_join_next,
    }
}

/// One direction of the refused-join check (Contract B: `earlier` has
/// the open start(s), `later` has the orphaned end(s)); returns the
/// neighbor date to name when it fires, `None` otherwise. Shared by
/// both call sites in `row_anomalies` so the two directions can never
/// diverge in how they interpret the predicate's result.
fn refused_join_neighbor(
    earlier: &DayStints,
    later: &DayStints,
    later_punches: &[Punch],
    neighbor_date: NaiveDate,
) -> Option<NaiveDate> {
    stint::refused_join(earlier, later, later_punches).then_some(neighbor_date)
}
```

2. `build_rows` changes only its per-row body to call the new helper:

```rust
std::array::from_fn(|i| {
    let date = dates[i];
    let bucket = by_date.get(&date).cloned().unwrap_or_default();
    let prev_bucket = date.pred_opt().and_then(|d| by_date.get(&d).cloned()).unwrap_or_default();
    let next_bucket = date.succ_opt().and_then(|d| by_date.get(&d).cloned()).unwrap_or_default();
    let day = stint::classify_at(&prev_bucket, &bucket, &next_bucket, now_utc);
    let anomalies = row_anomalies(&day, &prev_bucket, &bucket, &next_bucket, date, today, now_utc);

    WeekRow {
        weekday_abbrev: WEEKDAY_ABBREVS[i],
        date_display: format_date(date),
        minutes: day.completed_minutes(),
        is_ongoing: day.is_ongoing() && date == today,
        has_anomaly: anomalies.has_any(),
    }
})
```

No change to `WeekRow`'s or `WeekView`'s field sets — `has_anomaly` is
still the only anomaly-related field on a row, per this task's "no new
column, no per-row detail beyond the existing marker" scope limit.

3. `render_week` gains the footer, appended after the table's trailing
   blank line and before the Carry-in/Worked/Fulfillment/Target block
   — re-read §3.5: "one footer line follows the table after a blank
   line," and the worked example in the spec shows it standing alone,
   not interleaved with the trailing totals block. Placing it between
   the row block's existing blank line (current line 99, `out.push('\n')`
   right after the row loop) and the totals block satisfies "after the
   table, after a blank line" without adding a second blank line before
   the totals block on the anomaly-free path (the no-anomaly path must
   stay byte-identical to today's output, so the footer's insertion
   point must be additive-only, never restructuring the existing blank
   lines).

```rust
for row in &view.rows {
    // ... unchanged ...
}
out.push('\n');

if view.rows.iter().any(|r| r.has_anomaly) {
    out.push_str(render::WEEK_ANOMALY_FOOTER); // Task 2's literal — swap name at integration
    out.push('\n');
    out.push('\n');
}

for (label, minutes) in [ ... ] { ... }
```

Wait — re-check line count contract: A12's test asserts
`out.matches('\n').count() == 16` and `lines.len() == 17` for the
*no-anomaly* golden examples. Adding the `if` block only executes when
`has_anomaly` is true for at least one row, so the no-anomaly path's
newline count is untouched — A12 keeps passing unchanged. On the
anomaly path, the footer adds exactly 2 lines (the footer line itself,
plus a blank line separating it from the totals block, per "one footer
line follows the table after a blank line" — the blank line belongs
*before* the footer, separating it from the table; re-reading §3.5's
placement once more: "a blank line" then "one footer line", so the
blank line goes first, then the footer, with no second blank line
needed before Carry-in since the footer is the last "extra" line):

```rust
if view.rows.iter().any(|r| r.has_anomaly) {
    out.push_str(render::WEEK_ANOMALY_FOOTER);
    out.push('\n');
    out.push('\n'); // separates the footer from the trailing totals block
}
```

This keeps the existing single blank line (row loop's own trailing
`out.push('\n')`, current line 99) as the "blank line" the spec means,
then appends the footer line, then one more blank line before Carry-in
so the totals block's own formatting is untouched. Net effect on an
anomalous week: +2 lines over today's 16-line layout (footer text +
one blank line), landing at 18 lines total, table -> blank -> footer
-> blank -> totals.

### Imports needed

`src/week_view.rs`'s existing `use crate::render::{self, Anomalies};`
already covers the extended `Anomalies` struct and the footer
constant/function once Task 2 lands — no new `use` line required
beyond what is already there, assuming Task 2 keeps both in
`render.rs`'s existing public surface (very likely, since Contract A
puts both there explicitly).

## Test plan

One subsection per this task's acceptance criterion (changeset plan
§Task 5), each also citing the spec case it exercises.

### AC1 — stale/refused-join rows get `[!]`

Spec §3.1 (stale open start is an anomaly), §3.2 (refused join is an
anomaly on both dates).

- `b_stale_date_row_is_marked`: seed a week where Monday has exactly
  one open start dated `today - 3 days` (`age >= 2`); assert
  `rows[0].has_anomaly` is true, and every other row is false (mirrors
  existing `c9_anomaly_marker_end_to_end_and_exit_zero`'s isolation
  style).
- `b_stale_yesterday_not_marked`: a lone open start at age 1 (target
  date = `today - 1`) does not mark the row — regression against
  §3.1's "yesterday's lone open start... stay non-anomalies."
- `b_refused_join_both_rows_marked_same_week`: seed two open starts on
  date `D` (so `D`'s raw classification has 2 opens — a shape
  `splice_candidate` rejects on `earlier_open_count != 1`... **note**:
  per Task 1's acceptance criteria, refused join requires "at least
  one open start" on the earlier date and the gate's own rejection;
  the simplest positive fixture is the shape spec's own worked example
  uses: one open start on `A`, and on `A+1` an orphaned end that is
  *not* `A+1`'s first punch (so `splice_candidate` rejects on the
  "chronologically first punch" check, not the count check) — seed
  `A`: one open start; `A+1`: an ordinary start/end pair before an
  orphaned end. Assert both `A`'s row and `A+1`'s row (when both fall
  in the same displayed week) carry `[!]`.
- `b_refused_join_across_week_boundary_still_marks_boundary_row`:
  refused join between the week's last date (Sunday) and the following
  Monday (outside the displayed week) — assert Sunday's row is marked
  even though the other date never appears in `view.rows` (exercises
  the padded-fetch reasoning above; the Monday-side row is simply not
  rendered, which is correct — `week` only ever shows 7 rows).
- `b_clean_splice_still_no_anomaly`: reuse/extend
  `b2_bucketing_now_splices_across_midnight` (existing golden case) to
  confirm the new refused-join predicate does not fire on a shape
  `splice_candidate` already accepts — regression, not new behavior.

### AC2 — exactly one footer line, present iff any row marked

Spec §3.5 ("one footer line follows the table after a blank line...
No footer when no row is marked").

- `a_footer_present_when_any_row_anomalous`: one view with exactly one
  marked row; assert `render_week` output contains
  `"[!] needs a look: run mlm status DATE for details and fixes"`
  exactly once.
- `a_footer_absent_when_no_row_anomalous`: `example_a()`/`example_b()`
  unmodified; assert the footer text is absent (also covered
  incidentally by the byte-exact golden tests below, but an explicit
  `!out.contains(...)` assertion documents the intent directly).
- `a_footer_line_count_when_present`: assert the anomalous-week output
  has exactly 18 lines (16 + footer + blank), not 17 or 19, pinning
  the exact placement decided above.

### AC3 — no lookback in `week`

Spec §3.5 ("No lookback lines in `week`"); this is an absence, not a
new behavior — enforced structurally by never calling any lookback
function from this file (nothing in Task 3's `status.rs` is imported
here). No dedicated runtime test is meaningful beyond the golden
byte-exact tests already confirming the output contains nothing beyond
the table/footer/totals block.

### AC4 — regression: anomaly-free week byte-identical; previously-unmarked stale/refused week now marked

- `a1_current_week_golden_byte_exact` / `a3_past_week_golden_full_table_not_a_one_liner`
  (existing, unmodified fixtures) must keep passing verbatim — no
  fixture in this task's diff touches `EXAMPLE_A_GOLDEN`/
  `EXAMPLE_B_GOLDEN`.
- `b_stale_date_previously_unmarked_now_marked`: construct the exact
  §7.2-superseded shape (a lone open start at age >= 2, previously
  rendered with no marker at all, per the changeset plan's regression
  note) and assert the row *is* marked under the new code — this is
  the one test that must fail against the pre-Task-5 code and pass
  after, proving the behavior actually changed.

### AC5 — direct test coverage summary

Already covered above: stale-date row marked (AC1), refused-join both
rows marked when co-resident in one displayed week (AC1), footer
presence/absence both directions (AC2), no regression to existing
E7/E8 marking (`b4_orphaned_end_contributes_nothing`,
`b6_multi_open_flags_the_row`, `b7_zero_length_stint_no_anomaly` all
kept unmodified and re-asserted to still pass).

## Risks and ambiguities

1. **Field-name/shape guesses for `render::Anomalies` and the footer
   are unpinned.** No Task 2 plan exists yet to read; this plan's
   assumed field names (`stale_open_start`, `refused_join_prev`,
   `refused_join_next`, `WEEK_ANOMALY_FOOTER`) are my best-fit reading
   of Contract A's prose, not a confirmed interface. The Dependencies
   table above is written so a mechanical rename is the only fallout
   if Task 2 lands with different names — no logic in `row_anomalies`
   depends on the exact field names beyond construction syntax.
2. **Redundant `classify()` calls.** `row_anomalies` calls
   `stint::classify` on `prev_bucket`/`bucket`/`next_bucket`
   independently for every one of the 7 rows, even though e.g. row
   `i`'s `next_bucket` classification duplicates row `i+1`'s `bucket`
   classification. `classify()` is O(n log n) in that date's punch
   count, and a week's per-date punch count is small (single digits
   in ordinary use), so this is not a performance concern — flagged
   only in case a future reviewer wonders why the same date gets
   classified more than once per `build_rows` call.
3. **Refused-join fixture construction is fiddlier than E7/E8's.**
   The three refused-join shapes in the spec's test plan (two opens on
   `A`, orphan not first on `A+1`, two orphans on `A+1`) all need
   *both* dates' punches hand-built with specific tie-break ordering
   (`(at_utc, kind, id)`) to land the orphan/start counts exactly
   right — the existing `punch()`/`punch_from_local` test helpers
   already in `week_view.rs`'s test module are sufficient, but each
   fixture needs care that its "which shape triggers rejection" claim
   is actually correct against `splice_candidate`'s real logic (I
   traced this by hand against `src/stint.rs` lines 334-346 above,
   not just against the spec's prose).
4. **The footer's exact placement (one blank line before it, one
   blank line after, no blank line duplicated) is my own reading of
   §3.5's terse "after a blank line" wording**, not something the
   spec spells out byte-by-byte the way §3.4's `status` line shapes
   are spelled out. If Task 6 (docs) or a later review finds the
   quoted README/SPEC example expects a different blank-line count,
   this task's `render_week` insertion point needs a one-line
   adjustment, not a redesign — the insertion is additive and
   self-contained.
5. **No disagreement with the changeset plan or spec.** This task's
   design follows Contract A/B and §3.5 directly; the one judgment
   call not fully dictated by either document is the footer's
   blank-line placement (risk 4 above), which I resolved by preserving
   byte-identical no-anomaly output as the harder constraint and
   picking the placement that satisfies it most simply.
