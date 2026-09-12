//! Cross-cutting E6 end-to-end (SPEC.md §6.1, PLAN.md's "Cross-cutting
//! concerns" section): a DB open/migration failure surfacing as a
//! nonzero exit with an stderr message and no panic, through at least
//! one write command and one read command. Milestone 3 only proves this
//! at the schema layer (`src/db.rs`'s `connect_at_errors_when_parent_
//! path_is_a_regular_file` and friends) -- nothing previously re-asserts
//! it through an actual command invocation of the compiled binary.
//!
//! Per plans/milestone-10-status-command.md §5's test-infrastructure
//! note (also followed by `tests/status_cli.rs`): the compiled binary is
//! driven directly via `std::process::Command`, no `assert_cmd`.

use std::process::Command;

use tempfile::TempDir;

fn mlm(db_path: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_mlm"))
        .args(args)
        .env("MLM_DB_PATH", db_path)
        .output()
        .expect("failed to run mlm binary")
}

/// A path that can never be opened as a database: `<blocker>/sub/mlm.db`
/// where `blocker` is itself a regular file, so the parent directory
/// cannot be created (mirrors `src/db.rs`'s T11 unit test, but here
/// exercised through the actual binary/command dispatch).
fn unopenable_db_path(dir: &std::path::Path) -> std::path::PathBuf {
    let blocker = dir.join("blocker");
    std::fs::write(&blocker, b"not a directory").expect("write blocker");
    blocker.join("sub").join("mlm.db")
}

/// E6 through a write command: `start` must not panic, must exit
/// nonzero, must print a message to stderr, and (implicitly, since the
/// DB can never be opened) cannot have written anything.
#[test]
fn e6_db_open_failure_surfaces_through_start() {
    let dir = TempDir::new().expect("temp dir");
    let bad_path = unopenable_db_path(dir.path());

    let output = mlm(&bad_path, &["start", "09:00"]);

    assert!(
        !output.status.success(),
        "start against an unopenable DB should exit nonzero: {output:?}"
    );
    assert_ne!(output.status.code(), Some(0));
    assert!(
        output.status.code().is_some(),
        "process should exit cleanly, not die from a signal (no panic/abort): {output:?}"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.is_empty(),
        "an error message should be printed to stderr"
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).is_empty(),
        "nothing should be written to stdout on a hard error"
    );
    // No partial write: the blocker file must be untouched (still just
    // the literal bytes written above, not turned into a directory or
    // a sqlite file).
    assert!(bad_path.parent().unwrap().parent().unwrap().is_file());
}

/// E6 through a read command: `status` must behave identically -- a
/// clean nonzero exit with an stderr message, not a panic.
#[test]
fn e6_db_open_failure_surfaces_through_status() {
    let dir = TempDir::new().expect("temp dir");
    let bad_path = unopenable_db_path(dir.path());

    let output = mlm(&bad_path, &["status"]);

    assert!(
        !output.status.success(),
        "status against an unopenable DB should exit nonzero: {output:?}"
    );
    assert_ne!(output.status.code(), Some(0));
    assert!(
        output.status.code().is_some(),
        "process should exit cleanly, not die from a signal (no panic/abort): {output:?}"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.is_empty(),
        "an error message should be printed to stderr"
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).is_empty(),
        "nothing should be written to stdout on a hard error"
    );
}
