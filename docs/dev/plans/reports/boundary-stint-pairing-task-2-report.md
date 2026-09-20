# Task 2 completion report: call-site rewiring (`src/status.rs`, `src/week_view.rs`)

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.


**Branch**: `worktree-agent-a881fe57d02eaa8c4`. The worktree was behind
`boundary-stint-pairing`'s current tip (at `50e3e6a`, its grandparent) when
started — `git rebase boundary-stint-pairing` was run first (fast-forward,
no conflicts, no other unique commits on this branch) to land on `973af17`
("Merge Task 1"). Noting this explicitly per the pre-flight instruction.

**Plans/spec read in full before starting**:
`docs/dev/plans/reports/boundary-stint-pairing-task-1-report.md`,
`docs/dev/plans/boundary-stint-pairing-task-2-callsites.md`,
`docs/dev/plans/boundary-stint-pairing-plan.md`,
`docs/dev/specs/2026-09-19-boundary-stint-pairing.md` (§4.3/§4.4
especially, read in full). Every line number the task plan cited in
`src/status.rs`/`src/week_view.rs` was verified against the real files
before editing and matched exactly — no drift since the plan was written.

## What was implemented

All four `classify()` call sites now call `stint::classify_at`, each
fetching one extra calendar day of punches on the appropriate side(s):

1. **`status.rs::resolve`**: fetches `target_date.pred_opt()`/`succ_opt()`
   punches alongside the existing fetch, calls `classify_at` with all
   three slices.
2. **`status.rs::build_ledger`**: the day-by-day ledger walk fetches the
   previous/next calendar date's punches at each iteration, inside the
   existing `if !day_punches.is_empty()` branch (matching the task
   plan's own judgment call — see Deviations, though this one isn't
   really a deviation since the plan itself recommended it).
3. **`week_view.rs::build_week_view`**: widens its `punches_in_range`
   fetch by one day on each side (`start.pred_opt().unwrap()` ..
   `end.succ_opt().unwrap()`).
