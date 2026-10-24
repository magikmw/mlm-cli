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

fn mlm(db: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mlm"))
        .args(args)
        .env("MLM_DB_PATH", db)
        .env("TZ", "UTC")
        .output()
        .expect("failed to run mlm binary")
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

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
    s.split_inclusive('\n')
        .filter(|l| !l.starts_with(NOTE_PREFIX))
        .collect()
}

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
    seed_note(
        &db,
        "2026-01-05",
        "started punch pairing tests",
        "2026-01-05T15:00:00Z",
    );
    seed_note(
        &db,
        "2026-01-05",
        "fixed migration runner bug",
        "2026-01-05T10:00:00Z",
    );
    seed_note(
        &db,
        "2026-01-07",
        "code review for #41",
        "2026-01-07T15:05:00Z",
    );
    seed_note(
        &db,
        "2026-01-10",
        "planned next sprint",
        "2026-01-10T09:00:00Z",
    );
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

#[test]
fn notes_print_under_their_rows_in_insertion_order() {
    let (_d, db) = seeded();
    assert_eq!(
        ok_stdout(&db, &["week", "2026-02", "--notes"]),
        NOTES_2026_02
    );
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
    assert_eq!(
        ok_stdout(&db, &["week", "2026-02", "--notes"]),
        CARRY_NOTES_2026_02
    );
    assert_eq!(
        ok_stdout(&db, &["week", "2026-02"]),
        strip_note_lines(CARRY_NOTES_2026_02)
    );
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
        Connection::open(db)
            .unwrap()
            .query_row("SELECT COUNT(*) FROM week_targets", [], |r| r.get(0))
            .unwrap()
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
    assert!(
        ok_stdout(&db, &["week", "2026-01", "--notes"])
            .contains("  Sun 2026-01-04   00h 00m\n      - sunday before\n")
    );
    assert!(
        ok_stdout(&db, &["week", "2026-03", "--notes"])
            .contains("  Mon 2026-01-12   00h 00m\n      - monday after\n")
    );
}

#[test]
fn renamed_note_body_fails_only_the_notes_view() {
    let (_d, db) = seeded();
    Connection::open(&db)
        .unwrap()
        .execute_batch("ALTER TABLE notes RENAME COLUMN body TO body_gone")
        .expect("rename column");
    for args in [&["week", "2026-02", "--notes"][..], &["week", "--notes"]] {
        let o = mlm(&db, args);
        assert_eq!(o.status.code(), Some(1), "{args:?}");
        assert_eq!(stdout(&o), "", "{args:?}");
        assert!(stderr(&o).starts_with("error:"), "{}", stderr(&o));
    }
    assert_eq!(
        ok_stdout(&db, &["week", "2026-02"]),
        strip_note_lines(NOTES_2026_02)
    );
}

const NON_ASCII: &str = "caf\u{e9} review \u{2013} done";

#[test]
fn non_ascii_body_passes_through_and_the_rest_is_ascii() {
    let (_d, db) = seeded();
    seed_note(&db, "2026-01-06", NON_ASCII, "2026-01-06T13:00:00Z");
    let out = ok_stdout(&db, &["week", "2026-02", "--notes"]);
    assert!(out.contains(&format!(
        "  Tue 2026-01-06   03h 30m\n{NOTE_PREFIX}{NON_ASCII}\n"
    )));
    let swept = out.replace(NON_ASCII, "");
    assert!(
        swept
            .bytes()
            .all(|b| b == b'\n' || (0x20..=0x7E).contains(&b)),
        "{swept:?}"
    );
}

#[test]
fn help_lists_the_flag_and_the_glossary_entry() {
    let (_d, db) = fresh();
    let long = ok_stdout(&db, &["week", "--help"]);
    assert!(
        long.contains("  -n, --notes\n          Show each date's notes under its row\n"),
        "{long}"
    );
    assert!(
        long.contains("\n  - <note>\n      One of a date's notes"),
        "{long}"
    );
    let short = ok_stdout(&db, &["week", "-h"]);
    assert!(short.contains("  -n, --notes    Show each date's notes under its row\n"));
    assert!(ok_stdout(&db, &["status", "--help"]).contains("\n  - <note>\n"));
    assert!(GLOSSARY.contains("`mlm week --notes` under the date's"));
}
