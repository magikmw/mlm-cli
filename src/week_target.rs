//! `week target` command (SPEC.md §3.7, §2.3, §6.1).
//!
//! Wired into `main`'s dispatch by a later milestone (PLAN.md's
//! Milestone 8 scope note); this module is unit-testable standalone
//! against an in-memory `Connection` in the meantime.
#![allow(dead_code)]

use chrono::NaiveDate;
use rusqlite::{Connection, OptionalExtension};

use crate::cli::WeekTargetArgs;
use crate::date::{WeekId, parse_week_id};
use crate::time::parse_duration;

/// Set (insert or replace) a week's absolute target override (§2.3, §3.7).
///
/// `ON CONFLICT ... DO UPDATE` (rather than `INSERT OR REPLACE`) mutates
/// the row in place instead of delete-then-insert. `week_id.to_key()` is
/// the normalized `YYYY-WW` form, never the raw user token, so `2026-7`
/// and `2026-07` key the same row.
pub fn set_week_target(
    conn: &Connection,
    week_id: &WeekId,
    target_minutes: i64,
) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO week_targets (week_id, target_minutes)
         VALUES (?1, ?2)
         ON CONFLICT(week_id) DO UPDATE SET target_minutes = excluded.target_minutes",
        rusqlite::params![week_id.to_key(), target_minutes],
    )?;
    Ok(())
}

/// The stored override for a week, or `None` when the week has no row
/// (caller applies the 40h default — §2.3/§6.2).
pub fn get_week_target(conn: &Connection, week_id: &WeekId) -> anyhow::Result<Option<i64>> {
    conn.query_row(
        "SELECT target_minutes FROM week_targets WHERE week_id = ?1",
        rusqlite::params![week_id.to_key()],
        |row| row.get(0),
    )
    .optional()
    .map_err(anyhow::Error::from)
}

