# Task 5 plan: CLI surface (log level, glossary, argument order, `week target` echo)

> **Archived — historical planning record.** Not authoritative. Current spec: `docs/dev/SPEC.md`. Design/process history: `docs/dev/NOTES.md`.

Changeset plan: `docs/dev/plans/possible-issues-triage-plan.md` (T5 section, decisions F7, contracts 8 and 9). Spec: `docs/dev/specs/2026-10-03-possible-issues-triage.md` (§3.5, §4.1, §4.2, §4.3, §7).

## Read for this plan

- `docs/dev/plans/possible-issues-triage-plan.md` (whole)
- `docs/dev/specs/2026-10-03-possible-issues-triage.md` (whole)
- `docs/dev/plans/reports/possible-issues-triage-plan-review.md` (whole; F1 and F4 drive the clap mechanism)
- Sibling task plans: none exist yet in `docs/dev/plans/` for this changeset (`ls docs/dev/plans` shows only the changeset plan). T5 shares no file and no signature with any sibling, so nothing is imported.
- Source: `src/cli.rs`, `src/main.rs`, `src/week_target.rs` (all whole); `src/time.rs:109` (`format_minutes`), `src/date.rs:263,276,362` (`WeekId::current`, `to_key`, `Display`)
- Docs: `docs/dev/SPEC.md` §1.3 (lines 191-212), §2.4 (287-332), §3.7, §6.1-6.3, §7.1 (703-883), §7.2 (885-990), §7.3 (991-1077), §7.4
- Tests for style: `tests/e6_db_failure.rs`, `tests/status_cli.rs` (head)
- `clippy.toml` (cognitive-complexity-threshold 15), `.githooks/pre-commit` (fmt check, complexity, coverage ratchet), `AGENTS.md`
- clap: `Cargo.lock` pins `clap 4.6.6`, `clap_builder 4.6.6`, `clap_derive 4.6.4` (`Cargo.toml:32` asks `4.6.6` with default features plus `derive`, so `error-context` is on). I read `~/.cargo/registry/src/*/clap_builder-4.6.6/src/{error/mod.rs,builder/command.rs,output/help_template.rs}` and built a scratch crate (`/tmp/cd-scratch/t5/proto`, pinned `=4.6.6`) that ran every API call below. I also built the unmodified `mlm` into `/tmp/cd-scratch/t5/target` to capture today's `-h`/`--help` output (golden text below).

## Scope

Files this task owns (changeset plan, T5): `src/cli.rs`, `src/main.rs`, `src/week_target.rs`, new `tests/week_target_cli.rs`.

Files this task must not touch, and owner:

| File | Owner |
|---|---|
| `src/status.rs`, `src/week_view.rs`, `src/render.rs`, `src/stint.rs`, `src/week.rs`, `src/storage.rs`, `src/date.rs`, `src/lib.rs`, `tests/status_cli.rs`, `tests/e6_db_failure.rs`, `tests/ascii_sweep.rs`, `tests/week_read_errors.rs` | T1 |
| `src/anomalies.rs`, `tests/overlap_cli.rs` | T2 |
| `src/lookback.rs`, `src/commands.rs`, `tests/start_hint_cli.rs` | T3 |
| `src/carry_scan.rs`, `tests/carry_marker_cli.rs` | T4 |
| `docs/dev/SPEC.md`, `README.md`, `AGENTS.md` | T6 |
| `NOTES.md`, the spec, `coverage-baseline.json` | coordinator |

`src/lib.rs` already declares `pub mod cli` and `pub mod week_target` (the binary imports `mlm::cli`, `mlm::week_target`, `src/main.rs:3-4`), so T5 needs no `lib.rs` edit. No new module is created.

## Dependencies

| Dependency | State | Used for | Stub / swap |
|---|---|---|---|
| T1 `done for today` literal in `src/status.rs` | not landed | The glossary and its test mention `done for today` and require the absence of `target already met` in the help text only. They never read `status.rs` output. | None needed. T5's tests assert on `--help` text, which T5 owns. The coordinator checks glossary wording against the built binary at merge (changeset plan, ordering section). |
| `time::format_minutes(i64) -> String` (`src/time.rs:109`) | shipped on the branch | The echo line | Called as published. |
| `date::WeekId` (`Copy`, `Display` = `YYYY-WW`, `WeekId::current(NaiveDate)`) (`src/date.rs:263,362`) | shipped on the branch; T1 touches only comments in `src/date.rs` | The echo line | Called as published. |

No stub and no import swap. T5 is independent.

## Facts gathered (current state)

