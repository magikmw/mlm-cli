//! Binary-level tests for the carry-in anomaly marker (SPEC §3.3).
//!
//! Fixtures sit in January/February 2026, so every fixture date is
//! stale or otherwise anomalous by the time the tests run. `TZ=UTC`
//! makes the seeded `at_utc` equal the wall clock.

use std::path::Path;
use std::process::{Command, Output};

use rusqlite::Connection;
use tempfile::TempDir;

const MARKER_PREFIX: &str = "[!] carry-in includes anomalies";
const ROW_FOOTER: &str = "[!] needs a look: run mlm status DATE for details and fixes";

fn mlm_tz(db: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mlm"))
        .args(args)
        .env("MLM_DB_PATH", db)
        .env("TZ", "UTC")
        .output()
        .expect("failed to run mlm binary")
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn seed(db: &Path, date: &str, hhmm: &str, kind: &str) {
    let conn = Connection::open(db).expect("open db");
    conn.execute(
        "INSERT INTO punches (at_utc, \"date\", kind) VALUES (?1, ?2, ?3)",
        (format!("{date}T{hhmm}:00Z"), date, kind),
    )
    .expect("seed punch");
}

/// A fresh database with the schema applied.
fn fresh() -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().expect("tempdir");
    let db = dir.path().join("mlm.db");
    mlm_tz(&db, &["week", "2026-02"]);
    (dir, db)
}

/// Week 2026-02, date 01-06.
fn seed_e7(db: &Path) {
    seed(db, "2026-01-06", "09:00", "start");
    seed(db, "2026-01-06", "11:00", "start");
}

/// Week 2026-03, date 01-13.
fn seed_stale(db: &Path) {
    seed(db, "2026-01-13", "09:00", "start");
}

/// Week 2026-04, date 01-20.
fn seed_e8(db: &Path) {
    seed(db, "2026-01-20", "10:00", "end");
}

/// Week 2026-05, dates 01-27 and 01-28.
fn seed_refused_join(db: &Path) {
    seed(db, "2026-01-27", "09:00", "start");
    seed(db, "2026-01-28", "08:00", "end");
    seed(db, "2026-01-28", "09:00", "end");
}

fn seed_clean(db: &Path, date: &str) {
    seed(db, date, "09:00", "start");
    seed(db, date, "17:00", "end");
}

fn marker_line(output: &str) -> Option<String> {
    output
        .lines()
        .find(|l| l.starts_with(MARKER_PREFIX))
        .map(str::to_string)
}

fn singular(week: &str) -> String {
    format!("{MARKER_PREFIX} from 1 earlier week, {week} (see mlm week {week})")
}

fn plural(count: usize, oldest: &str) -> String {
    format!("{MARKER_PREFIX} from {count} earlier weeks, oldest {oldest} (see mlm week {oldest})")
}

fn week_marker(db: &Path, arg: &str) -> Option<String> {
    marker_line(&out(&mlm_tz(db, &["week", arg])))
}

fn row_flagged(output: &str) -> bool {
    output
        .lines()
        .any(|l| l.ends_with("  [!]") && !l.starts_with("Carry-in:"))
}

#[test]
fn old_anomaly_raises_marker_on_status_and_week() {
    let (_d, db) = fresh();
    seed_e7(&db);
    let expected = singular("2026-02");
    assert_eq!(
        marker_line(&out(&mlm_tz(&db, &["status"]))),
        Some(expected.clone())
    );
    assert_eq!(marker_line(&out(&mlm_tz(&db, &["week"]))), Some(expected));
}

#[test]
fn status_past_date_shows_marker() {
    let (_d, db) = fresh();
    seed_e7(&db);
    let got = marker_line(&out(&mlm_tz(&db, &["status", "2026-01-20"])));
    assert_eq!(got, Some(singular("2026-02")));
}

#[test]
fn clean_history_has_no_marker() {
    let (_d, db) = fresh();
    seed_clean(&db, "2026-01-06");
    assert_eq!(marker_line(&out(&mlm_tz(&db, &["status"]))), None);
    assert_eq!(week_marker(&db, "2026-10"), None);
}

#[test]
fn notes_only_earlier_week_has_no_marker() {
    let (_d, db) = fresh();
    let conn = Connection::open(&db).unwrap();
    conn.execute(
        "INSERT INTO notes (\"date\", body, created_at_utc) VALUES ('2026-01-05', 'x', '2026-01-05T09:00:00Z')",
        (),
    )
    .unwrap();
    let o = out(&mlm_tz(&db, &["week", "2026-10"]));
    assert_eq!(marker_line(&o), None);
    assert!(!o.contains("[!]"), "{o}");
}

#[test]
fn no_marker_means_no_flag_and_unchanged_layout() {
    let (_d, db) = fresh();
    seed_clean(&db, "2026-01-06");
    let o = out(&mlm_tz(&db, &["week", "2026-10"]));
    assert!(!o.contains("[!]"), "{o}");
    let lines: Vec<&str> = o.lines().collect();
    let carry = lines
        .iter()
        .position(|l| l.starts_with("Carry-in:"))
        .unwrap();
    assert_eq!(lines[carry - 1], "");
}

#[test]
fn two_anomalous_weeks_use_plural_form() {
    let (_d, db) = fresh();
    seed_e7(&db);
    seed_stale(&db);
    assert_eq!(week_marker(&db, "2026-10"), Some(plural(2, "2026-02")));
}

#[test]
fn one_anomalous_week_uses_singular_form() {
    let (_d, db) = fresh();
    seed_stale(&db);
    assert_eq!(week_marker(&db, "2026-10"), Some(singular("2026-03")));
}

