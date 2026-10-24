# Plan: export-notes

Spec: `docs/dev/specs/2026-10-24-export-notes.md` (locked). Review: `docs/dev/plans/reports/export-notes-spec-review.md` (folded, NOTES.md entry 133). Decision log: NOTES.md entries 125, 128, 133 and the "Triaging review findings" process section.

**Altitude**: scope of work only. Task plans choose identifiers. The one new source file name below is pinned for ownership.

**Read for this plan**: the spec in full; NOTES.md entries 125-133 and "Triaging review findings"; `docs/dev/plans/week-notes-plan.md` (format reference); `src/cli.rs` (command enum, aliases, week args, help surface, parse and error classification); `src/main.rs` (dispatch); `src/lib.rs` (module list); `src/date.rs` (DATE and WEEK_ID parsing, the error type and its messages, the year domain); `src/storage.rs` (the ranged note read); `tests/week_target_cli.rs` (help goldens); `tests/ascii_sweep.rs`; AGENTS.md's layout list; SPEC.md's section headings.

## Goal

A new `mlm export notes` command prints the notes of one date, one ISO week or one calendar month to stdout, one line per note, as the date in `YYYY-MM-DD`, two spaces, then the body as stored (spec §3, §5). The range comes from at most one of `--date`/`-d`, `--week`/`-w` or `--month`/`-m`, and defaults to the current week. Lines run ascending by date and in insertion order within a date. An empty or future range prints nothing and exits `0`. The command makes one ranged note read and touches no punch, total or target (spec §4). `export` and `notes` get the aliases `ex` and `n`. Nothing about `status`, `week` or the write commands changes (spec §1.1).

## Architecture

The top-level command enum in `src/cli.rs` gains an `export` command that requires a subcommand, with one subcommand today, `notes`. The `notes` arguments hold three optional, mutually exclusive range flags, each taking hyphen values so `-d -1`, `-w -1` and `-m -1` reach the parsers instead of clap. Clap owns the usage errors (bare `export`, two range flags, a repeated flag, a flag with no value), so they exit `2` through the existing parse-failure path in `src/cli.rs` with no new handling there. The `n` alias for `notes` lives inside `export`, so it does not collide with the top-level `n` alias for `note`. `ex` does not collide with `e` (`stop`).

A new module, `src/export.rs`, holds the command's entry point and is registered in `src/lib.rs`. `src/main.rs` gains one dispatch arm for it. The entry point resolves the range against today's local date, makes the one ranged note read, formats the lines and prints them. It reuses three existing pieces:

- the core DATE resolver in `src/date.rs`, the one without the future check, so `-N` works and a future date is allowed
- the WEEK_ID parser and the week's Monday-Sunday span in `src/date.rs`
- the ranged note read in `src/storage.rs`, added by `week-notes`, which already orders by date, then `created_at_utc ASC, id ASC`

`src/date.rs` gains MONTH parsing: `YYYY-MM`, `YYYY-M`, or a bare `M`/`MM` in the current calendar year, mirroring the WEEK_ID shape, plus a month's first and last day. Its error type gains a MONTH argument kind, so the two new messages print through the same `invalid NAME "input": ...` form as DATE and WEEK_ID (spec §6). A month of February resolves to 28 or 29 days by the calendar.

No schema change, no new runtime dependency, and no change to the history loader, the week view or `status`.

## Global constraints

