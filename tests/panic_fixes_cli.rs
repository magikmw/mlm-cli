//! panic-fixes end-to-end: the `week target` 168h cap (spec §4), stdout
//! write failures (spec §5) and the repeated local hour at `now` capture
//! (spec §3, §6), all through the compiled binary.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

fn mlm(db: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_mlm"));
    cmd.args(args)
        .env("MLM_DB_PATH", db)
        .env("TZ", "UTC")
        .env_remove("RUST_LOG");
    cmd
}

fn run(db: &Path, args: &[&str]) -> Output {
    mlm(db, args).output().expect("failed to run mlm binary")
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

fn fresh() -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("temp dir");
    let db = dir.path().join("mlm.db");
    (dir, db)
}

fn open(db: &Path) -> rusqlite::Connection {
    rusqlite::Connection::open(db).expect("open test db")
}

fn target_rows(db: &Path) -> Vec<(String, i64)> {
    let conn = open(db);
    let mut stmt = conn
        .prepare("SELECT week_id, target_minutes FROM week_targets ORDER BY week_id")
        .expect("prepare");
    stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("rows")
}

// ------------------------------------------------------------ cap (§4)

#[test]
fn cap_boundary_is_accepted() {
    for dur in ["168h", "167h60m", "10080m"] {
        let (_dir, db) = fresh();
        let o = run(&db, &["week", "target", "2026-41", dur]);
        assert_eq!(o.status.code(), Some(0), "{dur}: {o:?}");
        assert_eq!(out(&o), "Week 2026-41 target: 168h 00m\n", "{dur}");
        assert_eq!(err(&o), "", "{dur}");
        assert_eq!(target_rows(&db), [("2026-41".to_string(), 10080)], "{dur}");
    }
}

#[test]
fn over_cap_is_rejected_and_writes_nothing() {
    let cases = [
        (
            "168h1m",
            "error: target cannot exceed 168h 00m (got 168h 01m)\n",
        ),
        (
            "10081m",
            "error: target cannot exceed 168h 00m (got 168h 01m)\n",
        ),
        (
            "200h",
            "error: target cannot exceed 168h 00m (got 200h 00m)\n",
        ),
        (
            "99999999999h",
            "error: target cannot exceed 168h 00m (got 99999999999h 00m)\n",
        ),
    ];
    for (dur, stderr) in cases {
        let (_dir, db) = fresh();
        let o = run(&db, &["week", "target", "2026-41", dur]);
        assert_eq!(o.status.code(), Some(1), "{dur}: {o:?}");
        assert_eq!(out(&o), "", "{dur}");
        assert_eq!(err(&o), stderr, "{dur}");
        assert!(target_rows(&db).is_empty(), "{dur}");
    }
}

#[test]
fn over_cap_keeps_the_existing_row() {
    let (_dir, db) = fresh();
    let set = run(&db, &["week", "target", "2026-41", "40h"]);
    assert_eq!(set.status.code(), Some(0), "{set:?}");
    let o = run(&db, &["week", "target", "2026-41", "200h"]);
    assert_eq!(o.status.code(), Some(1), "{o:?}");
    assert_eq!(target_rows(&db), [("2026-41".to_string(), 2400)]);
}

#[test]
fn parse_errors_keep_their_messages() {
    let cases = [
        (
            "-5h",
            "error: duration cannot be negative (got -300 minutes)\n",
        ),
        (
            "10x",
            "error: invalid duration \"10x\": expected a form like 20h, 33h30m, or 45m\n",
        ),
        (
            "9999999999999999999h",
            "error: invalid duration \"9999999999999999999h\": expected a form like 20h, 33h30m, or 45m\n",
        ),
    ];
    for (dur, stderr) in cases {
        let (_dir, db) = fresh();
        let o = run(&db, &["week", "target", "2026-41", dur]);
        assert_eq!(o.status.code(), Some(1), "{dur}: {o:?}");
        assert_eq!(out(&o), "", "{dur}");
        assert_eq!(err(&o), stderr, "{dur}");
        assert!(target_rows(&db).is_empty(), "{dur}");
    }
}

