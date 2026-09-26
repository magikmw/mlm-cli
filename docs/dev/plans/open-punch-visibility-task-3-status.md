# Task 3 plan: `status` per-date anomalies and the shared lookback

## What I read

- `docs/dev/plans/open-punch-visibility-plan.md` (full) — changeset
  plan, architecture, global constraints, Contract A, Contract B, the
  ordering diagram, and Task 3's own section.
- `docs/dev/specs/2026-09-26-open-punch-visibility.md` (full, locked
  design spec) — Terms (§2), Behavior §3.1-§3.7, the docs list (§4),
  the test plan (§5).
- `docs/dev/plans/` directory listing: the only `open-punch-visibility`
  plan file that exists right now is the changeset plan itself.
  `open-punch-visibility-task-1-*.md`, `-task-2-*.md`, `-task-4-*.md`,
  `-task-5-*.md` do not exist yet — the sibling agents writing them are
  still running. This plan is therefore built directly against
  Contract A and Contract B in the changeset plan, not against sibling
  task documents.
- `clippy.toml` — `cognitive-complexity-threshold = 15`.
- `.githooks/pre-commit` — the three-gate hook (fmt, cognitive
  complexity via clippy, coverage-vs-baseline via `cargo-llvm-cov`);
  all three degrade to `SKIP` when tooling is missing, none can be
  bypassed except `--no-verify`.
- `src/status.rs` (full, both source and the existing test module) —
  current `StatusView`/`resolve`/`render` structure, the existing
  `to_anomalies`/`build_ledger`/`build_stint_lines` helpers, and the
  full existing test fixture set.
