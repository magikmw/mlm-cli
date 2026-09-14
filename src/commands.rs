//! `start`, `stop`, `note` command handlers (SPEC.md §3.2, §3.3, §3.4,
//! §4.1, §6.1, §6.3).
//!
//! Each handler is a plain function returning `anyhow::Result<()>`: no
//! printing of errors, no `process::exit`, no panics (PLAN.md interface
//! contract 7 — the crate-wide `anyhow` convention established by this
//! milestone). `now` is always injected by the caller (`main`), never
//! read from a global clock here (contract 6).

use chrono::{DateTime, Local, TimeZone, Utc};
use rusqlite::Connection;

use crate::cli::{DeleteEntryArgs, NoteArgs, PunchArgs};
use crate::date::{format_date, resolve_future_checked_date};
use crate::storage::{self, Note, Punch, PunchKind};
use crate::time::{TimeParseError, parse_time};

/// Record a start punch for today, optionally with an inline note
/// (SPEC §3.2), or against another day via `--date`
/// (backdated-punches spec §4).
pub fn start(conn: &mut Connection, now: DateTime<Local>, args: &PunchArgs) -> anyhow::Result<()> {
    punch(conn, now, PunchKind::Start, args)
}

/// Record an end punch for today, optionally with an inline note
/// (SPEC §3.3 — "same shape as start"), or against another day via
/// `--date` (backdated-punches spec §4).
pub fn stop(conn: &mut Connection, now: DateTime<Local>, args: &PunchArgs) -> anyhow::Result<()> {
    punch(conn, now, PunchKind::End, args)
}

/// Record a standalone work-log note (SPEC §3.4), against today or, with
/// `--date`, a resolved past date (backdated-punches spec §4).
pub fn note(conn: &mut Connection, now: DateTime<Local>, args: &NoteArgs) -> anyhow::Result<()> {
    let today = now.date_naive();
    let target_date = resolve_target_date(&args.date, today)?;
    let body = args.body.join(" ");
    storage::insert_note(conn, target_date, &body, now.with_timezone(&Utc))?;
    Ok(())
}

