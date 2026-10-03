# Task plan: possible-issues-triage, T4 Carry-in anomaly scan

Changeset plan: `docs/dev/plans/possible-issues-triage-plan.md` (T4 section, contracts 4, 6 and 7, decision F8). Spec: `docs/dev/specs/2026-10-03-possible-issues-triage.md` §3.3 and §7.

## What I read

- `docs/dev/plans/possible-issues-triage-plan.md`, full.
- `docs/dev/specs/2026-10-03-possible-issues-triage.md`, full.
- `docs/dev/plans/possible-issues-triage-task-1-foundation.md`, full. Its "Published seams" section is the signature source for everything below. Nothing from T1 has landed, so I plan against it as if present.
- Sibling plans T5 and T6 exist and touch no file of mine. The T2 plan (`possible-issues-triage-task-2-overlap.md`) did not exist when I wrote this. I used only the changeset plan's contract for it.
- `src/week.rs` 87-115 (`WeekLedger` derives `Default`, has `new()`), `src/date.rs` (`WeekId` API at 228-360: `from_date`, `start`, `end`, `Ord`, `Display`), `src/stint.rs` 330-405 (`stale_open_start`, `is_refused_join`: both fire only on open starts or orphaned ends), `src/storage.rs` 238-262 (`punch_from_local`, `PunchKind`), `src/db.rs` notes schema (54-62), `src/week_view.rs` 60-125 (current `render_week`, footer logic), `src/render.rs:375` (`WEEK_ANOMALY_FOOTER`).
- `tests/status_cli.rs` 1-60 (helpers `mlm`, `mlm_tz`, `seed_punch`, `stdout`, `stderr`) and `tests/e6_db_failure.rs` 1-60 (binary-driving pattern).
- `clippy.toml` (cognitive complexity threshold 15), `.githooks/pre-commit` (fmt check, complexity gate, coverage ratchet against `coverage-baseline.json`).
- AGENTS.md, via the dispatch context.

## Scope

Files this task owns exclusively:

| File | Action |
|---|---|
| `src/carry_scan.rs` | replace T1's stub body with the scan. Signature unchanged. Add `#[cfg(test)] mod tests`. |
| `tests/carry_marker_cli.rs` | new |

Files I must not touch:

| File | Owner |
|---|---|
| `src/anomalies.rs`, `tests/overlap_cli.rs` | T2 |
| `src/history.rs`, `src/render.rs` (marker type, line text, clean-pair helpers), `src/status.rs`, `src/week_view.rs`, `src/lib.rs`, `src/storage.rs`, `src/week.rs` | T1 (merged before I start) |
| `src/lookback.rs`, `src/commands.rs` | T3 |
| `src/cli.rs`, `src/main.rs`, `src/week_target.rs` | T5 |
| `docs/dev/SPEC.md`, `README.md`, `AGENTS.md` | T6 |
| NOTES.md, the spec file, `coverage-baseline.json` | coordinator |

If the work needs any other file, stop and report. T1 pinned that this task changes no output code.

## Dependencies

| Dependency | State | Test against today | Swap at integration |
|---|---|---|---|
| T1 `history::{History, PunchBuckets, load_history}`, `anomalies::date_facts`, `render::CarryMarker`, `src/carry_scan.rs` stub, both renderings in `status` and `mlm week` | not landed | Build against the signatures in T1's "Published seams". Until T1 merges, the code does not compile. Open the worktree from merged T1, as the changeset plan orders. | Nothing to swap. T1 is the base of my branch. |
| T2 overlap detection inside `anomalies::date_facts` | not landed | All tests except one use E7, stale, E8 and refused join, which T1's `date_facts` already detects. | After rebasing onto merged T2, add the one overlap-only test (CLI test `overlap_only_earlier_week_raises_marker` and unit test `overlap_only_date_counts_once_t2_merged`). No import changes, because the scan calls `date_facts`, which T2 extends in place. |

## Facts gathered

