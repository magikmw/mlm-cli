# Task plan: possible-issues-triage, T1 Foundation, read errors and one-line polish

Changeset plan: `docs/dev/plans/possible-issues-triage-plan.md` (T1 section, contracts 1-9, decisions F6 and F8). Spec: `docs/dev/specs/2026-10-03-possible-issues-triage.md`.

## What I read

- `docs/dev/plans/possible-issues-triage-plan.md`, full, including the coordinator's contract 6 update (marker line placement).
- `docs/dev/specs/2026-10-03-possible-issues-triage.md`, full.
- Sibling task plans for this changeset. None exist yet, so every seam below comes from the changeset plan's contracts.
- `src/status.rs` lines 1-700 (production code) plus the test regions at 1690-1730, 2470-2560, 2615-2760, 3260-3370 and grep passes over the rest.
- `src/week_view.rs` lines 1-375 plus the test regions around 1015-1075 and 1250-1320.
- `src/render.rs` lines 1-380 plus the test index (733-1082).
- `src/stint.rs` 1-135 and 260-404, `src/week.rs` 1-260, `src/week_target.rs` 1-70, `src/storage.rs` (error enum, punch read functions, `earliest_data_date`), `src/db.rs` schema and connect path, `src/date.rs` (error type, `resolve_date`, `resolve_future_checked_date`, `WeekId` API), `src/commands.rs` 1-110 and 289-297, `src/main.rs` 1-80, `src/lib.rs`.
- `tests/status_cli.rs` (helpers), `tests/e6_db_failure.rs` (head), `tests/ascii_sweep.rs` (grep).
- `clippy.toml` (cognitive complexity threshold 15), `.githooks/pre-commit` (fmt check, complexity gate, coverage ratchet against `coverage-baseline.json`, currently 99.218%).
- Scratch build at `/tmp/cd-scratch/task1`, used to capture golden outputs from the current binary (listed in the test plan).

## Scope

Files this task owns:

| File | Action |
|---|---|
| `src/lib.rs` | add four `pub mod` lines |
| `src/anomalies.rs` | new, owned until T2 takes it |
| `src/lookback.rs` | new, owned until T3 takes it |
| `src/history.rs` | new, T1 only |
| `src/carry_scan.rs` | new stub, T4 takes it over |
| `src/render.rs` | shed anomaly and lookback types, add clean-pair helper, phrases, marker type and line (frozen at merge) |
| `src/status.rs` | thin wiring, `done for today`, future-date rejection |
| `src/week_view.rs` | thin wiring, marker rendering, drop the swallowing adapter |
| `src/storage.rs` | one new function, `week_targets_in_range` |
| `src/week.rs` | one doc-comment touch only |
| `src/date.rs` | comments only (two comments that say `status` accepts future dates) |
| `src/stint.rs` | no edits needed (its `pub(crate)` predicates already suffice) |
| `tests/status_cli.rs`, `tests/e6_db_failure.rs`, `tests/ascii_sweep.rs` | may gain cases, existing assertions untouched |
| `tests/week_read_errors.rs` | new |
| `src/commands.rs` | no edit. Re-exports keep `status::lookback`, `render::LookbackFinding` and `render::LookbackKind` valid. T3 repoints them. |

Files I must not touch, and who owns them:

| File | Owner |
|---|---|
| `src/cli.rs`, `src/main.rs`, `src/week_target.rs`, `tests/week_target_cli.rs` | T5 |
| `docs/dev/SPEC.md`, `README.md`, `AGENTS.md` | T6 |
| `docs/dev/NOTES.md`, the spec file, `coverage-baseline.json` | coordinator |
| `src/anomalies.rs` after merge | T2 |
| `src/lookback.rs`, `src/commands.rs` after merge | T3 |
| `src/carry_scan.rs` after merge | T4 |

## Dependencies

| Dependency | State | Stub or import |
|---|---|---|
| Any sibling task | none needed | T1 is the root of the graph. |
| T5 (`main.rs` error path) | independent | Read errors reuse the existing `error: <text>` path in `src/main.rs`, which T5 does not change in behavior. No stub needed. |

## Published seams

These are frozen at T1 merge. T2, T3 and T4 build against them. All paths are `mlm::...`.

### 1. `src/anomalies.rs` (T2 extends it)

```rust
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use crate::stint::DayStints;
use crate::storage::Punch;

/// Raw punch buckets for one date and its two calendar neighbours.
/// Each slice holds punches of exactly one date (or is empty).
#[derive(Debug, Clone, Copy)]
pub struct DateBuckets<'a> {
    pub prev: &'a [Punch],
    pub day: &'a [Punch],
    pub next: &'a [Punch],
}

/// Every classification one date needs: the splice-aware view plus the
/// three raw single-date views the refused-join predicate reads.
pub struct DateContext {
    pub day: DayStints,
    pub prev_raw: DayStints,
    pub day_raw: DayStints,
    pub next_raw: DayStints,
}

/// Moved from render.rs, same fields as today.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaleOpenStart { pub time: NaiveTime, pub age_days: i64 }

/// Moved from render.rs, same fields and order as today. T2 adds fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateAnomalies {
    pub date: NaiveDate,
    pub open_stint_count: usize,
    pub latest_open_start: Option<NaiveTime>,
    pub orphaned_end_times: Vec<NaiveTime>,
    pub stale_open_start: Option<StaleOpenStart>,
    pub refused_join_with_previous: Option<NaiveDate>,
    pub refused_join_with_next: Option<NaiveDate>,
}

impl DateAnomalies {
    /// Default constructor: a date with no anomaly. Tests and callers
    /// build fixtures as `DateAnomalies { open_stint_count: 2, ..DateAnomalies::none(d) }`
    /// so a field T2 adds never breaks them.
    pub fn none(date: NaiveDate) -> Self;
    pub fn has_any(&self) -> bool;                 // unchanged semantics
    pub fn detail_lines(&self) -> Vec<String>;     // order E7, stale, E8, (T2: overlap), refused prev, refused next
    pub fn row_marker(&self) -> &'static str;      // "  [!]" or ""
}

pub fn classify_context(b: &DateBuckets<'_>, now_utc: DateTime<Utc>) -> DateContext;

/// The one fact builder. `now_utc` is unused until T2 (open stint shown as `HH:MM-now`).
pub fn build_facts(
    ctx: &DateContext,
    b: &DateBuckets<'_>,
    date: NaiveDate,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
) -> DateAnomalies;

/// `classify_context` then `build_facts`. T4 calls this per date.
pub fn date_facts(
    b: &DateBuckets<'_>,
    date: NaiveDate,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
) -> DateAnomalies;
```

