# Task plan: possible-issues-triage, T3 Yesterday hint, lookback window and `mlm start` warning

> **Archived — historical planning record.** Not authoritative. Current spec: `docs/dev/SPEC.md`. Design/process history: `docs/dev/NOTES.md`.

Changeset plan: `docs/dev/plans/possible-issues-triage-plan.md` (T3 section, contracts 3, 4, 7, 8). Spec: `docs/dev/specs/2026-10-03-possible-issues-triage.md` §3.1, §3.2.2, §7.

## What I read

- `docs/dev/plans/possible-issues-triage-plan.md`, full.
- `docs/dev/specs/2026-10-03-possible-issues-triage.md`, full.
- Sibling task plans, full: T1 (`...-task-1-foundation.md`, source of every seam below), T5, T6. T4 exists and I read its scope and ownership tables, since it shares `status` page composition. T2's plan did not exist. I used only the changeset plan's contract 7 for it.
- `src/commands.rs`, full production code (1-355) and the stale-warning tests (1741-1999). `src/status.rs` 372-450 (`group_punches_by_date`, `lookback_finding_for_date`, `lookback`), 640-700 (`resolve`), and the two direct lookback tests (2689-2750). `src/render.rs` 286-375 (`LookbackFinding`, `LookbackKind`, both renderers) and the `t33b` test (1015-1085). `src/stint.rs` 14-56 and 255-404 (`classify_at`, `splice_candidate`, `is_refused_join`). `src/storage.rs` (`punches_in_range`, `to_utc_and_date`, `insert_punch`). `src/date.rs` 168-177 (`format_date`, `format_date_with_weekday`). `src/lib.rs`. `tests/status_cli.rs` 1-60 and 195-250, `tests/e6_db_failure.rs` 1-80.
- `clippy.toml` (cognitive-complexity-threshold 15), `.githooks/pre-commit` (fmt, complexity, coverage ratchet against `coverage-baseline.json`), `AGENTS.md`.
- Nothing is built. Line numbers below are at baseline `cccfb8f`. After T1 merges the same code sits in `src/lookback.rs` (T1 plan step 3). Names are the T1 published seams.

## Scope

Files this task owns:

| File | Action |
|---|---|
| `src/lookback.rs` | extend (created by T1, mine after T1 merges) |
| `src/commands.rs` | repoint to the lookback module, new finding shape, update its tests |
| `tests/start_hint_cli.rs` | new |

Files this task must not touch, and owner:

| File | Owner |
|---|---|
| `src/status.rs`, `src/week_view.rs`, `src/render.rs`, `src/stint.rs`, `src/lib.rs`, `src/week.rs`, `src/storage.rs`, `src/date.rs`, `src/history.rs`, `tests/status_cli.rs`, `tests/e6_db_failure.rs`, `tests/ascii_sweep.rs`, `tests/week_read_errors.rs` | T1 |
| `src/anomalies.rs`, `tests/overlap_cli.rs` | T2 |
| `src/carry_scan.rs`, `tests/carry_marker_cli.rs` | T4 |
| `src/cli.rs`, `src/main.rs`, `src/week_target.rs`, `tests/week_target_cli.rs` | T5 |
| `docs/dev/SPEC.md`, `README.md`, `AGENTS.md` | T6 |
| `docs/dev/NOTES.md`, the spec, `coverage-baseline.json` | coordinator |

T1's `src/status.rs` already calls `lookback::today_page_lines(conn, today, now_utc, facts.refused_join_with_previous.is_some())` and re-exports `status::{LookbackFinding, LookbackKind, lookback}`. Both stay valid with my changes (the `pub use` compiles whatever the item's type becomes). So this task needs no edit in `status.rs`.

## Dependencies

| Dependency | State | What I use | Stub today | Swap at integration |
|---|---|---|---|---|
| T1 `render::{local_time, next_punch_after, stop_time_hint}` | not landed | clean-pair helper and the plain phrase | none needed, the worktree opens from merged T1. Before that, a private `mod shim` in `src/lookback.rs` with the three published signatures (T1 plan, seam 5) | delete `mod shim`, `use crate::render::{...}` |
| T1 `history::PunchBuckets::{from_punches, day, trio}` | not landed | bucket the one ranged read, take previous/own/next slices | same shim idea, a `BTreeMap<NaiveDate, Vec<Punch>>` wrapper with `trio` returning three slices | `use crate::history::PunchBuckets` |
| T1 `lookback::{LookbackFinding, LookbackKind, lookback, today_page_lines}` moved code | not landed | baseline I extend | none | n/a |
| T1 `anomalies::date_facts` | not landed | only in two round-trip assertions (`!date_facts(..).has_any()`) | omit those assertions | add them back |
| T2 overlap anomaly | not landed, soft | the round-trip assertions become meaningful once T2 merges. Before, `has_any()` is trivially false | none | no code change. The test already calls `date_facts`. A T2-merged rebase only strengthens it |
| T4 carry marker | not landed, no dependency | none. The marker line sits after my lines through T1's composition | none | none |

