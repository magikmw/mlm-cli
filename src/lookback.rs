//! Lookback scan of open starts on earlier dates (SPEC §7.1, §7.4).
//!
//! This module owns the scan, its finding types and the `mlm start`
//! stderr forms. Today's `status` page and `mlm start` both read it, so
//! the two can never disagree about what counts as a lookback date.

use std::collections::BTreeMap;

use chrono::{DateTime, Duration as ChronoDuration, NaiveDate, NaiveTime, Utc};
use rusqlite::Connection;

use crate::date::{format_date, format_date_with_weekday};
use crate::render;
use crate::stint;
use crate::storage::{self, Punch};

/// One window date's finding from the §7.1/§7.4 14-day lookback.
/// Deliberately narrower than `DateAnomalies`: a lookback line never
/// carries E8/refused-join detail (§1.2's residual-gap bullets), only
/// "this window date has an open start" in one of two shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LookbackFinding {
    /// The window date (age 2-14) this finding is about.
    pub date: NaiveDate,
    /// `today - date`. Always in 2..=14.
    pub age_days: i64,
    pub kind: LookbackKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookbackKind {
    /// Exactly one open start on the window date. `punch_id` is that
    /// start's row id, so `commands::start` can tell whether it's the
    /// punch it just wrote (§7.4).
    SingleOpenStart { time: NaiveTime, punch_id: i64 },
    /// Two or more open starts on the window date (that date's own E7).
    MultipleOpenStints { count: usize },
}

impl LookbackFinding {
    /// `status`'s block form (§7.1, one finding per line-group,
    /// appended after a date's own `DateAnomalies::detail_lines()`,
    /// oldest window date first).
    pub fn detail_lines(&self) -> Vec<String> {
        debug_assert!(
            (2..=14).contains(&self.age_days),
            "lookback age must be 2..=14"
        );
        let d = format_date(self.date);
        let wdy = format_date_with_weekday(self.date);
        match self.kind {
            LookbackKind::SingleOpenStart { time, .. } => {
                let time = time.format("%H:%M");
                vec![
                    format!(
                        "[!] unclosed start on {wdy} at {time} ({} days ago)",
                        self.age_days
                    ),
                    format!("    fix: mlm stop HH:MM --date {d}  (a time after {time})"),
                    format!(
                        "     or: mlm delete punch --date {d}  (lists punches; rerun with the start's number)"
                    ),
                ]
            }
            LookbackKind::MultipleOpenStints { count } => vec![
                format!(
                    "[!] {count} open stints on {wdy} ({} days ago)",
                    self.age_days
                ),
                format!("    fix: see mlm status {d}"),
            ],
        }
    }

    /// `start`'s stderr form (§7.4). A single open start prints three
    /// lines laid out like `detail_lines()`, `warning: ` in place of
    /// `[!] `; two or more open starts stay one line. No trailing
    /// newline either way.
    pub fn stderr_line(&self) -> String {
        debug_assert!(
            (2..=14).contains(&self.age_days),
            "lookback age must be 2..=14"
        );
        let d = format_date(self.date);
        let wdy = format_date_with_weekday(self.date);
        match self.kind {
            LookbackKind::SingleOpenStart { time, .. } => {
                let time = time.format("%H:%M");
                format!(
                    "warning: unclosed start on {wdy} at {time} ({} days ago)\n    fix: mlm stop HH:MM --date {d}  (a time after {time})\n     or: mlm delete punch --date {d}  (lists punches; rerun with the start's number)",
                    self.age_days
                )
            }
            LookbackKind::MultipleOpenStints { count } => format!(
                "warning: {count} open stints on {wdy} ({} days ago); see mlm status {d}",
                self.age_days
            ),
        }
    }
}

/// Groups `punches` by their local `date`, for the lookback scan's
/// per-date/per-neighbor lookups.
fn group_punches_by_date(punches: &[Punch]) -> BTreeMap<NaiveDate, Vec<Punch>> {
    let mut map: BTreeMap<NaiveDate, Vec<Punch>> = BTreeMap::new();
    for p in punches {
        map.entry(p.date).or_default().push(*p);
    }
    map
}

/// One window date's lookback finding, or `None` when that date has no
/// open start after splice resolution (idle, or an orphaned end only --
/// §1.2's known residual gap).
fn lookback_finding_for_date(
    date: NaiveDate,
    age: i64,
    by_date: &BTreeMap<NaiveDate, Vec<Punch>>,
    now_utc: DateTime<Utc>,
) -> Option<LookbackFinding> {
    let empty = Vec::new();
    let prev = by_date.get(&date.pred_opt().unwrap()).unwrap_or(&empty);
    let cur = by_date.get(&date).unwrap_or(&empty);
    let next = by_date.get(&date.succ_opt().unwrap()).unwrap_or(&empty);
    if cur.is_empty() {
        return None;
    }
    let day = stint::classify_at(prev, cur, next, now_utc);
    let kind = match day.open.len() {
        0 => return None,
        1 => LookbackKind::SingleOpenStart {
            time: render::local_time(day.open[0].start.at_utc),
            punch_id: day.open[0].start.id,
        },
        n => LookbackKind::MultipleOpenStints { count: n },
    };
    Some(LookbackFinding {
        date,
        age_days: age,
        kind,
    })
}