Rule enforced by T1: only `src/anomalies.rs` constructs `DateAnomalies` with a struct literal that lists fields. `render.rs`, `status.rs`, `week_view.rs` and `carry_scan.rs` call `build_facts` or `date_facts`.

`render.rs` keeps `pub use crate::anomalies::{DateAnomalies, StaleOpenStart};` so old paths compile.

### 2. `src/lookback.rs` (T3 extends it)

```rust
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use rusqlite::Connection;

/// Moved from render.rs, unchanged shape and line text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LookbackFinding { pub date: NaiveDate, pub age_days: i64, pub kind: LookbackKind }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookbackKind {
    SingleOpenStart { time: NaiveTime, punch_id: i64 },
    MultipleOpenStints { count: usize },
}

impl LookbackFinding {
    pub fn detail_lines(&self) -> Vec<String>;   // unchanged
    pub fn stderr_line(&self) -> String;         // unchanged
}

/// Entry point 1, for `mlm start` (moved from `status::lookback`, same body).
/// One ranged read, ages 2-14, oldest first.
pub fn lookback(
    conn: &Connection,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
) -> anyhow::Result<Vec<LookbackFinding>>;

/// Entry point 2, for today's page. Returns finished lines: lookback lines
/// oldest first, then (T3) the hint. `refused_join_names_yesterday` is true
/// when today's own anomaly block already holds a refused-join line naming
/// yesterday. T1 accepts and ignores it, and returns exactly
/// `lookback(..)?.iter().flat_map(detail_lines)`.
pub fn today_page_lines(
    conn: &Connection,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
    refused_join_names_yesterday: bool,
) -> anyhow::Result<Vec<String>>;
```

Re-exports kept: `render.rs` has `pub use crate::lookback::{LookbackFinding, LookbackKind};`. `status.rs` has `pub use crate::lookback::{LookbackFinding, LookbackKind, lookback};`.

### 3. `src/history.rs` (T1 only)

```rust
use std::collections::BTreeMap;
use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::Connection;
use crate::anomalies::DateBuckets;
use crate::date::WeekId;
use crate::storage::Punch;
use crate::week::{WeekAccounting, WeekLedger};

/// Punches bucketed by local date. Only dates that have punches are keys.
#[derive(Debug, Clone, Default)]
pub struct PunchBuckets { /* BTreeMap<NaiveDate, Vec<Punch>> */ }

impl PunchBuckets {
    pub fn from_punches(punches: &[Punch]) -> Self;
    /// The date's punches, empty slice when none.
    pub fn day(&self, date: NaiveDate) -> &[Punch];
    /// previous, own and next day buckets for `date`.
    pub fn trio(&self, date: NaiveDate) -> DateBuckets<'_>;
    /// Dates that have punches within `[from, to]`, ascending.
    /// Empty (never a panic) when `from > to`.
    pub fn dates_in(&self, from: NaiveDate, to: NaiveDate) -> impl Iterator<Item = NaiveDate> + '_;
}

#[derive(Debug, Clone)]
pub struct History {
    /// Worked per week (live added to the current week only), data weeks, target overrides.
    pub ledger: WeekLedger,
    /// Every punch from one day before the walk start through one day after the shown week's end.
    pub buckets: PunchBuckets,
    /// Earliest date with a punch or a note.
    pub earliest_date: Option<NaiveDate>,
    /// Today's running-stint minutes. 0 unless today's week is the shown week or earlier.
    pub live_minutes: i64,
}

impl History {
    pub fn accounting(&self, week: WeekId) -> WeekAccounting;   // week::week_accounting(week, &self.ledger, &self.ledger)
}

/// Walk start is min(earliest data week, shown). One earliest-date read, one
/// ranged punch read, one ranged target read. Any failure is an error.
pub fn load_history(
    conn: &Connection,
    shown: WeekId,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
) -> anyhow::Result<History>;
```

Deviation from contract 5: today's live minutes are not an input. The loader derives them from its own buckets, which always cover today whenever live matters. This removes status's three extra reads and makes status and `mlm week` ledgers equal by construction.

`History` has only public fields, so T4 can hand-build one in unit tests with `PunchBuckets::from_punches`.

### 4. Carry marker (`render.rs` for data and text, `carry_scan.rs` for the stub)

```rust
// src/render.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CarryMarker {
    pub weeks: usize,    // number of earlier weeks with an anomaly, >= 1
    pub oldest: WeekId,  // oldest such week
}

/// weeks >= 2: "[!] carry-in includes anomalies from {n} earlier weeks, oldest {id} (see mlm week {id})"
/// weeks == 1: "[!] carry-in includes anomalies from 1 earlier week, {id} (see mlm week {id})"
pub fn carry_marker_line(m: &CarryMarker) -> String;
```

