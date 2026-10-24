# Task plan: export-notes, Task 1 (code and tests)

Changeset plan: `docs/dev/plans/export-notes-plan.md`, "Task 1: code and tests".
Spec: `docs/dev/specs/2026-10-24-export-notes.md` (locked).
Branch: `export-notes` at `e80fa86`. No source file was edited to write this plan.

## Read for this plan

- `docs/dev/plans/export-notes-plan.md` in full (contracts 1-9, Task 1, risks)
- `docs/dev/specs/2026-10-24-export-notes.md` in full
- `docs/dev/plans/export-notes-task-2-docs.md` in full (sibling, docs)
- `src/cli.rs` in full (956 lines)
- `src/date.rs` lines 1-452, 595-716, 990-1032, 1110-1125, and the item outline of its tests
- `src/lib.rs` and `src/main.rs` in full
- `src/storage.rs` lines 1-19, 33-46, 117-175, 465-506, 584-585 and the `notes_in_range` test names (1548-1640)
- `src/week_view.rs` lines 1-40, 300-363, 1134-1135
- `src/status.rs` lines 415-430 (the only other `Cause` match), `src/commands.rs:1114`
- `src/db.rs` public function list
- `tests/week_notes_cli.rs` and `tests/ascii_sweep.rs` in full
- `tests/week_target_cli.rs` lines 1-40, 160-250, 385-400 (help goldens)
- `clippy.toml`, `.githooks/pre-commit`, `Cargo.toml` dependencies
- A clap 4.6.6 prototype at `/tmp/cd-scratch/export-notes-task-1` (offline build) that confirmed every parse and help behaviour cited below as "verified"

## Scope

Owns exclusively:

- `src/cli.rs`
- `src/date.rs`
- `src/lib.rs`
- `src/main.rs`
- `src/export.rs` (new)
- `tests/export_notes_cli.rs` (new)
- `coverage-baseline.json` (the pre-commit hook ratchets and stages it)

Must not touch:

| File | Owner |
|---|---|
| `docs/dev/SPEC.md`, `README.md`, `AGENTS.md` | Task 2 |
| `docs/dev/NOTES.md`, the spec archive banner, all plans | coordinator |
| `src/storage.rs`, `src/week_view.rs`, `src/status.rs`, every other `src/` file | nobody in this changeset (read-only) |
| `tests/ascii_sweep.rs`, `tests/week_notes_cli.rs`, `tests/week_target_cli.rs`, every other existing test | nobody (read-only) |

Handoffs, against the changeset plan's contracts only:

- Task 2 quotes contracts 2, 3 and 7 byte for byte. This task ships the same bytes (section 4.1, 3.2).
- Task 2's AGENTS.md entry names `src/export.rs` as "resolves the range, makes one ranged note read and prints one line per note". The module below does exactly that.
- Task 2 numbers the new SPEC sections §3.8 and §7.5. The module docs below cite those numbers.

## 1. Facts about the current code

### 1.1 `src/cli.rs`

- Line 6 imports `clap::{Args, CommandFactory, FromArgMatches, Parser, Subcommand}`. Nothing new is needed.
- `Command` enum, lines 74-105. Variants in help order are `Start` (`s`), `Stop` (`e`), `Note` (`n`), `Delete` (`del`), `Week` (`w`), `Status` (`d`). Clap adds `help` last.
- Top-level command names max out at 6 characters (`delete`, `status`). `export` is 6 too, so the top-level help column does not move.
- `DeleteArgs`/`DeleteTarget`, lines 163-187, are the model for a command with a required subcommand. `DeleteArgs` holds `#[command(subcommand)] pub target: DeleteTarget` with no `Option`.
- Every `-d` flag already uses `#[arg(short, long, value_name = "DATE", allow_hyphen_values = true)]` (lines 124, 144, 217).
- The `WeekArgs` doc comment, lines 226-229, has a first line of 86 columns. The rest are 72, 73 and 67.
- `command()`, lines 287-292, adds the glossary to `status` and `week` only. Export needs no change there.
- `classify`, lines 335-347, sends every stderr-bound clap error through `swapped_week_target` (which only matches `week`/`w`) and `rename_args_context` (which only touches `<ARGS>`). Both are no-ops for export, so its usage errors keep clap's exit `2`.
- Unit tests, lines 420-956, call `Cli::try_parse_from` via a local `parse` helper (line 424) and `super::parse` for the `ParseFailure` path. `command_passes_debug_assert` (line 828) runs clap's debug assertions over the whole tree.

### 1.2 `src/date.rs`

- Module doc, lines 1-6, names DATE and WEEK_ID only. Line 11 is `#![allow(dead_code)]`.
- Line 13 imports `chrono::{Datelike, Days, NaiveDate, Weekday}`.
- `ArgKind`, lines 19-24, has `Date` and `WeekId`. Its doc says "this milestone's two argument kinds".
- `Cause`, lines 27-45. `OutOfRange`'s doc says "(week `0`, or a year outside `1000..=9999`)".
- `Display for DateWeekError`, lines 66-89. The name match (68-71) is exhaustive over `ArgKind`, so a new variant fails to compile until it gets a name. The cause match has a catch-all `(_, Cause::OutOfRange) => "week number must be 1 or greater"` at line 77. A MONTH out-of-range arm must sit above it.
- `resolve_date`, lines 120-156, is the core resolver with no future check. `-N` results outside `1000..=9999` are `OutOfRange`. `YYYY-MM-DD` via `parse_date` (108-118) accepts any four digits, `0000` included.
- `MIN_YEAR = 1000`, `MAX_YEAR = 9999` at lines 209-210.
- `WeekId::current(today)` (263) and `WeekId::span()` (312) give `(Monday, Sunday)`.
- `parse_week_id`, lines 369-387, is the shape MONTH copies. A `-` splits a four-ASCII-digit year from the rest. No `-` means the bare form against `today.iso_week().year()`. The number part is 1-2 ASCII digits.
- Test helpers at lines 430-449 are `d(y, m, day)`, `wk(y, w)`, `TODAY()` (2026-02-12), `date_err`, `week_err`. `junk_input_never_panics` is at line 1010.
- `ArgKind` and `Cause` are used outside the module only in `src/status.rs:426` and `src/commands.rs:1114`, both via `matches!` on `Cause::Future`. A new `ArgKind` variant breaks neither.