Behavior frozen by T1, called as published, never edited:

- `render::next_punch_after(punches: &[Punch], start: &Punch) -> Option<NaiveTime>` (first punch of any kind on `start.date`, strictly later by `at_utc`, ties by kind then id).
- `render::stop_time_hint(start: NaiveTime, next: Option<NaiveTime>) -> String`, returns the parentheses: `(a time after 18:02)` or `(a time between 18:02 and 19:00)`.
- `render::local_time(DateTime<Utc>) -> NaiveTime`.
- `history::PunchBuckets` with `from_punches(&[Punch]) -> Self`, `day(NaiveDate) -> &[Punch]`, `trio(NaiveDate) -> DateBuckets<'_>` where `DateBuckets { prev, day, next }` are `&[Punch]`.
- `lookback::today_page_lines(conn, today, now_utc, refused_join_names_yesterday: bool) -> anyhow::Result<Vec<String>>`. Signature unchanged by this task.

## Facts gathered (current code)

- `src/status.rs:372-380` `group_punches_by_date`. `:385-412` `lookback_finding_for_date`: empty own bucket gives `None`, then `stint::classify_at(prev, cur, next, now)`, then `open.len()` 0 gives `None`, 1 gives `SingleOpenStart { time, punch_id }`, n gives `MultipleOpenStints { count }`. `:430-451` `lookback`: reads `[today-15, today-1]` with one `punches_in_range`, walks `(2..=14).rev()`, so the output is oldest first.
- `src/status.rs:655-660`: `resolve` appends `finding.detail_lines()` per finding for today only. T1 replaces that block with `lookback::today_page_lines`.
- `src/render.rs:291-374`: `LookbackFinding { date, age_days, kind }`, `LookbackKind::{SingleOpenStart { time, punch_id }, MultipleOpenStints { count }}`. Both renderers carry `debug_assert!((2..=14).contains(&age_days))`. Line text:
  - `detail_lines`, single: `[!] unclosed start on {wdy} at {HH:MM} ({age} days ago)`, `    fix: mlm stop HH:MM --date {D}  (a time after {HH:MM})`, `     or: mlm delete punch --date {D}  (lists punches; rerun with the start's number)`.
  - `detail_lines`, multi: `[!] {count} open stints on {wdy} ({age} days ago)`, `    fix: see mlm status {D}`.
  - `stderr_line`, single: `warning: unclosed start on ...` then the same `fix:` and `or:` lines joined by `\n`. Multi: `warning: {count} open stints on {wdy} ({age} days ago); see mlm status {D}`.
- `src/render.rs:1022-1082` test `t33b` builds `LookbackFinding { date, age_days, kind: SingleOpenStart { time, punch_id } }` literals. T1 keeps `t33b` in `render.rs` (T1 plan step 2). It is T1-owned and frozen, so it fixes the shape of `LookbackFinding` and `LookbackKind` (see Design D1).
- `src/commands.rs:10-18` imports `render` and `status`. `:25-35` `start` calls `warn_stale_starts(conn, today, now_utc, written_id, &mut stderr)`. `:49-60` `warn_stale_starts` calls `status::lookback` and swallows errors. `:72-85` `emit_warnings(findings: Vec<render::LookbackFinding>, written_punch_id, out)` skips a `SingleOpenStart` whose `punch_id == written_punch_id`, otherwise writes `finding.stderr_line()`. `render::` and `status::` are used in non-test code only at lines 56, 73, 78. Tests use full paths (`crate::render::...`, `crate::status::resolve`).
- `src/commands.rs:1741-1999` tests: helper `finding(date, age, kind) -> crate::render::LookbackFinding` (1755-1763), six `emit_warnings` tests, and `warn_stale_starts` tests built on `fixed_now()` (2026-02-12 14:23, a Thursday) and `today()`.
- `src/status.rs:2724-2750` test `lookback_excludes_age_one_open_start` pins the behavior this task inverts. T1 moves it to `lookback.rs`.
- Punches are stored at minute granularity (`storage.rs:200-212` zeroes seconds), so "same minute as S" equals "same instant as S".
- `stint::classify_at(prev, day, next, now)` (`stint.rs:264`) already splices: with exactly one open start on `day` and exactly one orphaned end that is `next`'s first punch, the start becomes a completed stint. Passing today's punches as `next` for the age-1 date is therefore all it takes to answer "does a `stop` typed today join yesterday's start".
- The binary reads the real clock and has no injection. Integration tests compute dates from `Utc::now()` with the child's `TZ=UTC`, as `tests/status_cli.rs:195-250` does.

## Design

### D1. Finding shape and where the extra facts live