4. **`week_view.rs::build_rows`**: looks up each date's `pred_opt()`/
   `succ_opt()` neighbor in the same `by_date` map it already builds
   (`unwrap_or_default()` — a missing neighbor bucket and "no such
   date" collapse to the same `&[]` case, both correct per
   `classify_at`'s no-panic contract), calls `classify_at` instead of
   `classify`. Its stale "out of scope (E15)" doc comment is rewritten
   to describe the padded-neighbor lookup instead.
5. **`week_view.rs::DbWeekData::worked_minutes`**: fetch widens the same
   way; the old `by_date.values().map(...).sum()` pattern is **fully
   removed** (not left alongside the fix) and replaced with
   `week.dates().into_iter().map(|date| { ... }).sum()`, calling
   `classify_at` per date with that date's neighbors pulled from the
   padded map. Verified by grep that no `.values()` call remains
   anywhere in the function body.

Grep sweep for stale `E15|cross-midnight|out of scope|out-of-scope|
midnight` in both files (post-edit) found only the intentional new
`midnight`-prefixed test/comment names describing the *new* behavior —
no remaining claim that cross-midnight pairing is out of scope.
`src/status.rs` had zero hits before or after (matching the task plan's
own sweep). Grep for `\.pred()\|\.succ()\|Days::new\|+ Days\|- Days` in
both files: zero hits — every new call site uses `pred_opt()`/
`succ_opt()` with `.unwrap()` (fetch-range sites) or
`.and_then(...).unwrap_or_default()` (neighbor-bucket-lookup sites in
`build_rows`/`worked_minutes`), never the panicking forms.

CI e2e smoke: added a new block to `.github/workflows/ci.yml`, appended
at the end of the file (after the existing "future --date is a hard
error" delete-block check) — a fresh, isolated `$midnight_db`, a
`start -d 2020-01-01 23:30` / `stop -d 2020-01-02 00:45` pair, and three
assertions: `status 2020-01-01` shows `Day total: 01h 15m` with no
`[!]`; `status 2020-01-02` shows no `[!]` and no `00:45` line of its
own. Ran this exact block by hand against a real `cargo build --release`
binary (not just trusted the YAML) — output:

```
--- midnight: start/stop straddling midnight via -d ---
--- midnight: status on the start date shows the completed total ---
Wed 2020-01-01

Day total:     01h 15m
Week 2020-01:  Total still owed: 38h 45m

  23:30-00:45  (01h 15m)
--- midnight: status on the end date shows no anomaly ---
Thu 2020-01-02

Day total:     00h 00m
Week 2020-01:  Total still owed: 38h 45m
ALL_MIDNIGHT_SMOKE_CHECKS_PASSED
```

All three assertions passed against the real built binary.

## Test-first evidence (strict TDD)

**`src/status.rs`** — four new tests added to `mod tests` *before* either
call site was touched:
- `resolve_midnight_splice_earlier_date_shows_completed_stint_no_anomaly`
- `resolve_midnight_splice_later_date_shows_no_orphan_anomaly`
- `resolve_midnight_boundary_still_flags_non_1to1_shapes`
- `resolve_midnight_splice_reflected_in_week_line`

Ran `cargo test --lib status::` before implementing: 3 of the 4 failed
as expected (`earlier_date` expected 75 got 0; `later_date` found the
un-spliced `[!] orphaned end at 00:45 ...` line still present;
`reflected_in_week_line` found `fulfillment 00h 00m` instead of `01h
15m`). `resolve_midnight_boundary_still_flags_non_1to1_shapes` passed
even under the old code — traced by hand and confirmed correct: with
two trailing opens on the earlier date (not 1:1), the splice gate was
never going to fire either before or after this task's changes, so
that specific fixture doesn't discriminate old-vs-new call-site wiring
on its own (its job is the negative-control assertion, not to catch
the wiring bug). Implemented both `status.rs` call-site edits; all 4
new tests plus all 27 pre-existing `status::` tests passed, zero
pre-existing test edited.

**`src/week_view.rs`** — `b9`, `c13`, `c14` added and `b2` rewritten in
place *before* any of the four `week_view.rs` call sites were touched:
- `b9_week_boundary_splice_lands_in_correct_weeks_rows_only` (new)
- `c13_week_boundary_splice_end_to_end_lands_in_starting_weeks_view_only` (new)
- `c14_worked_minutes_padding_day_isolation` (new)
- `b2_bucketing_does_not_pair_across_midnight` → rewritten in place to
  `b2_bucketing_now_splices_across_midnight`, asserting the new,
  correct outcome (old assertions were the exact behavior this
  changeset removes — see Deviations/notes below, this is the one
  place an existing test's assertions are flipped, not just extended)

Ran `cargo test --lib week_view::` before implementing:
- `b2_bucketing_now_splices_across_midnight` **failed** (expected 75
  minutes on Tue, got 0 — the old un-spliced behavior).
- `b9_week_boundary_splice_lands_in_correct_weeks_rows_only` **failed**
  (expected 80 minutes on the Sunday row, got 0).
- `c13_week_boundary_splice_end_to_end_lands_in_starting_weeks_view_only`
  **failed** (expected 80 minutes on the Sunday row via the real
  `build_week_view` DB-backed path, got 0).
- `c14_worked_minutes_padding_day_isolation` **passed even before the
  implementation change.** This is expected, not a test-quality bug:
  before `worked_minutes`'s fetch is widened to 9 days,
  `punches_in_range(start, end)` never even fetches the padding-day
  punches in the first place, so the old `by_date.values().sum()` code
  correctly excludes them too — for the wrong reason (never seeing
  them) rather than the right one (seeing them but not enumerating
  them). This fixture is deliberately built (per the task plan itself)
  to distinguish the *fixed* implementation from a version that widens
  the fetch but forgets to also fix the iteration — i.e. it's a
  regression guard against a specific way to get *this* task wrong,
  not a test that fails against the pre-Task-2 code. It stayed green
  through the whole implementation, confirming the `week.dates()`
  rewrite never regressed it.

Implemented all five `week_view.rs` edits (fetch widening ×2,
`build_rows`'s neighbor lookups + doc comment, `worked_minutes`'s full
rewrite); all `week_view::` tests (36 total, including the 3
newly-red-then-green ones, `c14`, and the rewritten `b2`) passed, plus
every other pre-existing test in the module (including `c7`/`c8`,
confirmed still passing unmodified, per the task plan's explicit
requirement).

## Full verification, all green

- `cargo fmt --check`: initially failed (5 spots in the newly-added
  test code, same shape of issue Task 1 hit — multi-line literals
  rustfmt preferred differently than typed). Ran `cargo fmt` once, then
  `cargo fmt --check` passed clean. No hand-editing.
- `cargo build`: clean, no warnings.
- `cargo test` (full crate): **435 tests total, all green** — 425 lib
  tests (including all `status::`/`week_view::`/`stint::` tests), 2
  `main.rs` tests, 1 ascii-sweep integration test, 2 `e6_db_failure`
  integration tests, 5 `status_cli` integration tests, 0 doctests. This
  is the first `cargo test` run where the crate compiles end-to-end
  with `classify_at` actually in use (Task 1 left it uncalled outside
  its own tests, by design).
- `cargo clippy --all-targets -- -D warnings`: clean, zero warnings.
- `cargo clippy --all-targets -- -W clippy::cognitive_complexity`: zero
  cognitive-complexity warnings anywhere in the crate — `build_ledger`
  and `worked_minutes`'s growth (both flagged by the task plan as worth
  a spot check) stayed well under `clippy.toml`'s threshold-15 without
  needing any extraction.
- Pre-commit hook (`.githooks/pre-commit`), run automatically on `git
  commit`: **all three gates green** —
  ```
  ==> quality gate: cargo fmt
      OK
  ==> quality gate: cognitive complexity
      OK (no functions exceed the complexity threshold)
  ==> quality gate: test coverage (vs. baseline)
      OK: coverage improved (98.93238434163702% -> 98.9556135770235%), baseline updated
  ```
  `coverage-baseline.json` was updated automatically by the hook (only
  ever raised, never lowered, per its own doc comment) — this is an
  expected side effect of the hook running on commit, not a manual
  edit.
- CI e2e smoke block: run by hand against a real `cargo build --release`
  binary (see above) — all three assertions passed.

## Deviations from the task plan, with reasons

1. **`build_ledger`'s fetch placement**: implemented exactly as the
   task plan's own sketch (inside `if !day_punches.is_empty()`), not a
   deviation — noting only because the plan itself flagged this as a
   judgment call for the coordinator to weigh in on. I agree with the
   plan's reasoning (an idle day can never produce a splice-affecting
   result, and this isn't a rolling-window optimization since it
   doesn't reuse a fetch across iterations) and made no different call
   here.
2. **`unwrap_or_default()` vs `.unwrap()` at neighbor-bucket lookups**
   in `build_rows`/`worked_minutes`: implemented exactly as the task
   plan's sketch, not a deviation from it, but (per the plan's own
   flag) a place where constraint 6's literal wording ("...with
   `.unwrap()`") is read as "never the panicking `pred()`/`succ()`
   forms," not "literally always `.unwrap()`". At the four
   *fetch-range* call sites I do use `.unwrap()` verbatim, matching the
   spec's own worked examples. At the neighbor-*lookup* sites inside
   the `build_rows`/`worked_minutes` per-date closures, a missing
   neighbor is an ordinary, already-meaningful "no such date" case that
   the interface contract already treats as `&[]`, so
   `unwrap_or_default()` is used instead. Flagging for the reviewer's
   explicit sign-off per the task plan's own request, not because I
   have a different reading than the plan already laid out.
3. **`resolve_midnight_boundary_still_flags_non_1to1_shapes`'s expected
   outcome for `D+1`**: the task plan's own test-plan prose (§Test
   plan, AC: `status.rs::resolve`) states this test should assert
   `resolve(D+1)` shows "no anomaly on that side either" once `D`'s
   splice gate fails (2 opens on `D`, not 1:1). I traced this by hand
   against `render::Anomalies::has_any()`/`detail_lines()`
   (`src/render.rs:118-144`) and the crate's own pre-existing test
   `resolve_e7_multi_open_and_e8_orphaned_end_via_anomalies_only`
   (`src/status.rs`, unmodified, still green): an orphaned end
   *always* produces its own `[!] orphaned end at ...` anomaly line
   regardless of how many open stints exist elsewhere that same
   date — `has_any()` is `open_stint_count > 1 || !orphaned_end_times.is_empty()`,
   an OR, not conditioned on the open count. Since `D`'s 2-open shape
   fails the splice gate (not 1:1), `D+1`'s `00:30 End` never gets
   consumed and stays a genuine orphan, which **must** render its own
   anomaly line — matching the pre-existing test's own established
   behavior for the same-date case. I implemented and shipped the test
   asserting the actually-correct outcome
   (`view_d2.anomaly_lines.len() == 1`, containing `"orphaned end at
   00:30"`) rather than the plan's stated expectation, and documented
   the reasoning directly in the test's own doc comment. This is a
   deviation from the task plan's prose, not from the spec or the
   shipped `classify_at`/`Anomalies` behavior — both of which this test
   now correctly pins down.