### 1.3 `src/lib.rs`, `src/main.rs`

- `src/lib.rs` lines 7-22 list modules alphabetically. `pub mod export;` goes between `db` (12) and `history` (13).
- `src/main.rs:35` is `use mlm::cli::{self, Cli, Command, DeleteTarget, WeekAction};` and `:36` is `use mlm::{commands, db, status, week_target, week_view};`.
- `dispatch`, lines 46-72, opens the connection first (`db::connect()?`), then matches `&cli.command`. `Delete` (59-62) is the nested-subcommand arm shape to copy.
- `exit_code`, lines 77-85, prints `error: {e}` on stderr and returns `1`. A `DateWeekError` or `StorageError` lifted into `anyhow` prints its own `Display`.
- `now` is `Local::now()` truncated to the minute (37-41). `now.date_naive()` is today's local date, which is UTC under the tests' `TZ=UTC`.

### 1.4 `src/storage.rs` (read-only)

- `notes_in_range(conn, from, to) -> Result<Vec<Note>, StorageError>`, lines 479-506. It is inclusive on both ends and orders `"date" ASC, created_at_utc ASC, id ASC`. An inverted range is empty.
- `Note` (117-132) has `pub date: NaiveDate` and `pub body: String`. The body is stored trimmed.
- `StorageError::Db` displays as `database error: {source}` (166). `week_view.rs:311` already lifts it into `anyhow` with `?`.

### 1.5 `src/week_view.rs` (read-only, nearest command module)

- `run(conn, now, args) -> anyhow::Result<()>` (318-331) resolves the week with `parse_week_id(s, today)?` or `WeekId::current(today)`, reads, then prints once with `print!`. A read error returns before anything prints, which keeps stdout empty.
- In-memory test DBs are built as `Connection::open_in_memory()` plus `db::apply_migrations(&mut conn)` (1134-1135).

### 1.6 Tests and gates

- `tests/week_notes_cli.rs` has helpers `mlm(db, args)` with `MLM_DB_PATH` and `TZ=UTC` (16-23), `stdout`, `stderr`, `fresh()` via `mlm::db::connect_at` (34-39), `seed_note(db, date, body, created_at_utc)` by raw insert (50-57), and `ok_stdout`. The read-failure case renames `notes.body` with `ALTER TABLE notes RENAME COLUMN body TO body_gone` (274-291). `db::connect` still succeeds on that DB.
- The ASCII check pattern is `b == b'\n' || (0x20..=0x7E).contains(&b)` after removing the known non-ASCII body (`week_notes_cli.rs:303-309`).
- `tests/week_target_cli.rs` pins only `status -h` and `week -h` goldens (STATUS_SHORT, WEEK_SHORT). No top-level `mlm --help` golden exists, so no existing golden changes.
- Dev-dependencies are `tempfile` and `chrono-tz`. Integration tests may use `chrono`, `rusqlite` and `mlm::*` as normal dependencies.
- `clippy.toml` sets `cognitive-complexity-threshold = 15`. The hook runs `cargo fmt --check`, clippy with `-W clippy::cognitive_complexity`, and a `cargo llvm-cov` line-coverage ratchet against `coverage-baseline.json`.

### 1.7 Verified clap 4.6.6 behaviour (prototype)

| Input | Result |
|---|---|
| `export`, `ex` | `DisplayHelpOnMissingArgumentOrSubcommand`, exit 2, help on stderr listing `notes  ... [alias: n]` |
| `ex n -w 43` | parses, `week = Some("43")` |
| `export notes -d -1` / `-w -1` / `-m -1` | parse, value `"-1"` |
| `export notes -m -d` | parses, `month = Some("-d")` |
| `export notes -d 2026-01-01 -w 3` | `ArgumentConflict`, exit 2 |
| `export notes -d 1 -d 2` | `ArgumentConflict` ("cannot be used multiple times"), exit 2 |
| `export notes -d` (last token) | `InvalidValue` ("a value is required"), exit 2 |
| `n hi` (top level) | still `Note` |
| struct-level `///` docs on `ExportArgs`/`ExportNotesArgs` | do not reach any help text; the variant doc wins |
| `NaiveDate::from_ymd_opt(0, 1, 1)` | `Some(0000-01-01)` |

## 2. `src/date.rs` changes

### 2.1 Module doc (replace lines 1-6)

```rust
//! Calendar dates (`DATE`), ISO week ids (`WEEK_ID`) and calendar months
//! (`MONTH`): parsing, validation, formatting, week iteration and a
//! month's first and last day.
//!
//! Spec basis: SPEC.md §1.3 (a week is an ISO year + ISO week tuple),
//! §2.3 (`week_targets.week_id` storage form), §3.5 (`DATE` argument),
//! §3.6 (`WEEK_ID` argument forms), §3.8 (`MONTH` argument forms), §6.1
//! (hard errors).
```

