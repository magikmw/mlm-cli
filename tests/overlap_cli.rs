//! Binary-level tests for the overlap anomaly and the clean-pair stop
//! hints (possible-issues-triage §3.2.1, §3.2.2).
//!
//! Punches are seeded straight into SQLite on Monday 2026-01-05 (week
//! 2026-02) with `TZ=UTC`, so the printed local times equal the seeded
//! UTC times.

use std::path::Path;
use std::process::{Command, Output};

use rusqlite::Connection;
use tempfile::TempDir;

const DATE: &str = "2026-01-05";
const WEEK: &str = "2026-02";

fn mlm_tz(db_path: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mlm"))
        .args(args)
        .env("MLM_DB_PATH", db_path)
        .env("TZ", "UTC")
        .output()
        .expect("failed to run mlm binary")
}

fn seed_punch(db_path: &Path, at_utc: &str, date: &str, kind: &str) {
    let conn = Connection::open(db_path).expect("open seeded db");
    conn.execute(
        "INSERT INTO punches (at_utc, \"date\", kind) VALUES (?1, ?2, ?3)",
        (at_utc, date, kind),
    )
    .expect("seed punch insert");
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// A fresh database with the schema applied and the given
/// `(kind, "HH:MM")` punches on [`DATE`].
fn seeded(punches: &[(&str, &str)]) -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().expect("temp dir");
    let db_path = dir.path().join("mlm.db");
    let boot = mlm_tz(&db_path, &["status"]);
    assert!(boot.status.success(), "bootstrap: {boot:?}");
    for (kind, time) in punches {
        seed_punch(&db_path, &format!("{DATE}T{time}:00Z"), DATE, kind);
    }
    (dir, db_path)
}

const NESTED: [(&str, &str); 4] = [
    ("start", "09:00"),
    ("start", "10:00"),
    ("end", "11:00"),
    ("end", "12:00"),
];

const STALE: [(&str, &str); 3] = [("start", "09:00"), ("start", "10:00"), ("end", "10:30")];

const E7: [(&str, &str); 4] = [
    ("start", "09:00"),
    ("start", "14:00"),
    ("start", "15:00"),
    ("end", "15:30"),
];

#[test]
fn status_prints_the_overlap_lines_and_exits_zero() {
    let (_dir, db) = seeded(&NESTED);
    let out = mlm_tz(&db, &["status", DATE]);
    assert!(out.status.success(), "{out:?}");
    let text = stdout(&out);
    assert!(
        text.contains(
            "[!] overlapping stints 09:00-12:00 and 10:00-11:00 (the overlap counts twice)"
        ),
        "{text}"
    );
    assert!(
        text.contains("    fix: mlm delete punch --date 2026-01-05  (lists punches; rerun with the number of the punch to remove)"),
        "{text}"
    );
    assert!(text.contains("04h 00m"), "double count stays: {text}");
}

#[test]
fn week_marks_the_row_and_prints_one_footer() {
    let (_dir, db) = seeded(&NESTED);
    let text = stdout(&mlm_tz(&db, &["week", WEEK]));
    assert!(text.contains("  Mon 2026-01-05   04h 00m  [!]"), "{text}");
    let footer = "[!] needs a look: run mlm status DATE for details and fixes";
    assert_eq!(text.matches(footer).count(), 1, "{text}");
    assert!(text.contains("Worked:        04h 00m"), "{text}");
}

#[test]
fn touching_stints_are_clean() {
    let (_dir, db) = seeded(&[
        ("start", "09:00"),
        ("end", "10:00"),
        ("start", "10:00"),
        ("end", "11:00"),
    ]);
    let text = stdout(&mlm_tz(&db, &["week", WEEK]));
    assert!(!text.contains("[!]"), "{text}");
}

#[test]
fn golden_fixture_stays_clean() {
    let dir = TempDir::new().expect("temp dir");
    let db = dir.path().join("mlm.db");
    assert!(mlm_tz(&db, &["status"]).status.success());
    seed_punch(&db, "2026-01-05T09:00:00Z", "2026-01-05", "start");
    seed_punch(&db, "2026-01-05T17:00:00Z", "2026-01-05", "end");
    seed_punch(&db, "2026-01-06T09:00:00Z", "2026-01-06", "start");
    seed_punch(&db, "2026-01-06T12:30:00Z", "2026-01-06", "end");
    let week = stdout(&mlm_tz(&db, &["week", WEEK]));
    assert!(!week.contains("[!]"), "{week}");
    assert!(week.contains("Worked:        11h 30m"), "{week}");
    let status = stdout(&mlm_tz(&db, &["status", DATE]));
    assert!(!status.contains("[!]"), "{status}");
}

#[test]
fn stale_start_hint_names_the_window() {
    let (_dir, db) = seeded(&STALE);
    let text = stdout(&mlm_tz(&db, &["status", DATE]));
    assert!(
        text.contains(
            "    fix: mlm stop HH:MM --date 2026-01-05  (a time between 09:00 and 10:00)"
        ),
        "{text}"
    );
}

#[test]
fn e7_hint_names_the_window() {
    let (_dir, db) = seeded(&E7);
    let text = stdout(&mlm_tz(&db, &["status", DATE]));
    assert!(
        text.contains("     or: mlm stop HH:MM --date 2026-01-05  (a time between 14:00 and 15:00 closes the 14:00 start)"),
        "{text}"
    );
}

#[test]
fn following_the_stale_hint_clears_the_block() {
    let (_dir, db) = seeded(&STALE);
    let stop = mlm_tz(&db, &["stop", "10:00", "--date", DATE]);
    assert!(stop.status.success(), "{stop:?}");
    let text = stdout(&mlm_tz(&db, &["status", DATE]));
    assert!(!text.contains("overlapping stints"), "{text}");
    assert!(!text.contains("unclosed start"), "{text}");
}

#[test]
fn following_the_e7_hint_creates_no_overlap() {
    let (_dir, db) = seeded(&E7);
    let stop = mlm_tz(&db, &["stop", "15:00", "--date", DATE]);
    assert!(stop.status.success(), "{stop:?}");
    let text = stdout(&mlm_tz(&db, &["status", DATE]));
    assert!(!text.contains("overlapping stints"), "{text}");
}