`src/cli.rs`
- Line 3: `use clap::{Args, Parser, Subcommand};`.
- Lines 5-14: `Cli` derives `Parser`, `#[command(name = "mlm", version, about = ...)]`, subcommand field, `verbose` with `#[arg(short, long, global = true)]` and doc `Enable debug-level logging.` (this doc text is in every `-h`, so it must not change).
- Lines 39-46: `Status { date }` variant, alias `d`. Lines 35-37: `Week(WeekArgs)`, alias `w`.
- Lines 172-182: `WeekArgs` with `args_conflicts_with_subcommands = true`, optional `WeekAction` subcommand and optional positional `WEEK_ID`.
- Lines 184-188: `WeekAction::Target(WeekTargetArgs)`.
- Lines 200-225: `WeekTargetArgs { args: Vec<String> }`, one positional `value_name = "ARGS"`, `num_args = 1..=2`, `required = true`, `allow_hyphen_values = true`; `split()` at 218.
- Lines 227-546: unit tests use `Cli::try_parse_from` directly (`parse` helper, 231-233). P3 (261-264) pins `MissingRequiredArgument` for `week target`; P4 (268-271) pins that three args error but not with `DisplayHelp`; P5 (275-278) pins `week target -5h`.

`src/main.rs`
- Line 2 `use clap::Parser;`. Line 15 `let cli = Cli::parse();` under the comment "clap handles --help/--version/missing-arg itself and exits(2) on error". Lines 19-24 pick `Debug` for `--verbose`, otherwise `Info`, then `env_logger::Builder::new().filter_level(level).init()` (so `RUST_LOG` is not read). Line 36 `exit_code(&dispatch(&cli, now))`; `exit_code` at 70-78 prints `error: {e}` and returns 1.
- Dispatch for `week target` is line 60: `week_target::run(&conn, now.date_naive(), t)`.

`src/week_target.rs`
- Line 6 `#![allow(dead_code)]`. Line 13 `use crate::time::parse_duration;`.
- Lines 55-68 `run(conn, today, args) -> anyhow::Result<()>`: split, parse week (or `WeekId::current(today)`), `parse_duration`, `set_week_target`, `Ok(())`. Silent today.
- Tests (70-277) call `run` and read storage via `get_week_target`. They stay valid because `run` keeps its signature.

Observed behavior of today's binary (scratch build):
- `mlm status -h` and `mlm status --help` print identical text; same for `week`. Golden text is in the test plan.
- `mlm week target` prints `Usage: mlm week target <ARGS>...` and `<ARGS>...` in the required-arguments list.
- `mlm week 2026-41 target 20h` prints `error: the subcommand 'target' cannot be used with '[WEEK_ID]'` plus a `Usage:` block, exit 2. `mlm week 7 foo` prints the same shape for `foo` (same `ArgumentConflict` kind).
- A fresh database prints `INFO  rusqlite_migration` on stderr.
- `mlm week target 3 -h` treats `-h` as the duration token (`allow_hyphen_values`), and `mlm week target -h 3` shows help. The change must keep both.

## Mechanism findings (clap 4.6.6, verified in scratch)

1. `#[command(override_usage = "...", help_template = "...", before_help = "...")]` on the `Args` struct of a `Subcommand` variant applies to that subcommand. A `hide = true` positional disappears from the generated Arguments section and `{all-args}` then prints only `Options:`.
2. Overriding usage changes the `Usage:` line in help and in every error (missing, too many). It does not change the argument list text.
3. The text `<ARGS>...` is still printed by (a) the `MissingRequiredArgument` list (`week target`) and (b) `TooManyValues` (`week target 1 2 3`: `unexpected value '3' for '<ARGS>...' found`). Review F1 caught (a); (b) is new here and the acceptance criterion ("any error printed for that subcommand") covers it.
4. `clap::Error::insert(ContextKind::InvalidArg, ContextValue)` and `Error::get(ContextKind)` exist (`error/mod.rs:193,202`, feature `error-context`, on by default). Those two kinds carry no pre-rendered message, so the printed text follows the replaced context, keeps clap's styling and keeps the overridden usage. This replaces the changeset plan's hand-printed E10 text with a smaller mechanism that covers (a) and (b) the same way. Confirmed output:
   ```
   error: the following required arguments were not provided:
     <DURATION>

   Usage: mlm week target [OPTIONS] [WEEK_ID] <DURATION>

   For more information, try '--help'.
   ```
5. Adding `after_long_help` to a subcommand changes its short help line to `Print help (see more with '--help')` and its long help to the next-line layout. Spec §4.2 and §6 accept that suffix on `status -h` and `week -h` and allow no replacement help flag, so the plan uses plain `after_long_help` and touches no help flag. `week target` gets no `after_long_help`, so its `-h`/`--help` line stays `Print help`.
6. `Error::use_stderr()` is false exactly for `DisplayHelp`/`DisplayVersion` kinds (`error/mod.rs:218`). Gating the interception on it keeps `--help`/`--version` on clap's exit-0 path.
7. `StyledStr` implements `Display` and strips styling, so tests can `err.render().to_string()`.

