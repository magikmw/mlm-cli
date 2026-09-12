//! Cross-cutting plain-ASCII sweep (SPEC.md §7): one dedicated byte-range
//! check over realistic, full `status` AND `week` output together,
//! driven through the actual compiled binary end-to-end. Individual
//! milestones already ASCII-check curated render fixtures in isolation
//! (`src/status.rs`'s `t13_...`, `src/week_view.rs`'s `a11_...`/
//! `c12_...`), but nothing previously sweeps both commands' real output
//! side by side in one pass. A non-ASCII, user-typed note body (SPEC.md
//! explicitly permits these to pass through unmangled) is included and
//! carved out of the byte check rather than silently allowed to make the
//! whole sweep meaningless.

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

fn stdout(output: &std::process::Output) -> String {
    let s = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(output.status.success(), "command failed: {output:?}");
    s
}

fn seed_punch(conn: &Connection, at_utc: &str, date: &str, kind: &str) {
    conn.execute(
        "INSERT INTO punches (at_utc, \"date\", kind) VALUES (?1, ?2, ?3)",
        (at_utc, date, kind),
    )
    .expect("seed punch insert");
}

fn seed_note(conn: &Connection, date: &str, body: &str, created_at_utc: &str) {
    conn.execute(
        "INSERT INTO notes (\"date\", body, created_at_utc) VALUES (?1, ?2, ?3)",
        (date, body, created_at_utc),
    )
    .expect("seed note insert");
}

const NON_ASCII_NOTE: &str = "caf\u{e9} planning notes, \u{2013} not a hyphen";

#[test]
fn ascii_sweep_over_realistic_status_and_week_output() {
    let dir = TempDir::new().expect("temp dir");
    let db_path = dir.path().join("mlm.db");

    // Bootstrap the schema (any invocation runs migrations).
    let boot = mlm(&db_path, &["status"]);
    assert!(boot.status.success(), "bootstrap: {:?}", boot);

    // Week 2026-02 = Mon 2026-01-05 .. Sun 2026-01-11.
    {
        let conn = Connection::open(&db_path).expect("open seeded db");

        // Mon: an ordinary completed stint.
        seed_punch(&conn, "2026-01-05T08:00:00Z", "2026-01-05", "start");
        seed_punch(&conn, "2026-01-05T12:00:00Z", "2026-01-05", "end");

        // Tue: a double-start anomaly (multi-open), plus an ASCII note.
        seed_punch(&conn, "2026-01-06T08:00:00Z", "2026-01-06", "start");
        seed_punch(&conn, "2026-01-06T09:00:00Z", "2026-01-06", "start");
        seed_note(
            &conn,
            "2026-01-06",
            "reviewed pull requests",
            "2026-01-06T09:30:00Z",
        );

        // Wed: an orphaned-end anomaly, plus a non-ASCII note.
        seed_punch(&conn, "2026-01-07T15:00:00Z", "2026-01-07", "end");
        seed_note(&conn, "2026-01-07", NON_ASCII_NOTE, "2026-01-07T15:05:00Z");

        // Thu: another ordinary completed stint, to give `week` a
        // non-trivial total alongside the anomaly-bearing rows.
        seed_punch(&conn, "2026-01-08T07:00:00Z", "2026-01-08", "start");
        seed_punch(&conn, "2026-01-08T16:30:00Z", "2026-01-08", "end");
    }

    // A non-default, negative-fulfillment-friendly target so `week`'s
    // signed-minute formatting (§4.2) is actually exercised too.
    let target = mlm(&db_path, &["week", "target", "2026-02", "50h"]);
    assert!(target.status.success(), "{:?}", target);

    let mut combined = String::new();
    combined.push_str(&stdout(&mlm(&db_path, &["status", "2026-01-05"])));
    combined.push_str(&stdout(&mlm(&db_path, &["status", "2026-01-06"])));
    combined.push_str(&stdout(&mlm(&db_path, &["status", "2026-01-07"])));
    combined.push_str(&stdout(&mlm(&db_path, &["status", "2026-01-08"])));
    combined.push_str(&stdout(&mlm(&db_path, &["week", "2026-02"])));

    // The note body must have passed through unmangled...
    assert!(
        combined.contains(NON_ASCII_NOTE),
        "non-ASCII note body must pass through verbatim: {combined:?}"
    );
    // ...and is the only reason this sweep should ever see a non-ASCII
    // byte. Carve it out, then everything else must be plain ASCII plus
    // newlines (§7).
    let swept = combined.replace(NON_ASCII_NOTE, "");
    let bad_bytes: Vec<u8> = swept
        .bytes()
        .filter(|&b| !(b == b'\n' || (0x20..=0x7E).contains(&b)))
        .collect();
    assert!(
        bad_bytes.is_empty(),
        "non-plain-ASCII bytes found outside note bodies: {bad_bytes:?} in {swept:?}"
    );

    // Sanity: this actually exercised anomalies and non-trivial content,
    // not an accidentally-empty sweep.
    assert!(combined.contains("[!]"), "expected an anomaly marker");
    assert!(combined.contains("reviewed pull requests"));
    assert!(combined.contains("Target:"));
}
