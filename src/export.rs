//! `mlm export notes [--date DATE | --week WEEK_ID | --month MONTH]`
//! (SPEC.md §3.8, §7.5).
//!
//! Resolves one date, ISO week or calendar month (the current week by
//! default), makes one ranged note read and prints one line per note as
//! `YYYY-MM-DD`, two spaces, then the body. No range is checked against
//! today, so a future range prints nothing.

use std::io::Write;

use chrono::NaiveDate;
use rusqlite::Connection;

use crate::cli::ExportNotesArgs;
use crate::date::{DateWeekError, WeekId, format_date, parse_month, parse_week_id, resolve_date};
use crate::emit;
use crate::storage::{self, Note};

/// The inclusive `(first, last)` dates the flags name. Clap's group
/// guarantees at most one flag is set.
pub fn resolve_range(
    args: &ExportNotesArgs,
    today: NaiveDate,
) -> Result<(NaiveDate, NaiveDate), DateWeekError> {
    if let Some(s) = &args.date {
        let date = resolve_date(s, today)?;
        return Ok((date, date));
    }
    if let Some(s) = &args.week {
        return Ok(parse_week_id(s, today)?.span());
    }
    if let Some(s) = &args.month {
        return Ok(parse_month(s, today)?.span());
    }
    Ok(WeekId::current(today).span())
}

/// One `YYYY-MM-DD  body\n` line per note, in the order given.
pub fn render_lines(notes: &[Note]) -> String {
    notes
        .iter()
        .map(|n| format!("{}  {}\n", format_date(n.date), n.body))
        .collect()
}