## Design

### cli.rs: new public surface

```rust
use std::ffi::OsString;

use clap::error::{ContextKind, ContextValue};
use clap::{Args, CommandFactory, FromArgMatches, Parser, Subcommand};

/// The `status`/`week` long-help glossary (SPEC §4.2).
pub const GLOSSARY: &str = "...";               // text below

/// The parse tree with the help surface applied.
pub fn command() -> clap::Command;

/// Why `parse` failed and what `main` should do about it.
#[derive(Debug)]
pub enum ParseFailure {
    /// Clap's own error: help, version, or a usage error (exit 2).
    Clap(clap::Error),
    /// `week WEEK_ID target DURATION`: the full `error: ...` line (exit 2).
    SwappedWeekTarget(String),
}

impl ParseFailure {
    pub fn exit_code(&self) -> i32;              // Swapped -> 2, Clap(e) -> e.exit_code()
    pub fn report(&self) -> i32;                 // print, then return exit_code()
}

/// `Cli` from raw arguments (argv[0] included).
pub fn parse(args: &[OsString]) -> Result<Cli, ParseFailure>;

/// Pure shape check over argument tokens (argv[0] included).
pub fn swapped_week_target(args: &[String]) -> Option<String>;
```

`Cli::try_parse_from` keeps working untouched for the existing unit tests. `parse` is the production entry. The unit-test helper `parse` at `src/cli.rs:231` has a different name resolution (it is inside `mod tests`, with its own `fn parse`), and `use super::*` brings in the new `parse`; the local test helper shadows it, since a local item wins over a glob import. Keep the helper's name. If the compiler reports ambiguity, rename the new function to `parse_args` and update `main.rs` only.

### `command()`

```rust
pub fn command() -> clap::Command {
    Cli::command()
        .mut_subcommand("status", with_glossary)
        .mut_subcommand("week", with_glossary)
}

fn with_glossary(cmd: clap::Command) -> clap::Command {
    cmd.after_long_help(GLOSSARY)
}
```

`week target` gets no glossary because `after_long_help` is set on `status` and `week` only. Its `-h`/`--help` and the hyphen-value behavior of `week target 3 -h` are clap's defaults, unchanged.

### `WeekTargetArgs` derive attributes

Replace the struct attributes and the `args` field docs (lines 190-212). The field, its type and `split()` do not change, so `src/week_target.rs` tests keep compiling.

```rust
#[derive(Args, Debug)]
#[command(
    override_usage = "mlm week target [OPTIONS] [WEEK_ID] <DURATION>",
    help_template = "{about-with-newline}\n{usage-heading} {usage}\n\n{before-help}\n\n{all-args}",
    before_help = "\
Arguments:
  [WEEK_ID]   Week to set: YYYY-WW (e.g. 2026-07) or a bare week number (e.g. 7). Defaults to the current week
  <DURATION>  Target to set: 20h, 33h30m or 45m (human format, never raw minutes)"
)]
pub struct WeekTargetArgs {
    /// Hidden from the generated help, which cannot show an optional positional before a
    /// required one. The hand-written Arguments section above stands in for it.
    #[arg(
        value_name = "ARGS",
        num_args = 1..=2,
        required = true,
        allow_hyphen_values = true,
        hide = true
    )]
    pub args: Vec<String>,
}
```

Keep the existing doc paragraph that explains the `num_args = 1..=2` trick and `allow_hyphen_values` (E9), adjusted: it should say the one variadic positional is hidden and why. The `value_name = "ARGS"` stays because `rename_args_context` keys on it.

Resulting `mlm week target --help` (verified shape in scratch; description line is the existing variant doc comment):

```
Set an absolute target override for a week

Usage: mlm week target [OPTIONS] [WEEK_ID] <DURATION>

Arguments:
  [WEEK_ID]   Week to set: YYYY-WW (e.g. 2026-07) or a bare week number (e.g. 7). Defaults to the current week
  <DURATION>  Target to set: 20h, 33h30m or 45m (human format, never raw minutes)

Options:
  -v, --verbose  Enable debug-level logging
  -h, --help     Print help
```

`{before-help}` writes its text followed by a blank line, so the template carries `\n\n` only to close the section. If the scratch spacing differs when written into the real file, adjust the template only (the acceptance check is on content, not blank lines). A trailing `\n` inside `before_help` doubles the gap; keep it out.

### Error rewrite for `<ARGS>`

