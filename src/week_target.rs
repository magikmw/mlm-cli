//! `week target` command (SPEC.md §3.7, §2.3, §6.1).
//!
//! Wired into `main`'s dispatch by a later milestone (PLAN.md's
//! Milestone 8 scope note); this module is unit-testable standalone
//! against an in-memory `Connection` in the meantime.
#![allow(dead_code)]

use std::io::Write;

use chrono::NaiveDate;
use rusqlite::{Connection, OptionalExtension};

use crate::cli::WeekTargetArgs;
use crate::date::{WeekId, parse_week_id};
use crate::emit;
use crate::time::{format_minutes, parse_duration};

/// The largest target `week target` accepts: the hours in a week (168h).
pub const MAX_TARGET_MINUTES: i64 = 168 * 60;

/// `week target` with a DURATION above `MAX_TARGET_MINUTES` (spec §4).
/// Holds the parsed minutes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetTooLarge(pub i64);

impl std::fmt::Display for TargetTooLarge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "target cannot exceed {} (got {})",
            format_minutes(MAX_TARGET_MINUTES),
            format_minutes(self.0)
        )
    }
}

impl std::error::Error for TargetTooLarge {}

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

/// The success line of `week target` (SPEC §3.5), e.g.
/// `Week 2026-41 target: 20h 00m`.
pub fn echo_line(week: &WeekId, minutes: i64) -> String {
    format!("Week {week} target: {}", format_minutes(minutes))
}

/// Parse, write, and return the echo line. Nothing prints here.
///
/// `today` is the injected "now" (never read from a global clock here),
/// which is what makes the omitted-`WEEK_ID` default deterministic under
/// test. WEEK_ID is validated before DURATION (a documented choice, not
/// a spec mandate) so that with both tokens malformed, the leftmost one
/// wins. The order is WEEK_ID, then the DURATION parse, then the 168h
/// cap. All three run before the single upsert call, so a failure at
/// any step cannot leave a write behind.
pub fn apply(conn: &Connection, today: NaiveDate, args: &WeekTargetArgs) -> anyhow::Result<String> {
    let (week_tok, dur_tok) = args.split();

    let week = match week_tok {
        Some(tok) => parse_week_id(tok, today)?,
        None => WeekId::current(today),
    };

    let minutes = parse_duration(dur_tok)?;
    debug_assert!(minutes >= 0, "parse_duration guarantees non-negative");

    if minutes > MAX_TARGET_MINUTES {
        return Err(TargetTooLarge(minutes).into());
    }

    set_week_target(conn, &week, minutes)?;
    Ok(echo_line(&week, minutes))
}