/// Walks ages 2 through 14 relative to `today` (§2's lookback window),
/// oldest date first, and returns one `LookbackFinding` per window date
/// that has one or more open starts after splice resolution. A window
/// date with zero open starts (idle, or an orphaned end only) produces no
/// finding -- §1.2's orphan gap is deliberate, not a bug.
///
/// Pure aside from a single `punches_in_range` read spanning
/// `[today - 15d, today - 1d]` (15 dates: the 13 window dates plus the
/// one-day neighbor each end of the window needs for its own
/// `classify_at`). `now_utc` is the caller's already-resolved clock
/// instant (contract 6: never read from a system clock in here).
///
/// Called by `commands::start` after every successful write (§7.4) and,
/// through [`today_page_lines`], by today's `status` (§7.1).
pub fn lookback(
    conn: &Connection,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
) -> anyhow::Result<Vec<LookbackFinding>> {
    let window_start = today - ChronoDuration::days(15); // (today - 14) - 1
    let window_end = today - ChronoDuration::days(1); // (today - 2) + 1
    let punches = storage::punches_in_range(conn, window_start, window_end)?;
    let by_date = group_punches_by_date(&punches);

    let mut findings = Vec::new();
    for age in (2..=14).rev() {
        // .rev() walks age 14 down to 2, i.e. the OLDEST window date
        // (smallest calendar date) first -- oldest-first per §7.1.
        let date = today - ChronoDuration::days(age);
        if let Some(finding) = lookback_finding_for_date(date, age, &by_date, now_utc) {
            findings.push(finding);
        }
    }
    Ok(findings)
}

