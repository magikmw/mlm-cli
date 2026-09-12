# Milestone 8 — `week target` command — completion report

## What was implemented

- `src/cli.rs`: added `Command::Week(WeekArgs)`, `WeekArgs`, `WeekAction`,
  `WeekTargetArgs`, and `WeekTargetArgs::split()`, exactly per the plan's
  §2.2/§2.3 clap shape (variadic `args: Vec<String>` positional,
  `num_args = 1..=2`, `required = true`, `allow_hyphen_values = true`,
  `args_conflicts_with_subcommands = true`). Existing `Start`/`Stop`/`Log`
  variants untouched except both `Cli` and `Command` gained `#[derive(Debug)]`
  (required so `Cli::try_parse_from(...).unwrap_err()` compiles in tests —
  a mechanical, low-risk addition).
- `src/week_target.rs` (new module): `set_week_target`, `get_week_target`
  (the upsert/read wrappers), and `run(conn, today, args) -> anyhow::Result<()>`
  — the full command body: split → resolve WEEK_ID (validated first) →
  parse DURATION → defensive non-negative assert → upsert. No stdout output
  on success, per SPEC.md §7.4 ("`week target` prints nothing on success");
  this **overrides** the plan's §3.3 suggestion of a confirmation line —
  the plan flagged that suggestion as its own open ambiguity (A4) and
  SPEC.md, read directly, resolves it explicitly.
- `src/main.rs`: added `mod week_target;` and a minimal
  `Command::Week(_args) => todo!(...)` arm, required only to keep the
  `match` exhaustive now that `Command::Week` exists. No real dispatch
  wired — that remains Milestone 7/11's job per the task's explicit scope
  boundary.

## Deviations from the plan (and why)

1. **`set_week_target`/`get_week_target` live in the new `src/week_target.rs`
   module, not in `src/db.rs`.** The plan's §4.2 puts them in `db.rs`, but
   the task's explicit scope list forbids touching `db.rs`. Implemented as
   plain `rusqlite` calls against `&Connection` in the new module instead —
   functionally identical, same SQL, same upsert semantics.
2. **No `tests/week_target.rs` process-level CLI integration tests
   (plan §5.4, C1–C6).** These require `mlm week target ...` to actually
   run end-to-end, which needs real dispatch wired in `main.rs` — explicitly
   out of scope for this milestone per the task instructions ("do not touch
   src/main.rs's command dispatch... a later milestone will call it"). The
   task's own test-case list for this milestone also only names the unit-level
   cases (override+read-back, default week, zero/negative duration, malformed
   week id, missing duration, replace-on-override), all of which are covered
   at the `run()` level. C1–C6 are the natural next step once Milestone 7/11
   wires the dispatch — this is called out below as an open item for the
   reviewer, not silently dropped.
3. **Upstream API adaptation** (per the task's guidance to use real
   signatures, not the plan's guesses):
   - `date.rs` exposes `parse_week_id(s: &str, today: NaiveDate) -> Result<WeekId, DateWeekError>`
     (not `(s, current_year: i32)`), and `WeekId::current(today: NaiveDate) -> WeekId`
     for the omitted-WEEK_ID case (not a free `week_of` function). Used both directly.
   - The error type is `date::DateWeekError` (not a `WeekIdParseError`), and
     `time::DurationParseError` (matches the plan's naming). Both are
     `std::error::Error`, so `?` inside `run()` auto-converts via `anyhow`'s
     blanket `From` impl exactly as planned.
   - `WeekId::to_key()` (== `Display`) is the normalized `YYYY-WW` storage
     key, used for both the upsert parameter and the read-back query.
   - `db::apply_migrations(&mut Connection)` (from the real Milestone 3)
     is used directly in the unit tests' in-memory-connection fixture.
4. **R2's plan-suggested boundary date was wrong** and was corrected using
   `date.rs`'s own test fixtures. The plan proposed `2026-12-28` as an
   ISO-year-vs-Gregorian-year boundary example, but `date.rs`'s own tests
   (`from_date_maps_dates_to_their_iso_week`, T76) establish that
   `2026-12-28` is ISO week `2026-53`, not `2027-01`. The correct boundary
   case (confirmed by that same test table, T73) is `2025-12-29` → ISO week
   `2026-01`, which is what the test now uses.

## Test count and pass/fail state

- `cargo test`: **150 passed, 0 failed** (all pre-existing Milestone 1/2/3/5/6
  tests plus this milestone's new tests: 6 clap-parsing tests (P1–P6) in
  `src/cli.rs`, 6 storage-wrapper tests (U1–U6) and 10 command-body tests
  (R1–R10) in `src/week_target.rs`).
- `cargo build`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean, zero warnings.
- `cargo fmt`: applied.

## Final shape of `WeekArgs`/`WeekAction`/`WeekTargetArgs` (for Milestone 11 to reuse verbatim)

```rust
#[derive(Args, Debug)]
#[command(args_conflicts_with_subcommands = true)]
pub struct WeekArgs {
    #[command(subcommand)]
    pub action: Option<WeekAction>,

    #[arg(value_name = "WEEK_ID")]
    pub week_id: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum WeekAction {
    Target(WeekTargetArgs),
}

#[derive(Args, Debug)]
pub struct WeekTargetArgs {
    #[arg(
        value_name = "ARGS",
        num_args = 1..=2,
        required = true,
        allow_hyphen_values = true
    )]
    pub args: Vec<String>,
}

impl WeekTargetArgs {
    pub fn split(&self) -> (Option<&str>, &str) { ... }
}
```

`Command::Week(WeekArgs)` is the enum variant. Milestone 11 fills in the
`action: None` arm (render the week); it must not declare a second,
differently-shaped struct for this.

## Open questions / risks for the reviewer

- **`src/cli.rs` merge-coordination risk (plan's R2), still live.**
  Milestone 7 has not landed yet in this worktree and will also rewrite
  `Command`/add its own variants; Milestone 10 and 11 will touch `WeekArgs`'
  `action: None` arm next. The `Week` variant and its types were added as a
  self-contained block, but whoever lands Milestone 7 next should expect a
  textual merge conflict in `src/cli.rs` (not a semantic one — the shapes
  don't overlap) and in `src/main.rs`'s `match` (the `todo!()` stub arm for
  `Command::Week` will need to become real dispatch calling
  `week_target::run` once Milestone 7 wires it up).
- **`main.rs`'s `Command::Week(_args) => todo!(...)` is a placeholder,
  not real dispatch.** It exists solely so the exhaustive `match` compiles.
  If any other milestone's binary path exercises `mlm week ...` before
  Milestone 7/11 replaces this arm, it will panic. This is intentional and
  matches the task's explicit scope boundary, but flagging it so it isn't
  mistaken for an oversight.
- **No end-to-end CLI (process-spawn) tests exist yet for this command**
  (see deviation #2 above). The plan's §5.4 C1–C6 test list is a
  ready-made checklist for whoever wires `main.rs` dispatch next.
- **`--verbose` global flag** (PLAN.md contract 12): confirmed absent from
  `src/cli.rs` before this change and still absent — correctly left to
  Milestone 7, not added here.
- **Defensive negative-duration guard**: `run()` has a `debug_assert!(minutes >= 0)`
  after `parse_duration` per the plan's §3.3 point 4, but does not have a
  release-mode fallback `bail!` beyond that assert, since `parse_duration`
  already structurally cannot return a negative value in its current
  implementation (confirmed by reading `time.rs` directly) — the schema
  `CHECK (target_minutes >= 0)` remains the final backstop either way.
