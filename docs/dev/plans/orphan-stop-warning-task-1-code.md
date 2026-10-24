# Task plan: orphan-stop-warning, Task 1 (code)

Changeset plan: `docs/dev/plans/orphan-stop-warning-plan.md` (Task 1).
Spec: `docs/dev/specs/2026-10-24-orphan-stop-warning.md` (locked).
Baseline: branch `orphan-stop-warning` at `4edcd2e`. No source was edited to write this plan.

## 0. Read for this plan

- `docs/dev/plans/orphan-stop-warning-plan.md` (full)
- `docs/dev/specs/2026-10-24-orphan-stop-warning.md` (full)
- Sibling plan `docs/dev/plans/orphan-stop-warning-task-2-docs.md`: did not exist at planning time. Task 2 is docs only and exports no signatures this task consumes.
- `src/anomalies.rs` (full), `src/lookback.rs` (full), `src/render.rs` (full), `src/commands.rs` (1-460, 1750-2111; outline of the rest), `src/stint.rs` (1-420), `src/history.rs` (1-200), `src/lib.rs`, `src/main.rs`
- `src/status.rs` (1-80, 400-510, 555-620, and the E8/lookback test regions 1500-1520, 2230-2275, 2455-2550, 3175-3225), `src/storage.rs` (95-110, 200-215, 355-430, 1420-1470), `src/db.rs` schema (44-68), `src/time.rs` `format_minutes` (109-114), `src/date.rs` formatters (170-177)
- `tests/start_hint_cli.rs` (full), `tests/week_target_cli.rs` (30-60, 125-160), `tests/overlap_cli.rs` (1-40, 135-164), `tests/overnight_visibility_cli.rs` (outline), `tests/ascii_sweep.rs` (1-60)
- `clippy.toml` (threshold 15), `.githooks/pre-commit` (fmt, cognitive complexity, llvm-cov ratchet), `coverage-baseline.json` (99.45 %), `AGENTS.md`

## 1. Scope

### 1.1 Files this task owns

| File | Change |
|---|---|
| `src/anomalies.rs` | E8 fact type with window, shared E8 hint renderer, prefix-swappable refused-join line, `stop`'s date-warning method |
| `src/lookback.rs` | `LineFacts` gains the start's age; 12h `or:` rule; today-filtered stderr lines; module docs name `stop` |
| `src/stop_warnings.rs` (new) | `stop`'s post-write pass |
| `src/lib.rs` | `pub mod stop_warnings;` |
| `src/commands.rs` | `stop` calls the pass; module and `stop` doc comments |
| `src/render.rs` | re-export of the new fact type; t33b fixture |
| `src/status.rs` | `e8_facts()` fixture; one E8-window `resolve` test |
| `tests/start_hint_cli.rs` | `or:`-line test moves to the 12h shape |
| `tests/week_target_cli.rs` | `other_writers_stay_silent` wording |
| `tests/stop_warning_cli.rs` (new) | binary-level `stop` stderr tests |
| `coverage-baseline.json` | hook ratchet only |

`src/stint.rs` and `src/main.rs` are listed as owned by the changeset plan but need no change. `stop`'s signature is unchanged.

### 1.2 Files this task must not touch

