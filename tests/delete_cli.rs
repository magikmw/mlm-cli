//! Binary-level tests for `mlm delete note|punch` (SPEC §3.9, §7.4).
//! Drives the binary with `MLM_DB_PATH` and `TZ=UTC`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use chrono::{NaiveDate, Utc};
use mlm::date::format_date;
use tempfile::TempDir;

const BACKSLASH_BODY: &str = r"path C:\dir\file";

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

fn fresh() -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("mlm.db");
    (dir, path)
}

/// Exit `0`, empty stderr; returns stdout.
fn ok_stdout(db: &Path, args: &[&str]) -> String {
    let o = mlm(db, args);
    assert_eq!(o.status.code(), Some(0), "{args:?}: {}", stderr(&o));
    assert_eq!(stderr(&o), "", "{args:?}");
    stdout(&o)
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

// --- backslash note body (SPEC §7.4) --------------------------------------

#[test]
fn backslash_note_delete_today_prints_plain_line_with_date() {
    on_stable_today(|today| {
        let (_dir, db) = fresh();
        ok_stdout(&db, &["note", BACKSLASH_BODY]);
        let out = ok_stdout(&db, &["delete", "note", "1"]);
        assert_eq!(
            out,
            format!("deleted note ({}): {BACKSLASH_BODY}\n", format_date(today))
        );
    });
}

#[test]
fn backslash_note_delete_other_day_prints_plain_line_with_date() {
    let (_dir, db) = fresh();
    ok_stdout(&db, &["note", "--date", "2026-02-10", BACKSLASH_BODY]);
    let out = ok_stdout(&db, &["delete", "note", "1", "--date", "2026-02-10"]);
    assert_eq!(
        out,
        format!("deleted note (2026-02-10): {BACKSLASH_BODY}\n")
    );
}

// --- `[ID]` help (SPEC §3.9) ----------------------------------------------

#[test]
fn delete_id_help_says_fresh_list_and_rejection() {
    let want = "Position in this date's list, numbered from 1, taken fresh when this \
                runs (run with no ID to see it). 0 or a number past the list is rejected";
    for kind in ["note", "punch"] {
        for flag in ["-h", "--help"] {
            let (_dir, db) = fresh();
            let s = ok_stdout(&db, &["delete", kind, flag]);
            let flat = s.split_whitespace().collect::<Vec<_>>().join(" ");
            assert!(flat.contains(want), "{kind} {flag}:\n{s}");
            assert!(!flat.contains("not here"), "{kind} {flag}:\n{s}");
        }
    }
}
