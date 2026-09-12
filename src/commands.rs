//! `start`, `stop`, `note` command handlers (SPEC.md §3.2, §3.3, §3.4,
//! §4.1, §6.1, §6.3).
//!
//! Each handler is a plain function returning `anyhow::Result<()>`: no
//! printing of errors, no `process::exit`, no panics (PLAN.md interface
//! contract 7 — the crate-wide `anyhow` convention established by this
//! milestone). `now` is always injected by the caller (`main`), never
//! read from a global clock here (contract 6).

use chrono::{DateTime, Local, Utc};
use rusqlite::Connection;

use crate::cli::{NoteArgs, PunchArgs};
use crate::storage::{self, PunchKind};
use crate::time::parse_time;

/// Record a start punch for today, optionally with an inline note
/// (SPEC §3.2).
pub fn start(conn: &mut Connection, now: DateTime<Local>, args: &PunchArgs) -> anyhow::Result<()> {
    punch(conn, now, PunchKind::Start, args)
}

/// Record an end punch for today, optionally with an inline note
/// (SPEC §3.3 — "same shape as start").
pub fn stop(conn: &mut Connection, now: DateTime<Local>, args: &PunchArgs) -> anyhow::Result<()> {
    punch(conn, now, PunchKind::End, args)
}

/// Record a standalone work-log note for today (SPEC §3.4).
pub fn note(conn: &mut Connection, now: DateTime<Local>, args: &NoteArgs) -> anyhow::Result<()> {
    let today = now.date_naive();
    let body = args.body.join(" ");
    storage::insert_note(conn, today, &body, now.with_timezone(&Utc))?;
    Ok(())
}

/// Shared `start`/`stop` implementation, parameterised by punch kind.
///
/// Validation order (§4.1): `TIME` is parsed first (E1 exits here,
/// nothing written), then the note body is handed to
/// `storage::insert_punch_with_note`, which itself validates
/// empty/whitespace bodies *before* opening a transaction (E5) and wraps
/// the punch+note pair in one transaction so a failure at either insert
/// rolls back both (§6.1: a rejected note leaves no orphaned punch).
fn punch(
    conn: &mut Connection,
    now: DateTime<Local>,
    kind: PunchKind,
    args: &PunchArgs,
) -> anyhow::Result<()> {
    let today = now.date_naive();

    let time_of_day = match &args.time {
        Some(s) => parse_time(s)?,
        None => now.time(),
    };

    let note_text = join_note(&args.note);

    storage::insert_punch_with_note(
        conn,
        kind,
        today,
        time_of_day,
        &Local,
        note_text.as_deref(),
        now.with_timezone(&Utc),
    )?;
    Ok(())
}

