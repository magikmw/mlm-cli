//! Binary-level tests for `mlm status` (SPEC.md §6.1, §6.3).
//!
//! Per plans/milestone-10-status-command.md §5's test-infrastructure
//! note: `assert_cmd`/`predicates` are declined project-wide, so the
//! compiled binary is driven directly via `std::process::Command`
//! against `env!("CARGO_BIN_EXE_mlm")`, with `MLM_DB_PATH` (Milestone
//! 3's fix) redirecting each run to an isolated temp file.

use std::process::Command;

use tempfile::TempDir;

fn mlm(db_path: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_mlm"))
        .args(args)
        .env("MLM_DB_PATH", db_path)
        .output()
        .expect("failed to run mlm binary")
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// T14 -- §6.3: a status view with anomalies still exits 0, and the
/// anomaly lines land on stdout, not stderr (anomalies are data, not
/// diagnostics).
#[test]
fn t14_status_with_anomalies_exits_zero_and_prints_to_stdout() {
    let dir = TempDir::new().expect("temp dir");
    let db_path = dir.path().join("mlm.db");

    // E7: two dangling starts, no matching ends -> multi-open anomaly.
    let r1 = mlm(&db_path, &["start", "09:00"]);
    assert!(r1.status.success(), "seed start 1: {:?}", r1);
    let r2 = mlm(&db_path, &["start", "11:00"]);
    assert!(r2.status.success(), "seed start 2: {:?}", r2);

    let status = mlm(&db_path, &["status"]);
    assert!(
        status.status.success(),
        "status should exit 0 even with anomalies present: {:?}",
        status
    );
    assert_eq!(status.status.code(), Some(0));

    let out = stdout(&status);
    assert!(
        out.contains("[!] 2 open stints for this date (unmatched starts)"),
        "anomaly line missing from stdout: {out:?}"
    );
    assert!(
        stderr(&status).is_empty(),
        "anomalies are data, not diagnostics: stderr should stay empty, got {:?}",
        stderr(&status)
    );
}

/// T15 -- §6.1/E2: malformed DATE is a hard error, nonzero exit, message
/// on stderr, nothing on stdout.
#[test]
fn t15_malformed_date_exits_nonzero_with_stderr_message() {
    let dir = TempDir::new().expect("temp dir");
    let db_path = dir.path().join("mlm.db");

    for bad_date in ["2026-02-30", "13/02/2026"] {
        let output = mlm(&db_path, &["status", bad_date]);
        assert!(
            !output.status.success(),
            "expected nonzero exit for {bad_date:?}: {:?}",
            output
        );
        assert_ne!(output.status.code(), Some(0));
        assert!(
            stdout(&output).is_empty(),
            "nothing should be printed to stdout for {bad_date:?}: {:?}",
            stdout(&output)
        );
        assert!(
            !stderr(&output).is_empty(),
            "an error message should be printed to stderr for {bad_date:?}"
        );
    }
}

/// Sanity check on the happy path: a fresh, empty day still exits 0 and
/// prints a well-formed status view (E11's binary-level counterpart).
#[test]
fn status_on_a_fresh_empty_database_exits_zero() {
    let dir = TempDir::new().expect("temp dir");
    let db_path = dir.path().join("mlm.db");

    let output = mlm(&db_path, &["status"]);
    assert!(output.status.success(), "{:?}", output);
    let out = stdout(&output);
    assert!(out.contains("Day total:"));
    assert!(out.contains("Week"));
    assert!(out.ends_with('\n') && !out.ends_with("\n\n"));
}
