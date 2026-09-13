//! Binary-level tests for `mlm status` (SPEC.md §6.1, §6.3).
//!
//! Per plans/milestone-10-status-command.md §5's test-infrastructure
//! note: `assert_cmd`/`predicates` are declined project-wide, so the
//! compiled binary is driven directly via `std::process::Command`
//! against `env!("CARGO_BIN_EXE_mlm")`, with `MLM_DB_PATH` (Milestone
//! 3's fix) redirecting each run to an isolated temp file.

use std::process::Command;

use rusqlite::Connection;
use tempfile::TempDir;

fn mlm(db_path: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_mlm"))
        .args(args)
        .env("MLM_DB_PATH", db_path)
        .output()
        .expect("failed to run mlm binary")
}

/// Same as [`mlm`] but with the child process's `TZ` pinned, so
/// `chrono::Local` (read exactly once per process, cached thereafter)
/// resolves against a specific IANA zone instead of the host's.
fn mlm_tz(db_path: &std::path::Path, tz: &str, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_mlm"))
        .args(args)
        .env("MLM_DB_PATH", db_path)
        .env("TZ", tz)
        .output()
        .expect("failed to run mlm binary")
}

/// Seed a punch row directly via SQL, bypassing the CLI (which only ever
/// writes punches for "today"). `at_utc` and `date` are given pre-
/// formatted so the test can pin exact historical instants around a real
/// DST transition. Mirrors the frozen schema in `src/db.rs`.
fn seed_punch(db_path: &std::path::Path, at_utc: &str, date: &str, kind: &str) {
    // Running `mlm` first (any command) creates the DB file and applies
    // migrations, so the schema exists before this raw insert runs.
    let conn = Connection::open(db_path).expect("open seeded db");
    conn.execute(
        "INSERT INTO punches (at_utc, \"date\", kind) VALUES (?1, ?2, ?3)",
        (at_utc, date, kind),
    )
    .expect("seed punch insert");
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

/// Cross-cutting: DST-safe per-instant conversion (§2.1, F12) end-to-end
/// through `status`'s rendering path, not just the storage layer
/// (Milestone 4 already covers storage in `src/storage.rs`'s D1/D2
/// tests, using an explicit `chrono_tz::TimeZone` value rather than the
/// OS's configured zone — fully portable, unaffected by the Windows
/// caveat below). Two completed stints straddle Europe/Warsaw's real
/// 2026 spring-forward (2026-03-29 02:00 -> 03:00 CET->CEST): one on
/// 2026-03-28 (still UTC+1) and one on 2026-03-30 (already UTC+2). Both
/// are the *same* local wall-clock stint (12:00-13:00), stored as
/// different UTC instants an hour apart in offset, and `status` must
/// render both back as "12:00-13:00" for their respective dates —
/// proving the UTC-to-local conversion is per-instant, not a single
/// cached offset.
///
/// Unix-only: this test forces a specific zone via the child process's
/// `TZ` environment variable, which `chrono::Local` (the real type the
/// production code path uses for "now"/local time — correctly, since it
/// must reflect the user's actual system zone) honors on Unix via
/// glibc's `tzset()`. Windows' CRT does not read `TZ` for IANA zone
/// names at all — it resolves the local zone from the OS's own
/// registry-backed timezone APIs, so setting `TZ=Europe/Warsaw` there
/// has no effect and this test's whole premise doesn't apply. This is a
/// test-infrastructure gap (no portable way to force chrono::Local to a
/// specific zone across platforms without changing the production
/// code's timezone source), not evidence of a Windows-specific bug in
/// mlm's own conversion logic — storage.rs's explicit-Tz DST tests
/// above already cover the actual conversion math portably.
#[cfg_attr(
    windows,
    ignore = "TZ env var does not control chrono::Local on Windows; see doc comment"
)]
#[test]
fn dst_transition_is_shown_correctly_end_to_end_via_status() {
    let dir = TempDir::new().expect("temp dir");
    let db_path = dir.path().join("mlm.db");

    // Any invocation bootstraps the schema (migrations run on connect).
    let boot = mlm_tz(&db_path, "Europe/Warsaw", &["status"]);
    assert!(boot.status.success(), "bootstrap status: {:?}", boot);

    // Before the transition: 2026-03-28 12:00-13:00 Warsaw (CET, UTC+1).
    seed_punch(&db_path, "2026-03-28T11:00:00Z", "2026-03-28", "start");
    seed_punch(&db_path, "2026-03-28T12:00:00Z", "2026-03-28", "end");
    // After the transition: 2026-03-30 12:00-13:00 Warsaw (CEST, UTC+2).
    seed_punch(&db_path, "2026-03-30T10:00:00Z", "2026-03-30", "start");
    seed_punch(&db_path, "2026-03-30T11:00:00Z", "2026-03-30", "end");

    let before = mlm_tz(&db_path, "Europe/Warsaw", &["status", "2026-03-28"]);
    assert!(before.status.success(), "{:?}", before);
    let before_out = stdout(&before);
    assert!(
        before_out.contains("12:00-13:00"),
        "pre-transition stint not shown in local wall-clock time: {before_out:?}"
    );
    assert!(
        before_out.contains("01h 00m"),
        "pre-transition duration wrong: {before_out:?}"
    );

    let after = mlm_tz(&db_path, "Europe/Warsaw", &["status", "2026-03-30"]);
    assert!(after.status.success(), "{:?}", after);
    let after_out = stdout(&after);
    assert!(
        after_out.contains("12:00-13:00"),
        "post-transition stint not shown in local wall-clock time: {after_out:?}"
    );
    assert!(
        after_out.contains("01h 00m"),
        "post-transition duration wrong: {after_out:?}"
    );
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