#[test]
fn swapped_target_over_cap_gets_the_reminder() {
    let (_dir, db) = fresh();
    let o = run(&db, &["week", "2026-41", "target", "200h"]);
    assert_eq!(o.status.code(), Some(2), "{o:?}");
    assert_eq!(out(&o), "");
    assert_eq!(
        err(&o),
        "error: put WEEK_ID after target: mlm week target 2026-41 200h\n"
    );
}

/// The `Day total:` line of `status` after a 168h target and an open
/// stint, or `None` when it lacks the estimate.
fn est_eod_line(db: &Path) -> Option<String> {
    for args in [&["week", "target", "168h"][..], &["start"], &["status"]] {
        let o = run(db, args);
        assert_eq!(o.status.code(), Some(0), "{args:?}: {o:?}");
        assert_eq!(err(&o), "", "{args:?}");
        if args == ["status"] {
            return out(&o)
                .lines()
                .find(|l| l.starts_with("Day total:"))
                .filter(|l| l.contains("(incl. ongoing), est. EOD "))
                .map(str::to_string);
        }
    }
    None
}

/// A rerun on a new DB covers a midnight crossing between `start` and
/// `status`, which cannot happen twice in a row.
#[test]
fn open_stint_with_max_target_shows_est_eod() {
    let (_dir, db) = fresh();
    if est_eod_line(&db).is_some() {
        return;
    }
    let (_dir2, db2) = fresh();
    assert!(
        est_eod_line(&db2).is_some(),
        "no est. EOD on the Day total line"
    );
}

// ------------------------------------------------- write failure (§5)

#[test]
fn storage_io_error_keeps_its_own_message() {
    let dir = TempDir::new().expect("temp dir");
    let blocker = dir.path().join("blocker");
    std::fs::write(&blocker, b"not a directory").expect("write blocker");
    let db = blocker.join("sub").join("mlm.db");

    let o = run(&db, &["status"]);
    assert_eq!(o.status.code(), Some(1), "{o:?}");
    let stderr = err(&o);
    assert!(
        stderr.starts_with("error: could not create data directory "),
        "{stderr}"
    );
    assert!(!stderr.contains("cannot write output"), "{stderr}");
}

#[cfg(unix)]
mod unix {
    use super::*;

    use std::process::{ChildStdin, Stdio};

    use chrono::{DateTime, Datelike, NaiveDateTime, TimeDelta, Timelike, Utc};

    /// The write end of a pipe whose read end is already closed. A helper
    /// process holds the read end as its stdin and exits. Taking `stdin`
    /// before `wait` keeps the write end open, because `wait` closes it.
    fn closed_pipe() -> ChildStdin {
        let mut helper = Command::new(env!("CARGO_BIN_EXE_mlm"))
            .arg("--version")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn pipe helper");
        let write_end = helper.stdin.take().expect("helper stdin");
        helper.wait().expect("wait for pipe helper");
        write_end
    }

    fn into_closed_stdout(db: &Path, args: &[&str]) -> Output {
        mlm(db, args)
            .stdout(closed_pipe())
            .stderr(Stdio::piped())
            .output()
            .expect("failed to run mlm binary")
    }

    fn count(db: &Path, table: &str) -> i64 {
        open(db)
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .expect("count rows")
    }

    // --------------------------------------------------- broken pipe

    #[test]
    fn closed_stdout_exits_141_quietly() {
        let (_dir, db) = fresh();
        let seeded = run(&db, &["note", "--date", "2026-10-19", "piped"]);
        assert_eq!(seeded.status.code(), Some(0), "{seeded:?}");

        let cases: [&[&str]; 3] = [
            &["week", "2026-41"],
            &["status"],
            &["export", "notes", "--date", "2026-10-19"],
        ];
        for args in cases {
            let o = into_closed_stdout(&db, args);
            assert_eq!(o.status.code(), Some(141), "{args:?}: {o:?}");
            assert_eq!(err(&o), "", "{args:?}");
        }
    }

