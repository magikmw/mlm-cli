# Milestone 11 — Independent adversarial review

Reviewer: independent pass, not the implementer. Scope: `src/week_view.rs`
(the only new module) and `src/main.rs`'s dispatch arm, against
`plans/milestone-11-week-command.md`, the implementer's own report
(`plans/reports/milestone-11-report.md`), SPEC.md §3.6/§7.2/§7.3/§7.4,
PLAN.md's Milestone 11 section (interface contract 4), and
`plans/reports/milestone-9-review.md`'s two landmines, plus the already-
merged `src/cli.rs`, `src/render.rs`, `src/week.rs`, `src/stint.rs`,
`src/storage.rs`, `src/date.rs`, `src/week_target.rs` this milestone
consumes.

Verification performed directly (not taken on faith):
- `cargo build`: clean.
- `cargo test`: **285 passed, 0 failed, 0 ignored** — matches the report's
  claim exactly (re-ran after `touch`ing the changed files to force a real
  rebuild, not a cached result).
- `cargo clippy --all-targets -- -D warnings`: clean, zero warnings, on a
  forced rebuild.
- `cargo fmt --check`: clean.
- `git status --short` / `git show --stat HEAD`: the commit touches exactly
  `src/week_view.rs` (new), `src/main.rs` (dispatch arm only, +18/-9 lines),
  and `plans/reports/milestone-11-report.md` — no other file changed. No
  scope creep into `src/render.rs`, `src/week.rs`, `src/stint.rs`,
  `src/storage.rs`, `src/date.rs`, `src/db.rs`, `src/cli.rs`, or
  `src/week_target.rs`.
- Read the real signatures of every API this module calls
  (`render::week_headline`, `render::Anomalies`/`has_any`, `date::WeekId`'s
  `new`/`from_date`/`current`/`span`/`dates`, `date::parse_week_id`,
  `date::format_date`, `week::WeekData`/`WeekTargets`/`week_accounting`,
  `stint::classify`/`DayStints::completed_minutes`/`is_ongoing`/`open`/
  `orphaned_ends`, `storage::punches_in_range`/`earliest_data_date`/
  `insert_punch`/`punch_from_local`) directly in their defining files, not
  from the plan's guessed shapes — every call site's argument types and
  order match the real definitions.
- Hand-traced `build_rows` and confirmed it is `std::array::from_fn` over a
  fixed `[NaiveDate; 7]`, i.e. `stint::classify` is called **exactly seven
  times** per requested week (once per index `0..7`, empty bucket included
  via `.unwrap_or_default()`), never once over a whole date range — this is
  structural (the loop shape enforces it), not merely commented.
- Hand-verified both §7.2 golden strings (`EXAMPLE_A_GOLDEN`,
  `EXAMPLE_B_GOLDEN` in `src/week_view.rs:335-371`) byte-for-byte against
  SPEC.md §7.2's literal text — identical, including the closed week's full
  7-row table and 4-line trailing block (not a shortened form), the
  `-02h 10m`/`00h 00m` carry-in values, and the colon in
  `Total still owed: 03h 10m`.

## Findings

