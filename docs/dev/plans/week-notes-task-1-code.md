# Task 1 plan: code and tests (`mlm week --notes`)

Changeset: week-notes. Altitude: task plan. No source file is edited by this plan.

## Read

- `docs/dev/plans/week-notes-plan.md` (changeset plan, all)
- `docs/dev/specs/2026-10-24-week-notes.md` (spec, all). Coordinator update applied: spec §1.2 is now "None", so this plan cites no known issue. The behavior it described is unchanged and is cited from spec §3 instead.
- `docs/dev/plans/week-notes-task-2-docs.md` (sibling docs plan, all)
- `src/cli.rs` (all 900 lines)
- `src/storage.rs` (lines 1-601 in full, tests 1109-1185 and 1349-1622; the rest is punch/DST tests)
- `src/week_view.rs` (all 1770 lines)
- `src/history.rs` (all 557 lines)
- `src/render.rs` (all 822 lines, read only)
- `src/status.rs` (lines 1-700 in full, and the `week_view::build_week_view` callers at 3085-3095, 3275-3300, 3375-3392; read only)
- `src/lookback.rs:1000-1020` (the other `build_week_view` test caller; read only)
- `src/main.rs` (all, read only)
- `src/carry_scan.rs:1-65` (marker semantics for the CLI fixture)
- `src/db.rs` (schema lines 44-75)
- `tests/week_read_errors.rs`, `tests/ascii_sweep.rs` (all, read only)
- `tests/week_target_cli.rs` (lines 1-40, 140-300, 355-410: help goldens)
- `tests/carry_marker_cli.rs:1-60`
- `clippy.toml`, `.githooks/pre-commit`, `Cargo.toml`, `Cargo.lock` (clap versions)
- `docs/dev/SPEC.md:313-323` (§3.5 glossary sentence)

