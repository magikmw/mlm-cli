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

fn mlm(db: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mlm"))
        .args(args)
        .env("MLM_DB_PATH", db)
        .env("TZ", "UTC")
        .env_remove("RUST_LOG")
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

fn seed_note(db: &Path, date: &str, body: &str, created_at_utc: &str) {
    let conn = Connection::open(db).expect("open db");
    conn.execute(
        "INSERT INTO notes (\"date\", body, created_at_utc) VALUES (?1, ?2, ?3)",
        (date, body, created_at_utc),
    )
    .expect("seed note");
}

/// Seed one note on `date`, created at noon UTC of that date.
fn seed_on(db: &Path, date: NaiveDate, body: &str) {
    let day = format_date(date);
    seed_note(db, &day, body, &format!("{day}T12:00:00Z"));
}

/// Exit `0`, empty stderr; returns stdout.
fn ok_stdout(db: &Path, args: &[&str]) -> String {
    let o = mlm(db, args);
    assert_eq!(o.status.code(), Some(0), "{args:?}: {}", stderr(&o));
    assert_eq!(stderr(&o), "", "{args:?}");
    stdout(&o)
}

/// That exit code, empty stdout and exactly `want_stderr`.
fn fails(db: &Path, args: &[&str], code: i32, want_stderr: &str) {
    let o = mlm(db, args);
    assert_eq!(o.status.code(), Some(code), "{args:?}: {}", stderr(&o));
    assert_eq!(stdout(&o), "", "{args:?}");
    assert_eq!(stderr(&o), want_stderr, "{args:?}");
}

/// Exit `2`, empty stdout; returns stderr.
fn usage_error(db: &Path, args: &[&str]) -> String {
    let o = mlm(db, args);
    assert_eq!(o.status.code(), Some(2), "{args:?}: {}", stderr(&o));
    assert_eq!(stdout(&o), "", "{args:?}");
    stderr(&o)
}

fn line(date: NaiveDate, body: &str) -> String {
    format!("{}  {body}\n", format_date(date))
}

/// Run `f(today)` until a run starts and ends on the same UTC date.
fn on_stable_today(f: impl Fn(NaiveDate)) {
    loop {
        let today = Utc::now().date_naive();
        f(today);
        if Utc::now().date_naive() == today {
            return;
        }
    }
}

fn is_ascii_text(s: &str) -> bool {
    s.bytes().all(|b| b == b'\n' || (0x20..=0x7E).contains(&b))
}

/// ISO week 2026-02 (2026-01-05 to 2026-01-11) and its two neighbours.
/// Insertion order makes `id` order disagree with date order, and with
/// `created_at_utc` order on Monday.
fn seeded_week() -> (TempDir, PathBuf) {
    let (dir, db) = fresh();
    for (date, body, at) in [
        ("2026-01-07", "code review for #41", "2026-01-07T15:05:00Z"),
        (
            "2026-01-05",
            "started punch pairing tests",
            "2026-01-05T15:00:00Z",
        ),
        (
            "2026-01-05",
            "fixed migration runner bug",
            "2026-01-05T10:00:00Z",
        ),
        ("2026-01-11", "sunday wrap-up", "2026-01-11T18:00:00Z"),
        ("2026-01-04", "sunday before", "2026-01-04T09:00:00Z"),
        ("2026-01-12", "monday after", "2026-01-12T09:00:00Z"),
        ("2026-01-06", "tuesday standup", "2026-01-06T09:00:00Z"),
    ] {
        seed_note(&db, date, body, at);
    }
    (dir, db)
}

const WEEK_2026_02: &str = "\
2026-01-05  fixed migration runner bug
2026-01-05  started punch pairing tests
2026-01-06  tuesday standup
2026-01-07  code review for #41
2026-01-11  sunday wrap-up
";

/// One note per date, body `m <date>`, around three month edges.
fn seeded_months() -> (TempDir, PathBuf) {
    let (dir, db) = fresh();
    for date in [
        "2024-01-31",
        "2024-02-01",
        "2024-02-29",
        "2024-03-01",
        "2026-01-31",
        "2026-02-01",
        "2026-02-28",
        "2026-03-01",
        "2025-11-30",
        "2025-12-01",
        "2025-12-31",
        "2026-01-01",
    ] {
        seed_note(
            &db,
            date,
            &format!("m {date}"),
            &format!("{date}T12:00:00Z"),
        );
    }
    (dir, db)
}

/// Seeds the current week's edges and both neighbours on a fresh DB.
/// Returns the DB and the expected output for the current week.
fn seeded_current_week(today: NaiveDate) -> (TempDir, PathBuf, String) {
    let (dir, db) = fresh();
    let (mon, sun) = WeekId::current(today).span();
    seed_on(&db, mon - Days::new(1), "before");
    seed_on(&db, mon, "first");
    seed_on(&db, sun, "last");
    seed_on(&db, sun + Days::new(1), "after");
    let want = line(mon, "first") + &line(sun, "last");
    (dir, db, want)
}

// --- 6.1 ranges and their edges -----------------------------------------

#[test]
fn week_range_prints_monday_to_sunday() {
    let (_d, db) = seeded_week();
    assert_eq!(
        ok_stdout(&db, &["export", "notes", "--week", "2026-02"]),
        WEEK_2026_02
    );
}

#[test]
fn date_range_prints_one_day() {
    let (_d, db) = seeded_week();
    assert_eq!(
        ok_stdout(&db, &["export", "notes", "--date", "2026-01-05"]),
        "2026-01-05  fixed migration runner bug\n2026-01-05  started punch pairing tests\n"
    );
    assert_eq!(
        ok_stdout(&db, &["export", "notes", "-d", "2026-01-04"]),
        "2026-01-04  sunday before\n"
    );
}

#[test]
fn month_range_prints_first_to_last_day() {
    let (_d, db) = seeded_months();
    assert_eq!(
        ok_stdout(&db, &["export", "notes", "--month", "2026-02"]),
        "2026-02-01  m 2026-02-01\n2026-02-28  m 2026-02-28\n"
    );
}

// --- 6.2 default range ----------------------------------------------------

#[test]
fn default_range_is_the_current_week() {
    on_stable_today(|today| {
        let (_d, db, want) = seeded_current_week(today);
        assert_eq!(ok_stdout(&db, &["export", "notes"]), want);
        assert_eq!(ok_stdout(&db, &["ex", "n"]), want);
    });
}

// --- 6.3 order ------------------------------------------------------------

#[test]
fn lines_run_by_date_then_insertion_order() {
    let (_d, db) = seeded_week();
    let out = ok_stdout(&db, &["export", "notes", "-w", "2026-02"]);
    assert_eq!(out, WEEK_2026_02);
    assert!(
        out.find("fixed") < out.find("started"),
        "same-date notes run by created_at_utc, not id: {out}"
    );
}

// --- 6.4 hyphen values ----------------------------------------------------

#[test]
fn minus_one_date_is_yesterday() {
    on_stable_today(|today| {
        let (_d, db) = fresh();
        let yesterday = today - Days::new(1);
        seed_on(&db, today - Days::new(2), "two days ago");
        seed_on(&db, yesterday, "yesterday");
        seed_on(&db, today, "today");
        assert_eq!(
            ok_stdout(&db, &["export", "notes", "-d", "-1"]),
            line(yesterday, "yesterday")
        );
    });
}

#[test]
fn minus_one_week_and_month_are_value_errors() {
    let (_d, db) = fresh();
    fails(
        &db,
        &["export", "notes", "-w", "-1"],
        1,
        "error: invalid WEEK_ID \"-1\": expected YYYY-WW or WW\n",
    );
    fails(
        &db,
        &["export", "notes", "-m", "-1"],
        1,
        "error: invalid MONTH \"-1\": expected YYYY-MM or MM\n",
    );
}

// --- 6.5 bare and unpadded forms ------------------------------------------

#[test]
fn bare_week_uses_the_current_iso_year() {
    on_stable_today(|today| {
        let (_d, db, want) = seeded_current_week(today);
        let week = WeekId::current(today).week().to_string();
        assert_eq!(ok_stdout(&db, &["export", "notes", "-w", &week]), want);
    });
}

#[test]
fn bare_month_uses_the_current_calendar_year() {
    on_stable_today(|today| {
        let (_d, db) = fresh();
        let first = today.with_day(1).unwrap();
        let next_first = if today.month() == 12 {
            NaiveDate::from_ymd_opt(today.year() + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(today.year(), today.month() + 1, 1)
        }
        .unwrap();
        let last = next_first - Days::new(1);
        seed_on(&db, first - Days::new(1), "before");
        seed_on(&db, first, "first");
        seed_on(&db, last, "last");
        seed_on(&db, next_first, "after");
        let want = line(first, "first") + &line(last, "last");
        for m in [today.month().to_string(), format!("{:02}", today.month())] {
            assert_eq!(ok_stdout(&db, &["export", "notes", "-m", &m]), want, "{m}");
        }
    });
}

#[test]
fn unpadded_year_month_equals_padded() {
    let (_d, db) = seeded_months();
    let padded = ok_stdout(&db, &["export", "notes", "-m", "2026-02"]);
    assert!(!padded.is_empty());
    assert_eq!(ok_stdout(&db, &["export", "notes", "-m", "2026-2"]), padded);
}

// --- 6.6 month edges ------------------------------------------------------

#[test]
fn month_edges_follow_the_calendar() {
    let (_d, db) = seeded_months();
    for (month, want) in [
        (
            "2024-02",
            "2024-02-01  m 2024-02-01\n2024-02-29  m 2024-02-29\n",
        ),
        (
            "2026-02",
            "2026-02-01  m 2026-02-01\n2026-02-28  m 2026-02-28\n",
        ),
        (
            "2025-12",
            "2025-12-01  m 2025-12-01\n2025-12-31  m 2025-12-31\n",
        ),
    ] {
        assert_eq!(
            ok_stdout(&db, &["export", "notes", "-m", month]),
            want,
            "{month}"
        );
    }
}

// --- 6.7 empty range, 6.8 future ranges -----------------------------------

#[test]
fn empty_range_prints_nothing() {
    let (_d, db) = seeded_week();
    for args in [
        &["export", "notes", "-w", "2026-05"][..],
        &["export", "notes", "-d", "2026-01-08"],
        &["export", "notes", "-m", "2025-06"],
    ] {
        assert_eq!(ok_stdout(&db, args), "", "{args:?}");
    }
    let (_d2, empty) = fresh();
    assert_eq!(ok_stdout(&empty, &["export", "notes"]), "");
}

#[test]
fn future_ranges_print_nothing() {
    let (_d, db) = seeded_week();
    for args in [
        &["export", "notes", "-d", "2099-06-15"][..],
        &["export", "notes", "-w", "2099-10"],
        &["export", "notes", "-m", "2099-06"],
    ] {
        assert_eq!(ok_stdout(&db, args), "", "{args:?}");
    }
}

// --- 6.9 value errors -----------------------------------------------------

const VALUE_ERRORS: [(&[&str], &str); 12] = [
    (
        &["-d", "abc"],
        "error: invalid DATE \"abc\": expected YYYY-MM-DD or -N (N >= 1)\n",
    ),
    (
        &["-d", "2026-02-30"],
        "error: invalid DATE \"2026-02-30\": not a real calendar date\n",
    ),
    (
        &["-w", "2026-54"],
        "error: invalid WEEK_ID \"2026-54\": 2026 has only 53 ISO weeks\n",
    ),
    (
        &["-w", "abc"],
        "error: invalid WEEK_ID \"abc\": expected YYYY-WW or WW\n",
    ),
    (
        &["-m", "2026-003"],
        "error: invalid MONTH \"2026-003\": expected YYYY-MM or MM\n",
    ),
    (
        &["-m", "26-03"],
        "error: invalid MONTH \"26-03\": expected YYYY-MM or MM\n",
    ),
    (
        &["-m", "abc"],
        "error: invalid MONTH \"abc\": expected YYYY-MM or MM\n",
    ),
    (
        &["-m", "2026-"],
        "error: invalid MONTH \"2026-\": expected YYYY-MM or MM\n",
    ),
    (
        &["-m", "-d"],
        "error: invalid MONTH \"-d\": expected YYYY-MM or MM\n",
    ),
    (
        &["-m", "0"],
        "error: invalid MONTH \"0\": month must be 1-12\n",
    ),
    (
        &["-m", "13"],
        "error: invalid MONTH \"13\": month must be 1-12\n",
    ),
    (
        &["-m", "2026-13"],
        "error: invalid MONTH \"2026-13\": month must be 1-12\n",
    ),
];

#[test]
fn value_errors_print_their_exact_message() {
    let (_d, db) = fresh();
    for (flags, want) in VALUE_ERRORS {
        let args: Vec<&str> = ["export", "notes"]
            .into_iter()
            .chain(flags.iter().copied())
            .collect();
        fails(&db, &args, 1, want);
    }
}

// --- 6.10 failed note read ------------------------------------------------

#[test]
fn renamed_note_body_is_an_error_not_an_empty_export() {
    let (_d, db) = seeded_week();
    Connection::open(&db)
        .unwrap()
        .execute_batch("ALTER TABLE notes RENAME COLUMN body TO body_gone")
        .expect("rename column");
    for args in [
        &["export", "notes"][..],
        &["export", "notes", "-w", "2026-02"],
        &["ex", "n", "-m", "2026-01"],
    ] {
        let o = mlm(&db, args);
        assert_eq!(o.status.code(), Some(1), "{args:?}");
        assert_eq!(stdout(&o), "", "{args:?}");
        let err = stderr(&o);
        assert!(err.starts_with("error: database error: "), "{err}");
        assert!(err.ends_with('\n'), "{err:?}");
    }
}

// --- 6.11 usage errors ----------------------------------------------------

#[test]
fn usage_errors_exit_two() {
    let (_d, db) = fresh();
    for args in [&["export"][..], &["ex"]] {
        let err = usage_error(&db, args);
        assert!(err.contains("notes"), "{args:?}: {err}");
        assert!(err.contains("Usage:"), "{args:?}: {err}");
    }
    for args in [
        &["export", "notes", "-d", "2026-01-05", "-w", "2026-02"][..],
        &["export", "notes", "-w", "2026-02", "-m", "2026-01"],
        &[
            "export",
            "notes",
            "-d",
            "2026-01-05",
            "-w",
            "2026-02",
            "-m",
            "2026-01",
        ],
        &["export", "notes", "-m", "3", "-m", "4"],
        &["export", "notes", "-d"],
        &["export", "notes", "-w"],
        &["export", "notes", "-m"],
    ] {
        usage_error(&db, args);
    }
}

// --- 6.12 aliases ---------------------------------------------------------

#[test]
fn aliases_behave_like_the_long_form() {
    let (_d, db) = seeded_week();
    for args in [
        &["ex", "n", "-w", "2026-02"][..],
        &["ex", "notes", "-w", "2026-02"],
        &["export", "n", "-w", "2026-02"],
        &["export", "notes", "--week", "2026-02"],
    ] {
        assert_eq!(ok_stdout(&db, args), WEEK_2026_02, "{args:?}");
    }
    assert_eq!(
        ok_stdout(&db, &["export", "notes", "--date", "2026-01-05"]),
        ok_stdout(&db, &["export", "notes", "-d", "2026-01-05"])
    );
    assert_eq!(
        ok_stdout(&db, &["export", "notes", "--month", "2026-01"]),
        ok_stdout(&db, &["export", "notes", "-m", "2026-01"])
    );
}

// --- 6.13 non-ASCII passthrough -------------------------------------------

const NON_ASCII: &str = "caf\u{e9} review \u{2013} done";

#[test]
fn non_ascii_body_passes_through_and_the_rest_is_ascii() {
    let (_d, db) = seeded_week();
    seed_note(&db, "2026-01-08", NON_ASCII, "2026-01-08T09:00:00Z");
    let out = ok_stdout(&db, &["export", "notes", "-w", "2026-02"]);
    assert!(out.contains(&format!("2026-01-08  {NON_ASCII}\n")), "{out}");
    let swept = out.replace(NON_ASCII, "");
    assert!(is_ascii_text(&swept), "{swept:?}");

    let value_err = stderr(&mlm(&db, &["export", "notes", "-m", "13"]));
    assert!(is_ascii_text(&value_err), "{value_err:?}");
    let usage_err = usage_error(&db, &["export"]);
    assert!(usage_err.is_ascii(), "{usage_err:?}");
    for args in [
        &["--help"][..],
        &["export", "--help"],
        &["export", "notes", "--help"],
        &["export", "notes", "-h"],
    ] {
        let help = ok_stdout(&db, args);
        assert!(is_ascii_text(&help), "{args:?}: {help:?}");
    }
}

// --- 6.14 help ------------------------------------------------------------

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

fn help(db: &Path, args: &[&str]) -> String {
    ok_stdout(db, args).replace("mlm.exe ", "mlm ")
}

#[test]
fn help_lines_match_the_contract() {
    let (_d, db) = fresh();
    assert_eq!(help(&db, &["export", "notes", "--help"]), EXPORT_NOTES_HELP);
    assert_eq!(help(&db, &["export", "notes", "-h"]), EXPORT_NOTES_HELP);
    let top = help(&db, &["--help"]);
    assert!(
        top.contains("\n  export  Print stored data as plain text [alias: ex]\n"),
        "{top}"
    );
    let export = help(&db, &["export", "--help"]);
    assert!(
        export.contains(
            "\n  notes  Print notes for a date, week or month, one per line (defaults to the current week) [alias: n]\n"
        ),
        "{export}"
    );
}