1. **No finding — landmine 1 (same-typed adjacent `i64` params) does not
   apply here, and the report's framing overstates the residual risk
   slightly.** `src/week_view.rs:223` calls only
   `render::week_headline(week, acct.owed, today)` — a `(WeekId, i64,
   NaiveDate)` signature with a single `i64` parameter, not the three
   adjacent same-typed `i64`s that made `status_week_line` dangerous in
   Milestone 9's review. There is no `status_week_line` call anywhere in
   this module (confirmed via grep — the only call site in the whole
   workspace naming `status_week_line` is Milestone 9's own test module).
   So there is structurally no argument-order transposition available at
   this call site beyond swapping `week`/`today`, which is caught by the
   type checker (`WeekId` vs `NaiveDate`). The report's own text
   acknowledges this reduces but "cannot fully eliminate" the risk; in
   fact for this specific call site the risk is eliminated by the type
   signature itself, not just reduced by a comment. Not a defect —
   recorded because the review brief asked for explicit confirmation of
   this landmine. No severity (informational).

2. **No finding — landmine 2 (`render::Anomalies` vs `DayStints::
   has_anomaly()`/`anomalies()`) is correctly avoided.** Grepped the new
   module for `.has_anomaly(` and `.anomalies(` — zero direct calls.
   `build_rows` (`src/week_view.rs:140-147`) constructs a
   `render::Anomalies { open_stint_count: day.open.len(),
   orphaned_end_times: day.orphaned_ends.iter().map(...).collect() }` from
   `DayStints`'s raw fields and calls `.has_any()` on that value exclusively.
   This is the single predicate Milestone 9's own tests (T29-T31) guarantee
   stays in lockstep with `row_marker()`/`detail_lines()`. No finding.

3. **Contract 4 (per-date rollup, never a whole-week batch) verified
   structurally, not just by comment.** `build_week_view` issues one
   `storage::punches_in_range(conn, start, end)` call for the whole span
   (§3.1's recommended single-round-trip design), buckets by `date` into a
   `HashMap<NaiveDate, Vec<Punch>>`, then `build_rows` calls
   `stint::classify` once per date via `std::array::from_fn` over the fixed
   7-date span — confirmed by reading the loop, not trusting the doc
   comment. Test B2 (`b2_bucketing_does_not_pair_across_midnight`) directly
   exercises the E15 guard: a `start 23:30` Tue / `end 00:45` Wed pair
   produces an open stint on Tue (0 minutes, no anomaly) and an orphaned
   end on Wed (0 minutes, `has_anomaly == true`) — no 1h15m cross-midnight
   stint appears. No finding.

4. **`(ongoing)` gating verified against real `is_ongoing()` semantics, not
   just the plan's intent.** `WeekRow.is_ongoing = day.is_ongoing() && date
   == today` (`src/week_view.rs:153`) — `today` comes from `now.date_naive()`
   in `build_week_view`, injected, never read from a hidden clock. Test B5
   (`b5_is_ongoing_false_on_past_date_with_dangling_start`) directly proves
   the Risk-R2 resolution: a past week's Monday with an unmatched trailing
   `start`, evaluated with `today` well after that week, renders
   `is_ongoing == false`. No finding.

5. **`WEEK_ID` bare-number-defaults-to-current-year (F6) verified against
   the real `date::parse_week_id`, not a guess.** `run()`
   (`src/week_view.rs:242-245`) calls `parse_week_id(s, today)` — the real
   Milestone 2 function — for `Some(s)`, and `WeekId::current(today)` for
   `None`. Test C2 independently derives `parse_week_id("7", ...)` and
   `parse_week_id("2026-07", ...)` and asserts equality before also
   checking the rendered view — this exercises the real parser end to end,
   not a stub. No finding.

6. **§7.2's two worked examples are reproduced byte-for-byte, and the
   closed-week example is the full form, not a regression to a one-liner.**
   `EXAMPLE_B_GOLDEN` (`src/week_view.rs:354-371`) contains all 7 date rows
   and the full `Carry-in:`/`Worked:`/`Fulfillment:`/`Target:` block; test
   A3 additionally asserts positive presence of all 7 date strings and all
   4 trailing labels as an explicit regression guard (not just relying on
   the golden string equality). This matches SPEC.md §7.2's second example
   and PLAN.md's explicit acceptance criterion. No finding.

7. **E13 (never-touched week) and F8b (idle-gap carry) verified against
   the real accounting walk, end to end.** C7 seeds only week `2026-01`
   with 8h of data and requests week `2026-06` (5 weeks later, no data in
   between or after): all 7 rows are `00h 00m`, target is the default
   `2400`, and `carry_in_minutes` is asserted `!=` both `0` and week 1's own
   isolated `carry_out` — i.e. it is genuinely the accumulated walk, not a
   naive skip or a single-hop carry. C8 seeds week N and week N+2 with data
   and leaves week N+1 empty, then asserts week N+2's `carry_in` equals
   week N+1's own `carry_in - 2400` (i.e. the full deficit of the idle week
   is visible, not skipped). Both ran against `week::week_accounting`'s
   real merged implementation (confirmed by reading `src/week.rs:150-200`'s
   walk, which has no "skip empty weeks" shortcut) via the `DbWeekData`
   adapter, not a stub. No finding.

8. **Scope confirmed clean.** `git show --stat 879ebca` lists only
   `src/week_view.rs` (new), `src/main.rs` (dispatch arm), and the report
   markdown. `src/cli.rs`'s `WeekArgs`/`WeekAction`/`WeekTargetArgs` are
   byte-identical to Milestone 8's shape (diffed by inspection — no new
   `WeekSubcommand` enum, no `week_id` re-declaration), and
   `src/week_target.rs` is untouched and still dispatched from `main.rs`
   exactly as Milestone 8 left it. No finding.

9. **`src/week_view.rs:182-193` (`DbWeekData::worked_minutes`) is a second,
   independently-run realization of "completed-stint minutes for a date",
   used across every week `week_accounting`'s walk visits — not memoized
   across the walk (minor, self-disclosed).** For a request far from the
   earliest data week (E13's scenario), the walk in `week::week_series`
   calls `worked_minutes` once per intervening week, each of which reissues
   its own `punches_in_range` query and a fresh `stint::classify` pass per
   date-with-punches in that week. This is not a correctness bug — B8
   (`b8_rows_sum_to_week_accounting_worked`) asserts the row-sum invariant
   against a real accounting run, and both paths bottom out in the same
   `stint::classify(...).completed_minutes()` primitive (Risk R3's
   recommended mitigation), so they cannot silently diverge. It is a
   real, if small, N-query cost for a CLI operating on an already-open
   local SQLite connection over what will typically be tens, not hundreds,
   of weeks. The report already discloses this explicitly as an open risk.
   Severity: **minor**.

10. **`DbWeekData`'s trait methods collapse a storage error to "no data"/
    "no override"** (`.ok().flatten()` in `earliest_data_week` and
    `target_override`, `.unwrap_or_default()` in `worked_minutes`) rather
    than surfacing a DB failure as a hard error. Traced: `week::WeekData`/
    `WeekTargets` are infallible-by-design traits (no `Result` in their
    signatures, per Milestone 6's own module doc), so there is no way for
    this adapter to propagate a `Result` without changing Milestone 6's
    trait shape, which is correctly out of this milestone's scope. In
    practice unreachable in normal operation since `main` has already
    opened and migrated the DB by the time `week_view::run` executes — a
    failure here would mean the DB became corrupted mid-process. Correctly
    self-disclosed in the report as a design tradeoff, not a defect
    introduced here. Severity: **minor** (pre-existing constraint from
    Milestone 6, not this milestone's to fix).

11. **Risk R2's documented consequence (E15's stale dangling start on a
    past date renders as a bare `00h 00m` row with neither `(ongoing)` nor
    `[!]`) is real, inherited from SPEC.md's literal text, and correctly
    not "fixed" unilaterally by this milestone.** Confirmed by B5's
    assertion and by re-reading SPEC.md §7.2's own prose
    ("That stale date renders as a plain total excluding the open stint's
    live minutes, with no `(ongoing)` and no `[!]` marker — an accepted,
    silent consequence of §1.2's limitation, not a bug in this rule").
    Correctly flagged for the wave-5 cross-cutting pass rather than
    changed here. No new finding beyond what the plan and report already
    surface.

## Verdict

**APPROVE** — build, full test suite (285/285), `cargo fmt --check`, and
`cargo clippy --all-targets -- -D warnings` were all independently
re-verified clean on a forced rebuild, not taken on faith. Both of
Milestone 9's landmines were traced structurally and are correctly
handled (landmine 1 does not even apply to this module's sole
`week_headline` call site; landmine 2's seam is used exclusively through
`render::Anomalies::has_any()`, confirmed by grep). Contract 4's per-date
rollup is provably seven `stint::classify` calls via the array-loop
shape, not a whole-week batch, and B2 directly proves the E15
cross-midnight guard holds. Both §7.2 worked examples are reproduced
byte-for-byte, with the closed week correctly rendering the full 7-row
table and trailing block rather than a regressed one-liner. E13 and F8b
were verified against the real, un-stubbed `week::week_accounting` walk,
not fixtures standing in for it. Scope is exactly the new module plus the
minimal `main.rs` dispatch change, with every already-merged module left
untouched. The only findings are pre-existing/self-disclosed minor
tradeoffs (unmemoized per-week query cost in the carry walk; infallible
trait shape inherited from Milestone 6) that do not block merge.