```rust
const ARGS_NAME: &str = "<ARGS>";

pub(crate) fn rename_args_context(err: &mut clap::Error) {
    let renamed = match err.get(ContextKind::InvalidArg) {
        Some(ContextValue::Strings(names)) if names.iter().any(|n| n.starts_with(ARGS_NAME)) => {
            // Missing-argument list: only DURATION can be missing.
            Some(ContextValue::Strings(vec!["<DURATION>".to_string()]))
        }
        Some(ContextValue::String(name)) if name.starts_with(ARGS_NAME) => {
            // Too many values.
            Some(ContextValue::String("[WEEK_ID] <DURATION>".to_string()))
        }
        _ => None,
    };
    if let Some(value) = renamed {
        err.insert(ContextKind::InvalidArg, value);
    }
}
```

Compiled in scratch (`clap 4.6.6`). The `Strings` arm replaces the whole list with `<DURATION>`, which is correct for the only sentinel case: a `Strings` list holding `<ARGS>...` arises when the required positional is absent, and nothing else can be missing in that subcommand. A grep shows no other argument in `mlm` is named `ARGS`, so nothing else matches.

### `swapped_week_target` (pure)

```rust
pub fn swapped_week_target(args: &[String]) -> Option<String> {
    let tokens: Vec<&str> = args
        .iter()
        .skip(1)
        .map(String::as_str)
        .filter(|t| !matches!(*t, "-v" | "--verbose"))
        .collect();
    match tokens.as_slice() {
        ["week" | "w", week, "target", duration]
            if is_week_token(week) && is_duration_token(duration) =>
        {
            Some(format!(
                "error: put WEEK_ID after target: mlm week target {week} {duration}"
            ))
        }
        _ => None,
    }
}

/// A week id is a plain token: not a flag, and not the subcommand name.
fn is_week_token(token: &str) -> bool {
    !token.starts_with('-') && token != "target"
}

/// A duration may be negative (`-5h` reaches the duration parser, E9) but never `--help`, `-h`, `-V`.
fn is_duration_token(token: &str) -> bool {
    match token.strip_prefix('-') {
        None => true,
        Some(rest) => rest.starts_with(|c: char| c.is_ascii_digit()),
    }
}
```

- Token shape decides; the clap error kind is never inspected (review F4: `week 7 foo` has the same kind).
- The message text is spec §4.3 byte for byte and echoes the tokens as typed (`2026-7` stays `2026-7`). It always spells `mlm week target`, even for the `w` alias (spec §6).
- Complexity is under 5 per function.

### `parse` and `ParseFailure`

```rust
pub fn parse(args: &[OsString]) -> Result<Cli, ParseFailure> {
    command()
        .try_get_matches_from(args)
        .and_then(|matches| Cli::from_arg_matches(&matches))
        .map_err(|err| classify(args, err))
}

fn classify(args: &[OsString], mut err: clap::Error) -> ParseFailure {
    if !err.use_stderr() {
        return ParseFailure::Clap(err);       // --help / --version: clap's own path
    }
    let text: Option<Vec<String>> = args
        .iter()
        .map(|a| a.to_str().map(str::to_owned))
        .collect();
    if let Some(message) = text.and_then(|a| swapped_week_target(&a)) {
        return ParseFailure::SwappedWeekTarget(message);
    }
    rename_args_context(&mut err);
    ParseFailure::Clap(err)
}

impl ParseFailure {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Clap(err) => err.exit_code(),
            Self::SwappedWeekTarget(_) => 2,
        }
    }

    pub fn report(&self) -> i32 {
        match self {
            Self::Clap(err) => {
                let _ = err.print();
            }
            Self::SwappedWeekTarget(message) => eprintln!("{message}"),
        }
        self.exit_code()
    }
}
```

`&[OsString]` satisfies `try_get_matches_from`'s `Into<OsString> + Clone` bound through `From<&T> for OsString`. Non-UTF-8 arguments skip interception and fall to clap's own error. A successful parse never reaches `classify`, so `week target target 20h` (parses OK as week `target`) is unaffected; the `is_week_token` guard keeps the pure function consistent with that.

### `GLOSSARY` text

Plain ASCII, lines at most 78 columns, backticks for literals as the existing help text does. Ends after the last sentence (clap adds the final newline).

