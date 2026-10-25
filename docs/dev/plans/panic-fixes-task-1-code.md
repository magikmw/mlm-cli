# Task plan: panic-fixes Task 1, code and tests

> **Archived — historical planning record.** Not authoritative. Current spec: `docs/dev/SPEC.md`. Design/process history: `docs/dev/NOTES.md`.

Changeset plan: `docs/dev/plans/panic-fixes-plan.md` (Task 1). Spec: `docs/dev/specs/2026-10-25-panic-fixes.md` (locked). Branch `panic-fixes`, tip `9c8108b`. Plan only. No source file was edited to write it.

## Read for this plan

- `docs/dev/plans/panic-fixes-plan.md`, in full
- `docs/dev/specs/2026-10-25-panic-fixes.md`, in full
- `docs/dev/plans/reports/panic-fixes-plan-review.md`, in full
- `docs/dev/plans/reports/unwrap-audit-review.md`, in full (F1-F4 apply)
- `docs/dev/NOTES.md` entries 138-142
- Sibling plan `docs/dev/plans/panic-fixes-task-2-docs.md`: not on disk at `9c8108b`. Task 2 is docs only and consumes the changeset plan's contracts, so no signature needs syncing.
- Source: `src/main.rs`, `src/lib.rs`, `src/cli.rs`, `src/week_target.rs`, `src/commands.rs` (all 2301 lines), `src/export.rs` (lines 1-210), `src/status.rs` (lines 1-90, 205-225, 355-395, 480-625), `src/week_view.rs` (lines 300-345, 1195-1290, 2010-2060), `src/time.rs` (lines 1-180), `src/storage.rs` (lines 18-50, 195-232), `src/db.rs` (lines 40-80, 112-140), `src/render.rs` (lines 1-80)
- Tests: `tests/week_target_cli.rs` (all), `tests/e6_db_failure.rs` (all), `tests/status_cli.rs` (lines 1-60, 248-258), `tests/export_notes_cli.rs` (lines 1-70)
- Config: `Cargo.toml`, `clippy.toml`, `.githooks/pre-commit`, `coverage-baseline.json`, `.github/workflows/ci.yml`
- chrono 0.4.45 `offset/local/tz_info/rule.rs:448-545` (zero-based `n` rule day is `Julian0WithLeap`), `offset/local/unix.rs:92,118` (Unix `Local` reads `TZ`)

## Scope

**Owns**: `src/main.rs`, `src/lib.rs`, `src/cli.rs`, `src/week_target.rs`, `src/commands.rs`, `src/status.rs`, `src/week_view.rs`, `src/export.rs`, `tests/week_target_cli.rs`, new `tests/panic_fixes_cli.rs`, `coverage-baseline.json` (hook-ratcheted).

`src/lib.rs` joins the owned set under the changeset plan's clause ("If ... the write path turns out to need an edit to ... `src/lib.rs`, that file also belongs to this task"). It gets the shared write type and helper.

**Reads, never edits**: `src/time.rs`, `src/storage.rs`, `src/db.rs`, `src/render.rs`.

**Must not touch**:

| File | Owner |
|---|---|
| `docs/dev/SPEC.md` | Task 2 |
| `README.md` | Task 2 |
| `AGENTS.md` | Task 2 |
| `docs/dev/NOTES.md`, spec archive banner | coordinator |
| any other `.md` | coordinator |

No new top-level module. No new dependency, dev-dependency included. `Cargo.toml` is untouched.

## Dependencies

| Dependency | State | Use |
|---|---|---|
| Task 2 (docs) | parallel, no code | none. Task 2 quotes contracts 2, 3 and 5. This task must produce those strings byte for byte. |

No stub or integration swap is needed.

## 1. Facts gathered

### 1.1 `now` capture

- `src/main.rs:37-41` builds `now` as `Local::now().with_second(0).unwrap().with_nanosecond(0).unwrap()`. In a repeated local hour, `with_second` rebuilds the local wall time, gets `Ambiguous`, returns `None`, and the unwrap panics (audit F1).
- `src/main.rs:3` imports `chrono::{DateTime, Local, Timelike}`.
- The untyped punch path passes `now.time()` (`src/commands.rs:384`). Storage resolves `Ambiguous(earliest, _)` to `earliest` (`src/storage.rs:217-222`). No change there (spec §3).

### 1.2 Exit path and stderr sites

- `src/main.rs:80-88` `exit_code`: `Ok` gives 0, `Err` gives `eprintln!("error: {e}")` and 1. Tests at `src/main.rs:94-103`.
- `src/main.rs:43` calls `exit_code(&dispatch(&cli, now))`. `src/main.rs:8` calls `std::process::exit(run())`.
- `src/lib.rs:27-32` has a second `exit_code` (0/1, no printing). Only unit tests use it (`src/commands.rs:685,689,693,768,1970`, `src/week_view.rs:1284`).
- `src/cli.rs:367` prints the swapped-target reminder with `eprintln!`. `src/cli.rs:365` already drops clap's `err.print()` result.
- `src/commands.rs:94,112` already write warnings with `let _ = writeln!`.

### 1.3 Stdout sites (ten)