#[test]
fn kind_e7_names_its_week() {
    let (_d, db) = fresh();
    seed_e7(&db);
    assert_eq!(week_marker(&db, "2026-10"), Some(singular("2026-02")));
}

#[test]
fn kind_stale_names_its_week() {
    let (_d, db) = fresh();
    seed_stale(&db);
    assert_eq!(week_marker(&db, "2026-10"), Some(singular("2026-03")));
}

#[test]
fn kind_e8_names_its_week() {
    let (_d, db) = fresh();
    seed_e8(&db);
    assert_eq!(week_marker(&db, "2026-10"), Some(singular("2026-04")));
}

#[test]
fn kind_refused_join_names_its_week() {
    let (_d, db) = fresh();
    seed_refused_join(&db);
    assert_eq!(week_marker(&db, "2026-10"), Some(singular("2026-05")));
}

#[test]
fn marker_weeks_each_have_a_marked_row() {
    let (_d, db) = fresh();
    seed_e7(&db);
    seed_stale(&db);
    seed_e8(&db);
    seed_refused_join(&db);
    for id in ["2026-02", "2026-03", "2026-04", "2026-05"] {
        let o = out(&mlm_tz(&db, &["week", id]));
        assert!(row_flagged(&o), "{id}: {o}");
        assert_eq!(o.matches(ROW_FOOTER).count(), 1, "{id}: {o}");
    }
    assert_eq!(week_marker(&db, "2026-10"), Some(plural(4, "2026-02")));
}

#[test]
fn clean_weeks_are_not_counted() {
    let (_d, db) = fresh();
    seed_e7(&db);
    seed_clean(&db, "2026-01-13");
    seed_e8(&db);
    assert!(!row_flagged(&out(&mlm_tz(&db, &["week", "2026-03"]))));
    assert_eq!(week_marker(&db, "2026-10"), Some(plural(2, "2026-02")));
}

#[test]
fn marker_shown_on_current_week_and_future_week() {
    let (_d, db) = fresh();
    seed_e7(&db);
    let expected = Some(singular("2026-02"));
    assert_eq!(marker_line(&out(&mlm_tz(&db, &["week"]))), expected);
    assert_eq!(week_marker(&db, "2099-01"), expected);
    assert_eq!(marker_line(&out(&mlm_tz(&db, &["status"]))), expected);
}

#[test]
fn shown_week_own_anomaly_not_counted() {
    let (_d, db) = fresh();
    seed_e7(&db);
    let o = out(&mlm_tz(&db, &["week", "2026-02"]));
    assert!(row_flagged(&o), "{o}");
    assert_eq!(o.matches(ROW_FOOTER).count(), 1, "{o}");
    assert_eq!(marker_line(&o), None);
    assert_eq!(
        marker_line(&out(&mlm_tz(&db, &["status", "2026-01-06"]))),
        None
    );
}

#[test]
fn later_anomaly_not_counted_for_earlier_week() {
    let (_d, db) = fresh();
    seed_e7(&db);
    seed_e8(&db);
    assert_eq!(week_marker(&db, "2026-03"), Some(singular("2026-02")));
    assert_eq!(week_marker(&db, "2026-04"), Some(singular("2026-02")));
}

#[test]
fn week_before_all_data_has_no_marker() {
    let (_d, db) = fresh();
    seed_e7(&db);
    assert_eq!(week_marker(&db, "2025-50"), None);
}

#[test]
fn status_and_week_print_the_same_marker_line() {
    let (_d, db) = fresh();
    seed_e7(&db);
    seed_stale(&db);
    let week = week_marker(&db, "2026-10");
    let status = marker_line(&out(&mlm_tz(&db, &["status", "2026-03-02"])));
    assert!(week.is_some());
    assert_eq!(week, status);
    let cur_week = marker_line(&out(&mlm_tz(&db, &["week"])));
    let cur_status = marker_line(&out(&mlm_tz(&db, &["status"])));
    assert_eq!(cur_week, cur_status);
}

#[test]
fn week_layout_with_and_without_a_marked_row() {
    let (_d, db) = fresh();
    seed_e7(&db);
    seed(&db, "2026-03-03", "09:00", "start");
    seed(&db, "2026-03-03", "11:00", "start");

    let marked = out(&mlm_tz(&db, &["week", "2026-10"]));
    let l: Vec<&str> = marked.lines().collect();
    let m = l.iter().position(|x| x.starts_with(MARKER_PREFIX)).unwrap();
    assert_eq!(l[m - 1], ROW_FOOTER);
    assert_eq!(l[m + 1], "");
    assert!(l[m + 2].starts_with("Carry-in:") && l[m + 2].ends_with("  [!]"));

    let unmarked = out(&mlm_tz(&db, &["week", "2026-11"]));
    let l: Vec<&str> = unmarked.lines().collect();
    let m = l.iter().position(|x| x.starts_with(MARKER_PREFIX)).unwrap();
    assert_eq!(l[m - 1], "");
    assert_eq!(l[m + 1], "");
    assert!(l[m + 2].starts_with("Carry-in:") && l[m + 2].ends_with("  [!]"));
}

#[test]
fn status_marker_follows_date_block() {
    let (_d, db) = fresh();
    seed_e7(&db);
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    seed(&db, &today, "00:00", "start");
    seed(&db, &today, "00:01", "start");
    let o = out(&mlm_tz(&db, &["status"]));
    let lines: Vec<&str> = o.lines().collect();
    let m = lines
        .iter()
        .position(|x| x.starts_with(MARKER_PREFIX))
        .unwrap();
    let last_fix = lines
        .iter()
        .rposition(|x| x.trim_start().starts_with("fix:"))
        .unwrap();
    assert!(last_fix < m, "{o}");
}