The hint and the clean-pair window need two facts a `LookbackFinding` does not carry: `N` (the next punch time on the finding's date) and whether today has punches (the `or:` line). Adding a field to `LookbackFinding` or `LookbackKind::SingleOpenStart` would break the struct literals in T1-owned `render.rs::t33b`, which this task cannot edit. So the two types keep their published shape and a wrapper carries the facts.

```rust
/// Facts a lookback line needs beyond the finding itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LineFacts {
    /// First punch on the finding's date after its single open start (clean-pair `N`).
    pub next: Option<NaiveTime>,
    /// True when today has at least one punch (the `or:` line is dropped).
    pub today_has_punches: bool,
}

/// One scan result: the finding plus its line facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LookbackEntry {
    pub finding: LookbackFinding,
    pub facts: LineFacts,
}
```

`LookbackFinding::detail_lines()` and `stderr_line()` keep their signatures and call the new `_with` forms with `LineFacts::default()`. That keeps every T1-moved test and `t33b` compiling and unchanged. Production code calls only the `_with` forms through `LookbackEntry`. This is a deviation from changeset contract 3's wording ("structured findings") only in that `lookback` returns entries, not bare findings. See Risks R1.

### D2. Window and ages

- Scan ages 1 through 14, oldest first (age 14 down to 1). Age 15 stays absent.
- Read `[today - 15, today]` in one `punches_in_range`. The range gains today. It is needed as the `next` neighbor of the age-1 date, and for `today_has_punches`.
- Ages 2-14 classify against the same neighbors as before. Only the age-1 date gains today as its `next`, so ages 2-14 output is unchanged except for the clean-pair window.

### D3. Rendering per kind and age

| Kind | Age | `detail_lines_with` | `stderr_line_with` |
|---|---|---|---|
| single open start | 2-14 | `[!] unclosed start on {wdy} at {t} ({age} days ago)` / `    fix: mlm stop HH:MM --date {D}  {hint}` / `     or: mlm delete punch ...` (unchanged `or:`) | `warning: unclosed start on ...` three lines as today, `{hint}` in the `fix:` line |
| single open start | 1 | `[i] start on {wdy} at {t} still open (yesterday; not counted until stopped)` / `    fix: mlm stop HH:MM --date {D}  {hint}` / `     or: mlm stop  (still working: ends it now, across midnight)` only when `!today_has_punches` | `warning: start on {wdy} at {t} still open (yesterday)` / `    fix: mlm stop HH:MM --date {D}  {hint}`, no `or:` |
| two or more open starts | 1 | `[!] {n} open stints on {wdy} (yesterday)` / `    fix: see mlm status {D}` | `warning: {n} open stints on {wdy} (yesterday); see mlm status {D}` |
| two or more open starts | 2-14 | unchanged | unchanged |

`{hint}` is `render::stop_time_hint(time, facts.next)`. With `next == None` it is byte-identical to today's `(a time after {t})`. Spec §3.1 literals pin the age-1 single lines (`[i] start on Fri 2026-10-02 at 18:02 still open (yesterday; not counted until stopped)`, `fix:` with two spaces before the parenthesis, ` or:` with five leading spaces).

### D4. Hint suppression

`today_page_lines(.., refused_join_names_yesterday)` drops exactly one thing when the flag is true. It drops the age-1 single-open-start entry (the hint). The age-1 multi-open `[!]` line stays, because the spec suppresses only the hint and says the E7 lines "already cover" the rest. All other entries print.

`mlm start` has no suppression. See Risks R2.

### D5. Order

`today_page_lines` walks the entries oldest first and the age-1 entry is last, so the hint lands after every lookback line with no extra sort. T1's `status` composition puts the whole result after the date's anomaly block and before the carry marker line (spec §3.1, §3.3).

## Steps (TDD order)

Write each step's tests first, watch them fail for the right reason, then implement. Commit per step. `cargo fmt` before each commit. If the hook rewrites `coverage-baseline.json`, revert that file before the final commit.

### Step 1. Types and rendering in `src/lookback.rs`

Tests first (in `lookback.rs` `mod tests`, with a fixed `today()` of 2026-02-12 and `d`, `t` helpers as T1 copied them):

- Unit tests on `LookbackFinding::detail_lines_with` / `stderr_line_with` for every row of the D3 table, with exact strings, with `next: None` and `next: Some(t(10, 0))`, and `today_has_punches` true and false for the age-1 single start.
- The T1-moved `t_lookback_*` tests stay as they are and must still pass (default facts, ages 2-14).

Implementation (replace the moved `impl LookbackFinding`, keep the debug asserts but widen the range):

```rust
const MIN_AGE_DAYS: i64 = 1;
const MAX_AGE_DAYS: i64 = 14;

impl LookbackFinding {
    /// Default-facts form: no later punch on the date, today empty.
    pub fn detail_lines(&self) -> Vec<String> {
        self.detail_lines_with(LineFacts::default())
    }

    pub fn stderr_line(&self) -> String {
        self.stderr_line_with(LineFacts::default())
    }

    pub fn detail_lines_with(&self, facts: LineFacts) -> Vec<String> {
        debug_assert!((MIN_AGE_DAYS..=MAX_AGE_DAYS).contains(&self.age_days));
        let d = format_date(self.date);
        let wdy = format_date_with_weekday(self.date);
        match self.kind {
            LookbackKind::SingleOpenStart { time, .. } if self.age_days == 1 => {
                yesterday_hint_lines(&wdy, &d, time, facts)
            }
            LookbackKind::SingleOpenStart { time, .. } => {
                unclosed_start_lines(&wdy, &d, time, self.age_days, facts)
            }
            LookbackKind::MultipleOpenStints { count } => vec![
                format!("[!] {count} open stints on {wdy} {}", age_phrase(self.age_days)),
                format!("    fix: see mlm status {d}"),
            ],
        }
    }

    pub fn stderr_line_with(&self, facts: LineFacts) -> String {
        debug_assert!((MIN_AGE_DAYS..=MAX_AGE_DAYS).contains(&self.age_days));
        let d = format_date(self.date);
        let wdy = format_date_with_weekday(self.date);
        match self.kind {
            LookbackKind::SingleOpenStart { time, .. } if self.age_days == 1 => format!(
                "warning: start on {wdy} at {} still open (yesterday)\n    fix: mlm stop HH:MM --date {d}  {}",
                time.format("%H:%M"),
                stop_time_hint(time, facts.next)
            ),
            LookbackKind::SingleOpenStart { time, .. } => format!(
                "warning: unclosed start on {wdy} at {} ({} days ago)\n    fix: mlm stop HH:MM --date {d}  {}\n     or: mlm delete punch --date {d}  (lists punches; rerun with the start's number)",
                time.format("%H:%M"),
                self.age_days,
                stop_time_hint(time, facts.next)
            ),
            LookbackKind::MultipleOpenStints { count } => format!(
                "warning: {count} open stints on {wdy} {}; see mlm status {d}",
                age_phrase(self.age_days)
            ),
        }
    }

    /// The age-1 single-start entry, which is the `[i]` hint rather than an anomaly line.
    pub fn is_yesterday_hint(&self) -> bool {
        self.age_days == 1 && matches!(self.kind, LookbackKind::SingleOpenStart { .. })
    }
}

/// `(yesterday)` at age 1, `(N days ago)` otherwise.
fn age_phrase(age_days: i64) -> String {
    if age_days == 1 {
        "(yesterday)".to_string()
    } else {
        format!("({age_days} days ago)")
    }
}

fn unclosed_start_lines(wdy: &str, d: &str, time: NaiveTime, age: i64, facts: LineFacts) -> Vec<String> {
    vec![
        format!("[!] unclosed start on {wdy} at {} ({age} days ago)", time.format("%H:%M")),
        format!("    fix: mlm stop HH:MM --date {d}  {}", stop_time_hint(time, facts.next)),
        format!("     or: mlm delete punch --date {d}  (lists punches; rerun with the start's number)"),
    ]
}

fn yesterday_hint_lines(wdy: &str, d: &str, time: NaiveTime, facts: LineFacts) -> Vec<String> {
    let mut lines = vec![
        format!(
            "[i] start on {wdy} at {} still open (yesterday; not counted until stopped)",
            time.format("%H:%M")
        ),
        format!("    fix: mlm stop HH:MM --date {d}  {}", stop_time_hint(time, facts.next)),
    ];
    if !facts.today_has_punches {
        lines.push("     or: mlm stop  (still working: ends it now, across midnight)".to_string());
    }
    lines
}

impl LookbackEntry {
    pub fn detail_lines(&self) -> Vec<String> {
        self.finding.detail_lines_with(self.facts)
    }

    pub fn stderr_line(&self) -> String {
        self.finding.stderr_line_with(self.facts)
    }
}
```

Check the `or:` text and the `[i]` text byte for byte against spec §3.1 lines 71-74 and 96-97 before committing. Update the doc comments on `LookbackFinding` and `LookbackKind` to present state (ages 1-14, age 1 renders as the hint or the `(yesterday)` E7 line). Remove the "age 2-14" wording.

### Step 2. The scan

Tests first:

- `lookback_returns_entries_oldest_first` (T1's `lookback_returns_findings_oldest_first` with `.finding.` added on each access).
- `lookback_includes_age_one_single_open_start` replaces T1's `lookback_excludes_age_one_open_start`. Same fixture (open start at age 1 and age 4). New assertion is two entries, ages `[4, 1]` in that order. This is a deliberate inversion of an old pin. Keep its comment pointing at the reason in present tense (spec §3.1: yesterday is in the window).
- `lookback_excludes_age_fifteen`: open start at age 15 and age 14, only 14 appears. Age 14 and age 2 boundaries both appear.
- `lookback_age_one_joined_by_todays_first_stop_is_absent`: yesterday one open start, today one `end`. Result empty.
- `lookback_age_one_not_joined_when_today_has_two_orphan_stops`: yesterday one open start, today two `end`s (refused join). Result holds the age-1 entry (the page decides suppression, not the scan).
- Facts: `facts.next == Some(t(10, 0))` for the age-5 fixture `start 09:00, start 10:00, end 10:30` (open start 09:00, nested 10:00-10:30). `facts.next == None` with no later punch. `today_has_punches` true with any punch today, false otherwise. Multi-open entries carry `next: None`.

Implementation:

```rust
use chrono::{DateTime, Duration as ChronoDuration, NaiveDate, NaiveTime, Utc};
use rusqlite::Connection;

use crate::date::{format_date, format_date_with_weekday};
use crate::history::PunchBuckets;
use crate::render::{local_time, next_punch_after, stop_time_hint};
use crate::{stint, storage};

/// One finding per date with open starts, ages 1-14, oldest first.
/// One ranged read over `[today - 15, today]`: the 14 window dates plus the
/// previous-day neighbor of age 14 and today, the next-day neighbor of age 1.
pub fn lookback(
    conn: &Connection,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
) -> anyhow::Result<Vec<LookbackEntry>> {
    let from = today - ChronoDuration::days(MAX_AGE_DAYS + 1);
    let punches = storage::punches_in_range(conn, from, today)?;
    let buckets = PunchBuckets::from_punches(&punches);
    let today_has_punches = !buckets.day(today).is_empty();
    Ok((MIN_AGE_DAYS..=MAX_AGE_DAYS)
        .rev()
        .filter_map(|age| {
            let date = today - ChronoDuration::days(age);
            entry_for_date(date, age, &buckets, today_has_punches, now_utc)
        })
        .collect())
}

/// The entry for one window date, or `None` when it has no open start after
/// splice resolution (idle, or an orphaned end only).
fn entry_for_date(
    date: NaiveDate,
    age: i64,
    buckets: &PunchBuckets,
    today_has_punches: bool,
    now_utc: DateTime<Utc>,
) -> Option<LookbackEntry> {
    let trio = buckets.trio(date);
    if trio.day.is_empty() {
        return None;
    }
    let day = stint::classify_at(trio.prev, trio.day, trio.next, now_utc);
    let (kind, next) = match day.open.as_slice() {
        [] => return None,
        [only] => (
            LookbackKind::SingleOpenStart {
                time: local_time(only.start.at_utc),
                punch_id: only.start.id,
            },
            next_punch_after(trio.day, &only.start),
        ),
        many => (LookbackKind::MultipleOpenStints { count: many.len() }, None),
    };
    Some(LookbackEntry {
        finding: LookbackFinding { date, age_days: age, kind },
        facts: LineFacts { next, today_has_punches },
    })
}
```

Delete `group_punches_by_date` and the old `lookback_finding_for_date` (T1 moved them into this file). `PunchBuckets` replaces them. `classify_at`'s debug asserts hold because every bucket holds one date.

### Step 3. `today_page_lines`

Tests first:

- AC1 to AC7 unit tests below, all through `today_page_lines`.
- T1's `today_page_lines_equals_flattened_findings` (its flag `true` and `false` fixture holds only ages 4 and 10, so both flags give the same output): rewrite its expected value as `lookback(..)?.iter().flat_map(LookbackEntry::detail_lines)`, keep both flag values.

Implementation:

```rust
/// Finished lookback lines for today's page, oldest first, then the age-1
/// hint. `refused_join_names_yesterday` drops the hint only (SPEC §3.1).
pub fn today_page_lines(
    conn: &Connection,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
    refused_join_names_yesterday: bool,
) -> anyhow::Result<Vec<String>> {
    Ok(lookback(conn, today, now_utc)?
        .iter()
        .filter(|e| !(refused_join_names_yesterday && e.finding.is_yesterday_hint()))
        .flat_map(LookbackEntry::detail_lines)
        .collect())
}
```

Complexity is under 5. Update the `today_page_lines` doc comment: it returns lookback and hint lines, flag is honored.

### Step 4. `src/commands.rs`

Tests first (see AC8 and AC9 below), then edit:

```rust
// imports: drop `render` and `status`, add the lookback module
use crate::lookback::{self, LookbackEntry, LookbackKind};
```

```rust
/// §7.4: after a successful `start` write, warn on stderr about every open
/// start the shared lookback (`lookback::lookback`) can see, yesterday
/// included. Best-effort -- a read failure here is swallowed, never
/// surfaced as an error and never turns a successful write into a failed
/// command.
fn warn_stale_starts<W: std::io::Write>(/* unchanged params */) {
    let Ok(entries) = lookback::lookback(conn, today, now_utc) else {
        return;
    };
    emit_warnings(entries, written_punch_id, out);
}

fn emit_warnings<W: std::io::Write>(
    entries: Vec<LookbackEntry>,
    written_punch_id: i64,
    out: &mut W,
) {
    for entry in entries {
        if let LookbackKind::SingleOpenStart { punch_id, .. } = entry.finding.kind
            && punch_id == written_punch_id
        {
            continue;
        }
        let _ = writeln!(out, "{}", entry.stderr_line());
    }
}
```

Keep the existing `emit_warnings` doc comment (the exemption rule is unchanged). Reword its first lines so they say "entries" and drop the `status::lookback` mention in the doc above `warn_stale_starts` (done above).

Existing `commands.rs` tests:

- Helper `finding(date, age, kind)` returns `LookbackEntry { finding: LookbackFinding { date, age_days: age, kind }, facts: LineFacts::default() }`. Import `crate::lookback::{LineFacts, LookbackEntry, LookbackFinding, LookbackKind}` in `mod tests`. Replace the four `crate::render::LookbackKind::...` paths with the imported name. `f.stderr_line()` stays valid, `LookbackEntry` is `Copy`.
- All assertions stay as they are. These tests pin the written-punch exemption and the no-reordering rule and they must pass unchanged.

### Step 5. Integration tests `tests/start_hint_cli.rs`

Pattern from `tests/status_cli.rs`: `Command::new(env!("CARGO_BIN_EXE_mlm"))`, `MLM_DB_PATH` into a `TempDir`, `TZ=UTC` on the child. Helpers copied locally (`mlm_utc`, `seed_punch`, `stdout`, `stderr`). A `boot(db)` runs `mlm status` once to create the schema. `today()` is `chrono::Utc::now().date_naive()`. `seed(db, date, "HH:MM", kind)` writes `at_utc = "{date}T{HH:MM}:00Z"`, `date`, `kind` (UTC equals local under `TZ=UTC`). Expected weekday forms come from `mlm::date::format_date_with_weekday(date)`. Cases are listed per acceptance criterion below.

### Step 6. Verification

`cargo fmt`, `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings`, and the two `MLM_DB_PATH` smoke runs from AGENTS.md. Complexity as the hook runs it: `cargo clippy --all-targets --quiet --message-format=json -- -W clippy::cognitive_complexity` reports nothing. If any T1-owned test (`src/status.rs`, `tests/*.rs`) fails because of the behavior change, stop and report it to the coordinator. Do not edit it. I found none by reading: every `status.rs` test that seeds a start dated yesterday asserts on `has_open_stint`, `eod`, `week_line`, `Day total` or `"not joined to"`, never on an empty `anomaly_lines` for today.

## Test plan, by acceptance criterion

Unit tests live in `src/lookback.rs` `mod tests` unless noted. Fixtures use `today()` 2026-02-12 (Thursday) and yesterday `Wed 2026-02-11`. Punches are inserted with `storage::insert_punch(&conn, kind, date, time, &Local)`, `now` is Thursday 18:00 local. The hint literal for yesterday is:

```
[i] start on Wed 2026-02-11 at 18:02 still open (yesterday; not counted until stopped)
    fix: mlm stop HH:MM --date 2026-02-11  (a time after 18:02)
     or: mlm stop  (still working: ends it now, across midnight)
```

### AC1. One open start yesterday, no punches today: three-line hint with `or:` (spec §3.1, §7 "no punches today (both lines)")

- Unit: `today_page_lines(&conn, today, now, false)` equals the three lines above exactly.
- Integration `status_hint_with_or_line_when_today_is_empty`: seed yesterday `start 18:02`, run `mlm status`, stdout contains the three lines with computed date and weekday. Every added line is ASCII.

### AC2. Today has a punch that does not join: `fix:` only (spec §3.1, §7 "punches today (no `or:`)")

- Unit: today `start 09:00` gives the first two lines only. Today `start 09:00` plus `end 10:00` (closed pair, no orphan) gives the first two lines only.
- Integration `status_hint_has_no_or_line_when_today_has_a_start`: seed today `start 00:00`, stdout has the `[i]` and `fix:` lines and does not contain ` or: mlm stop`.

### AC3. Today's first punch is a joining `stop`: no hint, no false positive after the hint's own fix (spec §3.1, §7 "a `stop` today that joins")

- Unit: today `end 00:30` with yesterday one open start gives an empty vector. `lookback()` itself returns no age-1 entry here.
- Unit: yesterday `start 18:02` plus `end 19:00` on yesterday (the hint's fix) gives an empty vector.
- Integration `stop_today_joins_and_clears_the_hint`: seed yesterday start, `mlm status` shows the hint, run `mlm stop`, `mlm status` shows no `[i]` line. A second case runs `mlm stop 19:00 --date <yesterday>` instead and checks the same.

### AC4. Refused-join line naming yesterday: no hint (spec §3.1, §7 "a refused join")

- Unit: yesterday one open start, today two `end`s. `today_page_lines(.., true)` is empty. `today_page_lines(.., false)` holds the hint (proves the flag is the only switch).
- Unit through `crate::status::resolve(None, now, &conn)` (T1's wiring): same fixture, `view.anomaly_lines` contains a `not joined to` line and no line starting with `[i]`.
- Unit: with the flag true, the age-1 multi-open `[!]` line still prints.
- Integration `refused_join_suppresses_the_hint`: seed yesterday start 18:02, today `end 00:10` and `end 00:20`, stdout has `not joined to` and no `[i]`.

### AC5. Two or more open starts yesterday: E7 lookback line with `(yesterday)` (spec §3.1, §7 "two open starts yesterday")

- Unit: yesterday `start 09:00` and `start 10:00` gives exactly `[!] 2 open stints on Wed 2026-02-11 (yesterday)` and `    fix: see mlm status 2026-02-11`. No `[i]`.
- Unit: ages 14, 5 and 1 each with one open start give entries in the order 14, 5, 1 (lines of the age-14 and age-5 groups first, hint last). The age-14 and age-5 groups are byte-identical to the T1-moved `t_lookback_*` strings.
- Unit: an open start at age 15 is absent (`lookback_excludes_age_fifteen`). Age 14 and age 2 are present (spec §7 and the plan's "15-day-old start is still absent").
- Unit: `LookbackFinding` default-facts renderings for ages 2-14 equal today's strings (the moved `t_lookback_*` tests, untouched).
- Integration `two_open_starts_yesterday_print_the_yesterday_e7_line`.

### AC6. The hint is `[i]`: no week-row mark, no footer, not on `status DATE` (spec §3.1 second paragraph, §6 last bullets)

- Integration `hint_does_not_mark_week_row_or_footer`: seed yesterday one open start. `mlm week <WeekId::from_date(yesterday)>` stdout has no `[!]` and no `needs a look`.
- Integration `hint_never_shows_on_status_for_another_date`: `mlm status -1` and `mlm status <yesterday>` stdout have no `[i]`. `mlm status <today's date string>` still has it (explicit today is the same code path).
- Unit: `crate::week_view::build_week_view(&conn, WeekId::from_date(yesterday), now)` has `rows[..].has_anomaly == false` for the yesterday row in the one-open-start fixture.

### AC7. Placement: after today's block and lookback lines, before the carry marker (spec §3.1, §3.3)

- Unit through `crate::status::resolve`: today has two starts (E7 on today), age 5 has one open start, yesterday has one open start. `view.anomaly_lines` order is today's E7 lines, the age-5 `[!] unclosed start` group, then the three hint lines. The marker line is T4's. After T4 merges, the coordinator's merged-branch check adds one assertion that a marker line follows the hint. This task does not test it.
- Unit: the hint is the last entry of `today_page_lines` when older findings exist.

### AC8. `mlm start` warning (spec §3.1 last block, §7 "`mlm start` warning")

- Unit through `warn_stale_starts` (fixed `today()` and `now_utc`, written id from `punch(...)`): yesterday has one open start at 18:02, today `start` is written. Output equals `"warning: start on Wed 2026-02-11 at 18:02 still open (yesterday)\n    fix: mlm stop HH:MM --date 2026-02-11  (a time after 18:02)\n"`. No `or:` anywhere.
- Unit: yesterday with two open starts, today `start` written. Output equals `"warning: 2 open stints on Wed 2026-02-11 (yesterday); see mlm status 2026-02-11\n"`.
- Unit, exemption: `start --date -1 09:00` into an empty yesterday writes the only open start. `warn_stale_starts` prints nothing. A second `start --date -1 10:00` prints the one-line multi form (the new punch made an E7).
- Unit, older-start-on-written-date rule (existing F3 behavior) still holds with an age-1 date: written id differs from the open start's id, so it prints.
- Unit, entry-level `emit_warnings` for an age-1 `SingleOpenStart` with matching `punch_id` prints nothing.
- Existing best-effort test `read_failure_after_write_is_best_effort` stays and passes with the new read range.
- Integration `start_warns_about_yesterday_open_start_on_stderr`: seed yesterday `start 18:02`, run `mlm start 00:00`, exit 0, stdout empty, stderr contains the two-line warning for the computed date and no `or:`. A second integration case runs `mlm start 18:02 --date <yesterday>` on an empty yesterday and expects no `warning:`.

### AC9. Clean-pair rule on every lookback line, hint and warning, with the round trip (spec §3.2.2, §7 "each hint kind with and without a following punch")

- Unit, one case per surface, each fixture `start 09:00`, `start 10:00`, `end 10:30` on the dated day (open start 09:00 alone, nested stint 10:00-10:30, so `N` is 10:00):
  - age 5, `detail_lines`: `fix:` line reads `(a time between 09:00 and 10:00)`.
  - age 1, hint: same window phrase in the `fix:` line.
  - age 5 `stderr_line` and age 1 `stderr_line`: same window phrase.
  - each of the four with no later punch keeps `(a time after 09:00)`.
  - `N` one minute after `S` (`start 09:00`, `start 09:01`, `end 09:30`) reads `(a time between 09:00 and 09:01)`.
- Unit round trip (spec-review F1 guard), for each of age 5 and age 1: with the fixture above, insert an `end` at 09:30, then at 10:00 (N's own minute), then at 09:01. For each, classify the date with `stint::classify` and assert the completed stints are exactly `[09:00 to the new stop]` and `[10:00 to 10:30]`, and `open` is empty, and `!anomalies::date_facts(&buckets, date, today, now).has_any()`. The structural assertion proves pairing now. The `date_facts` assertion becomes the overlap check once T2 merges. It is trivially true before then, and the test file notes that in a one-line comment.
- Unit: a stop at 10:01, past the window, is asserted to nest (`[09:00 to 10:01]` and `[10:00 to 10:30]` overlap), which shows the window really is the boundary. This is a plain stint assertion, no anomaly module.
- Integration `follow_the_hint_round_trip`: seed yesterday `start 09:00`, `start 10:00`, `end 10:30`. `mlm status` shows `(a time between 09:00 and 10:00)`. Run `mlm stop 09:30 --date <yesterday>`. `mlm status` shows no `[i]` and no `[!]` for yesterday, and `mlm status <yesterday>` shows no `[!]` line.

### AC10. Spec §3.1 cases that already hold, kept as guards

- Unit `age_one_multi_open_without_hint_form`: age-1 multi never renders the `[i]` form.
- Unit `yesterday_hint_needs_exactly_one_open_start`: two open starts give no `[i]` (covered by AC5, asserted explicitly).
- Unit `splice_through_today_uses_classify_at`: yesterday `start 23:00`, today `end 00:10` gives no entry. Guards against reading yesterday alone.

### AC11. Verification, complexity, coverage

- The full verification set in step 6.
- Every new branch has a test above: age 1 and age 2-14 for both kinds in both renderers, `next` present and absent, `today_has_punches` both ways, the suppression flag both ways, the `entry_for_date` empty-bucket and zero-open exits, the three `match` arms on `day.open`.
- New functions are small (`age_phrase`, `unclosed_start_lines`, `yesterday_hint_lines`, `entry_for_date`, `lookback`, `today_page_lines`, the four renderer methods). The largest, `detail_lines_with`, has one `match` with a guard and sits near 4.
- `coverage-baseline.json` is reverted if the hook rewrites it.

## Risks, ambiguities and disagreements

R1. **`lookback` returns `LookbackEntry`, not `LookbackFinding`.** Changeset contract 3 says `mlm start` consumes "structured findings". T1 freezes the shape of `LookbackFinding`/`LookbackKind` indirectly, because `render.rs::t33b` (T1-owned, frozen) builds them as struct literals. Adding `next` to either type breaks that test, and this task may not edit `render.rs`. The wrapper keeps `LookbackFinding` unchanged and adds `LineFacts`. If the coordinator prefers a field on the finding, the alternative is one edit to `t33b` (add `next: None`) by T1 before merge. I did not plan on that, because a missed handoff fails the build at integration. Cost of the wrapper is that `LookbackFinding::detail_lines()` and `stderr_line()` stay as default-facts forms used only by tests.

R2. **`mlm start` does not apply the refused-join suppression.** Spec §3.1 says "No hint when today's page already shows a refused-join line", which is a page rule, and `start` has no page. Yesterday's start is still open in that case, so the warning stays. It prints once per `start`. If the spec author meant `start` to stay quiet too, the change is one more argument to `warn_stale_starts` and is not planned here.

R3. **The `status` page flag is the refused-join-with-previous fact from T1's `resolve`.** `is_refused_join` needs yesterday to have an open start in its raw classification, which equals the scan's own "one open start after splice" when the join is refused, so the two cannot diverge for a lone start. I did not add a second check inside the scan.

R4. **Inverted pin.** T1 moves `lookback_excludes_age_one_open_start` into `lookback.rs`, and spec §3.1 reverses it. I rewrite it in place (Step 2), which is a spec-mandated change to a moved test, not a deletion.

R5. **Integration tests depend on the real clock.** `Utc::now()` in the test and in the child can straddle midnight UTC, which shifts "yesterday". The window is a few milliseconds. Seeds of "today" punches at `00:00` avoid clock-of-day sensitivity. Accepted. Deterministic cases live in the unit tests with injected `now`.

R6. **Hint ordering relies on the walk order**, oldest first, with age 1 last. No test seeds a hint and an older finding in the wrong order, because the scan cannot produce one. AC5 and AC7 pin the order anyway.

R7. **Same-instant punches and the window.** `next_punch_after` (T1) compares `at_utc`. Stored punches are minute-granular, so a punch in the same minute as `S` is excluded and `N` is always strictly later by at least one minute. A raw-seeded row with seconds could print `between 09:00 and 09:00`. Not reachable through the CLI. Not tested.

R8. **Today's status tests in `src/status.rs`.** The change adds hint lines for any `resolve(None, ..)` fixture with exactly one open start yesterday and no suppressing refusal. I read the existing tests and found none that asserts on an empty `anomaly_lines` for such a fixture, but I could not run them. Step 6 says stop and report on any failure.

R9. **`status` explicit today.** `status <today's date string>` runs the lookback and prints the hint, as the existing explicit-today lookback test (`resolve_explicit_todays_date_string_still_runs_lookback`) already does for older ages. That follows spec §3.1 "today's `status`" and the plan's note that `status -1` and other dates never print it.

R10. **Disagreement with the changeset plan.** None on scope. One note: the plan's T3 paragraph says "extend today's scan to read yesterday and today". The scan reads the whole `[today - 15, today]` range in one query, which includes both, so it still makes one read.
