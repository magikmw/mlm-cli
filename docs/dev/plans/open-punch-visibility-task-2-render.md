# Task 2 plan: Anomaly rendering and fix-line wording

Low-level plan for Task 2 of changeset `open-punch-visibility`.

## What was read

- `docs/dev/plans/open-punch-visibility-plan.md` (full) — goal,
  architecture, global constraints, Contract A/B, task breakdown for
  Tasks 1-6.
- `docs/dev/specs/2026-09-26-open-punch-visibility.md` (full, locked)
  — terms (§2), behavior (§3.1-§3.7), docs list (§4), test plan (§5).
- `docs/dev/plans/` directory listing: no
  `open-punch-visibility-task-*.md` sibling exists yet except this one
  being written now. Task 1 (`stint.rs`), Task 3 (`status.rs`), Task 4
  (`commands.rs`), Task 5 (`week_view.rs`), Task 6 (docs) have no plan
  file yet — this task pins Contract A itself, per the changeset plan's
  own instruction ("pin before Task 1 and Task 2 start together").
- Current source: `src/render.rs` (full, 675 lines), `src/stint.rs`
  (full, 1135 lines), `src/status.rs` (full file length checked,
  targeted reads around the `Anomalies`/`to_anomalies` call site and
  the golden-test block ~1180-1793), `src/week_view.rs` (targeted read
  around its `Anomalies` construction, lines 130-169), `src/date.rs`
  (targeted reads: `format_date`, `format_date_with_weekday`,
  `format_weekday_full`, `weekday_abbrev`, plus their tests),
  `tests/status_cli.rs` (full, 250 lines).
- `clippy.toml` (cognitive-complexity-threshold = 15) and
  `.githooks/pre-commit` (fmt / complexity / coverage-ratchet gate,
  degrades to SKIP when tooling is missing).

## Scope

**This task owns, exclusively:**
- `src/render.rs`
- `tests/status_cli.rs`