- Verification set: `cargo fmt`, `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- The pre-commit hook enforces cognitive complexity of 15 or less per function (`clippy.toml`) and a coverage ratchet against `coverage-baseline.json`. Every new branch needs a test: each MONTH error cause, each range flag, the default range and the read-failure path.
- No new runtime dependency and no schema change.
- Existing commands' output and help stay byte-identical. Existing tests keep their meaning. Mechanical updates to struct patterns and fixtures in owned files are allowed.
- A failed note read is an error with empty stdout, never an empty export (spec §6, SPEC §6.1).
- Note bodies pass through unchanged, non-ASCII included. All other output, help included, is ASCII.
- Errors exit `1` with `error: ...` on stderr and empty stdout. Clap usage errors exit `2`.

## Ordering and parallelization

Two tasks, run in parallel in separate worktrees. They share no file, and the contracts below are all they share. Nothing to sequence.

A finer split buys nothing. MONTH parsing, the clap surface and the command module are each small, and the CLI tests drive all three together. Splitting would put two tasks on `src/cli.rs` or on one test file.

## Interface contracts (pinned before worktrees open)

1. **Command and aliases.** `mlm export notes`. `export` has the visible alias `ex`; `notes` has the visible alias `n`. `mlm ex n -w 43` works.
2. **Top-level help.** `mlm --help` lists `export`. `mlm export --help` lists `notes`. `export`'s one-line description is `Print stored data as plain text`; `notes`'s is contract 3's command line.
3. **`mlm export notes --help` text** (spec §3), verbatim:
   - command: `Print notes for a date, week or month, one per line (defaults to the current week)`
   - `-d, --date <DATE>`: ``Date to export: YYYY-MM-DD, or `-N` for N days before today (e.g. `-1` = yesterday)``, backticks as in `status`'s DATE help
   - `-w, --week <WEEK_ID>`: `Week to export: YYYY-WW (e.g. 2026-07) or a bare week number (e.g. 7)`
   - `-m, --month <MONTH>`: `Month to export: YYYY-MM (e.g. 2026-03) or a bare month number (e.g. 3)`
4. **Output line.** `YYYY-MM-DD`, two spaces, the body as stored, newline. No header, no blank line, no summary. Empty range means empty stdout and exit `0`.
5. **Order.** Ascending by date, then `created_at_utc ASC, id ASC` within a date. A note belongs to its stored `date`.
6. **Ranges.** `--date`: one date, DATE forms of SPEC §3.4a, no future check. `--week`: Monday-Sunday of the WEEK_ID, forms of SPEC §3.6, bare `WW` in the current ISO year. `--month`: first to last calendar day, bare month in the current calendar year (not the ISO year: on 2025-12-30, `-m 1` is 2025-01), year exactly four digits with any value `0000`-`9999` accepted, as DATE's year is. None: the current ISO week.
7. **Error messages** (exit `1`):
   - bad DATE, bad WEEK_ID: the existing messages, unchanged
   - MONTH shape: `invalid MONTH "<input>": expected YYYY-MM or MM`
   - MONTH number 0 or above 12: `invalid MONTH "<input>": month must be 1-12`. The error type's catch-all out-of-range arm prints the week message today, so MONTH needs its own arm.
   - failed note read: the storage error, as other reads
8. **Usage errors** (clap, exit `2`): bare `mlm export` (its output lists `notes`), two or more range flags, a flag given twice, a flag with no value as the last token. Because the flags take hyphen values, a flag followed by another flag takes that token as its value and fails as that flag's exit-1 parse error (`-m -d` is a bad MONTH).
9. **Doc example.** SPEC and README use spec §5's block verbatim.

## Task 1: code and tests

**Spec**: §3 surface, §4 semantics, §5 layout, §6 errors, §7 testing, §8 help lines, "Housekeeping"; non-goals §1.1.

`src/cli.rs` gains the `export` command and the `notes` arguments with contracts 1-3 and 8, and its `WeekArgs` doc comment is re-wrapped to 80 columns (spec "Housekeeping"). `src/date.rs` gains MONTH parsing, the month span and the MONTH error kind with contract 7's messages, with unit tests for every form and cause. The new `src/export.rs` resolves the range, makes one ranged note read and prints contract 4's lines. `src/lib.rs` registers the module and `src/main.rs` dispatches to it. The module docs of `src/date.rs` and the new module say what they hold. CLI tests go in a new test file, driven through the binary with `MLM_DB_PATH` and `TZ=UTC`, seeding raw note rows the way `tests/week_notes_cli.rs` does, and naming past ranges wherever the case does not need today. Cases that depend on the real clock (default week, bare `WW`/`M`, `-d -1`, future ranges) compute the expected dates in the test from `Utc::now()`, matching the binary's `TZ=UTC`, or use unit tests over the resolver with a fixed today. One resolver test fixes today at 2025-12-30 and checks bare `-m 1` gives 2025-01, so a parser copied from WEEK_ID's ISO-year logic fails.

Acceptance criteria. CLI cases assert exact stdout, stderr and exit code.
- `--date`, `--week` and `--month` each print the notes inside the range, and notes dated the day before the first day and the day after the last day never appear.
- No range flag prints the current week's notes and nothing from the weeks either side.
- Notes on several dates print ascending by date. Two notes on one date, where the later `id` has the earlier `created_at_utc`, print in `created_at_utc` order, so an `id`-only sort fails.
- `-d -1` resolves to yesterday. `-w -1` prints `invalid WEEK_ID "-1": expected YYYY-WW or WW` and `-m -1` prints `invalid MONTH "-1": expected YYYY-MM or MM`, both exit `1`, empty stdout.
- A bare `WW` uses the current ISO year. A bare `M` and `MM` use the current calendar year. `YYYY-M` is accepted and equals `YYYY-0M`.
- February 2024 covers the 1st to the 29th, February 2026 the 1st to the 28th, December runs to the 31st.
- An empty range prints nothing and exits `0`.
- A future date, a future week and a future month print nothing and exit `0`.
- Each contract 7 row prints its exact message: a bad DATE, a bad WEEK_ID, MONTH shape errors (for example `2026-003`, `26-03`, `abc`, `2026-`), month `0`, `13` and `2026-13`. Each exits `1` with empty stdout.
- With the `notes.body` column renamed, `mlm export notes` prints `error: ...`, empty stdout and exits `1`.
- Each contract 8 case exits `2`, and bare `mlm export`'s output names `notes`.
- `mlm ex n` behaves like `mlm export notes`.
- A non-ASCII body passes through unchanged, and every other byte of stdout is ASCII.
- `mlm export notes --help` contains each contract 3 line. `mlm --help` lists `export` and `mlm export --help` lists `notes`.
- Existing tests pass unchanged, apart from mechanical updates the global constraints allow.

**Out of scope**: JSON, CSV or any other format. An output file. Punches, totals, targets. A `--from`/`--to` range. Any change to `status`, `week` or write commands. Any `.md` file.

**Owns exclusively**: `src/cli.rs`, `src/date.rs`, `src/lib.rs`, `src/main.rs`, the new `src/export.rs`, the new `tests/export_notes_cli.rs`, and `coverage-baseline.json`, which the pre-commit hook ratchets and stages. It reads `src/storage.rs` and `tests/week_notes_cli.rs` without editing them. No existing help golden changes: `tests/week_target_cli.rs` pins only the `status -h` and `week -h` output, and there is no top-level `mlm --help` golden. If one turns up, it belongs to this task. Any change to another file needs a coordinator decision first.

## Task 2: docs

**Spec**: §3 surface, §4 semantics, §5 layout and example, §6 errors, §8 user-visible surface, "Interaction with the project spec".

This task folds the behavior into the live docs so SPEC.md, README.md and AGENTS.md stand alone, with no pointer out to the working spec.

Acceptance criteria:
- SPEC §3 gains a `mlm export notes [--date DATE | --week WEEK_ID | --month MONTH]` subsection covering contracts 1, 3, 6 and 8, the hyphen-value rule, and that a future range is not an error.
- SPEC §6.1 gains the two MONTH errors (contract 7), adds the export note read to its read-failure rule, and its future-date rule names export as not checking.
- SPEC §7 gains the export layout (contracts 4 and 5) with spec §5's example verbatim.
- SPEC §8 gains a happy-path flow and the error flows for `mlm export notes`.
- README.md gains an `mlm export notes` section with spec §5's example and a Features line.
- AGENTS.md's layout list gains `src/export.rs`, its `src/date.rs` entry names MONTH parsing, its `src/storage.rs` entry names export as a caller of the ranged note read, and its "Current state" command list gains `export notes`.
- SPEC §6.2 (not errors) gains the empty range and the future range. SPEC §1.2 (non-goals) gains other export formats and an `--output` file.
- Every quoted help line, message and output line matches the contracts byte for byte.

**Out of scope**: `--help` text and module docs (Task 1). NOTES.md. Archiving the working spec. Any source or test file.

**Owns exclusively**: `docs/dev/SPEC.md`, `README.md`, `AGENTS.md`. NOTES.md and the spec archive banner stay with the coordinator.

## Risks and ambiguities

- **MONTH year range.** The spec requires a four-digit year but names no message for one outside `1000..=9999`. WEEK_ID rejects that domain, while DATE's `YYYY-MM-DD` form accepts any four digits. MONTH has only the two spec messages, so the reading the spec supports is to accept any four-digit year and let a far-past month print nothing. Task 1's plan pins this.
- **`mlm export` description.** The spec pins the `notes` help text but not `export`'s one-line description in `mlm --help`. Task 1's plan picks it (contract 2). Task 2 need not quote it.
- **Bare `mlm export` output.** The spec asks for exit `2` and a listing of `notes`. Whether clap prints the subcommand help or a short usage error is the task plan's choice, as long as both hold.
- **Clock-dependent tests.** The default range, bare `WW`/`M`, `-d -1` and future ranges depend on today. They need tests that compute the expected range from `Utc::now()`, or unit tests over a resolver that takes today as an input, so they do not flake at a week or month boundary inside a run.