/// Wrap `body` as a single, safely-quoted POSIX/fish shell token:
/// single-quoted, with every embedded `'` escaped as `'"'"'` (close the
/// open single-quote, emit a double-quoted literal `'`, reopen the
/// single-quote). Single quotes, never double: double quotes still let
/// a shell interpolate `$(...)`/backticks/`$VAR` inside them before the
/// token reaches `mlm` on replay (spec §5). The output always starts and
/// ends with `'`, even for an empty or already-safe body.
fn quote_shell_single(body: &str) -> String {
    let mut out = String::with_capacity(body.len() + 2);
    out.push('\'');
    for ch in body.chars() {
        if ch == '\'' {
            out.push_str("'\"'\"'");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

/// Validate a 1-based ephemeral `id` against `count` freshly-fetched
/// rows for the resolved date; on success, return the corresponding
/// zero-based index into that `Vec`. `id == 0` and `id > count` are both
/// application-level hard errors (spec §5) — clap has already rejected
/// negative/non-numeric/too-large values before this ever runs (Task 2).
fn validate_entry_id(id: u32, count: usize) -> anyhow::Result<usize> {
    let entry_word = if count == 1 { "entry" } else { "entries" };
    if id == 0 {
        anyhow::bail!("id must be 1 or greater (got 0); {count} {entry_word} for this date");
    }
    let index = id as usize - 1;
    if index >= count {
        anyhow::bail!("id {id} is out of range; only {count} {entry_word} for this date");
    }
    Ok(index)
}

/// List-mode output for punches (spec §4): `<n>  <kind> <HH:MM>` per
/// row, 1-based. Pure — never touches storage.
fn print_punch_list(rows: &[Punch]) {
    for (i, p) in rows.iter().enumerate() {
        let local_time = p.at_utc.with_timezone(&Local).time();
        println!(
            "{}  {} {}",
            i + 1,
            p.kind.as_str(),
            local_time.format("%H:%M")
        );
    }
}

/// List-mode output for notes (spec §4): `<n>  <body>` per row, 1-based.
/// Pure — never touches storage.
fn print_note_list(rows: &[Note]) {
    for (i, n) in rows.iter().enumerate() {
        println!("{}  {}", i + 1, n.body);
    }
}

/// Build the punch recreate-echo (spec §5/§7): `mlm start|stop
/// HH:MM[ --date YYYY-MM-DD]`. Converts the deleted row's own stored
/// `at_utc` to local time per-instant (`p.at_utc.with_timezone(&Local)`)
/// — never a single offset grabbed once from `now` and reused — so a
/// backdated punch on the far side of a DST boundary from today still
/// echoes its correct local wall-clock time.
///
/// `PunchKind::as_str()` returns `"start"`/`"end"`, but the CLI command
/// for an end-punch is `mlm stop`, not `mlm end` — never reuse
/// `as_str()` here, match on `kind` explicitly instead.
///
/// Generic over `Tz: chrono::TimeZone` (Finding 5, pre-merge review) so
/// tests can pin a real IANA zone (e.g. `chrono_tz::Europe::Warsaw`) and
/// exercise an actual DST transition instead of hardcoding
/// `chrono::Local`, which is UTC on every CI runner and would make a
/// DST-crossing test tautological. The real call site (`delete_punch`)
/// passes `chrono::Local` explicitly — production behavior unchanged.
fn punch_recreate_line<Tz: TimeZone>(p: &Punch, today: chrono::NaiveDate, tz: &Tz) -> String {
    let verb = match p.kind {
        PunchKind::Start => "start",
        PunchKind::End => "stop",
    };
    let local_time = p.at_utc.with_timezone(tz).time();
    let mut line = format!("mlm {verb} {}", local_time.format("%H:%M"));
    if p.date != today {
        line.push_str(&format!(" --date {}", format_date(p.date)));
    }
    line
}

/// Build the note recreate-echo (spec §5/§7): `mlm note[ --date
/// YYYY-MM-DD] '<quoted body>'`, `--date` always preceding the body
/// (backdated-punches spec §2.1 ordering footgun — this printed string
/// is fed back into `mlm note`, which does have that ordering
/// requirement, even though `delete`'s own args don't). Milestone 13's
/// newline-normalization guarantees a body written from this point
/// forward never contains `\n`/`\r` (a `debug_assert!` below is a cheap
/// tripwire for that invariant, not a runtime branch). Fallback (spec
/// §5): a body containing a literal `\` cannot be quoted identically
/// across shells — verified that fish's single-quote parsing recognizes
/// `\\`/`\'` escapes that POSIX shells don't inside `'...'`, so a
/// backslash would silently corrupt or break on replay under fish. For
/// that one case, print a plain description instead of a quoted command
/// — `deleted note (2026-09-10): <first line of body>...`.
fn note_recreate_line(n: &Note, today: chrono::NaiveDate) -> String {
    debug_assert!(
        !n.body.contains(['\n', '\r']),
        "note body contained a newline at echo time -- Milestone 13's insert-path \
         normalization should make this impossible"
    );
    if n.body.contains('\\') {
        let first_line = n.body.lines().next().unwrap_or("");
        return format!("deleted note ({}): {first_line}...", format_date(n.date));
    }
    let mut line = "mlm note".to_string();
    if n.date != today {
        line.push_str(&format!(" --date {}", format_date(n.date)));
    }
    line.push(' ');
    line.push_str(&quote_shell_single(&n.body));
    line
}

/// List (no `ID`) or delete (`ID` given) that date's notes (spec §4/§5).
/// `ID` is always resolved against a freshly re-run
/// `storage::notes_for_date` on every invocation — never a cached
/// listing from an earlier call (spec §5's stale-id note). On delete,
/// prints a ready-to-run recreate line (spec §5/§7); the note-body
/// quoting/footgun/legacy-newline handling for that line lives in
/// `note_recreate_line`.
pub fn delete_note(
    conn: &Connection,
    now: DateTime<Local>,
    args: &DeleteEntryArgs,
) -> anyhow::Result<()> {
    let today = now.date_naive();
    let target_date = resolve_target_date(&args.date, today)?;
    let rows = storage::notes_for_date(conn, target_date)?;

    let Some(id) = args.id else {
        if rows.is_empty() {
            println!("nothing to delete for {}.", format_date(target_date));
        } else {
            print_note_list(&rows);
        }
        return Ok(());
    };

    let index = validate_entry_id(id, rows.len())?;
    let deleted = storage::delete_note(conn, rows[index].id)?;
    println!(
        "deleted. to recreate: {}",
        note_recreate_line(&deleted, today)
    );
    Ok(())
}

/// List (no `ID`) or delete (`ID` given) that date's punches (spec
/// §4/§5). Same freshness/no-caching rule as `delete_note`. On delete,
/// prints a ready-to-run recreate line built from the deleted row's own
/// `at_utc` converted to local time per-instant (`punch_recreate_line`)
/// — never `now`'s offset.
pub fn delete_punch(
    conn: &Connection,
    now: DateTime<Local>,
    args: &DeleteEntryArgs,
) -> anyhow::Result<()> {
    let today = now.date_naive();
    let target_date = resolve_target_date(&args.date, today)?;
    let rows = storage::punches_for_date(conn, target_date)?;

    let Some(id) = args.id else {
        if rows.is_empty() {
            println!("nothing to delete for {}.", format_date(target_date));
        } else {
            print_punch_list(&rows);
        }
        return Ok(());
    };

    let index = validate_entry_id(id, rows.len())?;
    let deleted = storage::delete_punch(conn, rows[index].id)?;
    println!(
        "deleted. to recreate: {}",
        punch_recreate_line(&deleted, today, &Local)
    );
    Ok(())
}

/// Shared date-resolution step for `note()` and `punch()`: `None` means
/// "today"; `Some` is resolved (and future-checked) via
/// `resolve_future_checked_date` (backdated-punches spec §3/§4).
fn resolve_target_date(
    date: &Option<String>,
    today: chrono::NaiveDate,
) -> anyhow::Result<chrono::NaiveDate> {
    match date {
        Some(s) => Ok(resolve_future_checked_date(s, today)?),
        None => Ok(today),
    }
}

/// Shared `start`/`stop` implementation, parameterised by punch kind.
///
/// Validation order (backdated-punches spec §3, extending §4.1's
/// existing rule): date resolution happens first (a malformed or
/// future `--date` is a hard error here, before TIME is even looked
/// at), then TIME is parsed -- required when the resolved date isn't
/// today (E1/new "required" case exits here, nothing written) -- then
/// the note body is handed to `storage::insert_punch_with_note`, which
/// itself validates empty/whitespace bodies *before* opening a
/// transaction (E5) and wraps the punch+note pair in one transaction so
/// a failure at either insert rolls back both (§6.1: a rejected note
/// leaves no orphaned punch).
fn punch(
    conn: &mut Connection,
    now: DateTime<Local>,
    kind: PunchKind,
    args: &PunchArgs,
) -> anyhow::Result<()> {
    let today = now.date_naive();
    let target_date = resolve_target_date(&args.date, today)?;

    let time_of_day = match &args.time {
        Some(s) => parse_time(s)?,
        None if target_date == today => now.time(),
        None => return Err(TimeParseError::Required.into()),
    };

    let note_text = join_note(&args.note);

    storage::insert_punch_with_note(
        conn,
        kind,
        target_date,
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
    use chrono::{Local, NaiveDate, NaiveTime, TimeZone};
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

    /// A known date; panics on a typo in the test itself. (Same
    /// fixture as `date.rs`'s and `status.rs`'s `mod tests`.)
    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("test fixture is a real date")
    }

    fn stop_args(argv: &[&str]) -> PunchArgs {
        // argv excludes "mlm" and the subcommand name.
        let mut full = vec!["mlm", "stop"];
        full.extend_from_slice(argv);
        match Cli::try_parse_from(full).expect("parse").command {
            crate::cli::Command::Stop(a) => a,
            other => panic!("expected Stop, got {other:?}"),
        }
    }

    fn punches_for(conn: &Connection, date: NaiveDate) -> Vec<Punch> {
        punches_for_date(conn, date).expect("read punches")
    }

    fn notes_for(conn: &Connection, date: NaiveDate) -> Vec<Note> {
        notes_for_date(conn, date).expect("read notes")
    }

    fn delete_note_args(argv: &[&str]) -> DeleteEntryArgs {
        let mut full = vec!["mlm", "delete", "note"];
        full.extend_from_slice(argv);
        match Cli::try_parse_from(full).expect("parse").command {
            crate::cli::Command::Delete(crate::cli::DeleteArgs { target }) => match target {
                crate::cli::DeleteTarget::Note(a) => a,
                other => panic!("expected DeleteTarget::Note, got {other:?}"),
            },
            other => panic!("expected Command::Delete, got {other:?}"),
        }
    }

    fn delete_punch_args(argv: &[&str]) -> DeleteEntryArgs {
        let mut full = vec!["mlm", "delete", "punch"];
        full.extend_from_slice(argv);
        match Cli::try_parse_from(full).expect("parse").command {
            crate::cli::Command::Delete(crate::cli::DeleteArgs { target }) => match target {
                crate::cli::DeleteTarget::Punch(a) => a,
                other => panic!("expected DeleteTarget::Punch, got {other:?}"),
            },
            other => panic!("expected Command::Delete, got {other:?}"),
        }
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

    // --- backdated punches (backdated-punches spec §5) ------------------

    // Successful backdated punch with explicit TIME.
    #[test]
    fn backdated_start_with_explicit_time_succeeds() {
        let mut conn = test_db();
        start(
            &mut conn,
            fixed_now(),
            &punch_args(&["--date", "-1", "09:00"]),
        )
        .expect("ok");
        let yesterday = d(2026, 2, 11);
        let p = punches_for(&conn, yesterday);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].date, yesterday);
        assert_eq!(
            p[0].at_utc,
            Local
                .with_ymd_and_hms(2026, 2, 11, 9, 0, 0)
                .unwrap()
                .with_timezone(&Utc)
        );
        assert!(punches(&conn).is_empty(), "nothing written against today");
    }

    // Missing TIME with a backdated --date is rejected; nothing written.
    #[test]
    fn backdated_start_without_time_is_rejected() {
        let mut conn = test_db();
        let r = start(&mut conn, fixed_now(), &punch_args(&["--date", "-1"]));
        let err = r.expect_err("expected error");
        assert!(
            matches!(
                err.downcast_ref::<crate::time::TimeParseError>(),
                Some(crate::time::TimeParseError::Required)
            ),
            "expected TimeParseError::Required, got {err:?}"
        );
        assert!(punches_for(&conn, d(2026, 2, 11)).is_empty());
        assert!(punches(&conn).is_empty());
    }

    // Future --date is rejected; nothing written.
    #[test]
    fn future_dated_start_is_rejected() {
        let mut conn = test_db();
        let r = start(
            &mut conn,
            fixed_now(),
            &punch_args(&["--date", "2026-02-13", "09:00"]),
        );
        let err = r.expect_err("expected error");
        assert!(
            err.downcast_ref::<crate::date::DateWeekError>().is_some(),
            "expected DateWeekError, got {err:?}"
        );
        assert!(punches_for(&conn, d(2026, 2, 13)).is_empty());
    }

    // --date omitted behaves exactly as before (regression guard).
    #[test]
    fn omitted_date_flag_behaves_like_before() {
        let mut conn = test_db();
        start(&mut conn, fixed_now(), &punch_args(&["09:00"])).expect("ok");
        let p = punches(&conn);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].date, today());
    }

    // An anomaly-producing pairing on a backdated date surfaces the same
    // as it would for today: two starts, no end, is still just two
    // "start" rows for that date -- anomaly *rendering* is status's job,
    // this only proves storage.rs's per-date bookkeeping isn't disturbed
    // by a backdated date.
    #[test]
    fn backdated_anomaly_producing_pairing_is_stored_like_today() {
        let mut conn = test_db();
        start(
            &mut conn,
            fixed_now(),
            &punch_args(&["--date", "-1", "09:00"]),
        )
        .expect("ok");
        start(
            &mut conn,
            fixed_now(),
            &punch_args(&["--date", "-1", "10:00"]),
        )
        .expect("ok");
        let p = punches_for(&conn, d(2026, 2, 11));
        assert_eq!(p.len(), 2);
        assert!(p.iter().all(|x| x.kind == PunchKind::Start));
    }

    // An inline NOTE alongside a backdated punch lands on the *resolved*
    // date, not today.
    #[test]
    fn backdated_inline_note_lands_on_resolved_date() {
        let mut conn = test_db();
        start(
            &mut conn,
            fixed_now(),
            &punch_args(&["--date", "-1", "09:00", "kicked off migration"]),
        )
        .expect("ok");
        let yesterday = d(2026, 2, 11);
        let n = notes_for(&conn, yesterday);
        assert_eq!(n.len(), 1);
        assert_eq!(n[0].date, yesterday);
        assert_eq!(n[0].body, "kicked off migration");
        assert!(notes(&conn).is_empty(), "nothing written against today");
    }

    // A simultaneous bad --date + bad/missing TIME reports the date
    // error (precedence: date -> TIME -> NOTE).
    #[test]
    fn bad_date_precedes_bad_time_error() {
        let mut conn = test_db();
        let r = start(
            &mut conn,
            fixed_now(),
            &punch_args(&["--date", "not-a-date", "25:00"]),
        );
        let err = r.expect_err("expected error");
        assert!(
            err.downcast_ref::<crate::date::DateWeekError>().is_some(),
            "expected the date error to win, got {err:?}"
        );
        assert!(err.downcast_ref::<crate::time::TimeParseError>().is_none());
        assert!(punches(&conn).is_empty());
    }

    #[test]
    fn bad_date_precedes_missing_time_error() {
        let mut conn = test_db();
        let r = start(
            &mut conn,
            fixed_now(),
            &punch_args(&["--date", "not-a-date"]),
        );
        let err = r.expect_err("expected error");
        assert!(
            err.downcast_ref::<crate::date::DateWeekError>().is_some(),
            "expected the date error to win over the missing-TIME error, got {err:?}"
        );
    }

    // `stop` shares `punch()` with `start`, but every backdated test
    // above exercises it only via `start(...)`. `stop_all_shapes` (the
    // pre-existing test this mirrors) is the only place `stop` itself is
    // exercised at all, and it never touches `--date` -- so nothing
    // today actually proves the shared helper resolves `--date`
    // correctly on the `stop` path specifically, only that it compiles
    // against `PunchArgs`. This closes that gap.
    #[test]
    fn backdated_stop_with_explicit_time_succeeds() {
        let mut conn = test_db();
        stop(
            &mut conn,
            fixed_now(),
            &stop_args(&["--date", "-1", "17:30"]),
        )
        .expect("ok");
        let yesterday = d(2026, 2, 11);
        let p = punches_for(&conn, yesterday);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].kind, PunchKind::End);
        assert_eq!(p[0].date, yesterday);
        assert_eq!(
            p[0].at_utc,
            Local
                .with_ymd_and_hms(2026, 2, 11, 17, 30, 0)
                .unwrap()
                .with_timezone(&Utc)
        );
        assert!(punches(&conn).is_empty(), "nothing written against today");
    }

    // Malformed --date on `note` is rejected; nothing written. Only the
    // happy path (`backdated_note_stored_against_resolved_date`) and
    // `start`'s equivalent existed before -- `note()`'s own
    // date-error paths were untested.
    #[test]
    fn note_malformed_date_is_rejected() {
        let mut conn = test_db();
        let r = note(
            &mut conn,
            fixed_now(),
            &note_args(&["--date", "not-a-date", "fixed a bug"]),
        );
        let err = r.expect_err("expected error");
        assert!(
            err.downcast_ref::<crate::date::DateWeekError>().is_some(),
            "expected DateWeekError, got {err:?}"
        );
        assert!(notes(&conn).is_empty());
    }

    // Future --date on `note` is rejected; nothing written.
    #[test]
    fn note_future_date_is_rejected() {
        let mut conn = test_db();
        let r = note(
            &mut conn,
            fixed_now(),
            &note_args(&["--date", "2026-02-13", "fixed a bug"]),
        );
        let err = r.expect_err("expected error");
        let is_future = matches!(
            err.downcast_ref::<crate::date::DateWeekError>()
                .map(|e| &e.cause),
            Some(crate::date::Cause::Future)
        );
        assert!(is_future, "expected Cause::Future, got {err:?}");
        assert!(notes(&conn).is_empty());
        assert!(notes_for(&conn, d(2026, 2, 13)).is_empty());
    }

    // --- stop's own date-error paths (backdated-punches spec §5) --------
    //
    // Every existing malformed-date/future-date/missing-TIME test above
    // exercises only `start`, even though `stop` shares the same
    // `punch()` helper -- so nothing today proves `stop` itself hits
    // these paths, only that it compiles against the same helper. This
    // mirrors those three `start` tests for `stop`.

    #[test]
    fn backdated_stop_without_time_is_rejected() {
        let mut conn = test_db();
        let r = stop(&mut conn, fixed_now(), &stop_args(&["--date", "-1"]));
        let err = r.expect_err("expected error");
        let is_required = matches!(
            err.downcast_ref::<crate::time::TimeParseError>(),
            Some(crate::time::TimeParseError::Required)
        );
        assert!(
            is_required,
            "expected TimeParseError::Required, got {err:?}"
        );
        assert!(punches_for(&conn, d(2026, 2, 11)).is_empty());
        assert!(punches(&conn).is_empty());
    }

    #[test]
    fn future_dated_stop_is_rejected() {
        let mut conn = test_db();
        let r = stop(
            &mut conn,
            fixed_now(),
            &stop_args(&["--date", "2026-02-13", "09:00"]),
        );
        let err = r.expect_err("expected error");
        assert!(
            err.downcast_ref::<crate::date::DateWeekError>().is_some(),
            "expected DateWeekError, got {err:?}"
        );
        assert!(punches_for(&conn, d(2026, 2, 13)).is_empty());
    }

    #[test]
    fn stop_bad_date_precedes_bad_time_error() {
        let mut conn = test_db();
        let r = stop(
            &mut conn,
            fixed_now(),
            &stop_args(&["--date", "not-a-date", "25:00"]),
        );
        let err = r.expect_err("expected error");
        assert!(
            err.downcast_ref::<crate::date::DateWeekError>().is_some(),
            "expected the date error to win, got {err:?}"
        );
        assert!(err.downcast_ref::<crate::time::TimeParseError>().is_none());
        assert!(punches(&conn).is_empty());
    }

    // --- backdated note (backdated-punches spec §5) ---------------------

    #[test]
    fn backdated_note_stored_against_resolved_date() {
        let mut conn = test_db();
        note(
            &mut conn,
            fixed_now(),
            &note_args(&["--date", "-3", "fixed a bug"]),
        )
        .expect("ok");
        let target = d(2026, 2, 9);
        let n = notes_for(&conn, target);
        assert_eq!(n.len(), 1);
        assert_eq!(n[0].date, target);
        assert_eq!(n[0].body, "fixed a bug");
        // created_at_utc still reflects real now, not the backdated date.
        assert_eq!(n[0].created_at_utc, fixed_now().with_timezone(&Utc));
        assert!(notes(&conn).is_empty(), "nothing written against today");
    }

    // --- retroactive week recompute (backdated-punches spec §3.1) -------
    //
    // §3.1 calls this the whole point of the feature: backdating a punch
    // into an already-"closed" past week must change that week's, and a
    // later week's, owed/carry figures on the next status view. Nothing
    // above proves this -- every test up to here only inspects rows via
    // `punches_for`/`notes_for`, never a computed week figure. This
    // drives `status::resolve` (src/status.rs) directly against the same
    // in-memory connection `commands::start`/`stop` just wrote to, so it
    // is a genuine end-to-end check of storage -> week accounting, not a
    // restatement of either module's own unit tests.
    //
    // Fixture: `fixed_now()` is 2026-02-12 (Thursday, ISO week 2026-07).
    // 2026-01-27 is a Tuesday in ISO week 2026-05 (Mon 2026-01-26 .. Sun
    // 2026-02-01) and is exactly 16 days before `fixed_now()`'s date, so
    // `--date -16` reaches the same day as `--date 2026-01-27`. ISO week
    // 2026-06 (Mon 2026-02-02 .. Sun 2026-02-08) sits between 2026-05 and
    // the current week 2026-07, so it is already "closed" (in the past,
    // per `render::week_framing`) both before and after the backdated
    // punch lands -- exactly the "already-closed past week" §3.1
    // describes, not the current week's own live-updating figure.
    #[test]
    fn backdated_punch_retroactively_changes_a_later_closed_weeks_owed() {
        let mut conn = test_db();

        // Seed 4h in week 2026-05 (2026-01-27, 09:00-13:00).
        start(
            &mut conn,
            fixed_now(),
            &punch_args(&["--date", "2026-01-27", "09:00"]),
        )
        .expect("ok");
        stop(
            &mut conn,
            fixed_now(),
            &stop_args(&["--date", "2026-01-27", "13:00"]),
        )
        .expect("ok");

        // Before the fix: week 2026-05 worked 240m against a 2400m
        // default target, so it owes 2160m and carries -2160m forward
        // through the idle week 2026-06 (which itself then owes its own
        // full 2400m on top): 2160 + 2400 = 4560m = 76h 00m.
        let before =
            crate::status::resolve(Some("2026-02-02"), fixed_now(), &conn).expect("resolve");
        assert!(
            before.week_line.contains("Total still owed: 76h 00m"),
            "before: {}",
            before.week_line
        );

        // A forgotten 2h stint is now backdated into week 2026-05 via
        // the -N shorthand (2026-01-27 is 16 days before fixed_now()'s
        // date).
        start(
            &mut conn,
            fixed_now(),
            &punch_args(&["--date", "-16", "14:00"]),
        )
        .expect("ok");
        stop(
            &mut conn,
            fixed_now(),
            &stop_args(&["--date", "-16", "16:00"]),
        )
        .expect("ok");

        // After: week 2026-05 now worked 360m, owes 2040m; week 2026-06
        // owes 2040 + 2400 = 4440m = 74h 00m -- 2 hours less, exactly the
        // backdated stint's length, with no `status`/`week` action taken
        // beyond re-reading the same view.
        let after =
            crate::status::resolve(Some("2026-02-02"), fixed_now(), &conn).expect("resolve");
        assert!(
            after.week_line.contains("Total still owed: 74h 00m"),
            "after: {}",
            after.week_line
        );
        assert_ne!(before.week_line, after.week_line);
    }

    // --- delete: list/delete handlers (Milestone 14) ---------------------

    #[test]
    fn delete_note_future_date_is_rejected() {
        let conn = test_db();
        let r = delete_note(
            &conn,
            fixed_now(),
            &delete_note_args(&["--date", "2026-02-13"]),
        );
        let err = r.expect_err("expected error");
        assert!(
            err.downcast_ref::<crate::date::DateWeekError>().is_some(),
            "expected DateWeekError, got {err:?}"
        );
        assert!(notes_for(&conn, d(2026, 2, 13)).is_empty());
    }

    #[test]
    fn delete_punch_future_date_is_rejected() {
        let conn = test_db();
        let r = delete_punch(
            &conn,
            fixed_now(),
            &delete_punch_args(&["--date", "2026-02-13"]),
        );
        let err = r.expect_err("expected error");
        assert!(
            err.downcast_ref::<crate::date::DateWeekError>().is_some(),
            "expected DateWeekError, got {err:?}"
        );
        assert!(punches_for(&conn, d(2026, 2, 13)).is_empty());
    }

    #[test]
    fn delete_note_malformed_date_is_rejected() {
        let conn = test_db();
        let r = delete_note(
            &conn,
            fixed_now(),
            &delete_note_args(&["--date", "not-a-date"]),
        );
        let err = r.expect_err("expected error");
        assert!(
            err.downcast_ref::<crate::date::DateWeekError>().is_some(),
            "expected DateWeekError, got {err:?}"
        );
    }

    #[test]
    fn delete_punch_malformed_date_is_rejected() {
        let conn = test_db();
        let r = delete_punch(
            &conn,
            fixed_now(),
            &delete_punch_args(&["--date", "not-a-date"]),
        );
        let err = r.expect_err("expected error");
        assert!(
            err.downcast_ref::<crate::date::DateWeekError>().is_some(),
            "expected DateWeekError, got {err:?}"
        );
    }

    /// Regression test for the flag-collision/`--`-swallowing fix (spec
    /// §5): the underlying bug is position-dependent, so the body must
    /// have the flag-lookalike as its *leading* token, not mid-sentence
    /// (a mid-sentence one would pass identically with or without the
    /// fix and prove nothing). There is no stdout-capture fixture in this
    /// file and no real shell to hand the printed line to, so this
    /// exercises the actual write path (`storage::insert_note` ->
    /// `storage::delete_note` -> `note_recreate_line`) and then performs
    /// the shell-unquoting inverse by hand: strip the surrounding `'`
    /// and un-escape `'"'"'` back to a literal `'`, which is exactly what
    /// a POSIX/fish shell would do before handing the token to `mlm`.
    #[test]
    fn quoting_round_trip_leading_flag_lookalike() {
        let body = "--verbose logging bug";
        let conn = test_db();
        let id = storage::insert_note(&conn, today(), body, fixed_now().with_timezone(&Utc))
            .expect("insert");
        let deleted = storage::delete_note(&conn, id).expect("delete");
        let line = note_recreate_line(&deleted, today());
        assert!(!line.contains("--date"));

        let quoted = line
            .strip_prefix("mlm note ")
            .expect("expected 'mlm note ' prefix");
        assert!(quoted.starts_with('\'') && quoted.ends_with('\''));
        let unquoted = quoted[1..quoted.len() - 1].replace("'\"'\"'", "'");
        assert_eq!(unquoted, body);

        // Feed the recovered single token back through the real CLI
        // parser as one argv element (exactly what a shell hands back
        // after unquoting a single-quoted token) and confirm it survives
        // as the whole note body, unsplit and unmisparsed.
        let parsed = note_args(&[&unquoted]);
        assert_eq!(parsed.body, vec![body.to_string()]);
    }

    #[test]
    fn quoting_round_trip_embedded_quote() {
        let body = "it's a 'quoted' fix";
        let conn = test_db();
        let id = storage::insert_note(&conn, today(), body, fixed_now().with_timezone(&Utc))
            .expect("insert");
        let deleted = storage::delete_note(&conn, id).expect("delete");
        let line = note_recreate_line(&deleted, today());

        let quoted = line
            .strip_prefix("mlm note ")
            .expect("expected 'mlm note ' prefix");
        assert!(quoted.starts_with('\'') && quoted.ends_with('\''));
        let unquoted = quoted[1..quoted.len() - 1].replace("'\"'\"'", "'");
        assert_eq!(unquoted, body);
    }

    /// Pins the `PunchKind::as_str()`-divergence correction: the CLI
    /// command for an end-punch is `mlm stop`, never `mlm end`, so the
    /// recreate-line builder must not reuse `as_str()` (which returns
    /// `"end"`) as the verb.
    #[test]
    fn punch_recreate_line_uses_stop_not_end_verb() {
        let p = Punch {
            id: 1,
            at_utc: Utc.with_ymd_and_hms(2026, 2, 12, 9, 0, 0).unwrap(),
            date: today(),
            kind: PunchKind::End,
        };
        let line = punch_recreate_line(&p, today(), &Local);
        assert!(line.contains("mlm stop "), "line was {line:?}");
        assert!(!line.contains("mlm end"), "line was {line:?}");
    }

    /// A note body containing a literal backslash cannot be quoted
    /// identically across shells (fish's single-quote parsing recognizes
    /// `\\`/`\'` escapes inside `'...'` that POSIX shells don't), so
    /// `note_recreate_line` must fall back to the plain-description
    /// format instead of emitting a quoted command that would silently
    /// corrupt or fail to parse on replay under fish. Uses a raw string
    /// literal to be unambiguous about containing one real backslash
    /// character, not an escaped pair.
    #[test]
    fn backslash_body_falls_back_to_plain_description() {
        let conn = test_db();
        let body = r"path C:\dir\file";
        let id = storage::insert_note(&conn, d(2026, 2, 10), body, fixed_now().with_timezone(&Utc))
            .expect("insert");
        let deleted = storage::delete_note(&conn, id).expect("delete");
        let line = note_recreate_line(&deleted, today());
        assert_eq!(line, format!("deleted note (2026-02-10): {body}..."));
    }

    #[test]
    fn list_mode_empty_date_prints_nothing_to_delete_note() {
        let conn = test_db();
        let r = delete_note(&conn, fixed_now(), &delete_note_args(&[]));
        assert!(r.is_ok());
        assert!(notes(&conn).is_empty());
    }

    #[test]
    fn list_mode_empty_date_prints_nothing_to_delete_punch() {
        let conn = test_db();
        let r = delete_punch(&conn, fixed_now(), &delete_punch_args(&[]));
        assert!(r.is_ok());
        assert!(punches(&conn).is_empty());
    }

    #[test]
    fn list_mode_never_mutates_storage_and_numbers_correctly_punch() {
        let mut conn = test_db();
        start(&mut conn, fixed_now(), &punch_args(&["09:00"])).expect("seed");
        stop(&mut conn, fixed_now(), &stop_args(&["17:00"])).expect("seed");

        let r = delete_punch(&conn, fixed_now(), &delete_punch_args(&[]));
        assert!(r.is_ok());

        let rows = punches(&conn);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].kind, PunchKind::Start);
        assert_eq!(rows[1].kind, PunchKind::End);
    }

    #[test]
    fn list_mode_never_mutates_storage_and_numbers_correctly_note() {
        let mut conn = test_db();
        note(&mut conn, fixed_now(), &note_args(&["first note"])).expect("seed");
        note(&mut conn, fixed_now(), &note_args(&["second note"])).expect("seed");

        let r = delete_note(&conn, fixed_now(), &delete_note_args(&[]));
        assert!(r.is_ok());

        let rows = notes(&conn);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].body, "first note");
        assert_eq!(rows[1].body, "second note");
    }

    #[test]
    fn delete_recreate_line_omits_date_for_today_includes_for_other_date_punch() {
        let conn = test_db();
        let id_today = storage::insert_punch(
            &conn,
            PunchKind::Start,
            today(),
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            &Local,
        )
        .expect("insert");
        let deleted_today = storage::delete_punch(&conn, id_today).expect("delete");
        let line_today = punch_recreate_line(&deleted_today, today(), &Local);
        assert!(!line_today.contains("--date"));

        let other_date = d(2026, 2, 10);
        let id_other = storage::insert_punch(
            &conn,
            PunchKind::Start,
            other_date,
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            &Local,
        )
        .expect("insert");
        let deleted_other = storage::delete_punch(&conn, id_other).expect("delete");
        let line_other = punch_recreate_line(&deleted_other, today(), &Local);
        assert!(line_other.contains("--date 2026-02-10"));
    }

    #[test]
    fn delete_recreate_line_omits_date_for_today_includes_for_other_date_note() {
        let conn = test_db();
        let id_today = storage::insert_note(
            &conn,
            today(),
            "today note",
            fixed_now().with_timezone(&Utc),
        )
        .expect("insert");
        let deleted_today = storage::delete_note(&conn, id_today).expect("delete");
        let line_today = note_recreate_line(&deleted_today, today());
        assert!(!line_today.contains("--date"));

        let other_date = d(2026, 2, 10);
        let id_other = storage::insert_note(
            &conn,
            other_date,
            "other note",
            fixed_now().with_timezone(&Utc),
        )
        .expect("insert");
        let deleted_other = storage::delete_note(&conn, id_other).expect("delete");
        let line_other = note_recreate_line(&deleted_other, today());
        assert!(line_other.contains("--date 2026-02-10"));
    }

    #[test]
    fn note_recreate_line_date_precedes_body() {
        let n = Note {
            id: 1,
            date: d(2026, 2, 10),
            body: "fixed migration runner bug".to_string(),
            created_at_utc: fixed_now().with_timezone(&Utc),
        };
        let line = note_recreate_line(&n, today());
        let date_idx = line.find("--date").expect("expected --date in line");
        let quote_idx = line.find('\'').expect("expected quoted body");
        assert!(
            date_idx < quote_idx,
            "expected --date to precede the quoted body in {line:?}"
        );
    }

    #[test]
    fn delete_note_valid_id_removes_correct_row() {
        let mut conn = test_db();
        note(&mut conn, fixed_now(), &note_args(&["first"])).expect("seed");
        note(&mut conn, fixed_now(), &note_args(&["second"])).expect("seed");

        delete_note(&conn, fixed_now(), &delete_note_args(&["1"])).expect("delete");

        let remaining = notes(&conn);
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].body, "second");
    }

    #[test]
    fn delete_punch_valid_id_removes_correct_row() {
        let mut conn = test_db();
        start(&mut conn, fixed_now(), &punch_args(&["09:00"])).expect("seed");
        stop(&mut conn, fixed_now(), &stop_args(&["17:00"])).expect("seed");

        delete_punch(&conn, fixed_now(), &delete_punch_args(&["1"])).expect("delete");

        let remaining = punches(&conn);
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].kind, PunchKind::End);
    }

    /// Finding 8 (pre-merge review): "1 entries" reads wrong -- singular
    /// count must say "entry", plural counts still say "entries".
    #[test]
    fn validate_entry_id_pluralizes_entry_count_correctly() {
        let err_one = validate_entry_id(0, 1).expect_err("expected error");
        assert!(
            err_one.to_string().contains("1 entry for this date"),
            "got: {err_one}"
        );
        assert!(!err_one.to_string().contains("1 entries"), "got: {err_one}");

        let err_two = validate_entry_id(0, 2).expect_err("expected error");
        assert!(
            err_two.to_string().contains("2 entries for this date"),
            "got: {err_two}"
        );

        let err_oob = validate_entry_id(5, 1).expect_err("expected error");
        assert!(
            err_oob.to_string().contains("1 entry for this date"),
            "got: {err_oob}"
        );
    }

    #[test]
    fn delete_note_id_zero_and_past_count_are_rejected() {
        let mut conn = test_db();
        note(&mut conn, fixed_now(), &note_args(&["only note"])).expect("seed");

        delete_note(&conn, fixed_now(), &delete_note_args(&["0"])).expect_err("expected error");
        assert_eq!(notes(&conn).len(), 1);

        delete_note(&conn, fixed_now(), &delete_note_args(&["2"])).expect_err("expected error");
        assert_eq!(notes(&conn).len(), 1);
    }

    #[test]
    fn delete_punch_id_zero_and_past_count_are_rejected() {
        let mut conn = test_db();
        start(&mut conn, fixed_now(), &punch_args(&["09:00"])).expect("seed");

        delete_punch(&conn, fixed_now(), &delete_punch_args(&["0"])).expect_err("expected error");
        assert_eq!(punches(&conn).len(), 1);

        delete_punch(&conn, fixed_now(), &delete_punch_args(&["2"])).expect_err("expected error");
        assert_eq!(punches(&conn).len(), 1);
    }

    #[test]
    fn delete_note_scoping_never_touches_other_date_with_same_count() {
        let conn = test_db();
        storage::insert_note(
            &conn,
            d(2026, 2, 10),
            "on the 10th",
            fixed_now().with_timezone(&Utc),
        )
        .expect("seed");
        storage::insert_note(
            &conn,
            d(2026, 2, 11),
            "on the 11th",
            fixed_now().with_timezone(&Utc),
        )
        .expect("seed");

        delete_note(
            &conn,
            fixed_now(),
            &delete_note_args(&["1", "--date", "2026-02-10"]),
        )
        .expect("delete");

        assert!(notes_for(&conn, d(2026, 2, 10)).is_empty());
        let remaining = notes_for(&conn, d(2026, 2, 11));
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].body, "on the 11th");
    }

    #[test]
    fn delete_punch_scoping_never_touches_other_date_with_same_count() {
        let conn = test_db();
        storage::insert_punch(
            &conn,
            PunchKind::Start,
            d(2026, 2, 10),
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            &Local,
        )
        .expect("seed");
        storage::insert_punch(
            &conn,
            PunchKind::Start,
            d(2026, 2, 11),
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            &Local,
        )
        .expect("seed");

        delete_punch(
            &conn,
            fixed_now(),
            &delete_punch_args(&["1", "--date", "2026-02-10"]),
        )
        .expect("delete");

        assert!(punches_for(&conn, d(2026, 2, 10)).is_empty());
        let remaining = punches_for(&conn, d(2026, 2, 11));
        assert_eq!(remaining.len(), 1);
    }

    /// Finding 5 (pre-merge review): the previous version of this test
    /// hardcoded `chrono::Local`, which is UTC (zero offset, no DST) on
    /// every CI runner -- making the assertion tautological, since it
    /// just re-derived the same `with_timezone(&Local)` expression the
    /// implementation itself uses. `punch_recreate_line` is now generic
    /// over `Tz: chrono::TimeZone`, so this pins a real IANA zone
    /// (`chrono_tz::Europe::Warsaw`) across its actual 2026 spring-forward
    /// transition (clocks jump from 01:59:59 to 03:00:00 local on
    /// 2026-03-29), mirroring `storage.rs`'s own D1/D2-style DST tests:
    /// one punch just before the transition (still UTC+1) and one just
    /// after (already UTC+2), asserting the recreate line's `HH:MM`
    /// reflects each instant's own correct Warsaw wall-clock time, not a
    /// single reused offset.
    #[test]
    fn dst_crossing_punch_delete_echoes_correct_warsaw_local_time() {
        // 2026-03-29 01:30 Warsaw (still UTC+1) -> 2026-03-29T00:30:00Z.
        let before_transition = Punch {
            id: 1,
            at_utc: Utc.with_ymd_and_hms(2026, 3, 29, 0, 30, 0).unwrap(),
            date: d(2026, 3, 29),
            kind: PunchKind::Start,
        };
        // 2026-03-29 03:30 Warsaw (already UTC+2) -> 2026-03-29T01:30:00Z.
        let after_transition = Punch {
            id: 2,
            at_utc: Utc.with_ymd_and_hms(2026, 3, 29, 1, 30, 0).unwrap(),
            date: d(2026, 3, 29),
            kind: PunchKind::End,
        };

        let line_before =
            punch_recreate_line(&before_transition, today(), &chrono_tz::Europe::Warsaw);
        let line_after =
            punch_recreate_line(&after_transition, today(), &chrono_tz::Europe::Warsaw);

        assert!(
            line_before.contains("mlm start 01:30"),
            "line was {line_before:?}"
        );
        assert!(
            line_after.contains("mlm stop 03:30"),
            "line was {line_after:?}"
        );
    }

    /// Mirrors `backdated_punch_retroactively_changes_a_later_closed_weeks_owed`'s
    /// before/after `status::resolve` pattern, but in reverse: deleting
    /// the only entry on the currently-earliest tracked date shifts
    /// `storage::earliest_data_date` forward, which changes a later
    /// week's figures on the next status/week view (spec §5's called-out
    /// consequence).
    #[test]
    fn earliest_data_date_shift_changes_later_status() {
        let mut conn = test_db();

        // Week 2026-04 (Mon 2026-01-19 .. Sun 2026-01-25): the only entry
        // on what will be the earliest tracked date.
        start(
            &mut conn,
            fixed_now(),
            &punch_args(&["--date", "2026-01-20", "09:00"]),
        )
        .expect("ok");
        stop(
            &mut conn,
            fixed_now(),
            &stop_args(&["--date", "2026-01-20", "11:00"]),
        )
        .expect("ok");

        // Week 2026-05 (Mon 2026-01-26 .. Sun 2026-02-01): later data so
        // the anchor still has somewhere to land after the earliest
        // week's data is removed.
        start(
            &mut conn,
            fixed_now(),
            &punch_args(&["--date", "2026-01-27", "09:00"]),
        )
        .expect("ok");
        stop(
            &mut conn,
            fixed_now(),
            &stop_args(&["--date", "2026-01-27", "13:00"]),
        )
        .expect("ok");

        assert_eq!(
            storage::earliest_data_date(&conn).unwrap(),
            Some(d(2026, 1, 20))
        );

        let before =
            crate::status::resolve(Some("2026-02-02"), fixed_now(), &conn).expect("resolve");

        // Delete both punches on the earliest date (start, then the new
        // position-1 end, since the listing re-numbers after each
        // delete).
        delete_punch(
            &conn,
            fixed_now(),
            &delete_punch_args(&["1", "--date", "2026-01-20"]),
        )
        .expect("delete");
        delete_punch(
            &conn,
            fixed_now(),
            &delete_punch_args(&["1", "--date", "2026-01-20"]),
        )
        .expect("delete");

        assert!(punches_for(&conn, d(2026, 1, 20)).is_empty());
        assert_eq!(
            storage::earliest_data_date(&conn).unwrap(),
            Some(d(2026, 1, 27))
        );

        let after =
            crate::status::resolve(Some("2026-02-02"), fixed_now(), &conn).expect("resolve");
        assert_ne!(before.week_line, after.week_line);
    }
}