```
Glossary:
  worked
      Time in closed stints this week. In the current week it also counts
      the stint that is running now.
  carry-in
      The balance carried in from earlier weeks. A shortfall is negative
      and a surplus is positive.
  fulfillment
      Worked plus carry-in. A week's target is measured against it.
  left to / over
      The week line's figure. `X left to F` means X more reaches F, and
      `X over F` means fulfillment is X past F. Monday to Thursday F is
      the slice of the target due by end of today (one fifth per weekday
      so far), shown as `by end of <weekday>`. Friday to Sunday F is the
      full target, shown as `<target> target`.
  est. EOD
      With a stint open today, the clock time at which that stint closes
      today's gap. It reads `done for today` instead when what is due by
      end of today is already covered.
  (incl. ongoing), (+ ongoing)
      `(incl. ongoing)` means today's open stint is counted in the total.
      `(+ ongoing)` marks an earlier date's open stint, which counts for
      nothing until it is stopped (`(+ unclosed)` once it is two or more
      days old).
  [!], [i]
      `[!]` flags an anomaly, such as an unclosed start, an orphaned end
      or overlapping stints. In `mlm week` it marks the date's row. `[i]`
      is a hint, not an anomaly, such as a start still open from
      yesterday. It never marks a row.
```

Check against the spec (each claim, with its source):

| Glossary claim | Source |
|---|---|
| worked = closed stints; current week adds the running stint | §2.4 lines 301-306 ("`worked_minutes` is the sum of completed stints ... for the current week only, the running stint's live minutes join") |
| carry-in = balance from earlier weeks, shortfall negative, surplus positive | §1.3 lines 208-212 ("negative if the prior week fell short, positive if it exceeded target"), §2.4 lines 291-299 (walks every earlier week). Review F2 correction honored: surplus does carry. |
| fulfillment = worked + carry-in | §1.3 line 208-209, §2.4 |
| `left to` / `over`, gap | §7.1 lines 767-781 (`gap = required - fulfillment`, `gap == 0` reads `left to`), headline table lines 813-816 |
| Mon-Thu slice, `by end of <weekday>`; Fri-Sun full `<target> target` | §2.4 lines 317-325 (daily target = target / 5, Mon-Thu `daily x weekday number`, Fri-Sun the full target), table in §7.1 |
| est. EOD = clock time the open stint closes the gap, only with a stint open today | §7.1 lines 782-792 |
| `done for today` replaces it when gap <= 0 | spec §4.4 and §7.1 line 793 (`target already met` today; T1 renames) |
| `(incl. ongoing)` today; `(+ ongoing)` earlier date, no credit; `(+ unclosed)` at two or more days | §7.1 lines 747-756 |
| `[!]` anomaly, week row marker; `[i]` hint, never marks a row | §7.3 lines 993-1000 and 1054-1065, §7.2 lines 976-989, spec §3.1 (the `[i]` hint is not an anomaly and does not mark `mlm week` rows) and §3.2.1 (overlap) |

Items deliberately omitted: `Total behind`/`Total ahead` (not on the §4.2 list; changeset plan T5 puts anything outside that list out of scope).

### `main.rs`

```rust
use std::ffi::OsString;
use chrono::{DateTime, Local, Timelike};
use mlm::cli::{self, Cli, Command, DeleteTarget, WeekAction};
use mlm::{commands, db, status, week_target, week_view};

fn run() -> i32 {
    // Usage errors, help and version go through `ParseFailure::report` (exit 2, or 0 for help/version).
    let args: Vec<OsString> = std::env::args_os().collect();
    let cli = match cli::parse(&args) {
        Ok(cli) => cli,
        Err(failure) => return failure.report(),
    };

    // --verbose raises the log level to Debug; the default is Warn (SPEC §4.1).
    let level = if cli.verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Warn
    };
    env_logger::Builder::new().filter_level(level).init();
    ...unchanged...
}
```

Remove the now-unused `use clap::Parser;` (line 2) and keep `Cli` imported only if `dispatch`'s signature still names it (`fn dispatch(cli: &Cli, ...)`, line 39: yes). `-D warnings` fails on an unused import, so confirm with `cargo clippy`.

### `week_target.rs`

```rust
use crate::time::{format_minutes, parse_duration};

/// The success line of `week target` (SPEC §3.5), e.g. `Week 2026-41 target: 20h 00m`.
pub fn echo_line(week: &WeekId, minutes: i64) -> String {
    format!("Week {week} target: {}", format_minutes(minutes))
}

/// Parse, write, and return the echo line. Nothing prints here.
pub fn apply(conn: &Connection, today: NaiveDate, args: &WeekTargetArgs) -> anyhow::Result<String> {
    // body of today's `run` through `set_week_target`
    Ok(echo_line(&week, minutes))
}

/// `mlm week target [WEEK_ID] DURATION` (§3.7): writes the override, then prints one line (§3.5).
pub fn run(conn: &Connection, today: NaiveDate, args: &WeekTargetArgs) -> anyhow::Result<()> {
    println!("{}", apply(conn, today, args)?);
    Ok(())
}
```

