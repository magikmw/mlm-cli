# Task plan: overnight-visibility, Task 1 (code)

> **Archived — historical planning record.** Not authoritative. Current spec: `docs/dev/SPEC.md`. Design/process history: `docs/dev/NOTES.md`.

Changeset plan: `docs/dev/plans/overnight-visibility-plan.md` (Task 1, contracts 1-6).
Spec: `docs/dev/specs/2026-10-04-overnight-visibility.md` (sections 3, 4, 5, 6, 8).

## What I read

- `docs/dev/plans/overnight-visibility-plan.md`, `docs/dev/specs/2026-10-04-overnight-visibility.md` (both in full)
- Sibling task plans: none exist yet for this changeset (`ls docs/dev/plans` shows only the changeset plan). Task 2 (docs) has no code interface, so nothing from it is needed beyond the pinned contract texts.
- `src/lookback.rs` (all 969 lines), `src/week_view.rs` (all 1480), `src/status.rs` lines 1-760 plus the `StatusView` literal sites and the `resolve_*` test index, `src/cli.rs` 1-80 and 270-285, 760-775
- `src/history.rs` 1-260 (`PunchBuckets`, `load_history`, `read_range`), `src/render.rs` 1-30, 125-175, 550-650
- `src/anomalies.rs` (`DateContext`, `DateAnomalies`, `classify_context`, `build_facts`, `refused_neighbor`), `src/stint.rs` (`OpenStint`, `classify_at`, `splice_candidate`, `is_refused_join`)
- `src/time.rs:109` (`format_minutes`), `src/date.rs` (`format_date_with_weekday`, `WeekId` methods), `src/main.rs`, `src/lib.rs`
- `tests/start_hint_cli.rs`, `tests/week_target_cli.rs` 150-200, `tests/ascii_sweep.rs`, `tests/carry_marker_cli.rs` 1-60
- `clippy.toml` (cognitive-complexity-threshold = 15), `.githooks/pre-commit`, `AGENTS.md`

## Scope

Owns exclusively (per changeset plan): `src/lookback.rs`, `src/status.rs`, `src/week_view.rs`, `src/render.rs`, `src/cli.rs`, and `tests/` (new `tests/overnight_visibility_cli.rs`, edits to `tests/week_target_cli.rs` and `tests/start_hint_cli.rs`).

Must not touch:

| File | Owner |
|---|---|
| `docs/dev/SPEC.md`, `README.md` | Task 2 (docs) |
| `docs/dev/NOTES.md` | coordinator |
| `src/history.rs`, `src/anomalies.rs`, `src/stint.rs` | read only (report back if an edit seems needed) |
| `src/commands.rs` | not in scope; it calls `lookback::lookback`, whose signature is unchanged |

## Dependencies

| Dependency | State | Handling |
|---|---|---|
| Task 2 docs | parallel, no code coupling | none. Glossary sentence is pinned by contract 6 and quoted below. |
| `history::PunchBuckets::{trio, day}` | shipped (`src/history.rs:35-49`) | call as published |
| `anomalies::classify_context`, `stint::is_refused_join` (`pub(crate)`, `src/stint.rs:394`) | shipped | call as published; `pub(crate)` is reachable from `lookback.rs` |
| `history::load_history` padding | shipped (`src/history.rs:123-129`) | `week` needs no new read; see Fact F7 |

## Facts gathered