- T1 stub: `pub fn carry_marker(history: &History, shown: WeekId, today: NaiveDate, now_utc: DateTime<Utc>) -> Option<CarryMarker>` returns `None` (T1 plan, seam 4). Imports in the stub: `chrono::{DateTime, NaiveDate, Utc}`, `crate::date::WeekId`, `crate::history::History`, `crate::render::CarryMarker`.
- `CarryMarker { weeks: usize, oldest: WeekId }`, both public (T1 seam 4).
- `PunchBuckets` holds only dates with punches. `dates_in(from, to)` is ascending and empty when `from > to` (T1 seam 3). `trio(date)` returns the previous, own and next buckets as `DateBuckets`.
- The loader's buckets cover one day before the walk start through one day after the shown week's end (T1 seam 3). The walk start is `min(earliest data week, shown)`. Every scanned date `D` runs from the earliest date through `shown.start() - 1`. Both neighbours of every scanned date are therefore inside the buckets. `D - 1` is at least the walk start minus one. `D + 1` is at most `shown.start()`.
- A `mlm week` row for an earlier week `W` uses the same loader and `trio`, so its previous and next buckets are identical to mine, including the Sunday row whose next bucket is Monday of the following week. Same predicate, same inputs, so the marker and the `[!]` rows cannot disagree.
- `History.earliest_date` counts notes too. A notes-only earliest date yields a scan range with no punch dates, so no marker. The walk anchor is unaffected, which is T1's business.
- `WeekId` is `Ord`, so a `BTreeSet<WeekId>` gives "oldest" and "count" directly.
- `stale_open_start` and `is_refused_join` fire only on open starts or orphaned ends (`src/stint.rs:370-405`). Any such date in the past is therefore stale or E7 or E8 on its own. See the ambiguity section on isolating a refused-join date.
- Complexity limit is 15 per function (`clippy.toml`).

## Design

The scan is three small functions. Nothing reads storage. No connection is in the signature.

### `src/carry_scan.rs`

```rust
//! Carry-in anomaly scan (SPEC §3.3).

use std::collections::BTreeSet;

use chrono::{DateTime, NaiveDate, Utc};

use crate::anomalies;
use crate::date::WeekId;
use crate::history::History;
use crate::render::CarryMarker;

/// Earlier weeks that hold at least one date whose `mlm week` row would
/// carry `[!]`. Returns `None` when there are none, when no data exists,
/// or when `shown` is the earliest week or before all data.
pub fn carry_marker(
    history: &History,
    shown: WeekId,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
) -> Option<CarryMarker> {
    let earliest = history.earliest_date?;
    let last = shown.start().pred_opt()?;
    let weeks = anomalous_weeks(history, earliest, last, today, now_utc);
    let oldest = *weeks.first()?;
    Some(CarryMarker { weeks: weeks.len(), oldest })
}

/// Distinct weeks with an anomalous date in `[from, to]`, ascending.
/// Dates run ascending, so a week already flagged skips its remaining
/// dates. Only the count of weeks and the oldest one matter.
fn anomalous_weeks(
    history: &History,
    from: NaiveDate,
    to: NaiveDate,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
) -> BTreeSet<WeekId> {
    let mut weeks = BTreeSet::new();
    for date in history.buckets.dates_in(from, to) {
        let week = WeekId::from_date(date);
        if weeks.contains(&week) {
            continue;
        }
        if date_is_anomalous(history, date, today, now_utc) {
            weeks.insert(week);
        }
    }
    weeks
}

/// The same predicate a `mlm week` row uses for its `[!]`.
fn date_is_anomalous(
    history: &History,
    date: NaiveDate,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
) -> bool {
    anomalies::date_facts(&history.buckets.trio(date), date, today, now_utc).has_any()
}
```

Notes on the sketch:

- `earliest` bounds the scan from below. Because `dates_in` yields only punch dates, an earliest date that comes from a note changes nothing.
- When `shown` precedes all data, `earliest > last`, so `dates_in` is empty, `weeks` is empty and the result is `None`. T1 guarantees `dates_in` never panics on `from > to`.
- When `shown` is the earliest week, `last < earliest`. Same path, `None`. The shown week's own anomalies never count.
- `BTreeSet::first` needs Rust 1.66. Check `Cargo.toml` `rust-version` or the toolchain before relying on it. If older, use `weeks.iter().next()`.
- Cognitive complexity: `carry_marker` about 3 (two `?`, one more `?`), `anomalous_weeks` about 5 (for, two ifs, nesting), `date_is_anomalous` 0. Far below 15.
- Cost: every punch date before the shown week costs one `date_facts` call (four classifications) in memory, minus dates skipped by the per-week short-circuit. This is the cost the changeset plan accepts. Report the timing, see Risks.

