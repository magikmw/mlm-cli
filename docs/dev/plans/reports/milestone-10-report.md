# Milestone 10 — `status` command and rendering — completion report

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.


## What was implemented

- **`src/status.rs`** (new module): the full three-layer split from the
  plan.
  - `StatusView` and its component types (`EodState`, `DailyTargetHint`,
    `StintEnd`, `StintLine`) — plain owned data, all values pre-computed.
  - `render(&StatusView) -> String` — pure, no I/O, no clock. Implements
    the §3.1 layout algorithm exactly: header, blank, day-total line,
    week line, then conditionally-omitted anomaly/stint/notes blocks,
    each preceded by one blank line, ending in exactly one trailing
    `\n`.
  - `resolve(date_arg, now, conn) -> anyhow::Result<StatusView>` — all
    I/O and arithmetic, following the plan's §2.2 resolution order
    verbatim (parse DATE against `now`'s local date first, read
    punches/notes for `target_date`, classify via M5, account
    `target_date`'s own week via M6, call M9's headline/anomaly helpers,
    then gate the daily-target/EOD hints on `is_today` alone).
  - `run(conn, now, date_arg) -> anyhow::Result<()>` — the CLI entry
    point; prints the rendered view to stdout with `print!` (the string
    already carries its own trailing newline) and propagates errors for
    `main`'s existing `exit_code` to turn into a nonzero exit + stderr
    message.
  - A private `build_ledger` helper loads a `week::WeekLedger` covering
    exactly the span `week::week_series` would walk (earliest data week,
    or `through` itself if there's no data, through the target week),
    using only the existing public `storage`/`week_target` APIs —
    `storage.rs` was not touched.
- **`src/cli.rs`**: added `Command::Status { date: Option<String> }`
  (positional, no flags, matching §3.5's `mlm status [DATE]`). The plan's
  suggestion to also delete a leftover `Log` scaffold subcommand turned
  out to be already moot — Milestone 7 had already removed it before
  this worktree branched (confirmed: no `Log` variant exists in the
  merged `cli.rs`).
- **`src/main.rs`**: added `mod status;` and wired
  `Command::Status { date } => status::run(&conn, now, date.as_deref())`
  into `dispatch`.
- **`tests/status_cli.rs`** (new): binary-level tests (T14, T15) driving
  the compiled binary via `std::process::Command` +
  `env!("CARGO_BIN_EXE_mlm")` + `MLM_DB_PATH`, per the plan's declined-
  `assert_cmd` infrastructure note. Covers the exit-code/stdout-vs-stderr
  contract (§6.3) plus a fresh-empty-DB sanity check.

### Test coverage against the plan's §5 list

All of F1-F11, E7, E8, E11 render-level goldens are implemented as
listed (T1-T13), plus the two `resolve`-level tests explicitly called
out for F10/F11 (asserting `is_today` and M9's independent
week-currency check are never conflated), an `E7+E8` `resolve`-level
test, a malformed-DATE `resolve`-level hard-error test, and a note-only
`resolve`-level test. T14/T15 are implemented at the binary level as
specified. **T16 (DST)** is explicitly **not implemented** — the plan
itself flags it as blocked on a not-yet-decided "local timezone source
in tests" mechanism, and `status`'s local-time display conversions go
through `chrono::Local` (matching Milestone 7's own established
pattern for `start`/`stop`/`note`), which has no fixed-offset override
point to exercise a real DST boundary deterministically. Flagging this
as a known gap rather than inventing a second, ad hoc mechanism here.

## Test count and pass/fail state

- `cargo test`: **274 passed** (library, up from the pre-existing 252 —
  22 new tests in `status.rs`) **+ 3 passed** (new `tests/status_cli.rs`
  binary-level suite) = **277 passed, 0 failed**.
- `cargo build`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean, no warnings (one
  `vec_init_then_push` lint was hit and fixed during development by
  building `lines` with a `vec![...]` literal instead of four `push`
  calls).
- `cargo fmt`: applied.

## How the two landmines were handled

1. **`render::status_week_line`/`week_headline`'s adjacent same-typed
   `i64` params.** All calls in this module go through one private
   helper, `fn week_line(acct: &week::WeekAccounting, today: NaiveDate)
   -> String`, which reads `acct.owed`, `acct.fulfillment`, `acct.target`
   by field name rather than positionally at each call site. There is
   exactly one place in the whole module where the positional call
   happens, it is short, and it is covered by a dedicated test
   (`week_line_matches_status_week_line_field_for_field`) that compares
   its output against a direct call to `render::status_week_line` with
   the same fields, so a future transposition inside the helper itself
   would be caught. `resolve()` never calls `render::status_week_line`
   directly — always through `week_line`.
2. **`DayStints::has_anomaly()`/`anomalies()` vs. `render::Anomalies`.**
   `status.rs` never calls either of those two `DayStints` methods.
   `to_anomalies(&DayStints) -> render::Anomalies` builds the value
   directly from `DayStints`'s raw `open` (via `.len()`) and
   `orphaned_ends` fields, converting each orphan's punch instant to a
   local `NaiveTime`. From that point on, anomaly rendering (`has_any()`,
   `detail_lines()`) goes exclusively through `render::Anomalies`, which
   is the module Milestone 9's own tests bind together. This is called
   out explicitly in the module's top doc-comment so it isn't lost on a
   later edit.

## Deviations from the plan (with justification)

- **`StatusView` carries one pre-rendered `week_line: String` field
  instead of the plan's suggested `week_headline: String` +
  `week_id: String` + `week_detail_suffix: Option<String>` triple.**
  `render::status_week_line` (Milestone 9) already assembles the
  complete label + headline + optional parenthetical in exactly the
  form §3.5's L4 needs, so composing it a second time inside this
  module's `render()` would either duplicate that logic or require
  `render()` to re-derive "is this the current week" itself — which
  PLAN.md/§4 explicitly forbids Milestone 10 from doing. Storing the
  single finished string keeps that decision entirely inside the one
  `week_line()` call site (see landmine 1 above) instead of spreading it
  across `resolve` and `render`.
- **`week_line()`'s existence at all** is itself a deviation-by-addition
  beyond what the plan's `StatusView` shape implies, but it is exactly
  the "local named-fields helper" the milestone brief suggested
  considering for the landmine.
- **`Log` scaffold removal (plan §2.1/§6 A10) was a no-op.** Milestone 7
  had already removed it before this worktree's `milestone-10` branch
  point; verified by inspecting the merged `cli.rs`. No action was
  needed or taken.
- **T16 (DST) skipped**, as detailed above — matches the plan's own
  "blocked, coordinate with whatever M4 settles on" framing; nothing in
  the merged code between M2-M9 introduced such a mechanism, so
  inventing one unilaterally here felt more likely to diverge from a
  future decision than to help.
- **`build_ledger` iterates dates one at a time via the existing
  `punches_for_date`/`notes_for_date` calls** rather than adding a new
  `notes_in_range` bulk-read function to `storage.rs` (which is out of
  scope/frozen for this milestone) or a `WeekData`/`WeekTargets` trait
  adapter type living in `week.rs` (also out of scope). This mirrors
  §2.4's own "cost scales with total week count, negligible at
  personal-use scale" framing — the same complexity class M6's own walk
  already accepts — and keeps every DB access going through
  `storage.rs`'s existing public API surface.

## Open questions / risks for the reviewer

- **A2 (fulfillment/target suffix shown only on the current-week form)**
  and **A5 (the EOD estimate not crediting the open stint's own live
  minutes, so it's a moving target)** are both Milestone 9-owned
  behaviors, inherited here verbatim and golden-tested against the
  literal SPEC.md §7.1 examples (T7, T11). Nothing in this milestone
  second-guesses either; flagging only because a reviewer comparing
  against SPEC.md prose alone (rather than the worked examples) might
  expect otherwise.
- **A4 (`(+ ongoing)`/live stint duration on a past date with a dangling
  `start`)**: implemented per the plan's literal reading — `has_open_stint`
  and a stint's displayed duration are computed the same way regardless
  of whether `target_date` is today. No test exercises the extreme case
  (a stale `start` from weeks/months ago producing a very large hour
  count) beyond what `stint.rs`'s own tests already cover for the
  formatter; `format_minutes` is Milestone 1's code and already handles
  3+ digit hours without panicking, so this milestone adds no new risk
  there but also adds no new safeguard.
- **`build_ledger`'s per-date scan is O(days between the earliest data
  date and the target week's Sunday), same complexity class as
  `week::week_series`.** For an MVP single-user tool this is accepted by
  §2.4's own framing, but it's worth a reviewer's eye if a future
  milestone wants to speed up `status`/`week` for accounts with years of
  history — a `notes_in_range` bulk read (mirroring
  `storage::punches_in_range`) would be the natural follow-up and was
  deliberately not added here since it would touch the frozen
  `storage.rs`.
- **Timezone handling in `status.rs` uses `chrono::Local` directly**
  (for converting stored punch UTC instants back to local wall-clock
  time for display), matching the established pattern in
  `src/commands.rs` rather than accepting a generic `Tz: TimeZone`
  parameter the way `storage.rs`'s write path does. This means
  `status.rs`'s own tests are tied to whatever timezone the test runner
  happens to be in for anything that depends on local-vs-UTC date
  boundaries (none of the tests written here cross midnight, so none are
  currently sensitive to it) — same latent risk M7 already carries, not
  a new one introduced here.
