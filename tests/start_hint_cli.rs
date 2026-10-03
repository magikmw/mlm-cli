//! Binary-level tests for the yesterday hint on today's `status`, the
//! age-1 lookback line and the `mlm start` warning (SPEC §3.1, §3.2.2, §7).
//!
//! Each run uses `TZ=UTC`, so local dates and UTC dates agree and dates
//! derived from `Utc::now()` match the child's own clock.

use std::path::Path;
use std::process::{Command, Output};

use chrono::{Duration, NaiveDate, Utc};
use mlm::date::{WeekId, format_date, format_date_with_weekday};
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

fn yesterday() -> NaiveDate {
    today() - Duration::days(1)
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

fn yesterday_hint_line(hhmm: &str) -> String {
    format!(
        "[i] start on {} at {hhmm} still open (yesterday; not counted until stopped)",
        format_date_with_weekday(yesterday())
    )
}

fn fix_line(window: &str) -> String {
    format!(
        "    fix: mlm stop HH:MM --date {}  ({window})",
        format_date(yesterday())
    )
}

const OR_LINE: &str = "     or: mlm stop  (still working: ends it now, across midnight)";

#[test]
fn status_hint_with_or_line_when_today_is_empty() {
    let (_dir, db) = boot();
    seed(&db, yesterday(), "18:02", "start");

    let out = mlm_utc(&db, &["status"]);
    assert!(out.status.success());
    let text = stdout(&out);
    for line in [
        yesterday_hint_line("18:02"),
        fix_line("a time after 18:02"),
        OR_LINE.to_string(),
    ] {
        assert!(
            text.lines().any(|l| l == line),
            "missing {line:?} in {text}"
        );
    }
    assert!(text.is_ascii());
}

#[test]
fn status_hint_has_no_or_line_when_today_has_a_start() {
    let (_dir, db) = boot();
    seed(&db, yesterday(), "18:02", "start");
    seed(&db, today(), "00:00", "start");

    let text = stdout(&mlm_utc(&db, &["status"]));
    assert!(text.contains(&yesterday_hint_line("18:02")), "{text}");
    assert!(text.contains(&fix_line("a time after 18:02")), "{text}");
    assert!(!text.contains(" or: mlm stop"), "{text}");
}

#[test]
fn stop_today_joins_and_clears_the_hint() {
    let (_dir, db) = boot();
    seed(&db, yesterday(), "23:00", "start");
    assert!(stdout(&mlm_utc(&db, &["status"])).contains("[i] start on"));

    let stop = mlm_utc(&db, &["stop"]);
    assert!(stop.status.success(), "{stop:?}");
    let text = stdout(&mlm_utc(&db, &["status"]));
    assert!(!text.contains("[i] "), "{text}");
    assert!(!text.contains("[!]"), "{text}");
}

#[test]
fn stopping_yesterday_at_the_hinted_time_clears_the_hint() {
    let (_dir, db) = boot();
    seed(&db, yesterday(), "18:02", "start");
    let date = format_date(yesterday());

    let stop = mlm_utc(&db, &["stop", "19:00", "--date", &date]);
    assert!(stop.status.success(), "{stop:?}");
    let text = stdout(&mlm_utc(&db, &["status"]));
    assert!(!text.contains("[i] "), "{text}");
}

#[test]
fn refused_join_suppresses_the_hint() {
    let (_dir, db) = boot();
    seed(&db, yesterday(), "18:02", "start");
    seed(&db, today(), "00:10", "end");
    seed(&db, today(), "00:20", "end");

    let text = stdout(&mlm_utc(&db, &["status"]));
    assert!(text.contains("not joined to"), "{text}");
    assert!(!text.contains("[i] "), "{text}");
}

#[test]
fn two_open_starts_yesterday_print_the_yesterday_e7_line() {
    let (_dir, db) = boot();
    seed(&db, yesterday(), "09:00", "start");
    seed(&db, yesterday(), "10:00", "start");

    let text = stdout(&mlm_utc(&db, &["status"]));
    let expected = format!(
        "[!] 2 open stints on {} (yesterday)",
        format_date_with_weekday(yesterday())
    );
    assert!(text.lines().any(|l| l == expected), "{text}");
    assert!(
        text.contains(&format!(
            "    fix: see mlm status {}",
            format_date(yesterday())
        )),
        "{text}"
    );
    assert!(!text.contains("[i] "), "{text}");
}

#[test]
fn hint_does_not_mark_week_row_or_footer() {
    let (_dir, db) = boot();
    seed(&db, yesterday(), "18:02", "start");
    let week = WeekId::from_date(yesterday()).to_string();

    let text = stdout(&mlm_utc(&db, &["week", &week]));
    assert!(!text.contains("[!]"), "{text}");
    assert!(!text.contains("needs a look"), "{text}");
}

#[test]
fn hint_never_shows_on_status_for_another_date() {
    let (_dir, db) = boot();
    seed(&db, yesterday(), "18:02", "start");

    for arg in ["-1".to_string(), format_date(yesterday())] {
        let text = stdout(&mlm_utc(&db, &["status", &arg]));
        assert!(!text.contains("[i] "), "status {arg}: {text}");
    }
    let text = stdout(&mlm_utc(&db, &["status", &format_date(today())]));
    assert!(text.contains("[i] start on"), "{text}");
}

#[test]
fn start_warns_about_yesterdays_open_start_on_stderr() {
    let (_dir, db) = boot();
    seed(&db, yesterday(), "18:02", "start");

    let out = mlm_utc(&db, &["start", "00:00"]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(stdout(&out), "");
    let expected = format!(
        "warning: start on {} at 18:02 still open (yesterday)\n    fix: mlm stop HH:MM --date {}  (a time after 18:02)\n",
        format_date_with_weekday(yesterday()),
        format_date(yesterday())
    );
    assert_eq!(stderr(&out), expected);
}

#[test]
fn start_into_empty_yesterday_does_not_warn_about_itself() {
    let (_dir, db) = boot();
    let date = format_date(yesterday());

    let out = mlm_utc(&db, &["start", "18:02", "--date", &date]);
    assert!(out.status.success(), "{out:?}");
    assert!(!stderr(&out).contains("warning:"), "{}", stderr(&out));
}

#[test]
fn follow_the_hint_round_trip() {
    let (_dir, db) = boot();
    seed(&db, yesterday(), "09:00", "start");
    seed(&db, yesterday(), "10:00", "start");
    seed(&db, yesterday(), "10:30", "end");

    let first = stdout(&mlm_utc(&db, &["status"]));
    assert!(
        first.contains("(a time between 09:00 and 10:00)"),
        "{first}"
    );

    let date = format_date(yesterday());
    let stop = mlm_utc(&db, &["stop", "09:30", "--date", &date]);
    assert!(stop.status.success(), "{stop:?}");

    let after = stdout(&mlm_utc(&db, &["status"]));
    assert!(!after.contains("[i] "), "{after}");
    assert!(!after.contains("[!]"), "{after}");
    let that_day = stdout(&mlm_utc(&db, &["status", &date]));
    assert!(!that_day.contains("[!]"), "{that_day}");
}