- F1. `src/lookback.rs:195-211` `lookback(conn, today, now_utc)` reads `[today-15, today]`, builds `PunchBuckets`, then maps ages 14..1 through `entry_for_date` (215-249). Age-1 lone open start gives `LookbackKind::SingleOpenStart`, and `is_yesterday_hint()` (135) is the hint test.
- F2. `lookback.rs:255-266` `today_page_lines(conn, today, now_utc, refused_join_names_yesterday: bool)` filters the hint by the flag. Its only non-test caller is `src/status.rs:445-450`, which passes `facts.refused_join_with_previous.is_some()`. `commands::start` (`src/commands.rs:56`) calls `lookback()` directly, not `today_page_lines`, and must keep the yesterday warning even on a refused join. So the refused-join suppression must stay out of `lookback()`.
- F3. Today's `facts.refused_join_with_previous` is `stint::is_refused_join(&prev_raw, &day_raw, today_punches)` (`anomalies.rs:345-350`, `463-471`) where `prev_raw` is the raw classification of yesterday's bucket. Computing it from yesterday's side gives `is_refused_join(&ctx_y.day_raw, &ctx_y.next_raw, trio_y.next)`, the same three inputs. So one function can decide it from the buckets alone.
- F4. `stint::OpenStint.minutes_so_far` is `(now - start.at_utc).num_minutes().max(0)` (`stint.rs:28-34`, `180`). It is the "whole minutes from the start to now" the spec asks for. No recomputation is needed.
- F5. `status.rs:145-179` `StatusView`: `stints` is documented "sorted by start ascending"; `render` (262-293) prints the stint section only when `stints` is non-empty (279-282). `StintLine` (128) is `Copy` and its age variants drive `status -1` wording, so it stays untouched. `StatusView` literals exist at `status.rs` 466 (production), 555, 995, 1125, 1218, 1236, 1270 (tests).
- F6. `week_view.rs:42-52` `WeekRow` (literals at 174 production, 262 test fixture `row()`), `59-73` `WeekView` (literals at 211 production, 286, 312, 416, 438 tests). `push_rows` 97-114, `push_notices` 119-132. `build_rows_from(dates, buckets, today, now_utc)` 162-183; test-only `build_rows(dates, punches, today, now_utc)` 186-193 is called by about 15 tests, so its signature stays.
- F7. `history::read_range` (`history.rs:123-129`) pads the read by one day on each side of `[start_week.start, shown.end]`. Shown week contains yesterday implies `end >= yesterday`, so today (`end+1`) is in range. Shown week is a past week ending on Sunday, today is Monday, today = `end+1`, in range. Walk start <= shown, so a future week still covers through today. When the shown week neither contains yesterday nor is current, `overnight_stint` may return a value, but the date matches no row and the Monday field requires the week to contain today, so nothing renders.
- F8. `render.rs:134` `WEEK_ANOMALY_FOOTER`. `render.rs` imports `crate::date::{WeekId, format_weekday_full}` (line 22). The ASCII test `t33b_open_punch_visibility_literals_are_ascii` ends at `render.rs:647`.
- F9. `cli.rs:44-48` glossary `[!], [i]` entry ends with `yesterday. It never marks a row.`. Clap has no `wrap_help` feature (`Cargo.toml:32`), so the glossary prints verbatim. `tests/week_target_cli.rs:184` asserts `s.trim_end().ends_with("It never marks a row.")` across six help invocations.
- F10. `main.rs` takes the clock from `Local::now()` with no override env var. CLI tests therefore run on the real clock with `TZ=UTC` and seed with raw SQL (pattern: `tests/start_hint_cli.rs:44-58`). Exact durations and Monday behavior are tested at unit level with an injected `now`.
- F11. Existing tests in the touched files that assert the old rule (the three the changeset plan names): `src/lookback.rs:958-968` `hint_does_not_mark_the_yesterday_week_row`; `tests/week_target_cli.rs:184`; `tests/start_hint_cli.rs:~170-178` `hint_does_not_mark_week_row_or_footer`.

## Design

### Step 1. `src/lookback.rs`: shared decision

Add `use crate::anomalies;` to the imports (line 10-14 block).

```rust
/// The overnight stint (SPEC overnight-visibility §3): yesterday's one open
/// start, not joined to today by the splice, with no refused join naming
/// yesterday on today's page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OvernightStint {
    /// Yesterday.
    pub date: NaiveDate,
    /// Local time of the open start.
    pub start: NaiveTime,
    /// Whole minutes from the start to `now_utc`.
    pub minutes_so_far: i64,
}

/// `buckets` must hold yesterday and today (`status` and `week` loaders both do).
pub fn overnight_stint(
    buckets: &PunchBuckets,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
) -> Option<OvernightStint> {
    let date = today.pred_opt()?;
    let trio = buckets.trio(date);
    if trio.day.is_empty() {
        return None;
    }
    let ctx = anomalies::classify_context(&trio, now_utc);
    let [only] = ctx.day.open.as_slice() else {
        return None;
    };
    if stint::is_refused_join(&ctx.day_raw, &ctx.next_raw, trio.next) {
        return None;
    }
    Some(OvernightStint {
        date,
        start: render::local_time(only.start.at_utc),
        minutes_so_far: only.minutes_so_far,
    })
}
```

Cognitive complexity is about 5.

Refactor the scan so the hint filter and the decision share one buckets load. Replace `lookback` (195-211) and `today_page_lines` (255-266):