### 2.2 `ArgKind` gains `Month`

```rust
/// Which argument kind failed to parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgKind {
    Date,
    WeekId,
    Month,
}
```

`Cause::OutOfRange` doc becomes:

```rust
    /// Matched the grammar but a component is outside its legal range
    /// (week `0`, a year outside `1000..=9999`, or a month outside 1-12).
    OutOfRange,
```

### 2.3 `Display` (lines 66-89)

Name match gains `ArgKind::Month => "MONTH",`. The cause match gains two arms directly after the `WeekId` shape arm (line 75) and before the catch-all at line 77:

```rust
            (ArgKind::Month, Cause::Shape) => write!(f, "expected YYYY-MM or MM"),
            (ArgKind::Month, Cause::OutOfRange) => write!(f, "month must be 1-12"),
```

So the two messages render as contract 7 requires:

- `invalid MONTH "<input>": expected YYYY-MM or MM`
- `invalid MONTH "<input>": month must be 1-12`

### 2.4 `Month` type and parser

New section after `WeekId`'s `Display` impl (after line 387, before "Week iteration"):

```rust
// ---------------------------------------------------------------------------
// MONTH
// ---------------------------------------------------------------------------

/// A calendar month: (calendar year, month 1-12).
///
/// Invariant: `1 <= month <= 12` and `0 <= year <= 9999`, the same
/// four-digit domain `parse_date` accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Month {
    year: i32,
    month: u32,
}

impl Month {
    /// Validated constructor. `input` only populates `DateWeekError::input`.
    pub fn new(year: i32, month: u32, input: &str) -> Result<Self, DateWeekError> {
        if !(1..=12).contains(&month) || !(0..=MAX_YEAR).contains(&year) {
            return Err(DateWeekError::new(ArgKind::Month, input, Cause::OutOfRange));
        }
        Ok(Self { year, month })
    }

    /// The 1st of the month.
    pub fn first_day(self) -> NaiveDate {
        NaiveDate::from_ymd_opt(self.year, self.month, 1)
            .expect("Month's invariant guarantees the 1st exists")
    }

    /// The month's last calendar day: the day before the next month's 1st.
    pub fn last_day(self) -> NaiveDate {
        let (year, month) = if self.month == 12 {
            (self.year + 1, 1)
        } else {
            (self.year, self.month + 1)
        };
        NaiveDate::from_ymd_opt(year, month, 1)
            .and_then(|d| d.pred_opt())
            .expect("the next month's 1st exists for every year up to 9999")
    }

    /// `(first day, last day)`, inclusive.
    pub fn span(self) -> (NaiveDate, NaiveDate) {
        (self.first_day(), self.last_day())
    }
}

/// Parse a `MONTH` argument (§3.8): `YYYY-MM`, `YYYY-M`, or a bare
/// `M`/`MM` whose year defaults to `today`'s **calendar** year.
pub fn parse_month(s: &str, today: NaiveDate) -> Result<Month, DateWeekError> {
    let shape_err = || DateWeekError::new(ArgKind::Month, s, Cause::Shape);
    let (year, month_str) = match s.split_once('-') {
        Some((y, m)) => {
            if y.len() != 4 || !y.bytes().all(|c| c.is_ascii_digit()) {
                return Err(shape_err());
            }
            (y.parse::<i32>().map_err(|_| shape_err())?, m)
        }
        None => (today.year(), s),
    };
    if month_str.is_empty()
        || month_str.len() > 2
        || !month_str.bytes().all(|c| c.is_ascii_digit())
    {
        return Err(shape_err());
    }
    let month: u32 = month_str.parse().map_err(|_| shape_err())?;
    Month::new(year, month, s)
}
```

Notes for the implementer:

- `today.year()` is the calendar year. `parse_week_id` uses `today.iso_week().year()`, and copying that line is the bug the 2025-12-30 test catches.
- The year check in `Month::new` cannot fire from `parse_month`, because four ASCII digits and a real clock always land in `0..=9999`. It guards the type's invariant for direct callers, and a unit test covers it.
- How inputs fall through the checks:

| Input | Path | Cause |
|---|---|---|
| `2026-03`, `2026-3`, `3`, `03` | accepted | |
| `-1`, `-d` | `split_once` gives an empty year | Shape |
| `26-03` | year length 2 | Shape |
| `2026-003` | month length 3 | Shape |
| `2026-` | empty month | Shape |
| `abc`, `` | bare, non-digit or empty | Shape |
| `2026-1-1` | month part `1-1` has a non-digit | Shape |
| `0`, `13`, `2026-00`, `2026-13` | `Month::new` | OutOfRange |

Cognitive complexity matches `parse_week_id`, well under 15.

### 2.5 `date.rs` unit tests (in the existing `mod tests`)

Add a helper `fn mo(y: i32, m: u32) -> Month { Month::new(y, m, "test").expect("test fixture is a real month") }` and `fn month_err(input, cause)` beside `week_err`. Add a section `// --- MONTH (export-notes spec §3, §6) ---` with:

- `parse_month_accepts_all_four_equivalent_forms`. Under `TODAY()` (2026-02-12), `"2026-03"`, `"2026-3"`, `"3"` and `"03"` all equal `mo(2026, 3)`.
- `bare_month_defaults_to_todays_calendar_year_not_iso_year`. `parse_month("1", d(2025, 12, 30))` is `mo(2025, 1)`, and `parse_month("12", d(2027, 1, 2))` is `mo(2027, 12)`. Both dates sit in an ISO year that differs from the calendar year.
- `full_month_ignores_today`. `"2026-03"` under three different todays.
- `parse_month_accepts_any_four_digit_year`. `"0000-01"` is `mo(0, 1)` and `"9999-12"` is `mo(9999, 12)`.
- `parse_month_rejects_wrong_shapes`. Each of `"2026-003"`, `"26-03"`, `"abc"`, `"2026-"`, `"-1"`, `"-d"`, `""`, `" 3 "`, `"+1"`, `"2026-1-1"`, `"2026/03"`, `"123"`, `"999-03"` gives `month_err(input, Cause::Shape)`.
- `parse_month_rejects_month_numbers_outside_1_to_12`. Each of `"0"`, `"00"`, `"13"`, `"2026-00"`, `"2026-13"`, `"99"` gives `month_err(input, Cause::OutOfRange)`.
- `month_new_rejects_a_year_outside_the_domain`. `Month::new(10000, 1, "x")` and `Month::new(-1, 1, "x")` give `OutOfRange`.
- `month_span_follows_the_calendar`. Expected values come from spec §7's month edges:

| Month | span |
|---|---|
| `mo(2024, 2)` | `(d(2024,2,1), d(2024,2,29))` |
| `mo(2026, 2)` | `(d(2026,2,1), d(2026,2,28))` |
| `mo(2025, 12)` | `(d(2025,12,1), d(2025,12,31))` |
| `mo(2000, 2)` | `(.., d(2000,2,29))` |
| `mo(2100, 2)` | `(.., d(2100,2,28))` |
| `mo(2026, 4)` | `(.., d(2026,4,30))` |
| `mo(0, 1)` | `(d(0,1,1), d(0,1,31))` |
| `mo(9999, 12)` | `(d(9999,12,1), d(9999,12,31))` |

- `month_error_messages_match_the_spec`. Exact `to_string()`:
  - `parse_month("abc", TODAY())` gives `invalid MONTH "abc": expected YYYY-MM or MM`
  - `parse_month("-1", TODAY())` gives `invalid MONTH "-1": expected YYYY-MM or MM`
  - `parse_month("13", TODAY())` gives `invalid MONTH "13": month must be 1-12`
  - `parse_month("2026-13", TODAY())` gives `invalid MONTH "2026-13": month must be 1-12`
  - and the WEEK_ID out-of-range message is unchanged, so `parse_week_id("0", TODAY())` still gives `invalid WEEK_ID "0": week number must be 1 or greater`
- Extend `junk_input_never_panics` (line 1010) with `assert!(parse_month(input, TODAY()).is_err(), "month {input:?}");`. Every junk input there is rejected by the shape check.

## 3. `src/cli.rs` changes

### 3.1 `Command` gains `Export` (after `Status`, line 104)

```rust
    /// Print stored data as plain text
    #[command(visible_alias = "ex")]
    Export(ExportArgs),
```

Placing it after `Status` lists it last before clap's `help` in `mlm --help`. No doc comment here or below ends in a period, so clap's trailing-period stripping has nothing to change.

### 3.2 New section after `WeekTargetArgs` (after line 285, before `command()`)

```rust
// ---------------------------------------------------------------------------
// `export notes`
// ---------------------------------------------------------------------------

/// `mlm export notes [--date DATE | --week WEEK_ID | --month MONTH]`. A
/// required subcommand, as `delete` has, so bare `mlm export` is a usage
/// error that prints this command's help.
#[derive(Args, Debug)]
pub struct ExportArgs {
    #[command(subcommand)]
    pub target: ExportTarget,
}

/// What `export` prints.
#[derive(Subcommand, Debug)]
pub enum ExportTarget {
    /// Print notes for a date, week or month, one per line (defaults to
    /// the current week)
    #[command(visible_alias = "n")]
    Notes(ExportNotesArgs),
}

/// At most one range flag (the group). Each takes hyphen values, so `-1`
/// reaches its parser, and a following flag becomes this flag's value.
#[derive(Args, Debug)]
#[group(multiple = false)]
pub struct ExportNotesArgs {
    /// Date to export: YYYY-MM-DD, or `-N` for N days before today (e.g.
    /// `-1` = yesterday)
    #[arg(short, long, value_name = "DATE", allow_hyphen_values = true)]
    pub date: Option<String>,

    /// Week to export: YYYY-WW (e.g. 2026-07) or a bare week number (e.g. 7)
    #[arg(short, long, value_name = "WEEK_ID", allow_hyphen_values = true)]
    pub week: Option<String>,

    /// Month to export: YYYY-MM (e.g. 2026-03) or a bare month number (e.g.
    /// 3)
    #[arg(short, long, value_name = "MONTH", allow_hyphen_values = true)]
    pub month: Option<String>,
}
```

The prototype rendered these as contract 3 requires (section 6.13 holds the golden). Struct-level `///` docs do not reach any help text (1.7).

### 3.3 `WeekArgs` doc re-wrap (lines 226-229, spec "Housekeeping")

Replace with this text, whose lines run 75, 69, 76 and 78 columns. The words are unchanged.

```rust
/// `mlm week [WEEK_ID] [--notes]` and `mlm week target [WEEK_ID] DURATION`
/// share this struct: a first token matching a known subcommand name
/// (`target`) is taken as the subcommand, otherwise it falls through to the
/// positional `WEEK_ID` (no week id can ever be the literal string `target`).
```

Clap joins doc lines with spaces, and the variant doc supplies `week`'s about text, so `WEEK_SHORT` stays byte-identical.

### 3.4 No change

`command()`, `ParseFailure`, `parse`, `classify`, `rename_args_context` and `swapped_week_target` stay as they are.

