# Task 2 low-level plan: call-site rewiring (`src/status.rs`, `src/week_view.rs`)

> **Archived — historical planning record.** Not authoritative. Current spec: `docs/dev/SPEC.md`. Design/process history: `docs/dev/NOTES.md`.


**Read in full before writing this plan**:
- `docs/dev/plans/boundary-stint-pairing-plan.md` (changeset plan, all
  317 lines)
- `docs/dev/specs/2026-09-19-boundary-stint-pairing.md` (spec, all 482
  lines)
- `src/status.rs`, current, in full (1220 lines, including its
  `#[cfg(test)]` module)
- `src/week_view.rs`, current, in full (937 lines, including its
  `#[cfg(test)]` module, with particular attention to
  `c7_never_touched_week_carries_forward`/
  `c8_multi_week_idle_gap_carry_end_to_end` at lines 851-887)
- `src/storage.rs` lines 379-412 (`punches_for_date`/`punches_in_range`
  exact signatures)
- `clippy.toml` (cognitive-complexity-threshold = 15) and
  `.githooks/pre-commit` (fmt / cognitive-complexity / coverage-vs-
  baseline gates; coverage check degrades to SKIP without
  `cargo-llvm-cov`+`jq`)
- `.github/workflows/ci.yml` (254 lines; the e2e smoke job's existing
  structure/style, especially the backdated-punch block)

## Scope

This plan owns `src/status.rs` and `src/week_view.rs` exclusively,
including their `#[cfg(test)]` modules. It does **not** touch
`src/stint.rs` — that file is Task 1's, already shipped and merged by
the time this task starts. `classify_at` is treated here as an
already-published dependency with the signature pinned in the
changeset plan's "Interface contract to pin before either task starts"
section:

```rust
pub fn classify_at(
    prev_punches: &[Punch],
    punches: &[Punch],
    next_punches: &[Punch],
    now: DateTime<Utc>,
) -> DayStints
```

Never panics for any combination of empty/non-empty
prev/punches/next. With both neighbors empty, reduces exactly to
`classify(punches, now)`'s output. This plan does not re-derive or
re-justify that contract; it only relies on it.

## Verified current line numbers (spec citations checked against source)

The spec's own citations (§4.3) are approximate; here are the actual
current lines as read:

| Site | Spec's citation | Actual |
|---|---|---|
| `status.rs::resolve`'s `classify` call | line ~339 | line 339 (`let day = stint::classify(&punches, now_utc);`) — matches exactly |
| `status.rs::build_ledger`'s walk | lines ~291-305 | lines 291-305 (`let mut date = start_week.start();` through `date += ChronoDuration::days(1);`) — matches exactly |
| `week_view.rs::build_rows` | lines ~118-157 | doc comment 118-121, fn body 122-157 |
| `week_view.rs::build_week_view` | lines ~205-236 | fn body 205-236 — matches exactly |
| `week_view.rs::DbWeekData::worked_minutes` | lines ~182-193 | fn body 182-193 — matches exactly |
| `c7`/`c8` fixtures | `src/week_view.rs:851-887` | lines 851-887 (`c7_never_touched_week_carries_forward` 852-867, `c8_multi_week_idle_gap_carry_end_to_end` 870-887) — matches exactly |

All of the spec's line citations check out against the current tree
(no drift since the spec was written). Good — no correction needed
there.

**Current imports** (both files, exact):

`src/status.rs:28-38`:
```rust
use chrono::{DateTime, Datelike, Duration as ChronoDuration, Local, NaiveDate, NaiveTime, Utc};
use rusqlite::Connection;
use std::collections::{BTreeMap, BTreeSet};

use crate::date::{self, WeekId};
use crate::render::{self, Anomalies};
use crate::stint::{self, DayStints};
use crate::storage;
use crate::time::format_minutes;
use crate::week::{self, WeekAccounting, WeekLedger};
use crate::week_target;
```
`stint::{self, DayStints}` is already imported — `stint::classify_at`
needs no new `use` line, just a changed call (`stint::classify_at(...)`
in place of `stint::classify(...)`).

`src/week_view.rs:19-31`:
```rust
use std::collections::HashMap;

use chrono::{DateTime, Local, NaiveDate, Utc};
use rusqlite::Connection;

use crate::cli::WeekArgs;
use crate::date::{WeekId, format_date, parse_week_id};
use crate::render::{self, Anomalies};
use crate::stint;
use crate::storage::{self, Punch};
use crate::time::format_minutes;
use crate::week::{self, WeekData, WeekTargets};
use crate::week_target;
```
Same story: `stint` is already imported unqualified enough that
`stint::classify_at(...)` needs no new `use`.

**`storage.rs` signatures pinned** (lines 379/396-400):
```rust
pub fn punches_for_date(conn: &Connection, date: NaiveDate) -> Result<Vec<Punch>, StorageError>
pub fn punches_in_range(conn: &Connection, from: NaiveDate, to: NaiveDate) -> Result<Vec<Punch>, StorageError>
```
Both take `&Connection` and return `Result<Vec<Punch>, StorageError>`
— `?` propagates through `anyhow::Result` call sites exactly as today
(no signature friction).

## Grep sweep for stale cross-midnight/E15/"out of scope" comments

Ran (both files, `-i` on `E15|cross-midnight|out of scope|out-of-scope|
midnight`):

```
src/week_view.rs:121:/// cross-midnight pairing out of scope (E15).
src/week_view.rs:613:    // B2 (E15)
src/week_view.rs:615:    fn b2_bucketing_does_not_pair_across_midnight() {
```

