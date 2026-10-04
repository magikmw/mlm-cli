//! Binary-level tests for `mlm week` read failures and unchanged
//! success output (SPEC read-error rule: a failed storage read is a hard
//! error, never a silent default).
//!
//! Drives the compiled binary with `MLM_DB_PATH` and `TZ=UTC`, seeding
//! punches by raw insert.

use std::process::{Command, Output};
use std::time::{Duration, Instant};

use rusqlite::Connection;
use tempfile::TempDir;

fn mlm(db_path: &std::path::Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mlm"))
        .args(args)
        .env("MLM_DB_PATH", db_path)
        .env("TZ", "UTC")
        .output()
        .expect("failed to run mlm binary")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// A migrated database holding two days of January 2026 punches.
fn seeded_db() -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("mlm.db");
    drop(mlm::db::connect_at(&path).expect("create db"));
    let conn = Connection::open(&path).expect("open db");
    for (at_utc, date, kind) in [
        ("2026-01-05T09:00:00Z", "2026-01-05", "start"),
        ("2026-01-05T17:00:00Z", "2026-01-05", "end"),
        ("2026-01-06T09:00:00Z", "2026-01-06", "start"),
        ("2026-01-06T12:30:00Z", "2026-01-06", "end"),
    ] {
        conn.execute(
            "INSERT INTO punches (at_utc, \"date\", kind) VALUES (?1, ?2, ?3)",
            (at_utc, date, kind),
        )
        .expect("seed punch");
    }
    (dir, path)
}

fn drop_table(path: &std::path::Path, table: &str) {
    let conn = Connection::open(path).expect("open db");
    conn.execute_batch(&format!("DROP TABLE {table}"))
        .expect("drop table");
}

fn assert_hard_error(out: &Output) {
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(out));
    assert_eq!(stdout(out), "", "stdout must stay empty");
    assert!(
        stderr(out).starts_with("error:"),
        "stderr was: {}",
        stderr(out)
    );
}

// ---------------------------------------------------------------------------
// Golden success output, captured before the loader refactor
// ---------------------------------------------------------------------------

const GOLDEN_2026_02: &str = "\
Week 2026-02 (2026-01-05 - 2026-01-11)

Total behind: 28h 30m

  Mon 2026-01-05   08h 00m
  Tue 2026-01-06   03h 30m
  Wed 2026-01-07   00h 00m
  Thu 2026-01-08   00h 00m
  Fri 2026-01-09   00h 00m
  Sat 2026-01-10   00h 00m
  Sun 2026-01-11   00h 00m

Carry-in:      00h 00m
Worked:        11h 30m
Fulfillment:   11h 30m
Target:        40h 00m
";

const GOLDEN_2026_04: &str = "\
Week 2026-04 (2026-01-19 - 2026-01-25)

Total behind: 108h 30m

  Mon 2026-01-19   00h 00m
  Tue 2026-01-20   00h 00m
  Wed 2026-01-21   00h 00m
  Thu 2026-01-22   00h 00m
  Fri 2026-01-23   00h 00m
  Sat 2026-01-24   00h 00m
  Sun 2026-01-25   00h 00m

Carry-in:      -68h 30m
Worked:        00h 00m
Fulfillment:   -68h 30m
Target:        40h 00m
";

const GOLDEN_2099_01: &str = "\
Week 2099-01 (2098-12-29 - 2099-01-04)

Total behind: 152348h 30m

  Mon 2098-12-29   00h 00m
  Tue 2098-12-30   00h 00m
  Wed 2098-12-31   00h 00m
  Thu 2099-01-01   00h 00m
  Fri 2099-01-02   00h 00m
  Sat 2099-01-03   00h 00m
  Sun 2099-01-04   00h 00m

Carry-in:      -152308h 30m
Worked:        00h 00m
Fulfillment:   -152308h 30m
Target:        40h 00m
";

#[test]
fn golden_data_week_output_is_unchanged() {
    let (_dir, path) = seeded_db();
    let out = mlm(&path, &["week", "2026-02"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(stdout(&out), GOLDEN_2026_02);
}

#[test]
fn golden_later_closed_week_output_is_unchanged() {
    let (_dir, path) = seeded_db();
    let out = mlm(&path, &["week", "2026-04"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(stdout(&out), GOLDEN_2026_04);
}

#[test]
fn golden_future_week_carry_in_output_is_unchanged() {
    let (_dir, path) = seeded_db();
    let out = mlm(&path, &["week", "2099-01"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(stdout(&out), GOLDEN_2099_01);
}

// ---------------------------------------------------------------------------
// Read failures are hard errors
// ---------------------------------------------------------------------------

#[test]
fn week_with_week_targets_table_dropped_is_a_hard_error() {
    let (_dir, path) = seeded_db();
    drop_table(&path, "week_targets");
    assert_hard_error(&mlm(&path, &["week"]));
    assert_hard_error(&mlm(&path, &["week", "2026-41"]));
}

#[test]
fn week_with_notes_table_dropped_is_a_hard_error() {
    let (_dir, path) = seeded_db();
    drop_table(&path, "notes");
    assert_hard_error(&mlm(&path, &["week"]));
    assert_hard_error(&mlm(&path, &["week", "2026-41"]));
}

// ---------------------------------------------------------------------------
// Far-future week id stays cheap
// ---------------------------------------------------------------------------

#[test]
fn far_future_week_is_fast() {
    let (_dir, path) = seeded_db();
    let started = Instant::now();
    let out = mlm(&path, &["week", "9000-01"]);
    let elapsed = started.elapsed();
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert!(stdout(&out).starts_with("Week 9000-01 "));
    assert!(
        elapsed < Duration::from_secs(10),
        "took {elapsed:?}, expected under 10s"
    );
}