| Site | Code | Enclosing fn |
|---|---|---|
| `src/week_target.rs:79` | `println!("{}", apply(..)?)` | `week_target::run` |
| `src/status.rs:527` | `print!("{}", render(&view))` | `status::run` |
| `src/week_view.rs:329` | `print!("{}", render_week(&view))` | `week_view::run` |
| `src/export.rs:52` | `print!("{}", render_lines(&notes))` | `export::run_notes` |
| `src/commands.rs:198` | `println!` per punch row | `print_punch_list` |
| `src/commands.rs:211` | `println!` per note row | `print_note_list` |
| `src/commands.rs:296` | `println!("nothing to delete for ...")` | `delete_note` |
| `src/commands.rs:305` | `println!("deleted. to recreate: ...")` | `delete_note` |
| `src/commands.rs:328` | `println!("nothing to delete for ...")` | `delete_punch` |
| `src/commands.rs:337` | `println!("deleted. to recreate: ...")` | `delete_punch` |

Dispatch calls them at `src/main.rs:60,61,67,68,70,72`.

### 1.4 Cap and help

- `src/week_target.rs:61-74` `apply`: WEEK_ID parse, then `parse_duration`, then `debug_assert!(minutes >= 0)`, then `set_week_target`, then `echo_line`.
- `src/time.rs:60-104` `parse_duration`. `99999999999h` parses to 5,999,999,999,940. `9999999999999999999h` fails `digits()` and returns `InvalidFormat`. `167h60m` gives 10080.
- `src/time.rs:111-116` `format_minutes` gives `HHh MMm` with no upper width cap, so `format_minutes(5_999_999_999_940)` is `99999999999h 00m`.
- `src/time.rs:165-176` messages: `invalid duration "10x": expected a form like 20h, 33h30m, or 45m` and `duration cannot be negative (got -300 minutes)`.
- `src/cli.rs:278` holds the DURATION help line. `tests/week_target_cli.rs:263` holds the golden copy.
- `src/cli.rs:446-451` `is_duration_token` rejects only `InvalidFormat`, so `200h` still triggers the reminder (contract 8).

### 1.5 Test-side facts

- Test call sites that change because the signatures gain a writer:
  - `src/week_target.rs` tests, `run(` at lines 209, 218, 227, 236, 240, 249, 263, 277, 287, 302, 312, 313, 323 (13)
  - `src/week_view.rs` tests, `run(` at 1207, 1282, 2026, 2028, 2054 (5)
  - `src/export.rs` tests, `run_notes(` at 192, 200 (2)
  - `src/commands.rs` tests, handler `delete_note(`/`delete_punch(` at 1286, 1302, 1318, 1333, 1437, 1445, 1456, 1471, 1559, 1572, 1608, 1611, 1620, 1623, 1645, 1678, 1787, 1793 (18)
  - `src/main.rs` tests, `exit_code(` at 96, 102 (2)
  - `src/status.rs` tests never call `run`.
- `examples/seed_test_data.rs:242` calls only `week_target::set_week_target`. `tests/` calls none of the changed functions.
- Punch rows store `at_utc` as `%Y-%m-%dT%H:%M:%SZ` (`src/storage.rs:26`).
- `DbError::CreateDataDir` displays as `could not create data directory <path>: <io>` (`src/db.rs:119-122`).
- The process-test pattern is `Command::new(env!("CARGO_BIN_EXE_mlm"))` with `MLM_DB_PATH`, `TZ=UTC` and `env_remove("RUST_LOG")` (`tests/week_target_cli.rs:11-19`).
- A zero-length `&mut [u8]` fails every write with `WriteZero` (`src/commands.rs:2276-2278`).

### 1.6 Gates

- `clippy.toml`: `cognitive-complexity-threshold = 15`.
- `.githooks/pre-commit` runs `cargo fmt --check`, the clippy cognitive-complexity scan and `cargo llvm-cov --summary-only --json`, compared against `coverage-baseline.json` (99.47040498442368%).
- CI (`ci.yml`) runs fmt, build, test and clippy `-D warnings` on Linux x86/arm, Windows MSVC and macOS arm/x86, all on `stable`. Under `-D warnings`, any helper that is unused on Windows fails the Windows job.
- `Cargo.toml`: `rust-version = "1.85"`, edition 2024, dev-deps `chrono-tz 0.10.4` and `tempfile`.

### 1.7 Mechanisms verified in a scratch crate

I built a scratch crate at `/tmp/cd-scratch/panic-fixes-task-1` (rustc 1.99, `rust-version = "1.85"`, `cargo clippy --all-targets -- -D warnings` clean, so `incompatible_msrv` flags nothing). All of these passed:
- The closed-read-end pipe helper (§3.1), 50 runs in a row: exit `141`, empty stderr. With the same helper as stderr, the program exits `1`.
- Stdout set to `/dev/full`: stderr reads exactly `error: cannot write output: No space left on device (os error 28)\n`, exit `1`.
- The `TZ` rule builder (§3.2) on 2026-10-25 around 17:01 UTC. First pass gave `2026-10-25 18:01:00 +01:00` and second pass gave `2026-10-25 17:01:00 +00:00`, as predicted.
- `minute_floor` over `chrono_tz::Europe::Warsaw`. A second-pass instant keeps CET (+01:00) and a first-pass instant keeps CEST (+02:00). `instant.with_timezone(&Warsaw).with_second(0)` is `None` there, which proves the old expression's trap.

## 2. Implementation

### 2.1 `src/lib.rs`: the stdout write type

Add these below the `pub mod` list, next to `exit_code`:

```rust
use std::io::{ErrorKind, Write};

/// A failed write to stdout. `main` exits 141 on a broken pipe and
/// prints `error: cannot write output: <reason>` for anything else.
#[derive(Debug)]
pub struct OutputError(pub std::io::Error);

impl OutputError {
    pub fn is_broken_pipe(&self) -> bool {
        self.0.kind() == ErrorKind::BrokenPipe
    }
}

impl std::fmt::Display for OutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cannot write output: {}", self.0)
    }
}

impl std::error::Error for OutputError {}

/// Writes `text` to `out` and flushes. Every stdout write goes through
/// here, so a failure reaches `main` as an `OutputError`.
pub fn emit<W: Write + ?Sized>(out: &mut W, text: &str) -> Result<(), OutputError> {
    out.write_all(text.as_bytes())
        .and_then(|()| out.flush())
        .map_err(OutputError)
}
```

- `OutputError` gives no `source()`. Its `Display` already carries the I/O text, and a source would print it twice under `{:#}`.
- The flush lives in `emit`, so nothing is left in the buffer when `std::process::exit` runs. `main` needs no flush of its own (§4, R2).
- An empty `text` makes no syscall. So `mlm export notes` over an empty range exits `0` into a closed pipe (plan review U6, accepted).
- Module doc, line 3: change "No behavior lives here" to "Apart from the exit-code and stdout-write helpers below, no behavior lives here".
- `lib::exit_code` (`src/lib.rs:25-32`) keeps its 0/1 code. Change its doc's first line to "The test-side exit mapping: `Ok` is 0, any `Err` is 1. `main` adds exit 141 for a broken stdout pipe."

### 2.2 `src/main.rs`

Imports:

```rust
use std::ffi::OsString;
use std::io::Write;

use chrono::{DateTime, Local, TimeZone, Timelike, Utc};
use mlm::OutputError;
use mlm::cli::{self, Cli, Command, DeleteTarget, ExportTarget, WeekAction};
use mlm::{commands, db, export, status, week_target, week_view};

/// What a shell reports for a process killed by `SIGPIPE` (spec §5).
const EXIT_BROKEN_PIPE: i32 = 141;
```