    #[test]
    fn closed_stdout_keeps_committed_writes() {
        let (_dir, db) = fresh();
        let o = into_closed_stdout(&db, &["week", "target", "40h"]);
        assert_eq!(o.status.code(), Some(141), "{o:?}");
        assert_eq!(err(&o), "");
        let rows = target_rows(&db);
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert_eq!(rows[0].1, 2400);

        let seeded = run(&db, &["start", "--date", "2026-10-19", "09:00"]);
        assert_eq!(seeded.status.code(), Some(0), "{seeded:?}");
        let o = into_closed_stdout(&db, &["delete", "punch", "1", "--date", "2026-10-19"]);
        assert_eq!(o.status.code(), Some(141), "{o:?}");
        assert_eq!(err(&o), "");
        assert_eq!(count(&db, "punches"), 0);
    }

    #[test]
    fn closed_stderr_keeps_the_exit_code() {
        let (_dir, db) = fresh();
        let cases: [(&[&str], i32); 2] = [
            (&["start", "25:00"], 1),
            (&["week", "2026-41", "target", "20h"], 2),
        ];
        for (args, code) in cases {
            let o = mlm(&db, args)
                .stdout(Stdio::piped())
                .stderr(closed_pipe())
                .output()
                .expect("failed to run mlm binary");
            assert_eq!(o.status.code(), Some(code), "{args:?}: {o:?}");
            assert_eq!(out(&o), "", "{args:?}");
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn full_disk_prints_cannot_write_output() {
        let (_dir, db) = fresh();
        let full = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/full")
            .expect("open /dev/full");
        let o = mlm(&db, &["week", "2026-41"])
            .stdout(full)
            .stderr(Stdio::piped())
            .output()
            .expect("failed to run mlm binary");
        assert_eq!(o.status.code(), Some(1), "{o:?}");
        assert_eq!(
            err(&o),
            "error: cannot write output: No space left on device (os error 28)\n"
        );
    }

    // ------------------------------------------------- repeated hour

    #[derive(Clone, Copy)]
    enum Pass {
        First,
        Second,
    }

    /// A POSIX `TZ` rule whose DST (standard + 1h) starts on 1 January at
    /// 00:00 and ends so that `now` sits in `pass` of the repeated hour,
    /// 30 minutes from each edge. The standard offset puts `now` at 12:MM
    /// local standard time, so the repeated hour stays inside one local
    /// date. Also returns that standard wall time.
    fn repeated_hour_tz(now: DateTime<Utc>, pass: Pass) -> (String, NaiveDateTime) {
        let std_hours = 12 - i64::from(now.hour());
        let wall = floor(now).naive_utc() + TimeDelta::hours(std_hours);
        let t = match pass {
            Pass::First => wall + TimeDelta::minutes(30),
            Pass::Second => wall - TimeDelta::minutes(30),
        };
        let end = t + TimeDelta::hours(1);

        // POSIX offsets count hours west of UTC.
        let tz = format!(
            "AAA{}BBB{},0/0,{}/{}",
            -std_hours,
            -(std_hours + 1),
            end.ordinal0(),
            end.format("%H:%M")
        );
        (tz, wall)
    }

    fn floor(t: DateTime<Utc>) -> DateTime<Utc> {
        t.with_second(0)
            .and_then(|t| t.with_nanosecond(0))
            .expect("UTC has every minute")
    }

    fn status_in(db: &Path, tz: &str) {
        let o = mlm(db, &["status"])
            .env("TZ", tz)
            .output()
            .expect("failed to run mlm binary");
        assert_eq!(o.status.code(), Some(0), "{o:?}");
        assert_eq!(err(&o), "");
        assert!(out(&o).contains("Day total:"), "{}", out(&o));
    }

    #[test]
    fn repeated_hour_first_pass_status_exits_zero() {
        let (tz, _) = repeated_hour_tz(Utc::now(), Pass::First);
        let (_dir, db) = fresh();
        status_in(&db, &tz);
    }

    #[test]
    fn repeated_hour_second_pass_status_and_untyped_start() {
        let before = Utc::now();
        let (tz, wall) = repeated_hour_tz(before, Pass::Second);
        let (_dir, db) = fresh();
        status_in(&db, &tz);

        let o = mlm(&db, &["start"])
            .env("TZ", &tz)
            .output()
            .expect("failed to run mlm binary");
        let after = Utc::now();
        assert_eq!(o.status.code(), Some(0), "{o:?}");
        assert_eq!(err(&o), "");
        assert_eq!(out(&o), "");

        let rows = punch_rows(&db);
        assert_eq!(rows.len(), 1, "{rows:?}");
        let (at_utc, date, kind) = &rows[0];
        assert_eq!(kind, "start");

        // Storage takes the first pass, one hour before the captured minute.
        let stored = DateTime::parse_from_rfc3339(at_utc)
            .expect("rfc 3339")
            .with_timezone(&Utc);
        let lo = floor(before) - TimeDelta::hours(1);
        let hi = floor(after) - TimeDelta::hours(1);
        assert!(lo <= stored && stored <= hi, "{stored} not in {lo}..={hi}");
        assert_eq!(*date, wall.format("%Y-%m-%d").to_string());
    }

    fn punch_rows(db: &Path) -> Vec<(String, String, String)> {
        let conn = open(db);
        let mut stmt = conn
            .prepare(r#"SELECT at_utc, "date", kind FROM punches"#)
            .expect("prepare");
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .expect("query")
            .collect::<Result<_, _>>()
            .expect("rows")
    }

    #[test]
    fn typed_repeated_hour_start_stores_the_first_pass() {
        let (_dir, db) = fresh();
        let o = mlm(&db, &["start", "02:30", "--date", "2026-10-25"])
            .env("TZ", "Europe/Warsaw")
            .output()
            .expect("failed to run mlm binary");
        assert_eq!(o.status.code(), Some(0), "{o:?}");
        assert_eq!(
            punch_rows(&db),
            [(
                "2026-10-25T00:30:00Z".to_string(),
                "2026-10-25".to_string(),
                "start".to_string()
            )]
        );
    }

    // --------------------------------------------- DST boundary minutes

    fn warsaw_start(db: &Path, time: &str, date: &str) -> Output {
        mlm(db, &["start", time, "--date", date])
            .env("TZ", "Europe/Warsaw")
            .output()
            .expect("failed to run mlm binary")
    }

    fn assert_stored(date: &str, time: &str, at_utc: &str) {
        let (_dir, db) = fresh();
        let o = warsaw_start(&db, time, date);
        assert_eq!(o.status.code(), Some(0), "{time}: {o:?}");
        assert_eq!(err(&o), "", "{time}");
        let rows = punch_rows(&db);
        assert_eq!(rows.len(), 1, "{time}: {rows:?}");
        assert_eq!(
            (rows[0].0.as_str(), rows[0].1.as_str()),
            (at_utc, date),
            "{time}"
        );
    }

    #[test]
    fn fall_back_boundary_minutes_store_the_real_instant() {
        let cases = [
            ("01:59", "2026-10-24T23:59:00Z"),
            ("02:00", "2026-10-25T00:00:00Z"),
            ("02:30", "2026-10-25T00:30:00Z"),
            ("02:59", "2026-10-25T00:59:00Z"),
            ("03:00", "2026-10-25T02:00:00Z"),
            ("03:01", "2026-10-25T02:01:00Z"),
        ];
        for (time, at_utc) in cases {
            assert_stored("2026-10-25", time, at_utc);
        }
    }

    #[test]
    fn spring_forward_edges_store_the_real_instant() {
        assert_stored("2026-03-29", "01:59", "2026-03-29T00:59:00Z");
        assert_stored("2026-03-29", "03:00", "2026-03-29T01:00:00Z");
    }

    #[test]
    fn spring_forward_gap_minutes_are_rejected() {
        for time in ["02:00", "02:30", "02:59"] {
            let (_dir, db) = fresh();
            let o = warsaw_start(&db, time, "2026-03-29");
            assert_eq!(o.status.code(), Some(1), "{time}: {o:?}");
            assert_eq!(
                err(&o),
                format!(
                    "error: local time 2026-03-29 {time}:00 does not exist (spring-forward DST gap)\n"
                ),
            );
            assert!(punch_rows(&db).is_empty(), "{time}");
        }
    }
}
