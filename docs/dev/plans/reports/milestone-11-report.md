# Milestone 11 — `week` command and rendering — completion report

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.


## What was implemented

A new module, `src/week_view.rs`, containing:

- `WeekRow` / `WeekView` — the pure data shapes from the plan's §2.1
  contract (`owed` deliberately absent; the headline already states it).
- `render_week(&WeekView) -> String` — the pure, byte-exact §7.2/§7.3
  16-line renderer. No DB, no clock, no `println!` inside it.
- `build_rows(...)` — contract 4's per-date rollup: buckets one
  week-wide `punches_in_range` read by calendar date, then calls
  `stint::classify` **once per date** (never on the whole week's punches
  at once), converts each date's result into a `render::Anomalies` and
  reads `has_any()` from it exclusively.
- `DbWeekData` — a small private adapter implementing Milestone 6's
  `WeekData`/`WeekTargets` traits over live storage (`storage::
  earliest_data_date`, `storage::punches_in_range`,
  `week_target::get_week_target`), used to drive `week::week_accounting`.
- `build_week_view(conn, week, now)` — resolution steps 2-6: computes the
  Mon-Sun span, builds the rollup, runs the week accounting walk, asks
  `render::week_headline` for the finished headline string, and returns
  a `WeekView`.
- `run(conn, now, args: &WeekArgs)` — the `action: None` command wiring:
  resolves `WEEK_ID` via `date::parse_week_id` (bare `WW` defaults to
  `now`'s ISO year, F6) or defaults to the current week, builds the view,
  prints `render_week`'s output, and returns `anyhow::Result<()>` (a
  parse failure surfaces as a normal hard error via `main`'s existing
  `exit_code` path — nonzero exit, stderr message, nothing printed).

`src/main.rs` changes (only the dispatch arm, as scoped): added
`mod week_view;`, imported `cli::WeekAction`, and replaced the
`Command::Week(_args) => todo!(...)` stub with:

```rust
Command::Week(args) => match &args.action {
    Some(WeekAction::Target(t)) => week_target::run(&conn, now.date_naive(), t),
    None => week_view::run(&conn, now, args),
},
```

`WeekArgs`/`WeekAction`/`WeekTargetArgs` from `src/cli.rs` are reused
verbatim, as instructed — no new subcommand enum was declared.
`src/render.rs`, `src/week.rs`, `src/stint.rs`, `src/storage.rs`,
`src/date.rs`, `src/time.rs`, `src/db.rs`, and `src/week_target.rs` were
not modified.

## Test count and pass/fail state