`src/status.rs` has zero hits — nothing there asserts this scope
limit in prose. `src/week_view.rs` has exactly one production-code
hit (the `build_rows` doc comment, line 121, addressed below) and one
test (`b2_bucketing_does_not_pair_across_midnight`, line 613-635) whose
**name and assertions**, not just a comment, assert the old scope
limit. That test is now testing behavior this changeset deliberately
changes — see "B2 test disposition" below, this is a finding, not a
silent rename.

## Call site 1 — `src/status.rs::resolve`

Current (lines 335-339):
```rust
    let punches = storage::punches_for_date(conn, target_date)?;
    let notes = storage::notes_for_date(conn, target_date)?;

    let now_utc = now.with_timezone(&Utc);
    let day = stint::classify(&punches, now_utc);
```

New:
```rust
    let punches = storage::punches_for_date(conn, target_date)?;
    let notes = storage::notes_for_date(conn, target_date)?;
    let prev_punches = storage::punches_for_date(conn, target_date.pred_opt().unwrap())?;
    let next_punches = storage::punches_for_date(conn, target_date.succ_opt().unwrap())?;

    let now_utc = now.with_timezone(&Utc);
    let day = stint::classify_at(&prev_punches, &punches, &next_punches, now_utc);
```

Placement: insert the two new fetches immediately after the existing
`punches`/`notes` fetches, before `now_utc` is computed — keeps every
I/O call grouped together, matches constraint 7 (wiring change only,
no reordering of unrelated logic). `NaiveDate::pred_opt()`/
`succ_opt()` per constraint 6; `.unwrap()` is acceptable per spec
§4.3's closing paragraph (only unrepresentable at `NaiveDate`'s literal
range edge, never for real stored data).

No other line in `resolve` changes. `day_total_minutes`,
`has_open_stint`, `to_anomalies(&day)`, `build_stint_lines(&day)`
(lines 341-390) all consume `day` exactly as before — `classify_at`
returns the same `DayStints` shape `classify` did, so nothing
downstream needs to change.

## Call site 2 — `src/status.rs::build_ledger`

Current (lines 291-305):
```rust
    let mut date = start_week.start();
    let last_date = through.end();
    while date <= last_date {
        let day_punches = storage::punches_for_date(conn, date)?;
        let day_notes = storage::notes_for_date(conn, date)?;
        if !day_punches.is_empty() || !day_notes.is_empty() {
            let week = WeekId::from_date(date);
            data_weeks.insert(week);
            if !day_punches.is_empty() {
                let classified = stint::classify(&day_punches, now_utc);
                *worked_by_week.entry(week).or_insert(0) += classified.completed_minutes();
            }
        }
        date += ChronoDuration::days(1);
    }
```

New:
```rust
    let mut date = start_week.start();
    let last_date = through.end();
    while date <= last_date {
        let day_punches = storage::punches_for_date(conn, date)?;
        let day_notes = storage::notes_for_date(conn, date)?;
        if !day_punches.is_empty() || !day_notes.is_empty() {
            let week = WeekId::from_date(date);
            data_weeks.insert(week);
            if !day_punches.is_empty() {
                let prev_punches = storage::punches_for_date(conn, date.pred_opt().unwrap())?;
                let next_punches = storage::punches_for_date(conn, date.succ_opt().unwrap())?;
                let classified =
                    stint::classify_at(&prev_punches, &day_punches, &next_punches, now_utc);
                *worked_by_week.entry(week).or_insert(0) += classified.completed_minutes();
            }
        }
        date += ChronoDuration::days(1);
    }
```

