//! Binary-level tests for `mlm stop`'s orphaned-end warning (SPEC §7.4).
//!
//! Each run uses `TZ=UTC`, so local dates and UTC dates agree and dates
//! derived from `Utc::now()` match the child's own clock. Only the final
//! `stop`'s output is asserted.

use std::path::Path;
use std::process::{Command, Output};

use chrono::{Duration, NaiveDate, Utc};
use mlm::date::{format_date, format_date_with_weekday};
use rusqlite::Connection;
use tempfile::TempDir;

fn mlm_utc(db: &Path, args: &[&str]) -> Output {
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

fn today() -> NaiveDate {
    Utc::now().date_naive()
}

fn days_ago(n: i64) -> NaiveDate {
    today() - Duration::days(n)
}

/// Creates the schema by running one read-only command.
fn boot() -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().expect("temp dir");
    let db = dir.path().join("mlm.db");
    let r = mlm_utc(&db, &["status"]);
    assert!(r.status.success(), "boot: {r:?}");
    (dir, db)
}

fn seed(db: &Path, date: NaiveDate, hhmm: &str, kind: &str) {
    let conn = Connection::open(db).expect("open db");
    conn.execute(
        "INSERT INTO punches (at_utc, \"date\", kind) VALUES (?1, ?2, ?3)",
        (
            format!("{}T{hhmm}:00Z", format_date(date)),
            format_date(date),
            kind,
        ),
    )
    .expect("seed punch");
}

fn run_ok(db: &Path, args: &[&str]) {
    let o = mlm_utc(db, args);
    assert!(o.status.success(), "{args:?}: {o:?}");
}

fn assert_stop(o: &Output, expected_stderr: &str) {
    assert_eq!(o.status.code(), Some(0), "{o:?}");
    assert_eq!(stdout(o), "");
    assert_eq!(stderr(o), expected_stderr);
    assert!(stderr(o).is_ascii());
}

/// Contract 1's block, restated here so a drift in the shared hint lines
/// fails this file.
fn block(date: NaiveDate, hhmm: &str) -> String {
    let d = format_date(date);
    format!(
        "warning: orphaned end on {} at {hhmm} (no matching start)\n    fix: mlm start HH:MM --date {d}  (a time before {hhmm})\n     or: mlm delete punch --date {d}  (lists punches; rerun with the end's number)\n",
        format_date_with_weekday(date)
    )
}

#[test]
fn fe5_stop_today_after_a_start_two_days_back() {
    let (_dir, db) = boot();
    run_ok(&db, &["start", "22:00", "--date", "-2"]);
    let o = mlm_utc(&db, &["stop", "14:16"]);
    assert_stop(&o, &block(today(), "14:16"));
}

#[test]
fn spec_example_block_verbatim() {
    let (_dir, db) = boot();
    run_ok(&db, &["start", "22:00", "--date", "2026-10-03"]);
    let o = mlm_utc(&db, &["stop", "14:16", "--date", "2026-10-05"]);
    assert_stop(
        &o,
        "warning: orphaned end on Mon 2026-10-05 at 14:16 (no matching start)\n    fix: mlm start HH:MM --date 2026-10-05  (a time before 14:16)\n     or: mlm delete punch --date 2026-10-05  (lists punches; rerun with the end's number)\n",
    );
}

#[test]
fn clean_pair_is_silent() {
    let (_dir, db) = boot();
    run_ok(&db, &["start", "09:00"]);
    let o = mlm_utc(&db, &["stop", "17:00"]);
    assert_stop(&o, "");
}

#[test]
fn splice_with_yesterdays_open_start_is_silent() {
    let (_dir, db) = boot();
    seed(&db, days_ago(1), "22:00", "start");
    let o = mlm_utc(&db, &["stop", "01:00"]);
    assert_stop(&o, "");
}