```rust
fn load_buckets(conn: &Connection, today: NaiveDate) -> anyhow::Result<PunchBuckets> {
    let from = today - ChronoDuration::days(MAX_AGE_DAYS + 1);
    let punches = storage::punches_in_range(conn, from, today)?;
    Ok(PunchBuckets::from_punches(&punches))
}

fn scan(buckets: &PunchBuckets, today: NaiveDate, now_utc: DateTime<Utc>) -> Vec<LookbackEntry> {
    let today_has_punches = !buckets.day(today).is_empty();
    (MIN_AGE_DAYS..=MAX_AGE_DAYS)
        .rev()
        .filter_map(|age| {
            let date = today - ChronoDuration::days(age);
            entry_for_date(date, age, buckets, today_has_punches, now_utc)
        })
        .collect()
}

pub fn lookback(conn: &Connection, today: NaiveDate, now_utc: DateTime<Utc>)
    -> anyhow::Result<Vec<LookbackEntry>> {
    Ok(scan(&load_buckets(conn, today)?, today, now_utc))
}

/// Finished lookback lines for today's page, oldest first, with the age-1
/// hint last. The hint prints exactly when `overnight_stint` finds a stint.
pub fn today_page_lines(conn: &Connection, today: NaiveDate, now_utc: DateTime<Utc>)
    -> anyhow::Result<Vec<String>> {
    let buckets = load_buckets(conn, today)?;
    let overnight = overnight_stint(&buckets, today, now_utc).is_some();
    Ok(scan(&buckets, today, now_utc)
        .iter()
        .filter(|e| overnight || !e.finding.is_yesterday_hint())
        .flat_map(LookbackEntry::detail_lines)
        .collect())
}
```

`today_page_lines` loses its `bool` parameter. Update its doc comment accordingly.

Mechanical test edits in `lookback.rs` caused by the signature change (all in the `tests` module; no behavior assertion changes except where noted):

- Drop the 4th argument at every `today_page_lines(..., false)` call.
- `today_page_lines_equals_flattened_entries_for_either_flag`: remove the flag loop, rename to `today_page_lines_equals_flattened_entries`, keep the single comparison.
- `refused_join_flag_drops_only_the_hint`: rename `refused_join_drops_only_the_hint`. Keep the data (start yesterday, two orphan ends today). Assert the lines are empty. Delete the `false` branch that expected the hint, since the decision now derives suppression itself. Add one assertion that `lookback()` still returns the age-1 entry, which is what `mlm start` relies on.
- `refused_join_flag_keeps_the_yesterday_multi_line`: rename `yesterday_multi_line_prints_without_a_hint`, drop the flag argument.

### Step 2. `src/render.rs`: footer strings

Add near `WEEK_ANOMALY_FOOTER` (line 134), and extend the import on line 22 to `use crate::date::{WeekId, format_date_with_weekday, format_weekday_full};`:

```rust
/// Footer when a `mlm week` row shows `[i]` (contract 3).
pub const WEEK_OVERNIGHT_FOOTER: &str = "[i] open stint: run mlm status to stop it";

/// Monday footer for the current week's view (contract 4). `date` is the
/// overnight stint's date, in the existing weekday-date form.
pub fn week_overnight_monday_footer(date: NaiveDate) -> String {
    format!(
        "[i] open stint on {} (last week): run mlm status to stop it",
        format_date_with_weekday(date)
    )
}
```

Tests in `render.rs`: exact text for both (`week_overnight_monday_footer(2026-10-04)` equals `"[i] open stint on Sun 2026-10-04 (last week): run mlm status to stop it"`), and add both to the ASCII assertions near line 647 (`assert!(WEEK_OVERNIGHT_FOOTER.is_ascii()); assert!(week_overnight_monday_footer(d(2026,10,4)).is_ascii());`).

### Step 3. `src/status.rs`: field, resolve, render

New type, next to `StintLine` (after line 139):

```rust
/// The overnight stint's line on today's page (SPEC overnight-visibility §4).
/// Kept apart from `stints`, which stays sorted and day-total-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OvernightLine {
    pub start: NaiveTime,
    pub duration_minutes: i64,
}
```

`StatusView` gains, after `stints` (line 168):

```rust
    /// Today's page only: yesterday's still-open start. Printed before
    /// `stints`. Not part of any total.
    pub overnight: Option<OvernightLine>,
```

All `StatusView` literals (`status.rs` 466, 555, 995, 1125, 1218, 1236, 1270) gain `overnight: None,`; production site 466 gets `overnight,`.

Render. Replace the stint block of `render` (279-282) with `push_stint_section(&mut lines, view);` and add:

```rust
fn overnight_line(o: &OvernightLine) -> String {
    format!(
        "  {:<11}  ({} so far, started yesterday, not counted until stopped)",
        format!("{}-now", o.start.format("%H:%M")),
        format_minutes(o.duration_minutes)
    )
}

fn push_stint_section(lines: &mut Vec<String>, view: &StatusView) {
    if view.overnight.is_none() && view.stints.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.extend(view.overnight.iter().map(overnight_line));
    lines.extend(view.stints.iter().map(stint_line));
}
```

`"  22:00-now"` is 9 chars, padded to 11, then two spaces, giving `  22:00-now    (15h 30m so far, ...` with four spaces, matching spec section 4 and contract 1. Duration uses `format_minutes` (`time.rs:109`), which prints hours without a day roll-over, so 41h 30m renders `41h 30m`.

Resolve. Add a helper and call it so `resolve` gains no branching:

```rust
fn overnight_for(
    is_today: bool,
    buckets: &history::PunchBuckets,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
) -> Option<OvernightLine> {
    if !is_today {
        return None;
    }
    lookback::overnight_stint(buckets, today, now_utc).map(|o| OvernightLine {
        start: o.start,
        duration_minutes: o.minutes_so_far,
    })
}
```

In `resolve`: change lines 444-453 to `lookback::today_page_lines(conn, today, now_utc)?` (drop the flag; `facts` is still used by `compose_anomaly_lines`). Add `let overnight = overnight_for(is_today, &history.buckets, today, now_utc);` before the `Ok(StatusView {`. No new storage read: `history.buckets` is already loaded (F7). Day total, `eod`, `has_open_stint`, `open_stint_age` and `week_line` are untouched. `status DATE` for a non-today date gets `None` by construction.

### Step 4. `src/week_view.rs`: row flag, Monday field, footers

Types:

```rust
// WeekRow, after has_anomaly
    /// `[i]` marker: this row's date is the overnight stint's date. Never
    /// feeds `has_any`, the `[!]` footer, the carry marker or carry accounting.
    pub has_overnight: bool,

// WeekView, after carry_marker
    /// Monday case: the overnight stint's date when the shown week contains
    /// today and that date is in the previous week. Names Sunday in the
    /// `[i]` footer slot because no row of this week is marked.
    pub overnight_last_week: Option<NaiveDate>,
```

Fixtures: `row()` at 262 gets `has_overnight: false`; the four `WeekView` test literals (286, 312, 416, 438) get `overnight_last_week: None`.

Add `use crate::lookback;` to the imports (line 23-29 block).

