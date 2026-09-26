# Task 2 plan: Anomaly rendering and fix-line wording

Low-level plan for Task 2 of changeset `open-punch-visibility`.

**Re-aligned** to the changeset plan's phase-9 update (NOTES.md entry
79): Task 2 is now additive (adds `DateAnomalies` beside the existing
`render::Anomalies`, deletes nothing — Task 4 deletes `Anomalies`
later), no longer owns `tests/status_cli.rs` (Task 3 does), and this
pass also checks the Published interface against Task 1's pinned
`stint::StaleOpenStart` for a name collision (see "Name collision with
Task 1"). No published name changed and no scope was added.

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
- `src/render.rs` (batch 1 — additive only; see "Additive, not
  replacing" below)

**No longer owned by this task:** `tests/status_cli.rs`. It moves to
Task 3, since that is where the new E7 wording first reaches the
binary (`status.rs` is what switches to the new type). See "Handoff to
Task 3" below for the exact assertion text Task 3 needs.

**Must not touch (sibling ownership):**
- `src/stint.rs` — Task 1. Owns the two new predicates (staleness,
  refused-join) this task's callers (Task 3/5) will use to *compute*
  the facts this task's types carry. This task never imports
  `stint.rs` at all (Contract A: "Task 2 never calls into `stint.rs`
  directly; it only accepts facts about a date as plain data"). See
  "Name collision with Task 1" below — Task 1 also pins a type named
  `StaleOpenStart`, in its own module.
- `src/status.rs`, `tests/status_cli.rs` — Task 3. Builds
  `render::DateAnomalies` / `render::LookbackFinding` instances from
  real `DayStints` data, switches `status` over to the new type, and
  calls this task's rendering methods. Its own ~15 hardcoded golden
  tests asserting the *old* E7/E8 text (e.g. lines ~1180-1793,
  `"[!] 3 open stints for this date (unmatched starts)"`,
  `anomaly_lines: vec!["[!] orphaned end at 18:00 ..."]`) are **not**
  updated by this task — they live in Task 3's file. See "Handoff to
  Task 3" below.
- `src/commands.rs` — Task 4. Calls Task 3's lookback and this task's
  `LookbackFinding::stderr_line()`. Also, sequentially after Tasks 3
  and 5 have both merged, Task 4 deletes the old `render::Anomalies`
  type and its now-unused functions from `src/render.rs` — see
  "Additive, not replacing" below. This task does not delete anything.
- `src/week_view.rs` — Task 5. Decides *which* dates get `[!]` and
  *whether* the footer is appended; this task only supplies the footer
  string and the per-date line text.
- `docs/dev/SPEC.md`, `README.md` — Task 6.

No code in the working tree is edited to produce this plan file, and
nothing is committed.

## Additive, not replacing

Earlier drafting of this plan had this task delete the existing
`render::Anomalies` and replace it in place with `DateAnomalies`. That
would leave `status.rs`/`week_view.rs` failing to compile from this
task's commit until Task 3 and Task 5 land, which breaks `cargo build`
and the pre-commit test gate on Task 2's own merge. The changeset plan
now requires Task 2 to merge clean on its own, so that approach is
dropped.

Task 2 instead adds `DateAnomalies`, `StaleOpenStart`, `LookbackFinding`,
`LookbackKind`, and `WEEK_ANOMALY_FOOTER` *beside* the existing
`Anomalies` struct and its `impl` block. Nothing existing is deleted,
renamed, or changed in behavior:

- `render::Anomalies` (both fields, both methods) stays exactly as it
  is today, with its existing tests unchanged and passing.
- The new types have no in-crate caller yet (same as before), so
  `#![allow(dead_code)]` at the module top keeps rustc/clippy quiet
  — already present, no change needed.
- `cargo build`, `cargo test`, and `cargo clippy --all-targets -- -D
  warnings` are all green after this task's commit, on its own,
  without Task 3 or Task 5 having landed.

Deleting the old `Anomalies` type is explicitly **Task 4's job**, once
Task 3 and Task 5 have both switched their call sites to
`DateAnomalies` and nothing references the old type anymore (see the
parent plan's Task 4 "Files owned exclusively," which now names
`src/render.rs` for this one deletion step, sequenced after Tasks 3
and 5). This task does not delete `Anomalies` and does not need to
reason about when that deletion is safe.

## Name collision with Task 1

Task 1's plan (`open-punch-visibility-task-1-stint-predicates.md`)
pins `pub(crate) struct StaleOpenStart` in `src/stint.rs`, with fields
`at: DateTime<Utc>, age_days: i64`. This task pins a *different*
`pub struct StaleOpenStart` in `src/render.rs`, with fields
`time: NaiveTime, age_days: i64`. Same name, different module,
different field shapes.

This is not a Rust compile collision — the two types live in separate
modules (`stint::StaleOpenStart` vs. `render::StaleOpenStart`) and
Task 2 never imports `stint.rs` (Contract A), so nothing here forces a
rename. Per this changeset's instruction to keep already-pinned
Published-interface names stable for Tasks 3/4/5, **neither type is
renamed**. Resolution:

- `stint::StaleOpenStart` (Task 1) is authoritative for the
  predicate's raw output — a UTC instant plus age, matching the rest
  of `stint.rs`'s existing `DateTime<Utc>`-based fields.
- `render::StaleOpenStart` (this task) is authoritative for the
  render-ready fact — a local `NaiveTime` plus age, matching every
  other time field this task's types carry (`latest_open_start`,
  `LookbackKind::SingleOpenStart { time }`), since `render.rs` never
  formats a UTC instant directly.
- Task 3's builder is where one becomes the other: it reads
  `stint::stale_open_start(...)`'s `Option<stint::StaleOpenStart>` and,
  when `Some`, constructs a `render::StaleOpenStart { time: <local
  time derived from `.at`>, age_days: <copied> }`. Task 3's own plan
  should spell out the UTC-to-local conversion it uses (matching
  however the rest of `status.rs` already renders a punch's local
  time); this task does not perform that conversion since it never
  sees a `DateTime<Utc>` at all.
- Both plans refer to their own module's type by its bare name
  (`StaleOpenStart`) throughout; readers should track which file's
  section they're in rather than assume a single global type.

## Published interface (Contract A)

This section is authoritative for Task 3 and Task 5's field/method
names, since neither of those tasks' plans exist yet. Every output
string is quoted verbatim.

### Adding `DateAnomalies` beside `Anomalies`

The existing `render::Anomalies` (two fields: `open_stint_count`,
`orphaned_end_times`) is left exactly as it is — same fields, same
methods, same tests, still called by `status.rs::to_anomalies` and
`week_view.rs`'s per-row construction. This task adds a new, separate
type, `DateAnomalies`, carrying every fact one date's anomaly block
needs under the new, wider behavior. Nothing about `Anomalies` changes
until Task 3 and Task 5 each switch their own call site over to
`DateAnomalies` (Contract B); Task 4 then deletes `Anomalies` once both
have (see "Additive, not replacing" above).

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
| Task 1's stale/refused-join predicates (`stint.rs`) | Not landed | Not needed — this task never calls `stint.rs`. `render::StaleOpenStart`/`refused_join_with_*` are plain fields Task 3/5 populate from Task 1's predicates once those land. No stub needed since this task supplies no logic that depends on them, only the shape they get poured into. |
| Task 3's `status.rs` wiring (`to_anomalies`, lookback scan, `tests/status_cli.rs`) | Not landed | Out of scope, and no longer this task's file. This task's own tests construct `DateAnomalies`/`LookbackFinding` values by hand (unit tests, same style as the current `render.rs` test module), never via `status.rs`. `DateAnomalies`/`LookbackFinding` have no in-crate caller until Task 3/5 land, same as `Anomalies` had none of its own callers before its original milestone — `#![allow(dead_code)]` covers this, so this task's own `cargo build`/`test`/`clippy` stay green regardless of Task 3's status. |
| Task 5's `week_view.rs` wiring | Not landed | Out of scope; `WEEK_ANOMALY_FOOTER` and `row_marker()` are ready for it to consume as-is. |

## Implementation plan

### 1. `src/render.rs`

1.1. Add `use crate::date::{format_date, format_date_with_weekday, format_weekday_full};`
     (currently only `format_weekday_full` is imported — confirmed
     line 18 of the current file).

1.2. Add `DateAnomalies`, `StaleOpenStart`, `LookbackFinding`,
     `LookbackKind`, and `WEEK_ANOMALY_FOOTER` as specified above,
     placed directly after the existing `Anomalies` struct and its
     `impl` block (current lines 119-168). The existing `Anomalies`
     struct and `impl` block are left untouched, byte-for-byte, at
     their current location.

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

1.5. `DateAnomalies::row_marker` is a new method, same body shape as
     `Anomalies::row_marker` (unchanged, still present on `Anomalies`
     too), now keying off `DateAnomalies::has_any()`'s wider
     definition. Both `row_marker` methods coexist until Task 4 deletes
     the old type.

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

1.8. Extend the module's test block, additively:
   - Leave every existing test targeting `Anomalies` untouched — same
     names, same bodies, same expected strings, still passing.
   - Add new tests for `DateAnomalies`/`StaleOpenStart`/
     `LookbackFinding`/`LookbackKind`/`WEEK_ANOMALY_FOOTER` per the
     acceptance criteria (see Test plan below). These are new test
     functions with new names, not replacements of the `t16`-`t33`
     family.
   - Extend the module's existing plain-ASCII sweep test (AC6) to also
     cover the new literals, rather than adding a second, parallel
     sweep test.

## Handoff to Task 3

`tests/status_cli.rs` is Task 3's file now, not this task's. This
task's own tests never touch it. The exact change Task 3 needs there,
so it doesn't have to re-derive it:

- Update `t14_status_with_anomalies_exits_zero_and_prints_to_stdout`
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
  `out.contains("or: mlm stop HH:MM --date")`), since this is the one
  binary-level test this changeset has for E7 and the whole point of
  the changeset is that fix lines now exist. This is a judgment call
  within Task 3's owned file, not a spec requirement of exactly this
  test.
- No other existing assertion in that file names anomaly wording
  (confirmed by grep across the file during this task's planning —
  only line 83 matches `"open stints"`/`"orphaned end"`/`"unmatched
  starts"`).
- This assertion cannot pass until `status.rs` itself switches to
  `render::DateAnomalies` and produces the new wording — it is not
  something Task 2 can verify in isolation, since it exercises the
  compiled binary.

Further handoff, beyond the test file:

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
- Once Task 3 and Task 5 have both switched their call sites to
  `DateAnomalies`, the old `render::Anomalies` type (struct, `impl`
  block, and its own now-dead tests) becomes unreferenced. Deleting it
  is **Task 4's job**, sequenced after both have merged (see the parent
  plan's Task 4 file ownership and "Additive, not replacing" above) —
  not something Task 3 or Task 5 does as a side effect of switching
  over, and not something this task does either.
- Between this task's merge and Task 3/Task 5 each switching over,
  `render::Anomalies` and `render::DateAnomalies` both compile and both
  have passing tests, so `cargo build`/`test`/`clippy` stay green at
  every commit in between — no temporary red window, unlike the
  discarded replace-in-place approach (see "Additive, not replacing").

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

### AC5 — no test left asserting old E7/E8 text (reframed for the
additive approach — within this task's own new-type fixtures)

Under the additive approach, `render::Anomalies`'s own existing tests
correctly keep asserting the old E7/E8 text, because `Anomalies` itself
still renders that old text unchanged (it is not touched, deleted, or
reworded until Task 4). This AC therefore applies only to this task's
new `DateAnomalies`/`LookbackFinding` fixtures, not to `Anomalies`'s
pre-existing ones:

- Grep-equivalent assertion at the end of the new `DateAnomalies`
  fixtures in the `render.rs` test module: a single test that greps
  every literal produced by those new fixtures for the strings
  `"(unmatched starts)"` (old E7 suffix) and confirms zero matches,
  guarding against a stray leftover in the new code path only.
- `tests/status_cli.rs` line 83 is no longer this task's file (see
  Scope) — the "no test elsewhere is left asserting the old E7/E8
  text," changeset-wide, is Task 3's acceptance criterion to close out
  once it switches `status.rs` over; the exact replacement text is
  spelled out under "Handoff to Task 3" above so Task 3 doesn't have to
  re-derive it.

