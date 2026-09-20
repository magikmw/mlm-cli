# Milestone 7 adversarial review — `start`, `stop`, `note` commands

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.


Reviewer: independent read of the diff (`git show --stat HEAD` confirms
only `src/cli.rs`, `src/commands.rs`, `src/main.rs`, and the report were
touched — no scope creep into `src/db.rs`, `src/time.rs`, `src/date.rs`,
`src/storage.rs`, `src/stint.rs`, `src/week.rs`, `src/week_target.rs`).

## Verification performed

- `cargo build`: clean.
- `cargo test`: **218 passed, 0 failed, 0 ignored** — matches the
  implementer's report exactly.
- `cargo clippy --all-targets -- -D warnings`: clean, zero warnings.
- Manual invocations against a throwaway `MLM_DB_PATH` temp file
  (cleaned up afterward):
  - `start 09:00` / `start 10:00`: exit 0, stdout empty, stderr empty
    (confirms §7.4 total silence on success).
  - `start abc`: exit 1, stderr `error: invalid time "abc": expected
    HH:MM, HHMM, or HH (24-hour)`, no rows written.
  - `start "some text"`: exit 1, **same "invalid time" error** — the
    string is never shape-sniffed or reinterpreted as a note. Confirms
    §3.2's strict positional order is actually implemented, not just
    documented.
  - `note ""`: exit 1, stderr `error: note body is empty or
    whitespace-only`, no row written.
  - **E5 critical guarantee, traced end-to-end**: seeded 2 punches (0
    notes), then ran `start 11:00 "   "` → exit 1, and confirmed via
    direct `sqlite3` queries that punches stayed at exactly 2 and notes
    stayed at exactly 0 — no orphaned punch, no partial note, and no
    disturbance of prior rows. This is the single most important
    guarantee in the milestone and it holds.
  - `--verbose start 12:00`: DEBUG-level `rusqlite_migration` and the
    handler's own `log::debug!("db path: ...")` lines appear on stderr;
    the earlier non-verbose runs show none of these, confirming
    `--verbose` actually raises the log level and that omitting it
    doesn't error or change success behavior.
  - `log`: exit 2 (clap "unrecognized subcommand"), confirming
    `Command::Log` is genuinely gone at the CLI-parsing level, not just
    absent from the enum in isolation.
  - `week` (no target): exit 101, panics on the pre-existing `todo!()`
    from Milestone 8 — unchanged behavior, correctly left alone by this
    milestone, and correctly flagged as such in the report rather than
    silently left for the reviewer to discover.

## Findings

None. Every check below passed with no code-level issues worth flagging:

- **E5 orphan guarantee**: enforced twice, independently — `commands.rs`
  never reaches storage for TIME errors (validated first), and
  `storage::insert_punch_with_note` (Milestone 4, called verbatim)
  validates the note body before opening the transaction *and* wraps
  the punch+note pair in a real `rusqlite::Transaction` whose `Drop`
  rolls back on early return. Both the "validate first" and "real
  transaction" halves of the plan's design are present. Verified live
  (see above) and covered by unit tests T14/T18/T19/T20 plus storage's
  own `insert_punch_with_note_rejects_empty_note_leaving_no_orphaned_punch`.
- **E1 hard error**: malformed `TIME` on `start`/`stop` returns
  `anyhow::Error` downcastable to `time::TimeParseError`, exits nonzero,
  writes nothing, and prints a plain-ASCII stderr line. Confirmed live
  and in tests (T13, T14, T15).
- **§7.4 silence on success**: no `println!`/`print!` call anywhere in
  `commands.rs` or the `Start`/`Stop`/`Note` dispatch path in
  `main.rs`. Confirmed live: stdout and stderr both empty on a
  successful `start`.
- **Strict positional order (§3.2/§3.3)**: `PunchArgs` has exactly one
  optional `TIME` positional followed by a trailing `Vec<String>` NOTE
  — there is no shape-sniffing code path anywhere in `commands.rs` or
  `cli.rs`. `start "some text"` hard-errors as a malformed TIME, live
  and in test T16 (`note_positional_after_time_is_never_reparsed_as_time`).
- **`Command::Log` removal**: gone from the `Command` enum, gone from
  `dispatch`'s match, and a regression test (`log_subcommand_is_gone`)
  plus a live `mlm log` invocation both confirm it's a clap-level
  "unrecognized subcommand" (exit 2), not silently mapped to anything.