**Must not touch (sibling ownership):**
- `src/stint.rs` — Task 1. Owns the two new predicates (staleness,
  refused-join) this task's callers (Task 3/5) will use to *compute*
  the facts this task's types carry. This task never imports
  `stint.rs` at all (Contract A: "Task 2 never calls into `stint.rs`
  directly; it only accepts facts about a date as plain data").
- `src/status.rs` — Task 3. Builds `render::DateAnomalies` /
  `render::LookbackFinding` instances from real `DayStints` data and
  calls this task's rendering methods. Its own ~15 hardcoded golden
  tests asserting the *old* E7/E8 text (e.g. lines ~1180-1793,
  `"[!] 3 open stints for this date (unmatched starts)"`,
  `anomaly_lines: vec!["[!] orphaned end at 18:00 ..."]`) are **not**
  updated by this task — they live in Task 3's file. See "Handoff to
  Task 3/5" below; this is flagged as a risk, not silently absorbed.
- `src/commands.rs` — Task 4. Calls Task 3's lookback and this task's
  `LookbackFinding::stderr_line()`.
- `src/week_view.rs` — Task 5. Decides *which* dates get `[!]` and
  *whether* the footer is appended; this task only supplies the footer
  string and the per-date line text.
- `docs/dev/SPEC.md`, `README.md` — Task 6.

No code in the working tree is edited to produce this plan file, and
nothing is committed.

## Published interface (Contract A)

This section is authoritative for Task 3 and Task 5's field/method
names, since neither of those tasks' plans exist yet. Every output
string is quoted verbatim.

### Replacing `Anomalies`

The existing `render::Anomalies` (two fields: `open_stint_count`,
`orphaned_end_times`) is **replaced**, not extended in place, by a new
type carrying every fact one date's anomaly block needs. `Anomalies`'s
only two call sites today (`status.rs::to_anomalies`,
`week_view.rs`'s per-row construction, both literal-field struct
constructions) will fail to compile against the new type — this is
expected: Task 3 and Task 5 both already have "construct the finished
shape" as a stated acceptance-criterion dependency in the changeset
plan (Contract B). See "Risks" for why an in-place field-add was
rejected instead.

```rust
/// Everything one calendar date's `status`/`week` anomaly rendering
/// needs, as plain data (SPEC.md §3.1/§3.2/§3.4). Callers (Task 1's
/// predicates via Task 3/5) compute every fact; this type never reads
/// a clock, a database, or `stint.rs` itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateAnomalies {
    /// The date this block belongs to. Fills every `D` in this date's
    /// own `fix:`/`or:` lines (§3.4).
    pub date: NaiveDate,

    /// How many trailing unmatched starts this date has. 0 or 1 is
    /// normal; >= 2 is E7. Unchanged meaning from the old `Anomalies`.
    pub open_stint_count: usize,

    /// The most recently opened start's local time, when
    /// `open_stint_count >= 1`. Only consumed when `open_stint_count
    /// > 1` (E7's fix line names it, e.g. "14:00" in the spec's worked
    /// example: "the latest open start on the date"). `None` iff
    /// `open_stint_count == 0`.
    pub latest_open_start: Option<NaiveTime>,

    /// One entry per orphaned end (E8), ascending by instant. Same
    /// contract as the old `Anomalies::orphaned_end_times` — never
    /// deduplicated, never coalesced.
    pub orphaned_end_times: Vec<NaiveTime>,

    /// Set iff this date has exactly one open start AND it is stale
    /// (age >= 2, Task 1's predicate). Never set when
    /// `open_stint_count != 1` (E7 owns the 2+ case entirely, §3.1).
    pub stale_open_start: Option<StaleOpenStart>,

    /// Set to the previous calendar date iff this date and its
    /// predecessor form a refused join with THIS date as the later
    /// (orphaned-end) side (Task 1's predicate, §3.2).
    pub refused_join_with_previous: Option<NaiveDate>,

    /// Set to the next calendar date iff this date and its successor
    /// form a refused join with THIS date as the earlier
    /// (open-start) side (§3.2).
    pub refused_join_with_next: Option<NaiveDate>,
}

/// The one stale-open-start fact a date can carry (§3.1/§3.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaleOpenStart {
    /// Local time of the (only) open start.
    pub time: NaiveTime,
    /// `today - date` in whole days. Always >= 2 (Task 1's predicate
    /// contract) — callers never construct this with a smaller value.
    pub age_days: i64,
}

impl DateAnomalies {
    /// True iff this date has ANY anomaly under the new, wider
    /// definition: E7, E8, stale, or either refused-join direction.
    /// The single definition `week`'s per-row `[!]` (Task 5) and
    /// `status`'s block-or-not decision (Task 3) both key off.
    pub fn has_any(&self) -> bool { ... }

    /// `status`'s per-date block (§3.4), in the fixed order: E7, then
    /// stale, then E8 (ascending instant), then refused-join-with-
    /// previous, then refused-join-with-next. Empty vec when
    /// `has_any()` is false. Does NOT include lookback lines — those
    /// come from separate `LookbackFinding`s, appended by the caller
    /// (Task 3), never by this type.
    pub fn detail_lines(&self) -> Vec<String> { ... }

    /// `week`'s per-row suffix (§3.5, §3.4's "row gets `[!]`" rule,
    /// unchanged text/shape from the old `Anomalies::row_marker`):
    /// `"  [!]"` when `has_any()`, `""` otherwise.
    pub fn row_marker(&self) -> &'static str { ... }
}
```

### Lookback finding (separate, smaller shape — Contract A)

```rust
/// One window date's finding from the §3.3/§3.6 14-day lookback.
/// Deliberately narrower than `DateAnomalies`: a lookback line never
/// carries E8/refused-join detail (§1.2's residual-gap bullets), only
/// "this window date has an open start" in one of two shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LookbackFinding {
    /// The window date (age 2-14) this finding is about.
    pub date: NaiveDate,
    /// `today - date`. Always in 2..=14 (Task 3's scan contract).
    pub age_days: i64,
    pub kind: LookbackKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookbackKind {
    /// Exactly one open start on the window date.
    SingleOpenStart { time: NaiveTime },
    /// Two or more open starts on the window date (that date's own E7).
    MultipleOpenStints { count: usize },
}