### AC6 — every new literal is plain ASCII

- Extend the existing `t33_every_produced_string_is_ascii`-style sweep
  (module's existing plain-ASCII test) to include: an E7 block, a
  stale block, an E8 block, both refused-join lines, both lookback
  block variants, both lookback stderr variants, and
  `WEEK_ANOMALY_FOOTER`. One assertion loop over all of them, matching
  spec §3.7 / changeset plan's "extends the relevant existing golden
  test rather than adding a parallel check" constraint.

## Risks, ambiguities, and disagreement with the plan/spec

1. **Resolved (was: replacing `Anomalies` outright breaks `cargo build`
   until Task 3/5 land).** An earlier draft of this plan replaced
   `Anomalies` in place, which left the build red between this task's
   merge and Task 3/Task 5's. NOTES.md entry 79 settled this: Task 2 is
   additive (adds `DateAnomalies` beside `Anomalies`, deletes nothing),
   Task 3 and Task 5 each switch their own call site over, and Task 4
   deletes the now-unused `Anomalies` once both have. This task's
   `cargo build`/`test`/`clippy` are green on its own commit; see
   "Additive, not replacing" above.

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

4. **Resolved (was: `tests/status_cli.rs`'s updated assertion cannot be
   run to a passing result by this task alone).** That file is no
   longer this task's to edit — it moved to Task 3 (see Scope), which
   is also where the rewiring that makes the new assertion pass
   happens. The exact expected string is written under "Handoff to
   Task 3" above so Task 3 doesn't have to re-derive it.

5. **Minor spec ambiguity, not blocking**: spec §3.4 shows the E7
   `or:` line as `"a time after 14:00 closes the 14:00 start"` —
   the same time-value appears twice in one sentence. I render it that
   way literally (both interpolations read `latest_open_start`); it
   reads a little redundant in prose but matches the spec's own
   worked example exactly, and this task's job is verbatim rendering,
   not prose-polishing the spec.
