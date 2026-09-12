# Milestone 8 — `week target` command — independent adversarial review

Reviewed fresh against SPEC.md §3.7/§3.6/§4.2/§2.3/§6.1, PLAN.md's
contract 7 and the Milestone 8 canonical-shape guidance, and the
already-merged real APIs in `src/date.rs`, `src/time.rs`, `src/db.rs`.
Verification performed: read of all touched/relevant source, `cargo
build`, `cargo test`, `cargo clippy --all-targets -- -D warnings`,
`cargo fmt --check`, and manual `cargo run -- week target ...` against
all the arity cases named in the task.

## Findings

- **`src/main.rs:29-31`** — `Command::Week(_args) => todo!(...)`. Running
  `mlm week target 2026-07 33h30m` (or any `week`/`week target`
  invocation) against the real built binary panics (exit 101, an
  unwind message on stderr) instead of writing anything. Verified by
  hand: `cargo run -- week target 2026-07 33h30m` panics. This means
  the feature is not usable end-to-end yet — the acceptance criteria
  C1-C6 style "run the real binary" checks cannot pass today. This is
  explicitly called out by both the report and the task's scope
  boundary ("do not touch main.rs's command dispatch") as deferred to
  Milestone 7/11's wiring pass, and PLAN.md's own merge order (7 → 8 →
  10 → 11) anticipates Milestone 8 landing before real dispatch exists.
  Not a defect in this milestone's own deliverable, but it is a real
  gap the next integrator must close before `week target` is a shippable
  feature; flagging so it isn't lost. Severity: **minor** (matches
  documented, deliberate scope; would be a blocker if silently
  unmentioned).

- **`src/week_target.rs:64`** — `debug_assert!(minutes >= 0, ...)` is the
  only "defensive" check between `parse_duration` and the DB `CHECK`.
  `debug_assert!` compiles to a no-op in `--release` builds, so if a
  future change to `time::parse_duration` ever let a negative value
  through `Ok(...)`, a release binary would silently send it straight
  to `set_week_target`, relying entirely on the SQLite `CHECK` (an
  opaque `rusqlite::Error` / constraint-violation message, not the
  friendly §6.1 domain error the report itself says this guard exists
  to preserve). As implemented today `parse_duration` cannot structurally
  return a negative `Ok` (verified by reading `time.rs`: the negative
  branch always returns `Err(DurationParseError::Negative(..))`), so
  this is inert right now, not a live bug — but the comment/report
  frame it as a real defensive layer when in release mode it provides
  zero defense. Severity: **minor** (dead-in-release safety net,
  documented as a known trade-off in the report itself).

- **`src/week_target.rs` / `src/cli.rs`** — everything else checked
  came back clean: `ON CONFLICT(week_id) DO UPDATE SET target_minutes =
  excluded.target_minutes` is a genuine upsert (not `INSERT OR
  REPLACE`), confirmed both in `week_target.rs` and independently
  exercised in `db.rs`'s own T9; the key passed is always
  `week_id.to_key()` (§4.1's normalization requirement), never the raw
  token, and `set_week_target_keys_on_normalized_id` proves `2026-7`
  and `2026-07` collide on one row; zero-duration is accepted end to
  end (`R3`/`U2`, matching §6.1/F7b's final "zero legal, only negative
  rejected" rule, not an older "no zero" draft); malformed `WEEK_ID` is
  validated through the real `date::parse_week_id`/`WeekId::new`, which
  does a genuine per-year `iso_weeks_in_year` check (`R5` uses
  `2027-53`, a real 52-week year, not a stale flat range test); WEEK_ID
  is validated before DURATION per the plan's pinned order (`R8`); the
  clap shape was hand-verified at the real binary: `week target` with 0
  args is clap's `MissingRequiredArgument` (exit 2, E10), 5 tokens
  (`a b c` after the subcommand, i.e. 3 positional args) is a clap
  unexpected-value error (exit 2), and `week target -5h` parses
  successfully at the clap layer (reaches the `todo!()` panic rather
  than an "unknown flag" error), proving `allow_hyphen_values = true`
  is actually wired and load-bearing as intended. `WeekArgs`/
  `WeekAction`/`WeekTargetArgs` match PLAN.md's pinned canonical shape
  verbatim (`args_conflicts_with_subcommands`, the single variadic
  `args: Vec<String>` with `num_args = 1..=2`), so Milestone 11 has a
  clean, unmodified type to reuse. Error propagation throughout `run()`
  is plain `?` into `anyhow::Result<()>` with no hand-rolled `From`
  impls, consistent with PLAN.md's final `anyhow`-only contract 7 (no
  `AppError` enum anywhere in this milestone's code). No `.unwrap()`/
  `.expect()` on any user-input path in `week_target.rs`. Scope is
  exactly `src/cli.rs`, `src/main.rs`, `src/week_target.rs` (confirmed
  via `git show --stat` on the milestone commit) — `db.rs`, `time.rs`,
  `date.rs`, `stint.rs`, `week.rs`, and the pre-existing `Start`/
  `Stop`/`Log` variants are untouched except for a mechanical, additive
  `#[derive(Debug)]` on `Cli`/`Command` needed only so tests can
  `unwrap_err()` — no behavioral change. `cargo build`, `cargo test`
  (150 passed, 0 failed, matching the report's count), `cargo clippy
  --all-targets -- -D warnings`, and `cargo fmt --check` are all clean,
  confirming the report's claims rather than taking them on faith.

## Verdict

**APPROVE WITH NITS** — the implementation is correct against SPEC.md
and faithfully adapted to the real upstream APIs rather than the plan's
guesses; both findings above are minor, already self-documented by the
implementer, and don't block merging this milestone's own scope. The
one thing worth tracking as a follow-up (not a rework of this
milestone) is that `mlm week target` is not actually runnable end to
end until a later milestone replaces the `todo!()` dispatch stub in
`src/main.rs`.