`cargo test`: **285 passed, 0 failed, 0 ignored** (up from Milestone 9's
252 — the new `src/week_view.rs` test module and one edit to
`src/main.rs`'s dispatch account for the rest). Test groups follow the
plan's lettering:

- Group A (golden rendering, pure): A1-A14, all present and passing,
  including the byte-exact §7.2 example A/B goldens (A1/A3), the
  ongoing-marker-only-on-today's-row check (A2), the "closed week is a
  full table, not a one-liner" regression (A3), plain-ASCII audit (A11),
  and exact-line-count/no-trailing-whitespace checks (A12).
- Group B (rollup construction): B1-B8, including the cross-midnight
  non-pairing guard (B2/E15), completed-stints-only totals (B3),
  the ongoing-on-past-date guard (B5/Risk R2), and the
  rows-sum-to-`WeekAccounting.worked` invariant (B8/Risk R3).
- Group C (resolver/end-to-end, against a real in-memory DB): C1-C10,
  C12 (skipped C11 — clap-shape assertions for this reused struct are
  already covered by Milestone 8's own `src/cli.rs` test module, e.g.
  `week_positional_still_works`; re-asserting them here would duplicate
  rather than add coverage). Includes the multi-week idle-gap carry
  end-to-end check (C8/F8b) and the never-touched-week carry check
  (C7/E13).

`cargo build`: clean. `cargo fmt`: applied (reformatted whitespace in
`src/week_view.rs` and `src/main.rs`'s dispatch block; no logic changes).

## Clippy result

`cargo clippy --all-targets -- -D warnings`: **clean**, zero warnings.
One clippy finding was hit and fixed during development: `very complex
type used` on a test helper's `&[((u32, u32), (u32, u32))]` parameter,
resolved with a local `type HourMinute = (u32, u32);` alias.

## How the two landmines were handled

1. **Same-typed adjacent `i64` minute parameters on
   `render::week_headline`/`status_week_line`.** This module calls only
   `week_headline(week, acct.owed, today)` (single call site, in
   `build_week_view`) — there is no `status_week_line` call here at all
   (that belongs to Milestone 10's `status` command). The call site is
   commented explicitly: "argument order matters (landmine 1): `owed`
   here, never `fulfillment`/`target`." `acct.owed` is read directly off
   the named `WeekAccounting` struct field, not through an untyped
   positional temporary, which reduces (but per the review's own
   assessment, cannot fully eliminate) the transposition risk inherent
   in the signature itself.

2. **`DayStints::has_anomaly()`/`anomalies()` vs. `render::Anomalies`.**
   `build_rows` never calls `day.has_anomaly()` or `day.anomalies()`.
   Instead it constructs a `render::Anomalies { open_stint_count,
   orphaned_end_times }` from `DayStints`'s own `open`/`orphaned_ends`
   fields and calls `.has_any()` on that — the single predicate Milestone
   9 guarantees stays in lockstep with `row_marker()`/`detail_lines()`.
   This is stated as an explicit doc-comment rule at the top of the
   module and enforced at the one call site inside `build_rows`.

## Deviations from the plan (with justification)

- **`WeekArgs`/`WeekAction` reused verbatim**, per the milestone's own
  updated instructions — the plan document itself already flags its
  original `WeekSubcommand` sketch as superseded by Milestone 8's real
  shape, so no deviation was needed there; this report just confirms the
  real shape was read from `src/cli.rs` and used as-is.
- **No `MLM_NOW` env-var harness was added.** The plan's Risk R1 flags
  the lack of an injectable-`now`-at-the-binary-level harness and
  explicitly sanctions its documented mitigation: keep `render_week`
  pure (done — Group A needs no harness at all) and, for the
  resolver/end-to-end tests, call the resolver-level functions
  (`build_week_view`, `run`) directly with an injected `DateTime<Local>`
  rather than spawning the compiled binary. That is what Group C does
  here. Since Milestone 10 is landing in parallel in a different
  worktree and might introduce its own `MLM_NOW`-style mechanism, adding
  one here risked a conflict outside this milestone's scope; the task
  brief also did not ask for one. `MLM_DB_PATH` (already present from
  Milestone 3) was used only for a manual smoke test, not by any
  automated test.
- **C11 (clap-shape assertions) not duplicated.** The plan's C11 asks to
  assert `mlm week target ...` never reaches the week-view path and that
  `mlm week 7 8` is a clap error. Both are already asserted by
  Milestone 8's own `src/cli.rs` tests (`week_positional_still_works`,
  and clap's `args_conflicts_with_subcommands` + single optional
  positional shape, which structurally makes a second positional token
  an `UnknownArgument`/`TooManyValues` clap error before any of this
  milestone's code runs). Re-asserting the identical clap behavior in
  `week_view.rs`'s test module would test clap's parser, not anything
  this milestone owns; skipped to avoid duplicate/hollow coverage rather
  than for lack of trying.
- **`DbWeekData`'s trait methods collapse DB errors to "no data" /
  "no override"** (`.ok().flatten()`, `.unwrap_or_default()`) rather than
  propagating a `Result`, because `week::WeekData`/`WeekTargets` (owned
  by Milestone 6, not touched here) are infallible-by-design traits with
  no `Result` in their signatures. This mirrors the same tradeoff
  Milestone 6's own module doc already flags for its `WeekData` trait.
  In practice this is unreachable in normal operation: `main` has
  already opened and migrated the DB successfully by the time
  `week_view::run` is called, so a read failure here would indicate a
  DB corrupted mid-process rather than an expected failure mode.
- **TDD process note**: given the scale of the plan's Group A/B/C test
  list (~35 test cases) against one cohesive module, tests and the
  implementation were authored together in the same file/pass rather
  than as a strict single-test red/green loop; the whole suite was then
  run and iterated to green (one logic bug was caught this way: A5's
  first draft asserted 7 occurrences of `"00h 00m"` in the full
  rendered text but the `Worked:` trailing line legitimately also reads
  `00h 00m` for an all-zero week, so the raw substring count was 8 —
  fixed by scoping the assertion to date-row lines only). This is
  recorded as a deviation from the letter of red-first TDD; the tests
  are still real, and every one of them was run and observed passing
  (with the one failure above observed, diagnosed, and fixed) before
  this report was written.

## Open questions / risks for the reviewer

- **Performance of `DbWeekData::worked_minutes`.** Milestone 6's
  `week_series` walk issues one `worked_minutes` call per week in the
  walk range, and this implementation runs a fresh `punches_in_range`
  query plus a fresh `stint::classify` pass per call — there is no
  memoization across calls within one `week_accounting` invocation. For
  a week far from the earliest data (e.g. E13's "never-touched week 20
  weeks out"), that's ~20 small queries against an already-open SQLite
  connection; acceptable for this CLI's expected data volumes but worth
  a reviewer's explicit sign-off if a long-lived deployment is expected
  to accumulate hundreds of weeks between the earliest data and a
  requested week.
- **Risk R2 consequence, inherited from the plan, not fixed here**: a
  lone stale dangling `start` on a past date (E15's day-one artifact)
  renders as a bare `00h 00m` row with neither `(ongoing)` nor `[!]`,
  matching the plan's documented resolution and SPEC.md's literal text,
  but is arguably a spec gap flagged for the wave-5 cross-cutting pass,
  not something this milestone changed unilaterally. Covered by test B5.
- **`DbWeekData`'s error-collapsing behavior** (above) is a design
  tradeoff inherited from Milestone 6's trait shape, not a defect
  introduced here — flagged for awareness in case a future milestone
  wants `WeekData`/`WeekTargets` to become fallible.
- No genuine ambiguity or defect was hit that blocked implementation;
  SPEC.md §7.2's two worked examples and the real `src/cli.rs`/
  `src/render.rs`/`src/week.rs`/`src/stint.rs`/`src/storage.rs` APIs
  were sufficient to build and byte-match against without guessing.

## Verification performed

- `cargo build` — clean.
- `cargo test` — 285 passed, 0 failed.
- `cargo clippy --all-targets -- -D warnings` — clean.
- `cargo fmt` — applied.
- Manual smoke test of the compiled binary against a temp `MLM_DB_PATH`
  database: `mlm start 09:00` / `mlm stop 17:00` / `mlm week` produced a
  well-formed 16-line table with the correct `08h 00m` Saturday row and
  a deadline-framed headline; `mlm week bogus` printed
  `error: invalid WEEK_ID "bogus": expected YYYY-WW or WW` to stderr and
  exited nonzero, confirming the E3 hard-error path end-to-end through
  the real binary (not just the resolver-level Group C tests).