### 3.5 `cli.rs` unit tests (in the existing `mod tests`)

Add:

```rust
    // --- export notes: cli surface ---------------------------------------

    fn export_notes_args(cli: Cli) -> ExportNotesArgs {
        let Command::Export(ExportArgs {
            target: ExportTarget::Notes(a),
        }) = cli.command
        else {
            panic!("expected export notes");
        };
        a
    }
```

Tests:

- `export_aliases_parse_like_the_long_form`. `["mlm", "ex", "n", "-w", "43"]` gives `week == Some("43")` with the other two `None`. `["mlm", "n", "hi"]` is still `Command::Note`.
- `export_range_flags_take_hyphen_values`. `-d -1`, `-w -1`, `-m -1` and `--month=3` each land in their field, and `-m -d` gives `month == Some("-d")`, `date == None`.
- `export_with_no_flag_has_no_range`. All three fields are `None`.
- `export_usage_errors_are_clap_errors`. Through `super::parse(&os(..))`, each of `["mlm","export"]`, `["mlm","export","notes","-d","2026-01-05","-w","2026-02"]`, `["mlm","export","notes","-m","3","-m","4"]` and `["mlm","export","notes","-d"]` is `ParseFailure::Clap` with `exit_code() == 2`. The kinds are `DisplayHelpOnMissingArgumentOrSubcommand`, `ArgumentConflict`, `ArgumentConflict` and `InvalidValue`.

`command_passes_debug_assert` already covers the group and aliases.

## 4. `src/export.rs` (new)

```rust
//! `mlm export notes [--date DATE | --week WEEK_ID | --month MONTH]`
//! (SPEC.md §3.8, §7.5).
//!
//! Resolves one date, ISO week or calendar month (the current week by
//! default), makes one ranged note read and prints one line per note as
//! `YYYY-MM-DD`, two spaces, then the body. No range is checked against
//! today, so a future range prints nothing.

use chrono::NaiveDate;
use rusqlite::Connection;

use crate::cli::ExportNotesArgs;
use crate::date::{DateWeekError, WeekId, format_date, parse_month, parse_week_id, resolve_date};
use crate::storage::{self, Note};

/// The inclusive `(first, last)` dates the flags name. Clap's group
/// guarantees at most one flag is set.
pub fn resolve_range(
    args: &ExportNotesArgs,
    today: NaiveDate,
) -> Result<(NaiveDate, NaiveDate), DateWeekError> {
    if let Some(s) = &args.date {
        let date = resolve_date(s, today)?;
        return Ok((date, date));
    }
    if let Some(s) = &args.week {
        return Ok(parse_week_id(s, today)?.span());
    }
    if let Some(s) = &args.month {
        return Ok(parse_month(s, today)?.span());
    }
    Ok(WeekId::current(today).span())
}

/// One `YYYY-MM-DD  body\n` line per note, in the order given.
pub fn render_lines(notes: &[Note]) -> String {
    notes
        .iter()
        .map(|n| format!("{}  {}\n", format_date(n.date), n.body))
        .collect()
}

/// `mlm export notes`. A parse or read error returns before anything
/// prints, so stdout stays empty.
pub fn run_notes(conn: &Connection, today: NaiveDate, args: &ExportNotesArgs) -> anyhow::Result<()> {
    let (from, to) = resolve_range(args, today)?;
    let notes = storage::notes_in_range(conn, from, to)?;
    print!("{}", render_lines(&notes));
    Ok(())
}
```

- `resolve_date` is the core resolver, the one without the future check (`date.rs:126`). Do not use `resolve_future_checked_date`.
- The storage order (date, then `created_at_utc ASC, id ASC`) is the output order. Nothing re-sorts.
- `cargo fmt` decides the final wrapping of the `use` line and the `run_notes` signature.

### 4.1 `export.rs` unit tests

`#[cfg(test)] mod tests` with helpers `d(y, m, day)` and `fn args(date: Option<&str>, week: Option<&str>, month: Option<&str>) -> ExportNotesArgs`. Fixed today values keep these off the clock.

- `default_range_is_the_current_iso_week`. Today 2026-10-24 (Sat) gives `(d(2026,10,19), d(2026,10,25))`, spec §5's week 2026-43.
- `date_range_is_one_day_and_allows_minus_n_and_the_future`. With today 2026-10-24, `-1` gives `(d(2026,10,23), d(2026,10,23))`, `"2026-10-19"` gives that day twice, and `"2099-06-15"` resolves without error.
- `week_range_is_monday_to_sunday`. `"2026-43"` and bare `"43"` (today 2026-10-24) both give `(d(2026,10,19), d(2026,10,25))`.
- `bare_week_and_bare_month_use_different_years`. Today 2025-12-30, `-w 1` gives `(d(2025,12,29), d(2026,1,4))` (ISO 2026-W01), and `-m 1` gives `(d(2025,1,1), d(2025,1,31))` (calendar 2025). This is the changeset plan's named 2025-12-30 check.
- `month_range_is_first_to_last_day`. `"2026-3"` and `"2026-03"` both give `(d(2026,3,1), d(2026,3,31))`.
- `range_errors_carry_their_arg_kind`. `-m abc` gives `ArgKind::Month`/`Shape`, `-w -1` gives `ArgKind::WeekId`/`Shape`, `-d abc` gives `ArgKind::Date`/`Shape`.
- `render_lines_formats_date_two_spaces_body`. Two `Note`s give exactly `"2026-10-19  fixed migration runner bug\n2026-10-21  code review for #41\n"`, and `&[]` gives `""`. `Note` has public fields, so build it with a fixed `created_at_utc`.
- `run_notes_fails_on_a_broken_note_read`. In-memory DB with migrations applied (as `week_view.rs:1134`), `ALTER TABLE notes RENAME COLUMN body TO body_gone`, then `run_notes(&conn, d(2026,1,7), &args(None, Some("2026-02"), None))` is `Err` and its text starts with `database error:`.
- `run_notes_succeeds_on_an_empty_range`. Fresh in-memory DB gives `Ok(())`.

