# Task plan: possible-issues-triage, T2 Overlap anomaly and clean-pair stop hints in anomaly lines

> **Archived — historical planning record.** Not authoritative. Current spec: `docs/dev/SPEC.md`. Design/process history: `docs/dev/NOTES.md`.

Changeset plan: `docs/dev/plans/possible-issues-triage-plan.md` (T2 section, contracts 2 and 7, decision F8). Spec: `docs/dev/specs/2026-10-03-possible-issues-triage.md` (§3.2.1, §3.2.2, §7).

## What I read

- `docs/dev/plans/possible-issues-triage-plan.md`, full.
- `docs/dev/specs/2026-10-03-possible-issues-triage.md`, full.
- Sibling task plans, full: `possible-issues-triage-task-1-foundation.md` ("Published seams" is the contract I build on), `-task-5-cli-surface.md`, `-task-6-docs.md`. T3 and T4 plans do not exist yet, so nothing is imported from them.
- Source at the branch point (T1 not landed, so `src/anomalies.rs` does not exist yet):
  - `src/render.rs` 1-420 (today's home of `DateAnomalies`, `StaleOpenStart`, `refused_join_line`, `WEEK_ANOMALY_FOOTER`, tests `da()` helper at 735, t33b at 1015-1060).
  - `src/stint.rs` 1-420 (`classify`, `close_group`, `classify_at`, `splice_candidate`, `stale_open_start`, `is_refused_join`).
  - `src/status.rs` 300-372 (`DateContext`, `anomaly_facts_for_date`) and `src/week_view.rs` 60-180 (`render_week`, `build_rows`).
  - `src/storage.rs` `Punch` (94-106) and `punch_from_local` (238-252).
  - `tests/status_cli.rs` 1-120 (binary-test helpers).
- `clippy.toml` (cognitive-complexity-threshold 15) and `.githooks/pre-commit` (fmt check, complexity gate via clippy JSON, coverage ratchet vs `coverage-baseline.json`).

## Scope

Files this task owns:

| File | Action |
|---|---|
| `src/anomalies.rs` | extend (created by T1, mine after T1 merges) |
| `tests/overlap_cli.rs` | new |

Files this task must not touch, and owner:

| File | Owner |
|---|---|
| `src/status.rs`, `src/week_view.rs`, `src/render.rs`, `src/stint.rs`, `src/lib.rs`, `src/week.rs`, `src/storage.rs`, `src/date.rs`, `src/history.rs`, `tests/status_cli.rs`, `tests/e6_db_failure.rs`, `tests/ascii_sweep.rs`, `tests/week_read_errors.rs` | T1 (frozen at merge) |
| `src/lookback.rs`, `src/commands.rs`, `tests/start_hint_cli.rs` | T3 |
| `src/carry_scan.rs`, `tests/carry_marker_cli.rs` | T4 |
| `src/cli.rs`, `src/main.rs`, `src/week_target.rs`, `tests/week_target_cli.rs` | T5 |
| `docs/dev/SPEC.md`, `README.md`, `AGENTS.md` | T6 |
| NOTES.md, the spec, `coverage-baseline.json` | coordinator |

Exception (changeset plan, Global constraints): when this task's spec change inverts an existing assertion in a T1-owned file, I may re-pin that assertion's expected value and nothing else, and I list each re-pin in my completion report.

The `status` page, the `mlm week` row `[!]` and the footer count follow from `build_facts`/`detail_lines`/`has_any` with no edit to `src/status.rs` or `src/week_view.rs` (T1 plan steps 5 and 6 call `build_facts` from both).

## Dependencies

| Dependency | State | Used for | Stub today / swap at integration |
|---|---|---|---|
| T1 `src/anomalies.rs` seams: `DateBuckets`, `DateContext`, `StaleOpenStart`, `DateAnomalies` (+ `none`, `has_any`, `detail_lines`, `row_marker`), `classify_context`, `build_facts`, `date_facts` | not landed, worktree opens from merged T1 | everything below | None needed. Plan assumes T1's published signatures verbatim. If T1's completion report records a deviation in any identifier listed here, adjust it in step 0 and note it in my report. |
| T1 `src/render.rs`: `local_time`, `next_punch_after`, `stop_time_hint`, `stop_time_hint_closes` | not landed, frozen | clean-pair windows | Called as published. Never edited by T2. |
| T1 `stint::splice_candidate` (`pub(crate)`, exists today at `src/stint.rs:342`) | shipped | the receiving-side splice part | Called as published. |
| T4 (carry scan) | parallel | T4 calls `date_facts(..).has_any()` | No handoff needed. `has_any` includes overlap after this task, so T4's overlap-only-week test works after rebase onto merged T2 (changeset plan, ordering). |
| T3 | parallel | calls `render::stop_time_hint` itself | No dependency either way. |

## Facts gathered

Current code (line numbers at branch point `cccfb8f`, before T1's moves):

- `render.rs:130-164` `DateAnomalies` has seven fields and derives `Debug, Clone, PartialEq, Eq`. T1 moves it to `src/anomalies.rs` unchanged and adds `none(date)`.
- `render.rs:181-187` `has_any` ORs five conditions. `render.rs:195-202` `detail_lines` appends `e7_lines`, `stale_lines`, `e8_lines`, `refused_join_lines` in that order.
- `render.rs:219` the E7 `or:` line is `"     or: mlm stop HH:MM --date {d}  (a time after {latest} closes the {latest} start)"`. `render.rs:236` the stale `fix:` line is `"    fix: mlm stop HH:MM --date {d}  (a time after {time})"`. These are the two lines T2 rewires.
- `stint.rs:57-69` `DayStints { completed, open, orphaned_ends }`. `open` is ascending by start instant, so `.last()` is the latest open start (`stint.rs:117` uses it for `live_minutes`). `stint.rs:322-326`: a splice on the start side pushes `Stint { start: open.start, end: next_orphan.punch, .. }` into `day.completed` with `end.date` equal to the next date. A splice on the receiving side only removes the orphan from `day.orphaned_ends` (`stint.rs:309-311`). The stint then lives only in the earlier date's `completed`.
- `stint.rs:342-354` `splice_candidate(earlier_open_count, later, later_punches)` is `pub(crate)`, usable from `anomalies.rs`.
- `storage.rs:94-106` `Punch { id, at_utc, date, kind }`, `at_utc` minute-granular. `storage.rs:238` `punch_from_local(id, kind, local_date, local_time, &tz)` is the test constructor.
- T1 `build_facts(ctx, b, date, today, _now_utc)` already carries `now_utc` for this task. T1 `DateContext` holds `day`, `prev_raw`, `day_raw`, `next_raw`.
- Complexity: T1 keeps `build_facts` as one struct literal with a helper call per field. T2 adds two fields the same way.
- A search of `src/status.rs`, `src/week_view.rs`, `src/render.rs`, `src/commands.rs` and `tests/` finds no test named for nested or overlapping stints. Existing fixtures may still overlap by accident (see step 0 and risk 1).

## Design

### New data

All in `src/anomalies.rs`. Two fields join `DateAnomalies`, appended after `refused_join_with_next`.

```rust
use chrono::{DateTime, Local, NaiveDate, NaiveTime, TimeZone, Utc};
use crate::render;   // plus T1's existing imports (`format_date`, `DayStints`, ...)
use crate::stint::{self, Stint};

/// One stint's part on a single date. `start` and `end` bound that part and
/// drive the overlap test. `full_start` and `full_end` are the stint's real
/// times and drive the label, matching the status stint list. They equal
/// `start` and `end` unless the stint is spliced across midnight. `open` marks
/// today's running stint, whose `end` is `now`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StintSpan {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub full_start: DateTime<Utc>,
    pub full_end: DateTime<Utc>,
    pub open: bool,
}

impl StintSpan {
    /// A span whose label times equal its bounds.
    fn whole(start: DateTime<Utc>, end: DateTime<Utc>, open: bool) -> Self {
        Self { start, end, full_start: start, full_end: end, open }
    }
}

/// Two stints on one date whose ranges overlap. `first` starts no later
/// than `second`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverlapPair {
    pub first: StintSpan,
    pub second: StintSpan,
}

// DateAnomalies gains:
pub struct DateAnomalies {
    // ... T1's seven fields unchanged ...
    /// One entry per overlapping pair, ordered by first start then second start.
    pub overlaps: Vec<OverlapPair>,
    /// N for the latest open start: the local time of the first punch on the
    /// date strictly after it (§3.2.2). `None` when nothing follows or no
    /// start is open. Feeds the E7 and stale-start hints.
    pub latest_open_next: Option<NaiveTime>,
}
```

`StaleOpenStart` does not change. One field serves both hints, because a stale start is by definition the only open start, so it is also the latest. No literal of `StaleOpenStart` is touched, and `render.rs` tests that build it are not affected.

`none(date)` sets `overlaps: Vec::new()` and `latest_open_next: None`. Only `none` and `build_facts` list the new fields.

`has_any` gains `|| !self.overlaps.is_empty()`. `latest_open_next` never makes a date anomalous on its own.

### Span construction

```rust
/// Local midnight at the start of `date`, as a UTC instant. A zone whose
/// midnight falls in a DST gap uses the date's naive midnight read as UTC.
fn local_midnight(date: NaiveDate) -> DateTime<Utc> {
    let naive = date.and_time(NaiveTime::MIN);
    Local
        .from_local_datetime(&naive)
        .earliest()
        .map_or_else(|| naive.and_utc(), |t| t.with_timezone(&Utc))
}

fn day_end(date: NaiveDate) -> DateTime<Utc> {
    local_midnight(date.succ_opt().unwrap_or(date))
}

/// A completed stint's part on `date`. Only a spliced stint ends on a later
/// date. Its bounds stop at the date's end and its label keeps the real end.
fn closed_span(stint: &Stint, date: NaiveDate) -> StintSpan {
    let (start, full_end) = (stint.start.at_utc, stint.end.at_utc);
    let end = if stint.end.date > date { day_end(date) } else { full_end };
    StintSpan { start, end, full_start: start, full_end, open: false }
}

/// The part of a stint spliced in from the previous date: local midnight to
/// the matched orphaned end. The label keeps the stint's real start (the
/// previous date's open start) and real end. `None` when no splice lands on
/// this date.
fn received_span(ctx: &DateContext, b: &DateBuckets<'_>, date: NaiveDate) -> Option<StintSpan> {
    if !stint::splice_candidate(ctx.prev_raw.open.len(), &ctx.day_raw, b.day) {
        return None;
    }
    let end = ctx.day_raw.orphaned_ends.first()?.punch.at_utc;
    let full_start = ctx.prev_raw.open.first()?.start.at_utc;
    Some(StintSpan { start: local_midnight(date), end, full_start, full_end: end, open: false })
}

/// Today's most recent open stint, up to now. Past dates and a start at or
/// after now give nothing (a zero-minute stint is never flagged).
fn live_span(ctx: &DateContext, date: NaiveDate, today: NaiveDate, now_utc: DateTime<Utc>) -> Option<StintSpan> {
    if date != today {
        return None;
    }
    let start = ctx.day.open.last()?.start.at_utc;
    (start < now_utc).then_some(StintSpan::whole(start, now_utc, true))
}

fn date_spans(ctx: &DateContext, b: &DateBuckets<'_>, date: NaiveDate, today: NaiveDate, now_utc: DateTime<Utc>) -> Vec<StintSpan> {
    let mut spans: Vec<StintSpan> = ctx.day.completed.iter().map(|s| closed_span(s, date)).collect();
    spans.extend(received_span(ctx, b, date));
    spans.extend(live_span(ctx, date, today, now_utc));
    spans
}
```

Notes:

- `closed_span` clamps only when `stint.end.date > date`. A raw-inserted fixture whose stored `date` disagrees with its `at_utc` is therefore never distorted.
- The clamped bounds never print. A spliced stint's label uses `full_start` and `full_end`, so both its dates show the real range (for example `23:30-00:45`). See ambiguity A1.
- `received_span` reads the real start from `ctx.prev_raw.open.first()`. `splice_candidate` is true only when the previous date has exactly one open start (confirm against `src/stint.rs` `splice_candidate` at step 0 and use the same open entry the splice matched).
- A receiving span from `00:00` to an orphan at `00:00` has zero length and is dropped by `overlap_pairs`.
- `Stint` is `crate::stint::Stint`. Import it with `use crate::stint::{self, DayStints, Stint};` (T1's file already imports `DayStints`).

### Pairing

```rust
impl StintSpan {
    fn has_length(&self) -> bool { self.start < self.end }
    fn overlaps(&self, other: &Self) -> bool { self.start < other.end && other.start < self.end }
    /// The stint's real range, as the status stint list prints it.
    fn label(&self) -> String {
        let start = render::local_time(self.full_start).format("%H:%M");
        if self.open {
            format!("{start}-now")
        } else {
            format!("{start}-{}", render::local_time(self.full_end).format("%H:%M"))
        }
    }
}

/// Every overlapping pair among the spans, ordered by first start then second
/// start. Zero-length spans are dropped. Spans that share a start sort longest
/// first, so the outer stint is `first`.
pub fn overlap_pairs(mut spans: Vec<StintSpan>) -> Vec<OverlapPair> {
    spans.retain(StintSpan::has_length);
    spans.sort_by_key(|s| (s.start, std::cmp::Reverse(s.end)));
    let mut out = Vec::new();
    for (i, first) in spans.iter().enumerate() {
        for second in &spans[i + 1..] {
            if first.overlaps(second) {
                out.push(OverlapPair { first: *first, second: *second });
            }
        }
    }
    out
}
```

- With spans sorted by start, iterating `i` then `j` yields pairs ordered by first start, then second start. `sort_by_key` is stable, so identical spans keep input order.
- The strict `<` comparisons give the spec's rule (§3.2.1): nests sharing an endpoint overlap, identical ranges overlap, touching stints do not.
- `n` stints that all nest give `n(n-1)/2` pairs (3 levels give 3, 4 give 6).
- Cognitive complexity is about 6 (two `for`, one `if`, one nesting bonus).

### Facts

```rust
fn overlap_facts(ctx, b, date, today, now_utc) -> Vec<OverlapPair> {
    overlap_pairs(date_spans(ctx, b, date, today, now_utc))
}

/// N after the latest open start (§3.2.2).
fn latest_open_next(ctx: &DateContext, b: &DateBuckets<'_>) -> Option<NaiveTime> {
    let open = ctx.day.open.last()?;
    render::next_punch_after(b.day, &open.start)
}
```

`build_facts` adds two lines to its struct literal and drops the underscore on `now_utc`:

```rust
overlaps: overlap_facts(ctx, b, date, today, now_utc),
latest_open_next: latest_open_next(ctx, b),
```

`build_facts` stays one struct literal with one helper call per field. Its own complexity is 0, so no split is needed. Each new helper is at most 6.

### Rendering

`detail_lines` becomes E7, stale, E8, overlap, refused join:

```rust
out.extend(self.e7_lines());
out.extend(self.stale_lines());
out.extend(self.e8_lines());
out.extend(self.overlap_lines());
out.extend(self.refused_join_lines());
```

```rust
fn overlap_lines(&self) -> Vec<String> {
    let d = format_date(self.date);
    self.overlaps
        .iter()
        .flat_map(|p| {
            [
                format!(
                    "[!] overlapping stints {} and {} (the overlap counts twice)",
                    p.first.label(),
                    p.second.label()
                ),
                format!(
                    "    fix: mlm delete punch --date {d}  (lists punches; rerun with the number of the punch to remove)"
                ),
            ]
        })
        .collect()
}
```

Each pair prints the `[!]` line and its own `fix:` line, as E8 does for each orphan (ambiguity A2).

Two edits to existing lines. With `latest_open_next == None` both produce the same bytes as today.

```rust
// e7_lines, the third entry
format!(
    "     or: mlm stop HH:MM --date {d}  {}",
    render::stop_time_hint_closes(latest_time, self.latest_open_next)
),

// stale_lines, the second entry
format!(
    "    fix: mlm stop HH:MM --date {d}  {}",
    render::stop_time_hint(stale.time, self.latest_open_next)
),
```

In `e7_lines` the local `latest` is a formatted `%H:%M` today. Keep the raw `NaiveTime` under another name (for example `latest_time`) for the helper call, and drop the now-unused formatted copy. Update the doc comments on `has_any` and `detail_lines` to list overlap.

## Steps

TDD throughout. Write each step's tests first, watch them fail for the right reason, then implement. `cargo fmt` before each commit. If the pre-commit hook rewrites `coverage-baseline.json`, revert that file before the final commit.

### Step 0. Open from merged T1 and baseline

1. `grep -n "pub fn \|pub struct \|fn " src/anomalies.rs` and compare with T1's published seams. Record any rename and use T1's names in the steps below.
2. Run `cargo test`. It must be green before any edit.
3. Existing-fixture check, once after step 4 (see its last line). Any existing test elsewhere that turns red because its fixture contains overlapping stints is outside my files. When the red assertion sits in a T1-owned file (`src/status.rs`, `src/week_view.rs`, `src/render.rs`, T1's test files) and my spec change inverts it, apply the changeset plan's Global constraints exception. Re-pin that assertion's expected value and nothing else, and list each re-pin in my completion report. For a red test in any other task's file (for example T3's `src/commands.rs`), stop and report it to the coordinator.

### Step 1. Overlap data and `overlap_pairs` (tests first)

Add `StintSpan`, `OverlapPair`, `overlap_pairs`, `has_length`, `overlaps`. Unit tests in `anomalies.rs` `mod tests` (see AC1).

### Step 2. Span construction and facts

Add `local_midnight`, `day_end`, `closed_span`, `received_span`, `live_span`, `date_spans`, `overlap_facts`. Add the two `DateAnomalies` fields, `none`, `has_any`, and the `build_facts` lines. Tests per AC1 (fact level) and AC3.

### Step 3. Rendering

Add `overlap_lines`, `label`, and the `detail_lines` change. Tests per AC2 and AC3.

### Step 4. Clean-pair lines

Add `latest_open_next` and rewire the E7 and stale lines. Tests per AC4, AC5, AC6, AC7. Run the whole suite, and apply the Step 0 check.

### Step 5. Integration tests

Write `tests/overlap_cli.rs` (AC3, AC4, AC5 end to end, AC6).

### Step 6. Verification

The full AGENTS.md set: `cargo fmt`, `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings`, and both `MLM_DB_PATH` smoke runs. Complexity as the hook runs it: `cargo clippy --all-targets --quiet --message-format=json -- -W clippy::cognitive_complexity` reports no hits.

## Test plan, by acceptance criterion

Unit test helpers in `anomalies.rs` `mod tests` (check first whether T1 already defines equivalents, and reuse them):

```rust
fn day() -> NaiveDate { NaiveDate::from_ymd_opt(2026, 9, 1).unwrap() }          // past date
fn punch(id: i64, kind: PunchKind, h: u32, m: u32) -> Punch {
    storage::punch_from_local(id, kind, day(), NaiveTime::from_hms_opt(h, m, 0).unwrap(), &Local).unwrap()
}
fn at(h: u32, m: u32) -> DateTime<Utc> { punch(0, PunchKind::Start, h, m).at_utc }
fn span(sh: u32, sm: u32, eh: u32, em: u32) -> StintSpan {
    StintSpan::whole(at(sh, sm), at(eh, em), false)
}
fn facts_for(prev: &[Punch], day_p: &[Punch], next: &[Punch], today: NaiveDate, now: DateTime<Utc>) -> DateAnomalies {
    date_facts(&DateBuckets { prev, day: day_p, next }, day(), today, now)
}
```

Punches are built through `punch_from_local` with `Local`, so the tests hold under any host timezone.

### AC1. Overlap detection (spec §3.2.1, §7 "§3.2" bullet)

Pure `overlap_pairs` tests:

- nested `09:00-12:00` and `10:00-11:00` gives one pair, `first` is the outer.
- identical `09:00-10:00` twice gives exactly one pair.
- nest sharing the start `09:00-12:00` and `09:00-11:00` gives one pair, outer first.
- nest sharing the end `09:00-12:00` and `10:00-12:00` gives one pair.
- touching `09:00-10:00` and `10:00-11:00` gives none.
- a zero-length `10:00-10:00` inside `09:00-12:00` gives none.
- deep nests: three nested spans give 3 pairs, four give 6. Assert the ordered list of `(first.start, second.start)`.
- ordering: three spans in shuffled input order give pairs sorted by first start, then second start.
- disjoint stints give none.

Fact-level tests through `date_facts` over punches:

- closed nest on a past date: start 09:00, start 10:00, end 11:00, end 12:00 gives `overlaps.len() == 1`, and `has_any()` is true.
- today's open stint containing a closed one: `date == today`, punches start 09:00, start 10:00, end 11:00 gives open stint 09:00 and closed 10:00-11:00. With `now` 12:00 it yields one pair whose `first.open` is true and whose label is `09:00-now`.
- the same punches dated a past date (`today` later): no overlap (open stint excluded).
- today's open stint with `now` before its start (live zero): start 14:00 open, closed 09:00-12:00, `now` 13:00 gives no overlap. `now` exactly at the start also gives none.
- today's open stint with two open starts (E7 today): only the latest open start enters the spans.
- spliced cross-midnight stint, start side: previous date has start 23:00 open, this date has first punch end 01:00 (one orphan, first punch). On the start date the overlap test sees `23:00` to the date's end, and a closed stint `22:30-23:30` on that date overlaps it. Assert the pair's `second.end` equals `local_midnight(next date)` and that `second.label()` is `23:00-01:00` (real times, not `23:00-00:00`).
- spliced stint, receiving side: on the next date the overlap test sees `00:00` to `01:00`, so a closed `00:30-00:45` on that date overlaps it. Assert the spliced span's `label()` is `23:00-01:00`, the same text the start date prints. The splice-consumed orphan does not also appear in `orphaned_end_times`.
- spliced stint `23:30-00:45` (start 23:30 on the date, end 00:45 on the next) containing a closed `23:40-23:50`: the `detail_lines` overlap line reads `[!] overlapping stints 23:30-00:45 and 23:40-23:50 (the overlap counts twice)`, matching the stint-list text pinned at `src/status.rs:1755-1762`.
- a splice that ends exactly at `00:00` gives a zero-length receiving part and no overlap.
- a refused join (two orphans, so no splice) adds no received span.

### AC2. Line text, `fix:` text and block order (spec §3.2.1)

- `detail_lines` for one nested pair equals exactly:
  `[!] overlapping stints 09:00-12:00 and 10:00-11:00 (the overlap counts twice)` and
  `    fix: mlm delete punch --date 2026-09-01  (lists punches; rerun with the number of the punch to remove)`.
- Open form: `[!] overlapping stints 09:00-now and 10:00-11:00 (the overlap counts twice)`.
- Three nested stints give three `[!]` lines, each followed by its `fix:` line, in pair order.
- Block order: a fixture with E7, stale, an orphaned end, an overlap and both refused joins at once (extending the idea of T1's `t_ordering_all_kinds_at_once`, which stays unchanged) lists line prefixes in the order E7, stale, E8, overlap, refused previous, refused next.
- Every overlap line is ASCII.

### AC3. Row marker, footer and status block (spec §3.2.1 last bullet, §3.3 predicate)

- `has_any()` is true for an overlap-only fact (`DateAnomalies { overlaps: vec![..], ..DateAnomalies::none(d) }`) and `row_marker()` is `"  [!]"`.
- Predicate sweep: over all fixtures from AC1, AC2 and T1's, `has_any() == !detail_lines().is_empty()`.
- Binary tests in `tests/overlap_cli.rs`, TZ pinned to `UTC`, raw-seeded punches on `2026-01-05` (Monday of week `2026-02`) in a temp database created by running one `mlm` command first (as `seed_punch` does in `tests/status_cli.rs`). Seed the helper locally in the new file, a copy of `seed_punch`/`mlm_tz`/`stdout`/`stderr`.
  - `mlm status 2026-01-05` stdout contains the two overlap lines of AC2 and exit is 0. Its day total stays the double-counted `04h 00m` figure (09:00-12:00 plus 10:00-11:00 is 240 minutes).
  - `mlm week 2026-02` contains the row `  Mon 2026-01-05   04h 00m  [!]`, contains `[!] needs a look: run mlm status DATE for details and fixes` exactly once, and `Worked:        04h 00m` (unchanged double count).
  - `mlm week 2026-02` for the same database with only touching stints (09:00-10:00, 10:00-11:00) has no `[!]` and no footer.

### AC4. Clean-pair rule on the stale-start line (spec §3.2.2)

- Stale, no later punch (start 09:00 only, age 5): `    fix: mlm stop HH:MM --date 2026-09-01  (a time after 09:00)`, byte-identical to today.
- Stale with later punches (start 09:00, start 10:00, end 10:30, so one open start 09:00 and N 10:00): `    fix: mlm stop HH:MM --date 2026-09-01  (a time between 09:00 and 10:00)`.
- N is the first later punch of any kind: punches 09:00 start, 11:00 start, 11:30 end, 10:00 start, 10:15 end (a closed 10:00-10:15, a closed 11:00-11:30) gives `between 09:00 and 10:00`.
- A later punch at S's own minute does not count: two starts at 09:00 (E7) with nothing after gives `a time after 09:00 closes the 09:00 start`.
- `latest_open_next` is set only when a start is open: a clean day gives `None`.
- Binary test: `mlm status 2026-01-05` with those punches shows the `between 09:00 and 10:00` line (the `unclosed start at 09:00, N days old` line carries a clock-dependent N, so assert on the `fix:` line only).

### AC5. Clean-pair rule on the E7 line (spec §3.2.2)

- No later punch: `     or: mlm stop HH:MM --date 2026-09-01  (a time after 14:00 closes the 14:00 start)`, byte-identical to today (the spec's existing E7 example).
- With a punch after the latest open start (09:00 start, 14:00 start, 15:00 start, 15:30 end): `     or: mlm stop HH:MM --date 2026-09-01  (a time between 14:00 and 15:00 closes the 14:00 start)`, the spec's example.
- Binary test of the same fixture through `mlm status 2026-01-05`.

### AC6. Follow-the-hint round trip, spec-review F1 guard (spec §3.2.2, changeset contract 7)

Unit tests re-run `date_facts` with the stop added and assert `overlaps.is_empty()`. Fixtures sit on a past date, because a still-open earlier start on today's date legitimately spans to `now` and may nest the closed stints (risk 2).

- Stale fixture (start 09:00, start 10:00, end 10:30). Add an end at 09:01, 09:30 and 10:00 (N's own minute). Each: no overlap, and the new facts name no stale or E7 anomaly on the stale part where the stop resolved it.
- E7 fixture (start 09:00, start 14:00, start 15:00, end 15:30). Add an end at 14:01, 14:30 and 15:00 (N's own minute). Each: no overlap.
- N one minute after S (start 18:02, start 18:03, end 18:30): window `between 18:02 and 18:03`. Adding an end at 18:03 gives no overlap.
- Counterexample pinned: for the E7 fixture, an end at 16:00 (after N) nests 14:00-16:00 around 15:00-15:30 and yields exactly one overlap pair. This documents why the window stops at N.
- Binary test: `mlm stop 10:00 --date 2026-01-05` on the stale fixture (TZ=UTC), then `mlm status 2026-01-05` has no `overlapping stints` and no `unclosed start`. Same for the E7 fixture with `mlm stop 15:00 --date 2026-01-05`. The `stop` call is the real CLI, so it also proves the printed hint is executable.

### AC7. Byte-identical output elsewhere (changeset plan global constraints)

- Every existing `DateAnomalies` fixture that has no overlap and no later punch after its open start renders the same lines as before. The moved T1 tests (`t_e7_reworded_matches_spec_example` and the stale, E8, refused-join and ordering tests) keep their assertions unchanged and pass.
- Binary test: the T1 golden fixture (`2026-01-05` start 09:00Z, end 17:00Z, `2026-01-06` start 09:00Z, end 12:30Z) gives `mlm week 2026-02` with no `[!]`, no footer and `Worked:        11h 30m`, and `mlm status 2026-01-05` with no anomaly lines.
- Past-week stint lines, notes, headline and `est. EOD` are not touched: nothing in this task edits those paths. The full existing suite is the guard (Step 4).

### AC8. Verification, complexity, coverage

- The full set from Step 6. Every new function is at most 6 in cognitive complexity.
- Every new branch has a test above, including both arms of `label`, both arms of `closed_span`'s `end.date > date`, `received_span`'s two returns, `live_span`'s three outcomes. `local_midnight`'s DST-gap fallback closure cannot run under a normal host timezone (risk 4).
- Revert any hook rewrite of `coverage-baseline.json`.

## Risks, ambiguities and disagreements

### Risks

1. **Existing fixtures that overlap by accident.** Adding the anomaly changes output for any existing test or fixture that has nested closed stints, or a today open stint containing a closed one. I found no test named for either, but could not prove none exists without running the suite. Such tests may live in T1's files (`src/status.rs`, `src/week_view.rs`, `src/render.rs`, `tests/status_cli.rs`) or T3's (`src/commands.rs`). Step 4 runs the suite. For a T1-file assertion my spec change inverts, the changeset plan's Global constraints exception lets me re-pin the expected value and nothing else, and I list each re-pin in my completion report. For a red test in any other task's file I stop and report it with its location.
2. **Round trip for E7 or stale starts on today.** For a date equal to today, an earlier open start renders as `S-now` and can still nest closed stints after the user adds a stop between two later starts. That is the open-stint rule of §3.2.1 working as written, not a bad hint. The round-trip tests use past dates, as the spec's follow-the-hint guard needs only the printed window.
3. **Same-instant tie in the window.** The helper requires strictly later `at_utc`, and punches are minute-granular (`storage.rs:100-102`), so "same minute as S" and "same instant as S" coincide. T1's frozen helper owns this.
4. **Coverage of the DST-gap fallback.** `local_midnight`'s `None` arm is reachable only in a zone where local midnight does not exist. A unit test cannot reach it without a timezone database, and I declined to add a dependency. The loss is one closure on one line.
5. **DST fall-back hour.** Spans are ordered and compared as UTC instants, so a repeated clock hour cannot mis-order two stints. Labels are local `HH:MM` and may show equal times for different instants in that hour. Accepted.
6. **Cross-midnight spans on `mlm week` rows.** Every row calls `build_facts`, so both dates of a spliced stint compute their own part. A part never double-flags across dates, because each date's spans hold only its own part.

### Ambiguities

- **A1. How a spliced stint is labelled.** Spec §3.2.1 fixes the compared range (the part on the date) and gives no label. The coordinator ruled (consistency review F1) that the label uses the stint's real start and end, as the status stint list prints it (`src/status.rs:213-231`, test at `:1755-1762`, `23:30-00:45`). The overlap test still uses only the part on the date. `StintSpan` carries `full_start` and `full_end` for the label, and both dates of a spliced stint print the same real range.
- **A2. One `fix:` line per pair or one per block.** §3.2.1 shows one `[!]` line plus one `fix:` line and says "each overlapping pair gets one line". I print the two-line group per pair, which matches how E8 repeats per orphan and keeps every `[!]` line followed by its fix. The changeset plan's "one line per overlapping pair" counts `[!]` lines, and the AC1 deep-nest test counts those.
- **A3. Which stint is `first` on a shared start.** The spec orders by first start, then second start, and says nothing about equal starts. I put the longer stint first so the outer stint reads first (`09:00-12:00 and 09:00-11:00`). Identical ranges are one pair either way.
- **A4. The open stint on today with two or more open starts.** Spec says "the most recent open stint taken up to now", which is `open.last()` and matches `live_minutes`. Earlier open starts do not enter the spans.
- **A5. `latest_open_next` shares one field for E7 and stale.** The stale fact needs no separate N, since a stale start is the only open start.

### Disagreements

- None with the spec or the changeset plan. One note on the changeset plan: its acceptance wording "sorted by first start then second start" is silent on equal starts, so A3 fills that gap.
- One note for T1: `build_facts` keeps `now_utc` unused in T1's plan (`_now_utc`). T2 uses it through the live span, which T1's published signature already allows.