/// Today's page entry point: finished lookback lines, oldest date first.
/// `_refused_join_names_yesterday` is true when today's own anomaly block
/// already holds a refused-join line naming yesterday. It is accepted now
/// so callers keep one signature once the lines depend on it.
pub fn today_page_lines(
    conn: &Connection,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
    _refused_join_names_yesterday: bool,
) -> anyhow::Result<Vec<String>> {
    Ok(lookback(conn, today, now_utc)?
        .iter()
        .flat_map(LookbackFinding::detail_lines)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::storage::PunchKind;
    use chrono::{Local, TimeZone};

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("test fixture is a real date")
    }

    fn t(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).expect("test fixture is a real time")
    }

    fn test_db() -> Connection {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");
        db::apply_migrations(&mut conn).expect("apply migrations");
        conn
    }

    fn today() -> NaiveDate {
        d(2026, 2, 12)
    }

    fn now() -> DateTime<Utc> {
        Local
            .from_local_datetime(&today().and_hms_opt(18, 0, 0).unwrap())
            .unwrap()
            .with_timezone(&Utc)
    }

    fn insert_start(conn: &Connection, days_ago: i64, h: u32, m: u32) {
        storage::insert_punch(
            conn,
            PunchKind::Start,
            today() - ChronoDuration::days(days_ago),
            t(h, m),
            &Local,
        )
        .expect("insert");
    }

    // ---- finding rendering (moved from render.rs, text unchanged) ----

    #[test]
    fn t_lookback_single_block_matches_spec_example() {
        let f = LookbackFinding {
            date: d(2026, 9, 21),
            age_days: 5,
            kind: LookbackKind::SingleOpenStart {
                time: t(9, 0),
                punch_id: 1,
            },
        };
        assert_eq!(
            f.detail_lines(),
            vec![
                "[!] unclosed start on Mon 2026-09-21 at 09:00 (5 days ago)".to_string(),
                "    fix: mlm stop HH:MM --date 2026-09-21  (a time after 09:00)".to_string(),
                "     or: mlm delete punch --date 2026-09-21  (lists punches; rerun with the start's number)".to_string(),
            ]
        );
    }

    #[test]
    fn t_lookback_multi_block_matches_spec_example() {
        let f = LookbackFinding {
            date: d(2026, 9, 21),
            age_days: 5,
            kind: LookbackKind::MultipleOpenStints { count: 2 },
        };
        assert_eq!(
            f.detail_lines(),
            vec![
                "[!] 2 open stints on Mon 2026-09-21 (5 days ago)".to_string(),
                "    fix: see mlm status 2026-09-21".to_string(),
            ]
        );
    }

    #[test]
    fn t_lookback_ordering_oldest_first() {
        // This only proves rendering preserves caller-supplied order;
        // the actual "oldest first" scan guarantee is the lookback
        // scan's own job.
        let findings = [
            LookbackFinding {
                date: d(2026, 9, 20),
                age_days: 6,
                kind: LookbackKind::SingleOpenStart {
                    time: t(8, 0),
                    punch_id: 1,
                },
            },
            LookbackFinding {
                date: d(2026, 9, 21),
                age_days: 5,
                kind: LookbackKind::MultipleOpenStints { count: 2 },
            },
        ];
        let combined: Vec<String> = findings.iter().flat_map(|f| f.detail_lines()).collect();
        assert!(combined[0].contains("2026-09-20"));
        assert!(combined[3].contains("2026-09-21"));
    }

    #[test]
    fn t_lookback_stderr_single_is_three_lines_like_status_block() {
        let f = LookbackFinding {
            date: d(2026, 9, 21),
            age_days: 5,
            kind: LookbackKind::SingleOpenStart {
                time: t(9, 0),
                punch_id: 1,
            },
        };
        assert_eq!(
            f.stderr_line(),
            "warning: unclosed start on Mon 2026-09-21 at 09:00 (5 days ago)\n    fix: mlm stop HH:MM --date 2026-09-21  (a time after 09:00)\n     or: mlm delete punch --date 2026-09-21  (lists punches; rerun with the start's number)"
        );
    }

    #[test]
    fn t_lookback_stderr_multi_matches_spec_example() {
        let f = LookbackFinding {
            date: d(2026, 9, 21),
            age_days: 5,
            kind: LookbackKind::MultipleOpenStints { count: 2 },
        };
        assert_eq!(
            f.stderr_line(),
            "warning: 2 open stints on Mon 2026-09-21 (5 days ago); see mlm status 2026-09-21"
        );
    }

    #[test]
    fn t_lookback_stderr_single_matches_block_fix_or_wording_exactly() {
        // §7.4: the single-open-start warning is laid out
        // identically to status's own lookback block, "warning: "
        // swapped in for the leading "[!] " -- same fix/or wording,
        // same double-space before the parenthetical.
        let f = LookbackFinding {
            date: d(2026, 9, 21),
            age_days: 5,
            kind: LookbackKind::SingleOpenStart {
                time: t(9, 0),
                punch_id: 1,
            },
        };
        let block = f.detail_lines();
        let stderr = f.stderr_line();
        let stderr_lines: Vec<&str> = stderr.split('\n').collect();
        assert_eq!(stderr_lines.len(), 3);
        assert_eq!(stderr_lines[0], format!("warning: {}", &block[0][4..]));
        assert_eq!(stderr_lines[1], block[1]);
        assert_eq!(stderr_lines[2], block[2]);
    }

    // ---- the scan (moved from status.rs) ----

    #[test]
    fn lookback_returns_findings_oldest_first() {
        let conn = test_db();
        insert_start(&conn, 10, 9, 0);
        insert_start(&conn, 4, 9, 0);

        let findings = lookback(&conn, today(), now()).expect("lookback");
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].date, today() - ChronoDuration::days(10));
        assert_eq!(findings[1].date, today() - ChronoDuration::days(4));
        assert_eq!(findings[0].age_days, 10);
        assert_eq!(findings[1].age_days, 4);
    }

    // F5: the lookback's lower age bound was only ever caught by a
    // debug_assert!, never a real assertion. Age 1 (yesterday) must be
    // excluded even though it has an open start.
    #[test]
    fn lookback_excludes_age_one_open_start() {
        let conn = test_db();
        insert_start(&conn, 1, 9, 0);
        insert_start(&conn, 4, 9, 0);

        let findings = lookback(&conn, today(), now()).expect("lookback");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].date, today() - ChronoDuration::days(4));
        assert!(!findings.iter().any(|f| f.age_days == 1), "{:?}", findings);
    }

    // ---- today's page entry point ----

    #[test]
    fn today_page_lines_equals_flattened_findings_for_either_flag() {
        let conn = test_db();
        insert_start(&conn, 10, 9, 0);
        insert_start(&conn, 4, 9, 0);
        insert_start(&conn, 4, 11, 0);

        let expected: Vec<String> = lookback(&conn, today(), now())
            .expect("lookback")
            .iter()
            .flat_map(LookbackFinding::detail_lines)
            .collect();
        assert!(!expected.is_empty());
        assert!(expected[0].contains("(10 days ago)"));
        for flag in [true, false] {
            let lines = today_page_lines(&conn, today(), now(), flag).expect("lines");
            assert_eq!(lines, expected, "flag {flag}");
        }
    }

    #[test]
    fn today_page_lines_is_empty_without_findings() {
        let conn = test_db();
        let lines = today_page_lines(&conn, today(), now(), false).expect("lines");
        assert!(lines.is_empty());
    }
}