- `week` is the normalized `WeekId`, so `2026-7` echoes `2026-07` (the id written to; spec §3.5 "the week id is the one written to"), unlike the swapped-order message, which echoes typed tokens.
- A failure at either parse returns before the print, so failed runs leave stdout empty. `set_week_target` failing also returns before the print.
- `run` keeps its name and signature, so `src/main.rs:60` and R1-R10 stand unchanged.

## Steps (TDD order)

1. **Tests first.** Write `tests/week_target_cli.rs` and the `src/cli.rs` / `src/week_target.rs` unit tests below. Confirm each fails for the right reason (no echo, `<ARGS>` present, `Info` default, no glossary, clap message for the swapped shape).
2. `src/main.rs`: default level `Warn` (§4.1). Tests for log level pass.
3. `src/week_target.rs`: `echo_line`, `apply`, `run` (§3.5). Echo tests pass.
4. `src/cli.rs`: `WeekTargetArgs` attributes (help and usage), then `command()` with the glossary, then `rename_args_context`, `swapped_week_target`, `ParseFailure`, `parse`.
5. `src/main.rs`: call `cli::parse`, drop the `Parser` import.
6. `cargo fmt`, `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings`, the two smoke runs from AGENTS.md. Compare `-h` output of `status` and `week` against the goldens (golden tests do this).

Functions stay under complexity 15 by construction: each helper above has one `match` or one `if`. `classify` has two `if`s and two closures and sits near 5.

## Test plan

Integration tests in `tests/week_target_cli.rs` follow `tests/e6_db_failure.rs`: `Command::new(env!("CARGO_BIN_EXE_mlm"))` with `MLM_DB_PATH` set to a fresh path in a `tempfile::TempDir`, `TZ` pinned to `UTC`. Helpers: `run(db, args) -> Output`, `stdout(&Output)`, `stderr(&Output)`.

### §4.1 default log level (spec §4.1, §7 bullet list implies "each item gets tests")

- `start` (no args) on a fresh database: exit 0, stderr is exactly empty, stdout empty.
- `--verbose start` on a second fresh database: stderr contains `Database migrated` (the `rusqlite_migration` INFO line) and at least one `DEBUG` line (`db path:` from `main.rs:42`).
- `-v` short flag behaves like `--verbose`.
- Unit test in `src/main.rs` is not needed (the level choice is two lines of inline `if`); the integration pair covers both branches.

### §3.5 `week target` echo (spec §3.5, §6 last bullets)

- `week target 20h`: stdout equals `format!("Week {id} target: 20h 00m\n")` with `id` the current ISO week from `mlm::date::WeekId::current(chrono::Utc::now().date_naive())` (TZ pinned to UTC; a test crossing a week boundary at the exact minute is accepted flakiness, noted in risks). Stderr empty, exit 0.
- `week target 2026-41 20h`: stdout exactly `Week 2026-41 target: 20h 00m\n`.
- `week target 2026-7 33h30m`: `Week 2026-07 target: 33h 30m\n` (normalized id).
- `week target 2026-41 0h` and `0m`: `Week 2026-41 target: 00h 00m\n`.
- `week target 2026-41 90m`: `Week 2026-41 target: 01h 30m\n` (standard formatter, spec §6 last bullet).
- `w target 2026-41 20h` (alias) behaves like `week target`.
- `--verbose week target 2026-41 20h`: stdout still exactly the one line (log output goes to stderr).
- Failures print nothing on stdout: bad duration (`week target 2026-41 abc`), bad week (`week target 2027-53 20h`, `week target abcd 20h`), negative (`week target -5h`): each exit 1, stdout empty, stderr starts with `error:`.
- `start`, `stop`, `note hello` on a fresh database: stdout empty (spec: they "stay silent").
- Unit tests in `src/week_target.rs`: `echo_line(&wk(2026,41), 1200) == "Week 2026-41 target: 20h 00m"`; `apply` returns that line for `["2026-41","20h"]` and writes the row (`get_week_target == Some(1200)`); `apply` for `["33h30m"]` with a fixed `today` returns the current week's id; failed `apply` (bad week, bad duration) returns `Err` and row count 0 (reuses R4/R5 inputs).

### §4.2 glossary (spec §4.2)

Binary tests on `status --help` and `week --help` (also `d --help`, `w --help`, `help status`, `help week`):