A clap prototype in `/tmp/cd-scratch/week-notes-task-1/proto` (clap 4.6.6, builder 4.6.6, derive 4.6.4, the lockfile's versions) confirmed the parse and help behavior quoted below.

## Scope

| Owned by this task | Not to touch, and its owner |
|---|---|
| `src/cli.rs` | `docs/dev/SPEC.md`, `README.md`, `AGENTS.md`: Task 2 |
| `src/storage.rs` | `src/render.rs`, `src/status.rs`, `src/main.rs`, `src/lookback.rs`: read only, no owner change |
| `src/week_view.rs` | `tests/week_read_errors.rs`, `tests/ascii_sweep.rs`: left alone |
| `src/history.rs` (module doc only) | `tests/week_target_cli.rs`: not owned, but its `WEEK_SHORT` golden breaks (finding F1) |
| `tests/week_notes_cli.rs` (new) | `docs/dev/NOTES.md`, plans, reports: coordinator |

## Dependencies and handoffs

| Dependency | State | Used here | Swap at integration |
|---|---|---|---|
| none | | Task 2 is docs only and shares no code | none |

- **To Task 2.** The flag help line is `Show each date's notes under its row` (contract 2). The glossary entry text and slot are pinned in step 1.2. It sits third of three entries before `[!], [i]`, so Task 2's S2 variant A applies, including its shared-glossary sentence, because `status --help` shows the entry too.
- **To coordinator (F1).** `tests/week_target_cli.rs:218-234` (`WEEK_SHORT`) is an exact `mlm week -h` golden. The new flag adds one Options line, so `short_help_goldens` fails unless that constant gains it. The edit is one line (step 6). It needs a coordinator decision because the file is outside this task's ownership.

## Facts gathered

### `src/cli.rs`

- `GLOSSARY` constant, lines 9-57. Entries in order are worked, carry-in, fulfillment, left to / over, est. EOD, (incl. ongoing)/(+ ongoing), Total behind/ahead, overlapping stints, orphaned end, so far started yesterday (44-47), spans to next day (48-51), `[!], [i]` (52-57, last).
- Format: term at 2 spaces, body at 6 spaces, longest body line 74 columns. No trailing newline (asserted at line 787).
- `command()` at 280-284 attaches `GLOSSARY` to both `status` and `week` through `with_glossary` (286-288). Any entry shows in both long helps.
- `WeekArgs`, lines 222-236, has fields `action: Option<WeekAction>` and `week_id: Option<String>`, with `#[command(args_conflicts_with_subcommands = true)]`.
- `WeekTargetArgs.args` (268-277) is `num_args = 1..=2`, `allow_hyphen_values = true`.
- `swapped_week_target` (366-383) skips only `-v`/`--verbose`. It does not need to change.
- The test `week_positional_still_works` (466-476) destructures `WeekArgs { action: None, week_id: Some(id) }` with no `..`, so it breaks when a field is added.
- `glossary_is_ascii_and_ends_cleanly` (776-788) asserts `GLOSSARY.ends_with("stopped.")`.

### `tests/week_target_cli.rs` (not owned)

- `long_help_carries_glossary` (164-198) asserts the flattened long help ends with ``It marks yesterday's row in `mlm week` until that start is stopped.`` for both `status` and `week`. The new entry must not be last.
- `WEEK_SHORT` (218-234) and `short_help_goldens` (236-245) pin `mlm week -h` byte for byte.

### `src/storage.rs`

- `#![allow(dead_code)]` at line 12.
- `Note { id, date, body, created_at_utc }` at 116-132 derives `Debug, Clone, PartialEq, Eq`.
- `map_note_row` (444-446) and `note_from_row` (448-460) are the shared row helpers. `note_from_row` maps a bad `date`/`created_at_utc` to `CorruptRow`.
- `notes_for_date` (462-477) uses `ORDER BY created_at_utc ASC, id ASC`.
- `punches_in_range` (395-421) is the ranged-read pattern, with `WHERE "date" BETWEEN ?1 AND ?2` and dates bound as `%Y-%m-%d` text.
- `earliest_data_date` (500-517) reads `notes."date"` only, never `body`.
- Test helpers: `test_db()` 554, `d()` 560, `utc()` 568. Note-order tests N8-N12 are at 1109-1185, range tests at 1477-1517.

### `src/week_view.rs`

- Module doc lines 1-18. Lines 17-18 say accounting comes from `history::load_history` and a failed read is an error.
- Imports at 20-30. `crate::storage` is not imported outside tests.
- `WeekRow` (42-57) and `WeekView` (63-82) derive `Debug, Clone, PartialEq, Eq`. Struct literals of either exist only in this file: `build_rows_from` 208-216, test helper `row()` 311-320, and `WeekView` literals in tests built from `row()`.
- `push_rows` (105-121) writes `"  " + abbrev + " " + date + "   " + minutes`, then ` (incl. ongoing)`, then `row_markers(row)`, then `\n`. One blank line follows the seven rows.
- `build_week_view` (232-262) is documented "Every storage read goes through `history::load_history`". Its test callers are `src/status.rs` (3091, 3282, 3296, 3384-3385) and `src/lookback.rs:1011`. Those only read fields, so a new `WeekRow` field compiles there unchanged.
- `run` (275-287) parses the week id, calls `build_week_view` and prints `render_week`.
- `WeekArgs` literals in tests are at 1157-1160 (`c4`) and 1231-1234 (`c9`).
- Test helpers include `new_test_db()` 1088, `fixed_now()` 1094 (Thu 2026-02-12 18:00 local), `example_a()` 325 (Thu row `is_ongoing`), `example_b()` 353, `EXAMPLE_A_GOLDEN` 379, `EXAMPLE_B_GOLDEN` 398 and `marker()` 643.

### `src/history.rs`

- Module doc lines 1-7 describe one earliest-date read, one ranged punch read and one ranged target read. The code does not change.

### `src/main.rs`

- Line 68 dispatches `None => week_view::run(&conn, now, args)`. `exit_code` (77-85) prints `error: {e}` and returns 1. No change is needed.

### `src/status.rs`

- `render` (309-313) prints notes as `Notes:` then `  - {body}`. `resolve` (470) reads `notes_for_date`. These are the order and layout the glossary entry describes.

### Clap behavior (prototype, flag added as the last `WeekArgs` field)

| argv after `mlm` | Result |
|---|---|
| `week -n`, `week --notes` | view, `notes: true`, `week_id: None` |
| `week -n 43`, `week 43 -n`, `week 43 --notes`, `w -n 2026-43` | view, `notes: true`, `week_id: Some(..)` |
| `week target 40h --notes` | `Target { args: ["40h", "--notes"] }`, so `week_target::run` fails on WEEK_ID `40h`, exit 1 |
| `week target 43 40h --notes` | clap `TooManyValues`, exit 2 |
| `week target -n` | `Target { args: ["-n"] }`, duration `-n`, exit 1 |
| `week -n target` | view, `week_id: Some("target")`, so `parse_week_id` fails, exit 1 |
| `week 43 -n target 40h` | clap `ArgumentConflict`, exit 2 |
| `week -n -n` | clap `ArgumentConflict`, exit 2 |

`mlm week -h` with the flag:

```
Options:
  -n, --notes    Show each date's notes under its row
  -v, --verbose  Enable debug-level logging
  -h, --help     Print help (see more with '--help')
```

The option column stays at the `-v, --verbose` width, so the `-v` and `-h` lines do not move.

### Lint and gate

- `clippy.toml` sets `cognitive-complexity-threshold = 15`. Every new function below stays under 5.
- The pre-commit hook enforces `cargo fmt --check`, the complexity check and a coverage ratchet. New code paths (ranged read, read failure, attach, render) each get a test below.

## Steps

### 1. `src/cli.rs`

#### 1.1 The flag

Add a third field to `WeekArgs` (after `week_id`, line 235):

```rust
    /// Show each date's notes under its row.
    #[arg(short, long)]
    pub notes: bool,
```

- Long `--notes`, short `-n` (contract 1). It is not `global`, so `WeekTargetArgs` gains nothing.
- Clap strips the trailing period, giving the help text `Show each date's notes under its row` (contract 2).
- Update the doc comment on `WeekArgs` (222-225) to read ``/// `mlm week [WEEK_ID] [--notes]` and `mlm week target [WEEK_ID] DURATION` share this``, with the rest unchanged.

#### 1.2 The glossary entry (contract 2)

Insert between the `spans to next day, ...` entry (ends line 51) and `  [!], [i]` (line 52). It becomes the third of three entries before `[!], [i]`, and `[!], [i]` stays last.

Exact text, inside the `GLOSSARY` literal:

```
  - <note>
      One of a date's notes, in the order they were added. `mlm status`
      lists them under `Notes:`, and `mlm week --notes` under the date's
      row. Notes change no total, marker or footer.
```

- It reads true for both commands. Status lists notes under `Notes:` (`src/status.rs:311-312`), and `week --notes` lists them under rows (contract 3).
- Body lines are 71, 72 and 51 columns, all ASCII.
- The `GLOSSARY` literal still ends with `stopped.`, so `glossary_is_ascii_and_ends_cleanly` and `long_help_carries_glossary` hold unchanged.

#### 1.3 Mechanical test update

`week_positional_still_works` (470-473) changes its pattern to:

```rust
            Command::Week(WeekArgs {
                action: None,
                week_id: Some(id),
                notes: false,
            }) => assert_eq!(id, "2026-07"),
```

#### 1.4 New unit tests (in `mod tests`)

```rust
    fn week_view_args(cli: Cli) -> WeekArgs {
        match cli.command {
            Command::Week(a @ WeekArgs { action: None, .. }) => a,
            other => panic!("expected the week view, got {other:?}"),
        }
    }

    #[test]
    fn notes_flag_parses_long_and_short_in_either_order() {
        for args in [
            &["mlm", "week", "-n", "2026-43"][..],
            &["mlm", "week", "2026-43", "-n"],
            &["mlm", "week", "--notes", "2026-43"],
            &["mlm", "week", "2026-43", "--notes"],
            &["mlm", "w", "-n", "2026-43"],
        ] {
            let a = week_view_args(parse(args).unwrap());
            assert!(a.notes, "{args:?}");
            assert_eq!(a.week_id.as_deref(), Some("2026-43"), "{args:?}");
        }
        let bare = week_view_args(parse(&["mlm", "week", "-n"]).unwrap());
        assert!(bare.notes);
        assert_eq!(bare.week_id, None);
        assert!(!week_view_args(parse(&["mlm", "week"]).unwrap()).notes);
    }

    #[test]
    fn notes_after_target_is_a_target_token() {
        let t = week_target_args(parse(&["mlm", "week", "target", "40h", "--notes"]).unwrap());
        assert_eq!(t.split(), (Some("40h"), "--notes"));
        let err = parse(&["mlm", "week", "target", "43", "40h", "--notes"]).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::TooManyValues);
    }

    #[test]
    fn glossary_has_the_note_line_entry_before_markers() {
        let note = GLOSSARY.find("\n  - <note>\n").expect("note entry");
        let spans = GLOSSARY.find("\n  spans to next day").unwrap();
        let markers = GLOSSARY.find("\n  [!], [i]\n").unwrap();
        assert!(spans < note && note < markers);
        let flat = GLOSSARY.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(flat.contains(
            "One of a date's notes, in the order they were added. `mlm status` lists them under `Notes:`, and `mlm week --notes` under the date's row. Notes change no total, marker or footer."
        ));
    }
```

If the `a @ WeekArgs { .. }` binding trips the borrow checker on a non-`Copy` struct (it moves, which is fine here), use `Command::Week(a) if a.action.is_none() => a` instead.

### 2. `src/storage.rs`

#### 2.1 The ranged note read (contract 5)

Insert after `notes_for_date` (after line 477):

```rust
/// All notes whose local `date` falls within `[from, to]` inclusive,
/// ascending by date, then in insertion order (`created_at_utc ASC, id
/// ASC`) within a date, the order `notes_for_date` uses. Empty `Vec` for
/// a range with no rows, or when `from > to`.
pub fn notes_in_range(
    conn: &Connection,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<Note>, StorageError> {
    let mut stmt = conn.prepare(
        "SELECT id, \"date\", body, created_at_utc FROM notes \
         WHERE \"date\" BETWEEN ?1 AND ?2 \
         ORDER BY \"date\" ASC, created_at_utc ASC, id ASC",
    )?;
    let rows = stmt.query_map(
        (
            from.format("%Y-%m-%d").to_string(),
            to.format("%Y-%m-%d").to_string(),
        ),
        map_note_row,
    )?;
    let mut out = Vec::new();
    for row in rows {
        let (id, date, body, created_at_utc) = row?;
        out.push(note_from_row(id, date, body, created_at_utc)?);
    }
    Ok(out)
}
```

- The name mirrors `punches_in_range`. `export-notes` reuses it (spec §6).
- The existing index `idx_notes_date_created_at_utc_id` (`src/db.rs:61-62`) covers the filter and the order.
- A renamed `body` column fails at `prepare` as `StorageError::Db`. A bad `date` or `created_at_utc` fails as `CorruptRow`.

#### 2.2 Unit tests (new block after line 1517, header `// --- notes_in_range (week --notes) ---`)

```rust
    #[test]
    fn notes_in_range_includes_both_edges_and_excludes_outside() {
        let conn = test_db();
        for (day, body) in [(4, "before"), (5, "from"), (8, "mid"), (11, "to"), (12, "after")] {
            insert_note(&conn, d(2026, 1, day), body, utc(2026, 1, day, 9, 0, 0)).expect("insert");
        }
        let got = notes_in_range(&conn, d(2026, 1, 5), d(2026, 1, 11)).expect("read");
        assert_eq!(
            got.iter().map(|n| n.body.as_str()).collect::<Vec<_>>(),
            vec!["from", "mid", "to"]
        );
    }

    #[test]
    fn notes_in_range_orders_by_date_then_created_at_then_id() {
        let conn = test_db();
        // Inserted out of date order, later id with earlier created_at_utc.
        insert_note(&conn, d(2026, 1, 6), "tue", utc(2026, 1, 6, 8, 0, 0)).expect("insert");
        insert_note(&conn, d(2026, 1, 5), "mon late", utc(2026, 1, 5, 15, 0, 0)).expect("insert");
        insert_note(&conn, d(2026, 1, 5), "mon early", utc(2026, 1, 5, 10, 0, 0)).expect("insert");
        insert_note(&conn, d(2026, 1, 5), "mon tie a", utc(2026, 1, 5, 16, 0, 0)).expect("insert");
        insert_note(&conn, d(2026, 1, 5), "mon tie b", utc(2026, 1, 5, 16, 0, 0)).expect("insert");
        let got = notes_in_range(&conn, d(2026, 1, 5), d(2026, 1, 11)).expect("read");
        assert_eq!(
            got.iter().map(|n| (n.date, n.body.as_str())).collect::<Vec<_>>(),
            vec![
                (d(2026, 1, 5), "mon early"),
                (d(2026, 1, 5), "mon late"),
                (d(2026, 1, 5), "mon tie a"),
                (d(2026, 1, 5), "mon tie b"),
                (d(2026, 1, 6), "tue"),
            ]
        );
    }

    #[test]
    fn notes_in_range_matches_notes_for_date_per_date() {
        let conn = test_db();
        insert_note(&conn, d(2026, 1, 5), "B", utc(2026, 1, 5, 12, 5, 0)).expect("insert");
        insert_note(&conn, d(2026, 1, 5), "A", utc(2026, 1, 5, 12, 1, 0)).expect("insert");
        let ranged = notes_in_range(&conn, d(2026, 1, 5), d(2026, 1, 5)).expect("read");
        assert_eq!(ranged, notes_for_date(&conn, d(2026, 1, 5)).expect("read"));
    }

    #[test]
    fn notes_in_range_is_empty_for_an_empty_or_inverted_range() {
        let conn = test_db();
        insert_note(&conn, d(2026, 1, 8), "x", utc(2026, 1, 8, 9, 0, 0)).expect("insert");
        assert!(notes_in_range(&conn, d(2026, 2, 1), d(2026, 2, 7)).unwrap().is_empty());
        assert!(notes_in_range(&conn, d(2026, 1, 11), d(2026, 1, 5)).unwrap().is_empty());
    }

    #[test]
    fn notes_in_range_renamed_body_column_is_a_db_error() {
        let conn = test_db();
        conn.execute("ALTER TABLE notes RENAME COLUMN body TO body_gone", [])
            .expect("rename column");
        let err = notes_in_range(&conn, d(2026, 1, 5), d(2026, 1, 11));
        assert!(matches!(err, Err(StorageError::Db(_))));
        assert_eq!(earliest_data_date(&conn).unwrap(), None);
    }

    #[test]
    fn notes_in_range_reports_a_corrupt_created_at() {
        let conn = test_db();
        conn.execute(
            "INSERT INTO notes (\"date\", body, created_at_utc) VALUES ('2026-01-06', 'x', 'garbage')",
            [],
        )
        .expect("raw insert");
        let err = notes_in_range(&conn, d(2026, 1, 5), d(2026, 1, 11));
        assert!(matches!(
            err,
            Err(StorageError::CorruptRow { column: "created_at_utc", .. })
        ));
    }
```

`cargo fmt` reflows the long tuple lines. The second test's two `16:00` notes cover the `id ASC` tiebreak.

### 3. `src/week_view.rs`

#### 3.1 Docs

- Module doc line 1: `` //! `mlm week [WEEK_ID] [--notes]` — the rendering half of Milestone 11 (SPEC.md ``
- Module doc, replace lines 17-18 with:

```rust
//! - Accounting comes from `history::load_history`, the same loader
//!   `status` uses. A failed storage read is a hard error.
//! - With `--notes`, [`run`] makes one ranged note read of its own
//!   (`storage::notes_in_range`, Monday to Sunday), after the loader and
//!   outside it. Without the flag it makes no note read.
```

- `build_week_view` doc (232-233):

```rust
/// Build the full `WeekView` for `week`, with no notes attached. Every
/// storage read here goes through `history::load_history`, so a failed
/// read is an error. [`run`] adds the `--notes` read outside this builder.
```

- `run` doc (275-276): `` /// `mlm week [WEEK_ID] [--notes]` (the `action: None` arm; `week target` is ``, remainder unchanged.

#### 3.2 Imports

Line 27 area gains `use crate::storage::{self, Note};`. The test module's existing `use crate::storage::{self, Punch, PunchKind, punch_from_local};` (293) then shadows `self` harmlessly. If clippy or rustc flags a duplicate import, trim the test-side `self`.

#### 3.3 Row field

`WeekRow` gains a last field:

```rust
    /// This date's note bodies, in insertion order. Empty unless `run`
    /// attached them for `--notes`.
    pub notes: Vec<String>,
```

`build_rows_from` (208-216) sets `notes: Vec::new()`. The test helper `row()` (311-320) sets `notes: Vec::new()` as well (mechanical).

#### 3.4 Rendering (contract 3)

Add a constant near `WEEKDAY_ABBREVS` (line 38):

```rust
/// Prefix of a note line under its row: six spaces, then `- `.
const NOTE_PREFIX: &str = "      - ";
```

In `push_rows`, after the row's `out.push('\n');` (line 118), add `push_row_notes(out, row);`, with:

```rust
/// The row's notes, one line each, directly under the row.
fn push_row_notes(out: &mut String, row: &WeekRow) {
    for body in &row.notes {
        out.push_str(NOTE_PREFIX);
        out.push_str(body);
        out.push('\n');
    }
}
```

- The marker still ends the row line, and notes follow (spec §5).
- Bodies pass through unchanged, with no wrap or truncation (spec §1.1, §5).
- With every `notes` empty, `render_week` output is byte-identical to today's.
- `notice_lines`, `push_totals` and `push_header` do not change, so footers, the carry-in marker line and the Carry-in block stay the same.

#### 3.5 Attaching notes

```rust
/// Puts each note's body on the row whose date it carries, keeping the
/// order of `notes`. A note dated outside `dates` is dropped.
fn attach_notes(rows: &mut [WeekRow; 7], dates: [NaiveDate; 7], notes: Vec<Note>) {
    for note in notes {
        if let Some(i) = dates.iter().position(|d| *d == note.date) {
            rows[i].notes.push(note.body);
        }
    }
}
```

- A note belongs to its stored `date` (contract 4). Pairing and the splice never touch it, because attaching happens after rows are built and reads no punch.
- `storage::notes_in_range` returns notes ascending by date and in insertion order within a date, and `push` keeps that order.

#### 3.6 `run`

```rust
pub fn run(conn: &Connection, now: DateTime<Local>, args: &WeekArgs) -> anyhow::Result<()> {
    let today = now.date_naive();
    let week = match &args.week_id {
        Some(s) => parse_week_id(s, today)?,
        None => WeekId::current(today),
    };

    let mut view = build_week_view(conn, week, now)?;
    if args.notes {
        let (start, end) = week.span();
        let notes = storage::notes_in_range(conn, start, end)?;
        attach_notes(&mut view.rows, week.dates(), notes);
    }
    print!("{}", render_week(&view));
    Ok(())
}
```

- One ranged read over the shown week's Monday to Sunday, only with the flag (contract 5, spec §6).
- A failed read returns `Err` before anything prints. `main::exit_code` writes `error: database error: ...` to stderr and exits 1, with empty stdout (contract 7).
- `build_week_view`'s signature is unchanged, so `src/status.rs` and `src/lookback.rs` callers compile untouched.

#### 3.7 Mechanical test updates

- `c4_malformed_week_id_is_a_hard_error` (1157-1160) and `c9_anomaly_marker_end_to_end_and_exit_zero` (1231-1234) add `notes: false,` to their `WeekArgs` literals.

#### 3.8 New unit tests (in `mod tests`)

Helper:

```rust
    fn notes(bodies: &[&str]) -> Vec<String> {
        bodies.iter().map(|b| b.to_string()).collect()
    }

    fn strip_note_lines(s: &str) -> String {
        s.split_inclusive('\n')
            .filter(|l| !l.starts_with(NOTE_PREFIX))
            .collect()
    }
```

Tests:

1. `notes_render_under_their_rows_byte_exact`. `example_b()` with Mon notes `["fixed migration runner bug", "started punch pairing tests"]` and Wed `has_anomaly = true` plus notes `["code review for #41"]`. The full golden is `EXAMPLE_B_GOLDEN` with the Wed marker, the two Mon lines after the Mon row, the Wed line after the Wed row, and the `[!]` footer:

```
Week 2026-06 (2026-02-02 - 2026-02-08)

Total behind: 03h 10m

  Mon 2026-02-02   07h 30m
      - fixed migration runner bug
      - started punch pairing tests
  Tue 2026-02-03   08h 00m
  Wed 2026-02-04   07h 45m  [!]
      - code review for #41
  Thu 2026-02-05   08h 10m
  Fri 2026-02-06   05h 25m
  Sat 2026-02-07   00h 00m
  Sun 2026-02-08   00h 00m

[!] needs a look: run mlm status DATE for details and fixes

Carry-in:      00h 00m
Worked:        36h 50m
Fulfillment:   36h 50m
Target:        40h 00m
```

2. `marked_rows_keep_marker_then_notes`. `example_b()`, with Tue `has_overnight`, Wed `has_anomaly`, Thu both, and one note each (`"t"`, `"w"`, `"th"`). Assert the output contains:
   - `"  Tue 2026-02-03   08h 00m  [i]\n      - t\n"`
   - `"  Wed 2026-02-04   07h 45m  [!]\n      - w\n"`
   - `"  Thu 2026-02-05   08h 10m  [!] [i]\n      - th\n"`
   - Also assert `strip_note_lines(&out) == render_week(&same view with notes cleared)`.

3. `ongoing_row_is_followed_by_its_notes`. `example_a()` with Thu notes `["on call"]`. Assert it contains `"  Thu 2026-02-12   07h 25m (incl. ongoing)\n      - on call\n  Fri 2026-02-13"`, and `strip_note_lines` equals `EXAMPLE_A_GOLDEN`.

4. `note_only_row_keeps_its_zero_row`. `example_b()` with Sat (index 5) notes `["planning only"]`. Assert it contains `"  Sat 2026-02-07   00h 00m\n      - planning only\n  Sun 2026-02-08"`.

5. `notes_leave_footers_and_carry_block_unchanged`. Run the `blank_line_follows_footer_block_only_when_a_line_printed` mask loop (0..16) again, with two notes on Wed and one on Sun in every case. Assert `out.matches('\n').count() == 16 + extra + 3` and `strip_note_lines(&out) == render_week(&view_without_notes)`. This covers the `[!]` footer, the `[i]` footer, the Monday `[i]` line, the carry-in marker line and the `Carry-in: ... [!]` figure.

6. `monday_footer_unchanged_with_notes`. `example_a()` with `overnight_last_week = Some(d(2026, 2, 15))` and Mon notes `["x"]`. Assert `strip_note_lines` equals the `monday_footer_hand_built_golden` expectation (`EXAMPLE_A_GOLDEN` with the `[i] open stint on Sun 2026-02-15 (last week): ...` line).

7. `empty_notes_render_byte_identical`. For `example_a()` and `example_b()`, assert `render_week` equals `EXAMPLE_A_GOLDEN` and `EXAMPLE_B_GOLDEN` while every `notes` is empty. (Existing A1/A3 already pin this. This test names the rule.)

8. `non_ascii_note_passes_through`. `example_b()` with Mon notes `["caf\u{e9} \u{2013} review"]`. Assert the output contains `"      - caf\u{e9} \u{2013} review\n"`, and that `strip_note_lines(&out)` is all `\n` or `0x20..=0x7E`.

9. `attach_notes_by_date_in_order_and_drops_outside`. Use `build_rows(week_dates(), &[], ...)` and `storage::Note` values built directly, with ids 1-4: Mon "a", Mon "b", Sun "c", and one dated 2026-02-16 (the next Monday) "z". After `attach_notes`, `rows[0].notes == ["a", "b"]`, `rows[6].notes == ["c"]`, and no row holds `"z"`.

10. `run_with_notes_reads_and_errors_on_renamed_body`. Call `new_test_db()`, then `ALTER TABLE notes RENAME COLUMN body TO body_gone`. `run(&conn, fixed_now(), &WeekArgs { action: None, week_id: Some("2026-07".into()), notes: true })` is `Err`, and the same args with `notes: false` are `Ok`. This covers the failure path for coverage, and proves the plain view makes no note read.

11. `run_with_notes_attaches_through_storage`. Call `storage::insert_note` twice on `d(2026, 2, 10)`, `"second"` at 15:00 UTC, then `"first"` at 10:00 UTC. Then `build_week_view`, plus the `attach_notes` call `run` makes over `storage::notes_in_range(span)`, gives `rows[1].notes == ["first", "second"]`. Calling `run` with `notes: true` returns `Ok` (stdout is not captured in unit tests).

Test 11 repeats `run`'s two lines. To avoid that, extract `fn with_notes(conn, view: &mut WeekView, week) -> anyhow::Result<()>` from `run`, and have both `run` and the test call it. The implementer picks one form, and either is fine for complexity.

### 4. `src/history.rs` (module doc only)

Replace lines 1-7 with:

```rust
//! Shared history loader for `status` and `mlm week` (SPEC §2.4, §3.4).
//!
//! One earliest-date read, one ranged punch read and one ranged target
//! read feed the week ledger and hand back the bucketed punches, so every
//! consumer works from the same data. The loader visits only dates that
//! have punches, so a far-future week id costs nothing extra. Any storage
//! failure is an error, never a default. `mlm week --notes` makes one
//! ranged note read of its own, outside this loader.
```

No code change.

### 5. `tests/week_notes_cli.rs` (new)

The file drives the binary with `MLM_DB_PATH` and `TZ=UTC`, seeds raw rows, and names past week `2026-02` (Mon 2026-01-05 to Sun 2026-01-11), so no case depends on the clock.

```rust
//! Binary-level tests for `mlm week --notes` (week-notes spec §3-§7).
//!
//! Drives the compiled binary with `MLM_DB_PATH` and `TZ=UTC`, seeding
//! punches and notes by raw insert. Every fixture sits in past weeks, so
//! no output depends on the clock.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use mlm::cli::GLOSSARY;
use rusqlite::Connection;
use tempfile::TempDir;

const NOTE_PREFIX: &str = "      - ";

fn mlm(db: &Path, args: &[&str]) -> Output { /* as week_read_errors.rs:14-21 */ }
fn stdout(o: &Output) -> String { /* lossy */ }
fn stderr(o: &Output) -> String { /* lossy */ }

/// A migrated, empty database.
fn fresh() -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("mlm.db");
    drop(mlm::db::connect_at(&path).expect("create db"));
    (dir, path)
}

fn seed_punch(db: &Path, date: &str, hhmm: &str, kind: &str) {
    let conn = Connection::open(db).expect("open db");
    conn.execute(
        "INSERT INTO punches (at_utc, \"date\", kind) VALUES (?1, ?2, ?3)",
        (format!("{date}T{hhmm}:00Z"), date, kind),
    )
    .expect("seed punch");
}

fn seed_note(db: &Path, date: &str, body: &str, created_at_utc: &str) {
    let conn = Connection::open(db).expect("open db");
    conn.execute(
        "INSERT INTO notes (\"date\", body, created_at_utc) VALUES (?1, ?2, ?3)",
        (date, body, created_at_utc),
    )
    .expect("seed note");
}

fn ok_stdout(db: &Path, args: &[&str]) -> String {
    let o = mlm(db, args);
    assert_eq!(o.status.code(), Some(0), "{args:?}: {}", stderr(&o));
    stdout(&o)
}

fn strip_note_lines(s: &str) -> String {
    s.split_inclusive('\n').filter(|l| !l.starts_with(NOTE_PREFIX)).collect()
}
```

#### 5.1 Main fixture

```rust
/// Week 2026-02: Mon 8h with two notes (the later id has the earlier
/// created_at_utc), Tue 3h30, Wed an orphaned end with one note, Sat a
/// note-only date.
fn seeded() -> (TempDir, PathBuf) {
    let (dir, db) = fresh();
    seed_punch(&db, "2026-01-05", "09:00", "start");
    seed_punch(&db, "2026-01-05", "17:00", "end");
    seed_punch(&db, "2026-01-06", "09:00", "start");
    seed_punch(&db, "2026-01-06", "12:30", "end");
    seed_punch(&db, "2026-01-07", "15:00", "end");
    seed_note(&db, "2026-01-05", "started punch pairing tests", "2026-01-05T15:00:00Z");
    seed_note(&db, "2026-01-05", "fixed migration runner bug", "2026-01-05T10:00:00Z");
    seed_note(&db, "2026-01-07", "code review for #41", "2026-01-07T15:05:00Z");
    seed_note(&db, "2026-01-10", "planned next sprint", "2026-01-10T09:00:00Z");
    (dir, db)
}

const NOTES_2026_02: &str = "\
Week 2026-02 (2026-01-05 - 2026-01-11)

Total behind: 28h 30m

  Mon 2026-01-05   08h 00m
      - fixed migration runner bug
      - started punch pairing tests
  Tue 2026-01-06   03h 30m
  Wed 2026-01-07   00h 00m  [!]
      - code review for #41
  Thu 2026-01-08   00h 00m
  Fri 2026-01-09   00h 00m
  Sat 2026-01-10   00h 00m
      - planned next sprint
  Sun 2026-01-11   00h 00m

[!] needs a look: run mlm status DATE for details and fixes

Carry-in:      00h 00m
Worked:        11h 30m
Fulfillment:   11h 30m
Target:        40h 00m
";
```

How the figures come out:
- Mon and Tue match `tests/week_read_errors.rs`'s `GOLDEN_2026_02` (08h 00m, 03h 30m, Total behind 28h 30m, Worked 11h 30m).
- The Wed orphaned end adds no minutes. Tue has no open start, so no splice consumes it, and it marks `[!]` with the footer (SPEC E8).
- Every note is in week 2026-02, so the earliest data date stays 2026-01-05 and carry-in stays 0.

#### 5.2 Carry-marker fixture

```rust
/// Week 2026-01 holds an orphaned end (Tue 2025-12-30), so week 2026-02
/// prints the carry-in marker. Mon 2026-01-05 8h with one note.
fn seeded_carry() -> (TempDir, PathBuf) {
    let (dir, db) = fresh();
    seed_punch(&db, "2025-12-30", "10:00", "end");
    seed_punch(&db, "2026-01-05", "09:00", "start");
    seed_punch(&db, "2026-01-05", "17:00", "end");
    seed_note(&db, "2026-01-05", "carry note", "2026-01-05T12:00:00Z");
    (dir, db)
}

const CARRY_NOTES_2026_02: &str = "\
Week 2026-02 (2026-01-05 - 2026-01-11)

Total behind: 72h 00m

  Mon 2026-01-05   08h 00m
      - carry note
  Tue 2026-01-06   00h 00m
  Wed 2026-01-07   00h 00m
  Thu 2026-01-08   00h 00m
  Fri 2026-01-09   00h 00m
  Sat 2026-01-10   00h 00m
  Sun 2026-01-11   00h 00m

[!] carry-in includes anomalies from 1 earlier week, 2026-01 (see mlm week 2026-01)

Carry-in:      -40h 00m  [!]
Worked:        08h 00m
Fulfillment:   -32h 00m
Target:        40h 00m
";
```

How the figures come out:
- The walk starts at week 2026-01, which has 0 worked against 40h, so carry-out is -40h 00m.
- Week 2026-02 fulfillment is -40h + 8h = -32h 00m, and owed is 40h - (-32h) = 72h 00m.
- The marker uses the singular form (`src/render.rs:206`). The Carry-in figure gains `  [!]` (`src/week_view.rs:180-182`). This follows the arithmetic in `GOLDEN_2026_04`.

#### 5.3 Tests

One per acceptance criterion, as listed in the test plan below:

```rust
#[test]
fn notes_print_under_their_rows_in_insertion_order() {
    let (_d, db) = seeded();
    assert_eq!(ok_stdout(&db, &["week", "2026-02", "--notes"]), NOTES_2026_02);
}

#[test]
fn marked_row_keeps_marker_then_notes() {
    let (_d, db) = seeded();
    let out = ok_stdout(&db, &["week", "2026-02", "--notes"]);
    assert!(out.contains("  Wed 2026-01-07   00h 00m  [!]\n      - code review for #41\n  Thu"));
}

#[test]
fn note_only_date_keeps_its_zero_row() {
    let (_d, db) = seeded();
    let out = ok_stdout(&db, &["week", "2026-02", "--notes"]);
    assert!(out.contains("  Sat 2026-01-10   00h 00m\n      - planned next sprint\n  Sun"));
}

#[test]
fn week_without_notes_is_byte_identical_with_the_flag() {
    let (_d, db) = fresh();
    seed_punch(&db, "2026-01-05", "09:00", "start");
    seed_punch(&db, "2026-01-05", "17:00", "end");
    seed_punch(&db, "2026-01-07", "15:00", "end");
    let plain = ok_stdout(&db, &["week", "2026-02"]);
    assert_eq!(ok_stdout(&db, &["week", "2026-02", "--notes"]), plain);
    assert!(plain.contains("[!]"));
}

#[test]
fn plain_week_prints_no_note_lines() {
    let (_d, db) = seeded();
    let plain = ok_stdout(&db, &["week", "2026-02"]);
    assert_eq!(plain, strip_note_lines(NOTES_2026_02));
    assert!(!plain.contains(NOTE_PREFIX));
}

#[test]
fn carry_marker_line_unchanged_with_notes() {
    let (_d, db) = seeded_carry();
    assert_eq!(ok_stdout(&db, &["week", "2026-02", "--notes"]), CARRY_NOTES_2026_02);
    assert_eq!(ok_stdout(&db, &["week", "2026-02"]), strip_note_lines(CARRY_NOTES_2026_02));
}

#[test]
fn short_and_long_flag_before_and_after_week_id() {
    let (_d, db) = seeded();
    for args in [
        &["week", "-n", "2026-02"][..],
        &["week", "2026-02", "-n"],
        &["week", "--notes", "2026-02"],
        &["week", "2026-02", "--notes"],
        &["w", "-n", "2026-02"],
    ] {
        assert_eq!(ok_stdout(&db, args), NOTES_2026_02, "{args:?}");
    }
}

#[test]
fn notes_after_week_target_write_nothing() {
    let (_d, db) = fresh();
    let count = |db: &Path| -> i64 {
        Connection::open(db).unwrap()
            .query_row("SELECT COUNT(*) FROM week_targets", [], |r| r.get(0)).unwrap()
    };
    let o = mlm(&db, &["week", "target", "40h", "--notes"]);
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    assert_eq!(stdout(&o), "");
    let o = mlm(&db, &["week", "target", "2026-02", "40h", "--notes"]);
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    assert_eq!(count(&db), 0);
}

#[test]
fn notes_from_adjacent_weeks_never_appear() {
    let (_d, db) = fresh();
    seed_punch(&db, "2026-01-05", "09:00", "start");
    seed_punch(&db, "2026-01-05", "17:00", "end");
    seed_note(&db, "2026-01-04", "sunday before", "2026-01-04T09:00:00Z");
    seed_note(&db, "2026-01-12", "monday after", "2026-01-12T09:00:00Z");
    let shown = ok_stdout(&db, &["week", "2026-02", "--notes"]);
    assert_eq!(shown, ok_stdout(&db, &["week", "2026-02"]));
    assert!(!shown.contains("sunday before") && !shown.contains("monday after"));
    // Both edge notes are real: each shows in its own week.
    assert!(ok_stdout(&db, &["week", "2026-01", "--notes"])
        .contains("  Sun 2026-01-04   00h 00m\n      - sunday before\n"));
    assert!(ok_stdout(&db, &["week", "2026-03", "--notes"])
        .contains("  Mon 2026-01-12   00h 00m\n      - monday after\n"));
}

#[test]
fn renamed_note_body_fails_only_the_notes_view() {
    let (_d, db) = seeded();
    Connection::open(&db).unwrap()
        .execute_batch("ALTER TABLE notes RENAME COLUMN body TO body_gone")
        .expect("rename column");
    for args in [&["week", "2026-02", "--notes"][..], &["week", "--notes"]] {
        let o = mlm(&db, args);
        assert_eq!(o.status.code(), Some(1), "{args:?}");
        assert_eq!(stdout(&o), "", "{args:?}");
        assert!(stderr(&o).starts_with("error:"), "{}", stderr(&o));
    }
    assert_eq!(ok_stdout(&db, &["week", "2026-02"]), strip_note_lines(NOTES_2026_02));
}

const NON_ASCII: &str = "caf\u{e9} review \u{2013} done";

#[test]
fn non_ascii_body_passes_through_and_the_rest_is_ascii() {
    let (_d, db) = seeded();
    seed_note(&db, "2026-01-06", NON_ASCII, "2026-01-06T13:00:00Z");
    let out = ok_stdout(&db, &["week", "2026-02", "--notes"]);
    assert!(out.contains(&format!("  Tue 2026-01-06   03h 30m\n{NOTE_PREFIX}{NON_ASCII}\n")));
    let swept = out.replace(NON_ASCII, "");
    assert!(swept.bytes().all(|b| b == b'\n' || (0x20..=0x7E).contains(&b)), "{swept:?}");
}

#[test]
fn help_lists_the_flag_and_the_glossary_entry() {
    let (_d, db) = fresh();
    let long = ok_stdout(&db, &["week", "--help"]);
    assert!(long.contains("  -n, --notes    Show each date's notes under its row\n"), "{long}");
    assert!(long.contains("\n  - <note>\n      One of a date's notes"), "{long}");
    let short = ok_stdout(&db, &["week", "-h"]);
    assert!(short.contains("  -n, --notes    Show each date's notes under its row\n"));
    assert!(ok_stdout(&db, &["status", "--help"]).contains("\n  - <note>\n"));
    assert!(GLOSSARY.contains("`mlm week --notes` under the date's"));
}
```

`cargo fmt` reflows the long lines. The `(incl. ongoing)`, `[i]`, `[!] [i]` and Monday-line cases are unit tests (step 3.8), because they need the real clock. The `count` closure needs the table, which `fresh()` creates. The `exit 2` case never connects, so `fresh()` must run first.

### 6. `tests/week_target_cli.rs` (needs a coordinator decision, F1)

`WEEK_SHORT` (`Options:` block at line 231) gains one line above `-v`:

```
Options:
  -n, --notes    Show each date's notes under its row
  -v, --verbose  Enable debug-level logging
  -h, --help     Print help (see more with '--help')
```

Without this edit, `short_help_goldens` fails. Nothing else in that file changes. `long_help_carries_glossary` holds because `[!], [i]` stays last.

## Test plan

Verification set: `cargo fmt`, `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings`. The pre-commit gate adds complexity ≤ 15 and the coverage ratchet.

### T1. Notes under their rows, insertion order, two notes on one date (spec §7 case 1; §4, §5)

- CLI `notes_print_under_their_rows_in_insertion_order`. Mon's later id (`fixed migration runner bug`) has the earlier `created_at_utc` and prints first, so an id-only sort fails. Wed's note prints under Wed.
- Storage `notes_in_range_orders_by_date_then_created_at_then_id`, `notes_in_range_matches_notes_for_date_per_date`.
- Unit `attach_notes_by_date_in_order_and_drops_outside`, `run_with_notes_attaches_through_storage`, `notes_render_under_their_rows_byte_exact`.

### T2. Marked rows keep the marker, notes below (spec §7 case 2; §5)

- CLI `marked_row_keeps_marker_then_notes` covers `[!]`.
- Unit `marked_rows_keep_marker_then_notes` covers `[i]` and `[!] [i]`, since those depend on the clock.

### T3. Note-only date shows its `00h 00m` row plus notes (spec §7 case 3; §5)

- CLI `note_only_date_keeps_its_zero_row` (Sat 2026-01-10).
- Unit `note_only_row_keeps_its_zero_row`.

### T4. A week with no notes is byte-identical (spec §7 case 4; §5)

- CLI `week_without_notes_is_byte_identical_with_the_flag`.
- Unit `empty_notes_render_byte_identical`. Existing A1, A3 and all goldens in `src/week_view.rs` and `tests/week_read_errors.rs` pass untouched.

### T5. Plain `mlm week` with notes stored prints no note lines (spec §7 case 5; §1.1)

- CLI `plain_week_prints_no_note_lines`.

### T6. `(incl. ongoing)` row followed by notes (spec §7 case 6)

- Unit `ongoing_row_is_followed_by_its_notes`. This is a renderer test because the ongoing marker needs today.

### T7. Carry-in marker line and `[i]` footer unchanged (spec §7 case 7; §4, §5)

- CLI `carry_marker_line_unchanged_with_notes` (exact golden, both with and without the flag).
- Unit `notes_leave_footers_and_carry_block_unchanged` (all 16 footer combinations) and `monday_footer_unchanged_with_notes`.

### T8. `-n` and `--notes`, before and after `WEEK_ID` (spec §7 case 8; §3)

- CLI `short_and_long_flag_before_and_after_week_id`.
- `cli.rs` `notes_flag_parses_long_and_short_in_either_order`.

### T9. `mlm week target 40h --notes` exits nonzero and writes no target (spec §7 case 9; §3)

- CLI `notes_after_week_target_write_nothing` covers exit 1, then exit 2 for the three-token form, with zero rows in `week_targets`.
- `cli.rs` `notes_after_target_is_a_target_token`.

### T10. Notes from adjacent weeks never appear (spec §7 case 10; §6)

- CLI `notes_from_adjacent_weeks_never_appear`. The Sunday before and the Monday after are seeded, both are absent from week 2026-02, and each is present in its own week.
- Storage `notes_in_range_includes_both_edges_and_excludes_outside`.

### T11. `notes.body` renamed: `--notes` hard error, plain succeeds (spec §7 case 11; §6; contract 7)

- CLI `renamed_note_body_fails_only_the_notes_view`. Exit 1, empty stdout, stderr `error:...`, for both a past `WEEK_ID` and the default week. Plain `mlm week 2026-02` prints its golden.
- Unit `run_with_notes_reads_and_errors_on_renamed_body`.
- Storage `notes_in_range_renamed_body_column_is_a_db_error`, `notes_in_range_reports_a_corrupt_created_at`.

### T12. Non-ASCII body passes through, the rest is ASCII (spec §7 case 12; §5)

- CLI `non_ascii_body_passes_through_and_the_rest_is_ascii`.
- Unit `non_ascii_note_passes_through`.
- `tests/ascii_sweep.rs` stays untouched and passing.

### T13. `mlm week --help` lists the flag (spec §7 case 13; §8; contract 2)

- CLI `help_lists_the_flag_and_the_glossary_entry` checks the long and short week help and `status --help`.
- `cli.rs` `glossary_has_the_note_line_entry_before_markers`.
- `tests/week_target_cli.rs` `short_help_goldens` passes after step 6.

### T14. Existing tests pass with only mechanical updates (global constraints)

- Mechanical edits: the `cli.rs` pattern at 470-473; `WeekArgs` literals at `week_view.rs` 1157 and 1231; the `row()` helper and the `build_rows_from` literal.
- The `WEEK_SHORT` golden needs F1's decision.

## Risks, ambiguities and disagreements

- **F1. Unowned help golden breaks.** `tests/week_target_cli.rs` `WEEK_SHORT` pins `mlm week -h` exactly. The changeset plan does not list the file, and its Task 1 ownership excludes it. Adding the flag (contract 1) cannot avoid a one-line change there. The recommendation is to give Task 1 that one constant (step 6). It is mechanical and adds no mechanism.
- **Glossary entry wording.** The term `- <note>` mirrors the printed `- ` prefix, as other terms mirror printed tokens (`[!], [i]`, `(incl. ongoing)`). It reads true for `status` (two-space bullets under `Notes:`) and for `week --notes` (contract 3), per the coordinator's note. It sits third before `[!], [i]`, so Task 2's S2 variant A applies, with its shared sentence kept.
- **Spec §1.2 dropped.** The coordinator reports spec §1.2 is now "None". Behavior is unchanged, and the target-token cases are pinned from spec §3. Task 2's plan still adds §1.2a and README known-issue bullets (its S1, R2), and it cites `-n` before `target` as a rough edge in S5. Those now conflict with the update. That is Task 2's to drop, and it is noted here only so the coordinator sees it.
- **Read position.** The note read runs after the loader's three reads. A database where both fail reports the loader's error. That matches "after the loader's reads" in the changeset plan.
- **`attach_notes` drops out-of-range notes silently.** The range read never returns any, so the drop is defensive only. It is tested.
- **CLI goldens were derived, not executed.** This planner's sandbox could not run the binary. The main fixture's figures are `GOLDEN_2026_02`'s, and the carry fixture follows `GOLDEN_2026_04`'s carry arithmetic. If either golden is off by a figure at implementation, fix the constant to the binary's output only after confirming the figure from the arithmetic in 5.1 and 5.2.
- **Default-week error case.** `renamed_note_body_fails_only_the_notes_view` also runs `mlm week --notes` with no `WEEK_ID`. It asserts only the error shape, so the clock does not matter.