## 5. `src/lib.rs` and `src/main.rs`

### 5.1 `src/lib.rs`

Insert `pub mod export;` between `pub mod db;` (line 12) and `pub mod history;` (line 13).

### 5.2 `src/main.rs`

- Line 35 becomes `use mlm::cli::{self, Cli, Command, DeleteTarget, ExportTarget, WeekAction};`.
- Line 36 becomes `use mlm::{commands, db, export, status, week_target, week_view};`.
- In `dispatch`, after the `Status` arm (line 70), add:

```rust
        Command::Export(args) => match &args.target {
            ExportTarget::Notes(a) => export::run_notes(&conn, now.date_naive(), a),
        },
```

No other change. Errors flow through the existing `exit_code` (line 77), which prints `error: {e}` and exits `1`.

## 6. Test plan (`tests/export_notes_cli.rs`)

### 6.0 File skeleton

```rust
//! Binary-level tests for `mlm export notes` (export-notes spec §3-§7).
//!
//! Drives the compiled binary with `MLM_DB_PATH` and `TZ=UTC`, seeding
//! notes by raw insert. Fixed past and far-future ranges keep most cases
//! off the clock. Cases about today derive it from `Utc::now()` and rerun
//! when the date rolls over mid-test.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use chrono::{Datelike, Days, NaiveDate, Utc};
use mlm::date::{WeekId, format_date};
use rusqlite::Connection;
use tempfile::TempDir;
```

Helpers, copied in shape from `tests/week_notes_cli.rs:16-63`:

- `mlm(db, args) -> Output`, with `MLM_DB_PATH`, `TZ=UTC` and `.env_remove("RUST_LOG")` (as `week_target_cli.rs:17`)
- `stdout(&Output)`, `stderr(&Output)`
- `fresh() -> (TempDir, PathBuf)` via `mlm::db::connect_at`
- `seed_note(db, date, body, created_at_utc)`, the raw insert
- `ok_stdout(db, args) -> String`, which asserts exit `0` and empty stderr
- `fails(db, args, code, want_stderr)`, which asserts that exit code, `stdout == ""` and `stderr == want_stderr`
- `usage_error(db, args) -> String`, which asserts exit `2` and empty stdout and returns stderr
- `line(date: NaiveDate, body: &str) -> String`, giving `format!("{}  {body}\n", format_date(date))`
- `on_stable_today(f: impl Fn(NaiveDate))`, a loop that reads `Utc::now().date_naive()`, runs `f(today)` and returns if the date is unchanged afterwards, otherwise runs again. Each `f` builds its own fresh DB, so a rerun starts clean.

Week fixture `seeded_week()`, ISO week 2026-02 (2026-01-05 to 2026-01-11), inserted in this order so `id` order disagrees with date order and with `created_at_utc` on Monday:

| id | date | body | created_at_utc |
|---|---|---|---|
| 1 | 2026-01-07 | `code review for #41` | 2026-01-07T15:05:00Z |
| 2 | 2026-01-05 | `started punch pairing tests` | 2026-01-05T15:00:00Z |
| 3 | 2026-01-05 | `fixed migration runner bug` | 2026-01-05T10:00:00Z |
| 4 | 2026-01-11 | `sunday wrap-up` | 2026-01-11T18:00:00Z |
| 5 | 2026-01-04 | `sunday before` | 2026-01-04T09:00:00Z |
| 6 | 2026-01-12 | `monday after` | 2026-01-12T09:00:00Z |
| 7 | 2026-01-06 | `tuesday standup` | 2026-01-06T09:00:00Z |

```rust
const WEEK_2026_02: &str = "\
2026-01-05  fixed migration runner bug
2026-01-05  started punch pairing tests
2026-01-06  tuesday standup
2026-01-07  code review for #41
2026-01-11  sunday wrap-up
";
```

Month fixture `seeded_months()`, one note on each date, body = `"m <date>"`, created at noon UTC of that date:

- Feb 2024: `2024-01-31`, `2024-02-01`, `2024-02-29`, `2024-03-01`
- Feb 2026: `2026-01-31`, `2026-02-01`, `2026-02-28`, `2026-03-01`
- Dec 2025: `2025-11-30`, `2025-12-01`, `2025-12-31`, `2026-01-01`

### 6.1 Each range prints its notes and never the dates just outside (spec §7 "day, week and month ranges")

- `week_range_prints_monday_to_sunday`. `export notes --week 2026-02` equals `WEEK_2026_02`. It holds both edge days (05, 11) and neither neighbour (04, 12).
- `date_range_prints_one_day`. `--date 2026-01-05` equals the first two lines of `WEEK_2026_02`. The neighbours 2026-01-04 and 2026-01-06 hold notes and do not appear. `-d 2026-01-04` prints `2026-01-04  sunday before\n`, which proves the edge note is real.
- `month_range_prints_first_to_last_day`. On `seeded_months()`, `--month 2026-02` equals `m 2026-02-01` and `m 2026-02-28` lines only, with `2026-01-31` and `2026-03-01` absent.

### 6.2 No range flag prints the current week (spec §7 "default range")