impl LookbackFinding {
    /// `status`'s block form (§3.3, one finding per line-group,
    /// appended after a date's own `DateAnomalies::detail_lines()`,
    /// oldest window date first — Task 3's job to order/append).
    pub fn detail_lines(&self) -> Vec<String> { ... }

    /// `start`'s compressed one-line stderr form (§3.6), no trailing
    /// newline.
    pub fn stderr_line(&self) -> String { ... }
}
```

### `week`'s footer (§3.5)

```rust
/// The exact, unparameterized §3.5 footer line. Task 5 decides
/// whether to append it (at least one row has `[!]`) and where (after
/// the table, after one blank line) — this constant is the only thing
/// this task owns.
pub const WEEK_ANOMALY_FOOTER: &str =
    "[!] needs a look: run mlm status DATE for details and fixes";
```

### Exact literal strings (copied verbatim from spec §3.4/§3.6, `D` =
`crate::date::format_date(date)`, `Wdy D` = `crate::date::format_date_with_weekday(date)`, `HH:MM` literal placeholder unchanged)

**E7** (`open_stint_count = count >= 2`, `latest` = `latest_open_start` as `%H:%M`):
```
[!] {count} open stints for this date (only one can be open at a time)
    fix: mlm delete punch --date {D}  (lists punches; rerun with the extra start's number)
     or: mlm stop HH:MM --date {D}  (a time after {latest} closes the {latest} start)
```

**Stale open start** (`time`, `age_days` from `StaleOpenStart`):
```
[!] unclosed start at {time}, {age_days} days old (no stop recorded)
    fix: mlm stop HH:MM --date {D}  (a time after {time})
     or: mlm delete punch --date {D}  (lists punches; rerun with the start's number)
```

**E8** (one block per orphan, `time` = each `orphaned_end_times[i]`, first line text unchanged from today):
```
[!] orphaned end at {time} (no matching start)
    fix: mlm start HH:MM --date {D}  (a time before {time})
     or: mlm delete punch --date {D}  (lists punches; rerun with the end's number)
```

**Refused join** (no `fix:`/`or:` lines). Given `(earlier, later)` = the
chronologically earlier/later of the two dates in the pair, and
`named` = whichever of `{prev, next}` is NOT `self.date`:
```
[!] not joined to {Wdy named} across midnight (a join needs exactly one open start on {Wdy earlier} and exactly one unmatched end, as {Wdy later}'s first punch)
```
- When rendering `refused_join_with_previous = Some(prev)`: `named = prev`, `earlier = prev`, `later = self.date`.
- When rendering `refused_join_with_next = Some(next)`: `named = next`, `earlier = self.date`, `later = next`.
- Order: the previous-neighbor line (if present) always precedes the next-neighbor line (if present) — a date can carry both.

**Lookback, single open start** (block form):
```
[!] unclosed start on {Wdy D} at {time} ({age_days} days ago)
    fix: mlm stop HH:MM --date {D}  (a time after {time})
     or: mlm delete punch --date {D}  (lists punches; rerun with the start's number)
```

**Lookback, 2+ open stints** (block form — note: only ONE `fix:` line, no `or:` line):
```
[!] {count} open stints on {Wdy D} ({age_days} days ago)
    fix: see mlm status {D}
```

**Lookback, single open start** (stderr form — single space before the parenthetical, unlike the block form's double space; no `or:` content at all):
```
warning: unclosed start on {Wdy D} at {time} ({age_days} days ago); fix: mlm stop HH:MM --date {D} (a time after {time})
```

**Lookback, 2+ open stints** (stderr form — no `fix:` word):
```
warning: {count} open stints on {Wdy D} ({age_days} days ago); see mlm status {D}
```

**`week` footer** (constant, no interpolation):
```
[!] needs a look: run mlm status DATE for details and fixes
```

Indentation is exact and load-bearing: `fix:` lines are 4 spaces then
`fix: `; `or:` lines are 5 spaces then `or: ` (so both texts start in
the same column, verified byte-for-byte against the spec's fenced
blocks via `cat -A`/`repr()` during this planning pass — no line
wraps, no trailing whitespace). The double space between a `--date D`
value and its following parenthetical is exact in every block-form
line; the stderr forms use a single space instead — confirmed by
diffing the two spec examples directly (`sed -n | cat -A`).

Minimum age for both `StaleOpenStart.age_days` and
`LookbackFinding.age_days` is 2 by the callers' own contract (stale
requires age >= 2; the lookback window is ages 2-14), so no plural/
singular "day"/"days" branch is needed anywhere in this task — always
literal `"days"`. A `debug_assert!(age_days >= 2, ...)` is added at
the top of each formatting path that consumes an age, as a caller-
contract tripwire, matching this codebase's existing `debug_assert!`
style in `stint.rs`'s `classify_at`.

## Dependencies

| Dependency | Status | What Task 2 does about it |
|---|---|---|
| Task 1's stale/refused-join predicates (`stint.rs`) | Not landed | Not needed — this task never calls `stint.rs`. `StaleOpenStart`/`refused_join_with_*` are plain fields Task 3/5 populate from Task 1's predicates once those land. No stub needed since this task supplies no logic that depends on them, only the shape they get poured into. |
| Task 3's `status.rs` wiring (`to_anomalies`, lookback scan) | Not landed | Out of scope. This task's own tests construct `DateAnomalies`/`LookbackFinding` values by hand (unit tests, same style as the current `render.rs` test module), never via `status.rs`. The one integration test this task edits (`tests/status_cli.rs` line 83) exercises the real binary and therefore transitively depends on Task 3 rewiring `status.rs` to use the new type before it can pass — see "Risks". |
| Task 5's `week_view.rs` wiring | Not landed | Out of scope; `WEEK_ANOMALY_FOOTER` and `row_marker()` are ready for it to consume as-is. |

## Implementation plan

### 1. `src/render.rs`

1.1. Add `use crate::date::{format_date, format_date_with_weekday, format_weekday_full};`
     (currently only `format_weekday_full` is imported — confirmed
     line 18 of the current file).

1.2. Delete the current `Anomalies` struct and its `impl` block
     (current lines 119-168), replacing with `DateAnomalies`,
     `StaleOpenStart`, `LookbackFinding`, `LookbackKind`, and
     `WEEK_ANOMALY_FOOTER` as specified above.

1.3. Implement `DateAnomalies::has_any`:
```rust
pub fn has_any(&self) -> bool {
    self.open_stint_count > 1
        || !self.orphaned_end_times.is_empty()
        || self.stale_open_start.is_some()
        || self.refused_join_with_previous.is_some()
        || self.refused_join_with_next.is_some()
}
```

1.4. Implement `detail_lines` as a thin assembler over five private
     helpers, keeping cognitive complexity low (budget 15; each helper
     below is a single early-return-free straight line or a short
     loop, well under threshold):

```rust
pub fn detail_lines(&self) -> Vec<String> {
    let mut out = Vec::new();
    out.extend(self.e7_lines());
    out.extend(self.stale_lines());
    out.extend(self.e8_lines());
    out.extend(self.refused_join_lines());
    out
}

fn e7_lines(&self) -> Vec<String> {
    let Some(latest) = self.latest_open_start.filter(|_| self.open_stint_count > 1) else {
        return Vec::new();
    };
    let d = format_date(self.date);
    let latest = latest.format("%H:%M");
    vec![
        format!(
            "[!] {} open stints for this date (only one can be open at a time)",
            self.open_stint_count
        ),
        format!(
            "    fix: mlm delete punch --date {d}  (lists punches; rerun with the extra start's number)"
        ),
        format!("     or: mlm stop HH:MM --date {d}  (a time after {latest} closes the {latest} start)"),
    ]
}

fn stale_lines(&self) -> Vec<String> {
    let Some(stale) = self.stale_open_start else {
        return Vec::new();
    };
    debug_assert!(stale.age_days >= 2, "stale open start must be age >= 2");
    let d = format_date(self.date);
    let time = stale.time.format("%H:%M");
    vec![
        format!("[!] unclosed start at {time}, {} days old (no stop recorded)", stale.age_days),
        format!("    fix: mlm stop HH:MM --date {d}  (a time after {time})"),
        format!("     or: mlm delete punch --date {d}  (lists punches; rerun with the start's number)"),
    ]
}

fn e8_lines(&self) -> Vec<String> {
    let d = format_date(self.date);
    let mut out = Vec::new();
    for t in &self.orphaned_end_times {
        let time = t.format("%H:%M");
        out.push(format!("[!] orphaned end at {time} (no matching start)"));
        out.push(format!("    fix: mlm start HH:MM --date {d}  (a time before {time})"));
        out.push(format!("     or: mlm delete punch --date {d}  (lists punches; rerun with the end's number)"));
    }
    out
}

fn refused_join_lines(&self) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(prev) = self.refused_join_with_previous {
        out.push(refused_join_line(prev, prev, self.date));
    }
    if let Some(next) = self.refused_join_with_next {
        out.push(refused_join_line(next, self.date, next));
    }
    out
}
```

(free function, not a method, since it needs no `&self` beyond the
three dates already passed in — keeps it trivially testable alone):
```rust
fn refused_join_line(named: NaiveDate, earlier: NaiveDate, later: NaiveDate) -> String {
    format!(
        "[!] not joined to {} across midnight (a join needs exactly one open start on {} and exactly one unmatched end, as {}'s first punch)",
        format_date_with_weekday(named),
        format_date_with_weekday(earlier),
        format_date_with_weekday(later),
    )
}
```

1.5. `row_marker` unchanged in body from today's `Anomalies` version,
     just moved onto `DateAnomalies` and now keying off the wider
     `has_any()`.

1.6. `LookbackFinding` methods:

```rust
impl LookbackFinding {
    pub fn detail_lines(&self) -> Vec<String> {
        debug_assert!((2..=14).contains(&self.age_days), "lookback age must be 2..=14");
        let d = format_date(self.date);
        let wdy = format_date_with_weekday(self.date);
        match self.kind {
            LookbackKind::SingleOpenStart { time } => {
                let time = time.format("%H:%M");
                vec![
                    format!("[!] unclosed start on {wdy} at {time} ({} days ago)", self.age_days),
                    format!("    fix: mlm stop HH:MM --date {d}  (a time after {time})"),
                    format!("     or: mlm delete punch --date {d}  (lists punches; rerun with the start's number)"),
                ]
            }
            LookbackKind::MultipleOpenStints { count } => vec![
                format!("[!] {count} open stints on {wdy} ({} days ago)", self.age_days),
                format!("    fix: see mlm status {d}"),
            ],
        }
    }

    pub fn stderr_line(&self) -> String {
        debug_assert!((2..=14).contains(&self.age_days), "lookback age must be 2..=14");
        let d = format_date(self.date);
        let wdy = format_date_with_weekday(self.date);
        match self.kind {
            LookbackKind::SingleOpenStart { time } => {
                let time = time.format("%H:%M");
                format!(
                    "warning: unclosed start on {wdy} at {time} ({} days ago); fix: mlm stop HH:MM --date {d} (a time after {time})",
                    self.age_days
                )
            }
            LookbackKind::MultipleOpenStints { count } => format!(
                "warning: {count} open stints on {wdy} ({} days ago); see mlm status {d}",
                self.age_days
            ),
        }
    }
}
```

1.7. Keep `#![allow(dead_code)]` at module top (already present, still
     needed: `DateAnomalies`/`LookbackFinding` have no in-crate caller
     until Task 3/5 land, same situation Milestone 9 was in
     originally).

1.8. Update the module's test block:
   - Delete/replace tests 4.8-4.12 (`t16`-`t33`, the ones built against
     the old two-field `Anomalies`) with equivalents against
     `DateAnomalies`. Where a test's *point* survives unchanged
     (multi-open-then-orphan ordering, no-divergence invariants,
     ASCII sweep), keep its name and intent, just retarget the
     construction and the expected strings.
   - Add new tests per the acceptance criteria (see Test plan below).

### 2. `tests/status_cli.rs`

2.1. Update `t14_status_with_anomalies_exits_zero_and_prints_to_stdout`
     (current line 83) from:
     ```rust
     out.contains("[!] 2 open stints for this date (unmatched starts)")
     ```
     to the new E7 header line:
     ```rust
     out.contains("[!] 2 open stints for this date (only one can be open at a time)")
     ```
     Consider also asserting the two new fix/or lines are present
     (`out.contains("fix: mlm delete punch --date")` and
     `out.contains("or: mlm stop HH:MM --date")`), since this is the
     one binary-level test this changeset has for E7 and the whole
     point of the changeset is that fix lines now exist. This is a
     judgment call within this task's owned file, not a spec
     requirement of exactly this test.

2.2. No other existing assertion in this file names anomaly wording
     (confirmed by grep across the file — only line 83 matches
     `"open stints"`/`"orphaned end"`/`"unmatched starts"`).

## Handoff to Task 3 / Task 5

- Task 3 must replace `status.rs::to_anomalies` (current lines
  311-328) with a builder that populates every new `DateAnomalies`
  field from `DayStints` plus Task 1's two predicates plus each
  neighbor's own `DayStints`, and must add the lookback scan producing
  `Vec<LookbackFinding>`, appended (as `detail_lines()`) after the
  date's own block, only for today.
- Task 3 must also update its own hardcoded golden-test fixtures that
  assert old E7/E8 text (grep hits in `src/status.rs` at approximately
  lines 1180, 1185, 1215, 1234-1235, 1246-1247, 1436, 1631, 1793) —
  these are outside this task's file ownership and are **not** touched
  here. Flagged as a required follow-up so "no test elsewhere is left
  asserting the old E7/E8 text" (this task's own acceptance criterion,
  read literally, is broader than this task's file scope) actually
  becomes true changeset-wide.
- Task 5 must replace its per-row `Anomalies { open_stint_count, ... }`
  construction (`week_view.rs` lines ~152-159) with `DateAnomalies`,
  add the neighbor-fetch this needs (already flagged in the changeset
  plan's Task 5 section), and append `WEEK_ANOMALY_FOOTER` once, after
  a blank line, iff any row's `has_any()` is true.
- Until Task 3 and Task 5 both land, `cargo build` for the whole crate
  fails (see Risks). This is an expected, temporary state of this
  changeset's parallel-worktree structure, not a defect in this task.

## Test plan

One subsection per this task's acceptance criterion (changeset plan's
Task 2 section), each citing the spec case it proves.

### AC1 — every §3.4 line shape renders verbatim

- `t_e7_reworded_matches_spec_example`: `DateAnomalies` with
  `open_stint_count: 2`, `latest_open_start: Some(14:00)` ->
  `detail_lines()` equals the three-line block quoted in spec §3.4
  E7, byte for byte.
- `t_stale_matches_spec_example`: `stale_open_start: Some(StaleOpenStart { time: 09:00, age_days: 5 })`,
  `open_stint_count: 1` -> matches spec §3.4's stale block.
- `t_e8_gains_fix_lines_text_unchanged`: `orphaned_end_times: vec![18:00]`
  -> first line identical to today's text, plus the two new fix/or
  lines, matching spec §3.4 E8 block.
- `t_refused_join_previous_side` / `t_refused_join_next_side`: a date
  with only `refused_join_with_previous`/`_next` set renders the
  correct headline-neighbor + shared parenthetical, matching spec
  §3.4's two example lines (`"not joined to Tue 2026-09-22..."` and
  the mirrored `"not joined to Mon 2026-09-21..."`).
- `t_lookback_single_block_matches_spec_example`,
  `t_lookback_multi_block_matches_spec_example`: `LookbackFinding::detail_lines()`
  for both `LookbackKind` variants, matching spec §3.3/§3.4's two
  lookback blocks exactly.
- Spec case: §5 "`fix:`/`or:` lines carry the real date for every
  anomaly kind" — covered by using a non-trivial, distinct date in
  every fixture above (not a hand-wavy placeholder), asserting the
  real `D` appears in every `--date` occurrence.

### AC2 — line ordering inside one date's block

- `t_ordering_all_kinds_at_once`: one `DateAnomalies` with
  `open_stint_count: 2` (+ `latest_open_start`), a non-`None`
  `stale_open_start` — note this combination is impossible under
  Task 1's real predicate (stale only fires at `open_stint_count ==
  1`) but `render.rs` itself enforces no such invariant (it only
  renders what it's given, per Contract A: "Task 2 never calls into
  `stint.rs`"), so the ordering test is free to supply an
  unrealistic-but-well-formed combination purely to pin sequence —
  two orphans, and both refused-join directions set. Asserts
  `detail_lines()` returns lines in exactly the order: E7 (3 lines),
  stale (3 lines), E8 orphan 1 (3 lines), E8 orphan 2 (3 lines),
  refused-join-previous (1 line), refused-join-next (1 line). Matches
  spec §3.4's fixed order 1-4 (lookback, order-position 5, is a
  separate type entirely and therefore not assembled by this method —
  Task 3 appends it after, per Contract A).
- `t_lookback_ordering_oldest_first`: a `Vec<LookbackFinding>` in
  caller-supplied order, each rendered via `detail_lines()` and
  concatenated by the test itself (mirroring what Task 3 will do),
  asserting the overall block reads oldest-window-date first when the
  input vec is already oldest-first — this only proves rendering
  preserves caller order; the actual "oldest first" guarantee is Task
  3's scan, out of this task's scope, and the test's docstring says so.

### AC3 — `week`'s footer wording

- `t_week_footer_matches_spec_example`: `WEEK_ANOMALY_FOOTER ==
  "[!] needs a look: run mlm status DATE for details and fixes"`.

### AC4 — `start`-stderr lookback rendering

- `t_lookback_stderr_single_matches_spec_example`,
  `t_lookback_stderr_multi_matches_spec_example`: `LookbackFinding::stderr_line()`
  for both variants, matching spec §3.6's two example lines exactly,
  including the single-space-before-parenthetical distinction from the
  block form (asserted by a companion test that constructs the *same*
  `LookbackFinding` and checks `detail_lines()[1]` and `stderr_line()`
  differ exactly in that one respect, so a future accidental unify
  regresses loudly).

### AC5 — no test left asserting old E7/E8 text (within this task's file)

- Grep-equivalent assertion at the end of the `render.rs` test module:
  a single test that greps every literal produced by the module's own
  fixtures for the strings `"(unmatched starts)"` (old E7 suffix) and
  confirms zero matches, guarding against a stray leftover.
- `tests/status_cli.rs` line 83 updated per Implementation §2.1 — see
  Handoff section for why the equivalent status.rs golden tests are
  explicitly out of scope, not silently skipped.

### AC6 — every new literal is plain ASCII

- Extend the existing `t33_every_produced_string_is_ascii`-style sweep
  (module's existing plain-ASCII test) to include: an E7 block, a
  stale block, an E8 block, both refused-join lines, both lookback
  block variants, both lookback stderr variants, and
  `WEEK_ANOMALY_FOOTER`. One assertion loop over all of them, matching
  spec §3.7 / changeset plan's "extends the relevant existing golden
  test rather than adding a parallel check" constraint.

## Risks, ambiguities, and disagreement with the plan/spec

1. **Replacing `Anomalies` outright breaks `cargo build` until Task
   3/5 land.** The changeset plan's own ordering diagram already
   sequences Task 3/5 strictly after Task 1+2, so this is expected,
   but the plan doesn't say *how* broken the intermediate state is
   allowed to be. An in-place field-add (keeping `Anomalies`'s name,
   adding new `Option` fields with sensible defaults via a `Default`
   impl) would have kept `status.rs`/`week_view.rs` compiling
   unchanged in the interim, at the cost of a stale/misleading
   `has_any()`/`detail_lines()` on a type that no longer matches its
   own doc comment. I chose the breaking rename because (a) the
   architecture prose explicitly sanctions it ("its `Anomalies` type
   (or whatever replaces it)"), and (b) a compile error is a much
   louder, more precise signal to Task 3/5's authors about exactly
   what needs wiring than a type that compiles but silently renders
   incomplete anomaly blocks. Flagging this choice explicitly in case
   the coordinator wants the non-breaking path instead for CI-green-at-
   every-commit reasons.

2. **Contract A (as written in the changeset plan) never mentions
   `latest_open_start`**, but spec §3.4's E7 fix line requires a real
   open-start time ("14:00 is the latest open start on the date" —
   stated as a filled-in value, not a literal placeholder like
   `HH:MM`). I added the field because the wording is unrenderable
   without it. This is the one place this task's "Published interface"
   materially extends what the changeset plan's Contract A bullets
   literally enumerate; Task 1/3's authors should treat this file's
   Contract A section as the actual source of truth over the
   changeset plan's summary, per this task's own instructions.

3. **The ordering test in AC2 needs an unrealistic input** (stale +
   E7 simultaneously) to pin sequence, since `render.rs` accepts
   whatever `DateAnomalies` it's given and enforces no cross-field
   invariant. This is intentional (pure rendering layer, no validation
   layer) but worth flagging: a real bug in Task 3's builder that sets
   both `open_stint_count > 1` and `stale_open_start: Some(_)`
   simultaneously would render silently instead of panicking. Given
   the plan's "no automatic repair, no exit-code change" spirit, I
   think silent-render-whatever-you're-given is the right call for
   this layer, but it does mean the stale/E7 mutual-exclusion
   invariant from spec §3.1 lives entirely in Task 1's predicate and
   Task 3's builder, with no defense-in-depth here.

4. **`tests/status_cli.rs`'s updated assertion cannot be run to a
   passing result by this task alone** — it exercises the real
   compiled binary, which needs Task 3's `status.rs` rewiring to even
   compile, let alone produce the new wording. I've written the exact
   expected string in Implementation §2.1 so Task 3's author (or
   whoever integrates last) can verify it mechanically rather than
   re-deriving it.

5. **Minor spec ambiguity, not blocking**: spec §3.4 shows the E7
   `or:` line as `"a time after 14:00 closes the 14:00 start"` —
   the same time-value appears twice in one sentence. I render it that
   way literally (both interpolations read `latest_open_start`); it
   reads a little redundant in prose but matches the spec's own
   worked example exactly, and this task's job is verbatim rendering,
   not prose-polishing the spec.
