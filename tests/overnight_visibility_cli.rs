//! Binary-level tests for the overnight stint: the `status` line and the
//! `mlm week` row marker and footer.
//!
//! Each run uses `TZ=UTC` and the real clock. A run that straddles UTC
//! midnight can flip a date, as in `start_hint_cli.rs`.

use std::path::Path;
use std::process::{Command, Output};

use chrono::{Datelike, Duration, NaiveDate, Utc, Weekday};
use mlm::date::{WeekId, format_date, format_date_with_weekday};
use rusqlite::Connection;
use tempfile::TempDir;

const WEEK_FOOTER: &str = "[i] open stint: run mlm status to stop it";
const STATUS_LINE_TAIL: &str = "started yesterday, not counted until stopped";

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

fn today() -> NaiveDate {
    Utc::now().date_naive()
}

fn yesterday() -> NaiveDate {
    today() - Duration::days(1)
}

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

type Seeder = Box<dyn Fn(&Path)>;

fn status_fixtures() -> Vec<(&'static str, Seeder, bool)> {
    vec![
        (
            "lone start yesterday",
            Box::new(|db| seed(db, yesterday(), "00:00", "start")),
            true,
        ),
        (
            "lone start plus own punch",
            Box::new(|db| {
                seed(db, yesterday(), "00:00", "start");
                seed(db, today(), "00:00", "start");
            }),
            true,
        ),
        (
            "joined",
            Box::new(|db| {
                seed(db, yesterday(), "23:00", "start");
                seed(db, today(), "00:10", "end");
            }),
            false,
        ),
        (
            "refused join",
            Box::new(|db| {
                seed(db, yesterday(), "18:02", "start");
                seed(db, today(), "00:10", "end");
                seed(db, today(), "00:20", "end");
            }),
            false,
        ),
        (
            "two starts yesterday",
            Box::new(|db| {
                seed(db, yesterday(), "09:00", "start");
                seed(db, yesterday(), "10:00", "start");
            }),
            false,
        ),
        (
            "start two days ago",
            Box::new(|db| seed(db, today() - Duration::days(2), "09:00", "start")),
            false,
        ),
    ]
}

#[test]
fn hint_and_line_appear_together() {
    for (name, seeder, expected) in status_fixtures() {
        let (_dir, db) = boot();
        seeder(&db);
        let text = stdout(&mlm_utc(&db, &["status"]));
        let hint = text.lines().any(|l| l.starts_with("[i] start on"));
        let line = text.lines().any(|l| l.contains(STATUS_LINE_TAIL));
        assert_eq!(hint, line, "{name}: {text}");
        assert_eq!(line, expected, "{name}: {text}");
    }
}

#[test]
fn status_for_other_dates_has_no_stint_line() {
    let (_dir, db) = boot();
    seed(&db, yesterday(), "18:02", "start");
    for arg in ["-1".to_string(), format_date(yesterday())] {
        let text = stdout(&mlm_utc(&db, &["status", &arg]));
        assert!(!text.contains("started yesterday"), "status {arg}: {text}");
    }
}

#[test]
fn week_by_id_marks_yesterday_row() {
    let (_dir, db) = boot();
    seed(&db, yesterday(), "18:02", "start");
    let week = WeekId::from_date(yesterday()).to_string();

    let text = stdout(&mlm_utc(&db, &["week", &week]));
    let prefix = format!("  {}", format_date_with_weekday(yesterday()));
    let row = text
        .lines()
        .find(|l| l.starts_with(&prefix))
        .expect("yesterday row");
    assert!(row.ends_with("  [i]"), "{row}");
    let marked = text
        .lines()
        .filter(|l| l.starts_with("  ") && l.contains("[i]"))
        .count();
    assert_eq!(marked, 1, "{text}");
    let footers = text.lines().filter(|l| *l == WEEK_FOOTER).count();
    assert_eq!(footers, 1, "{text}");
}

#[test]
fn week_without_id_follows_the_weekday() {
    let (_dir, db) = boot();
    seed(&db, yesterday(), "18:02", "start");

    let text = stdout(&mlm_utc(&db, &["week"]));
    if today().weekday() == Weekday::Mon {
        let monday = format!(
            "[i] open stint on {} (last week): run mlm status to stop it",
            format_date_with_weekday(yesterday())
        );
        assert_eq!(text.lines().filter(|l| *l == monday).count(), 1, "{text}");
        assert!(!text.contains("  [i]"), "{text}");
    } else {
        assert_eq!(
            text.lines().filter(|l| *l == WEEK_FOOTER).count(),
            1,
            "{text}"
        );
        let marked = text
            .lines()
            .filter(|l| l.starts_with("  ") && l.ends_with("  [i]"))
            .count();
        assert_eq!(marked, 1, "{text}");
    }
}

#[test]
fn overnight_output_is_ascii() {
    let (_dir, db) = boot();
    seed(&db, yesterday(), "18:02", "start");
    let week = WeekId::from_date(yesterday()).to_string();
    assert!(stdout(&mlm_utc(&db, &["status"])).is_ascii());
    assert!(stdout(&mlm_utc(&db, &["week", &week])).is_ascii());
    assert!(stdout(&mlm_utc(&db, &["week"])).is_ascii());
}