### Contract check against T1's renderings

I change no rendering. For review, the behavior I rely on from T1 is:

- `status`: marker line is the last entry of `anomaly_lines`, after the date block and today's lookback lines.
- `mlm week`: `Carry-in:      <figure>  [!]` plus the marker line in the footer slot. With a marked row, the order is rows, blank, `[!] needs a look: run mlm status DATE for details and fixes`, marker line, blank, `Carry-in:` line. With no marked row it is rows, blank, marker line, blank, `Carry-in:` line (T1 plan step 5, contract 6).

## Steps

TDD throughout. Write each step's tests first and watch them fail for the right reason (with the stub returning `None`, each positive test fails on a missing marker). Run `cargo fmt` before each commit. Revert any hook rewrite of `coverage-baseline.json` before the final commit.

1. Open the worktree from merged T1. Add the unit tests below to `src/carry_scan.rs` (keeping the stub's `None` body). Confirm the positive tests fail and `stub_returns_none` is deleted or rewritten (T1's `carry_scan::tests::stub_returns_none` must be removed, because the stub is gone).
2. Write `tests/carry_marker_cli.rs` against the spec's exact lines. Confirm the positive cases fail.
3. Implement the three functions above. Unit tests go green, then CLI tests.
4. Run the full AGENTS.md verification set, the hook's complexity command (`cargo clippy --all-targets --quiet --message-format=json -- -W clippy::cognitive_complexity`, no hits) and the smoke runs.
5. After T2 merges: rebase, add the two overlap-only tests, rerun everything. This is the only step that waits.

## Test plan, by acceptance criterion

### Shared fixtures

Unit tests (`src/carry_scan.rs` `mod tests`):

```rust
fn d(y: i32, m: u32, day: u32) -> NaiveDate
fn wk(y: i32, w: u32) -> WeekId { WeekId::new(y, w, "t").unwrap() }
fn p(id: i64, kind: PunchKind, date: NaiveDate, hhmm: (u32, u32)) -> Punch
    // storage::punch_from_local(id, kind, date, NaiveTime::from_hms_opt(h, m, 0).unwrap(), &Utc).unwrap()
fn history_of(punches: &[Punch], earliest: Option<NaiveDate>) -> History {
    History {
        ledger: WeekLedger::new(),
        buckets: PunchBuckets::from_punches(punches),
        earliest_date: earliest,
        live_minutes: 0,
    }
}
const TODAY: (2026, 2, 12);      // a Thursday, week 2026-07
fn now() -> DateTime<Utc>         // 2026-02-12 18:00 UTC
```

`History` has only public fields (T1 seam 3), so these hand-built histories involve no database. That also pins "the scan makes no storage reads" structurally, since `carry_marker` takes no connection.