- **`Command::Week` untouched**: `WeekArgs`/`WeekAction`/`WeekTargetArgs`
  in `cli.rs` are byte-for-byte what Milestone 8 landed (confirmed via
  `git show --stat` — `cli.rs`'s diff only adds the new `Start`/`Stop`/
  `Note` shapes and deletes `Log`; the `week`/`week target` parse tests
  P1–P6 are original and still pass). `main.rs`'s dispatch still routes
  `Command::Week(_)` to the pre-existing `todo!()`, exactly as
  Milestone 8 left it — this milestone neither broke it nor pretended
  to finish it, and the report is explicit that this is out of scope.
- **`--verbose` / `env_logger` (contract 12)**: initialized once in
  `run()` before `dispatch()` runs, gated on `cli.verbose`, defaulting
  to `Info` and raising to `Debug` when passed. Confirmed live that
  omitting `--verbose` does not error and produces no debug spam, and
  that passing it does raise visible log output.
- **Contract 7 (`anyhow`, no shared enum)**: `commands.rs` and `main.rs`
  contain zero `match ... Err` / `map_err` / `.context()` calls outside
  test code — every fallible call uses bare `?`, relying on `anyhow`'s
  blanket `From` impl over `time::TimeParseError` and
  `storage::StorageError`. No hand-rolled `AppError` enum was
  introduced, and no local `EmptyNoteError` was reintroduced now that
  Milestone 4's `StorageError::EmptyNote` exists — a sensible deviation
  from the (pre-M4) plan, explained accurately in the report.
- **"now" truncation**: truncated to minute granularity exactly once,
  in `main::run()` (`with_second(0).unwrap().with_nanosecond(0)`),
  before being threaded down as a parameter through `dispatch` →
  `commands::start/stop/note` → `storage::insert_punch_with_note` /
  `insert_note`. `storage.rs`'s `to_utc_and_date` and `insert_note` also
  truncate defensively a second time — this is deliberate
  defense-in-depth on Milestone 4's side (documented as such in
  storage.rs and flagged as an intentional agreement in the plan's
  §8.4), not an accidental double-truncation bug: truncating an
  already-truncated value is idempotent and produces the same result.
- **`.unwrap()`/`.expect()` audit**: the only two `.unwrap()` calls
  outside test code are `main.rs`'s `with_second(0).unwrap()` /
  `with_nanosecond(0).unwrap()` on the freshly-captured `Local::now()`
  — setting a literal 0 second/nanosecond on any valid time is
  infallible in `chrono`, not user-input-dependent (the same pattern,
  with an explicit comment to that effect, exists in `storage.rs`).
  Every other `.unwrap()`/`.expect()` in the two new files lives inside
  `#[cfg(test)]` modules.
- **DB API usage (§7.2 confirm)**: `commands.rs` calls
  `storage::insert_punch_with_note(conn, kind, today, time_of_day,
  &Local, note_text.as_deref(), now.with_timezone(&Utc))` — matching
  Milestone 4's real signature exactly (borrowed `&mut Connection`,
  local wall-clock date+time, generic timezone, optional note body,
  explicit UTC `now`). No guessed shape survived from the plan; the
  report's stated deviations (thinner handlers, no local
  `EmptyNoteError`, no duplicated T22-style mechanism test) are
  accurate and reasonable given what Milestone 4 actually shipped.
- **Test placement**: inline `#[cfg(test)]` modules in `commands.rs`
  and `cli.rs` rather than a `tests/write_commands.rs`, justified by
  the binary-only crate (no `lib.rs` target) — consistent with every
  other milestone's existing convention (`storage.rs`, `time.rs`,
  `week.rs`, `week_target.rs` all do the same).
- **Code quality**: idiomatic, minimal, no dead weight; `punch()`'s
  shared implementation avoids duplicating `start`/`stop`; `join_note`
  is a small pure helper with a clear doc comment about the `None` vs.
  `Some("")` distinction that matters for E5.

## Verdict

**APPROVE** — mergeable as-is. Build, full test suite (218/218), and
clippy (`-D warnings`) are all clean; the milestone's single most
load-bearing guarantee (no orphaned punch on a rejected note) was
traced end-to-end both by reading the code and by direct manual
reproduction against a real SQLite file, and it holds. No scope creep,
no reintroduced shape-sniffing, no panics on user-input paths, and the
implementer's report accurately describes every deviation from the
(necessarily pre-M4) plan.