/// `mlm export notes`. A parse or read error returns before anything is
/// written, so stdout stays empty.
pub fn run_notes<W: Write>(
    conn: &Connection,
    today: NaiveDate,
    args: &ExportNotesArgs,
    out: &mut W,
) -> anyhow::Result<()> {
    let (from, to) = resolve_range(args, today)?;
    let notes = storage::notes_in_range(conn, from, to)?;
    emit(out, &render_lines(&notes))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::date::{ArgKind, Cause};
    use crate::db;
    use chrono::{TimeZone, Utc};

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("test fixture is a real date")
    }

    fn args(date: Option<&str>, week: Option<&str>, month: Option<&str>) -> ExportNotesArgs {
        ExportNotesArgs {
            date: date.map(str::to_string),
            week: week.map(str::to_string),
            month: month.map(str::to_string),
        }
    }

    fn note(id: i64, date: NaiveDate, body: &str) -> Note {
        Note {
            id,
            date,
            body: body.to_string(),
            created_at_utc: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        }
    }

    fn fresh_db() -> Connection {
        let mut conn = Connection::open_in_memory().expect("in-memory db");
        db::apply_migrations(&mut conn).expect("migrations apply");
        conn
    }

    #[test]
    fn default_range_is_the_current_iso_week() {
        let today = d(2026, 10, 24);
        assert_eq!(
            resolve_range(&args(None, None, None), today),
            Ok((d(2026, 10, 19), d(2026, 10, 25)))
        );
    }

    #[test]
    fn date_range_is_one_day_and_allows_minus_n_and_the_future() {
        let today = d(2026, 10, 24);
        assert_eq!(
            resolve_range(&args(Some("-1"), None, None), today),
            Ok((d(2026, 10, 23), d(2026, 10, 23)))
        );
        assert_eq!(
            resolve_range(&args(Some("2026-10-19"), None, None), today),
            Ok((d(2026, 10, 19), d(2026, 10, 19)))
        );
        assert_eq!(
            resolve_range(&args(Some("2099-06-15"), None, None), today),
            Ok((d(2099, 6, 15), d(2099, 6, 15)))
        );
    }

    #[test]
    fn week_range_is_monday_to_sunday() {
        let today = d(2026, 10, 24);
        for w in ["2026-43", "43"] {
            assert_eq!(
                resolve_range(&args(None, Some(w), None), today),
                Ok((d(2026, 10, 19), d(2026, 10, 25))),
                "{w}"
            );
        }
    }

    #[test]
    fn bare_week_and_bare_month_use_different_years() {
        let today = d(2025, 12, 30);
        assert_eq!(
            resolve_range(&args(None, Some("1"), None), today),
            Ok((d(2025, 12, 29), d(2026, 1, 4)))
        );
        assert_eq!(
            resolve_range(&args(None, None, Some("1")), today),
            Ok((d(2025, 1, 1), d(2025, 1, 31)))
        );
    }

    #[test]
    fn month_range_is_first_to_last_day() {
        let today = d(2026, 10, 24);
        for m in ["2026-3", "2026-03"] {
            assert_eq!(
                resolve_range(&args(None, None, Some(m)), today),
                Ok((d(2026, 3, 1), d(2026, 3, 31))),
                "{m}"
            );
        }
    }

    #[test]
    fn range_errors_carry_their_arg_kind() {
        let today = d(2026, 10, 24);
        let err = |a: ExportNotesArgs| {
            let e = resolve_range(&a, today).unwrap_err();
            (e.arg, e.cause)
        };
        assert_eq!(
            err(args(None, None, Some("abc"))),
            (ArgKind::Month, Cause::Shape)
        );
        assert_eq!(
            err(args(None, Some("-0"), None)),
            (ArgKind::WeekId, Cause::Shape)
        );
        assert_eq!(
            err(args(None, Some("-9999999999999999999999999"), None)),
            (ArgKind::WeekId, Cause::OffsetOutOfRange)
        );
        assert_eq!(
            err(args(Some("abc"), None, None)),
            (ArgKind::Date, Cause::Shape)
        );
    }

    #[test]
    fn minus_n_week_and_month_ranges() {
        let today = d(2026, 10, 24);
        for w in ["-1", "-01"] {
            assert_eq!(
                resolve_range(&args(None, Some(w), None), today),
                Ok((d(2026, 10, 12), d(2026, 10, 18))),
                "{w}"
            );
        }
        assert_eq!(
            resolve_range(&args(None, None, Some("-1")), today),
            Ok((d(2026, 9, 1), d(2026, 9, 30)))
        );
        assert_eq!(
            resolve_range(&args(None, None, Some("-1")), d(2026, 1, 7)),
            Ok((d(2025, 12, 1), d(2025, 12, 31)))
        );
    }

    #[test]
    fn render_lines_formats_date_two_spaces_body() {
        let notes = [
            note(1, d(2026, 10, 19), "fixed migration runner bug"),
            note(2, d(2026, 10, 21), "code review for #41"),
        ];
        assert_eq!(
            render_lines(&notes),
            "2026-10-19  fixed migration runner bug\n2026-10-21  code review for #41\n"
        );
        assert_eq!(render_lines(&[]), "");
    }

    #[test]
    fn run_notes_fails_on_a_broken_note_read() {
        let conn = fresh_db();
        conn.execute_batch("ALTER TABLE notes RENAME COLUMN body TO body_gone")
            .expect("rename column");
        let err = run_notes(
            &conn,
            d(2026, 1, 7),
            &args(None, Some("2026-02"), None),
            &mut std::io::sink(),
        )
        .expect_err("a broken read must be an error");
        assert!(err.to_string().starts_with("database error:"), "{err}");
    }

    #[test]
    fn run_notes_succeeds_on_an_empty_range() {
        let conn = fresh_db();
        assert!(
            run_notes(
                &conn,
                d(2026, 1, 7),
                &args(None, None, None),
                &mut std::io::sink()
            )
            .is_ok()
        );
    }

    #[test]
    fn run_notes_writes_lines() {
        let conn = fresh_db();
        let at = Utc.with_ymd_and_hms(2026, 10, 19, 10, 0, 0).unwrap();
        storage::insert_note(&conn, d(2026, 10, 19), "piped", at).unwrap();
        let a = args(Some("2026-10-19"), None, None);
        let mut out = Vec::new();
        run_notes(&conn, d(2026, 10, 24), &a, &mut out).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "2026-10-19  piped\n");

        let mut full: &mut [u8] = &mut [];
        let err = run_notes(&conn, d(2026, 10, 24), &a, &mut full).unwrap_err();
        assert!(err.downcast_ref::<crate::OutputError>().is_some());
    }

    #[test]
    fn run_notes_empty_range_writes_nothing_even_to_a_failing_writer() {
        let conn = fresh_db();
        let mut full: &mut [u8] = &mut [];
        assert!(run_notes(&conn, d(2026, 1, 7), &args(None, None, None), &mut full).is_ok());
    }
}