`default_range_is_the_current_week`, inside `on_stable_today`:

- `week = WeekId::current(today)`, `(mon, sun) = week.span()`
- seed `mon - 1 day` `"before"`, `mon` `"first"`, `sun` `"last"`, `sun + 1 day` `"after"`
- `export notes` equals `line(mon, "first") + line(sun, "last")`
- `ex n` gives the same (also counts toward 6.12)

### 6.3 Order across and within dates (spec §4, §7 "order")

`lines_run_by_date_then_insertion_order`. `WEEK_2026_02` already checks both. The 2026-01-07 note has the lowest `id` but prints after 2026-01-05 and 2026-01-06. On 2026-01-05, `id` 3 (earlier `created_at_utc`) prints before `id` 2. An `id`-only sort would put `started punch pairing tests` first and fail. The test asserts the full string, plus `out.find("fixed") < out.find("started")` with a message naming the rule.

### 6.4 `-d -1`, `-w -1`, `-m -1` (spec §3 hyphen values, §7)

- `minus_one_date_is_yesterday`, inside `on_stable_today`. Seed `today - 2`, `today - 1` (`"yesterday"`), `today`. `export notes -d -1` equals `line(today - 1, "yesterday")`.
- `minus_one_week_and_month_are_value_errors`, on `fresh()`:
  - `-w -1`: exit `1`, stdout empty, stderr `error: invalid WEEK_ID "-1": expected YYYY-WW or WW\n`
  - `-m -1`: exit `1`, stdout empty, stderr `error: invalid MONTH "-1": expected YYYY-MM or MM\n`

### 6.5 Bare `WW`, bare `M`/`MM`, `YYYY-M` (spec §3, §7)

- `bare_week_uses_the_current_iso_year`, inside `on_stable_today`. Same seed as 6.2. `-w {week.week()}` equals the 6.2 expected output. The calendar-vs-ISO-year difference is unit-tested in 4.1, because a CLI test cannot choose today.
- `bare_month_uses_the_current_calendar_year`, inside `on_stable_today`. Compute `first = today.with_day(1)` and `last` as the day before the next month's 1st, independent of `mlm::date::Month`. Seed `first - 1`, `first`, `last`, `last + 1`. `-m {today.month()}` and `-m {today.month():02}` both equal `line(first, ..) + line(last, ..)`.
- `unpadded_year_month_equals_padded`. On `seeded_months()`, `-m 2026-2` equals `-m 2026-02`, and is non-empty.

### 6.6 Month edges (spec §7 "month edges")

`month_edges_follow_the_calendar`, on `seeded_months()`:

| Args | stdout |
|---|---|
| `-m 2024-02` | `2024-02-01  m 2024-02-01\n2024-02-29  m 2024-02-29\n` |
| `-m 2026-02` | `2026-02-01  m 2026-02-01\n2026-02-28  m 2026-02-28\n` |
| `-m 2025-12` | `2025-12-01  m 2025-12-01\n2025-12-31  m 2025-12-31\n` |

### 6.7 Empty range (spec §5, §7 "empty range")

`empty_range_prints_nothing`. On `seeded_week()`, `-w 2026-05`, `-d 2026-01-08` and `-m 2025-06` each exit `0` with empty stdout and stderr. On `fresh()`, bare `export notes` does the same.

### 6.8 Future ranges (spec §3, §7 "future")

`future_ranges_print_nothing`. On `seeded_week()`, `-d 2099-06-15`, `-w 2099-10` and `-m 2099-06` each exit `0` with empty stdout and stderr. Fixed far-future values keep this off the clock until 2099.

### 6.9 Contract 7 messages (spec §6)

`value_errors_print_their_exact_message`, on `fresh()`. Each row exits `1` with empty stdout and this exact stderr:

| Args | stderr |
|---|---|
| `-d abc` | `error: invalid DATE "abc": expected YYYY-MM-DD or -N (N >= 1)\n` |
| `-d 2026-02-30` | `error: invalid DATE "2026-02-30": not a real calendar date\n` |
| `-w 2026-54` | `error: invalid WEEK_ID "2026-54": 2026 has only 53 ISO weeks\n` |
| `-w abc` | `error: invalid WEEK_ID "abc": expected YYYY-WW or WW\n` |
| `-m 2026-003` | `error: invalid MONTH "2026-003": expected YYYY-MM or MM\n` |
| `-m 26-03` | `error: invalid MONTH "26-03": expected YYYY-MM or MM\n` |
| `-m abc` | `error: invalid MONTH "abc": expected YYYY-MM or MM\n` |
| `-m 2026-` | `error: invalid MONTH "2026-": expected YYYY-MM or MM\n` |
| `-m -d` | `error: invalid MONTH "-d": expected YYYY-MM or MM\n` |
| `-m 0` | `error: invalid MONTH "0": month must be 1-12\n` |
| `-m 13` | `error: invalid MONTH "13": month must be 1-12\n` |
| `-m 2026-13` | `error: invalid MONTH "2026-13": month must be 1-12\n` |

The `-m -d` row is contract 8's "flag followed by another flag" case.

### 6.10 Failed note read (spec §6, SPEC §6.1)

`renamed_note_body_is_an_error_not_an_empty_export`. On `seeded_week()`, rename `notes.body` as `week_notes_cli.rs:277-280` does. Then `export notes`, `export notes -w 2026-02` and `ex n -m 2026-01` each exit `1`, stdout empty, and stderr starting with `error: database error: ` and ending with `\n`. The rusqlite text after that prefix is not pinned.

