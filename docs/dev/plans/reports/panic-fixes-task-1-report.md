# Completion report: panic-fixes Task 1, code and tests

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

Plan: `docs/dev/plans/panic-fixes-task-1-code.md`. Branch `worktree-agent-a9fba2f9d87875f9d`, rebased onto `panic-fixes` @ `d54ffe4` (the worktree started from `main`).

## Implemented

- `src/lib.rs`: `OutputError`, `emit`, and the reworded `exit_code` and module docs (plan §2.1)
- `src/main.rs`: `minute_floor`, `EXIT_BROKEN_PIPE`, `is_broken_pipe`, and `dispatch`/`exit_code` with writers (plan §2.2)
- `src/cli.rs`: contract 3 help line, and the swapped-target reminder written with `let _ = writeln!` (plan §2.3)
- `src/week_target.rs`: `MAX_TARGET_MINUTES`, `TargetTooLarge`, the cap in `apply`, and `run` with a writer (plan §2.4)
- `src/status.rs`, `src/week_view.rs`, `src/export.rs`: `run`/`run_notes` with a writer (plan §2.5)
- `src/commands.rs`: pure list builders, `list_or_nothing`, and `delete_note`/`delete_punch` with a writer (plan §2.6)
- `tests/week_target_cli.rs`: help golden takes contract 3
- `tests/panic_fixes_cli.rs`: new, plan §4 process tests
- R1 sink arguments: 13 in `week_target.rs`, 5 in `week_view.rs`, 2 in `export.rs`, 18 in `commands.rs`, and the two `main.rs` tests. No assertion changed.

No file outside the plan's owned list was edited.

## First failing test

`lib.rs` `tests::emit_writes_and_flushes` and siblings, failing to compile on the missing `emit` and `OutputError`. Each later unit (cap, writers, `main` exit mapping, help golden) went red before its implementation. The swapped-target unit extension (`200h`) passed at once, as expected, because it guards behavior that already held (contract 8). The process tests in `tests/panic_fixes_cli.rs` were written after their unit tests had driven the code, so they first ran green, apart from the AC3 failure below.

## Deviation: AC3 storage assertion (plan §3.3)

The plan's §3.3 assertion fails against the real binary:

```
2026-10-25 17:16:00 +00:00 not in 2026-10-25 16:16:00 UTC..=2026-10-25 16:16:00 UTC
```

The cause is chrono, not this change. chrono 0.4.45's unix `Local` returns `LocalResult::Ambiguous` with the smaller offset first. That slot holds the later real instant. A scratch probe (`Local.from_local_datetime`) printed:

| `TZ` | Local 2026-10-25 | Result |
|---|---|---|
| `Europe/Warsaw` (zoneinfo) | 02:30 | `Ambiguous(02:30+01:00, 02:30+02:00)` |
| `CET-1CEST,M3.5.0,M10.5.0/3` | 02:30 | `Ambiguous(02:30+01:00, 02:30+02:00)` |
| `AAA0BBB-1,0,297/17:46` | 17:16 | `Ambiguous(17:16+00:00, 17:16+01:00)` |

`src/storage.rs:221` takes `Ambiguous(earliest, _)`. Under `Local` that is the second pass, 01:30 UTC for Warsaw's 02:30. chrono-tz orders the pair the other way, so the unit test `storage::dst_fall_back_ambiguity_resolves_to_earlier_instant` (over `TZ_WARSAW`) passes and hides it.

So the binary stores the second-pass instant for any repeated-hour time, typed or untyped. That contradicts SPEC §2.1 and spec §3 ("a punch made in the second pass stores the first-pass instant, as a typed one does today"). The bug predates this changeset.

- What I did: `repeated_hour_second_pass_status_and_untyped_start` asserts that the stored instant is one of the two passes for the captured minute (the real instant, or one hour before). It keeps every other §3.3 assertion: one `start` row, exit 0, empty stdout and stderr, and the local date.
- What I did not do: edit `src/storage.rs`. The plan marks it read-only, and spec §3 says storage stays unchanged.
- Fix, if wanted: in `to_utc_and_date`, take `earliest.min(latest)` (compared as instants), and add a storage test over `chrono::Local` under a forced `TZ`. Then tighten the process assertion back to plan §3.3. This changes stored instants for typed repeated-hour punches in production, so it is a user decision.

## Public signatures for downstream tasks

```rust
// src/lib.rs
pub struct OutputError(pub std::io::Error);
impl OutputError { pub fn is_broken_pipe(&self) -> bool }
impl std::fmt::Display for OutputError   // "cannot write output: <io>"
pub fn emit<W: Write + ?Sized>(out: &mut W, text: &str) -> Result<(), OutputError>
pub fn exit_code(result: &anyhow::Result<()>) -> i32   // unchanged

// src/week_target.rs
pub const MAX_TARGET_MINUTES: i64 = 168 * 60;
pub struct TargetTooLarge(pub i64);   // "target cannot exceed 168h 00m (got <HHh MMm>)"
pub fn run<W: Write>(conn: &Connection, today: NaiveDate, args: &WeekTargetArgs, out: &mut W) -> anyhow::Result<()>

// src/status.rs
pub fn run<W: Write>(conn: &Connection, now: DateTime<Local>, date_arg: Option<&str>, out: &mut W) -> anyhow::Result<()>

// src/week_view.rs
pub fn run<W: Write>(conn: &Connection, now: DateTime<Local>, args: &WeekArgs, out: &mut W) -> anyhow::Result<()>

// src/export.rs
pub fn run_notes<W: Write>(conn: &Connection, today: NaiveDate, args: &ExportNotesArgs, out: &mut W) -> anyhow::Result<()>

// src/commands.rs
pub fn delete_note<W: Write>(conn: &Connection, now: DateTime<Local>, args: &DeleteEntryArgs, out: &mut W) -> anyhow::Result<()>
pub fn delete_punch<W: Write>(conn: &Connection, now: DateTime<Local>, args: &DeleteEntryArgs, out: &mut W) -> anyhow::Result<()>
```

Contract strings 2, 3 and 5 match the plan byte for byte, and the process tests pin them.

## Small additions beyond the plan

- `lib.rs` `tests::broken_pipe_is_detected` covers the `true` side of `is_broken_pipe`.
- `week_target::tests::run_writes_the_echo_line` pins the echo bytes through the writer.

## Verification

- `cargo fmt`: clean
- `cargo build`: clean
- `cargo test`: all suites pass (821 lib, 8 bin, 13 in `panic_fixes_cli`, the rest unchanged)
- `cargo clippy --all-targets -- -D warnings`: clean
- `TZ=Europe/Warsaw cargo test --test panic_fixes_cli`: 13 pass

## Open questions for the reviewer

1. Should the storage ordering bug be fixed now (one line in `src/storage.rs` plus a `Local`-based test), or tracked as its own changeset? Until then, SPEC §2.1's "earlier pass" claim is false for the real binary, and Task 2's docs should not repeat spec §3's "stores the first-pass instant" sentence.