Deliberate placement choice: the two new fetches sit **inside** the
`if !day_punches.is_empty()` branch, not fetched unconditionally at
the top of the loop body. Reasoning: `classify_at`'s neighbor data
only matters for the splice check, and the splice check is only
reachable when `day_punches` is non-empty to begin with (an empty
`punches` bordering a non-empty neighbor is a well-defined no-panic
no-splice no-op per constraint 4, so it would be *correct* either way
— but fetching prev/next unconditionally for every idle day in a
possibly-long ledger walk is two wasted queries per idle day for no
behavioral gain). This is not a rolling-window optimization (the spec
explicitly disclaims those) — it's simply not fetching data that
provably cannot change the result for that iteration, and it costs
nothing to justify since it's `if`-scoped already-existing structure,
not new bookkeeping. **This is a judgment call beyond the spec's
literal text** — flagged in Risks below since the spec's own prose
("fetch the previous and next calendar date's punches at each
iteration") could be read as literally unconditional. I judge the
conditional placement compliant with the *intent* (no rolling-window
optimization = don't try to reuse a previous iteration's fetch across
iterations; this isn't that) but call it out for the coordinator to
weigh in on if there's a reason it must be unconditional (e.g. a
future test asserting query count).

Cognitive-complexity note: `build_ledger`'s loop body gains two more
statements inside the same `if` branch. Given clippy.toml's
`cognitive-complexity-threshold = 15`, this is worth a quick
`cargo clippy -- -W clippy::cognitive_complexity` check against just
this function once written — no branching structure is added (no new
`if`/`match`), only two more sequential statements, so it should stay
well under threshold, but confirm rather than assume.

## Call site 3 — `src/week_view.rs::build_week_view` / `build_rows`

### `build_week_view` (lines 205-236)

Current (line 215):
```rust
    let punches = storage::punches_in_range(conn, start, end)?;
    let rows = build_rows(dates, &punches, today, now_utc);
```

New:
```rust
    let punches =
        storage::punches_in_range(conn, start.pred_opt().unwrap(), end.succ_opt().unwrap())?;
    let rows = build_rows(dates, &punches, today, now_utc);
```

`build_rows`'s own signature (`dates: [NaiveDate; 7], punches: &[Punch],
today: NaiveDate, now_utc: DateTime<Utc>`) does not change — it already
takes the full punch slice and buckets internally; widening what's in
that slice is transparent to its signature (constraint 7: no
call-site public-signature changes beyond threading the two extra
fetches — here even that isn't needed, since `build_rows` builds its
own `by_date` map from whatever it's handed).

### `build_rows` (lines 122-157)

Current:
```rust
fn build_rows(
    dates: [NaiveDate; 7],
    punches: &[Punch],
    today: NaiveDate,
    now_utc: DateTime<Utc>,
) -> [WeekRow; 7] {
    let mut by_date: HashMap<NaiveDate, Vec<Punch>> = HashMap::new();
    for p in punches {
        by_date.entry(p.date).or_default().push(*p);
    }

    std::array::from_fn(|i| {
        let date = dates[i];
        let bucket = by_date.get(&date).cloned().unwrap_or_default();
        let day = stint::classify(&bucket, now_utc);

        // Landmine 2: go through `render::Anomalies` exclusively, never
        // `day.has_anomaly()`/`day.anomalies()` directly.
        let anomalies = Anomalies {
            open_stint_count: day.open.len(),
            orphaned_end_times: day
                .orphaned_ends
                .iter()
                .map(|o| o.punch.at_utc.time())
                .collect(),
        };

        WeekRow {
            weekday_abbrev: WEEKDAY_ABBREVS[i],
            date_display: format_date(date),
            minutes: day.completed_minutes(),
            is_ongoing: day.is_ongoing() && date == today,
            has_anomaly: anomalies.has_any(),
        }
    })
}
```

New:
```rust
fn build_rows(
    dates: [NaiveDate; 7],
    punches: &[Punch],
    today: NaiveDate,
    now_utc: DateTime<Utc>,
) -> [WeekRow; 7] {
    let mut by_date: HashMap<NaiveDate, Vec<Punch>> = HashMap::new();
    for p in punches {
        by_date.entry(p.date).or_default().push(*p);
    }

    std::array::from_fn(|i| {
        let date = dates[i];
        let bucket = by_date.get(&date).cloned().unwrap_or_default();
        let prev_bucket = date
            .pred_opt()
            .and_then(|d| by_date.get(&d).cloned())
            .unwrap_or_default();
        let next_bucket = date
            .succ_opt()
            .and_then(|d| by_date.get(&d).cloned())
            .unwrap_or_default();
        let day = stint::classify_at(&prev_bucket, &bucket, &next_bucket, now_utc);

        // Landmine 2: go through `render::Anomalies` exclusively, never
        // `day.has_anomaly()`/`day.anomalies()` directly.
        let anomalies = Anomalies {
            open_stint_count: day.open.len(),
            orphaned_end_times: day
                .orphaned_ends
                .iter()
                .map(|o| o.punch.at_utc.time())
                .collect(),
        };

        WeekRow {
            weekday_abbrev: WEEKDAY_ABBREVS[i],
            date_display: format_date(date),
            minutes: day.completed_minutes(),
            is_ongoing: day.is_ongoing() && date == today,
            has_anomaly: anomalies.has_any(),
        }
    })
}
```

Notes on this diff:
- `date.pred_opt()` returns `Option<NaiveDate>`; chained with
  `.and_then(|d| by_date.get(&d).cloned())` this collapses "date
  arithmetic overflowed" and "no bucket for that neighbor date" into
  the same `unwrap_or_default()` — both cases correctly want `&[]`
  (or here, `vec![]`), and per the interface contract `classify_at`
  never panics on an empty neighbor regardless of which reason
  produced it. This is *not* the same `.unwrap()` pattern used at the
  two `build_week_view`/`resolve`/`build_ledger` sites — there,
  `pred_opt()`/`succ_opt()` failing would mean the **current date
  under classification** itself is at `NaiveDate`'s range edge, which
  per spec §4.3's closing paragraph is fine to `.unwrap()`. Here, a
  neighbor lookup failing just means "there is no such neighbor date"
  — for the two boundary dates of the padded 9-day fetch range this
  is expected to be `None` only at the literal `NaiveDate` range edge
  (never in practice), so `unwrap_or_default()` is strictly more
  defensive than needed but costs nothing and avoids a second
  `.unwrap()` idiom doing a subtly different job in the same function.
  This is a **deliberate deviation from constraint 6's literal
  "pred_opt()/succ_opt() ... `.unwrap()`" phrasing** for the neighbor-
  lookup case specifically — constraint 6 is about never using the
  panicking `pred()`/`succ()`/`Days` arithmetic, which this still
  honors; it doesn't mandate `.unwrap()` over a graceful
  `unwrap_or_default()` at a call site where "no such date" is itself
  a valid, already-handled input shape. Flagged in Risks for the
  coordinator/reviewer to confirm this reading is acceptable.
- `by_date` already holds the two padding-day buckets (Mon-1 and
  Sun+1) because `build_week_view` now fetches the widened 9-day
  range — `build_rows` itself needs no new fetch, just the two new
  lookups per iteration, exactly as spec §4.3 describes ("`build_rows`
  looks up `date.pred_opt()`/`date.succ_opt()` in the same `by_date`
  map it already builds").
- The padding-day buckets are **never** turned into their own
  `WeekRow`s — `std::array::from_fn` still only produces 7 rows, one
  per `dates[i]`, unchanged. The padding days exist solely as
  neighbor-lookup fodder inside this closure, never rendered directly.
  This is what keeps `build_rows`'s own contract (exactly 7 rows) firm
  while widening what feeds the splice check.

### `build_rows`'s stale doc comment (spec §4.4)

Current (lines 118-121):
```rust
/// Contract 4: bucket `punches` by their `date` column, then pair each of
/// the week's 7 dates independently via `stint::classify` — one call per
/// date, never a whole week's punches in one call, which is what keeps
/// cross-midnight pairing out of scope (E15).
```

New:
```rust
/// Contract 4: bucket `punches` by their `date` column, then classify
/// each of the week's 7 dates independently via `stint::classify_at`,
/// looking up each date's immediate neighbors (`date.pred_opt()`/
/// `date.succ_opt()`) in the same bucket map — `punches` is expected to
/// already span one day past each end of the week (SPEC.md §4.3's
/// widened `punches_in_range` fetch in `build_week_view`), so a
/// midnight-spanning stint that starts or ends just outside the week's
/// own 7 dates still resolves correctly at the boundary date it touches.
```

The replacement keeps the "one call per date" framing (still true —
`classify_at` is still called exactly once per one of the 7 dates, not
once for the whole week's punches at once) while removing the false
"out of scope (E15)" claim and stating what actually keeps it correct
now: the padded fetch plus per-date neighbor lookup, not a scope
limitation.

### B2 test disposition — `b2_bucketing_does_not_pair_across_midnight` (E15)

This is a **finding, not a silent edit**: `b2` (lines 613-635) is
named and written to assert the exact behavior this changeset removes
— it directly seeds a Tue 23:30 `start` / Wed 00:45 `end` pair and
asserts `tue_row.minutes == 0`, `!tue_row.has_anomaly` (a single open
stint, non-anomalous) and `wed_row.minutes == 0`,
`wed_row.has_anomaly` (orphaned end, flagged). Once `build_rows` calls
`classify_at`, this exact fixture is now the clean-splice case: it
will produce `tue_row.minutes == 75`, `wed_row.minutes == 0`, and
**neither** row anomalous. The test's current assertions would fail,
correctly — this is signal, not a false negative.

Resolution: rename and rewrite it in place, keeping the same fixture
shape (it's a good minimal midnight-spanning fixture) but asserting
the new outcome:

```rust
// B2 (superseded E15 — now the ordinary midnight-splice case)
#[test]
fn b2_bucketing_now_splices_across_midnight() {
    use PunchKind::{End, Start};
    let tue = d(2026, 2, 10);
    let wed = d(2026, 2, 11);
    let punches = vec![punch(1, tue, (23, 30), Start), punch(2, wed, (0, 45), End)];
    let rows = build_rows(
        week_dates(),
        &punches,
        d(2026, 2, 9),
        now_utc(2026, 2, 11, 1, 0),
    );
    let tue_row = &rows[1];
    let wed_row = &rows[2];
    assert_eq!(tue_row.minutes, 75, "23:30-00:45 spans midnight, lands on Tue");
    assert!(!tue_row.has_anomaly, "clean splice, no anomaly");
    assert_eq!(wed_row.minutes, 0, "the spliced stint never appears on Wed");
    assert!(!wed_row.has_anomaly, "the orphan is consumed by the splice");
}
```

Do not delete the old assertions silently — this rewrite (old
assertions replaced with their exact opposite, same fixture) is itself
the acceptance evidence that the splice reaches all the way through
`build_rows`, so it doubles as one of the §5.3 integration tests
("`week`'s per-day ... totals include the spliced stint's minutes on
the correct (starting) day only") rather than being redundant with a
new one. See Test plan below — I reuse this test for that bullet
rather than adding a near-duplicate.

## Call site 4 — `src/week_view.rs::DbWeekData::worked_minutes`

Current (lines 182-193):
```rust
    fn worked_minutes(&self, week: WeekId) -> i64 {
        let (start, end) = week.span();
        let punches = storage::punches_in_range(self.conn, start, end).unwrap_or_default();
        let mut by_date: HashMap<NaiveDate, Vec<Punch>> = HashMap::new();
        for p in punches {
            by_date.entry(p.date).or_default().push(p);
        }
        by_date
            .values()
            .map(|bucket| stint::classify(bucket, self.now_utc).completed_minutes())
            .sum()
    }
```

New (the one call site round 2 flagged as most likely to be gotten
wrong — written out in full, not sketched):
```rust
    fn worked_minutes(&self, week: WeekId) -> i64 {
        let (start, end) = week.span();
        let padded_start = start.pred_opt().unwrap();
        let padded_end = end.succ_opt().unwrap();
        let punches =
            storage::punches_in_range(self.conn, padded_start, padded_end).unwrap_or_default();
        let mut by_date: HashMap<NaiveDate, Vec<Punch>> = HashMap::new();
        for p in punches {
            by_date.entry(p.date).or_default().push(p);
        }

        week.dates()
            .into_iter()
            .map(|date| {
                let bucket = by_date.get(&date).cloned().unwrap_or_default();
                let prev_bucket = date
                    .pred_opt()
                    .and_then(|d| by_date.get(&d).cloned())
                    .unwrap_or_default();
                let next_bucket = date
                    .succ_opt()
                    .and_then(|d| by_date.get(&d).cloned())
                    .unwrap_or_default();
                stint::classify_at(&prev_bucket, &bucket, &next_bucket, self.now_utc)
                    .completed_minutes()
            })
            .sum()
    }
```

Concrete, deliberate points, since this is the line the changeset plan
calls out as most likely to be gotten wrong:

1. `by_date.values().map(...).sum()` — the **entire old iteration
   strategy** — is gone. There is no `.values()` call anywhere in the
   new body. The replacement iterates `week.dates()` (the fixed
   7-element `[NaiveDate; 7]` array `build_rows` already keys off,
   constraint 5's exact required pattern), not the padded `HashMap`'s
   keys.
2. `by_date` still holds up to 9 dates' worth of buckets after the
   widened fetch (the week's own 7, plus `padded_start`, plus
   `padded_end`), but it is now used **only** via `.get(&date)` /
   `.get(&d)` lookups keyed by dates this function itself chooses
   (`week.dates()` and their immediate neighbors) — never enumerated.
   This is the exact shape that makes leaking an adjacent week's
   padding-day stint into this week's total structurally impossible:
   a padding day's bucket is reachable from this function only as
   *someone's* `prev_bucket`/`next_bucket` neighbor argument to
   `classify_at`, which by the interface contract consumes it only to
   decide whether to *splice into* one of the week's own 7 dates
   (never to add its own separate total) — it can never itself
   contribute a standalone `completed_minutes()` figure to this sum.
3. Same `pred_opt()`/`succ_opt()`/`unwrap_or_default()` neighbor-
   lookup idiom as `build_rows` above, for the same reason (a
   neighbor's absence, whether "no data that day" or the literal
   `NaiveDate` range edge, is already the correct `&[]` case per the
   interface contract) — kept textually identical between the two
   functions on purpose, so a future reader sees the same idiom twice
   rather than two different-looking but equivalent ways of doing it.
4. `week.dates()` returns `[NaiveDate; 7]` (confirmed at
   `src/date.rs:317`); `.into_iter()` on a fixed-size array yields
   owned `NaiveDate` items (Rust 2021+ array `IntoIterator`), matching
   what `build_rows` already relies on implicitly via
   `std::array::from_fn`/indexing — no new trait bound or import
   needed.
5. Cognitive complexity: this closure body is straight-line (three
   lookups, one call, no branching) — should sit comfortably under the
   threshold-15 gate, but worth the same one-off
   `cargo clippy -- -W clippy::cognitive_complexity` spot check as
   `build_ledger` above, since `worked_minutes` picked up meaningfully
   more code than before.

No signature change to `worked_minutes` itself (`fn worked_minutes(&self, week: WeekId) -> i64`, the `WeekData` trait method) — constraint 7 holds.

## Test plan

One subsection per Task 2 acceptance criterion (changeset plan,
"Task 2" section), each citing its spec §5.3 case.

### AC: `status.rs::resolve` fetches neighbor punches, calls `classify_at`

Spec case: "`status` on each half of a midnight-spanning pair shows
the correct completed/no-anomaly state" (§5.3 bullet 1).

New tests in `src/status.rs`'s `mod tests`:
- `resolve_midnight_splice_earlier_date_shows_completed_stint_no_anomaly`:
  seed `Start` at `23:30` on some date `D`, `End` at `00:45` on
  `D.succ_opt()`. Call `resolve(Some(<D's ISO string>), <some `now`
  after both dates>, &conn)`. Assert: `view.day_total_minutes == 75`,
  `view.anomaly_lines.is_empty()`, `view.stints.len() == 1` with
  `stints[0].start == 23:30` and `stints[0].end == StintEnd::At(00:45)`
  — note the *rendered* end time on `D`'s own status view is `D+1`'s
  clock time, since `StintLine`'s `end` is just the raw `NaiveTime`
  the `Local` end instant falls on; this is expected and matches how
  any completed stint already renders (§7.1 doesn't special-case a
  spliced stint's display, per spec §2/§7 — no new render marker).
- `resolve_midnight_splice_later_date_shows_no_orphan_anomaly`: same
  seed, call `resolve` for `D+1` instead. Assert:
  `view.anomaly_lines.is_empty()` (the `00:45` orphan is consumed),
  `view.stints.is_empty()` (the spliced stint belongs to `D`, never
  appears in `D+1`'s own list per spec §4.1), `view.day_total_minutes
  == 0`.
- `resolve_midnight_boundary_still_flags_non_1to1_shapes`: a direct
  regression companion — seed `D` with **two** trailing opens (e.g.
  `Start 22:00`, `Start 23:00`, nothing closing either) and `D+1` with
  a single `End 00:30`. Assert `resolve(D)` still shows the E7-style
  2-open anomaly (`view.anomaly_lines.len() == 1` containing "2 open
  stints") and `resolve(D+1)` still shows nothing splicing in
  (`day_total_minutes == 0`, and if `D+1` has no other punches, no
  anomaly on that side either — the `00:30 End` stays a genuine
  orphan). Confirms the new call site doesn't accidentally always
  splice; only ever mechanically, per the 1:1 gate Task 1 owns.

### AC: `status.rs::build_ledger` fetches prev/next per iteration, no rolling window

No direct spec §5.3 bullet names `build_ledger` by name, but it's
exercised transitively by every `resolve_*` test above (`resolve`
always calls `build_ledger` for its week accounting) plus:
- `resolve_midnight_splice_reflected_in_week_line`: seed the same
  midnight-spanning pair as above, landing entirely inside one week
  (both `D` and `D+1` in the same `WeekId`). Call `resolve(D)` and
  assert `view.week_line` reflects the recovered 75 minutes in
  `fulfillment` (e.g. via the same `week_line` test helper pattern
  used by `t11_golden_full_first_spec_example`, comparing against a
  `WeekAccounting` built with the expected `fulfillment`). This is the
  direct evidence `build_ledger`'s per-iteration `classify_at` call
  (not just `resolve`'s own top-level one) picked up the spliced
  minutes into the week total — `build_ledger` is where `worked_by_week`
  is accumulated, so a bug isolated to `build_ledger` (e.g. an
  off-by-one in which iteration owns the splice) would only show up
  here, not in the two tests above (which only check the single-date
  `day` view, not the week accounting fed by the ledger walk).

### AC: `week_view.rs::build_rows`/`build_week_view` widen the fetch, splice via `by_date` lookups

Spec case: bullet 2 ("`week`'s per-day and week-aggregate totals
include the spliced stint's minutes on the correct (starting) day
only") and bullet 3 ("a splice landing exactly on a week boundary...").

- `b2_bucketing_now_splices_across_midnight` (rewritten in place, see
  above) — covers the in-week-interior case directly at the
  `build_rows` unit level (no DB, pure function).
- New: `b9_week_boundary_splice_lands_in_correct_weeks_rows_only`
  (unit-level, `build_rows` directly, mirroring `b2`'s style): seed a
  splice landing exactly on the `week_dates()`-derived week's Sunday →
  next Monday boundary (`week_dates()` is `wk(2026, 7).dates()`, so
  Sunday is `d(2026, 2, 15)`, next Monday `d(2026, 2, 16)`) — `Start
  23:00` on the Sunday, `End 00:20` on the Monday (outside this week's
  own 7 dates, but present in `punches` since `build_rows` is handed
  whatever `punches` slice the test constructs directly — no DB
  needed for this unit test, it can just include the Monday punch in
  the `punches` vec passed to `build_rows` even though `dates` is
  Mon-Feb-9..Sun-Feb-15). Assert `rows[6]` (Sunday) shows the 80
  minutes, no anomaly; there is no 8th row to accidentally show it
  again — the array is fixed at `[WeekRow; 7]`, so "not leaking into
  the wrong week's rows" is structurally enforced at this level and
  the test's job is just to confirm the *right* row (Sunday, not some
  other row or a panic) gets the minutes.
- New (integration, DB-backed, mirrors `c1`-style tests):
  `c13_week_boundary_splice_end_to_end_lands_in_starting_weeks_view_only`:
  seed via `storage::insert_punch` a `Start` at `23:00` on
  `wk(2026, 7)`'s Sunday and an `End` at `00:20` the next day (which
  falls in `wk(2026, 8)`). Call `build_week_view` for `wk(2026, 7)`:
  assert the Sunday row's minutes include the 80 and `worked_minutes ==`
  (whatever the week's total should be, including those 80). Call
  `build_week_view` for `wk(2026, 8)`: assert its Monday row shows 0
  minutes and no anomaly (the punch that would have been Monday's own
  orphan is consumed by the splice into the previous week), and its
  `worked_minutes` does not include the 80. This is the direct
  cross-week non-leak assertion the spec's bullet 3 asks for, driven
  through the real `punches_in_range`-widened fetch, not just the
  unit-level `build_rows` call.

### AC: `week_view.rs::DbWeekData::worked_minutes` sums over `week.dates()`, not `by_date.values()`

Spec case: bullet 4, "`DbWeekData::worked_minutes` padding-day
isolation" (round 2 finding 2) — the fixture the changeset plan
explicitly says `c7`/`c8` don't cover (they seed fully-idle padding
weeks).

**Concrete fixture** (new test,
`c14_worked_minutes_padding_day_isolation`):

```rust
#[test]
fn c14_worked_minutes_padding_day_isolation() {
    let conn = new_test_db();
    let week = wk(2026, 7); // Mon 2026-02-09 .. Sun 2026-02-15
    let dates = week.dates();

    // An ordinary, unrelated, fully self-contained stint on the padding
    // day immediately BEFORE the week's own span (Sun 2026-02-08 --
    // outside `week`'s own 7 dates, but inside `worked_minutes`'s
    // widened 9-day fetch). No splice involved: starts and ends the
    // same UTC/local date, nothing open, nothing orphaned.
    let padding_before = dates[0].pred_opt().unwrap(); // 2026-02-08
    storage::insert_punch(&conn, PunchKind::Start, padding_before, t(9, 0), &TZ_UTC).unwrap();
    storage::insert_punch(&conn, PunchKind::End, padding_before, t(11, 0), &TZ_UTC).unwrap(); // 120m

    // Same on the padding day immediately AFTER the week's span.
    let padding_after = dates[6].succ_opt().unwrap(); // 2026-02-16
    storage::insert_punch(&conn, PunchKind::Start, padding_after, t(9, 0), &TZ_UTC).unwrap();
    storage::insert_punch(&conn, PunchKind::End, padding_after, t(10, 30), &TZ_UTC).unwrap(); // 90m

    // One ordinary in-week stint too, so the assertion isn't just "0
    // in, 0 out" -- confirms the real week total is exactly the
    // in-week figure, neither padding day's minutes folded in.
    storage::insert_punch(&conn, PunchKind::Start, dates[2], t(9, 0), &TZ_UTC).unwrap(); // Wed
    storage::insert_punch(&conn, PunchKind::End, dates[2], t(17, 0), &TZ_UTC).unwrap(); // 480m

    let now = now_utc(2026, 2, 15, 20, 0);
    let data = DbWeekData { conn: &conn, now_utc: now };
    let this_week = data.worked_minutes(week);
    assert_eq!(this_week, 480, "padding-day stints must not leak in");

    // And the mirror check: each padding day's own week sees ITS
    // stint, undiminished and un-doubled.
    let week_before = WeekId::from_date(padding_before); // wk 2026-06
    let week_after = WeekId::from_date(padding_after);   // wk 2026-08
    assert_eq!(data.worked_minutes(week_before), 120);
    assert_eq!(data.worked_minutes(week_after), 90);

    // Total across the three adjacent weeks equals the sum of the
    // three independent stints exactly once each -- the direct
    // anti-double-count assertion.
    assert_eq!(
        data.worked_minutes(week_before) + this_week + data.worked_minutes(week_after),
        120 + 480 + 90
    );
}
```

Why this fixture specifically satisfies round 2 finding 2, unlike
`c7`/`c8`: `c7`/`c8` (lines 852-887) seed only a fully-idle padding
week — the old buggy `.values().sum()` code would produce the exact
same (zero) contribution from an idle padding day as the fixed
`.dates()`-based code, so neither test can distinguish the two
implementations. Here, `padding_before`/`padding_after` each carry a
real, nonzero, ordinary (non-spliced) stint — under the old
`by_date.values().sum()` code, `this_week` would wrongly include both
120 and 90 (406 too high... actually 480+120+90=690), and
`week_before`'s own `worked_minutes` call would *also* independently
include its own padding day's neighbor (the in-week Wednesday stint,
480, leaking the other direction too) — i.e. the bug is caught by
`this_week != 480` alone, but the three-way sum assertion is the
sharper, unambiguous "double-counted across two weeks' totals" check
the spec's bullet 4 asks for by name.

`c7`/`c8` (lines 851-887) are re-run unmodified as part of the full
suite per the acceptance criteria — confirmed still passing, not
sufficient alone (as the changeset plan already states), superseded
for *this specific* finding by `c14` above, not replaced.

### AC: `build_rows`'s stale doc comment rewritten; grep sweep clean

Covered above (doc comment replacement text given in full; grep
results listed; `b2` renamed/rewritten as the one test-level
consequence found).

### AC: `NaiveDate` arithmetic uses `pred_opt()`/`succ_opt()` with `.unwrap()`, never `pred()`/`succ()`/`Days`

No new test needed — this is enforced by code review / grep, not a
runtime assertion. As a cheap belt-and-suspenders check, run (after
writing the code):
```
grep -n '\.pred()\|\.succ()\|Days::new\|+ Days\|- Days' src/status.rs src/week_view.rs
```
expect zero hits outside pre-existing unrelated code (none currently
exist in either file per this session's read).

### AC: CI e2e smoke addition

Sketch, styled to match the existing backdated-punch block
(`.github/workflows/ci.yml` lines 79-113) — same isolated-DB pattern,
same `-d`/`--date`-backed punch style, inserted as a new block
immediately after that one (before the "delete note/punch" block that
currently starts at line 115), since it's the closest existing
precedent for scripted date-backed punches:

```yaml
          # --- midnight-spanning splice (boundary-stint-pairing spec):
          # a fresh, separate db, kept independent of the others above
          # so the two straddled dates can't collide with unrelated data. ---
          midnight_db="${RUNNER_TEMP}/mlm-ci-smoke-midnight.db"

          echo "--- midnight: start/stop straddling midnight via -d ---"
          MLM_DB_PATH="$midnight_db" "$mlm_bin" start -d 2020-01-01 23:30
          MLM_DB_PATH="$midnight_db" "$mlm_bin" stop -d 2020-01-02 00:45

          echo "--- midnight: status on the start date shows the completed total ---"
          MLM_DB_PATH="$midnight_db" "$mlm_bin" status 2020-01-01 | tee "${RUNNER_TEMP}/status-midnight-start.out"
          grep -qE "Day total: *01h 15m" "${RUNNER_TEMP}/status-midnight-start.out"
          if grep -q '\[!\]' "${RUNNER_TEMP}/status-midnight-start.out"; then
            echo "expected no anomaly on the start date, but one was rendered" >&2
            exit 1
          fi

          echo "--- midnight: status on the end date shows no anomaly ---"
          MLM_DB_PATH="$midnight_db" "$mlm_bin" status 2020-01-02 | tee "${RUNNER_TEMP}/status-midnight-end.out"
          if grep -q '\[!\]' "${RUNNER_TEMP}/status-midnight-end.out"; then
            echo "expected no orphaned-end anomaly on the end date, but one was rendered" >&2
            exit 1
          fi
          if grep -qE "^\s*00:45" "${RUNNER_TEMP}/status-midnight-end.out"; then
            echo "expected the spliced stint not to appear as its own line on the end date" >&2
            exit 1
          fi
```

Notes on this sketch:
- `-d 2020-01-01`/`-d 2020-01-02` matches the existing block's fixed
  far-past absolute date style (`2020-01-01`) rather than the `-N`
  relative style used later in the file — deliberately, since a
  relative `-N` pair straddling midnight risks the same "local
  midnight rollover between calls" residual risk the file's own
  comment (lines 93-100) already accepts for the *existing* `-N` block,
  and there's no reason to add a second instance of that risk when an
  absolute two-date pair sidesteps it entirely.
  `01h 15m` is `23:30`→`00:45` = 75 minutes, matches `format_minutes`'s
  `HHh MMm` style used throughout the file's existing `grep -qE`
  patterns (e.g. line 90, `08h 00m`).
- Placed as its own isolated `$midnight_db`, consistent with every
  other block in this file (backdate/delete/delete-oob/delete-flag/
  delete-quote all use their own DB) — never reuses an earlier block's
  DB, so this addition can't be perturbed by nor perturb unrelated
  state.
- Exact insertion point: I recommend directly after line 113 (the end
  of the existing backdated-punch block, right before the `# --- delete
  note/punch (milestone 14)` comment at line 115) since it's the
  thematically closest existing block (date-backed punches) and keeps
  all "punch semantics" blocks grouped before the "delete" blocks
  begin. This is a style judgment, not dictated by the spec — flagged
  in Risks in case the coordinator wants it placed elsewhere (e.g. at
  the very end, after all existing blocks, to minimize diff blast
  radius against a large existing file). I'd lean towards **end of
  file** instead, actually, purely to keep the diff to a pure append
  and avoid any risk of subtly perturbing an existing block via a
  misplaced insertion in a 254-line shell script — noting this as the
  safer default unless there's a strong reason to group thematically.

## Summary of new/changed test functions

`src/status.rs`:
- `resolve_midnight_splice_earlier_date_shows_completed_stint_no_anomaly` (new)
- `resolve_midnight_splice_later_date_shows_no_orphan_anomaly` (new)
- `resolve_midnight_boundary_still_flags_non_1to1_shapes` (new)
- `resolve_midnight_splice_reflected_in_week_line` (new)

`src/week_view.rs`:
- `b2_bucketing_does_not_pair_across_midnight` → renamed/rewritten to
  `b2_bucketing_now_splices_across_midnight` (existing, rewritten)
- `b9_week_boundary_splice_lands_in_correct_weeks_rows_only` (new)
- `c13_week_boundary_splice_end_to_end_lands_in_starting_weeks_view_only` (new)
- `c14_worked_minutes_padding_day_isolation` (new)
- `c7_never_touched_week_carries_forward`,
  `c8_multi_week_idle_gap_carry_end_to_end` — unmodified, re-confirmed
  passing (no code change needed; listed here only because the
  changeset plan's acceptance criteria name them explicitly).

## Risks, ambiguities, and disagreements

1. **`build_ledger`'s fetch placement (conditional vs. unconditional)**:
   I placed the two new `punches_for_date` calls inside the
   `if !day_punches.is_empty()` branch rather than unconditionally at
   the top of the loop body. This is functionally equivalent (an idle
   day can never produce a splice-affecting result — `classify_at`
   with an empty `punches` and any neighbors is a documented no-op per
   constraint 4) and avoids two wasted queries per idle day in a
   possibly-long walk, but the spec's literal sentence ("fetch the
   previous and next calendar date's punches at each iteration") could
   be read as wanting it unconditional for uniformity/simplicity. I
   believe the conditional placement is correct and still fully
   compliant with "no rolling-window optimization" (that phrase is
   about not caching/reusing fetches *across* iterations, which this
   doesn't do), but flag it since it's a place I made a judgment call
   the spec didn't fully pin down.
2. **`unwrap_or_default()` vs. `.unwrap()` at neighbor lookups in
   `build_rows`/`worked_minutes`**: constraint 6 says every call site
   uses `pred_opt()`/`succ_opt()` "matching the existing overflow
   discipline" — I read this as "never the panicking `pred()`/`succ()`
   or `Days` arithmetic," not literally "always follow with
   `.unwrap()`." At the two *fetch-range* call sites (`resolve`,
   `build_ledger`, `build_week_view`, `worked_minutes`'s own padded
   fetch) I do use `.unwrap()`, matching the spec's own worked
   examples verbatim. At the two *neighbor-bucket-lookup* sites inside
   `build_rows`/`worked_minutes`'s per-date closure, a missing
   neighbor is an expected, already-meaningful case (no such date, or
   no data that date) that the interface contract already treats as
   `&[]`, so I use `unwrap_or_default()` there instead of unwrapping
   into a panic. I'm confident this is correct behavior, but it's a
   textual deviation from constraint 6's literal wording worth a
   reviewer's explicit sign-off rather than assuming it's obviously
   fine.
3. **`b2` test rename is a behavior-reversing edit, not a pure
   addition**: flagged prominently above already, repeating here since
   it's the one place this task edits an *existing* test's assertions
   to their logical opposite rather than only adding new tests or
   editing a doc comment. This is necessary (the old assertions
   describe behavior the spec explicitly requires removed) but is
   exactly the kind of edit the changeset plan's constraint 1 ("no
   existing non-tied-group test may be edited to pass") warns about —
   note that constraint 1 is scoped to `src/stint.rs`'s tests
   specifically (Task 1's file), not this file, so it does not
   literally forbid this edit; I flag it anyway since it's the same
   *shape* of risk (an existing green test silently flipped) even
   though it's outside constraint 1's literal file scope.
4. **CI block placement** (thematic grouping after the backdate block
   vs. pure end-of-file append): stated a preference for end-of-file
   above; either is defensible, flagging so the coordinator/reviewer
   picks one rather than it being an unstated implementation detail.
5. **No test for `build_ledger`'s *own* prev/next fetch in isolation**:
   `build_ledger` is private (`fn build_ledger`, not `pub`), so it can
   only be exercised through `resolve` in this file's existing test
   style (no direct unit tests of `build_ledger` exist today either) —
   `resolve_midnight_splice_reflected_in_week_line` is the closest
   available proxy. If a bug were isolated to *only* `build_ledger`'s
   per-iteration splice (e.g. correct for `resolve`'s own top-level
   `classify_at` call but wrong inside the ledger walk), this test
   would catch it, but there's no lower-level unit test possible
   without either making `build_ledger` `pub(crate)` (out of scope —
   constraint 7 forbids unnecessary signature/visibility changes) or
   duplicating its logic in a test harness. Acceptable given the
   existing test style already accepts this level of indirection for
   `build_ledger`.
6. **Performance**: per spec §4.3's own closing paragraph and the
   changeset plan's constraints, this task explicitly does not attempt
   any rolling-window optimization for `build_ledger`'s per-iteration
   fetches nor `worked_minutes`'s per-week-visited widened fetch — this
   plan follows that instruction as given, not flagging it as a risk
   of its own, just confirming no optimization was silently added or
   silently skipped-with-a-TODO.

No disagreement with the spec's or changeset plan's substance — the
above are implementation judgment calls within the pinned contract,
not objections to the design.