Row building. `build_rows_from` gains a fifth parameter `overnight: Option<NaiveDate>` (the overnight stint's date) and sets `has_overnight: Some(date) == overnight`. The test-only `build_rows` keeps its signature and computes it:

```rust
let buckets = PunchBuckets::from_punches(punches);
let overnight = lookback::overnight_stint(&buckets, today, now_utc).map(|o| o.date);
build_rows_from(dates, &buckets, today, now_utc, overnight)
```

`build_week_view`:

```rust
let overnight = lookback::overnight_stint(&history.buckets, today, now_utc).map(|o| o.date);
let rows = build_rows_from(week.dates(), &history.buckets, today, now_utc, overnight);
...
overnight_last_week: monday_overnight(week, today, overnight),
```

```rust
/// The overnight date when the shown week holds today but not that date.
fn monday_overnight(week: WeekId, today: NaiveDate, overnight: Option<NaiveDate>) -> Option<NaiveDate> {
    let (start, end) = week.span();
    let holds = |d: NaiveDate| (start..=end).contains(&d);
    overnight.filter(|&d| holds(today) && !holds(d))
}
```

This yields the spec cases: the current week on a Monday sets the field; the previous week (by id) has the Sunday row marked and the field stays `None` (week does not contain today); other past and future weeks mark nothing and set nothing.

Rendering.

```rust
/// `  [!]`, `  [i]` or `  [!] [i]`, or nothing.
fn row_markers(row: &WeekRow) -> &'static str {
    match (row.has_anomaly, row.has_overnight) {
        (true, true) => "  [!] [i]",
        (true, false) => "  [!]",
        (false, true) => "  [i]",
        (false, false) => "",
    }
}
```

In `push_rows` replace the `if row.has_anomaly { ... }` block (108-110) with `out.push_str(row_markers(row));`. Existing `{ongoing}  [!]` bytes are unchanged.

Rewrite `push_notices` (119-132) around a line list so the blank-line rule is one condition:

```rust
fn push_notices(out: &mut String, view: &WeekView) {
    let lines = notice_lines(view);
    for line in &lines {
        out.push_str(line);
        out.push('\n');
    }
    if !lines.is_empty() {
        out.push('\n');
    }
}

/// Footer order: `[!]` footer, `[i]` footer or Monday line, carry marker line.
fn notice_lines(view: &WeekView) -> Vec<String> {
    let mut lines = Vec::new();
    if view.rows.iter().any(|r| r.has_anomaly) {
        lines.push(render::WEEK_ANOMALY_FOOTER.to_string());
    }
    lines.extend(overnight_footer(view));
    lines.extend(view.carry_marker.as_ref().map(render::carry_marker_line));
    lines
}

/// The plain footer when any row shows `[i]`, else the Monday line.
fn overnight_footer(view: &WeekView) -> Option<String> {
    if view.rows.iter().any(|r| r.has_overnight) {
        return Some(render::WEEK_OVERNIGHT_FOOTER.to_string());
    }
    view.overnight_last_week.map(render::week_overnight_monday_footer)
}
```

The `[!]` footer condition keeps reading only `has_anomaly`, the Monday line and plain footer depend only on the trigger (not on `[!]`), and `push_totals` is untouched, so `Carry-in ... [!]` still keys on `carry_marker`. If a hand-built view sets both a marked row and the Monday field, the plain footer wins and only one line prints (the two never co-occur from `build_week_view`).

### Step 5. `src/cli.rs`: glossary

Replace the tail of the `[!], [i]` entry (lines 46-48). Contract 6 text, wrapped at the glossary's width, sentence unchanged:

```
      or overlapping stints. In `mlm week` it marks the date's row. `[i]`
      is a hint, not an anomaly, such as a start still open from
      yesterday. It marks yesterday's row in `mlm week` until that start
      is stopped.";
```

The sentence reads `It marks yesterday's row in `mlm week` until that start is stopped.` once the wrap newline is read as a space. `tests/week_target_cli.rs:184` is changed to compare against whitespace-normalised help text (see test plan, AC 14). The existing assertions at `cli.rs:769-770` (ASCII, no trailing newline) still hold.

## Test plan

Rules: write each failing test first, run `cargo fmt`, `cargo test`, `cargo clippy --all-targets -- -D warnings`. Unit tests use the in-memory DB and `Local` timezone helpers already in each module. CLI tests use a temp `MLM_DB_PATH` and `TZ=UTC`.

Shared fixtures for new unit tests (`status.rs` and `week_view.rs` tests, reuse existing `today()` = Thu 2026-02-12 and `d`, `t`, `test_db`):

- `now_1330()` = `Local.from_local_datetime(&today().and_hms_opt(13,30,0).unwrap()).unwrap()`. A 22:00 start yesterday gives the spec's `15h 30m`.
- `seed(conn, kind, date, h, m)` = `storage::insert_punch(conn, kind, date, t(h,m), &Local)`.

### AC 1. Status, no punches today, line under `Day total: 00h 00m`, no `est. EOD` (spec section 4, section 7 last-but-few bullets)

`src/status.rs` test `overnight_line_with_no_punches_today`: start yesterday 22:00, `resolve(None, now_1330(), &conn)`, `render`.

- `view.overnight == Some(OvernightLine { start: t(22,0), duration_minutes: 930 })`
- output contains `"Day total:     00h 00m\n"` and the exact line `"  22:00-now    (15h 30m so far, started yesterday, not counted until stopped)"`
- output does not contain `est. EOD`, `(incl. ongoing)`
- the line follows the hint block, and is the last line (no notes)

### AC 2. Same with punches today, line before own stints, figures identical (spec section 4 bullet 4)

`overnight_line_precedes_todays_stints_and_figures_unchanged`: today 09:00-12:00 closed. Build the same DB without yesterday's start. Assert:

- the rendered lines place the overnight line immediately before `  09:00-12:00` (index compare)
- for the two views, `day_total_minutes`, `has_open_stint`, `eod`, `week_line`, `stints`, `open_stint_age` are equal
- `week_view::build_week_view` for the two DBs gives equal `worked_minutes`, `fulfillment_minutes`, `carry_in_minutes`, `headline` (covers Worked and Fulfillment)

### AC 3. Today also has its own open stint (spec section 7, "two -now lines")

`overnight_line_and_own_open_stint_both_print_overnight_first`: yesterday 22:00 start, today 09:00 start. Assert two lines contain `-now`, the first is `  22:00-now    (15h 30m so far, ...)`, the second is `  09:00-now    (04h 30m, ongoing)`, `day_total_minutes == 270` (only the own stint counts), `has_open_stint`, and `eod` is `Some`.

### AC 4. No line when joined, refused, two opens, age 2 (spec section 3, section 8 §4 bullet)

`src/lookback.rs` test `overnight_stint_trigger_table` over fixtures (each a fresh DB, `now()` 18:00 on 2026-02-12, helper `buckets_of(&conn)` = `PunchBuckets::from_punches(&storage::punches_in_range(conn, today()-15d, today())?)`):

| Fixture | Expected |
|---|---|
| start yesterday 18:02 | `Some`, `start == 18:02`, `date == yesterday` |
| start yesterday 23:00, end today 00:10 (splice) | `None` |
| start yesterday 18:02, ends today 00:10 and 00:20 (refused) | `None` |
| starts yesterday 09:00 and 10:00 | `None` |
| start 2 days ago | `None` |
| start 2 days ago and start yesterday | `Some` for yesterday (the older one is its own anomaly) |
| no punches | `None` |

Plus a `status::resolve` check for the same table at the view level: `view.overnight.is_none()` for the four none cases, and for age 2 `view.anomaly_lines` holds `unclosed start` and `(2 days ago)`.

### AC 5. Duration standard formatter, including over 24h (spec section 4 bullet 2, section 8 also-list)

`overnight_line_formats_over_24h`: start yesterday 00:30, `now_1330()`. Yesterday 00:30 to today 13:30 is 37h, so assert line contains `(37h 00m so far,`. Also unit-level in `status.rs`: render a hand-built `StatusView` with `overnight: Some(OvernightLine { start: t(0,30), duration_minutes: 2490 })` and assert `(41h 30m so far,`. A same-minute case (`duration_minutes: 0`) prints `(00h 00m so far,`.

### AC 6. `status DATE` other than today is byte-identical, including `status -1` (spec section 4 last bullet)

- `status.rs` test `overnight_absent_for_other_dates`: with a start yesterday, `resolve(Some("-1"))` and `resolve(Some("2026-02-11"))` have `overnight == None`. The rendered `-1` page contains no `so far, started yesterday` and still holds `ongoing - duration as of right now, not a running total`.
- `tests/overnight_visibility_cli.rs` `status_for_other_dates_has_no_stint_line` repeats it end to end for `-1` and the explicit date; asserts `!text.contains("started yesterday")`.
- Existing `tests/start_hint_cli.rs::hint_never_shows_on_status_for_another_date` stays unedited and must pass.

### AC 7. Hint and line appear and disappear together, one end-to-end `status` test (spec section 8 last bullet)

`tests/overnight_visibility_cli.rs` `hint_and_line_appear_together`: a table of seed closures run through the real binary (`TZ=UTC`, dates from `Utc::now()` like `start_hint_cli.rs`):

- lone start yesterday 00:00 (no `now`-dependent duration assert; only presence)
- same plus own punch today 00:00 start
- joined (start yesterday 23:00, end today 00:10, as in `stop_today_joins_and_clears_the_hint`)
- refused join (start yesterday 18:02, ends today 00:10 and 00:20, as in `refused_join_suppresses_the_hint`)
- two starts yesterday
- start 2 days ago

For each: `hint = text.lines().any(|l| l.starts_with("[i] start on"))`, `line = text.lines().any(|l| l.contains("started yesterday, not counted until stopped"))`, assert `hint == line` and assert the expected value from the table. Unit mirror in `status.rs`: for each fixture, `view.anomaly_lines.iter().any(|l| l.starts_with("[i] start on")) == view.overnight.is_some()`.

Race note: these tests read the real clock. A run straddling UTC midnight can flip a date; existing hint tests carry the same exposure.

### AC 8. `mlm week` marks yesterday's row, footer once, no other row (spec section 5 bullets 1, 3; section 8)

- `week_view.rs` unit `build_week_view_marks_yesterday_row`: start Wed 2026-02-11 18:02 with `fixed_now()` (Thu 2026-02-12 18:00), `build_week_view(&conn, wk(2026, 7), fixed_now())`. Assert `view.rows[2].has_overnight`, no other row has it, `overnight_last_week == None`, all `has_anomaly` false. Render and assert `"  Wed 2026-02-11   00h 00m  [i]\n"` appears, `matches(WEEK_OVERNIGHT_FOOTER).count() == 1`, no `needs a look`.
- Same for a past week by id: `now` Mon 2026-02-16 09:00 (`Local.with_ymd_and_hms(2026,2,16,9,0,0)`), start Sun 2026-02-15 22:00, `build_week_view(wk(2026,7))`: row 6 marked, plain footer printed, `overnight_last_week == None`.
- `lookback.rs` rewrite of `hint_does_not_mark_the_yesterday_week_row` (F11) as `hint_marks_the_yesterday_week_row`: same fixture; assert the row whose date is yesterday has `has_overnight`, every other row does not, and every `has_anomaly` is false.
- e2e `tests/overnight_visibility_cli.rs` `week_by_id_marks_yesterday_row`: seed start yesterday, run `week <WeekId::from_date(yesterday)>`; the line starting with `  {Wdy} {yesterday}` ends with `  [i]`; exactly one `  `-prefixed row line contains `[i]`; exactly one line equals `[i] open stint: run mlm status to stop it`. Also `week` with no id: if `today` is a Monday expect the Monday footer and no marked row, else the marked row (branch on `today().weekday()` from `chrono::Datelike`).
- Existing `tests/start_hint_cli.rs::hint_does_not_mark_week_row_or_footer` is rewritten to `hint_marks_week_row_and_prints_footer`: no `[!]` or `needs a look`; row ends with `  [i]`; footer once.

### AC 9. Row mark follows the same trigger (spec section 3; section 8)

`week_view.rs` unit `row_mark_follows_the_trigger`, `fixed_now()`, table over the same fixtures as AC 4 (joined, refused, two opens, age 2): `rows.iter().all(|r| !r.has_overnight)`. For age 2 (Tue 2026-02-10), assert `rows[1].has_anomaly` (the `[!]` stale marker, existing B10 behavior) and the rendered row ends with `  [!]`, not `[i]`.

### AC 10. Monday footer needs Sunday's facts from the loader's buckets (changeset plan, review F5)

`week_view.rs` unit `monday_current_week_uses_loaded_buckets`: `now` Mon 2026-02-16 09:00, start Sun 2026-02-15 22:00, `build_week_view(&conn, wk(2026, 8), now)` (current week). The only reads are `load_history`'s. Assert `overnight_last_week == Some(d(2026,2,15))` and no row marked. Seed with `&Local` so local equals the seeded date. No new storage read is introduced, and `build_week_view`'s signature is unchanged, which the compile itself pins.

### AC 11. Shared row, both markers, both footers in order (spec section 5 bullet 2, bullet 4)

`render_week` unit `shared_row_prints_both_markers_and_footers_in_order`: `example_b()` with `rows[2].has_anomaly = true; rows[2].has_overnight = true; carry_marker = marker(2, wk(2026, 38))`. Byte-exact golden:

```
Week 2026-06 (2026-02-02 - 2026-02-08)

Total behind: 03h 10m

  Mon 2026-02-02   07h 30m
  Tue 2026-02-03   08h 00m
  Wed 2026-02-04   07h 45m  [!] [i]
  Thu 2026-02-05   08h 10m
  Fri 2026-02-06   05h 25m
  Sat 2026-02-07   00h 00m
  Sun 2026-02-08   00h 00m

[!] needs a look: run mlm status DATE for details and fixes
[i] open stint: run mlm status to stop it
[!] carry-in includes anomalies from 2 earlier weeks, oldest 2026-38 (see mlm week 2026-38)

Carry-in:      00h 00m  [!]
Worked:        36h 50m
Fulfillment:   36h 50m
Target:        40h 00m
```

Also `ongoing_anomaly_and_overnight_marker_order`: `example_a()` with row 3 `has_anomaly` and `has_overnight` contains `07h 25m (incl. ongoing)  [!] [i]`.

### AC 12. Monday: current-week footer, last week's Sunday marked, Sunday `[!]` coexistence, other weeks nothing (spec section 5 bullets 5-7)

`week_view.rs` unit, `now` Mon 2026-02-16 09:00, start Sun 2026-02-15 22:00:

- week 8 (current): `overnight_last_week == Some(2026-02-15)`, no row `has_overnight`; rendered text contains `[i] open stint on Sun 2026-02-15 (last week): run mlm status to stop it` once and not the plain footer.
- week 7: row 6 `has_overnight`, `overnight_last_week == None`, plain footer once, no Monday line.
- week 6 and week 9: no marks, no `[i]` line at all, `output == output without the start` (compare against a DB without the punch).
- Sunday also anomalous: add a second Sunday punch so the row also gets `[!]` (an orphan end earlier that day at 08:00 with no start before it, so Sunday has an orphaned end plus the one open start at 22:00). Week 8 view still prints the Monday line; week 7 row ends `  [!] [i]`.
- Hand-built renderer golden: `example_a()` with `overnight_last_week = Some(d(2026,2,15))` and no marked row, byte-exact: footer slot holds the Monday line followed by one blank line; contract-4 text with weekday-date from the stint.

### AC 13. `has_any`, `[!]` footer, carry marker and carry accounting ignore `[i]`; `[i]`-only row prints no `[!]` footer (spec section 2 last bullet, section 8)

- `render_week` unit `overnight_only_row_prints_no_anomaly_footer`: `example_b()` with `rows[2].has_overnight = true`. Output does not contain `WEEK_ANOMALY_FOOTER`, does not contain `carry-in includes`, `Carry-in:      00h 00m\n` has no `[!]`, line count is 18 (16 + footer + blank).
- `build_week_view` unit `overnight_does_not_change_any_figure`: same DB with and without yesterday's open start, `fixed_now()`. Assert equal `rows[i].minutes` and `is_ongoing`, `carry_in_minutes`, `worked_minutes`, `fulfillment_minutes`, `target_minutes`, `headline`, `carry_marker`, and every `has_anomaly`.

### AC 14. Blank line before Carry-in exactly when any footer or carry marker line printed (spec section 5 bullet 4)

`render_week` unit `blank_line_follows_footer_block_only_when_a_line_printed`: table over `(has_anomaly, has_overnight, overnight_last_week, carry_marker)` combinations (all 2x2x2x2 minus the contradictory both-`[i]`-forms case, which keeps the plain-wins rule). For each, count lines printed by `notice_lines`; assert `out.matches('\n').count() == 16 + if n > 0 { n + 1 } else { 0 }`.

### AC 15. Glossary golden (spec section 6, contract 6)

`tests/week_target_cli.rs` `long_help_carries_glossary` (line 184): replace the `ends_with` assertion with

```rust
let flat = s.split_whitespace().collect::<Vec<_>>().join(" ");
assert!(
    flat.ends_with("It marks yesterday's row in `mlm week` until that start is stopped."),
    "{args:?}"
);
```

Also assert `!flat.contains("It never marks a row.")`. A `cli.rs` unit test `glossary_i_entry_says_it_marks_yesterdays_row` checks the same on `GLOSSARY` and that `GLOSSARY.ends_with("stopped.")`.

### AC 16. No overnight stint: output byte-identical (spec section 1)

Existing goldens (`a1_`, `a3_`, `t11_`, `t7_`, `t5_`, week/status CLI tests, `ascii_sweep`) pass unedited apart from the added `has_overnight: false`, `overnight_last_week: None` and `overnight: None` fixture fields. Add `week_view.rs` unit `no_overnight_output_matches_golden`: `render_week(&example_a())` equals `EXAMPLE_A_GOLDEN` after the fields are added (already covered by `a1`; keep a note, no new test needed). Add a `status.rs` unit that a view with `overnight: None` and empty `stints` still renders exactly four lines (existing `t5_`).

### AC 17. Cognitive complexity at or below 15 (changeset plan)

No test; verify with `cargo clippy --all-targets -- -W clippy::cognitive_complexity` and read the output for any hit (the hook runs the same lint). The design splits `push_notices` into `notice_lines` and `overnight_footer`, `render` into `push_stint_section`, row markers into `row_markers`, and `resolve` uses `overnight_for`. Test functions with loops over tables stay flat (one closure per fixture, no nested conditionals).

### Cross-cutting checks

- ASCII: the new CLI test asserts `text.is_ascii()` for a status page and a week table that carry the new lines. `tests/ascii_sweep.rs` is not edited.
- DB isolation: every new test uses `Connection::open_in_memory()` or a `TempDir` with `MLM_DB_PATH`.

## Order of work

1. `render.rs` constants plus tests.
2. `lookback.rs`: `overnight_stint`, scan refactor, `today_page_lines` signature, adapt existing tests, AC 4 table.
3. `status.rs`: type, field, fixtures, render, resolve, AC 1-7 tests.
4. `week_view.rs`: fields, fixtures, rows, notices, AC 8-14 tests.
5. `cli.rs` glossary and `tests/week_target_cli.rs`.
6. `tests/start_hint_cli.rs` rewrite and `tests/overnight_visibility_cli.rs`.
7. `cargo fmt`, `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings`, then the complexity lint run above.

## Risks, ambiguities, disagreements

- R1. The changeset plan says `today_page_lines` takes no flag. That forces edits to about ten existing `lookback.rs` tests (argument removal, one flag-loop test collapsed, one test that asserted the hint appears when the flag was `false` on refused-join data). Those assertions encoded the flag, not behavior, so I treat them as in-scope mechanical edits. The changeset plan's list of allowed test rewrites does not mention them. Flagging it so the coordinator is not surprised.
- R2. Trigger agreement. `overnight_stint` reads the refused join from yesterday's side (`day_raw`, `next_raw`, today's punches) while status's own `[!]` refused-join line reads it from today's side. F3 shows the inputs are identical. The AC 7 tests (unit and e2e) pin it. If they diverge, stop and report rather than patch quietly.
- R3. `commands::start` still warns on a refused join because it calls `lookback()`, whose output I leave unfiltered. Only `today_page_lines` applies the suppression.
- R4. Clock. CLI tests run on the real clock, so exact durations, the Monday e2e case and the near-midnight window are unit-tested with injected `now`. The e2e `week` test branches on the real weekday. A run across UTC midnight can flip dates; existing hint tests share that exposure.
- R5. Glossary wrap. Contract 6 pins the sentence, not the line breaks. I break after `start` (72 columns). Task 2 must quote the sentence, not the wrapped form. The test normalises whitespace. If Task 2 chooses to compare raw bytes of `--help`, it must use the same wrap. Coordinator check at merge.
- R6. Plain-footer-wins rule when a hand-built `WeekView` sets both a marked row and `overnight_last_week`. `build_week_view` never produces that pair, and the plan says they "never print together", so the choice only affects hand-built views.
- R7. Spec section 8 "`[!]` precedence on a shared row" is stale per the changeset plan. I follow section 5 (`  [!] [i]`).
- R8. `status.rs`'s stint list comment says "already sorted by start time ascending". It stays true because the overnight line lives in its own field.
- R9. Over-24h arithmetic in AC 5: `yesterday 00:30 -> today 13:30` is 37h 00m (`2220` minutes). The `41h 30m` figure applies to the hand-built render case (`2490` minutes). They test different layers, so do not mix the numbers.
- R10. Coverage ratchet: the tests above exercise every new branch, so the baseline should hold or rise.