/// `mlm week target [WEEK_ID] DURATION` (§3.7).
///
/// `today` is the injected "now" (never read from a global clock here),
/// which is what makes the omitted-`WEEK_ID` default deterministic under
/// test. WEEK_ID is validated before DURATION (a documented choice, not
/// a spec mandate) so that with both tokens malformed, the leftmost one
/// wins. Both parses complete before the single upsert call, so a
/// failure at either step cannot leave a write behind.
pub fn run(conn: &Connection, today: NaiveDate, args: &WeekTargetArgs) -> anyhow::Result<()> {
    let (week_tok, dur_tok) = args.split();

    let week = match week_tok {
        Some(tok) => parse_week_id(tok, today)?,
        None => WeekId::current(today),
    };

    let minutes = parse_duration(dur_tok)?;
    debug_assert!(minutes >= 0, "parse_duration guarantees non-negative");

    set_week_target(conn, &week, minutes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn new_conn() -> Connection {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");
        db::apply_migrations(&mut conn).expect("apply migrations");
        conn
    }

    fn wk(y: i32, w: u32) -> WeekId {
        WeekId::new(y, w, "test").expect("valid test week")
    }

    fn args(tokens: &[&str]) -> WeekTargetArgs {
        WeekTargetArgs {
            args: tokens.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn row_count(conn: &Connection) -> i64 {
        conn.query_row("SELECT COUNT(*) FROM week_targets", [], |r| r.get(0))
            .expect("count rows")
    }

    // --- 5.1 unit tests: set_week_target / get_week_target -----------------

    // U1
    #[test]
    fn set_week_target_inserts_new_row() {
        let conn = new_conn();
        set_week_target(&conn, &wk(2026, 7), 2010).unwrap();
        assert_eq!(get_week_target(&conn, &wk(2026, 7)).unwrap(), Some(2010));
    }

    // U2
    #[test]
    fn set_week_target_zero_is_accepted() {
        let conn = new_conn();
        set_week_target(&conn, &wk(2026, 7), 0).unwrap();
        assert_eq!(get_week_target(&conn, &wk(2026, 7)).unwrap(), Some(0));
    }

    // U3
    #[test]
    fn set_week_target_replaces_existing() {
        let conn = new_conn();
        set_week_target(&conn, &wk(2026, 7), 2400).unwrap();
        set_week_target(&conn, &wk(2026, 7), 2010).unwrap();
        assert_eq!(get_week_target(&conn, &wk(2026, 7)).unwrap(), Some(2010));
        assert_eq!(row_count(&conn), 1);
    }

    // U4
    #[test]
    fn set_week_target_absent_week_reads_none() {
        let conn = new_conn();
        assert_eq!(get_week_target(&conn, &wk(2030, 1)).unwrap(), None);
    }

    // U5
    #[test]
    fn set_week_target_negative_is_rejected_by_schema() {
        let conn = new_conn();
        let err = set_week_target(&conn, &wk(2026, 7), -1).unwrap_err();
        assert!(err.downcast_ref::<rusqlite::Error>().is_some());
        assert_eq!(row_count(&conn), 0);
    }

    // U6
    #[test]
    fn set_week_target_keys_on_normalized_id() {
        let conn = new_conn();
        let today = NaiveDate::from_ymd_opt(2026, 2, 12).unwrap();
        let from_unpadded = parse_week_id("2026-7", today).unwrap();
        let from_padded = parse_week_id("2026-07", today).unwrap();
        set_week_target(&conn, &from_unpadded, 100).unwrap();
        set_week_target(&conn, &from_padded, 200).unwrap();
        assert_eq!(row_count(&conn), 1);
        assert_eq!(get_week_target(&conn, &wk(2026, 7)).unwrap(), Some(200));
    }

    // --- 5.3 command-body tests: run -----------------------------------

    // R1
    #[test]
    fn explicit_week_id_writes_that_week() {
        let conn = new_conn();
        let today = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        run(&conn, today, &args(&["2026-07", "33h30m"])).unwrap();
        assert_eq!(get_week_target(&conn, &wk(2026, 7)).unwrap(), Some(2010));
    }

    // R2
    #[test]
    fn omitted_week_id_defaults_to_current_week() {
        let conn = new_conn();
        let today = NaiveDate::from_ymd_opt(2026, 2, 12).unwrap(); // Thu, ISO week 2026-07
        run(&conn, today, &args(&["33h30m"])).unwrap();
        assert_eq!(row_count(&conn), 1);
        assert_eq!(get_week_target(&conn, &wk(2026, 7)).unwrap(), Some(2010));

        // Year-boundary case: Gregorian 2025-12-29 is ISO week 2026-01
        // (per date.rs's own T73 fixture) — the ISO year, not the
        // Gregorian one, must be used.
        let conn2 = new_conn();
        let boundary_today = NaiveDate::from_ymd_opt(2025, 12, 29).unwrap();
        run(&conn2, boundary_today, &args(&["33h30m"])).unwrap();
        assert_eq!(get_week_target(&conn2, &wk(2026, 1)).unwrap(), Some(2010));
    }

    // R3
    #[test]
    fn zero_duration_accepted() {
        let conn = new_conn();
        let today = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        run(&conn, today, &args(&["2026-07", "0h"])).unwrap();
        assert_eq!(get_week_target(&conn, &wk(2026, 7)).unwrap(), Some(0));

        let conn2 = new_conn();
        run(&conn2, today, &args(&["2026-08", "0m"])).unwrap();
        assert_eq!(get_week_target(&conn2, &wk(2026, 8)).unwrap(), Some(0));
    }

    // R4
    #[test]
    fn negative_duration_rejected_no_write() {
        let conn = new_conn();
        let today = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let err = run(&conn, today, &args(&["2026-07", "-5h"])).unwrap_err();
        assert!(
            err.downcast_ref::<crate::time::DurationParseError>()
                .is_some()
        );
        assert_eq!(row_count(&conn), 0);
    }

    // R5
    #[test]
    fn malformed_week_id_rejected_no_write() {
        let today = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        for bad in ["0", "abcd", "2027-53"] {
            let conn = new_conn();
            let err = run(&conn, today, &args(&[bad, "33h30m"])).unwrap_err();
            assert!(
                err.downcast_ref::<crate::date::DateWeekError>().is_some(),
                "input {bad}: {err}"
            );
            assert_eq!(row_count(&conn), 0);
        }
    }

    // R6
    #[test]
    fn unpadded_week_id_normalized() {
        let conn = new_conn();
        let today = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        run(&conn, today, &args(&["2026-7", "33h30m"])).unwrap();
        assert_eq!(get_week_target(&conn, &wk(2026, 7)).unwrap(), Some(2010));
    }

    // R7
    #[test]
    fn malformed_duration_rejected_no_write() {
        let today = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        for bad in ["10", "10x", ""] {
            let conn = new_conn();
            let err = run(&conn, today, &args(&["2026-07", bad])).unwrap_err();
            assert!(
                err.downcast_ref::<crate::time::DurationParseError>()
                    .is_some(),
                "input {bad:?}: {err}"
            );
            assert_eq!(row_count(&conn), 0);
        }
    }

    // R8
    #[test]
    fn week_id_validated_before_duration() {
        let conn = new_conn();
        let today = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let err = run(&conn, today, &args(&["2027-53", "-5h"])).unwrap_err();
        assert!(err.downcast_ref::<crate::date::DateWeekError>().is_some());
        assert_eq!(row_count(&conn), 0);
    }

    // R9
    #[test]
    fn override_replaces_not_duplicates() {
        let conn = new_conn();
        let today = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        run(&conn, today, &args(&["2026-07", "40h"])).unwrap();
        run(&conn, today, &args(&["2026-07", "33h30m"])).unwrap();
        assert_eq!(row_count(&conn), 1);
        assert_eq!(get_week_target(&conn, &wk(2026, 7)).unwrap(), Some(2010));
    }

    // R10
    #[test]
    fn succeeds_for_a_week_with_no_data() {
        let conn = new_conn();
        let today = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        run(&conn, today, &args(&["2099-01", "10h"])).unwrap();
        assert_eq!(get_week_target(&conn, &wk(2099, 1)).unwrap(), Some(600));
    }
}
