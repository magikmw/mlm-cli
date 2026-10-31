# Task 1 report: code and tests (`mlm week --notes`)

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

Changeset: week-notes. Branch `week-notes-task-1`, rebased onto `week-notes` @ `4c43b14` (the worktree started on `main`).

## Implemented

- `src/cli.rs`
  - `WeekArgs.notes: bool` (`-n`, `--notes`)
  - `- <note>` glossary entry, third before `[!], [i]`, text as pinned in plan step 1.2
  - the three unit tests from step 1.4
- `src/storage.rs`: `notes_in_range` and the six tests from step 2.2
- `src/week_view.rs`
  - `WeekRow.notes`, `NOTE_PREFIX`, `push_row_notes`, `attach_notes`, `with_notes`, `run` wiring
  - module, `build_week_view` and `run` docs
  - the eleven tests from step 3.8
- `src/history.rs`: module doc sentence on the `--notes` read
- `tests/week_notes_cli.rs`: new, the twelve CLI tests from step 5
- `tests/week_target_cli.rs`: `WEEK_SHORT` gains the `-n, --notes` line (F1, granted by the dispatch)

## Test order

| Step | First failure |
|---|---|
| cli | `notes` field missing on `WeekArgs` (E0026/E0609). After the field, `glossary_has_the_note_line_entry_before_markers` failed until the entry went in. |
| storage | `notes_in_range` not found (E0425) |
| week_view | `notes` field, `NOTE_PREFIX`, `attach_notes`, `with_notes` missing (24 compile errors) |
| `WEEK_SHORT` | `short_help_goldens` failed once the flag existed. The binary's output matched the plan's derived block byte for byte. |
| CLI file | Written after the lib code, as the plan sequences it. 11 of 12 passed first run. `help_lists_the_flag_and_the_glossary_entry` failed (below). |

## Deviations

- **Long-help golden.** The plan asserted `mlm week --help` contains `  -n, --notes    Show each date's notes under its row\n`. Clap's long help puts the description on its own line, so the binary prints `  -n, --notes\n          Show each date's notes under its row\n`. The test now asserts the real layout. The `-h` assertion keeps the one-line form, which matches.
- **`with_notes` extraction.** Plan step 3.8 offered two forms for test 11. I took the extraction: `fn with_notes(conn: &Connection, view: &mut WeekView, week: WeekId) -> anyhow::Result<()>`, private, called by `run` and by the test.
- **Test helper `clear_notes`.** Tests 2 and 5 need "the same view with notes cleared". A small `clear_notes(&WeekView) -> WeekView` test helper builds it.
- **cli test helpers share one panic arm.** The plan's `week_view_args` added an uncovered panic line, and the coverage ratchet failed (99.45719% to 99.45704%). Now `week_args` holds the only `Command::Week` match. `week_target_args` destructures the single `WeekAction::Target` variant on top of it, and `week_view_args` asserts `action.is_none()`. The other new uncovered line is the `?` after `query_map` in `notes_in_range`, which no test can reach (`punches_in_range` has the same gap).
- **Test 9 extra assert.** `attach_notes_by_date_in_order_and_drops_outside` also asserts Tue-Sat stay empty.

All derived CLI goldens (`NOTES_2026_02`, `CARRY_NOTES_2026_02`, the adjacent-week rows) matched the binary unchanged.

## Public signatures

```rust
// src/storage.rs
pub fn notes_in_range(
    conn: &Connection,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<Note>, StorageError>

// src/cli.rs, WeekArgs
/// Show each date's notes under its row.
#[arg(short, long)]
pub notes: bool,

// src/week_view.rs, WeekRow
pub notes: Vec<String>,
```

`build_week_view` and `run` signatures are unchanged.

## Verification

- `cargo fmt`: clean
- `cargo build`: ok
- `cargo test`: all pass (lib 781, every integration binary green)
- `cargo clippy --all-targets -- -D warnings`: clean
- No `.claude/cd-verify` in the worktree or main checkout

## Open questions for the reviewer

- Task 2's plan, line 540, says to confirm `mlm week --help` prints the help text "on the `-n, --notes` line". That is true for `mlm week -h` only. In `--help` the text sits on the next line, indented ten spaces. Task 2's check should name `-h`, or accept the two-line form.