```rust
// src/carry_scan.rs (T1 stub, T4 fills in; signature frozen)
use chrono::{DateTime, NaiveDate, Utc};
use crate::date::WeekId;
use crate::history::History;
use crate::render::CarryMarker;

/// Scan dates with punches from `history.earliest_date` through the day before
/// `shown.start()`, using `anomalies::date_facts(..).has_any()` on
/// `history.buckets.trio(date)`. No storage reads. T1 returns None.
pub fn carry_marker(
    history: &History,
    shown: WeekId,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
) -> Option<CarryMarker>;
```

Renderings T1 builds (T4 changes no output code):

- `status`: the marker line is the last entry of `StatusView.anomaly_lines`, after the date block and today's lookback lines.
- `mlm week`: `WeekView` gains `pub carry_marker: Option<CarryMarker>`. See step 5 for the exact layout (coordinator's contract 6).

### 5. Clean-pair helper and phrases (`src/render.rs`, frozen)

```rust
/// Local wall-clock time of a UTC instant (moved here from status.rs, now shared).
pub fn local_time(at_utc: DateTime<Utc>) -> NaiveTime;

/// N: the first punch of any kind on `start.date` that is strictly later
/// than `start`, by (at_utc, kind, id). Punches at S's own instant, and
/// punches on other dates, are ignored. `punches` may span several dates.
pub fn next_punch_after(punches: &[Punch], start: &Punch) -> Option<NaiveTime>;

/// "(a time after 18:02)"  or  "(a time between 18:02 and 19:00)"
pub fn stop_time_hint(start: NaiveTime, next: Option<NaiveTime>) -> String;

/// "(a time after 14:00 closes the 14:00 start)"
/// or "(a time between 14:00 and 15:00 closes the 14:00 start)"
pub fn stop_time_hint_closes(start: NaiveTime, next: Option<NaiveTime>) -> String;
```

Both phrase functions return the surrounding parentheses. Both format times as `%H:%M`. T1 does not apply them to any existing line.

## Facts gathered (current code)

- `status.rs:312-370`: `DateContext`, `classify_date_context`, `anomaly_facts_for_date`. `latest_open_start` uses `.iter().map(local_time).max()`. `week_view.rs:186-236` (`row_anomalies`) uses `.last()`. Both builders otherwise agree.
- `status.rs:372-450`: `group_punches_by_date`, `lookback_finding_for_date`, `lookback` (ages 2-14, one read of `[today-15, today-1]`).
- `status.rs:484-575`: `credit_live`, `build_ledger`. `build_ledger` does about six queries per calendar day from the earliest week through `through` (`punches_for_date` x3, `notes_for_date`).
- `status.rs:612-683`: `resolve`. Line 620 calls `date::resolve_date`, which allows future dates.
- `status.rs:206`: literal `, target already met`. Test literals at about lines 860, 1123, 1140, 1226, 1244, 1916, 2851, 2871, 3023 and others (`grep -n "target already met" src/status.rs`).
- `status.rs:~1706` test `resolve_future_absolute_date_still_succeeds_and_renders_empty`. `status.rs:~3288` test `pit_future_week_carry_in_includes_live` (resolves `2026-02-16` with `now_thu_1800()`, expects `"Week 2026-08:  Total behind: 50h 30m"`).
- `status.rs:2690-2745`: the only two direct `lookback(..)` tests (`lookback_returns_findings_oldest_first`, `lookback_excludes_age_one_open_start`).
- `week_view.rs:133-177` `build_rows(dates, punches: &[Punch], today, now_utc)` is called by about 15 tests with a punch slice. `week_view.rs:264-319` `DbWeekData` swallows errors (`.ok()`, `unwrap_or_default()`). Tests using `DbWeekData` directly are `b8` (row sum equals `acct.worked`, about line 1015-1040), `b8b_other_week_worked_gets_no_live` and `c14_worked_minutes_padding_day_isolation`.
- `week_view.rs:100-106`: footer printed only when a row is marked, followed by a blank line. Then the four-line totals loop (`LABEL_WIDTH` 15).
- `week_view.rs` tests build `WeekView { .. }` literals at about lines 417, 442, 545, 566. They need `carry_marker: None` added.
- `render.rs:733-1082`: all `DateAnomalies`/`LookbackFinding`/ascii tests. `t33b` (line 1022) builds both types.
- `storage.rs`: `punches_in_range` (396), `earliest_data_date` (501, reads punches and notes), `StorageError::{Db, CorruptRow}`. `punches_in_range` parses `at_utc` per row, `earliest_data_date` parses only the minimum `date`. A raw punch row with a valid date and `at_utc = 'garbage'` therefore breaks only the ranged read.
- `date.rs:123-125` and `:158-160` comments say `status` accepts future dates. `resolve_future_checked_date` (161) yields `invalid DATE "<s>": date is in the future`.
- `main.rs` prints `error: {e}` and exits 1 for every error.
- Golden check on the current binary (TZ=UTC): `mlm week 9000-01` takes 59 s today. `status 2026-12-01` exits 0 today, and after this task exits 1 with `error: invalid DATE "2026-12-01": date is in the future`.
- `chrono` 0.4.45, `rusqlite` 0.40.2. `WeekId` is `Copy + Ord + Display`, has `from_date`, `start`, `end`, `to_key`, `from_key`, `next`. Keys are `YYYY-WW` with a 4-digit year, so string order equals chronological order.

## Steps

TDD throughout: write each step's tests first, watch them fail for the right reason, then implement. Commit per step. Run `cargo fmt` before each commit. If the hook rewrites `coverage-baseline.json`, revert that file before the final commit.

### Step 0. Scaffolding and goldens

1. Create `src/anomalies.rs`, `src/lookback.rs`, `src/history.rs`, `src/carry_scan.rs` containing only a `//!` module doc. Add `pub mod anomalies; pub mod carry_scan; pub mod history; pub mod lookback;` to `src/lib.rs` in alphabetical position. This is the only edit to `lib.rs`.
2. Create `tests/week_read_errors.rs` with the golden success tests (see test plan AC3). Run them against the unchanged code. They must pass before any refactor.

### Step 1. `render.rs` additions

Add `local_time`, `next_punch_after`, `stop_time_hint`, `stop_time_hint_closes`, `CarryMarker`, `carry_marker_line`.

```rust
pub fn local_time(at_utc: DateTime<Utc>) -> NaiveTime {
    at_utc.with_timezone(&Local).time()
}

pub fn next_punch_after(punches: &[Punch], start: &Punch) -> Option<NaiveTime> {
    punches
        .iter()
        .filter(|p| p.date == start.date && p.at_utc > start.at_utc)
        .min_by_key(|p| (p.at_utc, p.kind, p.id))
        .map(|p| local_time(p.at_utc))
}

fn stop_window(start: NaiveTime, next: Option<NaiveTime>) -> String {
    let s = start.format("%H:%M");
    match next {
        None => format!("a time after {s}"),
        Some(n) => format!("a time between {s} and {}", n.format("%H:%M")),
    }
}

pub fn stop_time_hint(start: NaiveTime, next: Option<NaiveTime>) -> String {
    format!("({})", stop_window(start, next))
}

pub fn stop_time_hint_closes(start: NaiveTime, next: Option<NaiveTime>) -> String {
    format!("({} closes the {} start)", stop_window(start, next), start.format("%H:%M"))
}

pub fn carry_marker_line(m: &CarryMarker) -> String {
    debug_assert!(m.weeks >= 1);
    if m.weeks == 1 {
        format!("[!] carry-in includes anomalies from 1 earlier week, {0} (see mlm week {0})", m.oldest)
    } else {
        format!(
            "[!] carry-in includes anomalies from {} earlier weeks, oldest {1} (see mlm week {1})",
            m.weeks, m.oldest
        )
    }
}
```

Fix the format-argument numbering when typing it (the plural form mixes implicit and indexed args, so use `{0}`/`{1}` consistently or named args). Imports to add: `chrono::{DateTime, Local, Utc}`, `crate::storage::Punch`.

Tests (in `render.rs` `mod tests`, build punches with `storage::punch_from_local(id, kind, date, time, &Local)`):

- `next_punch_after`: no later punch gives None. Later start. Later end. Later punch at S's minute gives None. S at 23:59 gives None. Punches on another date (before and after) ignored. Two later punches return the earlier one.
- Phrases, plain and closes forms, both `None` and `Some`. Include the spec example `(a time between 14:00 and 15:00 closes the 14:00 start)` and `(a time between 18:02 and 18:03)` (N one minute after S).
- `carry_marker_line` for weeks 2 and 1, byte for byte as in contract 6.

### Step 2. `anomalies.rs`

Move from `render.rs`: `DateAnomalies`, `StaleOpenStart`, their `impl` (including `e7_lines`, `stale_lines`, `e8_lines`, `refused_join_lines`, `row_marker`) and `refused_join_line`. Line text stays byte for byte. In `render.rs` add `pub use crate::anomalies::{DateAnomalies, StaleOpenStart};`.

Move tests `t_e7_reworded_matches_spec_example` through `t_ordering_all_kinds_at_once` (render.rs 748-890) to `anomalies.rs`. Replace helper `da(date)` by `DateAnomalies::none(date)`. `t33b` stays in `render.rs`, importing both types through the re-exports, with `..DateAnomalies::none(..)`.

New code:

```rust
pub fn classify_context(b: &DateBuckets<'_>, now_utc: DateTime<Utc>) -> DateContext {
    DateContext {
        day: stint::classify_at(b.prev, b.day, b.next, now_utc),
        prev_raw: stint::classify(b.prev, now_utc),
        day_raw: stint::classify(b.day, now_utc),
        next_raw: stint::classify(b.next, now_utc),
    }
}

pub fn build_facts(ctx: &DateContext, b: &DateBuckets<'_>, date: NaiveDate,
                   today: NaiveDate, _now_utc: DateTime<Utc>) -> DateAnomalies {
    DateAnomalies {
        date,
        open_stint_count: ctx.day.open.len(),
        latest_open_start: ctx.day.open.last().map(|o| render::local_time(o.start.at_utc)),
        orphaned_end_times: orphan_times(&ctx.day),
        stale_open_start: stale_fact(&ctx.day, date, today),
        refused_join_with_previous: refused_neighbor(&ctx.prev_raw, &ctx.day_raw, b.day, date.pred_opt()),
        refused_join_with_next: refused_neighbor(&ctx.day_raw, &ctx.next_raw, b.next, date.succ_opt()),
    }
}
```

- `orphan_times(day) -> Vec<NaiveTime>`: map `day.orphaned_ends` through `local_time`.
- `stale_fact(day, date, today) -> Option<StaleOpenStart>`: wraps `stint::stale_open_start`.
- `refused_neighbor(earlier, later, later_punches, neighbor: Option<NaiveDate>) -> Option<NaiveDate>`: `stint::is_refused_join(..)` then neighbor. This replaces `week_view::refused_join_neighbor`.
- `DateAnomalies::none(date)` sets every field to its empty value.
- `date_facts` is `classify_context` plus `build_facts`.

Each function is small, so none nears 15. T2 grows `build_facts`, so it already delegates field by field.

New tests:

- `build_facts` over hand-built buckets reproduces each fixture: E7, stale (age 2 and 15), E8, refused join in both directions, clean day, splice (no anomaly).
- `has_any() == !detail_lines().is_empty()` over all those fixtures (the row marker and the block cannot disagree).
- `latest_open_start` is the instant-latest open start.

### Step 3. `lookback.rs`

Move `LookbackFinding`, `LookbackKind` and both impl methods from `render.rs`. Move `group_punches_by_date`, `lookback_finding_for_date` and `lookback` from `status.rs`. They use `render::local_time`. Add:

```rust
pub fn today_page_lines(conn: &Connection, today: NaiveDate, now_utc: DateTime<Utc>,
                        _refused_join_names_yesterday: bool) -> anyhow::Result<Vec<String>> {
    Ok(lookback(conn, today, now_utc)?
        .iter()
        .flat_map(LookbackFinding::detail_lines)
        .collect())
}
```

Re-exports in `render.rs` and `status.rs` as listed under Published seams. Do not touch `commands.rs`.

Tests moved:

- render.rs `t_lookback_*` tests (single block, multi block, ordering oldest first, stderr forms) go to `lookback.rs`.
- status.rs `lookback_returns_findings_oldest_first` and `lookback_excludes_age_one_open_start` go to `lookback.rs` with local copies of the small helpers (`test_db`, `d`, `t`, a fixed `today()` of 2026-02-12 and `now`).
- New: `today_page_lines` equals the flattened `detail_lines` of `lookback` and is identical for flag `true` and `false` (AC10).

### Step 4. `storage.rs`, `history.rs`, `carry_scan.rs`

`storage.rs` (add `use crate::date::WeekId;`):

```rust
/// Overrides for weeks in `[from, to]`, ascending. Keys sort chronologically
/// as text (`YYYY-WW`). A stored key that is not a valid week id is a CorruptRow.
pub fn week_targets_in_range(conn: &Connection, from: WeekId, to: WeekId)
    -> Result<Vec<(WeekId, i64)>, StorageError> {
    let mut stmt = conn.prepare(
        "SELECT week_id, target_minutes FROM week_targets \
         WHERE week_id BETWEEN ?1 AND ?2 ORDER BY week_id")?;
    let rows = stmt.query_map((from.to_key(), to.to_key()), |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
    let mut out = Vec::new();
    for row in rows {
        let (key, minutes) = row?;
        let week = WeekId::from_key(&key)
            .map_err(|_| StorageError::CorruptRow { column: "week_id", value: key.clone() })?;
        out.push((week, minutes));
    }
    Ok(out)
}
```

`history.rs`:

```rust
pub fn load_history(conn, shown, today, now_utc) -> anyhow::Result<History> {
    let earliest_date = storage::earliest_data_date(conn)?;
    let start_week = walk_start(earliest_date, shown);
    let (from, to) = read_range(start_week, shown);
    let punches = storage::punches_in_range(conn, from, to)?;
    let targets = storage::week_targets_in_range(conn, start_week, shown)?;
    let buckets = PunchBuckets::from_punches(&punches);
    let live_minutes = live_for(&buckets, today, shown, now_utc);
    let mut worked = worked_by_week(&buckets, start_week.start(), shown.end(), now_utc);
    credit_live(&mut worked, today, shown, live_minutes);
    Ok(History { ledger: build_ledger(earliest_date, worked, targets), buckets, earliest_date, live_minutes })
}
```

- `walk_start(earliest, shown) -> WeekId`: `earliest.map(WeekId::from_date).map_or(shown, |e| e.min(shown))`.
- `read_range(start_week, shown) -> (NaiveDate, NaiveDate)`: `(start_week.start().pred_opt().unwrap_or(start), shown.end().succ_opt().unwrap_or(end))`. Pad one day each side.
- `worked_by_week(buckets, from, to, now_utc) -> BTreeMap<WeekId, i64>`: for each `date` in `buckets.dates_in(from, to)`, `classify_at(trio)` and add `completed_minutes()` to `WeekId::from_date(date)`. Dates outside `[from, to]` (the padding days) never count.
- `live_for(buckets, today, shown, now_utc) -> i64`: `0` when `WeekId::from_date(today) > shown`, else `classify_at(buckets.trio(today)...).live_minutes()`.
- `credit_live`: moved from `status.rs` unchanged (`live > 0 && current <= through`).
- `build_ledger(earliest, worked, targets) -> WeekLedger`: `with_data_week(week of earliest)` if any, then `with_target` for each override, then `with_worked` for each week. This keeps the two behaviours the plan names (earliest data week, notes included, anchors the walk; live credit marks the current week as data).

`carry_scan.rs`: the stub from Published seams, body `let _ = (history, shown, today, now_utc); None`.

Tests:

- `storage::week_targets_in_range`: in-range rows only, a legal `0` row kept, ordering, empty range, dropped table gives an `Err`.
- `PunchBuckets`: `day`, `trio` at both ends, `dates_in` ascending, `dates_in` with `from > to` is empty.
- `load_history`: empty database gives an empty ledger and `earliest_date None`. Two weeks of data gives expected worked per week. Padding-day isolation (the `c14` fixture rewritten, see step 5). Live credited only when the current week is at or before `shown`. Far-future `shown` (`wk(9000, 1)`) returns with `buckets` holding only the seeded dates.
- Failure injection, all through `load_history`. Dropped `notes` gives `Err`. Dropped `week_targets` gives `Err`. Only the ranged punch read failing gives `Err`: insert a raw row `INSERT INTO punches (at_utc, "date", kind) VALUES ('garbage', '2026-01-05', 'start')`. `earliest_data_date` still passes, `punches_in_range` fails.

### Step 5. `week_view.rs` onto the shared modules

1. Delete `DbWeekData`, `row_anomalies` and `refused_join_neighbor`.
2. Add `build_rows_from` and keep a test-only wrapper so the 15 existing tests keep their call shape:

```rust
fn build_rows_from(dates: [NaiveDate; 7], buckets: &PunchBuckets, today: NaiveDate,
                   now_utc: DateTime<Utc>) -> [WeekRow; 7] {
    std::array::from_fn(|i| {
        let date = dates[i];
        let trio = buckets.trio(date);
        let ctx = anomalies::classify_context(&trio, now_utc);
        let facts = anomalies::build_facts(&ctx, &trio, date, today, now_utc);
        WeekRow {
            weekday_abbrev: WEEKDAY_ABBREVS[i],
            date_display: format_date(date),
            minutes: ctx.day.completed_minutes() + if date == today { ctx.day.live_minutes() } else { 0 },
            is_ongoing: ctx.day.is_ongoing() && date == today,
            has_anomaly: facts.has_any(),
        }
    })
}

#[cfg(test)]
fn build_rows(dates: [NaiveDate; 7], punches: &[Punch], today: NaiveDate,
              now_utc: DateTime<Utc>) -> [WeekRow; 7] {
    build_rows_from(dates, &PunchBuckets::from_punches(punches), today, now_utc)
}
```

3. `build_week_view`:

```rust
let history = history::load_history(conn, week, today, now_utc)?;
let rows = build_rows_from(dates, &history.buckets, today, now_utc);
let acct = history.accounting(week);
let carry_marker = carry_scan::carry_marker(&history, week, today, now_utc);
```

`headline` and the other `WeekView` fields stay as today. Add `pub carry_marker: Option<CarryMarker>` to `WeekView`.

4. Split `render_week` into helpers so complexity stays low. Output with `carry_marker == None` is byte-identical to today. Layout with a marker, per the coordinator's contract 6:

```
  Sun 2026-10-11   00h 00m
<blank>
[!] needs a look: run mlm status DATE for details and fixes     <- only if a row is marked
[!] carry-in includes anomalies from 2 earlier weeks, oldest 2026-38 (see mlm week 2026-38)   <- directly under, no blank between
<blank>
Carry-in:      -80h 00m  [!]
Worked:        ...
```

With no marked row, the marker line takes the footer's place (row table, blank, marker line, blank, Carry-in block).

```rust
fn push_notices(out: &mut String, view: &WeekView) {
    if view.rows.iter().any(|r| r.has_anomaly) {
        out.push_str(render::WEEK_ANOMALY_FOOTER);
        out.push('\n');
    }
    if let Some(m) = &view.carry_marker {
        out.push_str(&render::carry_marker_line(m));
        out.push('\n');
    }
    if view.carry_marker.is_some() || view.rows.iter().any(|r| r.has_anomaly) {
        out.push('\n');
    }
}

fn push_totals(out: &mut String, view: &WeekView) {
    for (label, minutes) in [("Carry-in:", view.carry_in_minutes), ("Worked:", view.worked_minutes),
                             ("Fulfillment:", view.fulfillment_minutes), ("Target:", view.target_minutes)] {
        out.push_str(&format!("{:<w$}{}", label, format_minutes(minutes), w = LABEL_WIDTH));
        if label == "Carry-in:" && view.carry_marker.is_some() {
            out.push_str("  [!]");
        }
        out.push('\n');
    }
}
```

Compute `any marked` once into a local bool. Existing output with a footer but no marker: footer line, then one blank (unchanged).

5. Tests:

- Add `carry_marker: None` to the four `WeekView` literals.
- Rewrite the three `DbWeekData` tests onto `load_history`, same assertions:
  - `b8` row-sum test: `history::load_history(&conn, week, d(2026,2,15), now)` then `history.accounting(week).worked == row_sum`.
  - `b8b`: load with `shown = wk(2026, 8)`, assert `ledger.worked_minutes(wk(2026,7)) == 120` (needs `use crate::week::WeekData`), weeks 6 and 8 give 0.
  - `c14`: load with `shown = wk(2026, 8)`, assert the three weeks' ledger worked values 120, 480, 90 and their sum.
- New: hand-built `WeekView` with `carry_marker` renders the layouts above byte for byte for three cases (marker alone, marker plus footer, 1-week form), and renders unchanged output when `None`.
- `build_week_view` returns `Err` on a database with `week_targets` dropped.

### Step 6. `status.rs` onto the shared modules

Delete `DateContext`, `classify_date_context`, `anomaly_facts_for_date`, `group_punches_by_date`, `lookback_finding_for_date`, `lookback`, `credit_live`, `live_minutes_today`, `build_ledger`, and the private `local_time` (use `render::local_time`). Replace the `pub use` on line 43 as listed in Published seams.

New `resolve`:

```rust
pub fn resolve(date_arg: Option<&str>, now: DateTime<Local>, conn: &Connection) -> anyhow::Result<StatusView> {
    let today = now.date_naive();
    let target_date = match date_arg {
        None => today,
        Some(s) => date::resolve_future_checked_date(s, today)?,   // §4.5
    };
    let is_today = target_date == today;
    let open_stint_age = open_stint_age_for(target_date, today);
    let now_utc = now.with_timezone(&Utc);
    let target_week = WeekId::from_date(target_date);

    let notes = storage::notes_for_date(conn, target_date)?;
    let history = history::load_history(conn, target_week, today, now_utc)?;
    let buckets = history.buckets.trio(target_date);
    let ctx = anomalies::classify_context(&buckets, now_utc);

    let receiving_date_suffix = receiving_suffix(&ctx, buckets.day);
    let live = history.live_minutes;
    let day_total_minutes = ctx.day.completed_minutes() + if is_today { live } else { 0 };
    let acct = history.accounting(target_week);

    let facts = anomalies::build_facts(&ctx, &buckets, target_date, today, now_utc);
    let lookback_lines = if is_today {
        lookback::today_page_lines(conn, today, now_utc, facts.refused_join_with_previous.is_some())?
    } else { Vec::new() };
    let marker = carry_scan::carry_marker(&history, target_week, today, now_utc);
    let anomaly_lines = compose_anomaly_lines(&facts, lookback_lines, marker.as_ref());
    // eod, stints, notes, StatusView { .. } as today
}
```

Helpers (each small):

- `open_stint_age_for(target_date, today) -> OpenStintAge`: the existing three-way if-chain.
- `receiving_suffix(ctx, day_punches) -> Option<String>`: the existing `splice_candidate(ctx.prev_raw.open.len(), &ctx.day_raw, day_punches)` expression with the `"  ({} continues previous day's stint)"` format.
- `compose_anomaly_lines(facts: &DateAnomalies, lookback: Vec<String>, marker: Option<&CarryMarker>) -> Vec<String>`: `facts.detail_lines()`, then lookback lines, then `marker.map(render::carry_marker_line)`. This is the §3.1/§3.3 placement.

Polish in the same file:

- Line 206 becomes `", done for today"`. Update the doc comment on `TargetAlreadyMet` (line 94). The variant name stays.
- In tests, replace every `target already met` literal with `done for today`, including the negative pins (`!out.contains("done for today")`). Their intent (not-met states omit the phrase) is unchanged.
- `resolve_future_absolute_date_still_succeeds_and_renders_empty` becomes `resolve_future_date_is_rejected`: `resolve(Some("2026-03-01"), now_thu_1800(), &conn)` is `Err`, message `invalid DATE "2026-03-01": date is in the future`. Add a case for `2026-02-13` (tomorrow, rejected) and `2026-02-12` (today, accepted).
- `pit_future_week_carry_in_includes_live` keeps its fixture and checks `week_view::build_week_view(&conn, wk(2026, 8), now_thu_1800())`: `view.headline == "Total behind: 50h 30m"` and `view.carry_in_minutes == -630` (1770 fulfillment minus 2400 target).
- Fix the stale comment at about line 1740 (`build_ledger`) and the module header's `anomaly_facts_for_date` mention.

Add `compose_anomaly_lines` tests with hand-built markers (see AC8).

### Step 7. Comments

- `date.rs` lines 123-125 and 158-160: say `status` uses `resolve_future_checked_date` and rejects a future date, while `resolve_date` stays the core resolver.
- `week.rs` lines 84-86: the `WeekLedger` doc says it serves "the wave-3 adapter that loads one GROUP BY query's worth of rows". Reword to name `history::load_history` as the loader. No code change.

### Step 8. Integration tests

`tests/week_read_errors.rs` (new) and one case each in `tests/status_cli.rs` (see test plan). Run the full verification set from AGENTS.md.

## Test plan, by acceptance criterion

### AC1. Existing tests keep their expectations (§7, §4.4, §4.5)

- `cargo test` passes with no edit to any assertion except these:
  - `target already met` pins on the Day total line, rewritten to `done for today`.
  - The two future-date `status` tests described in step 6.
  - `WeekView` literals gain `carry_marker: None`.
  - The three `DbWeekData` tests move onto `load_history` with equal assertions.
  - Moved tests keep their assertions.
- If any other test fails with `date is in the future`, stop and report. The changeset plan says only two exist.

### AC2. Anomaly blocks, row markers, footer and lookback lines are byte-identical (§3.3 same predicate, §7)

- Moved `DateAnomalies` and `LookbackFinding` tests pass unchanged in their new homes.
- `anomalies::tests::build_facts_*` cover E7, stale, E8, refused join both ways, splice and clean days.
- `anomalies::tests::has_any_matches_nonempty_block` over the same fixtures.
- Existing `week_view` tests `a*`/`b*` row tests and `status` ordering tests pass untouched.
- `grep -n "DateAnomalies {" src` shows struct literals only in `src/anomalies.rs` (plus `..none()` update forms in tests).

### AC3. §3.4 read failures are hard errors, success output unchanged

`tests/week_read_errors.rs`, TZ pinned to `UTC`. Fixture seeded by raw insert (as in `tests/status_cli.rs::seed_punch`), after creating the database with `mlm::db::connect_at(&path)`:

```
2026-01-05 start 09:00Z, end 17:00Z
2026-01-06 start 09:00Z, end 12:30Z
```

Failure cases, each for `mlm week` and `mlm week 2026-41`:

- `DROP TABLE week_targets` before the run.
- `DROP TABLE notes` before the run.

Assert exit code `1`, empty stdout, stderr starts with `error:`.

The ranged punch read alone cannot fail from the CLI, because `earliest_data_date` reads `punches` too. It is covered by the loader unit test with a `'garbage'` `at_utc` row (step 4).

Golden success outputs, captured from the current binary and pasted as literals (all three must pass before step 1 and after step 8):

`mlm week 2026-02`:
```
Week 2026-02 (2026-01-05 - 2026-01-11)

Total behind: 28h 30m

  Mon 2026-01-05   08h 00m
  Tue 2026-01-06   03h 30m
  Wed 2026-01-07   00h 00m
  Thu 2026-01-08   00h 00m
  Fri 2026-01-09   00h 00m
  Sat 2026-01-10   00h 00m
  Sun 2026-01-11   00h 00m

Carry-in:      00h 00m
Worked:        11h 30m
Fulfillment:   11h 30m
Target:        40h 00m
```

`mlm week 2026-04` has `Total behind: 108h 30m`, all rows `00h 00m` (2026-01-19 to 2026-01-25), `Carry-in:      -68h 30m`, `Worked:        00h 00m`, `Fulfillment:   -68h 30m`, `Target:        40h 00m`.

`mlm week 2099-01` (2098-12-29 - 2099-01-04) has `Total behind: 152348h 30m`, all rows `00h 00m`, `Carry-in:      -152308h 30m`, `Worked:        00h 00m`, `Fulfillment:   -152308h 30m`, `Target:        40h 00m`. This is the future-week carry-in case. It holds as long as the database has no open stint today.

The live-inclusive current week is real-clock dependent, so it stays covered by the existing `week_view` unit tests (`pit_future_week_carry_in_includes_current_week_live`, the `b8` live tests).

### AC4. Far-future week completes quickly, one ranged read (§3.4, F6)

- `tests/week_read_errors.rs::far_future_week_is_fast`: seeded database, `mlm week 9000-01` exits 0, prints `Week 9000-01`, and takes under 10 seconds wall time (59 s before this change in a debug build).
- Loader unit test: `load_history(.., wk(9000, 1), ..)` over data in 2026 holds in `buckets` only the seeded dates, which shows the loader visits only dates that have punches.

### AC5. `status` and `mlm week` produce equal ledgers (§3.4)

Unit test in `status.rs` over a multi-week fixture with carry, an override and a note-only first week:

- For a closed week `W`: `resolve(date in W).week_line == format!("Week {W}:  {}", build_week_view(W).headline)`.
- For the current week: the week line parenthetical contains the view's fulfillment, worked (`fulfillment - carry`), carry-in and target figures formatted with `format_minutes`.
- `History::accounting(W)` from two separate `load_history` calls (one as status would, one as week would) are equal.

### AC6. §4.4 `done for today`

- Existing gap-at-or-below-zero tests, rewritten to the new literal, pin the trigger: open stint today with `gap <= 0`.
- A not-met fixture asserts `done for today` is absent and `est. EOD` wording unchanged.
- `resolve` pins at lines 1916, 2851, 2871, 3023 (`EodState::TargetAlreadyMet` through `resolve`) check `, done for today`.

### AC7. §4.5 future `status DATE`

- Unit: step 6 rejection tests.
- Integration (`tests/status_cli.rs`): `mlm status 2099-01-01` exits `1`, stdout empty, stderr `error: invalid DATE "2099-01-01": date is in the future`. `mlm status` and `mlm status -1` still succeed (existing tests). `mlm week 2099-01` exits 0 (covered in AC3).

### AC8. Carry marker renderings (§3.3)

- `render::tests::carry_marker_line_*`: the two literal lines from contract 6.
- `status::tests::compose_anomaly_lines_*`:
  - marker only gives `[marker line]`.
  - block plus lookback plus marker gives the three groups in that order (block, lookback, marker).
  - `None` gives exactly today's lines.
  - Rendered through `render(&view)` with the marker in `anomaly_lines`, the marker follows the week line and one blank line.
- `week_view::tests::render_week_*`: the three layouts in step 5 byte for byte, using `Carry-in:      -80h 00m  [!]`.
- `carry_scan::tests::stub_returns_none`.
- With the stub, every existing and golden output is unchanged.

### AC9. Clean-pair helper and phrases (§3.2.2)

Step 1 tests: no later punch, later start, later end, same-minute punch, S at 23:59, other dates ignored, plain and closes phrases for both `None` and `Some`, and the one-minute-window case.

### AC10. Lookback entry point (§3.1 ordering, T1 equals today)

`lookback::tests::today_page_lines_equals_flattened_findings` for flag `true` and `false`, with a fixture holding two findings (a single open start at age 10 and a multi open date at age 4). Output order oldest first.

### AC11. Verification, complexity, coverage

- `cargo fmt`, `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings`, and the two smoke runs from AGENTS.md.
- Complexity check as the hook runs it: `cargo clippy --all-targets --quiet --message-format=json -- -W clippy::cognitive_complexity` has no hits.
- Coverage: every new branch has a test above. `build_rows` stays `#[cfg(test)]` so it adds no uncovered production lines. Revert any hook rewrite of `coverage-baseline.json`.

## Risks, ambiguities and disagreements

1. **Marker placement in `mlm week`.** I follow the coordinator's updated contract 6 (footer slot, marker directly under the footer, then one blank, then the Carry-in block). The spec text "after the footer" alone is ambiguous, and the old plan wording ("footer, blank line, marker") differs. T4's integration tests must assert this exact layout.
2. **Loader input.** Contract 5 lists live minutes as an input. I derive them inside the loader (`History.live_minutes`). Reason: the data is already in the buckets whenever live matters, and one source keeps `status` and `week` equal. If a caller needs to inject live minutes, that is a signature change after merge, so flag it now.
3. **`latest_open_start`.** `status` used the maximum local clock time and `week` used the instant-latest start. I unify on instant-latest (`.last()`). They differ only across a DST fall-back hour on a date with two or more open starts. The instant-latest start is also what the clean-pair rule needs.
4. **`commands.rs`.** The changeset plan lets T1 edit its import lines. Re-exports make that unnecessary, so T1 leaves it alone and T3 repoints the paths.
5. **Variant name.** `EodState::TargetAlreadyMet` keeps its name to avoid churn in about 15 tests. A rename is cosmetic and can ride with T3 or later.
6. **Corrupt week keys.** `week_targets_in_range` reports a stored `week_id` that is not a valid `YYYY-WW` inside the queried key range as a hard error. Before, only the exact keys for walked weeks were read. Writes always normalize the key, so this only affects hand-edited databases.
7. **Memory and time.** The loader reads every punch from the earliest date to the shown week on each `status` and `mlm week`. This is the cost the changeset plan accepts. `mlm week 9000-01` still builds about 363,000 accounting rows in memory (about 20 MB) through `week_series`, which T1 does not change.
8. **Unused `now_utc` in `build_facts`.** It is accepted now (named `_now_utc`) so T2 does not change a frozen signature.
9. **Fixed-date test hazard.** The 2099-01 golden stays valid only while the test database has no punch today, which holds because the fixture is January 2026.
10. **No disagreement with the spec.** One note on the changeset plan: it says T1's `src/commands.rs` edits are "import lines only". The plan here makes it zero edits, which is a subset of that.