- Each of these substrings is present in both outputs: `worked`, `carry-in`, `fulfillment`, `left to`, `over`, `by end of <weekday>`, `<target> target`, `est. EOD`, `done for today`, `(incl. ongoing)`, `(+ ongoing)`, `[!]`, `[i]`.
- `shortfall is negative` and `surplus is positive` present (review F2 guard on carry-in).
- `target already met` absent from both.
- Output ends with the glossary: `stdout.trim_end().ends_with("It never marks a row.")`.
- The glossary text is non-ASCII free: `assert!(GLOSSARY.is_ascii())` (the unit test in `src/cli.rs`; AGENTS keeps rendered output ASCII, `tests/ascii_sweep.rs` is T1's file so the check lives here).
- Short help keeps today's text except the help line, which gains clap's `(see more with '--help')` suffix (spec §4.2, §6). Golden strings are the pre-change output with that one line changed:

```
$ mlm status -h
Show a date's stints, notes and totals (defaults to today)

Usage: mlm status [OPTIONS] [DATE]

Arguments:
  [DATE]  Date to show: YYYY-MM-DD, or `-N` for N days before today (e.g. `-1` = yesterday). Defaults to today

Options:
  -v, --verbose  Enable debug-level logging
  -h, --help     Print help (see more with '--help')
```

```
$ mlm week -h
Show a week's totals, or set its target

Usage: mlm week [OPTIONS] [WEEK_ID]
       mlm week <COMMAND>

Commands:
  target  Set an absolute target override for a week
  help    Print this message or the help of the given subcommand(s)

Arguments:
  [WEEK_ID]  Week to show: YYYY-WW (e.g. 2026-07) or a bare week number (e.g. 7). Defaults to the current week

Options:
  -v, --verbose  Enable debug-level logging
  -h, --help     Print help (see more with '--help')
```

Both goldens end with a trailing newline. Assert equality on the full `stdout`. Also assert `-h` outputs do not contain `Glossary`, and that `week target -h` still prints `Print help` with no `see more with`.

- Unit test in `src/cli.rs`: `command().debug_assert()` passes (catches duplicate or conflicting help arguments).

### §4.3 help and usage for `week target` (spec §4.3 first bullet)

- `week target --help` and `week target -h`: exit 0; stdout contains `Usage: mlm week target [OPTIONS] [WEEK_ID] <DURATION>`, an `Arguments:` line followed by lines naming `[WEEK_ID]` and `<DURATION>`, `Options:`, and does not contain `<ARGS>`. `-h` equals `--help` byte for byte.
- `week target` (E10): exit 2, stderr contains `<DURATION>`, `Usage: mlm week target [OPTIONS] [WEEK_ID] <DURATION>`, not `<ARGS>`, stdout empty.
- `week target 2026-41 20h extra` (three tokens): exit 2, stderr contains `unexpected value 'extra'`, not `<ARGS>`.
- `week target 1 2 3`: exit 2, not `<ARGS>`.
- Parse unit tests in `src/cli.rs`, all through the new `parse` (as opposed to the existing `Cli::try_parse_from` ones): `week target` gives `ParseFailure::Clap(e)` with `e.kind() == MissingRequiredArgument` and `e.render().to_string()` free of `<ARGS>`; same for three tokens with `TooManyValues`.
- Existing tests P1-P6 and the hyphen-duration case stay as they are and keep passing (they use `Cli::try_parse_from`, untouched by the help surface).
- `week target 3 -h` still reaches the duration parser (exit 1, `error: invalid duration "-h"`), matching today's behavior. `week target -h 3` shows help.

### §4.3 argument-order interception (spec §4.3 second bullet, §7 "§4.3" bullet)

Binary tests, each checking exit code, exact stderr and empty stdout:

- `week 2026-41 target 20h`: exit 2, stderr exactly `error: put WEEK_ID after target: mlm week target 2026-41 20h\n` (no `Usage:` text anywhere).
- `w 2026-41 target 20h`: same exact line (alias not echoed).
- `--verbose week 7 target 33h30m`, `week --verbose 7 target 33h30m`, `week 7 --verbose target 33h30m`, `week 7 target --verbose 33h30m`, `week 7 target 33h30m -v`: exit 2, stderr exactly `error: put WEEK_ID after target: mlm week target 7 33h30m\n`.
- `week 2026-7 target 20h`: echoes `2026-7` as typed.
- `week 2026-41 target -5h`: matches (echoes `-5h`), exit 2.
- Keep clap's own text and exit 2 (stderr contains `Usage:` and does not contain `put WEEK_ID`):
  - `week 7 foo`
  - `week 2026-41 target` (no duration)
  - `week 2026-41 target 20h extra`
  - `week 7 target --help` (a `--` duration token is rejected by the shape check)
- Unaffected (exit 0, as today): `week target 2026-41 20h` (correct order), `week 2026-41` (view), `week` (view).
- `--help` and `--version` still exit 0 with clap's text (`mlm --help`, `mlm week --help`, `mlm --version`), proving `use_stderr` gating.
- Pure unit tests for `swapped_week_target` (no database, `src/cli.rs`):
  - `Some` for `["mlm","week","2026-41","target","20h"]`, with `w`, with `-v` / `--verbose` at each position, `2026-7`, `-5h` duration.
  - `None` for `week 7 foo`, `week 2026-41 target`, `week 2026-41 target 20h extra`, `week target 2026-41 20h`, `week 2026-41`, `week target target 20h`, `status 7 target 20h`, `week -x target 20h`, `week 7 target --help`, `week 7 target -h`, `week 7 target -V`, an empty slice and a one-element slice.
  - The message is exactly the spec §4.3 line for the sample tokens.
- Unit tests for `ParseFailure`: `exit_code()` is 2 for `SwappedWeekTarget`, 0 for a `DisplayHelp` clap error (`parse(["mlm","--help"])`), 2 for a usage error.
- Unit test: `parse(["mlm","week","2026-41","target","20h"])` returns `Err(SwappedWeekTarget("error: put WEEK_ID after target: mlm week target 2026-41 20h"))`; `parse(["mlm","week","target","20h"])` returns `Ok`.

### Cross-cutting

- Existing `src/cli.rs` and `src/week_target.rs` tests pass unmodified.
- The full AGENTS.md verification set plus the two smoke runs, the complexity limit (every new function at or under 15) and the coverage ratchet. Every branch added has a test above: both `rename_args_context` arms and its no-op arm (E10, too-many, an unrelated error such as `week 7 foo`), `classify`'s three exits, `report`'s two arms (the `SwappedWeekTarget` arm is hit by the binary tests; to count toward coverage the `report`/`exit_code` code needs a unit test or an instrumented binary run, so add unit tests of `exit_code` and let the binary tests cover `report`, which `cargo llvm-cov` measures when it runs integration tests).

## Risks, ambiguities, disagreements

1. **E10 mechanism (F7).** The changeset plan F7 now adopts this plan's mechanism, `Error::insert` on the error context in the same fallible-parse step. It keeps clap's rendering and styling and also fixes `TooManyValues`, which still prints `<ARGS>...`.
2. **Glossary and the `-h` suffix.** In clap 4.6.6, setting `after_long_help` changes the `-h` help line on `status` and `week` to `Print help (see more with '--help')`. Spec §4.2 and §6 accept that and allow no replacement help flag, so the plan sets `after_long_help` only. If a later clap bump changes the suffix wording, the golden `-h` tests fail loudly.
3. **`Cli::try_parse_from` and `parse` are two entry points with slightly different help text.** Existing unit tests use the first, the binary the second. `-h`/`--help` differences are covered by binary tests only. A unit test cannot compare help text through `Cli::try_parse_from` for the glossary. This is acceptable and noted so nobody wires `Cli::parse` back in `main.rs` by accident.
4. **`rename_args_context` keys on the string `<ARGS>`.** If a clap upgrade formats positional names differently (for example drops the `...`), the `starts_with` check still matches `<ARGS>`. If clap changes the context kinds for these errors, the unit tests for both kinds fail.
5. **Alias echo.** The swapped-order message always spells `mlm week target` even for `w` (spec §6); the success echo (§3.5) normalizes the week id (`2026-7` -> `2026-07`), while the swapped message echoes typed tokens (spec §4.3). Both are deliberate; the tests pin both.
6. **Midnight/week-boundary flake** in the "current week" echo test (`week target 20h` compares against a week id computed in the test). The window is the instant the test crosses a week boundary. Accepted; the other echo tests use fixed ids.
7. **Glossary scope.** The text covers exactly the §4.2 list. It does not define `Total behind`/`Total ahead`, `(+ unclosed)` is mentioned only inside the `(+ ongoing)` entry because the spec text ties it to that suffix. A reviewer may want `Total behind`/`Total ahead` added; the changeset plan keeps it out of scope.
8. **Glossary names `overlapping stints` and the `[i]` hint**, which only exist once T2 and T3 merge. The text is correct at the integration merge. Before then the help text is slightly ahead of the behavior on the T5 branch, which the changeset plan accepts ("coordinator checks T5's glossary against the built binary at merge").
9. **Glossary wording of "one fifth per weekday so far"** compresses the daily-target rule (§2.4 floors the daily target to the minute and, for Fri-Sun, uses the exact target). It is correct for the Mon-Thu description it sits in. If a reviewer wants the floor mentioned, add it here, not in the spec.
10. **Ambiguity: the `-h` golden strings are copied from this branch's current binary**, built at the state of commit `cccfb8f`. If T1 or T6 change the `Cli`/`about` strings (they own neither `src/cli.rs` nor `--help`), the goldens still hold; nothing outside T5's files affects these two outputs.
11. **`ParseFailure::report` ignores a failed `print()`** (closed stdout/stderr). The old `Cli::parse()` path called `exit`, which also ignores it.
