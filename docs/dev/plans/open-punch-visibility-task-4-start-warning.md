# Task 4 plan: `start` stale-start warning (`src/commands.rs`)

> **Archived — historical planning record.** Not authoritative. Current spec: `docs/dev/SPEC.md`. Design/process history: `docs/dev/NOTES.md`.

**Update note**: this plan was originally written before Task 2's and
Task 3's own plans existed, on assumed signatures for both
(`docs/dev/plans/open-punch-visibility-task-2-render.md`,
`docs/dev/plans/open-punch-visibility-task-3-status.md`). Both now
exist; this revision replaces every assumed name/signature with the
real one from those plans' "Published interface" sections, and adds
the `render.rs` cleanup step the changeset plan's own update assigned
to this task (deleting the old `Anomalies` type once Tasks 3 and 5 have
both switched off it). The "What was read"/dependency sections below
still narrate the original, pre-update state where they describe what
was true when this plan was first drafted; corrections are called out
inline rather than silently rewriting that history.

## What was read for this plan

- `docs/dev/plans/open-punch-visibility-plan.md` (full, 450 lines) —
  changeset plan: goal, architecture, global constraints, interface
  contracts A/B, ordering/parallelization, and Task 1-6 acceptance
  criteria.
- `docs/dev/specs/2026-09-26-open-punch-visibility.md` (full, 276
  lines) — locked spec, all of §1-§5, especially §2 (terms), §3.3
  (lookback), §3.6 (`start`'s warning), §7 (test plan).
- `docs/dev/plans/` directory listing, checked twice: once at the start
  of this work (only the changeset plan itself present) and again just
  before finishing (Task 1's plan,
  `open-punch-visibility-task-1-stint-predicates.md`, had landed by
  then; Task 2, 3, and 5's plans were still absent both times). Task 1
  owns `src/stint.rs` only and has no contract with Task 4 (the
  changeset plan pairs Task 4 with Task 3 alone), so its arrival
  doesn't change anything in this plan. Task 3's plan (which would pin
  the lookback function signature) and Task 2's plan (which would pin
  the stderr line renderer's signature) are both still missing. This
  plan proceeds on assumed shapes for those two, recorded in the
  dependencies table below, since the changeset plan explicitly allows
  Task 4 to be written against "the finished code" once Task 3 lands
  rather than a pre-pinned contract (no Contract exists between Task 3
  and Task 4).
- `src/commands.rs` (full, 1693 lines — read in two pages) — current
  `start`/`stop`/`note`/`delete_*`/`punch` implementation and its
  entire `#[cfg(test)] mod tests` block, to learn the file's existing
  helper structure, test fixture conventions (`fixed_now()`, `d()`,
  `punch_args()`), and its doc-comment style.
- `src/main.rs` (full, 95 lines) — confirms `commands::start` is called
  from `dispatch()` with exactly `(&mut conn, now, a)`, that `main`
  itself does no stdout/stderr formatting beyond the one `error: {e}`
  line in `exit_code`, and that no CLI flags route through `main.rs`
  for `start` beyond what `PunchArgs` already carries.
- `clippy.toml` and `.githooks/pre-commit` (both full) — cognitive
  complexity threshold is 15 (`clippy::cognitive_complexity`,
  clippy.toml), enforced pre-commit alongside `cargo fmt --check` and
  a coverage-regression check (`cargo llvm-cov` vs.
  `coverage-baseline.json`, ratcheted up only, hooked via
  `.githooks/pre-commit`, opt-in via
  `git config core.hooksPath .githooks`).
- Targeted reads (`grep`/`sed -n`) of `src/stint.rs` (`DayStints`,
  `classify`/`classify_at`/`splice_candidate` signatures),
  `src/storage.rs` (`punches_in_range` signature), `src/date.rs`
  (`format_date`/`format_date_with_weekday` — the `Wdy YYYY-MM-DD`
  helper spec lookback lines need), `src/render.rs`'s module doc and
  head (confirms it is still the pure `now()`-free, DB-free layer the
  changeset plan describes), and `tests/status_cli.rs`/
  `tests/e6_db_failure.rs` (confirms this project's stderr-testing
  convention: `assert_cmd`/`predicates` are declined project-wide;
  stderr is checked either by spawning the real binary via
  `std::process::Command` in `tests/*.rs`, or — the pattern this task
  uses — by giving the unit under test an injectable `Write` sink and
  asserting against an in-memory buffer).

## Scope

**Files this task owns**: `src/commands.rs` exclusively, plus —
sequentially, only after Tasks 3 and 5 have both merged — one bounded
job in `src/render.rs`: deleting the old per-date anomaly type
(`Anomalies`) and whatever renders only it, once nothing in the crate
still calls it (per the changeset plan's amendment; see "`render.rs`
cleanup" below). Nothing else is touched, edited, or committed.

**Files this task must not touch, and who owns them**:

| File | Owner |
|---|---|
| `src/stint.rs` | Task 1 — the two new anomaly predicates (stale, refused-join) |
| `src/render.rs` (everything except the post-merge deletion job below), `tests/status_cli.rs` | Task 2 — anomaly line wording, including the `start`-stderr line shape (§3.6) |
| `src/status.rs` | Task 3 — per-date anomaly assembly and the shared 14-day lookback scan |
| `src/week_view.rs` | Task 5 — `week` row markers and footer |
| `docs/dev/SPEC.md`, `README.md` | Task 6 — docs |

`src/main.rs` and `src/storage.rs` are untouched by every task in this
changeset (per the changeset plan's own "no task" note) — this plan
does not touch them either. No new CLI flag, no new dispatch arm, no
new `storage.rs` query.

**What this task is not**: it does not compute *whether* a date is
stale or implement any pairing/anomaly math (Task 1's job); it does
not decide the lookback window's own bounds or walk the date range
itself (Task 3's job, since §3.3's scan already has to exist there for
`status`, and the changeset architecture explicitly assigns it there
so `start` only calls in); it does not word the stderr line (Task 2's
job — this task only calls the renderer and writes what it returns).
This task's whole job is: after `start` writes successfully, call
Task 3's lookback, filter out the one excluded case (§3.6's
just-written-date rule), and print what Task 2's renderer produces to
stderr — plus wiring that stays best-effort on read failure.

## Facts gathered from the current source

- `src/commands.rs:21-23` — current `start`:
  ```rust
  pub fn start(conn: &mut Connection, now: DateTime<Local>, args: &PunchArgs) -> anyhow::Result<()> {
      punch(conn, now, PunchKind::Start, args)
  }
  ```
- `src/commands.rs:28-30` — current `stop`, sharing the same `punch()`
  helper with a different `PunchKind`.
- `src/commands.rs:256-283` — `punch()`, the shared `start`/`stop`
  implementation. It resolves `target_date` via `resolve_target_date`
  (line 263), parses `TIME` (267-269), joins the note (271), calls
  `storage::insert_punch_with_note` (273-281), and currently returns
  `Ok(())`. **`target_date` is computed inside `punch()` and discarded**
  — nothing today surfaces it to the caller.
- `src/commands.rs:234-242` — `resolve_target_date(date: &Option<String>, today: NaiveDate) -> anyhow::Result<NaiveDate>`, already `pub`-within-module (no visibility modifier, so crate-private but callable from anywhere in `commands.rs`), already exactly the date-resolution logic `start` itself would need to know "the date just written to" without re-deriving backdating rules.
- `src/main.rs:45` — `Command::Start(a) => commands::start(&mut conn, now, a)`. This call site's argument list must not change (task's "no change to `main.rs`" constraint) — `start`'s public signature (`(conn: &mut Connection, now: DateTime<Local>, args: &PunchArgs) -> anyhow::Result<()>`) stays exactly as it is today.
- `src/stint.rs:57-65` — `DayStints { open: Vec<OpenStint>, orphaned_ends: Vec<OrphanedEnd>, .. }` (plus other fields not relevant here); `classify`/`classify_at`/`splice_candidate` signatures confirm Task 1's predicates will take already-classified `DayStints`-shaped data, not punches directly — consistent with the changeset architecture's "pure functions over data stint.rs already produces."
- `src/storage.rs:396` — `punches_in_range(conn, start_date, end_date)` already exists and is the only DB read Task 3's lookback needs (confirmed by the changeset plan's own "no new storage.rs function" claim) — Task 4 never calls it directly, only through Task 3's lookback entry point.
- `src/date.rs:175-176` — `format_date_with_weekday(d) -> "Thu 2026-02-12"` shape, matching the `Wdy YYYY-MM-DD` piece of every §3.4/§3.6 line — confirms Task 2's renderer has this helper available; Task 4 does not call it directly (Task 2 owns all wording).
- `tests/status_cli.rs:4` and `tests/e6_db_failure.rs:11` — both state explicitly that `assert_cmd`/`predicates` are declined for this project; stderr is asserted either through a spawned real binary (`std::process::Command`, `.output()`, decode `output.stderr`) or, as this task does, through an injectable writer captured in-process. Since this task owns only `src/commands.rs` (no new `tests/*.rs` file), its tests must live in `commands.rs`'s own `#[cfg(test)] mod tests` block, which rules out the spawned-binary approach for this task specifically (that approach needs a separate `tests/*.rs` file, which is Task 2's/nobody's to add here) — the injectable-writer approach is not just preferred, it's the only one this task's scope permits.

## Dependencies not yet landed

Task 2's and Task 3's own plans now exist and pin the real shapes
below (Task 2's "Published interface" section in
`open-punch-visibility-task-2-render.md`; Task 3's "Published
interface" section in `open-punch-visibility-task-3-status.md`). Both
plans are still being re-aligned in parallel against each other, but
per this update's own brief, Task 2's names don't change and Task 3's
`lookback` function is Task 3's authoritative published signature — so
the rows below are pinned imports now, not assumed shapes, even though
neither has merged yet.

**Naming inconsistency found and flagged, not fixed here**: Task 3's
plan re-exports `pub use crate::render::{LookbackFinding,
LookbackFindingKind};` and calls a free function
`render::lookback_stderr_line(&finding)`. Task 2's plan (authoritative
for `render.rs` names) instead defines the enum as `render::LookbackKind`
(not `LookbackFindingKind`) and puts the stderr renderer on the type
itself as a method, `LookbackFinding::stderr_line(&self) -> String`
(not a free function). This plan is written against Task 2's actual
names, per this update's instruction to align to Task 2 when the two
disagree — Task 3's plan has the same inconsistency to resolve on its
own side.

| Dependency | Real shape (pinned by sibling plan) | Import path to use once merged |
|---|---|---|
| Task 3's lookback scan (`src/status.rs`) | `pub fn lookback(conn: &Connection, today: NaiveDate, now_utc: DateTime<Utc>) -> anyhow::Result<Vec<render::LookbackFinding>>`, ages 2..=14, oldest-first, one entry per window date with >=1 open start after splice resolution. Note the extra `now_utc: DateTime<Utc>` parameter this plan's earlier draft didn't assume — required because the scan calls `stint::classify_at`, which needs a clock instant, not just `today`. | `use crate::status;` (already in scope), call as `status::lookback(conn, today, now_utc)`. |
| `LookbackFinding`/`LookbackKind` (`src/render.rs`, Task 2's types — `status.rs` only constructs and returns values of them, per Contract A) | `pub struct LookbackFinding { pub date: NaiveDate, pub age_days: i64, pub kind: LookbackKind }`; `pub enum LookbackKind { SingleOpenStart { time: NaiveTime }, MultipleOpenStints { count: usize } }`. Field is `age_days`, not `age`; variants are `SingleOpenStart`/`MultipleOpenStints`, not `Single`/`Multiple`. | `use crate::render::LookbackKind;` (`LookbackFinding` values arrive already typed from `status::lookback`'s return, no separate import needed beyond what a fully-qualified match arm requires). |
| Task 2's stderr line renderer (`src/render.rs`) | A method, not a free function: `impl LookbackFinding { pub fn stderr_line(&self) -> String }`, producing exactly §3.6's one-line `warning: ...; fix: ...` / `warning: ...; see mlm status ...` shape. | Call `finding.stderr_line()` directly; no import beyond `LookbackFinding` itself already being in scope from `status::lookback`'s return type. |

Until Tasks 3 (and, transitively, Task 2) actually merge, `cargo
build` for this crate does not succeed against these real names — this
is the expected, temporary state the changeset's parallel-worktree
structure already accepts (see Task 2's plan, "Handoff to Task 3 /
Task 5"). Per the changeset ordering, Task 4 does not start
implementation until Task 3 has merged, so this plan proceeds directly
against the real signatures above with no stub layer.

## Design

### Signature changes (internal only — public API unchanged)

`start`'s and `stop`'s public signatures do not change (main.rs's call
sites are untouched, per the changeset's "no change to main.rs" global
constraint). The only signature change is to the existing private
`punch()` helper, so `start` can learn which date it just wrote to
without re-deriving `resolve_target_date` a second time:

```rust
// before: fn punch(...) -> anyhow::Result<()>
// after:
fn punch(
    conn: &mut Connection,
    now: DateTime<Local>,
    kind: PunchKind,
    args: &PunchArgs,
) -> anyhow::Result<NaiveDate> {
    // ...unchanged body...
    storage::insert_punch_with_note(/* ...unchanged... */)?;
    Ok(target_date)   // was: Ok(())
}
```

`stop()` ignores the new return value:

```rust
pub fn stop(conn: &mut Connection, now: DateTime<Local>, args: &PunchArgs) -> anyhow::Result<()> {
    punch(conn, now, PunchKind::End, args)?;
    Ok(())
}
```

`start()` uses it to know the written date, then runs the warning as a
strictly-after, best-effort step:

```rust
pub fn start(conn: &mut Connection, now: DateTime<Local>, args: &PunchArgs) -> anyhow::Result<()> {
    let written_date = punch(conn, now, PunchKind::Start, args)?;
    warn_stale_starts(
        conn,
        now.date_naive(),
        now.with_timezone(&Utc),
        written_date,
        &mut std::io::stderr(),
    );
    Ok(())
}
```

`now.with_timezone(&Utc)` matches this file's own existing convention
for deriving a UTC instant from the injected `Local` clock (see
`src/commands.rs:38` and `:280` today) — needed here because Task 3's
real `lookback` signature takes `now_utc: DateTime<Utc>` in addition to
`today: NaiveDate` (see Dependencies table above).

This keeps `punch()`'s date-resolution logic computed exactly once
(no duplicate call to `resolve_target_date`), keeps `stop` untouched
in behavior, and keeps the write and the warning strictly sequenced —
if `punch()` errors, `start` returns before the warning code ever
runs, satisfying "after a *successful* start write" (§3.6).

### `warn_stale_starts`: the new logic

```rust
/// §3.6: after a successful `start` write, warn on stderr about every
/// stale open start the shared lookback (Task 3, `status.rs`) can see.
/// Best-effort: a read failure here is swallowed, never surfaced as an
/// error and never turns a successful write into a failed command
/// (§3.6 "the check is best-effort"). Generic over `W: io::Write` so
/// tests can capture output in a `Vec<u8>` instead of touching real
/// stderr; the real call site passes `std::io::stderr()`.
fn warn_stale_starts<W: std::io::Write>(
    conn: &Connection,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
    written_date: NaiveDate,
    out: &mut W,
) {
    let Ok(findings) = status::lookback(conn, today, now_utc) else {
        // Best-effort per §3.6: swallow the error, print nothing extra,
        // exit 0 either way (start()'s own Ok(()) already covers exit
        // code -- this function returns nothing, so there is nothing
        // here that could fail the command).
        return;
    };

    for finding in findings {
        if finding.date == written_date
            && matches!(finding.kind, render::LookbackKind::SingleOpenStart { .. })
        {
            // The one open start on the date we just wrote to is the
            // punch this very command made -- never warn about it.
            continue;
        }
        let _ = writeln!(out, "{}", finding.stderr_line());
    }
}
```

Two small functions instead of one keeps each well under the
cognitive-complexity budget of 15: `start()` (3 lines of logic) and
`warn_stale_starts()` (a swallowed-`Result` early return plus a single
filtering loop) are both far simpler than the threshold, unlike (per
the changeset plan's own complexity warning) the lookback scan itself
or the §3.4 assembly logic, neither of which lives in this file.

### Why the exclusion check is exactly `date == written_date && SingleOpenStart`

§3.6: "The date the command just wrote to counts only when it now has
two or more open starts (so the new punch made an E7); a single open
start there is the punch just written and gets no line." A
`MultipleOpenStints` finding on `written_date` must still be printed
(it proves the new punch collided with an existing open start on that
same backdated date, i.e. created an E7) — only `SingleOpenStart` on
`written_date` is suppressed, since that single entry is definitionally
the punch this command just made. This matches Task 1's predicate
boundary too: `LookbackKind::SingleOpenStart` only exists when there is
exactly one open start on that date, which — on `written_date`, right
after a `start` write — can only be the punch just made (nothing else
could have produced a second one without also flipping the kind to
`MultipleOpenStints`).

### Where the warning fires relative to today vs. backdated dates

`start` for today writes to `today`, which is always outside the
lookback window (ages 2-14) by definition, so the exclusion check
never actually triggers for an un-backdated `start` — consistent with
spec's own observation ("`start` for today writes to a date outside
the window, so the rule only matters for a backdated `start`"). No
special-casing is needed in the code for "today vs. backdated" beyond
the one exclusion check above; the age arithmetic itself is entirely
Task 3's.

### `render.rs` cleanup (sequential, after Tasks 3 and 5 merge)

Per the changeset plan's amendment, Task 2 lands `render.rs`
additively: the old `Anomalies` type keeps working alongside the new
`DateAnomalies`/`LookbackFinding` types, so the crate stays buildable
between Task 2's merge and Task 3/5's. Once Task 3 and Task 5 have both
merged and switched their own call sites over to `DateAnomalies`,
nothing in the crate constructs or calls the old `Anomalies` type any
more — this task deletes it, as a separate, sequential step after its
own `commands.rs` work above (not blocking it — `commands.rs`'s changes
don't depend on this cleanup happening first or after).

What today's `src/render.rs` has that becomes dead once Task 3/Task 5
land:
- `pub struct Anomalies { open_stint_count, orphaned_end_times }`
  (currently `src/render.rs:119-132`).
- `impl Anomalies { has_any, detail_lines, row_marker }` (currently
  `src/render.rs:135-168`).
- Every test in `render.rs`'s own `#[cfg(test)] mod tests` block that
  constructs an `Anomalies` value directly (currently the majority of
  the module's test fixtures, e.g. `src/render.rs:417-651` in the
  pre-Task-2 file) — these exact line numbers will have shifted once
  Task 2's additive changes land, so this task's implementer locates
  them by symbol (`grep -n "Anomalies" src/render.rs`) rather than by
  the line numbers recorded here.

Nothing else in `render.rs` renders only the old type (confirmed by
reading the current file in full while writing this update — there is
no free function taking `&Anomalies`, only the struct's own methods
above).

**Steps**:
1. `grep -rn '\bAnomalies\b' src/ tests/` and confirm every remaining
   hit is either the new `DateAnomalies` (a different identifier, so it
   won't match a plain `\bAnomalies\b` word-boundary grep for the old
   name) or is inside `src/render.rs` itself, in the type/impl/tests
   being deleted. Any hit in `src/status.rs` or `src/week_view.rs`
   naming the old `Anomalies` at this point is a sign Task 3 or Task 5
   didn't finish migrating — stop and do not delete until that's
   resolved (out of scope for this task to fix in someone else's file).
2. Delete the struct, its `impl` block, and its dedicated test
   fixtures, identified per the grep above.
3. Verify nothing was actually still depending on the deleted code:
   - `grep -rn '\bAnomalies\b' src/ tests/` again — zero hits anywhere
     in the crate.
   - `cargo build` — must succeed; a lingering caller would fail here
     first, loudly, as a compile error.
   - `cargo clippy --all-targets -- -D warnings` — must be clean (also
     catches any now-dead private helper the deletion exposes).
   - `cargo test` — full suite green; confirms no test elsewhere
     depended on `Anomalies` existing (beyond the ones just deleted)
     and that `render.rs`'s own remaining tests (the `DateAnomalies`/
     `LookbackFinding` ones Task 2 added) still pass untouched.
4. `cargo fmt` before committing, per this repo's standard verification
   sequence (`AGENTS.md`).

This step touches only `src/render.rs` — no edit to `src/status.rs` or
`src/week_view.rs`, consistent with this task's file ownership even
during the sequential cleanup phase.

## Test plan

Every test below lives in `src/commands.rs`'s existing
`#[cfg(test)] mod tests` block, constructing `render::LookbackFinding`/
`render::LookbackKind` values directly (both are real, pinned types by
the time this task is implemented — Task 4 does not start until Task 3,
and transitively Task 2, have merged, so no stub layer is needed). All
tests use the module's existing `fixed_now()`/`d()`/`punch_args()`
fixtures.

### §3.6 — warning present with right lines, oldest-first ordering

Spec case: "Lookback: one line per window date, oldest first."
Construct two `render::LookbackFinding`s out of chronological order
(e.g. age 5 then age 10), feed them through a test-only seam that lets
`warn_stale_starts` accept a pre-built findings list directly (see
"Test seam" below) rather than a live scan, and assert the captured
buffer's lines appear oldest-date-first, one per finding, in the exact
count expected, matching `finding.stderr_line()`'s real §3.6 text.

### §3.6 — stdout empty and exit `0` throughout

Spec case: "`start`: warning on stderr, stdout empty, exit `0`." Reuse
the existing `T9`-style `success_exits_zero` pattern: call `start()`
in a test with a `Vec<u8>` stderr sink, assert `crate::exit_code(&r)
== 0` regardless of the sink's contents, and assert nothing was ever
written to real stdout (this file's existing handlers, e.g.
`delete_punch`, already avoid `println!` on the success path being
tested here — `start()` today has no `println!` at all, and this task
adds none).

### §3.6 — no warning for the just-written date with a single open start

Spec case: "no warning for the date just written" / "into an empty
stale date [a backdated start] does not warn about `D` itself." Seed a
findings list containing exactly one `LookbackKind::SingleOpenStart`
finding whose `date` equals the `written_date` the test's `start()`
call resolves to; assert the captured stderr buffer is empty.

### §3.6 — multi-open warning when the new punch made an E7

Spec case: "a backdated `start --date D` into an existing stale date
warns with the multi-open form." Seed a findings list with a
`LookbackKind::MultipleOpenStints` finding on `written_date`; assert
exactly one line is written and that it is not suppressed (this is the
one case the exclusion filter must let through despite matching
`written_date`).

### §3.6 — no warning when the window is clean

Spec case: "no warning when the window is clean." Empty findings list
(`status::lookback`'s real return is `Ok(Vec::new())` when the window
has no findings); assert the stderr sink stays empty and `start()`
still returns `Ok`.

### §3.6 — read-failure path is best-effort

Spec case: "If the lookback read fails after the write already
committed, `start` still exits `0` and prints nothing extra." Test
seam: drop the `punches` table (or otherwise corrupt the schema) on
the in-memory test connection *after* seeding it via the normal
migration path, immediately before calling `warn_stale_starts`
directly (bypassing `start()`'s own successful write, since the point
here is to fail only the read, not the write) — this is a genuine seam
that needs no new production plumbing, since `Connection` is already
mutable and test-owned: seed a valid DB, run `start()` once
successfully, corrupt the schema, then call `warn_stale_starts` against
the corrupted connection and assert it returns without writing
anything and without panicking. If Task 3's real `lookback` tolerates a
missing table by returning empty results rather than erroring (so no
reachable seam produces an `Err`), this test is dropped and the
swallowed-error path is instead documented with a single comment at the
`let Ok(findings) = ... else { return }` site, and the gap named in
this task's commit message, per the acceptance criterion's own fallback
clause.

### §3.6 — backdated `start --date D` into an existing stale date, and into an empty stale date

Spec case: "a backdated `start --date D` into an existing stale date
warns with the multi-open form; into an empty stale date it does not
warn about `D` itself" (already covered structurally by the two tests
above, restated here as the exact spec wording this task's acceptance
criteria cite verbatim, to make the mapping explicit rather than
implicit).

### Non-lookback regression: `start`'s existing behavior is undisturbed

Not a new spec case, but required by this task's "out of scope: any
change to `start`'s existing argument handling or its own write path"
constraint. Every existing test in `commands.rs` that calls `start(...)`
today (T1-T5, the backdated-punch suite, the retroactive-week-recompute
test, etc.) must still pass unchanged after `punch()`'s return type
changes from `()` to `NaiveDate` — none of them inspect `punch()`'s
return value directly (they all call `start`/`stop`, whose public
`Result<()>` signature is unchanged), so no existing assertion needs
editing; this is a compile-and-rerun check, not a new assertion.

### Test seam note

`warn_stale_starts` as sketched above calls `status::lookback` directly,
which makes "seed a specific findings list" tests (everything above
except the read-failure and regression tests) awkward without a live DB
shaped exactly right. Two options:

1. Split `warn_stale_starts` into a thin outer function (does the
   `conn`/`now_utc` read, calls Task 3's scan) and an inner pure function
   `fn emit_warnings<W: Write>(findings: Vec<render::LookbackFinding>, written_date: NaiveDate, out: &mut W)` that takes the findings as a plain parameter. The outer function is trivial (one call, one delegate) and therefore doesn't need its own exhaustive tests; every case above becomes a direct, DB-free unit test of `emit_warnings`.
2. Seed `punches` rows in a real in-memory `Connection` shaped to
   produce the desired findings from the real scan (e.g. an actual open
   start dated 5 days back for the "single" case). This is more
   end-to-end but couples every test here to Task 3's real date-window
   math.

Option 1 is preferred (keeps this task's tests independent of Task 3's
internal correctness, which is Task 3's own test suite's job to prove)
and is what the code sketch above should be read as implying, even
though the sketch inlines the split for readability. This is now the
clearly practical choice, not just the preferred one: Task 2's real
`LookbackFinding` is `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` with
plain public fields (`date`, `age_days`, `kind`), so it costs nothing to
hand-construct directly in a test — there is no remaining reason to
fall back to option 2's live-DB seeding.

## Risks, ambiguities, and disagreements

- **Task 2's and Task 3's plans now exist and this plan is updated
  against them** (this section previously flagged both dependency rows
  as unpinned assumptions, written before either sibling plan existed).
  The real names are `status::lookback(conn, today, now_utc)`,
  `render::LookbackFinding { date, age_days, kind }`,
  `render::LookbackKind::{SingleOpenStart { time }, MultipleOpenStints {
  count }}`, and `LookbackFinding::stderr_line(&self)` — all reflected
  in the Dependencies table and code sketches above. Two differences
  from the original assumption, both now folded in above rather than
  left as open risk: `lookback` takes an extra `now_utc: DateTime<Utc>`
  parameter this plan's first draft didn't anticipate, and the stderr
  renderer is a method on `LookbackFinding`, not a free function in
  `render.rs`.
- **Task 3's own plan has a naming inconsistency against Task 2's
  published names**, noted above in "Dependencies not yet landed":
  Task 3's plan re-exports `LookbackFindingKind` and calls
  `render::lookback_stderr_line(&finding)` as a free function, while
  Task 2's plan (authoritative for `render.rs` names) calls the enum
  `LookbackKind` and puts `stderr_line` on the type as a method. This
  plan is written against Task 2's names, per instruction to align to
  Task 2 when the two disagree. Flagged, not fixed here — Task 3's plan
  is being re-aligned separately and may already correct this by the
  time it merges.
- **Whether the read-failure test survives to the real implementation**
  is genuinely open (flagged inline in the test plan above) — it
  depends on whether Task 3's real `lookback` can be made to `Err` via
  a reachable, non-invasive seam (schema corruption on an in-memory
  connection). If Task 3's function instead treats a missing table as
  "no findings" rather than an error (plausible, since `rusqlite`
  queries against a dropped table do return an `Err`, but a future
  Task 3 implementation detail could catch and swallow that internally),
  this task's read-failure path becomes untestable without new
  production plumbing that's explicitly out of scope, and the
  acceptance criterion's own fallback (comment + commit-message note)
  applies. This isn't a disagreement, just a documented contingency.
- **`punch()`'s return-type change touches a function `stop()` also
  calls.** This is a minimal, behavior-preserving change (an `Ok(())`
  becomes an `Ok(target_date)`, `stop()` discards it), but it is worth
  flagging explicitly since it's the one edit in this task that touches
  code both `start` and `stop` depend on. No test of `stop()`'s
  behavior needs to change, and none of `stop()`'s existing assertions
  inspect the discarded return value.
- **No disagreement with the spec or changeset plan otherwise.** The
  written-date exclusion rule, the best-effort read-failure handling,
  and the "no lookback scope creep beyond `start`" boundary are all
  unambiguous in §3.6 and in Task 4's acceptance criteria; this plan
  did not have to resolve any genuine conflict between the two
  documents.