- `src/stint.rs` (full) — `DayStints`, `classify`, `classify_at`,
  `close_group`, `splice_candidate` (the existing sibling predicate
  Task 1's two new ones sit beside), and their tests.
- `src/render.rs` (relevant sections: header comment, `Anomalies`,
  `detail_lines`, `row_marker`, `LABEL_WIDTH`) — the current shared
  rendering surface Task 2 will extend.
- `src/storage.rs` (relevant sections): `punches_for_date`,
  `punches_in_range`, `earliest_data_date` signatures and doc comments.
- `src/commands.rs` (relevant sections): `start`/`stop`/`note` handler
  signatures, to see exactly what Task 4 will have available to call
  into.
- `src/date.rs`: public surface listing (`resolve_date`, `WeekId`,
  etc.) — confirmed there is no existing "age in days" helper anywhere
  in the crate; every date-arithmetic caller does `today - date` with
  `chrono::Duration` inline.
- `tests/status_cli.rs` (relevant section): the binary-level `status`
  tests, including the E7 line asserted at `t14_status_with_anomalies_...`
  (this is the line the changeset plan's Task 2 section says it owns
  and will reword — confirmed to exist and confirmed its exact current
  text, `"[!] 2 open stints for this date (unmatched starts)"`).

## Scope

**This task owns exclusively**: `src/status.rs`. No other file is
touched, per the changeset plan's Task 3 file-ownership line and per
my own brief.

**Files this task must not touch, and who owns them**:

| File | Owner |
|---|---|
| `src/stint.rs` | Task 1 (the two new predicates this task calls) |
| `src/render.rs` | Task 2 (the new per-date fact struct, lookback-finding struct, and their rendering functions this task calls) |
| `tests/status_cli.rs` | Task 2 (rewords the E7 golden line this task's own output must still produce, unchanged in text) |
| `src/week_view.rs` | Task 5 (parallel to this task; consumes the same Task 1/Task 2 surface independently — no shared symbol with this task) |
| `src/commands.rs` | Task 4 (sequenced after this task; calls this task's published `lookback` function) |
| `docs/dev/SPEC.md`, `README.md` | Task 6 |

No contract is needed with Task 4 (it is sequenced strictly after this
task and reads the finished code). Contract B is the agreement this
task must hold to with Task 5, since neither touches the other's file
but both consume Task 1's predicates and Task 2's render shapes the
same way.

## Published interface (for Task 4)

Task 4 (`src/commands.rs`, `start`'s warning) calls exactly this,
after `start`'s write has committed:

```rust
// src/status.rs

/// One dated finding from the §3.3/§3.6 14-day lookback scan: a window
/// date (age 2-14 relative to `today`) that currently has at least one
/// open start, after that date's own boundary splices are resolved.
/// Defined in `render.rs` (Task 2) per Contract A/B — re-exported here
/// only in the sense that this module constructs and returns values of
/// it; see "Dependencies" below for today's stand-in.
pub use crate::render::{LookbackFinding, LookbackFindingKind};

/// Walks ages 2 through 14 relative to `today` (§2's lookback window),
/// oldest date first, and returns one `LookbackFinding` per window date
/// that has one or more open starts after splice resolution. A window
/// date with zero open starts (idle, or an orphaned end only) produces
/// no finding — §1.2's orphan gap is deliberate, not a bug.
///
/// Pure aside from a single `punches_in_range` read spanning
/// `[today - 15d, today - 1d]` (15 dates: the 13 window dates plus the
/// one-day neighbor each end of the window needs for its own
/// `classify_at`). `now_utc` is the caller's already-resolved clock
/// instant (contract 6: never read from a system clock in here).
///
/// Called by `status::resolve` for today's own `status` (§3.3) and by
/// `commands::start` after every successful write (§3.6) — this is the
/// one place the scan is implemented, so the two callers can never
/// disagree about what counts as a stale window date.
pub fn lookback(
    conn: &rusqlite::Connection,
    today: chrono::NaiveDate,
    now_utc: chrono::DateTime<chrono::Utc>,
) -> anyhow::Result<Vec<LookbackFinding>>;
```

Task 4's own filtering (excluding the date it just wrote to, unless
that date now has two or more open starts) is Task 4's job on the
returned `Vec<LookbackFinding>` — a plain `.filter()` over `date` and
`kind`, no extra parameter needed on `lookback` itself, matching the
changeset plan's "no contract is needed between Task 3 and Task 4"
line.

Task 4 also needs, from Task 2's `render.rs` (not this task's file,
listed here only so Task 4's call site reads completely):
`render::lookback_stderr_line(&finding) -> String` for the §3.6
one-line stderr form.

## Dependencies (Task 1 / Task 2 — not yet landed)

Both land before this task is implemented (per the changeset plan's
ordering: Task 3 depends on Task 1 and Task 2 merging first), so in
the ordinary case this task's implementer codes directly against the
real, merged `stint.rs`/`render.rs`. The stand-ins below exist for the
case this task's worktree is opened before that merge lands, so
implementation can start from the pinned contract text rather than
wait idle.

| Dependency | Landed? | Stub to test against today | Import to swap at integration |
|---|---|---|---|
| `stint::stale_open_start(day: &DayStints, date: NaiveDate, today: NaiveDate) -> Option<NaiveTime>` | No (Task 1) | Local fn in this task's worktree, same signature, same body: `(day.open.len() == 1 && (today - date).num_days() >= 2).then(|| local_time(&day.open[0].start))` | Delete the local fn; `use crate::stint::stale_open_start;` once Task 1's real one lands under this name |
| `stint::refused_join(earlier: &DayStints, later: &DayStints, later_punches: &[Punch]) -> bool` | No (Task 1) | Local fn: `earlier.open.len() >= 1 \|\| later.orphaned_ends.is_empty() { return false } else { !stint::splice_candidate(earlier.open.len(), later, later_punches) }` — note `splice_candidate` already exists and is `pub(crate)`, so this stub can call the real thing today | Delete the local fn; `use crate::stint::refused_join;` once Task 1 lands |
| `render::DateAnomalyFacts { open_stint_times: Vec<NaiveTime>, orphaned_end_times: Vec<NaiveTime>, stale_open_start: Option<StaleOpenStart>, refused_join_prev: Option<NaiveDate>, refused_join_next: Option<NaiveDate> }`, `render::StaleOpenStart { time: NaiveTime, age_days: i64 }` | No (Task 2) | Local structs, same fields, in this task's worktree | Delete local structs; `use crate::render::{DateAnomalyFacts, StaleOpenStart};` |
| `render::anomaly_detail_lines(date: NaiveDate, facts: &DateAnomalyFacts) -> Vec<String>` | No (Task 2) | Local fn producing exactly the §3.4 text for E7/stale/E8/refused-prev/refused-next, in that fixed order — copy the spec's literal strings verbatim into the stub so tests are already asserting real wording, not placeholder text | Delete local fn; `use crate::render::anomaly_detail_lines;` |
| `render::LookbackFinding { date: NaiveDate, age_days: i64, kind: LookbackFindingKind }`, `render::LookbackFindingKind::{SingleOpenStart { time: NaiveTime }, MultipleOpenStints { count: usize }}` | No (Task 2) | Local structs/enum, same shape, in this task's worktree | Delete local defs; `use crate::render::{LookbackFinding, LookbackFindingKind};` |
| `render::lookback_block_lines(finding: &LookbackFinding) -> Vec<String>` | No (Task 2) | Local fn producing the §3.4 lookback block form verbatim (both single- and multi-open variants) | Delete local fn; `use crate::render::lookback_block_lines;` |

**Handoff needed against Contract A, not planned here**: Contract A's
list of per-date facts (open-start count, orphaned-end list, stale
info, refused-join info) does not by itself carry what E7's fix line
needs — the spec's E7 example fills `HH:MM` with "the latest open
start on the date" (§3.4: *"In the E7 hint, `14:00` is the latest open
start on the date."*). A bare `open_stint_count: usize` cannot supply
that value. This task's stand-in therefore widens that fact to
`open_stint_times: Vec<NaiveTime>` (count is `.len()`, latest is
`.iter().max()`), and assumes Task 2 lands the same widening. This is
flagged again under Risks — it is the one place this task's design
depends on a fact Contract A's own text doesn't literally enumerate.

## Algorithm and code sketch

### 1. Per-date anomaly facts

The current `resolve()` computes `day = stint::classify_at(...)` (the
post-splice view) and, inside the `receiving_end_time` closure,
separately recomputes raw `classify()` results for `prev_punches` and
`punches` just to run `splice_candidate` for the "continues previous
day's stint" header suffix. This task's refused-join check needs the
same raw classifications plus a third one for `next_punches`, so it
replaces that ad hoc pair with one small struct built once:

```rust
/// Every classification `resolve()` needs for one target date: the
/// post-splice view (`day`) plus the three raw, pre-splice views
/// (`prev_raw`/`day_raw`/`next_raw`) the refused-join predicate
/// requires (Contract B: "raw single-date classifications ... never
/// classify_at-resolved ones").
struct DateContext {
    day: DayStints,
    prev_raw: DayStints,
    day_raw: DayStints,
    next_raw: DayStints,
}

fn classify_date_context(
    prev_punches: &[Punch],
    punches: &[Punch],
    next_punches: &[Punch],
    now_utc: DateTime<Utc>,
) -> DateContext {
    DateContext {
        day: stint::classify_at(prev_punches, punches, next_punches, now_utc),
        prev_raw: stint::classify(prev_punches, now_utc),
        day_raw: stint::classify(punches, now_utc),
        next_raw: stint::classify(next_punches, now_utc),
    }
}
```

`receiving_end_time` becomes:

```rust
let receiving_end_time = stint::splice_candidate(ctx.prev_raw.open.len(), &ctx.day_raw, &punches)
    .then(|| local_time(&ctx.day_raw.orphaned_ends[0].punch));
```

— byte-identical behavior to today's code (`fresh_target_classified`
was always exactly this `day_raw`), just sourced from the shared
context instead of a second local `classify()` call.

Facts for `target_date`:

```rust
fn anomaly_facts_for_date(
    ctx: &DateContext,
    target_date: NaiveDate,
    today: NaiveDate,
    punches: &[Punch],
    next_punches: &[Punch],
) -> render::DateAnomalyFacts {
    render::DateAnomalyFacts {
        open_stint_times: ctx.day.open.iter().map(|o| local_time(&o.start)).collect(),
        orphaned_end_times: ctx.day.orphaned_ends.iter().map(|o| local_time(&o.punch)).collect(),
        stale_open_start: stint::stale_open_start(&ctx.day, target_date, today)
            .map(|time| render::StaleOpenStart {
                time,
                age_days: (today - target_date).num_days(),
            }),
        refused_join_prev: stint::refused_join(&ctx.prev_raw, &ctx.day_raw, punches)
            .then(|| target_date.pred_opt().unwrap()),
        refused_join_next: stint::refused_join(&ctx.day_raw, &ctx.next_raw, next_punches)
            .then(|| target_date.succ_opt().unwrap()),
    }
}
```

`stale_open_start` reads `ctx.day` (post-splice), matching §2's own
definition of "open start" ("a trailing unmatched `start` left on a
date after `classify_at`"). `refused_join` reads the raw pair, per
Contract B's explicit instruction. Both open-start-count-driven checks
(E7 vs. stale) stay mutually exclusive by construction, since Task 1's
`stale_open_start` only returns `Some` when `ctx.day.open.len() == 1`
and E7's own line only renders when `open_stint_times.len() > 1` —
Task 2's `anomaly_detail_lines` owns that branch, this task only
supplies both facts and trusts Task 1's stated exclusivity.

`resolve()`'s new anomaly section:

```rust
let ctx = classify_date_context(&prev_punches, &punches, &next_punches, now_utc);
let facts = anomaly_facts_for_date(&ctx, target_date, today, &punches, &next_punches);
let mut anomaly_lines = render::anomaly_detail_lines(target_date, &facts);

if is_today {
    for finding in lookback(conn, today, now_utc)? {
        anomaly_lines.extend(render::lookback_block_lines(&finding));
    }
}
```

`day_total_minutes`/`has_open_stint` keep reading `ctx.day` exactly as
`day` does today (`ctx.day.completed_minutes()`, `ctx.day.is_ongoing()`)
— no behavior change there.

The old `to_anomalies` helper and its doc comment (about not calling
`DayStints::has_anomaly()`/`anomalies()` to avoid a second formula) are
deleted: `anomaly_facts_for_date` replaces it, and the "route
everything through one rendering formula" property it protected is now
`render::anomaly_detail_lines`'s job to uphold, not this module's.

### 2. The lookback scan

```rust
pub fn lookback(
    conn: &Connection,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
) -> anyhow::Result<Vec<render::LookbackFinding>> {
    let window_start = today - ChronoDuration::days(15); // (today - 14) - 1
    let window_end = today - ChronoDuration::days(1);    // (today - 2) + 1
    let punches = storage::punches_in_range(conn, window_start, window_end)?;
    let by_date = group_punches_by_date(&punches);

    let mut findings = Vec::new();
    for age in (2..=14).rev() {
        // .rev() walks age 14 down to 2, i.e. the OLDEST window date
        // (smallest calendar date) first -- oldest-first per §3.3.
        let date = today - ChronoDuration::days(age);
        if let Some(finding) = lookback_finding_for_date(date, age, &by_date, now_utc) {
            findings.push(finding);
        }
    }
    Ok(findings)
}

fn group_punches_by_date(punches: &[Punch]) -> BTreeMap<NaiveDate, Vec<Punch>> {
    let mut map: BTreeMap<NaiveDate, Vec<Punch>> = BTreeMap::new();
    for p in punches {
        map.entry(p.date).or_default().push(*p);
    }
    map
}

fn lookback_finding_for_date(
    date: NaiveDate,
    age: i64,
    by_date: &BTreeMap<NaiveDate, Vec<Punch>>,
    now_utc: DateTime<Utc>,
) -> Option<render::LookbackFinding> {
    static EMPTY: Vec<Punch> = Vec::new();
    let prev = by_date.get(&date.pred_opt().unwrap()).unwrap_or(&EMPTY);
    let cur = by_date.get(&date).unwrap_or(&EMPTY);
    let next = by_date.get(&date.succ_opt().unwrap()).unwrap_or(&EMPTY);
    if cur.is_empty() {
        return None; // nothing on the date at all -> nothing to report
    }
    let day = stint::classify_at(prev, cur, next, now_utc);
    let kind = match day.open.len() {
        0 => return None, // idle, or an orphaned end only (§1.2's known gap)
        1 => render::LookbackFindingKind::SingleOpenStart { time: local_time(&day.open[0].start) },
        n => render::LookbackFindingKind::MultipleOpenStints { count: n },
    };
    Some(render::LookbackFinding { date, age_days: age, kind })
}
```

(`static EMPTY: Vec<Punch> = Vec::new()` needs `Punch: Sync` — if that
doesn't hold, use a local `let empty = Vec::new();` per call instead;
either compiles the same logic, this is a detail to settle against the
real `Punch` derive at implementation time, not a design question.)

Range check against the spec's own accounting: window is
`[today-15, today-1]` inclusive, 15 calendar dates — exactly matching
§1.2's own count ("Every plain `status` for today now reads 15 extra
dates (one range query)").

`local_time(p: &Punch) -> NaiveTime` is a one-line helper
(`p.at_utc.with_timezone(&Local).time()`) factoring out the pattern
already repeated four times in the current file (`to_anomalies`,
`build_stint_lines` twice, the `receiving_end_time` closure) — adding
it here is in scope since it is a private helper inside the file this
task owns, and every one of its current call sites is inside this same
function or its immediate neighbors.

### 3. Complexity budget

Every new/changed unit stays small and single-purpose, per the
changeset plan's explicit warning that the lookback scan and the
anomaly-line assembly are shaped to blow past 15 if written flat:

- `classify_date_context` — 4 straight-line calls, no branches.
- `anomaly_facts_for_date` — 5 struct-field assignments, each a single
  predicate call or `.map()`/`.then()`; no nested branching.
- `lookback` — one loop over 13 fixed ages, delegating the actual
  per-date logic out.
- `group_punches_by_date` — one loop, one line body.
- `lookback_finding_for_date` — three lookups, one early return, one
  three-arm match; the natural size limit for "one date's lookback
  fact", not expected to need further splitting.
- `resolve()` itself grows by roughly 6 lines (construct `ctx`,
  construct `facts`, call `anomaly_detail_lines`, conditionally extend
  with lookback lines) replacing the ~10 lines it deletes
  (`to_anomalies` call plus the old two-line `receiving_end_time`
  setup) — expected to land at or below its current complexity, but
  this task's implementer must run
  `cargo clippy --all-targets -- -W clippy::cognitive_complexity`
  after wiring this in and split further (e.g. an
  `assemble_anomaly_lines` helper bundling the `facts`/lookback lines
  together) if `resolve()` itself trips the threshold.

## Test plan

One subsection per Task 3 acceptance criterion (changeset plan) /
spec test-plan line it is observable through. All new tests live in
`src/status.rs`'s existing `#[cfg(test)] mod tests`, extending its
`resolve()`-level fixtures (`test_db()`, `insert_punch` seeding) —
`render()`-level golden tests don't change shape for this task, since
`anomaly_lines` is still just `Vec<String>` from `resolve()`'s point of
view.

### `status DATE` reflects stale-open-start and refused-join facts for any date (spec §3.1, §3.2)

- Seed a single open start dated `today - 5 days`; `status` for that
  date's own `DATE` shows the stale-open-start line with age 5 and the
  right start time (§3.1, spec worked example's "5 days old" text).
- Seed a lone open start at age 0 and at age 1; neither date's `status`
  shows a stale line (§3.1's "yesterday's lone open start ... stay
  non-anomalies", spec §5's boundary-age bullet: 0 and 1 not an
  anomaly).
- Boundary: age exactly 2 is stale; age 14 is stale (also appears in
  today's lookback, tested separately below); age 15 is stale (spec
  §5: "at age 15 it is not [in the lookback]" — but the predicate
  itself, and therefore `status DATE` for that exact date, still shows
  it; this is the distinction the changeset plan's Task 1 section
  draws explicitly between "stale" and "in the lookback window").
- Seed two open starts on a date at age 5: `status DATE` shows only the
  E7 line, no stale line (Task 1 acceptance: stale predicate never
  fires for 2+ opens).

### Refused join in both directions and at the yesterday-today boundary (spec §3.2, §5)

- Construct the three refused-join shapes from the spec's test plan:
  two opens on the earlier date, the orphan not first on the later
  date, two orphans on the later date. For each, assert both dates'
  `status` shows the refused-join line naming the other date, with the
  exact parenthetical from §3.4 (verbatim, both directions' wording
  differ only in which date is named).
- A clean splice (the shape `splice_candidate` already accepts, per
  Task 1's own test fixtures) produces no refused-join line on either
  date — regression guard, mirrors Task 1's own acceptance criterion
  from this task's (status-level) vantage point.
- Refused join between yesterday and today: seed yesterday with an
  unmatched open start and today with an orphaned end that doesn't
  cleanly splice (e.g. a second punch precedes it); `status` (today,
  `DATE` omitted) shows the refused-join line. This is the case spec
  §5 calls out by name ("A refused join between yesterday and today
  shows on today's `status`").

### Anomaly line ordering, including the "all at once" interaction case (spec §3.4)

- One date carrying E7, a refused join (with either neighbor), and,
  because it's today, lookback lines: assert the full line sequence
  matches §3.4's fixed order 1-5 (E7, stale [absent here since E7
  precludes it], E8, refused-prev-then-next, lookback-oldest-first).
- A date with E8 (two orphaned ends) plus a stale open start plus a
  refused join with the next date: order is E7 [absent] / stale / E8
  ascending-by-instant / refused-next — asserts the E8-before-refused
  ordering specifically, since that's the one pair not covered by the
  "all at once" test above.

### Today's lookback (spec §3.3, §5)

- Seed single open starts at ages 3, 7, and 14; `status` (today) lists
  all three, oldest first (age 14, then 7, then 3), each in the
  single-open lookback block form.
- Seed two open starts at one window age; that date's line uses the
  multi-open lookback form ("N open stints on ...; see mlm status ...").
- `status DATE` for a non-today date (including an explicit past date
  that happens to itself be a window-aged date under some other
  reference) runs no lookback at all — only its own per-date facts.
- `status` with an explicit `today`'s own date string (`mlm status
  2026-02-12` when today is 2026-02-12) still runs the lookback —
  distinct code path from `DATE` omitted, both must hit `is_today`.
- An orphaned end alone at a window age (no open start) produces no
  lookback line for that date (§1.2's residual gap, spec §5's explicit
  test-plan bullet).
- Age-15 open start: present on its own `status DATE`'s stale line
  (tested above) but absent from today's lookback block (window is
  ages 2-14 only) — the two behaviors' boundary is exercised as one
  test with two assertions on the same fixture.

### Fix commands carry the real date, for every anomaly kind (spec §3.4, §5)

- For each of E7 / stale / E8 / lookback (both forms), assert the
  rendered `fix:`/`or:` line contains the literal `YYYY-MM-DD` of the
  anomaly's own date (in-date lines) or the window date (lookback
  lines) — never a placeholder, never today's date substituted for a
  past one. This is mostly an assertion on `render.rs`'s output as
  invoked through `resolve()`, guarding against this task passing the
  wrong date into the wrong fact field (e.g. `target_date` where
  `today` was meant, or vice versa in `refused_join_prev`/`_next`).

### Fix-command round-trip (spec §5)

- Seed a stale open start; run the hinted `stop` (a time after the
  open start's own time) against that date via `commands::stop`
  directly (not the binary — this module's tests don't spawn
  processes); re-`resolve()` the same date and assert no anomaly lines
  remain.
- Seed an orphaned end; the delete hint says "lists punches; rerun
  with the end's number" — assert `storage::delete_punch` on that row
  (simulating the second `delete` invocation) clears the E8 line on
  re-`resolve()`. This test only exercises that the hinted command's
  *effect* actually clears the fact this task computes; it is not a
  test of `delete`'s own CLI listing/renumbering behavior (out of
  scope here, already covered by `delete`'s own test suite).

### No regression outside Task 2's reworded text (spec: changeset plan's global constraints)

- Re-run (unmodified) every existing golden test in `src/status.rs`'s
  current test module that does not touch `anomaly_lines` — they must
  keep passing byte-for-byte, since this task changes `resolve()`'s
  internals but not `render()`'s contract (`anomaly_lines: Vec<String>`
  unchanged in shape).
- The existing T9/T10 anomaly-rendering tests
  (`t9_e7_multi_open_anomaly_rendering`, `t10_e8_...`) construct
  `StatusView.anomaly_lines` by hand and don't call `resolve()`, so
  they need no change from this task at all — flagged here only to
  confirm that fact, not to add new assertions to them.

## Risks, ambiguities, and disagreements

- **Contract A under-specifies E7's fix-line input** (detailed above
  under Dependencies). Contract A's own text lists "the existing
  open-start count" as a fact, but §3.4's E7 example needs the latest
  open start's *time*, not just a count. This task's design widens the
  fact to `open_stint_times: Vec<NaiveTime>`. If Task 2 lands with a
  bare `open_stint_count: usize` instead, this task's
  `anomaly_facts_for_date` needs a one-field change (add the vec
  alongside or instead of the count) and `render::anomaly_detail_lines`
  needs the same widening on its input — a small, contained mismatch
  to catch at integration, not a structural one.
- **`static EMPTY: Vec<Punch> = Vec::new()` needs `Punch: Sync`.**
  `Punch` derives only `Debug, Clone, Copy, PartialEq, Eq` today (per
  `stint.rs`'s use of it) with no explicit `!Sync` marker, so it should
  qualify automatically, but this is worth a one-line check at
  implementation time rather than an assumption baked into the plan —
  the fallback (`let empty = Vec::new();` per call) costs nothing if
  needed.
- **`resolve()`'s cognitive complexity is unverified by this plan.**
  `resolve()` was already a fairly full function before this task; I
  estimated the net line delta as small, but I did not (and could not,
  without editing source) run clippy's actual cognitive-complexity
  count against it. The pre-commit hook will catch a miss, but the
  changeset plan explicitly warns this exact area is likely to need
  splitting — budget for one extra extraction pass
  (`assemble_anomaly_lines(ctx, target_date, today, is_today, conn,
  now_utc, punches, next_punches) -> anyhow::Result<Vec<String>>`) if
  the hook flags it.
- **Whether `LookbackFinding`/`LookbackFindingKind` should live in
  `render.rs` or `status.rs`.** I read Contract B's "the finished shape
  of Task 2's per-date fact structure and lookback finding structure"
  as meaning both types are Task 2's (`render.rs`), with `status.rs`
  only constructing values of them — consistent with render.rs staying
  the single place wording/shape lives and with this task's own
  out-of-scope line ("leaving both textual renderings ... to Task 2's
  functions"). If Task 2 instead expects `status.rs` to own the type
  and `render.rs` to take it as a generic/duck-typed parameter, the fix
  is a one-line `pub struct` relocation, not a logic change — flagged
  here since I can't pin this against a sibling plan that doesn't exist
  yet.
- **No disagreement with the changeset plan's substance.** The
  architecture, the interface contracts, and Task 3's own acceptance
  criteria are all self-consistent and directly implementable; the
  items above are gaps in *specification precision* at the boundary
  with a not-yet-written sibling plan, not disagreements with what's
  already locked.