| File | Owner |
|---|---|
| `docs/dev/SPEC.md` | Task 2 |
| `README.md` | Task 2 |
| `AGENTS.md` | Task 2 (adds the `src/stop_warnings.rs` layout bullet and names `stop` in the `src/lookback.rs` bullet, per the changeset plan's "New module" contract) |
| `docs/dev/NOTES.md` | coordinator |
| `--help` text in `src/cli.rs` | nobody (spec §9 keeps it unchanged) |

### 1.3 Dependencies

| Dependency | State | Use |
|---|---|---|
| Task 2 (docs) | parallel, not consumed | none. The only shared contract is the module name `src/stop_warnings.rs` and its one-line description, both fixed by the changeset plan. |

No stub is needed.

## 2. Facts about the current code

### 2.1 `src/anomalies.rs`

- `DateAnomalies` (36-80) holds E8 as `orphaned_end_times: Vec<NaiveTime>` (55).
- `DateAnomalies::none` (171-183) is the fixture base. Fixtures use struct update syntax.
- `has_any` (188-195) reads `!self.orphaned_end_times.is_empty()`.
- `detail_lines` (201-209) orders E7, stale, E8, overlap, refused joins.
- `e8_lines` (272-286) hardcodes `[!] orphaned end at T (no matching start)`, `fix: mlm start HH:MM --date D  (a time before T)`, `or: mlm delete punch ...`.
- `refused_join_lines` (288-297) calls the free fn `refused_join_line(named, earlier, later)` (306-313), which hardcodes the `[!]` prefix.
- `build_facts` (328-360) fills `orphaned_end_times` from `orphan_times(&ctx.day)` (446-451). It already receives `b: &DateBuckets` with the date's raw punches in `b.day`.
- `date_facts` (436-444) is `classify_context` then `build_facts`.
- Field users outside the module: `src/render.rs:621` (t33b fixture), `src/status.rs:3186` (`e8_facts()` fixture). Inside: 55, 176, 191, 275, 343, and tests at 560, 610, 668, 727, 1055, 1122.

### 2.2 `src/lookback.rs`

- `LineFacts` (44-53) derives `Default`, with fields `next` and `today_has_punches`.
- `yesterday_hint_lines` (172-187) pushes the `or:` line when `facts.next.is_none() && !facts.today_has_punches`.
- `stderr_line_with` (111-132) is `start`'s stderr form. The yesterday form has no `or:` line, and that stays.
- `entry_for_date` (261-295) builds `LineFacts` from the splice-aware `classify_at` view. The single open start is `only: &OpenStint`, which carries `minutes_so_far` computed against `now_utc` (`src/stint.rs:33`, `:180`).
- `today_page_lines` (299-311) loads buckets, asks `overnight_stint`, filters out the yesterday hint when there is no overnight stint, and flattens `detail_lines`.
- `load_buckets` (227-231) reads `[today - 15, today]`.
- Test clock `now()` is Thu 2026-02-12 18:00 local (338-343). Seeded yesterday `18:02` starts are therefore 1438 minutes old.

### 2.3 `src/stint.rs`

- `classify_at(prev, day, next, now)` (264-331) appends the spliced stint to `day.completed` with `end` on the next date and real `minutes` (315-327). The `(prev, day)` splice only removes `day`'s first orphan (309-311), so it never changes whether `day` splices into `next`.
- `Stint { start, end, minutes }` is public (18-24). `Punch.date` gives the end's date.

### 2.4 `src/commands.rs`

- `start` (24-34) writes, then `warn_stale_starts(conn, today, now_utc, written_id, &mut stderr())`.
- `stop` (39-42) writes through `punch(conn, now, PunchKind::End, args)` and returns. `punch` (315-342) returns `(target_date, punch_id)`.
- `read_failure_after_write_is_best_effort` (1974-1989) shows the drop-table pattern.

### 2.5 Other

- Punch instants are minute-granular (`src/storage.rs:102`, `:207-213`). Equal minute means equal `at_utc`.
- `punches_in_range` maps every row through `punch_from_row` (`src/storage.rs:363-375`, `398-422`). A row with `at_utc = 'not-a-timestamp'` makes the read fail with `CorruptRow` (`src/storage.rs:1445-1460` test X2). This gives a read failure confined to one date range.
- `format_minutes` (`src/time.rs:109`) prints `29h 16m`, `12h 00m`.
- `render::stop_time_hint(start, next)` (`src/render.rs:177`) and `render::next_punch_after(punches, start)` (`src/render.rs:160`) are the stop-hint helper and its `N`.
- `format_date_with_weekday` (`src/date.rs:175`) gives `Mon 2026-10-05`.
- No CLI clock override exists. Binary tests run on the real clock with `TZ=UTC` (`tests/start_hint_cli.rs:15-22`).

## 3. Design

### 3.1 Constants

```rust
// src/lookback.rs
/// A start this old warns as a long splice, and the yesterday hint stops
/// offering a bare `mlm stop` (720 minutes).
pub const LONG_STINT_MINUTES: i64 = 12 * 60;
```

Source: changeset plan "Thresholds" (720 warns, 719 silent), spec §3.2 and §6.

### 3.2 `src/anomalies.rs`

#### 3.2.1 E8 fact

```rust
/// One orphaned end (E8) and what its fix hint needs (spec §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrphanedEndFact {
    /// Local time of the end.
    pub time: NaiveTime,

    /// `P`: the latest punch on the date before this end's minute.
    pub after: Option<NaiveTime>,

    /// Another end on the date shares this end's minute.
    pub shares_minute: bool,
}

impl OrphanedEndFact {
    /// An orphan with no earlier punch and no twin. Fixtures use it.
    pub fn at(time: NaiveTime) -> Self {
        Self { time, after: None, shares_minute: false }
    }
}
```

`DateAnomalies.orphaned_end_times: Vec<NaiveTime>` becomes:

```rust
    /// One entry per orphaned end (E8), ascending by instant. Never
    /// deduplicated, never coalesced.
    pub orphaned_ends: Vec<OrphanedEndFact>,
```

The rename makes every stale fixture fail to compile, so none keeps the old type silently. `none()` and `has_any()` switch to the new name.

#### 3.2.2 Builder

Replace `orphan_times` (446-451) with:

```rust
/// E8 facts for the date. `punches` is the date's raw bucket.
fn orphan_facts(day: &DayStints, punches: &[Punch]) -> Vec<OrphanedEndFact> {
    day.orphaned_ends
        .iter()
        .map(|o| orphan_fact(&o.punch, punches))
        .collect()
}

fn orphan_fact(end: &Punch, punches: &[Punch]) -> OrphanedEndFact {
    OrphanedEndFact {
        time: render::local_time(end.at_utc),
        after: punches
            .iter()
            .filter(|p| p.at_utc < end.at_utc)
            .max_by_key(|p| (p.at_utc, p.kind, p.id))
            .map(|p| render::local_time(p.at_utc)),
        shares_minute: punches
            .iter()
            .any(|p| p.kind == PunchKind::End && p.id != end.id && p.at_utc == end.at_utc),
    }
}
```

`build_facts` sets `orphaned_ends: orphan_facts(&ctx.day, b.day)`. Add `PunchKind` to the `crate::storage` import.

The `(at_utc, kind, id)` key is pairing order (SPEC §4.3 step 1). `p.at_utc < end.at_utc` is "before T's minute" because instants are minute-granular. Only `P`'s time is stored. Minute inclusion is a pairing consequence and is tested as behaviour (test plan §5.21).

#### 3.2.3 One renderer for the E8 hint lines

```rust
/// The lines after an orphan's header, shared by `status` and `stop`
/// (spec §4.1, §5). Two lines, or one delete-only `fix:` line when another
/// end shares the orphan's minute.
pub fn e8_hint_lines(date: NaiveDate, o: &OrphanedEndFact) -> Vec<String> {
    let d = format_date(date);
    let delete = format!(
        "mlm delete punch --date {d}  (lists punches; rerun with the end's number)"
    );
    if o.shares_minute {
        return vec![format!("    fix: {delete}")];
    }
    vec![
        format!("    fix: mlm start HH:MM --date {d}  {}", start_time_hint(o.after, o.time)),
        format!("     or: {delete}"),
    ]
}

/// `(a time before T)`, or `(a time between P and T)`.
fn start_time_hint(after: Option<NaiveTime>, end: NaiveTime) -> String {
    let t = end.format("%H:%M");
    match after {
        None => format!("(a time before {t})"),
        Some(p) => format!("(a time between {} and {t})", p.format("%H:%M")),
    }
}
```

#### 3.2.4 Surface prefix

```rust
/// Where a block prints. Only the header differs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Surface {
    Status,
    Stop,
}

impl Surface {
    fn mark(self) -> &'static str {
        match self {
            Self::Status => "[!]",
            Self::Stop => "warning:",
        }
    }

    fn orphan_header(self, date: NaiveDate, time: NaiveTime) -> String {
        let t = time.format("%H:%M");
        match self {
            Self::Status => format!("[!] orphaned end at {t} (no matching start)"),
            Self::Stop => format!(
                "warning: orphaned end on {} at {t} (no matching start)",
                format_date_with_weekday(date)
            ),
        }
    }
}
```

`refused_join_line` gains the prefix as its first parameter:

```rust
fn refused_join_line(mark: &str, named: NaiveDate, earlier: NaiveDate, later: NaiveDate) -> String {
    format!(
        "{mark} not joined to {} across midnight (a join needs exactly one open start on {} and exactly one unmatched end, as {}'s first punch)",
        format_date_with_weekday(named),
        format_date_with_weekday(earlier),
        format_date_with_weekday(later),
    )
}
```

#### 3.2.5 Block methods

Replace `e8_lines` and `refused_join_lines` with surface-aware versions, and add `stop`'s entry point:

```rust
    fn e8_block(&self, surface: Surface) -> Vec<String> {
        let mut out = Vec::new();
        for o in &self.orphaned_ends {
            out.push(surface.orphan_header(self.date, o.time));
            out.extend(e8_hint_lines(self.date, o));
        }
        out
    }

    fn refused_join_block(&self, surface: Surface, with_previous: bool) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(prev) = self.refused_join_with_previous.filter(|_| with_previous) {
            out.push(refused_join_line(surface.mark(), prev, prev, self.date));
        }
        if let Some(next) = self.refused_join_with_next {
            out.push(refused_join_line(surface.mark(), next, self.date, next));
        }
        out
    }

    /// `stop`'s warnings for this date (spec §4): orphan blocks ascending,
    /// then the refused join with the previous date (when `with_previous`),
    /// then with the next date.
    pub fn stop_warning_lines(&self, with_previous: bool) -> Vec<String> {
        let mut out = self.e8_block(Surface::Stop);
        out.extend(self.refused_join_block(Surface::Stop, with_previous));
        out
    }
```

`detail_lines` calls `self.e8_block(Surface::Status)` and `self.refused_join_block(Surface::Status, true)` in the existing positions. `status` text is unchanged except for §5's hint rules.

### 3.3 `src/lookback.rs`

#### 3.3.1 Line facts

```rust
pub struct LineFacts {
    pub next: Option<NaiveTime>,
    pub today_has_punches: bool,
    /// Whole minutes the single open start has run at `now`. `Default` is 0,
    /// which keeps the `or:` line.
    pub start_minutes: i64,
}
```

`Default` stays derived, so `LineFacts::default()` (`src/commands.rs:1762`, `LookbackFinding::detail_lines` at 74-76) keeps its meaning.

`entry_for_date` returns `(kind, next, minutes)`. The single-start arm yields `only.minutes_so_far`, and the many arm yields `0`. It sets `start_minutes` from it.

#### 3.3.2 12h rule

`yesterday_hint_lines` condition becomes:

```rust
    if facts.next.is_none()
        && !facts.today_has_punches
        && facts.start_minutes < LONG_STINT_MINUTES
    {
```

The `fix:` line and `stderr_line_with` are unchanged (spec §6).

#### 3.3.3 Today-filtered entries and stderr lines

Split the filter out of `today_page_lines` so the `status` page and `stop` share one definition:

```rust
/// Today's lookback entries, oldest first. The yesterday hint survives only
/// when `overnight_stint` finds a stint, as on today's `status` page.
fn today_entries(
    buckets: &PunchBuckets,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
) -> Vec<LookbackEntry> {
    let overnight = overnight_stint(buckets, today, now_utc).is_some();
    scan(buckets, today, now_utc)
        .into_iter()
        .filter(|e| overnight || !e.finding.is_yesterday_hint())
        .collect()
}

pub fn today_page_lines(conn, today, now_utc) -> anyhow::Result<Vec<String>> {
    let buckets = load_buckets(conn, today)?;
    Ok(today_entries(&buckets, today, now_utc)
        .iter()
        .flat_map(LookbackEntry::detail_lines)
        .collect())
}

/// Today's lookback in the stderr forms, filtered as today's page is.
/// One string per finding, which may hold newlines. `mlm stop` prints these.
pub fn today_stderr_lines(
    conn: &Connection,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
) -> anyhow::Result<Vec<String>> {
    let buckets = load_buckets(conn, today)?;
    Ok(today_entries(&buckets, today, now_utc)
        .iter()
        .map(LookbackEntry::stderr_line)
        .collect())
}
```

#### 3.3.4 Docs

- Module doc (1-5): "This module owns the scan, its finding types and the stderr forms that `mlm start` and `mlm stop` print."
- `stderr_line_with` doc (107-110): "The stderr form `start` and `stop` print (§7.4, §3.1)."
- `lookback` doc (249-250): add "and, through [`today_stderr_lines`], by `mlm stop`'s warnings".

### 3.4 `src/stop_warnings.rs` (new)

```rust
//! `stop`'s post-write warnings: orphaned ends, refused joins, long splices,
//! then today's lookback.
//!
//! Best-effort, like `start`'s (SPEC §7.4). Every read happens before any
//! line prints, so a failed read prints nothing.

use std::io::Write;

use chrono::{DateTime, Duration as ChronoDuration, NaiveDate, Utc};
use rusqlite::Connection;

use crate::anomalies;
use crate::date::{format_date, format_date_with_weekday};
use crate::history::PunchBuckets;
use crate::lookback::{self, LONG_STINT_MINUTES};
use crate::render;
use crate::stint::{self, Stint};
use crate::storage::{self, Punch};
use crate::time::format_minutes;

/// Warns after a committed `stop` write to `written`. Prints nothing when
/// no warning triggers or any read fails.
pub fn warn_after_stop<W: Write>(
    conn: &Connection,
    written: NaiveDate,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
    out: &mut W,
) {
    let Ok(lines) = collect(conn, written, today, now_utc) else {
        return;
    };
    for line in lines {
        let _ = writeln!(out, "{line}");
    }
}

/// Every line, reads first. The lookback is read only when a warning
/// triggered.
fn collect(
    conn: &Connection,
    written: NaiveDate,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
) -> anyhow::Result<Vec<String>> {
    let (from, to) = read_range(written, today);
    let buckets = PunchBuckets::from_punches(&storage::punches_in_range(conn, from, to)?);
    let mut lines = warning_lines(&buckets, written, today, now_utc);
    if !lines.is_empty() {
        lines.extend(lookback::today_stderr_lines(conn, today, now_utc)?);
    }
    Ok(lines)
}

/// `D-1` through `D+2`. When `D` is today, through `D+1` only: `D`'s own
/// classification reads tomorrow as today's `status` does.
fn read_range(written: NaiveDate, today: NaiveDate) -> (NaiveDate, NaiveDate) {
    let after = if written == today { 1 } else { 2 };
    (
        written - ChronoDuration::days(1),
        written + ChronoDuration::days(after),
    )
}

/// The warnings before the lookback, in spec §4 order: date warnings for
/// `D`, then `D+1`, then long-splice blocks ending on `D`, then on `D+1`.
/// `D+1` is skipped when `D` is today.
pub(crate) fn warning_lines(
    buckets: &PunchBuckets,
    written: NaiveDate,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
) -> Vec<String> {
    let next = (written < today).then(|| written + ChronoDuration::days(1));
    let mut lines = date_lines(buckets, written, today, now_utc, true);
    lines.extend(next.into_iter().flat_map(|n| date_lines(buckets, n, today, now_utc, false)));
    lines.extend(long_splice_into(buckets, written, now_utc));
    lines.extend(next.into_iter().flat_map(|n| long_splice_into(buckets, n, now_utc)));
    lines
}

/// One date's orphan blocks and refused-join lines, from the fact builder
/// `status` uses. `D+1` passes `with_previous = false`, because a refused
/// join between `D` and `D+1` prints once, as `D`'s line naming `D+1`
/// (spec §3.1).
fn date_lines(
    buckets: &PunchBuckets,
    date: NaiveDate,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
    with_previous: bool,
) -> Vec<String> {
    anomalies::date_facts(&buckets.trio(date), date, today, now_utc)
        .stop_warning_lines(with_previous)
}

/// The long-splice block for the splice ending on `end_date`, when it ran
/// 12h or more.
fn long_splice_into(
    buckets: &PunchBuckets,
    end_date: NaiveDate,
    now_utc: DateTime<Utc>,
) -> Vec<String> {
    let start_date = end_date - ChronoDuration::days(1);
    let day = buckets.day(start_date);
    // The earlier neighbour never changes whether `start_date` splices
    // forward, so it is not passed.
    stint::classify_at(&[], day, buckets.day(end_date), now_utc)
        .completed
        .iter()
        .find(|s| s.end.date == end_date && s.minutes >= LONG_STINT_MINUTES)
        .map_or_else(Vec::new, |s| long_splice_block(start_date, day, s))
}

/// Spec §4.3's three lines.
fn long_splice_block(start_date: NaiveDate, start_day: &[Punch], s: &Stint) -> Vec<String> {
    let start = render::local_time(s.start.at_utc);
    let end = render::local_time(s.end.at_utc).format("%H:%M");
    let a = format_date_with_weekday(start_date);
    vec![
        format!(
            "warning: end on {} at {end} joins the start on {a} at {} ({}, counted on {a})",
            format_date_with_weekday(s.end.date),
            start.format("%H:%M"),
            format_minutes(s.minutes),
        ),
        format!(
            "    fix: mlm stop HH:MM --date {}  {}",
            format_date(start_date),
            render::stop_time_hint(start, render::next_punch_after(start_day, &s.start)),
        ),
        format!(
            "   then: mlm status {}  (the {end} end shows as orphaned, with its fixes)",
            format_date(s.end.date)
        ),
    ]
}
```

Notes for the implementer:

- `read_range` covers every trio the pass reads. `D`'s trio is `D-1..D+1`, `D+1`'s is `D..D+2`. The splice into `D` classifies `D-1` against `D`, and the splice into `D+1` classifies `D` against `D+1`.
- The splice detection uses `classify_at`'s own splice (`src/stint.rs:315-327`), so the 12h check applies to exactly the stint `status` counts.
- No branch in this module is unreachable, so the coverage ratchet holds when the tests in §5 run. Avoid `let Some(..) = x.pred_opt() else { return }` forms, because their `return` lines are never hit.
- Every function sits well under complexity 15. `warning_lines` is the largest, at four sequential statements with no branching beyond `then`.

### 3.5 `src/commands.rs`

```rust
use crate::stop_warnings;

/// Record an end punch for today, optionally with an inline note
/// (SPEC §3.3), or against another day via `--date`. After a successful
/// write, warns on stderr about orphaned ends, refused joins and long
/// splices on the written date and the next one, then today's lookback
/// (§7.4). Best-effort, like `start`.
pub fn stop(conn: &mut Connection, now: DateTime<Local>, args: &PunchArgs) -> anyhow::Result<()> {
    stop_to(conn, now, args, &mut std::io::stderr())
}

/// `stop` with its warnings written to `err`.
pub(crate) fn stop_to<W: std::io::Write>(
    conn: &mut Connection,
    now: DateTime<Local>,
    args: &PunchArgs,
    err: &mut W,
) -> anyhow::Result<()> {
    let (written, _) = punch(conn, now, PunchKind::End, args)?;
    stop_warnings::warn_after_stop(conn, written, now.date_naive(), now.with_timezone(&Utc), err);
    Ok(())
}
```

A hard error in `punch` returns before the pass, so §6.1 errors are unchanged. The module doc (1-8) gains one sentence: "`start` and `stop` print best-effort warnings on stderr after a successful write (§7.4)."

### 3.6 `src/lib.rs`

Add `pub mod stop_warnings;` between `pub mod stint;` and `pub mod storage;`.

### 3.7 `src/render.rs`

- Line 127: `pub use crate::anomalies::{DateAnomalies, OrphanedEndFact, StaleOpenStart};`
- t33b fixture (620-623): `orphaned_ends: vec![OrphanedEndFact::at(t(18, 0))]`. Add a fixture with `after: Some(t(12, 0))` and one with `shares_minute: true` so the sweep covers both new forms.

### 3.8 `src/status.rs`

- `e8_facts()` (3184-3189): `orphaned_ends: vec![OrphanedEndFact::at(t(18, 0))]`. Import `anomalies::OrphanedEndFact` in the test module.
- Add one `resolve` test (§5.21, last bullet).

## 4. Order of work

1. `src/anomalies.rs`: fact type, builder, renderer, surface, methods; fix every fixture (§2.1 list). Run `cargo test anomalies`.
2. `src/render.rs`, `src/status.rs` fixture updates. Run `cargo build --all-targets`.
3. `src/lookback.rs`: `start_minutes`, 12h rule, `today_entries`, `today_stderr_lines`, docs; update tests (§5.20, §5.22). Run `cargo test lookback`.
4. `src/stop_warnings.rs`, `src/lib.rs`, `src/commands.rs`.
5. Unit tests in `src/stop_warnings.rs`, binary tests in `tests/stop_warning_cli.rs`, `tests/start_hint_cli.rs` and `tests/week_target_cli.rs` edits.
6. Full verification set: `cargo fmt`, `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings`. Then run the cognitive-complexity check as the hook does: `cargo clippy --all-targets -- -W clippy::cognitive_complexity`.

## 5. Test plan

### 5.0 Harness

Unit tests for `stop` live in `src/stop_warnings.rs`'s `mod tests`. They go through `crate::commands::stop_to` with a `Vec<u8>` writer, so they cover arg parsing, the write, the pass and the exit code. Each asserts:

- exact `String::from_utf8(err)` text,
- `crate::exit_code(&r) == 0`,
- stdout is empty by construction, because no code on the `stop` path prints to stdout. The binary tests in §5.24 assert it directly.

Clock: `fn now() -> DateTime<Local>` is **Mon 2026-02-09 14:16 local**, so `today()` is Mon 2026-02-09. The spec's examples use Mon 2026-10-05. Their structure is kept, but the dates are moved to February, because Australian DST starts Sun 2026-10-04 and would turn `29h 16m` into `28h 16m` on a runner in that zone. February 7-9 2026 has no DST change in any common zone. Weekdays: 2026-02-09 Mon, 02-08 Sun, 02-07 Sat, 02-06 Fri, 02-04 Wed, 01-20 Tue, 01-15 Thu.

Helpers:

```rust
fn test_db() -> Connection                       // in-memory + migrations, as src/commands.rs:367
fn stop_args(argv: &[&str]) -> PunchArgs         // as src/commands.rs:416
fn seed(conn: &Connection, kind: PunchKind, days_ago: i64, h: u32, m: u32)  // storage::insert_punch with &Local
fn run_stop(conn: &mut Connection, argv: &[&str]) -> String  // stop_to, asserts exit 0, returns stderr
```

Lines below use `MON = Mon 2026-02-09`, `SUN = Sun 2026-02-08`, `SAT = Sat 2026-02-07`.

### 5.1 Orphan on `D`, empty lookback (spec §8 first bullet)

`stop 08:00 --date 2026-02-06` on an empty db. Expect exactly:

```
warning: orphaned end on Fri 2026-02-06 at 08:00 (no matching start)
    fix: mlm start HH:MM --date 2026-02-06  (a time before 08:00)
     or: mlm delete punch --date 2026-02-06  (lists punches; rerun with the end's number)
```

### 5.2 FE5 (spec §4.5 first example, §8)

Seed `start 22:00` on SAT. Bare `stop` (14:16). Expect:

```
warning: orphaned end on Mon 2026-02-09 at 14:16 (no matching start)
    fix: mlm start HH:MM --date 2026-02-09  (a time before 14:16)
     or: mlm delete punch --date 2026-02-09  (lists punches; rerun with the end's number)
warning: unclosed start on Sat 2026-02-07 at 22:00 (2 days ago)
    fix: mlm stop HH:MM --date 2026-02-07  (a time after 22:00)
     or: mlm delete punch --date 2026-02-07  (lists punches; rerun with the start's number)
```

### 5.3 Clean pair, stale start in the window: silent (spec §1.1, §8)

Seed `start 09:00` five days ago (02-04) and `start 09:00` today. `stop 12:00`. Expect `""`.

### 5.4 Displaced end (spec §3.1, §5, §8)

Seed `start 09:00`, `end 17:00` on 02-06. `stop 12:00 --date 2026-02-06`. Expect the three-line block for `17:00` with `(a time between 12:00 and 17:00)`.

### 5.5 Pre-existing orphan plus a new clean pair (spec §8, §1.2)

Seed `end 08:00`, `start 09:00` on 02-06. `stop 12:00 --date 2026-02-06`. Expect the `08:00` block with `(a time before 08:00)` and nothing else.

### 5.6 Two orphans at different minutes, ascending (spec §4, §8)

Seed `end 08:00` on 02-06. `stop 10:00 --date 2026-02-06`. Expect the `08:00` block `(a time before 08:00)`, then the `10:00` block `(a time between 08:00 and 10:00)`.

### 5.7 Duplicate same-minute stop (spec §4.1 second form, §5, §8)

Seed `end 12:00` on 02-06. `stop 12:00 --date 2026-02-06`. Expect two two-line blocks:

```
warning: orphaned end on Fri 2026-02-06 at 12:00 (no matching start)
    fix: mlm delete punch --date 2026-02-06  (lists punches; rerun with the end's number)
warning: orphaned end on Fri 2026-02-06 at 12:00 (no matching start)
    fix: mlm delete punch --date 2026-02-06  (lists punches; rerun with the end's number)
```

### 5.8 Double-tapped `stop` after a splice (spec §4.5 third example, exact)

Seed SUN `start 22:00`. `stop 01:00` returns `""` (3h splice, silent). A second `stop 01:00` returns exactly:

```
warning: orphaned end on Mon 2026-02-09 at 01:00 (no matching start)
    fix: mlm delete punch --date 2026-02-09  (lists punches; rerun with the end's number)
warning: orphaned end on Mon 2026-02-09 at 01:00 (no matching start)
    fix: mlm delete punch --date 2026-02-09  (lists punches; rerun with the end's number)
warning: not joined to Sun 2026-02-08 across midnight (a join needs exactly one open start on Sun 2026-02-08 and exactly one unmatched end, as Mon 2026-02-09's first punch)
```

No yesterday warning prints, because `overnight_stint` refuses the join (spec §4.4).

### 5.9 Backdated `stop` breaks `D+1`'s splice (spec §3.1 second bullet)

Seed `start 22:00` on SUN, `end 01:00` on MON. `stop 23:00 --date -1`. Expect exactly the MON block:

```
warning: orphaned end on Mon 2026-02-09 at 01:00 (no matching start)
    fix: mlm start HH:MM --date 2026-02-09  (a time before 01:00)
     or: mlm delete punch --date 2026-02-09  (lists punches; rerun with the end's number)
```

### 5.10 `D` is today: `D+1` checks skipped, `D` uses tomorrow (spec §3, §8)

Two tests through `stop_to`. Tomorrow's punches are seeded with `storage::insert_punch` directly, since the CLI refuses future dates.

- Seed tomorrow (02-10) `end 08:00`, today `start 09:00`. `stop 12:00`. Expect `""`. A `D+1` check would have printed the 02-10 orphan.
- Seed today `start 22:00`, tomorrow `end 01:00` and `end 02:00`, today `start 09:00`. `stop 12:00` (pairs with 09:00, leaving 22:00 open). Expect exactly today's refused-join line naming tomorrow:
  `warning: not joined to Tue 2026-02-10 across midnight (a join needs exactly one open start on Mon 2026-02-09 and exactly one unmatched end, as Tue 2026-02-10's first punch)`.
  No 02-10 orphan blocks print.

Also unit-test `read_range`. `(MON, MON)` gives `(SUN, 02-10)`, and `(SUN, MON)` gives `(SAT, 02-10)`.

### 5.11 Refused join between `D` and `D+1` prints once (spec §3.1, changeset plan risk 1)

Seed SUN `start 22:00`, MON `end 01:00`, `end 02:00`. `stop 21:00 --date -1`. Expect exactly:

```
warning: orphaned end on Sun 2026-02-08 at 21:00 (no matching start)
    fix: mlm start HH:MM --date 2026-02-08  (a time before 21:00)
     or: mlm delete punch --date 2026-02-08  (lists punches; rerun with the end's number)
warning: not joined to Mon 2026-02-09 across midnight (a join needs exactly one open start on Sun 2026-02-08 and exactly one unmatched end, as Mon 2026-02-09's first punch)
warning: orphaned end on Mon 2026-02-09 at 01:00 (no matching start)
    fix: mlm start HH:MM --date 2026-02-09  (a time before 01:00)
     or: mlm delete punch --date 2026-02-09  (lists punches; rerun with the end's number)
warning: orphaned end on Mon 2026-02-09 at 02:00 (no matching start)
    fix: mlm start HH:MM --date 2026-02-09  (a time between 01:00 and 02:00)
     or: mlm delete punch --date 2026-02-09  (lists punches; rerun with the end's number)
```

The yesterday lookback entry is filtered out because the join is refused.

### 5.12 Long-splice thresholds (spec §3.2, §4.3, §4.5 second example)

- 11h 59m: seed SUN `start 21:01`. `stop 09:00`. Expect `""`.
- Exactly 12h: seed SUN `start 21:00`. `stop 09:00`. Expect:
  ```
  warning: end on Mon 2026-02-09 at 09:00 joins the start on Sun 2026-02-08 at 21:00 (12h 00m, counted on Sun 2026-02-08)
      fix: mlm stop HH:MM --date 2026-02-08  (a time after 21:00)
     then: mlm status 2026-02-09  (the 09:00 end shows as orphaned, with its fixes)
  ```
- 29h 16m (spec §4.5 second example): seed SUN `start 09:00`. Bare `stop`. Expect:
  ```
  warning: end on Mon 2026-02-09 at 14:16 joins the start on Sun 2026-02-08 at 09:00 (29h 16m, counted on Sun 2026-02-08)
      fix: mlm stop HH:MM --date 2026-02-08  (a time after 09:00)
     then: mlm status 2026-02-09  (the 14:16 end shows as orphaned, with its fixes)
  ```

### 5.13 Long splice with a punch after `S` names the window (spec §4.3)

Seed SUN `start 09:00`, `start 10:00`, `end 11:00`. Bare `stop`. Expect the 29h 16m block with `fix: mlm stop HH:MM --date 2026-02-08  (a time between 09:00 and 10:00)`.

### 5.14 Backdated `stop` clears `D`'s E7 and splices 12h+ into `D+1` (spec §3.2)

Seed SUN `start 08:00`, `start 09:00`, MON `end 01:00`. `stop 10:00 --date -1`. Expect exactly:

```
warning: end on Mon 2026-02-09 at 01:00 joins the start on Sun 2026-02-08 at 08:00 (17h 00m, counted on Sun 2026-02-08)
    fix: mlm stop HH:MM --date 2026-02-08  (a time between 08:00 and 09:00)
   then: mlm status 2026-02-09  (the 01:00 end shows as orphaned, with its fixes)
```

### 5.15 `D+1` orphan and a long splice in spec §4 order

Seed SAT `start 09:00`, MON `end 07:00`. `stop 08:00 --date -1` (SAT->SUN splice, 23h). Expect the MON `07:00` orphan block `(a time before 07:00)` first, then:

```
warning: end on Sun 2026-02-08 at 08:00 joins the start on Sat 2026-02-07 at 09:00 (23h 00m, counted on Sat 2026-02-07)
    fix: mlm stop HH:MM --date 2026-02-07  (a time after 09:00)
   then: mlm status 2026-02-08  (the 08:00 end shows as orphaned, with its fixes)
```

The date warnings for `D` and then `D+1` come before every long-splice block.

### 5.16 Orphan, then the yesterday warning (spec §4.4)

Seed SUN `start 18:02`. `stop 08:00 --date 2026-02-06`. Expect the 02-06 block, then:

```
warning: start on Sun 2026-02-08 at 18:02 still open (yesterday)
    fix: mlm stop HH:MM --date 2026-02-08  (a time after 18:02)
```

### 5.17 `--date` older than 14 days (spec §1.2, §4.4)

Seed `start 09:00` on 02-04 (age 5) and on 01-15 (age 25). `stop 17:00 --date 2026-01-20`. Expect the Tue 2026-01-20 block `(a time before 17:00)`, then the Wed 2026-02-04 `unclosed start ... (5 days ago)` three lines. Nothing names 2026-01-15, and nothing contains `carry-in`.

### 5.18 Read failure after the committed write (spec §7)

- Date read fails: write an orphan with `stop_to`, then `DROP TABLE punches` and call `warn_after_stop` directly. Expect `""`. A dropped table also blocks the write, so this case runs at the `warn_after_stop` seam, as `src/commands.rs:1974-1989` does for `start`.
- Only the lookback read fails: raw-insert `INSERT INTO punches (at_utc, "date", kind) VALUES ('not-a-timestamp', '2026-02-06', 'start')`. `stop 17:00 --date 2026-01-20` through `stop_to`. The date read covers 01-19..01-22 and succeeds. The lookback read covers 01-25..02-09 and fails. Expect `""`, exit 0, and `punches_for_date(2026-01-20)` holds one `end`. This proves no orphan block printed before the failing read.

### 5.19 No carry marker, no own-punch suppression (spec §4.4)

Seed `start 09:00` on 02-04. `stop 08:00 --date 2026-02-04`. Expect the 02-04 orphan block `(a time before 08:00)`, then the 02-04 `unclosed start at 09:00 (5 days ago)` lines with `(a time after 09:00)`. The written date's own lookback entry prints. §5.17 covers the absent marker.

### 5.20 Yesterday hint boundary, 719 vs 720 (spec §6, changeset plan "Thresholds")

In `src/lookback.rs` tests:

- Unit: `single(1).detail_lines_with(LineFacts { next: None, today_has_punches: false, start_minutes: 719 })` has three lines. With `720`, it has two (`[i]` and `fix:`).
- Page: seed yesterday `start 18:00`. `today_page_lines` at today 05:59 gives three lines. At today 06:00 it gives two. Add `fn now_at(h: u32, m: u32) -> DateTime<Utc>` on `today()`.

### 5.21 `status` E8 window cases (spec §5, §1.2)

In `src/anomalies.rs` tests, through `facts(&[], &day, &[], date)` on a past date, asserting exact `detail_lines()`:

- No earlier punch: `end 18:00` alone gives `(a time before 18:00)`. The existing `t_e8_gains_fix_lines_text_unchanged` covers the fixture path.
- Earlier closed stint, `P`'s minute included: `start 09:00`, `end 12:00`, `end 17:00` gives `(a time between 12:00 and 17:00)`. Adding `start 12:00` leaves `has_any() == false`, which is the inclusion as behaviour.
- Earlier orphan as `P`, minute excluded: `end 08:00`, `end 10:00` gives `(a time between 08:00 and 10:00)` on the second block. Adding `start 08:00` still leaves `10:00` in `orphaned_ends`.
- E14 pair as `P`: `start 12:00`, `end 12:00`, `end 17:00` gives `(a time between 12:00 and 17:00)`. Adding `start 12:00` gives `has_any() == false`.
- Another end at `T`'s minute: `start 09:00`, `end 17:00`, `end 17:00` gives one two-line block (`[!] orphaned end at 17:00 (no matching start)`, `    fix: mlm delete punch --date D  (lists punches; rerun with the end's number)`).
- Empty window: `end 12:00`, `end 12:01` gives `(a time before 12:00)`, then `(a time between 12:00 and 12:01)`.
- Fact level: `orphan_fact` sets `after` from the max `(at_utc, kind, id)` below `T`, and `shares_minute` ignores the orphan's own id.
- Through `status::resolve` in `src/status.rs`: today `start 09:00`, `end 12:00`, `end 17:00` puts `    fix: mlm start HH:MM --date 2026-02-12  (a time between 12:00 and 17:00)` in `anomaly_lines`.

Also unit-test `stop_warning_lines(false)` drops only the previous-side line and keeps the next-side line.

### 5.22 `status` yesterday hint (spec §6)

`src/lookback.rs`:

- `page_shows_three_line_hint_when_today_is_empty` moves to clock `now_at(6, 0)` with yesterday `18:02` (718 minutes). It keeps `HINT`.
- `hint_is_last_after_older_findings` keeps `now()` (1438 minutes) and asserts 8 lines, with `lines[6..] == HINT[..2]`.
- `fix:` is identical in both shapes, which the `HINT[..2]` comparisons check.

### 5.23 Existing tests updated, not deleted (changeset plan Task 1 last criterion)

| Test | Change |
|---|---|
| `tests/start_hint_cli.rs` `status_hint_with_or_line_when_today_is_empty` | Rename to `status_hint_drops_or_line_once_the_start_is_12h_old`. Seed yesterday `00:00` (always 24h+ old), assert the `[i]` and `fix:` lines are present and `OR_LINE` is absent. The `or:`-present case lives in §5.20 and §5.22 with an injected clock. |
| `src/lookback.rs` `lookback_entry_facts_have_no_next_without_a_later_punch` | `entries[0].facts == facts(None, true)` fails once `start_minutes` is set. Assert `next` and `today_has_punches` field by field. |
| `src/lookback.rs` `facts()` helper | Sets `start_minutes: 0`. |
| `src/lookback.rs` `page_shows_three_line_hint_when_today_is_empty`, `hint_is_last_after_older_findings` | §5.22 |
| `src/anomalies.rs`, `src/render.rs`, `src/status.rs` fixtures | `orphaned_end_times` -> `orphaned_ends` with `OrphanedEndFact::at` |
| `tests/week_target_cli.rs` `other_writers_stay_silent` | Rename to `other_writers_keep_stdout_empty`. It asserts stdout only, and a real-clock `stop` before 09:00 now warns on stderr. |

`tests/start_hint_cli.rs` `stop_today_joins_and_clears_the_hint` stays as is. It may print a long-splice warning after 11:00 UTC, but it asserts only the later `status` stdout.

### 5.24 Binary tests, `tests/stop_warning_cli.rs` (new)

Same harness as `tests/start_hint_cli.rs` (`TZ=UTC`, `boot()`, raw `seed`). Each asserts exit 0, `stdout == ""` and exact stderr built with `format_date`/`format_date_with_weekday`:

- FE5: seed `start 22:00` two days ago, run `mlm stop 00:30`. The expected stderr is the orphan block for today at `00:30` `(a time before 00:30)`, then the `unclosed start ... (2 days ago)` lines.
- Silent: seed `start 09:00` five days ago and `start 00:00` today, run `mlm stop 00:10`. Stderr is `""`.
- Double tap: seed `start 22:00` yesterday. `mlm stop 01:00` gives `""`. A second `mlm stop 01:00` gives the §5.8 text with real dates.

None of these depend on the time of day, apart from a run that straddles midnight. The existing CLI tests carry the same caveat.

### 5.25 ASCII

`render.rs` t33b covers the new E8 forms (§3.7). The stop unit tests assert `is_ascii()` on one combined output (§5.11 text).

## 6. Risks, ambiguities, disagreements

1. **Spec dates moved in unit tests.** Unit tests reproduce spec §4.5 with February 2026 dates (§5.0). The text is identical apart from dates. The 2026-10-04 Australian DST change would break a `Local`-based 29h 16m assertion. The binary tests run in UTC and cover the real-date shape.
2. **`P` across a spliced first end.** When `D`'s first end splices with `D-1`'s start and a later orphan `T` exists, `P` can be that spliced end. A start at `P`'s minute would E14-pair with it and break the splice. The hint text is still correct (`P` is printed, the inclusion is not), but the spec's "window includes `P`'s minute when `P` closed a stint" does not hold for this shape. This is a SPEC note for Task 2 or a later changeset, and no code change is proposed.
3. **"`D+1` not read" (spec §8) versus the changeset plan.** When `D` is today, the pass reads `D+1` (tomorrow) because `D`'s own trio needs it, as today's `status` does. Only `D+1`'s checks are skipped. This follows the changeset plan's reconciliation, and §5.10 tests the observable half.
4. **Lookback read is conditional.** The lookback is read only after a warning triggered. This is still reads-before-print (spec §7), and it keeps a silent `stop` to one range read.
5. **`stop`'s exit code with a failing stderr.** `writeln!` errors are discarded, as in `start` (`src/commands.rs:83`).
6. **Real-clock CLI tests.** `stop_today_joins_and_clears_the_hint` and `other_writers_keep_stdout_empty` may now print stderr warnings depending on the hour. Both assert stdout only. Any new binary test must use explicit times (§5.24).
7. **Field rename churn.** Renaming `orphaned_end_times` touches fixtures in three files. A parallel field would avoid that, but two vectors that must stay in step are a worse trade.
8. **Coverage.** The baseline is 99.45 %. `warn_after_stop`'s error arm, both arms of `read_range`, `Surface::Status`/`Stop`, both `start_time_hint` arms and the two-line form each have a test above. The implementer should check `cargo llvm-cov` before committing.