4. **CI block placement**: placed at the very end of
   `.github/workflows/ci.yml` (pure append), per the task plan's own
   stated preference ("I'd lean towards end of file instead... to keep
   the diff to a pure append and avoid any risk of subtly perturbing an
   existing block"), rather than its alternative thematic-grouping
   sketch (right after the backdated-punch block). The plan flagged
   both as defensible and asked the coordinator/reviewer to pick one;
   I went with the plan's own stated lean.
5. Everything else (signatures, doc-comment text, `b2`'s
   rewrite content, `c13`/`c14`'s fixture shapes, CI script body)
   matches the task plan's sketches as written — no other substantive
   deviation.

## Open questions for the reviewer

- **Deviation 3 above** is the one place this task's shipped test
  assertion differs from the task plan's own stated expectation. I'm
  confident in the trace (it's directly backed by a pre-existing,
  unmodified, still-green test in the same file), but since it
  contradicts the plan's explicit prose, it's worth a reviewer's
  explicit sign-off rather than assuming my reading is obviously
  right.
- **Deviation 2** (`unwrap_or_default()` at neighbor-lookup sites) is
  carried over unchanged from the task plan's own flagged risk #2 — it
  asked for the same reviewer sign-off there; repeating that request
  here since nothing about implementing it changed the plan's own
  analysis.
- `build_ledger`'s per-iteration `classify_at` call has no direct unit
  test in isolation (it's a private function; the task plan's own risk
  #5 already accepts this, since `build_ledger` was already only
  tested indirectly through `resolve` before this task). Same
  limitation carried forward, not introduced by this task.
- No other known gaps. `cargo test`, `cargo clippy -D warnings`, the
  pre-commit hook (fmt/complexity/coverage), and the CI e2e smoke block
  (run by hand against a real release binary) are all green.