### 6.11 Contract 8 usage errors (spec §6 clap rows)

`usage_errors_exit_two`, on `fresh()`. Each exits `2` with empty stdout:

- `export` and `ex`. Stderr contains `notes` and `Usage:`.
- `export notes -d 2026-01-05 -w 2026-02`
- `export notes -w 2026-02 -m 2026-01`
- `export notes -d 2026-01-05 -w 2026-02 -m 2026-01`
- `export notes -m 3 -m 4`
- `export notes -d`, `export notes -w`, `export notes -m` (no value, last token)

### 6.12 Aliases (spec §3, §7 "aliases")

`aliases_behave_like_the_long_form`. On `seeded_week()`, `ex n -w 2026-02`, `ex notes -w 2026-02`, `export n -w 2026-02` and `export notes --week 2026-02` all equal `WEEK_2026_02`. `-d`/`--date` and `-m`/`--month` long forms equal their short forms for one fixture value each.

### 6.13 Non-ASCII passthrough and ASCII elsewhere (spec §5, §7)

`non_ascii_body_passes_through_and_the_rest_is_ascii`. On `seeded_week()`, add `seed_note(db, "2026-01-08", NON_ASCII, "2026-01-08T09:00:00Z")` with `const NON_ASCII: &str = "caf\u{e9} review \u{2013} done";`. Then:

- `-w 2026-02` contains `format!("2026-01-08  {NON_ASCII}\n")`
- after `out.replace(NON_ASCII, "")`, every byte is `\n` or `0x20..=0x7E`
- the stderr of one 6.9 case and of one 6.11 case, and every help output in 6.14, is ASCII

### 6.14 Help (spec §3, §8; contracts 2, 3)

`help_lines_match_the_contract`, on `fresh()`, with `.replace("mlm.exe ", "mlm ")` as `week_target_cli.rs:240` does:

- `export notes --help` and `export notes -h` both equal this golden. The prototype printed it, with the binary name swapped in.

```rust
const EXPORT_NOTES_HELP: &str = "\
Print notes for a date, week or month, one per line (defaults to the current week)

Usage: mlm export notes [OPTIONS]

Options:
  -d, --date <DATE>     Date to export: YYYY-MM-DD, or `-N` for N days before today (e.g. `-1` = yesterday)
  -w, --week <WEEK_ID>  Week to export: YYYY-WW (e.g. 2026-07) or a bare week number (e.g. 7)
  -m, --month <MONTH>   Month to export: YYYY-MM (e.g. 2026-03) or a bare month number (e.g. 3)
  -v, --verbose         Enable debug-level logging
  -h, --help            Print help
";
```

- `--help` contains `"\n  export  Print stored data as plain text [alias: ex]\n"`
- `export --help` contains `"\n  notes  Print notes for a date, week or month, one per line (defaults to the current week) [alias: n]\n"`

### 6.15 Existing tests unchanged (changeset plan last criterion)

No existing test file is edited. `cargo test` must pass as is. In particular, `short_help_goldens` (`STATUS_SHORT`, `WEEK_SHORT`) proves the `WeekArgs` re-wrap and the new command leave `status -h` and `week -h` byte-identical. `long_help_carries_glossary` and `help_and_version_exit_zero` cover `--help` still exiting `0`.

## 7. Verification

- `cargo fmt`, `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings`
- `cargo clippy --all-targets -- -W clippy::cognitive_complexity` shows no hit. `parse_month` and `resolve_range` are the largest new functions, both far under 15.
- Smoke runs:
  - `MLM_DB_PATH=/tmp/mlm-check.db cargo run -- note "export smoke"`
  - `MLM_DB_PATH=/tmp/mlm-check.db cargo run -- export notes`
  - `MLM_DB_PATH=/tmp/mlm-check.db cargo run -- ex n -d -1`
- Commit through the hook so `coverage-baseline.json` ratchets. Every new branch has a test (each MONTH cause, each flag, the default, the read failure, the `Month::new` year guard).

## 8. Risks, ambiguities, disagreements

- **Bare `mlm export` output.** Clap prints the full `export` help on stderr with exit `2` (`DisplayHelpOnMissingArgumentOrSubcommand`), which satisfies "exit 2, lists `notes`". The tests assert those two facts, not the text, so a clap upgrade that switches to a short usage error still passes.
- **MONTH year domain.** Any four-digit year is accepted, `0000` included, per contract 6. WEEK_ID's `1000..=9999` floor stays untouched. `Month::new`'s year guard reuses the `month must be 1-12` message, which no CLI input can reach.
- **`-d -N` versus `-d YYYY-MM-DD` year floor.** `resolve_date` rejects a `-N` that lands before year 1000 but accepts `0000-01-01` written out. That is existing DATE behaviour, and export inherits it unchanged.
- **Midnight rollover.** Four CLI cases depend on today (6.2, 6.4 first case, 6.5 first two). `on_stable_today` reruns a case whose date changed during the run, and fixed-today unit tests in 4.1 carry the boundary logic. Neither depends on the local zone, since the binary runs under `TZ=UTC`.
- **`tests/ascii_sweep.rs` is not extended.** No task owns it, and 6.13 sweeps export's own output. Adding export to the cross-command sweep would need a coordinator decision.
- **Help golden.** 6.14 pins the full `export notes --help` text, alignment included, which goes beyond contract 3's "contains each line". This matches the existing `status -h`/`week -h` goldens. If the coordinator wants a lighter pin, line-containment checks of the four contract strings suffice.
- **No disagreement** with the spec or the changeset plan. Task 2's plan quotes the same contract strings and the same `src/export.rs` role.
