# Task 4 plan: `start` stale-start warning (`src/commands.rs`)

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

**Files this task owns exclusively**: `src/commands.rs`. Nothing else
is touched, edited, or committed.

**Files this task must not touch, and who owns them**:

| File | Owner |
|---|---|
| `src/stint.rs` | Task 1 — the two new anomaly predicates (stale, refused-join) |
| `src/render.rs`, `tests/status_cli.rs` | Task 2 — anomaly line wording, including the `start`-stderr line shape (§3.6) |
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

| Dependency | Assumed shape (used by this plan) | Stub to compile/test against today | Swap at integration |
|---|---|---|---|
| Task 3's lookback scan (`src/status.rs`) | `pub fn lookback_open_starts(conn: &Connection, today: NaiveDate) -> anyhow::Result<Vec<LookbackFinding>>`, ages 2..=14, oldest-first, one entry per window date with >=1 open start; `LookbackFinding { date: NaiveDate, age: i64, kind: LookbackKind }` where `LookbackKind` is `Single { start: NaiveTime }` or `Multiple { count: usize }` (per §3.4/§3.6's two line shapes: "unclosed start ... at HH:MM" vs. "N open stints ..."). Pure signature-wise (takes `today`, not `Local::now()`), matching the crate's "now is always injected" convention. | A private stub block dropped into `commands.rs` (see code sketch) — `LookbackFindingStub`/`LookbackKindStub`/`lookback_open_starts_stub`, returning `Ok(Vec::new())` — lets this task's own code compile and its "no findings" tests pass before Task 3 exists. Tests that need non-empty findings construct `LookbackFindingStub` values directly (a plain data struct, not gated on real DB classification), so they exercise this task's filtering/ordering/rendering-call logic without needing Task 3's real scan logic at all. | Delete the stub block; change the two call sites (`warn_stale_starts`'s call into the scan, and its parameter/return types) to `crate::status::{lookback_open_starts, LookbackFinding, LookbackKind}`; re-run this task's tests unchanged except for the stub-construction helper, which becomes a thin wrapper around the real type if field names differ. |
| Task 2's stderr line renderer (`src/render.rs`) | `pub fn lookback_warning_line(finding: &LookbackFinding) -> String`, producing exactly §3.6's one-line `warning: ...; fix: ...` / `warning: ...; see mlm status ...` shape (Contract A: "same underlying fact, two textual shapes, both owned by Task 2" — this is the second, compressed shape). | A private stub `fn lookback_warning_line_stub(finding: &LookbackFindingStub) -> String` that renders a recognizable placeholder (e.g. `format!("warning: stub for {} (age {})", finding.date, finding.age)`) — good enough to prove this task's own line-count/ordering/exclusion tests pass; not asserted against §3.6's literal text (that assertion is meaningless until Task 2's real wording exists). | Delete the stub; call `crate::render::lookback_warning_line` instead. At that point, and only then, add the literal-text assertions this task's test plan below marks as "deferred to integration" (the exact strings from §3.6). |

Both rows are flagged again under Risks, since a plan written against
an assumed shape can't fully verify the assumption. The changeset
plan's own ordering diagram makes this low-risk in practice — Task 4
"cannot start before Task 3 merges," so by the time this plan is
actually executed as code, the stub row should already be moot.

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
    warn_stale_starts(conn, now.date_naive(), written_date, &mut std::io::stderr());
    Ok(())
}
```

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
    written_date: NaiveDate,
    out: &mut W,
) {
    let Ok(findings) = status::lookback_open_starts(conn, today) else {
        // Best-effort per §3.6: swallow the error, print nothing extra,
        // exit 0 either way (start()'s own Ok(()) already covers exit
        // code -- this function returns nothing, so there is nothing
        // here that could fail the command).
        return;
    };

    for finding in findings {
        if finding.date == written_date && matches!(finding.kind, status::LookbackKind::Single { .. }) {
            // The one open start on the date we just wrote to is the
            // punch this very command made -- never warn about it.
            continue;
        }
        let _ = writeln!(out, "{}", render::lookback_warning_line(&finding));
    }
}
```

Two small functions instead of one keeps each well under the
cognitive-complexity budget of 15: `start()` (3 lines of logic) and
`warn_stale_starts()` (a swallowed-`Result` early return plus a single
filtering loop) are both far simpler than the threshold, unlike (per
the changeset plan's own complexity warning) the lookback scan itself
or the §3.4 assembly logic, neither of which lives in this file.

### Why the exclusion check is exactly `date == written_date && Single`

§3.6: "The date the command just wrote to counts only when it now has
two or more open starts (so the new punch made an E7); a single open
start there is the punch just written and gets no line." A `Multiple`
finding on `written_date` must still be printed (it proves the new
punch collided with an existing open start on that same backdated
date, i.e. created an E7) — only `Single` on `written_date` is
suppressed, since that single entry is definitionally the punch this
command just made. This matches Task 1's predicate boundary too:
`LookbackKind::Single` only exists when there is exactly one open
start on that date, which — on `written_date`, right after a `start`
write — can only be the punch just made (nothing else could have
produced a second one without also flipping the kind to `Multiple`).

### Where the warning fires relative to today vs. backdated dates

`start` for today writes to `today`, which is always outside the
lookback window (ages 2-14) by definition, so the exclusion check
never actually triggers for an un-backdated `start` — consistent with
spec's own observation ("`start` for today writes to a date outside
the window, so the rule only matters for a backdated `start`"). No
special-casing is needed in the code for "today vs. backdated" beyond
the one exclusion check above; the age arithmetic itself is entirely
Task 3's.

## Test plan

Every test below lives in `src/commands.rs`'s existing
`#[cfg(test)] mod tests` block, using the stub types from the
dependency table until Task 3/Task 2 land (see integration-swap note
per test). All tests use the module's existing `fixed_now()`/`d()`/
`punch_args()` fixtures.

### §3.6 — warning present with right lines, oldest-first ordering

Spec case: "Lookback: one line per window date, oldest first."
Construct two `LookbackFindingStub`s out of chronological order (e.g.
age 5 then age 10), feed them through a test-only seam that lets
`warn_stale_starts` accept a pre-built findings list directly (see
"Test seam" below) rather than a live scan, and assert the captured
buffer's lines appear oldest-date-first, one per finding, in the exact
count expected. Deferred to integration: asserting the literal §3.6
text (needs Task 2's real `lookback_warning_line`).

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
stale date [a backdated start] does not warn about `D` itself." Seed
a stub findings list containing exactly one `Single` finding whose
`date` equals the `written_date` the test's `start()` call resolves
to; assert the captured stderr buffer is empty.

### §3.6 — multi-open warning when the new punch made an E7

Spec case: "a backdated `start --date D` into an existing stale date
warns with the multi-open form." Seed a stub findings list with a
`Multiple` finding on `written_date`; assert exactly one line is
written and that it is not suppressed (this is the one case the
exclusion filter must let through despite matching `written_date`).

### §3.6 — no warning when the window is clean

Spec case: "no warning when the window is clean." Empty findings list
(the stub's default `Ok(Vec::new())` or an explicit empty `Vec`);
assert the stderr sink stays empty and `start()` still returns `Ok`.

### §3.6 — read-failure path is best-effort

Spec case: "If the lookback read fails after the write already
committed, `start` still exits `0` and prints nothing extra." Test
seam: drop the `punches` table (or otherwise corrupt the schema) on
the in-memory test connection *after* seeding it via the normal
migration path, immediately before calling `warn_stale_starts`
directly (bypassing `start()`'s own successful write, since the point
here is to fail only the read, not the write) — this is a genuine seam
that needs no new production plumbing, since `Connection` is already
mutable and test-owned. Once Task 3's real `lookback_open_starts` is
wired in, this becomes: seed a valid DB, run `start()` once
successfully, corrupt the schema, then call `warn_stale_starts`
against the corrupted connection and assert it returns without
writing anything and without panicking. If, once Task 3's real
function lands, no such corruption reliably produces an `Err` from
`lookback_open_starts` (e.g. it happens to tolerate a missing table by
returning empty results rather than erroring), this test is dropped
and the swallowed-error path is instead documented with a single
comment at the `let Ok(findings) = ... else { return }` site, and the
gap named in this task's commit message, per the acceptance criterion's
own fallback clause.

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

`warn_stale_starts` as sketched above calls `status::lookback_open_starts`
directly, which makes "seed a specific findings list" tests
(everything above except the read-failure and regression tests)
awkward without a live DB shaped exactly right. Two options, decided
at implementation time depending on how intrusive each is once Task 3's
real signature is visible:

1. Split `warn_stale_starts` into a thin outer function (does the
   `conn` read, calls Task 3's scan) and an inner pure function
   `fn emit_warnings<W: Write>(findings: Vec<LookbackFinding>, written_date: NaiveDate, out: &mut W)` that takes the findings as a plain parameter. The outer function is trivial (one call, one delegate) and therefore doesn't need its own exhaustive tests; every case above becomes a direct, DB-free unit test of `emit_warnings`.
2. If Task 3's `LookbackFinding` type is cheap to construct directly
   (a plain struct, no private fields), skip the split and seed
   `punches` rows in a real in-memory `Connection` shaped to produce
   the desired findings from the real scan (e.g. an actual open start
   dated 5 days back for the "single" case). This is more end-to-end
   but couples every test here to Task 3's real date-window math.

Option 1 is preferred (keeps this task's tests independent of Task 3's
internal correctness, which is Task 3's own test suite's job to prove)
and is what the code sketch above should be read as implying, even
though the sketch inlines the split for readability. The plan does not
hard-commit to option 1 over option 2, since the deciding factor (how
awkward `LookbackFinding` is to hand-construct) is only knowable once
Task 3's real type exists.

## Risks, ambiguities, and disagreements

- **Both dependency rows are assumptions.** Task 3's and Task 2's plans
  don't exist yet as of this writing (checked at the start of this
  work and again just now). The assumed `LookbackFinding`/`LookbackKind`
  shape and the `lookback_warning_line` signature are this plan's best
  read of Contract A/B's prose plus §3.4/§3.6's line shapes, but they
  are not pinned by a written contract the way Task 1/2 and Task 3/5
  are. If Task 3 lands with, say, a `Vec<(NaiveDate, i64, bool, u32)>`
  tuple instead of a named struct, or folds "single vs. multiple" into
  a `count: usize` with `1` meaning single, this task's call site and
  test-seam constructors need a small adjustment — the filtering logic
  and control flow described above do not change either way. Low risk
  in practice, since the changeset's own ordering forces Task 3 to
  merge before Task 4 starts for real.
- **The changeset plan states no contract is needed between Task 3 and
  Task 4** ("it simply reads the finished code rather than a
  pinned-in-advance shape"). This plan agrees with that call for the
  *implementation* step, but had to make an assumption anyway to write
  a testable plan today, before Task 3 exists. That's a difference in
  what "no contract needed" can mean for planning vs. for coding — not
  a disagreement with the changeset plan's substance, just a note that
  "no contract" doesn't mean "no assumption," only "no assumption that
  has to be pinned before both sides start," since Task 4 doesn't
  start until Task 3 already has a real shape to read.
- **Whether the read-failure test survives to the real implementation**
  is genuinely open (flagged inline in the test plan above) — it
  depends on whether Task 3's real `lookback_open_starts` can be made
  to `Err` via a reachable, non-invasive seam (schema corruption on an
  in-memory connection). If Task 3's function instead treats a missing
  table as "no findings" rather than an error (plausible, since
  `rusqlite` queries against a dropped table do return an `Err`, but a
  future Task 3 implementation detail could catch and swallow that
  internally), this task's read-failure path becomes untestable without
  new production plumbing that's explicitly out of scope, and the
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