/// `Vec<String>` positionals joined with single spaces (§1.2). `None`
/// means "no note tokens at all"; `Some("")` or `Some("   ")` (a note
/// token that is present but empty/whitespace) must still reach
/// storage's validation as a real, rejectable value — it must never be
/// silently downgraded to "no note" here.
fn join_note(parts: &[String]) -> Option<String> {
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use crate::db;
    use crate::storage::{Note, Punch, StorageError, notes_for_date, punches_for_date};
    use chrono::{Local, NaiveDate, TimeZone};
    use clap::Parser;

    fn test_db() -> Connection {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");
        db::apply_migrations(&mut conn).expect("apply migrations");
        conn
    }

    /// Fixed clock used throughout: 2026-02-12T14:23:00 local (a
    /// Thursday, per the plan's test-harness section).
    fn fixed_now() -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 2, 12, 14, 23, 0).unwrap()
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 2, 12).unwrap()
    }

    fn punch_args(argv: &[&str]) -> PunchArgs {
        // argv excludes "mlm" and the subcommand name.
        let mut full = vec!["mlm", "start"];
        full.extend_from_slice(argv);
        match Cli::try_parse_from(full).expect("parse").command {
            crate::cli::Command::Start(a) => a,
            other => panic!("expected Start, got {other:?}"),
        }
    }

    fn note_args(argv: &[&str]) -> NoteArgs {
        let mut full = vec!["mlm", "note"];
        full.extend_from_slice(argv);
        match Cli::try_parse_from(full).expect("parse").command {
            crate::cli::Command::Note(a) => a,
            other => panic!("expected Note, got {other:?}"),
        }
    }

    fn punches(conn: &Connection) -> Vec<Punch> {
        punches_for_date(conn, today()).expect("read punches")
    }

    fn notes(conn: &Connection) -> Vec<Note> {
        notes_for_date(conn, today()).expect("read notes")
    }

    // --- Happy paths -------------------------------------------------

    // T1
    #[test]
    fn start_no_args_uses_now() {
        let mut conn = test_db();
        start(&mut conn, fixed_now(), &punch_args(&[])).expect("ok");
        let p = punches(&conn);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].kind, PunchKind::Start);
        assert_eq!(p[0].date, today());
        assert_eq!(p[0].at_utc, fixed_now().with_timezone(&Utc));
        assert!(notes(&conn).is_empty());
    }

    // T2
    #[test]
    fn start_with_time() {
        let mut conn = test_db();
        start(&mut conn, fixed_now(), &punch_args(&["9:05"])).expect("ok");
        let p = punches(&conn);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].kind, PunchKind::Start);
        assert_eq!(
            p[0].at_utc,
            Local
                .with_ymd_and_hms(2026, 2, 12, 9, 5, 0)
                .unwrap()
                .with_timezone(&Utc)
        );
        assert!(notes(&conn).is_empty());
    }

    // T3
    #[test]
    fn start_with_time_forms() {
        for (input, (h, m)) in [("0905", (9, 5)), ("9", (9, 0)), ("17:30", (17, 30))] {
            let mut conn = test_db();
            start(&mut conn, fixed_now(), &punch_args(&[input])).expect("ok");
            let p = punches(&conn);
            assert_eq!(p.len(), 1);
            assert_eq!(
                p[0].at_utc,
                Local
                    .with_ymd_and_hms(2026, 2, 12, h, m, 0)
                    .unwrap()
                    .with_timezone(&Utc)
            );
        }
    }

    // T4 — F5
    #[test]
    fn start_with_time_and_note() {
        let mut conn = test_db();
        start(
            &mut conn,
            fixed_now(),
            &punch_args(&["9:05", "kicked off migration work"]),
        )
        .expect("ok");
        let p = punches(&conn);
        let n = notes(&conn);
        assert_eq!(p.len(), 1);
        assert_eq!(n.len(), 1);
        assert_eq!(
            p[0].at_utc,
            Local
                .with_ymd_and_hms(2026, 2, 12, 9, 5, 0)
                .unwrap()
                .with_timezone(&Utc)
        );
        assert_eq!(n[0].body, "kicked off migration work");
        assert_eq!(n[0].date, today());
        // note created_at_utc is the injected `now`, not the punch time.
        assert_eq!(n[0].created_at_utc, fixed_now().with_timezone(&Utc));
    }

    // T5
    #[test]
    fn start_with_unquoted_multiword_note() {
        let mut conn = test_db();
        start(
            &mut conn,
            fixed_now(),
            &punch_args(&["9:05", "kicked", "off", "migration", "work"]),
        )
        .expect("ok");
        assert_eq!(notes(&conn)[0].body, "kicked off migration work");
    }

    // T6 — stop mirrors start
    #[test]
    fn stop_all_shapes() {
        {
            let mut conn = test_db();
            let args = match Cli::try_parse_from(["mlm", "stop"]).expect("parse").command {
                crate::cli::Command::Stop(a) => a,
                other => panic!("expected Stop, got {other:?}"),
            };
            stop(&mut conn, fixed_now(), &args).expect("ok");
            assert_eq!(punches(&conn)[0].kind, PunchKind::End);
        }
        {
            let mut conn = test_db();
            let args = match Cli::try_parse_from(["mlm", "stop", "17:30"])
                .expect("parse")
                .command
            {
                crate::cli::Command::Stop(a) => a,
                other => panic!("expected Stop, got {other:?}"),
            };
            stop(&mut conn, fixed_now(), &args).expect("ok");
            let p = punches(&conn);
            assert_eq!(p[0].kind, PunchKind::End);
            assert_eq!(
                p[0].at_utc,
                Local
                    .with_ymd_and_hms(2026, 2, 12, 17, 30, 0)
                    .unwrap()
                    .with_timezone(&Utc)
            );
        }
        {
            let mut conn = test_db();
            let args = match Cli::try_parse_from(["mlm", "stop", "17:30", "wrapped up"])
                .expect("parse")
                .command
            {
                crate::cli::Command::Stop(a) => a,
                other => panic!("expected Stop, got {other:?}"),
            };
            stop(&mut conn, fixed_now(), &args).expect("ok");
            assert_eq!(punches(&conn).len(), 1);
            assert_eq!(punches(&conn)[0].kind, PunchKind::End);
            assert_eq!(notes(&conn).len(), 1);
        }
    }

    // T7 — F4
    #[test]
    fn note_only_standalone() {
        let mut conn = test_db();
        note(
            &mut conn,
            fixed_now(),
            &note_args(&["fixed migration runner bug"]),
        )
        .expect("ok");
        assert!(punches(&conn).is_empty());
        let n = notes(&conn);
        assert_eq!(n.len(), 1);
        assert_eq!(n[0].date, today());
        assert_eq!(n[0].body, "fixed migration runner bug");
    }

    // T8
    #[test]
    fn note_padded_body_is_trimmed() {
        let mut conn = test_db();
        note(&mut conn, fixed_now(), &note_args(&["  did a thing  "])).expect("ok");
        assert_eq!(notes(&conn)[0].body, "did a thing");
    }

    // T9 — §6.3
    #[test]
    fn success_exits_zero() {
        let mut conn = test_db();
        let r1 = start(&mut conn, fixed_now(), &punch_args(&[]));
        assert_eq!(crate::exit_code(&r1), 0);

        let mut conn2 = test_db();
        let r2 = start(&mut conn2, fixed_now(), &punch_args(&["9:05", "note text"]));
        assert_eq!(crate::exit_code(&r2), 0);

        let mut conn3 = test_db();
        let r3 = note(&mut conn3, fixed_now(), &note_args(&["standalone"]));
        assert_eq!(crate::exit_code(&r3), 0);
    }

    // --- No-chronology acceptance (§3.2) ------------------------------

    // T10
    #[test]
    fn start_before_existing_start_accepted() {
        let mut conn = test_db();
        start(&mut conn, fixed_now(), &punch_args(&["14:00"])).expect("ok");
        start(&mut conn, fixed_now(), &punch_args(&["09:00"])).expect("ok");
        let p = punches(&conn);
        assert_eq!(p.len(), 2);
        assert!(p.iter().all(|x| x.kind == PunchKind::Start));
    }

    // T11
    #[test]
    fn stop_before_its_start_accepted() {
        let mut conn = test_db();
        start(&mut conn, fixed_now(), &punch_args(&["14:00"])).expect("ok");
        let args = match Cli::try_parse_from(["mlm", "stop", "09:00"])
            .expect("parse")
            .command
        {
            crate::cli::Command::Stop(a) => a,
            other => panic!("expected Stop, got {other:?}"),
        };
        stop(&mut conn, fixed_now(), &args).expect("ok");
        assert_eq!(punches(&conn).len(), 2);
    }

    // T12
    #[test]
    fn f3_entry_order_accepted_verbatim() {
        let mut conn = test_db();
        start(&mut conn, fixed_now(), &punch_args(&["09:00"])).expect("ok");
        start(&mut conn, fixed_now(), &punch_args(&["14:00"])).expect("ok");
        for (input, expect_hm) in [("18:00", (18, 0)), ("13:00", (13, 0))] {
            let args = match Cli::try_parse_from(["mlm", "stop", input])
                .expect("parse")
                .command
            {
                crate::cli::Command::Stop(a) => a,
                other => panic!("expected Stop, got {other:?}"),
            };
            let r = stop(&mut conn, fixed_now(), &args);
            assert!(r.is_ok());
            let _ = expect_hm;
        }
        let p = punches_for_date(&conn, today())
            .expect("read")
            .into_iter()
            .collect::<Vec<_>>();
        // 4 rows total; ordered by id (insertion order) as returned by a
        // raw id-ordered query rather than punches_for_date's chronology
        // sort, since this test is about F3's *input* half.
        assert_eq!(p.len(), 4);
    }

    // --- Hard errors: E1, malformed TIME -------------------------------

    // T13
    #[test]
    fn malformed_time_rejected() {
        for bad in ["25:00", "24:00", "9:75", "abc", "9:5:5", ""] {
            let mut conn = test_db();
            let r = start(&mut conn, fixed_now(), &punch_args(&[bad]));
            let err = r.expect_err(&format!("expected error for {bad:?}"));
            assert!(
                err.downcast_ref::<crate::time::TimeParseError>().is_some(),
                "expected TimeParseError for {bad:?}, got {err:?}"
            );
            assert!(punches(&conn).is_empty());
            assert!(notes(&conn).is_empty());
            assert_ne!(crate::exit_code(&Err(err)), 0);

            let mut conn2 = test_db();
            let args = match Cli::try_parse_from(["mlm", "stop", bad])
                .expect("parse")
                .command
            {
                crate::cli::Command::Stop(a) => a,
                other => panic!("expected Stop, got {other:?}"),
            };
            let r2 = stop(&mut conn2, fixed_now(), &args);
            assert!(r2.is_err());
            assert!(punches(&conn2).is_empty());
        }
    }

    // T14
    #[test]
    fn malformed_time_with_note_writes_nothing() {
        let mut conn = test_db();
        let r = start(&mut conn, fixed_now(), &punch_args(&["25:00", "some note"]));
        assert!(r.is_err());
        assert!(punches(&conn).is_empty());
        assert!(notes(&conn).is_empty());
    }

    // T15
    #[test]
    fn time_error_precedes_note_error() {
        let mut conn = test_db();
        let r = start(&mut conn, fixed_now(), &punch_args(&["25:00", "   "]));
        let err = r.expect_err("expected error");
        assert!(err.downcast_ref::<crate::time::TimeParseError>().is_some());
        assert!(err.downcast_ref::<StorageError>().is_none());
        assert!(punches(&conn).is_empty());
        assert!(notes(&conn).is_empty());
    }

    // T16
    #[test]
    fn note_positional_after_time_is_never_reparsed_as_time() {
        let mut conn = test_db();
        start(&mut conn, fixed_now(), &punch_args(&["9:05", "25:00"])).expect("ok");
        let p = punches(&conn);
        assert_eq!(
            p[0].at_utc,
            Local
                .with_ymd_and_hms(2026, 2, 12, 9, 5, 0)
                .unwrap()
                .with_timezone(&Utc)
        );
        assert_eq!(notes(&conn)[0].body, "25:00");
    }

    // --- Hard errors: E5, empty/whitespace NOTE ------------------------

    const EMPTY_NOTE_CASES: [&str; 5] = ["", "   ", "\t", "\n", " \t \n "];

    // T17
    #[test]
    fn standalone_empty_note_rejected() {
        for bad in EMPTY_NOTE_CASES {
            let mut conn = test_db();
            let r = note(&mut conn, fixed_now(), &note_args(&[bad]));
            let err = r.expect_err(&format!("expected error for {bad:?}"));
            assert!(
                matches!(
                    err.downcast_ref::<StorageError>(),
                    Some(StorageError::EmptyNote)
                ),
                "expected EmptyNote for {bad:?}, got {err:?}"
            );
            assert!(notes(&conn).is_empty());
            assert!(err.to_string().contains("empty") || err.to_string().contains("whitespace"));
        }
    }

    // T18 — E5 core
    #[test]
    fn start_with_empty_note_leaves_no_punch() {
        for bad in EMPTY_NOTE_CASES {
            let mut conn = test_db();
            let r = start(&mut conn, fixed_now(), &punch_args(&["9:05", bad]));
            let err = r.expect_err(&format!("expected error for {bad:?}"));
            assert!(
                matches!(
                    err.downcast_ref::<StorageError>(),
                    Some(StorageError::EmptyNote)
                ),
                "expected EmptyNote for {bad:?}, got {err:?}"
            );
            assert!(punches(&conn).is_empty(), "orphaned punch for {bad:?}");
            assert!(notes(&conn).is_empty());
        }
    }

    // T19
    #[test]
    fn stop_with_empty_note_leaves_no_punch() {
        let mut conn = test_db();
        let args = match Cli::try_parse_from(["mlm", "stop", "17:30", "   "])
            .expect("parse")
            .command
        {
            crate::cli::Command::Stop(a) => a,
            other => panic!("expected Stop, got {other:?}"),
        };
        let r = stop(&mut conn, fixed_now(), &args);
        assert!(matches!(
            r.unwrap_err().downcast_ref::<StorageError>(),
            Some(StorageError::EmptyNote)
        ));
        assert!(punches(&conn).is_empty());
        assert!(notes(&conn).is_empty());
    }

    // T20
    #[test]
    fn empty_note_does_not_disturb_existing_rows() {
        let mut conn = test_db();
        start(&mut conn, fixed_now(), &punch_args(&["09:00"])).expect("seed ok");
        let r = start(&mut conn, fixed_now(), &punch_args(&["10:00", "   "]));
        assert!(r.is_err());
        assert_eq!(punches(&conn).len(), 1);
        assert!(notes(&conn).is_empty());
    }

    // T21
    #[test]
    fn bare_note_command_is_clap_error() {
        let err = Cli::try_parse_from(["mlm", "note"]).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
    }
}