CLI tests (`tests/carry_marker_cli.rs`): copy the `mlm_tz`, `seed_punch`, `stdout`, `stderr` helpers from `tests/status_cli.rs` (each test file has its own helpers in this repo). Pin `TZ=UTC` through `mlm_tz`, so seeded `at_utc` equals wall clock. Create the database by running any command first (`mlm week 2026-02` is enough), then insert rows raw. Week map for the fixtures (2026-01-05 is the Monday of week 2026-02, per T1's goldens):

| Week | Monday | Fixture date | Seeded punches (UTC) | Anomaly |
|---|---|---|---|---|
| 2026-02 | 01-05 | 01-06 | start 09:00, start 11:00 | E7 |
| 2026-03 | 01-12 | 01-13 | start 09:00 | stale (age above 2) |
| 2026-04 | 01-19 | 01-20 | end 10:00 | E8 |
| 2026-05 | 01-26 | 01-27 | start 09:00; plus 01-28 end 08:00, end 09:00 | refused join (two orphans on 01-28 refuse the splice) |
| 2026-06 | 02-02 | 02-03 | start 09:00, end 12:00, start 10:00, end 11:00 | overlap, T2 only |
| clean | any | any | start 09:00, end 17:00 | none |

Helper `marker_line(out) -> Option<&str>` returns the first stdout line that starts with `[!] carry-in includes anomalies`. Literal forms:

- plural: `[!] carry-in includes anomalies from 2 earlier weeks, oldest 2026-02 (see mlm week 2026-02)`
- singular: `[!] carry-in includes anomalies from 1 earlier week, 2026-02 (see mlm week 2026-02)`

### AC1. An anomaly in a week older than 14 days raises the marker on `status` and `mlm week` (§3.3, §7 "anomaly in a week older than 14 days")

- CLI `old_anomaly_raises_marker_on_status_and_week`: seed the 2026-02 E7 fixture only. Today is real clock (October 2026), so the anomaly is over 14 days old. `mlm status` stdout contains the singular line naming 2026-02. `mlm week` (current week) stdout contains the same line.
- CLI `status_past_date_shows_marker`: same database, `mlm status 2026-01-20`. The marker line is present. This also covers "any date".
- Unit `anomaly_far_in_past_raises_marker`: E7 on 2026-01-06, shown `wk(2026, 7)`, expect `Some(CarryMarker { weeks: 1, oldest: wk(2026, 2) })`.

### AC2. No marker for a clean history, no earlier data, or a notes-only earlier week (§3.3 "A week with no earlier data, or whose earlier weeks have no anomaly")

- Unit `clean_history_gives_none`: the clean fixture on one date, shown a later week, expect `None`.
- Unit `empty_history_gives_none`: `history_of(&[], None)`, expect `None`.
- Unit `notes_only_earlier_week_gives_none`: no punches, `earliest_date: Some(d(2026,1,5))`, shown `wk(2026, 7)`, expect `None`.
- CLI `clean_history_has_no_marker`: clean fixture in 2026-02. `mlm status` and `mlm week 2026-10` print no `carry-in includes` line.
- CLI `notes_only_earlier_week_has_no_marker`: insert a note row raw (`INSERT INTO notes ("date", body, created_at_utc) VALUES ('2026-01-05', 'x', '2026-01-05T09:00:00Z')`). `mlm week 2026-10` prints no marker line and no `[!]` on the Carry-in figure.
- CLI `no_marker_means_byte_identical_output`: with the clean fixture, `mlm week 2026-10` output has no line containing `[!]` and its line order is rows, blank, `Carry-in:`. This repeats T1's golden check for this scenario.

### AC3. Several weeks use the plural form with the oldest id, one week the singular (§3.3 text)

- CLI `two_anomalous_weeks_use_plural_form`: seed E7 in 2026-02 and stale in 2026-03. `mlm week 2026-10` stdout contains exactly the plural line (`2 earlier weeks, oldest 2026-02`). The same fixture with only 2026-03 gives the singular line naming 2026-03.
- Unit `counts_distinct_weeks_not_dates`: three anomalous dates in week 2026-02 and one in 2026-04, shown `wk(2026, 7)`, expect `weeks: 2, oldest: wk(2026, 2)`.
- Unit `several_anomalous_weeks_oldest_is_earliest`: anomalies in 2026-04, 2026-02 and 2026-05, inserted in that order, expect oldest `2026-02`, `weeks: 3`.

### AC4. Each anomaly kind in an earlier week raises the marker (§3.3 list: E7, stale, E8, refused join, overlap)

- Unit, one test each, shown `wk(2026, 7)`, each expecting `weeks: 1`:
  - `kind_e7_raises_marker`
  - `kind_stale_raises_marker`
  - `kind_e8_raises_marker`
  - `kind_refused_join_raises_marker`
  - `kind_overlap_raises_marker` (post-T2, step 5)
- CLI, one test each over the fixture table above, asserting the singular line names the right week id: `kind_e7_...`, `kind_stale_...`, `kind_e8_...`, `kind_refused_join_...`. The overlap case is `overlap_only_earlier_week_raises_marker`, added after T2 rebases. It also asserts `mlm week 2026-06` shows `[!]` on the 2026-02-03 row (a date that is only an overlap counts, matching the row marker), and that `mlm status 2026-02-03` shows the overlap block.
- Unit `overlap_only_date_counts_once_t2_merged` (post-T2): the one overlapping date, nothing else anomalous, expect `weeks: 1`.

### AC5. The marker uses the row predicate (§3.3 "never disagree")

- CLI `marker_weeks_each_have_a_marked_row`: seed all of the fixtures above except overlap (and the overlap one too after T2). For each week id in `[2026-02, 2026-03, 2026-04, 2026-05]` run `mlm week <id>` and assert at least one row line ends with `  [!]` and the footer line appears once. Then assert `mlm week 2026-10` reports a count equal to the number of those weeks and oldest `2026-02`.
- CLI `clean_weeks_are_not_counted`: a clean week between two anomalous weeks does not change the count, and `mlm week <clean week>` has no `[!]` row.
- Unit `scan_agrees_with_has_any_per_date`: for each fixture date build `anomalies::date_facts(..).has_any()` directly, and assert `carry_marker(..).is_some()` equals `any(has_any)` over those dates, with the shown week set to the one after the last fixture date.

### AC6. The marker does not depend on the shown week and excludes the shown week's own anomalies (§3.3, §6 last bullets)

- CLI `marker_shown_on_current_week_and_future_week`: E7 fixture in 2026-02. `mlm week` (current), `mlm week 2099-01` and `mlm status` all show the singular line.
- CLI `shown_week_own_anomaly_not_counted`: E7 in 2026-02 only. `mlm week 2026-02` has a marked row and footer but no marker line and no `[!]` on the Carry-in figure.
- CLI `later_anomaly_not_counted_for_earlier_week`: anomalies in 2026-02 and 2026-04. `mlm week 2026-03` shows the singular line for 2026-02 only. `mlm week 2026-04` shows singular for 2026-02 (its own anomaly is not counted).
- Unit `shown_week_anomaly_excluded`: E7 only in the shown week, shown equals that week, expect `None`.
- Unit `only_dates_before_shown_week_start_scanned`: anomaly on Sunday before the shown Monday, expect `Some`. Anomaly on the shown Monday, expect `None`. This pins the `shown.start() - 1` bound.
- Unit `future_shown_week_scans_all_history`: shown `wk(9000, 1)`, anomaly in 2026, expect `Some(.. weeks: 1 ..)`. The scan loops only over punch dates, so this stays fast.
- Unit `current_week_anomaly_is_counted_for_a_future_shown_week`: today's date with two open starts, shown the next week, expect `Some`, matching what the current week's own row shows.

### AC7. A week id before all data prints no marker (§3.3)

- Unit `shown_before_all_data_gives_none`: data in 2026-03, shown `wk(2026, 1)`, expect `None`.
- CLI `week_before_all_data_has_no_marker`: E7 in 2026-02. `mlm week 2025-50` prints no marker.
- Unit `shown_is_earliest_week_gives_none`: anomaly in the earliest week, shown equals it, expect `None`.

### AC8. `status` and `mlm week` print the same line for the same shown week; absent when the shown week is the earliest (§3.3, §7 "agree")

- CLI `status_and_week_print_the_same_marker_line`: seed E7 in 2026-02 and stale in 2026-03. For the same week (`mlm week 2026-10` against `mlm status 2026-03-02`, both inside week 2026-10) the two extracted marker lines are equal strings. Also assert the current-week pair `mlm status` and `mlm week`.
- CLI `marker_absent_when_shown_week_is_earliest`: covered by `shown_week_own_anomaly_not_counted` and `mlm status 2026-01-06`, which has no marker line.
- Layout assertions (T1 pinned these, I only verify they hold end to end). In `mlm week 2026-10` for the E7 fixture, with lines `L`, marker line at index `m`:
  - `L[m+1] == ""` and `L[m+2]` starts with `Carry-in:` and ends with `  [!]`;
  - with a marked row in the shown week, `L[m-1]` equals `[!] needs a look: run mlm status DATE for details and fixes`;
  - with no marked row in the shown week, `L[m-1] == ""`.
- CLI `status_marker_follows_date_block`: for `mlm status` today with an anomaly today (two starts today via `mlm start`) plus an old anomaly, the marker line is after the date's `fix:` lines.

### AC9. The scan makes no storage reads of its own (F6, F8)

- Structural: `carry_marker` and its helpers take `&History` only, no `Connection`. A review grep confirms `src/carry_scan.rs` has no `rusqlite` or `storage` import: `grep -n "rusqlite\|storage::" src/carry_scan.rs` is empty outside the test module's `punch_from_local`.
- Unit tests build `History` by hand (no database), which only compiles because the scan needs none.

### AC10. Output with no earlier anomaly is byte-identical to before (changeset global constraint)

- CLI `no_earlier_anomaly_output_unchanged_week` and `_status`: with the clean fixture, `mlm week 2026-02` and `mlm week 2026-04` stdout equal T1's golden literals for the same database (T1 `tests/week_read_errors.rs`). I copy only the 2026-02 golden (8h + 3h30m) and assert equality, to avoid a second copy to maintain. If the golden fixtures drift in T1, adjust to match T1's merged file.

### AC11. Verification, complexity, coverage

- `cargo fmt`, `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings`, and the two smoke runs from AGENTS.md.
- Complexity gate command as the hook runs it, no hits.
- Coverage: every branch has a unit test.
  - `earliest_date: None` return (`empty_history_gives_none`).
  - `pred_opt()?` stays inside one line, covered whenever the function runs.
  - `weeks.contains` skip (`counts_distinct_weeks_not_dates`).
  - `weeks.first()?` empty set (`clean_history_gives_none`).
  - The anomalous insert path (every positive test).
- Revert any hook rewrite of `coverage-baseline.json`.

## Risks, ambiguities and disagreements

1. **Refused-join cannot be isolated at the CLI.** `is_refused_join` needs an open start on the earlier date, and a past open start is always stale (age 2 or more) or E7 on its own, so that date flags anyway. The earlier date's week raises the marker for a reason other than the refused join. The later date (the orphan side) carries E8 too. My CLI and unit refused-join tests therefore prove that the fixture raises the marker, not that the refused-join term alone does. A unit test could isolate it by calling `date_facts` with `today` equal to the earlier date plus one day (age 1, not stale). I add that: `refused_join_with_recent_open_start_flags_later_date`, with `today = d(2026,1,28)`, start on 01-27, two orphan ends on 01-28 (today itself, so the orphan side flags via E8 as well). I accept that the refused-join term is not isolated; this is a limit of the predicate, not of this task.
2. **Shown week is its own exclusion.** The scan stops at `shown.start() - 1`, which means the day before the shown week is the last date scanned and its next bucket is the shown week's Monday. A refused join across that boundary counts for the earlier week (its Sunday row would show `[!]` in `mlm week` of that week). I treat that as correct and consistent with the row. The later (Monday) side belongs to the shown week and is not counted.
3. **Marker count depends on `date_facts` including overlap.** Until T2 merges, an overlap-only week gives no marker, and `mlm week` shows no `[!]` row for it, so the two still agree. No fixture before T2 depends on overlap.
4. **Timing at scale.** The scan classifies up to every punch date of history on every `status` and `mlm week`. The per-week short-circuit saves work for anomalous weeks only. For a clean multi-year history the cost is full. Before merge, run `cargo run --release` against a seeded database (`cargo run --example seed_test_data -- --db /tmp/mlm-seed.db --weeks 150 --seed 42`) and record the `status` wall time in the completion report. If it is visible (more than about 100 ms in release), report it so the coordinator can decide. Do not add caching in this task.
5. **`BTreeSet::first` toolchain.** Needs Rust 1.66. If the repo's MSRV is lower, use `weeks.iter().next()`. Check at step 3.
6. **Today in a future shown week.** For a future shown week, today's own date is inside the scanned range. Its open stint, E7 and so on count, as the current week's row would show them. `now_utc` is passed through so T2's `HH:MM-now` handling sees the real clock. This is consistent but means a today-open stint inside an overlap with a closed stint raises the marker on a future week view. Accepted.
7. **CLI fixtures depend on the real clock only for staleness.** Fixtures are dated January 2026, and the tests run on any date from February 2026 on. The `mlm week 2099-01` case is safe as long as no punch exists today in the test database.
8. **Test-file helper duplication.** I copy the `mlm_tz`, `seed_punch`, `stdout`, `stderr` helpers, as every integration test file in the repo does. A shared helper module would touch `tests/` files I do not own.
9. **No disagreement with the spec or the changeset plan.** One note on the changeset plan: the T4 acceptance bullet "A week id before all data prints no marker" holds by the empty-range path above, and needs no special case in code.