/// `mlm week target [WEEK_ID] DURATION` (§3.7): writes the override, then
/// writes one line to `out` (§3.5). The target stays written when the
/// output fails.
pub fn run<W: Write>(
    conn: &Connection,
    today: NaiveDate,
    args: &WeekTargetArgs,
    out: &mut W,
) -> anyhow::Result<()> {
    let line = apply(conn, today, args)?;
    emit(out, &format!("{line}\n"))?;
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

    // --- echo line (§3.5) ------------------------------------------------

    #[test]
    fn echo_line_formats_week_and_duration() {
        assert_eq!(
            echo_line(&wk(2026, 41), 1200),
            "Week 2026-41 target: 20h 00m"
        );
    }

    #[test]
    fn apply_returns_echo_and_writes_row() {
        let conn = new_conn();
        let today = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let line = apply(&conn, today, &args(&["2026-41", "20h"])).unwrap();
        assert_eq!(line, "Week 2026-41 target: 20h 00m");
        assert_eq!(get_week_target(&conn, &wk(2026, 41)).unwrap(), Some(1200));
    }

    #[test]
    fn apply_without_week_echoes_current_week() {
        let conn = new_conn();
        let today = NaiveDate::from_ymd_opt(2026, 2, 12).unwrap();
        let line = apply(&conn, today, &args(&["33h30m"])).unwrap();
        assert_eq!(line, "Week 2026-07 target: 33h 30m");
    }

    #[test]
    fn apply_failures_return_err_and_write_nothing() {
        let conn = new_conn();
        let today = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        assert!(apply(&conn, today, &args(&["2027-53", "20h"])).is_err());
        assert!(apply(&conn, today, &args(&["2026-07", "abc"])).is_err());
        assert_eq!(row_count(&conn), 0);
    }

    // --- 5.3 command-body tests: run -----------------------------------

    // R1
    #[test]
    fn explicit_week_id_writes_that_week() {
        let conn = new_conn();
        let today = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        run(
            &conn,
            today,
            &args(&["2026-07", "33h30m"]),
            &mut std::io::sink(),
        )
        .unwrap();
        assert_eq!(get_week_target(&conn, &wk(2026, 7)).unwrap(), Some(2010));
    }

    // R2
    #[test]
    fn omitted_week_id_defaults_to_current_week() {
        let conn = new_conn();
        let today = NaiveDate::from_ymd_opt(2026, 2, 12).unwrap(); // Thu, ISO week 2026-07
        run(&conn, today, &args(&["33h30m"]), &mut std::io::sink()).unwrap();
        assert_eq!(row_count(&conn), 1);
        assert_eq!(get_week_target(&conn, &wk(2026, 7)).unwrap(), Some(2010));

        // Year-boundary case: Gregorian 2025-12-29 is ISO week 2026-01
        // (per date.rs's own T73 fixture) — the ISO year, not the
        // Gregorian one, must be used.
        let conn2 = new_conn();
        let boundary_today = NaiveDate::from_ymd_opt(2025, 12, 29).unwrap();
        run(
            &conn2,
            boundary_today,
            &args(&["33h30m"]),
            &mut std::io::sink(),
        )
        .unwrap();
        assert_eq!(get_week_target(&conn2, &wk(2026, 1)).unwrap(), Some(2010));
    }

    // R3
    #[test]
    fn zero_duration_accepted() {
        let conn = new_conn();
        let today = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        run(
            &conn,
            today,
            &args(&["2026-07", "0h"]),
            &mut std::io::sink(),
        )
        .unwrap();
        assert_eq!(get_week_target(&conn, &wk(2026, 7)).unwrap(), Some(0));

        let conn2 = new_conn();
        run(
            &conn2,
            today,
            &args(&["2026-08", "0m"]),
            &mut std::io::sink(),
        )
        .unwrap();
        assert_eq!(get_week_target(&conn2, &wk(2026, 8)).unwrap(), Some(0));
    }

    // R4
    #[test]
    fn negative_duration_rejected_no_write() {
        let conn = new_conn();
        let today = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let err = run(
            &conn,
            today,
            &args(&["2026-07", "-5h"]),
            &mut std::io::sink(),
        )
        .unwrap_err();
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
            let err = run(&conn, today, &args(&[bad, "33h30m"]), &mut std::io::sink()).unwrap_err();
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
        run(
            &conn,
            today,
            &args(&["2026-7", "33h30m"]),
            &mut std::io::sink(),
        )
        .unwrap();
        assert_eq!(get_week_target(&conn, &wk(2026, 7)).unwrap(), Some(2010));
    }

    // R7
    #[test]
    fn malformed_duration_rejected_no_write() {
        let today = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        for bad in ["10", "10x", ""] {
            let conn = new_conn();
            let err =
                run(&conn, today, &args(&["2026-07", bad]), &mut std::io::sink()).unwrap_err();
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
        let err = run(
            &conn,
            today,
            &args(&["2027-53", "-5h"]),
            &mut std::io::sink(),
        )
        .unwrap_err();
        assert!(err.downcast_ref::<crate::date::DateWeekError>().is_some());
        assert_eq!(row_count(&conn), 0);
    }

    // R9
    #[test]
    fn override_replaces_not_duplicates() {
        let conn = new_conn();
        let today = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        run(
            &conn,
            today,
            &args(&["2026-07", "40h"]),
            &mut std::io::sink(),
        )
        .unwrap();
        run(
            &conn,
            today,
            &args(&["2026-07", "33h30m"]),
            &mut std::io::sink(),
        )
        .unwrap();
        assert_eq!(row_count(&conn), 1);
        assert_eq!(get_week_target(&conn, &wk(2026, 7)).unwrap(), Some(2010));
    }

    // R10
    #[test]
    fn succeeds_for_a_week_with_no_data() {
        let conn = new_conn();
        let today = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        run(
            &conn,
            today,
            &args(&["2099-01", "10h"]),
            &mut std::io::sink(),
        )
        .unwrap();
        assert_eq!(get_week_target(&conn, &wk(2099, 1)).unwrap(), Some(600));
    }

    // --- cap (panic-fixes spec §4) -------------------------------------

    #[test]
    fn cap_boundary_values_are_accepted() {
        let today = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        for dur in ["168h", "167h60m", "10080m"] {
            let conn = new_conn();
            let line = apply(&conn, today, &args(&["2026-41", dur])).unwrap();
            assert_eq!(line, "Week 2026-41 target: 168h 00m", "input {dur}");
            assert_eq!(get_week_target(&conn, &wk(2026, 41)).unwrap(), Some(10080));
        }
    }

    #[test]
    fn over_cap_is_rejected_after_the_parse_and_writes_nothing() {
        let today = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let cases = [
            (
                "168h1m",
                10081,
                "target cannot exceed 168h 00m (got 168h 01m)",
            ),
            (
                "10081m",
                10081,
                "target cannot exceed 168h 00m (got 168h 01m)",
            ),
            (
                "200h",
                12000,
                "target cannot exceed 168h 00m (got 200h 00m)",
            ),
            (
                "99999999999h",
                5_999_999_999_940,
                "target cannot exceed 168h 00m (got 99999999999h 00m)",
            ),
        ];
        for (dur, minutes, message) in cases {
            let conn = new_conn();
            let err = apply(&conn, today, &args(&["2026-41", dur])).unwrap_err();
            assert_eq!(
                err.downcast_ref::<TargetTooLarge>(),
                Some(&TargetTooLarge(minutes)),
                "input {dur}"
            );
            assert_eq!(err.to_string(), message);
            assert_eq!(row_count(&conn), 0);
        }
    }

    #[test]
    fn cap_runs_after_the_week_id_check() {
        let conn = new_conn();
        let today = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let err = apply(&conn, today, &args(&["2027-53", "200h"])).unwrap_err();
        assert!(err.downcast_ref::<crate::date::DateWeekError>().is_some());
        assert!(err.downcast_ref::<TargetTooLarge>().is_none());
    }

    #[test]
    fn i64_overflow_keeps_the_format_error() {
        let conn = new_conn();
        let today = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let err = apply(&conn, today, &args(&["2026-41", "9999999999999999999h"])).unwrap_err();
        assert!(matches!(
            err.downcast_ref::<crate::time::DurationParseError>(),
            Some(crate::time::DurationParseError::InvalidFormat(_))
        ));
        assert_eq!(row_count(&conn), 0);
    }

    // --- output writer (panic-fixes spec §5) ---------------------------

    #[test]
    fn run_writes_the_echo_line() {
        let conn = new_conn();
        let today = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let mut out = Vec::new();
        run(&conn, today, &args(&["2026-41", "20h"]), &mut out).unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "Week 2026-41 target: 20h 00m\n"
        );
    }

    #[test]
    fn run_keeps_the_target_when_output_fails() {
        let conn = new_conn();
        let today = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let mut full: &mut [u8] = &mut [];
        let err = run(&conn, today, &args(&["2026-41", "20h"]), &mut full).unwrap_err();
        assert!(err.downcast_ref::<crate::OutputError>().is_some());
        assert_eq!(get_week_target(&conn, &wk(2026, 41)).unwrap(), Some(1200));
    }
}