#[test]
fn displaced_end_warns_for_the_later_end() {
    let (_dir, db) = boot();
    run_ok(&db, &["start", "09:00"]);
    run_ok(&db, &["stop", "17:00"]);
    let o = mlm_utc(&db, &["stop", "12:00"]);
    assert_stop(&o, &block(today(), "17:00"));
}

#[test]
fn pre_existing_orphan_is_reported_after_a_clean_pair() {
    let (_dir, db) = boot();
    seed(&db, today(), "08:00", "end");
    run_ok(&db, &["start", "09:00"]);
    let o = mlm_utc(&db, &["stop", "17:00"]);
    assert_stop(&o, &block(today(), "08:00"));
}

#[test]
fn two_orphans_print_two_blocks_ascending() {
    let (_dir, db) = boot();
    seed(&db, today(), "16:00", "end");
    let o = mlm_utc(&db, &["stop", "10:00"]);
    assert_stop(&o, &(block(today(), "10:00") + &block(today(), "16:00")));
}

#[test]
fn past_date_block_names_that_date() {
    let (_dir, db) = boot();
    let date = format_date(days_ago(5));
    let o = mlm_utc(&db, &["stop", "18:00", "--date", &date]);
    assert_stop(&o, &block(days_ago(5), "18:00"));
}

#[test]
fn stop_hint_lines_match_status_page() {
    let (_dir, db) = boot();
    let date = format_date(days_ago(5));
    let o = mlm_utc(&db, &["stop", "18:00", "--date", &date]);
    let status = mlm_utc(&db, &["status", &date]);
    assert!(status.status.success(), "{status:?}");
    let page = stdout(&status);
    let warning = stderr(&o);
    let lines: Vec<&str> = warning.lines().collect();
    assert_eq!(lines.len(), 3, "{warning}");
    for hint in &lines[1..] {
        assert!(page.lines().any(|l| l == *hint), "{hint:?} not in:\n{page}");
    }
}

/// The stored end's date and `HH:MM`, so the bare-`stop` test never
/// races the clock.
fn stored_end(db: &Path) -> (NaiveDate, String) {
    let conn = Connection::open(db).expect("open db");
    let (date, at_utc): (String, String) = conn
        .query_row(
            "SELECT \"date\", at_utc FROM punches WHERE kind = 'end'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("one stored end");
    let date = NaiveDate::parse_from_str(&date, "%Y-%m-%d").expect("stored date");
    (date, at_utc[11..16].to_string())
}

#[test]
fn bare_stop_warns_with_status_page_hints() {
    let (_dir, db) = boot();
    run_ok(&db, &["start", "22:00", "--date", "-2"]);
    let o = mlm_utc(&db, &["stop"]);
    let (date, hhmm) = stored_end(&db);
    let status = mlm_utc(&db, &["status", &format_date(date)]);
    assert!(status.status.success(), "{status:?}");
    let page = stdout(&status);

    assert_stop(&o, &block(date, &hhmm));
    let warning = stderr(&o);
    let lines: Vec<&str> = warning.lines().collect();
    assert_eq!(lines.len(), 3, "{warning}");
    let header = format!(
        "warning: orphaned end on {} at ",
        format_date_with_weekday(date)
    );
    assert!(lines[0].starts_with(&header), "{warning}");
    for hint in &lines[1..] {
        assert!(page.lines().any(|l| l == *hint), "{hint:?} not in:\n{page}");
    }
}

#[test]
fn read_failure_on_the_previous_date_prints_nothing() {
    let (_dir, db) = boot();
    let conn = Connection::open(&db).expect("open db");
    conn.execute(
        "INSERT INTO punches (at_utc, \"date\", kind) VALUES ('garbage', ?1, 'start')",
        [format_date(days_ago(1))],
    )
    .expect("seed garbage row");
    let o = mlm_utc(&db, &["stop", "17:00"]);
    assert_stop(&o, "");
    let ends: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM punches WHERE \"date\" = ?1 AND kind = 'end'",
            [format_date(today())],
            |r| r.get(0),
        )
        .expect("count ends");
    assert_eq!(ends, 1);
}