`run()`, replacing lines 33-43 (keep the existing comment's first sentence and add the UTC point):

```rust
    // Seconds and nanoseconds are truncated at capture (§4.1: "no seconds
    // precision anywhere"), on the UTC instant, so a repeated local hour
    // cannot make it fail.
    let now = minute_floor(Utc::now(), &Local);

    let mut stdout = std::io::stdout().lock();
    let result = dispatch(&cli, now, &mut stdout);
    exit_code(&result, &mut std::io::stderr())
```

New function:

```rust
/// `instant` truncated to the whole minute, in `tz`. The truncation runs
/// in UTC, which has one instant per wall time, so it never fails.
fn minute_floor<Tz: TimeZone>(instant: DateTime<Utc>, tz: &Tz) -> DateTime<Tz> {
    instant
        .with_second(0)
        .and_then(|t| t.with_nanosecond(0))
        .unwrap_or(instant)
        .with_timezone(tz)
}
```

`unwrap_or(instant)` is never taken for `Utc`. It sits on the same line as the call, so it costs no line coverage and adds no panic site.

`dispatch` gains the writer. Its body is unchanged apart from the six printing arms:

```rust
fn dispatch<W: Write>(cli: &Cli, now: DateTime<Local>, out: &mut W) -> anyhow::Result<()> {
    ...
        Command::Delete(args) => match &args.target {
            DeleteTarget::Note(a) => commands::delete_note(&conn, now, a, out),
            DeleteTarget::Punch(a) => commands::delete_punch(&conn, now, a, out),
        },
        Command::Week(args) => match &args.action {
            Some(WeekAction::Target(t)) => week_target::run(&conn, now.date_naive(), t, out),
            None => week_view::run(&conn, now, args, out),
        },
        Command::Status { date } => status::run(&conn, now, date.as_deref(), out),
        Command::Export(args) => match &args.target {
            ExportTarget::Notes(a) => export::run_notes(&conn, now.date_naive(), a, out),
        },
```

`exit_code`, replacing lines 77-88:

```rust
/// Maps the command's result to the process exit code. `Ok` is 0. A
/// broken stdout pipe is 141 with nothing printed. Any other error
/// prints one `error: ` line on `stderr`, ignoring a failed write, and
/// is 1 (§6.3 only requires "nonzero").
fn exit_code<W: Write>(result: &anyhow::Result<()>, stderr: &mut W) -> i32 {
    match result {
        Ok(()) => 0,
        Err(e) if is_broken_pipe(e) => EXIT_BROKEN_PIPE,
        Err(e) => {
            let _ = writeln!(stderr, "error: {e}");
            1
        }
    }
}

/// True only when the top-level error is an `OutputError` for a broken
/// pipe. A storage-side I/O error is never an output error.
fn is_broken_pipe(e: &anyhow::Error) -> bool {
    e.downcast_ref::<OutputError>()
        .is_some_and(OutputError::is_broken_pipe)
}
```

`downcast_ref` checks the error object itself. It never walks `source()`, so `DbError::CreateDataDir`'s I/O source cannot match (contract 5, plan review F6). Every other `OutputError` takes the third arm and prints `error: cannot write output: <io display>`.

The `StdoutLock` is held for the whole dispatch. Nothing else writes stdout (clap prints help and version before this point, and env_logger writes stderr).

### 2.3 `src/cli.rs`

- Line 278 becomes contract 3, verbatim:
  `  <DURATION>  Target to set: 20h, 33h30m or 45m, up to 168h (human format, never raw minutes)"`
- Line 367: `Self::SwappedWeekTarget(message) => { let _ = writeln!(std::io::stderr(), "{message}"); }`. Add `use std::io::Write;` at the top.

### 2.4 `src/week_target.rs`: the cap

```rust
use std::io::Write;

use crate::emit;

/// The largest target `week target` accepts: the hours in a week (168h).
pub const MAX_TARGET_MINUTES: i64 = 168 * 60;

/// `week target` with a DURATION above `MAX_TARGET_MINUTES` (spec §4).
/// Holds the parsed minutes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetTooLarge(pub i64);

impl std::fmt::Display for TargetTooLarge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "target cannot exceed {} (got {})",
            format_minutes(MAX_TARGET_MINUTES),
            format_minutes(self.0)
        )
    }
}

impl std::error::Error for TargetTooLarge {}
```

In `apply`, between the `debug_assert!` (line 70) and `set_week_target` (line 72):

```rust
    if minutes > MAX_TARGET_MINUTES {
        return Err(TargetTooLarge(minutes).into());
    }
```

Extend `apply`'s doc. The order is WEEK_ID, then the DURATION parse, then the cap, and all three run before the single upsert.

`run` becomes:

```rust
/// `mlm week target [WEEK_ID] DURATION` (§3.7): writes the override, then
/// writes one line to `out` (§3.5). The target stays written when the
/// output fails.
pub fn run<W: Write>(
    conn: &Connection,
    today: NaiveDate,
    args: &WeekTargetArgs,
    out: &mut W,
) -> anyhow::Result<()> {
    let line = apply(conn, today, args)?;
    emit(out, &format!("{line}\n"))?;
    Ok(())
}
```

Contract 2's examples, derived with `format_minutes`:

| Input | Minutes | Result |
|---|---|---|
| `168h`, `167h60m`, `10080m` | 10080 | accepted, echo `168h 00m` |
| `168h1m`, `10081m` | 10081 | `error: target cannot exceed 168h 00m (got 168h 01m)` |
| `200h` | 12000 | `error: target cannot exceed 168h 00m (got 200h 00m)` |
| `99999999999h` | 5999999999940 | `error: target cannot exceed 168h 00m (got 99999999999h 00m)` |
| `9999999999999999999h` | parse fails | `error: invalid duration "9999999999999999999h": expected a form like 20h, 33h30m, or 45m` |
| `-5h` | parse fails | `error: duration cannot be negative (got -300 minutes)` |
| `10x` | parse fails | `error: invalid duration "10x": expected a form like 20h, 33h30m, or 45m` |

### 2.5 `src/status.rs`, `src/week_view.rs`, `src/export.rs`

Each gets `use std::io::Write;` and `use crate::emit;`, and its `run` gains `out: &mut W` as the last parameter:

```rust
pub fn run<W: Write>(conn: &Connection, now: DateTime<Local>, date_arg: Option<&str>, out: &mut W) -> anyhow::Result<()> {
    let view = resolve(date_arg, now, conn)?;
    emit(out, &render(&view))?;
    Ok(())
}
```

- `week_view::run<W: Write>(conn, now, args, out: &mut W)`: same shape, `emit(out, &render_week(&view))?`.
- `export::run_notes<W: Write>(conn, today, args, out: &mut W)`: same shape, `emit(out, &render_lines(&notes))?`.
- Docs: `status.rs:9` "prints on `Ok`" becomes "writes to `out` on `Ok`". `status.rs:520-523` "Prints the rendered view to stdout" becomes "Writes the rendered view to `out`". `export.rs:42-43` "returns before anything prints" becomes "returns before anything is written".

### 2.6 `src/commands.rs`

Add `use std::io::Write;` and `use crate::emit;`.

Replace `print_punch_list` and `print_note_list` (lines 193-213) with pure builders:

```rust
/// List-mode text for punches (spec §4): `<n>  <kind> <HH:MM>` per row,
/// 1-based. Pure, never touches storage.
fn punch_list_text(rows: &[Punch]) -> String {
    rows.iter()
        .enumerate()
        .map(|(i, p)| {
            let local_time = p.at_utc.with_timezone(&Local).time();
            format!("{}  {} {}\n", i + 1, p.kind.as_str(), local_time.format("%H:%M"))
        })
        .collect()
}

/// List-mode text for notes (spec §4): `<n>  <body>` per row, 1-based.
fn note_list_text(rows: &[Note]) -> String {
    rows.iter()
        .enumerate()
        .map(|(i, n)| format!("{}  {}\n", i + 1, n.body))
        .collect()
}

/// List mode's text for `date`: the numbered rows, or the
/// "nothing to delete" line when there are none.
fn list_or_nothing(text: String, date: chrono::NaiveDate) -> String {
    if text.is_empty() {
        format!("nothing to delete for {}.\n", format_date(date))
    } else {
        text
    }
}
```

`delete_note`, with `delete_punch` the same shape (`punch_list_text`, `punch_recreate_line(&deleted, today, &Local)`):

```rust
pub fn delete_note<W: Write>(
    conn: &Connection,
    now: DateTime<Local>,
    args: &DeleteEntryArgs,
    out: &mut W,
) -> anyhow::Result<()> {
    let today = now.date_naive();
    let target_date = resolve_target_date(&args.date, today)?;
    let rows = storage::notes_for_date(conn, target_date)?;

    let Some(id) = args.id else {
        emit(out, &list_or_nothing(note_list_text(&rows), target_date))?;
        return Ok(());
    };

    let index = validate_entry_id(id, rows.len())?;
    let deleted = storage::delete_note(conn, rows[index].id)?;
    let line = note_recreate_line(&deleted, today);
    emit(out, &format!("deleted. to recreate: {line}\n"))?;
    Ok(())
}
```

- Output bytes are unchanged. Each list now goes out in one write instead of one per row. That is still the same lines in the same order, and a failure part-way keeps whatever was written.
- The delete commits before `emit`, so a failed output keeps the delete (contract 6).
- Extend the docs of `delete_note` and `delete_punch` with "The delete stays when the output fails".

### 2.7 Existing test call sites (mechanical)

Each call site listed in §1.5 gains one trailing argument, `&mut std::io::sink()`. No assertion changes. The two `src/main.rs` tests become:

```rust
#[test]
fn ok_exits_zero() {
    let mut err = Vec::new();
    assert_eq!(exit_code(&Ok(()), &mut err), 0);
    assert!(err.is_empty());
}

#[test]
fn err_exits_nonzero() {
    let mut err = Vec::new();
    let result: anyhow::Result<()> = Err(anyhow::anyhow!("boom"));
    assert_eq!(exit_code(&result, &mut err), 1);
    assert_eq!(String::from_utf8(err).unwrap(), "error: boom\n");
}
```

`tests/week_target_cli.rs:263` takes contract 3's line. Nothing else in that file changes.

### 2.8 Complexity

| Function | Shape | Approx. score |
|---|---|---|
| `main::exit_code` | one 3-arm match with a guard | ≤ 3 |
| `main::dispatch` | unchanged nesting | unchanged |
| `week_target::apply` | adds one `if` | +1 |
| `commands::delete_note`/`delete_punch` | the `let-else` loses its inner `if` | lower than today |
| `list_or_nothing` | one `if` | 1 |
| test helpers in `tests/panic_fixes_cli.rs` | ≤ 2 branches each | ≤ 5 |

None comes near 15.

## 3. Test mechanisms

### 3.1 Closed-read-end pipe on Rust 1.85

`std::io::pipe` (1.87) is out because clippy's `incompatible_msrv` flags it. `libc` would be a new dev-dependency. The helper borrows a pipe from a short-lived child instead:

```rust
#[cfg(unix)]
/// The write end of a pipe whose read end is already closed. A helper
/// process holds the read end as its stdin and exits. Taking `stdin`
/// before `wait` keeps the write end open, because `wait` closes it.
fn closed_pipe() -> std::process::ChildStdin {
    let mut helper = Command::new(env!("CARGO_BIN_EXE_mlm"))
        .arg("--version")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn pipe helper");
    let write_end = helper.stdin.take().expect("helper stdin");
    helper.wait().expect("wait for pipe helper");
    write_end
}
```

- Pass the result as `.stdout(closed_pipe())` or `.stderr(closed_pipe())`, using `From<ChildStdin> for Stdio`.
- std's spawn resets `SIGPIPE` to default in the child, then `mlm`'s runtime ignores it again, so the write fails with `EPIPE`. `status.code() == Some(141)` also proves the process exited normally and did not die from a signal (a signal death gives `None`).
- The parent never holds the read end after the helper's spawn. Std creates both ends `O_CLOEXEC`, so a concurrent test's fork can hold the read end only until that child execs. That window is microseconds, and `mlm` writes milliseconds later, after opening the database (R4).
- Verified 50 runs in a row in scratch (§1.7).

### 3.2 POSIX `TZ` rule for the repeated hour

Build the rule so that the standard zone is UTC, DST is UTC+1, and DST ends at a computed instant `T`:

- Before `T`, local wall time is UTC+1. From `T` on, it is UTC. The repeated local hour is `[T, T+1h)` as a naive UTC wall time.
- Let `u` be the test's current UTC instant floored to the minute.
- **First pass**: `T = u + 30m`. Real now is 30 min before `T`, local now is `u + 1h`, and the margin is 30 min on each side.
- **Second pass**: `T = u - 30m`. Real now is 30 min after `T`, local now is `u`, and the margin is 30 min on each side.
- The POSIX end time is local time in DST, so the rule's end is the wall time `T + 1h`. Its day uses the zero-based `n` form (0-365, which counts 29 February), as plan review F8 asked. The start day is `0` (1 January 02:00).

```rust
#[cfg(unix)]
#[derive(Clone, Copy)]
enum Pass { First, Second }

#[cfg(unix)]
/// A POSIX `TZ` rule (standard UTC, DST UTC+1) whose DST ends so that
/// `now` sits in `pass` of the repeated hour, 30 minutes from each edge.
/// `None` when the repeated hour would not fit inside one local date
/// other than 1 January, where DST could not start before it ends.
fn repeated_hour_tz(now: DateTime<Utc>, pass: Pass) -> Option<String> {
    let u = now.with_second(0)?.with_nanosecond(0)?.naive_utc();
    let t = match pass {
        Pass::First => u + TimeDelta::minutes(30),
        Pass::Second => u - TimeDelta::minutes(30),
    };
    let end = t + TimeDelta::hours(1);
    if t.date() != end.date() || t.ordinal0() == 0 {
        return None;
    }
    Some(format!("AAA0BBB-1,0,{}/{}", end.ordinal0(), end.format("%H:%M")))
}
```

- Example: at 2026-10-25 17:01 UTC, the second pass gives `AAA0BBB-1,0,297/17:31`. Local now is `17:01 +00:00` and the repeated hour runs 16:31-17:31.
- The test skips when `T` and `T + 1h` fall on different dates, or when `T` is on 1 January. For the second pass that means `u` outside 00:30-23:30 UTC, and for the first pass `u` outside 23:30 (the day before) to 22:30. Each pass skips about one hour a day, plus all of 1 January.
- When `now`'s local date and the repeated hour share one date, the punch's `date` is that date in both passes.
- A skip prints `skipped: repeated hour crosses a date boundary` with `eprintln!` and returns, so the test passes. Rust has no runtime skip.

### 3.3 Repeated-hour storage assertion (second pass)

- Read `before = Utc::now()`, build the rule from `before`, spawn `mlm start` (no TIME), then read `after = Utc::now()`.
- The binary's `now` floors to `Nf` with `floor(before) <= Nf <= floor(after)`. Local wall time equals `Nf`, and storage takes the earlier pass, `Nf - 1h`.
- Read `SELECT at_utc, "date", kind FROM punches` with rusqlite and parse `at_utc` with `DateTime::parse_from_rfc3339`. Then assert:
  - one row, `kind == "start"`
  - `floor(before) - 1h <= at_utc <= floor(after) - 1h`. This tolerates a minute boundary between the clock reads.
  - `date == floor(before).date_naive()` formatted `%Y-%m-%d`
- The 30-minute margin keeps `Nf` inside the repeated hour.

## 4. Test plan

Process tests live in a new `tests/panic_fixes_cli.rs`. Its shared helpers run on every platform:

- `mlm(db: &Path, args: &[&str]) -> Command`: sets `MLM_DB_PATH`, `TZ=UTC`, `env_remove("RUST_LOG")`, as in `tests/week_target_cli.rs:11-19`.
- `out(&Output)` and `err(&Output)`: lossy UTF-8.
- `fresh() -> (TempDir, PathBuf)`.
- `target_rows(db) -> Vec<(String, i64)>`: reads `week_targets` with rusqlite.

Unix-only helpers and tests go inside `#[cfg(unix)] mod unix { use super::*; ... }`, and the Linux test inside it under `#[cfg(target_os = "linux")]`. Nothing is unused on Windows, so CI's `-D warnings` stays clean.

Exact-output cases assert stdout, stderr and exit code exactly. The clock-dependent cases (4.2, 4.3, 4.8) assert exit code, stderr and named clock-free lines (changeset plan's criteria preamble).

### 4.1 Truncation keeps the instant (spec §3, §6; plan AC1)

Unit tests in `src/main.rs` over `chrono_tz::Europe::Warsaw`. On 2026-10-25 the first pass is 00:00-00:59 UTC (CEST, +02:00) and the second pass is 01:00-01:59 UTC (CET, +01:00).

- `minute_floor_keeps_a_second_pass_instant`. Input `2026-10-25T01:22:37.5Z`.
  - `instant.with_timezone(&Warsaw).with_second(0)` is `None`, which documents the trap.
  - The result in UTC is `01:22:00Z`. `naive_local()` is `2026-10-25 02:22:00`. The offset is +3600 s (`offset().fix().local_minus_utc()`).
  - The result is not `00:22:00Z`, the first-pass rebuild.
- `minute_floor_keeps_a_first_pass_instant`. Input `2026-10-25T00:22:37Z` gives `00:22:00Z`, local `02:22`, offset +7200 s.

### 4.2 First pass, `status` exits 0 (spec §6 first bullet; plan AC2; Unix)

`repeated_hour_first_pass_status_exits_zero`. Build the rule for `Pass::First` (skip on `None`). Run `status` on a fresh DB with `TZ=<rule>`. Assert exit `0`, stderr `""`, and stdout containing `Day total:`.

### 4.3 Second pass, `status` and untyped `start` (spec §3, §6; plan AC3; Unix)

`repeated_hour_second_pass_status_and_untyped_start`. Build the rule for `Pass::Second` (skip on `None`).

- Run `status`. Assert exit `0`, stderr `""`, stdout containing `Day total:`.
- Run `start` per §3.3. Assert exit `0`, stderr `""`, stdout `""`, and the stored-instant assertions.

### 4.4 Cap boundary accepted (spec §4; contract 1; plan AC4)

- Process test `cap_boundary_is_accepted`. For each of `168h`, `167h60m`, `10080m` on a fresh DB, run `week target 2026-41 <d>`. Assert exit `0`, stdout exactly `Week 2026-41 target: 168h 00m\n`, stderr `""`, and `target_rows == [("2026-41", 10080)]`.
- Unit test `week_target::cap_boundary_values_are_accepted`. `apply` with each of the three values stores 10080.

### 4.5 Over cap rejected, nothing written (spec §4; contract 2; plan AC5)

- Process test `over_cap_is_rejected_and_writes_nothing`. Each case runs `week target 2026-41 <d>` on a fresh DB and expects exit `1`, stdout `""`, the exact stderr below, and `target_rows` empty:

  | `<d>` | stderr |
  |---|---|
  | `168h1m` | `error: target cannot exceed 168h 00m (got 168h 01m)\n` |
  | `10081m` | `error: target cannot exceed 168h 00m (got 168h 01m)\n` |
  | `200h` | `error: target cannot exceed 168h 00m (got 200h 00m)\n` |
  | `99999999999h` | `error: target cannot exceed 168h 00m (got 99999999999h 00m)\n` |

- Process test `over_cap_keeps_the_existing_row`. Set `week target 2026-41 40h`, then run `week target 2026-41 200h`. Assert exit `1` and `target_rows == [("2026-41", 2400)]`.
- Unit test `week_target::over_cap_is_rejected_after_the_parse_and_writes_nothing`. `apply` returns an error whose `downcast_ref::<TargetTooLarge>()` is `Some(&TargetTooLarge(10081))` (and so on), whose `to_string()` matches the table without the `error: ` prefix, and leaves `row_count == 0`.
- Unit test `week_target::cap_runs_after_the_week_id_check`. `apply(["2027-53", "200h"])` gives `DateWeekError`, not `TargetTooLarge`.

### 4.6 Parse errors keep their messages (spec §4; contract 2; plan AC6)

Process test `parse_errors_keep_their_messages`. Each case runs on a fresh DB and expects exit `1`, stdout `""`, the exact stderr below, and no row:

| Args | stderr |
|---|---|
| `week target 2026-41 -5h` | `error: duration cannot be negative (got -300 minutes)\n` |
| `week target 2026-41 10x` | `error: invalid duration "10x": expected a form like 20h, 33h30m, or 45m\n` |
| `week target 2026-41 9999999999999999999h` | `error: invalid duration "9999999999999999999h": expected a form like 20h, 33h30m, or 45m\n` |

Unit test `week_target::i64_overflow_keeps_the_format_error`. `9999999999999999999h` gives `DurationParseError::InvalidFormat`.

### 4.7 Swapped-target reminder still first (spec §4; contract 8; plan AC7)

- Process test `swapped_target_over_cap_gets_the_reminder`. Run `week 2026-41 target 200h`. Assert exit `2`, stdout `""`, and stderr exactly `error: put WEEK_ID after target: mlm week target 2026-41 200h\n`.
- Unit test, extending `src/cli.rs` `swapped_matches_expected_shapes`: `swapped(&["mlm", "week", "2026-41", "target", "200h"])` is `Some("error: put WEEK_ID after target: mlm week target 2026-41 200h")`.

### 4.8 `status` with an open stint and a 168h target (spec §6 third bullet; plan AC8)

Process test `open_stint_with_max_target_shows_est_eod`. On a fresh DB, run `week target 168h` (current week), then `start`, then `status`.

- Assert all three exit `0` with stderr `""`.
- Assert the `status` stdout has a line starting `Day total:` that contains `(incl. ongoing), est. EOD `.
- If that line is missing on the first attempt, rerun the whole case once on a new DB. That covers a midnight crossing between `start` and `status`, which cannot happen twice in a row.
- The gap is always positive. Monday's slice of 168h is 33h 36m, more than one day's live minutes. Friday to Sunday the full 168h is due. So `est. EOD` always prints, never `done for today`.

### 4.9 Help golden (spec §7; contract 3; plan AC9)

`tests/week_target_cli.rs` `week_target_help_text` keeps its structure. The `expected` DURATION line becomes contract 3's line. It already asserts that `--help` equals `-h`.

### 4.10 Broken pipe, read commands (spec §5, §6; contract 4; plan AC10; Unix)

Process test `closed_stdout_exits_141_quietly`, all with `.stdout(closed_pipe())` and `.stderr(Stdio::piped())`:

- `week 2026-41` on a fresh DB. Assert `code() == Some(141)` and stderr `""`.
- `status` on a fresh DB. Same asserts.
- `export notes --date 2026-10-19`, after seeding with `note --date 2026-10-19 piped`. Same asserts. The fixed past date avoids the week-rollover race. The seeded note makes the write non-empty (plan review F4).

### 4.11 Broken pipe, write commands keep their change (spec §5 last bullet, §6 last bullet; contract 6; plan AC11; Unix)

Process test `closed_stdout_keeps_committed_writes`:

- `week target 40h` with `.stdout(closed_pipe())`. Assert exit `141`, stderr `""`, and exactly one `week_targets` row holding `2400`. Counting rows avoids recomputing the current week id.
- `start --date 2026-10-19 09:00` (normal stdout), then `delete punch 1 --date 2026-10-19` with `.stdout(closed_pipe())`. Assert exit `141`, stderr `""`, and `SELECT COUNT(*) FROM punches` is `0`.

Unit tests with a failing writer (`let mut full: &mut [u8] = &mut [];`):

- `week_target::run_keeps_the_target_when_output_fails`. The `Err` downcasts to `crate::OutputError`, and `get_week_target` returns `Some(1200)`.
- `commands::delete_punch_keeps_the_delete_when_output_fails`, and the same for `delete_note`. The `Err` is an `OutputError` and the row is gone.

### 4.12 Other write failure (spec §5, §6; contract 5; plan AC12; Linux)

Process test `full_disk_prints_cannot_write_output`, under `#[cfg(target_os = "linux")]`. Run `week 2026-41` with stdout set to `OpenOptions::new().write(true).open("/dev/full")`. Assert exit `1`, and stderr exactly `error: cannot write output: No space left on device (os error 28)\n`.

Platform-neutral unit tests in `src/main.rs`:

- `broken_pipe_output_error_exits_141_silently`. `Err(OutputError(io::Error::from(ErrorKind::BrokenPipe)).into())` exits `141` and writes nothing to the stderr sink.
- `other_output_error_prints_cannot_write_output`. `OutputError(io::Error::other("disk on fire"))` exits `1` with sink text `error: cannot write output: disk on fire\n`.
- `bare_io_error_is_not_an_output_error`. `anyhow::Error::from(io::Error::from(ErrorKind::BrokenPipe))` exits `1`. The sink starts with `error: ` and does not contain `cannot write output`.

Process test `storage_io_error_keeps_its_own_message` (plan review F6), every platform. Use the blocker-file path from `tests/e6_db_failure.rs:30-34` and run `status`. Assert exit `1`, stderr starting `error: could not create data directory `, and stderr not containing `cannot write output`.

### 4.13 Stderr failures are ignored (spec §5 fourth bullet; contract 7; plan AC13; Unix)

Process test `closed_stderr_keeps_the_exit_code`, with `.stderr(closed_pipe())` and `.stdout(Stdio::piped())`:

- `start 25:00` gives exit `1` and stdout `""`. This covers `main`'s `error:` line.
- `week 2026-41 target 20h` gives exit `2` and stdout `""`. This covers `src/cli.rs:367`.

Unit test `main::stderr_write_failure_is_ignored`. `exit_code(&Err(anyhow!("boom")), &mut full)` with a zero-length slice returns `1`.

### 4.14 Writers receive the output (no spec case; coverage and regression)

- `lib.rs` `mod tests`:
  - `emit_writes_and_flushes`: into a `Vec`
  - `emit_maps_a_failed_write`: a zero-length slice gives `OutputError` with kind `WriteZero`, and `is_broken_pipe()` is false
  - `output_error_display`: `OutputError(io::Error::other("x")).to_string() == "cannot write output: x"`
- `status::run_writes_the_rendered_view`: in-memory DB, `now = Local.with_ymd_and_hms(2026, 2, 12, 14, 23, 0)`. `run` into a `Vec` equals `render(&resolve(None, now, &conn)?)`. A failing writer gives `OutputError`.
- `week_view::run_writes_the_rendered_week`: same pattern with `render_week(&build_week_view(..))`.
- `export::run_notes_writes_lines` and `run_notes_empty_range_writes_nothing_even_to_a_failing_writer`. The second returns `Ok` (records U6).
- `commands::delete_list_and_recreate_lines_reach_the_writer`. Seed `09:00`/`17:00` via `storage::insert_punch` with `&Local`, then:
  - list mode writes `"1  start 09:00\n2  end 17:00\n"`
  - an empty date writes `"nothing to delete for 2026-02-11.\n"` (yesterday of `fixed_now`)
  - `delete punch 1` writes `"deleted. to recreate: mlm start 09:00\n"`
- `commands::delete_note_lines_reach_the_writer`: the same for notes.

### 4.15 Existing tests (plan last AC)

All existing tests pass. Their assertions are unchanged. Only the call sites in §1.5 gain `&mut std::io::sink()`, and the two `main.rs` tests take a sink (§2.7). The help golden is the one changed expectation.

## 5. Verification

- `cargo fmt`, `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- The repeated-hour tests can also run by hand under a forced zone. `TZ=Europe/Warsaw cargo test --test panic_fixes_cli` must still pass, since the child gets its own `TZ`.
- Commit through the hook (`git config core.hooksPath .githooks`). Coverage should rise, since every new line has a test, and the hook ratchets and stages `coverage-baseline.json`.

## 6. Risks, ambiguities, disagreements

- **R1. Existing test call sites change.** Plan AC "Existing tests pass unchanged" cannot hold to the letter, because six functions gain a writer parameter.
  - The edit is mechanical: one `&mut std::io::sink()` argument at each site in §1.5, assertions untouched.
  - The alternative keeps the old signatures as stdout wrappers. Tests would then write to the real stdout, which the harness does not capture for `io::stdout()` writes, and the failure path would have no in-process test. I chose the parameter.
- **R2. The flush lives in `emit`, not in `main`.** The changeset plan says stdout is flushed before exit. `emit` flushes after every write, so the buffer is always empty at exit. A separate `main`-level flush would have an error branch no test can reach, which costs coverage under the ratchet.
- **R3. Two `exit_code` functions.** `lib::exit_code` (0/1, test-only) and `main::exit_code` (0/141/1, printing) now differ. The lib copy is used only by unit tests, and its doc is reworded to say so. Merging them is out of scope.
- **R4. Pipe-helper fork window.** A concurrently forked test child could hold the read end between its fork and its exec. The window is microseconds against milliseconds of DB setup before `mlm`'s first write. Scratch runs showed no flake in 50 tries.
- **R5. Repeated-hour tests skip silently.** They skip about one hour a day per pass and all of 1 January, and pass with an `eprintln!` note. CI runs at random times, so a skip is rare. No coverage depends on them, because every line they reach is covered elsewhere.
- **R6. Write granularity of delete lists.** Lists now go out in one write instead of one per row. The bytes are identical. On failure, std's `LineWriter` may keep a different number of complete lines than before. Spec §5 only requires that written lines stay written.
- **R7. `lib.rs` grows two items.** I chose `lib.rs` over `render.rs`, whose module doc promises pure string functions, and over a new module, which would need a coordinator decision. AGENTS.md's layout line for `lib.rs` does not mention it. Task 2 owns AGENTS.md and may add a word there, but nothing goes stale.
- **R8. Deterministic substitutes in two criteria.** Plan AC11's `mlm delete punch 1` runs with `--date 2026-10-19`, and AC10's export uses `--date 2026-10-19`. Both avoid a midnight or week-rollover race. `week target 40h` keeps the plan's exact form, and the test counts rows instead of computing the week id.
- **R9. Windows `141` stays untested** (changeset plan risk). The pipe tests are Unix-gated.
- **R10. A closed descriptor (`>&-`) still succeeds** (plan review U7). std turns `EBADF` into a successful write. This plan does not change that, and no test covers it.
- **R11. `--help`/`--version` into a closed pipe exits `0`** (plan review U5). `src/cli.rs:365` is unchanged by contract 8.
