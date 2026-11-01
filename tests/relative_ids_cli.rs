//! Binary-level tests for relative `WEEK_ID` and `MONTH` ids (relative-ids
//! spec §3-§5). Drives the binary with `MLM_DB_PATH` and `TZ=UTC`. Ids that
//! depend on the clock come from the test's own UTC today, and the test
//! reruns when the date rolls over mid-test.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use chrono::{Days, NaiveDate, Utc};
use mlm::date::{WeekId, format_date};
use rusqlite::Connection;
use tempfile::TempDir;

const WEEK_SHAPE: &str = "expected YYYY-WW, WW or -N (N >= 1)";
const MONTH_SHAPE: &str = "expected YYYY-MM, MM or -N (N >= 1)";
const HUGE: &str = "-9999999999999999999999999";

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

/// Seed one note on `date`, created at noon UTC of that date.
fn seed_on(db: &Path, date: NaiveDate, body: &str) {
    let day = format_date(date);
    let conn = Connection::open(db).expect("open db");
    conn.execute(
        "INSERT INTO notes (\"date\", body, created_at_utc) VALUES (?1, ?2, ?3)",
        (&day, body, format!("{day}T12:00:00Z")),
    )
    .expect("seed note");
}

fn target_rows(db: &Path) -> Vec<(String, i64)> {
    let conn = Connection::open(db).expect("open db");
    let mut stmt = conn
        .prepare("SELECT week_id, target_minutes FROM week_targets ORDER BY week_id")
        .expect("prepare");
    stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("rows")
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

fn last_week(today: NaiveDate) -> WeekId {
    WeekId::from_date(today - Days::new(7))
}

// --- shape and offset errors ----------------------------------------------

#[test]
fn minus_zero_is_a_shape_error() {
    let (_d, db) = fresh();
    let week = format!("error: invalid WEEK_ID \"-0\": {WEEK_SHAPE}\n");
    fails(&db, &["week", "-0"], 1, &week);
    fails(&db, &["ex", "n", "-w", "-0"], 1, &week);
    fails(
        &db,
        &["ex", "n", "-m", "-0"],
        1,
        &format!("error: invalid MONTH \"-0\": {MONTH_SHAPE}\n"),
    );
}

#[test]
fn huge_offsets_are_out_of_range() {
    let (_d, db) = fresh();
    let week = format!("error: invalid WEEK_ID \"{HUGE}\": week offset is out of range\n");
    fails(&db, &["week", HUGE], 1, &week);
    fails(&db, &["ex", "n", "-w", HUGE], 1, &week);
    fails(
        &db,
        &["ex", "n", "-m", HUGE],
        1,
        &format!("error: invalid MONTH \"{HUGE}\": month offset is out of range\n"),
    );
    fails(&db, &["week", "target", HUGE, "35h"], 1, &week);
    assert!(target_rows(&db).is_empty());
}

#[test]
fn plus_digits_get_the_future_hint() {
    let (_d, db) = fresh();
    let week = |input: &str| {
        format!(
            "error: invalid WEEK_ID \"{input}\": {WEEK_SHAPE}; a future week takes YYYY-WW or WW\n"
        )
    };
    fails(&db, &["week", "+1"], 1, &week("+1"));
    fails(&db, &["week", "target", "+1", "35h"], 1, &week("+1"));
    fails(&db, &["ex", "n", "-w", "+1"], 1, &week("+1"));
    fails(&db, &["week", "+0"], 1, &week("+0"));
    fails(
        &db,
        &["ex", "n", "-m", "+1"],
        1,
        &format!(
            "error: invalid MONTH \"+1\": {MONTH_SHAPE}; a future month takes YYYY-MM or MM\n"
        ),
    );
    fails(
        &db,
        &["ex", "n", "-w", "+1x"],
        1,
        &format!("error: invalid WEEK_ID \"+1x\": {WEEK_SHAPE}\n"),
    );
    assert!(target_rows(&db).is_empty());
}

#[test]
fn hyphen_junk_after_target_and_export_flags() {
    let (_d, db) = fresh();
    let week = format!("error: invalid WEEK_ID \"-1x\": {WEEK_SHAPE}\n");
    fails(&db, &["week", "target", "-1x", "35h"], 1, &week);
    fails(&db, &["ex", "n", "-w", "-1x"], 1, &week);
    fails(
        &db,
        &["ex", "n", "-m", "-1x"],
        1,
        &format!("error: invalid MONTH \"-1x\": {MONTH_SHAPE}\n"),
    );
    assert!(target_rows(&db).is_empty());
}

#[test]
fn date_plus_one_has_no_hint() {
    let (_d, db) = fresh();
    fails(
        &db,
        &["status", "+1"],
        1,
        "error: invalid DATE \"+1\": expected YYYY-MM-DD or -N (N >= 1)\n",
    );
}

// --- mlm week -N ------------------------------------------------------------

#[test]
fn week_minus_one_shows_last_week() {
    on_stable_today(|today| {
        let (_d, db) = fresh();
        let last = last_week(today);
        let out = ok_stdout(&db, &["week", "-1"]);
        assert!(out.starts_with(&format!("Week {last} (")), "{out}");
        assert_eq!(out, ok_stdout(&db, &["week", &last.to_string()]));
        assert_eq!(out, ok_stdout(&db, &["week", "-01"]));
        assert_eq!(out, ok_stdout(&db, &["w", "-1"]));
    });
}

#[test]
fn week_minus_one_with_notes_in_either_order() {
    on_stable_today(|today| {
        let (_d, db) = fresh();
        let last = last_week(today);
        seed_on(&db, last.start(), "last week");
        seed_on(&db, today, "this week");
        let a = ok_stdout(&db, &["week", "-1", "-n"]);
        let b = ok_stdout(&db, &["week", "-n", "-1"]);
        let c = ok_stdout(&db, &["week", &last.to_string(), "-n"]);
        assert_eq!(a, b);
        assert_eq!(a, c);
        assert!(a.contains("      - last week\n"), "{a}");
        assert!(!a.contains("this week"), "{a}");
    });
}

#[test]
fn week_hyphen_junk_is_a_usage_error() {
    for tok in ["-1x", "-x", "--bogus"] {
        let dir = TempDir::new().expect("temp dir");
        let db = dir.path().join("never-created.db");
        let o = mlm(&db, &["week", tok]);
        assert_eq!(o.status.code(), Some(2), "{tok}: {}", stderr(&o));
        assert_eq!(stdout(&o), "", "{tok}");
        let e = stderr(&o);
        assert!(e.starts_with("error: unexpected argument '"), "{tok}: {e}");
        // Windows prints the binary as mlm.exe in usage lines.
        let usage = e.replace("mlm.exe ", "mlm ");
        assert!(usage.contains("Usage: mlm week"), "{tok}: {e}");
        assert!(!db.exists(), "{tok}");
    }
}

#[test]
fn week_float_shapes_reach_the_parser() {
    let (_d, db) = fresh();
    for tok in ["-1.5", "-1e3"] {
        fails(
            &db,
            &["week", tok],
            1,
            &format!("error: invalid WEEK_ID \"{tok}\": {WEEK_SHAPE}\n"),
        );
    }
}

// --- mlm week target -N -----------------------------------------------------

#[test]
fn week_target_minus_one_writes_last_week() {
    on_stable_today(|today| {
        let (_d, db) = fresh();
        let this = WeekId::current(today);
        let last = last_week(today);
        assert_eq!(
            ok_stdout(&db, &["week", "target", "20h"]),
            format!("Week {this} target: 20h 00m\n")
        );
        assert_eq!(
            ok_stdout(&db, &["week", "target", "-1", "35h"]),
            format!("Week {last} target: 35h 00m\n")
        );
        assert_eq!(
            target_rows(&db),
            vec![(last.to_string(), 2100), (this.to_string(), 1200)]
        );
    });
}

#[test]
fn swapped_relative_week_gets_the_reminder() {
    let (_d, db) = fresh();
    for cmd in ["week", "w"] {
        fails(
            &db,
            &[cmd, "-1", "target", "35h"],
            2,
            "error: put WEEK_ID after target: mlm week target -1 35h\n",
        );
    }
}

#[test]
fn lone_week_id_after_target_is_a_missing_duration() {
    let (_d, db) = fresh();
    let bare = mlm(&db, &["week", "target"]);
    assert_eq!(bare.status.code(), Some(2));
    let bare = stderr(&bare);
    for args in [
        &["week", "target", "-1"][..],
        &["week", "target", "7"],
        &["week", "target", "2026-07"],
        &["w", "target", "-1"],
        &["-v", "week", "target", "-1"],
        &["week", "target", "-v", "7"],
        &["week", "target", "--", "-1"],
    ] {
        fails(&db, args, 2, &bare);
        assert!(target_rows(&db).is_empty(), "{args:?}");
    }
    on_stable_today(|today| {
        let (_d, db) = fresh();
        assert_eq!(
            ok_stdout(&db, &["week", "target", "35h"]),
            format!("Week {} target: 35h 00m\n", WeekId::current(today))
        );
    });
}

#[test]
fn lone_non_week_tokens_keep_invalid_duration() {
    let (_d, db) = fresh();
    for tok in ["-0", "-99999999999999999999"] {
        fails(
            &db,
            &["week", "target", tok],
            1,
            &format!(
                "error: invalid duration \"{tok}\": expected a form like 20h, 33h30m, or 45m\n"
            ),
        );
    }
    assert!(target_rows(&db).is_empty());
}
