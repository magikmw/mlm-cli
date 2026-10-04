//! Lookback scan of open starts on earlier dates (SPEC §7.1, §7.4).
//!
//! This module owns the scan, its finding types and the `mlm start`
//! stderr forms. Today's `status` page and `mlm start` both read it, so
//! the two can never disagree about what counts as a lookback date.

use chrono::{DateTime, Duration as ChronoDuration, NaiveDate, NaiveTime, Utc};
use rusqlite::Connection;

use crate::anomalies;
use crate::date::{format_date, format_date_with_weekday};
use crate::history::PunchBuckets;
use crate::render;
use crate::stint;
use crate::storage;

const MIN_AGE_DAYS: i64 = 1;
const MAX_AGE_DAYS: i64 = 14;

/// One window date's finding from the §7.1/§7.4 14-day lookback.
/// Deliberately narrower than `DateAnomalies`: a lookback line never
/// carries E8/refused-join detail (§1.2's residual-gap bullets), only
/// "this window date has an open start" in one of two shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LookbackFinding {
    /// The window date (age 1-14) this finding is about.
    pub date: NaiveDate,
    /// `today - date`. Always in 1..=14. Age 1 renders as the yesterday
    /// hint or the `(yesterday)` multi-open line.
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

/// Facts a lookback line needs beyond the finding itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LineFacts {
    /// First punch on the finding's date after its single open start (the
    /// clean-pair `N`).
    pub next: Option<NaiveTime>,
    /// True when today has at least one punch, which drops the hint's `or:`
    /// line.
    pub today_has_punches: bool,
}

/// One scan result: the finding plus its line facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LookbackEntry {
    pub finding: LookbackFinding,
    pub facts: LineFacts,
}

impl LookbackEntry {
    pub fn detail_lines(&self) -> Vec<String> {
        self.finding.detail_lines_with(self.facts)
    }

    pub fn stderr_line(&self) -> String {
        self.finding.stderr_line_with(self.facts)
    }
}

impl LookbackFinding {
    /// Block form with default facts (no later punch, today empty).
    pub fn detail_lines(&self) -> Vec<String> {
        self.detail_lines_with(LineFacts::default())
    }

    /// Stderr form with default facts.
    pub fn stderr_line(&self) -> String {
        self.stderr_line_with(LineFacts::default())
    }

    /// `status`'s block form (§7.1, §3.1). One line-group per finding,
    /// oldest window date first. A lone age-1 open start renders as the
    /// `[i]` hint.
    pub fn detail_lines_with(&self, facts: LineFacts) -> Vec<String> {
        debug_assert!((MIN_AGE_DAYS..=MAX_AGE_DAYS).contains(&self.age_days));
        let d = format_date(self.date);
        let wdy = format_date_with_weekday(self.date);
        match self.kind {
            LookbackKind::SingleOpenStart { time, .. } if self.is_yesterday_hint() => {
                yesterday_hint_lines(&wdy, &d, time, facts)
            }
            LookbackKind::SingleOpenStart { time, .. } => {
                unclosed_start_lines(&wdy, &d, time, self.age_days, facts)
            }
            LookbackKind::MultipleOpenStints { count } => vec![
                format!(
                    "[!] {count} open stints on {wdy} {}",
                    age_phrase(self.age_days)
                ),
                format!("    fix: see mlm status {d}"),
            ],
        }
    }

    /// `start`'s stderr form (§7.4, §3.1). A lone older open start prints
    /// three lines laid out like the block form, `warning: ` in place of
    /// `[!] `. A lone age-1 start prints two lines and no `or:`. Two or
    /// more open starts stay one line. No trailing newline.
    pub fn stderr_line_with(&self, facts: LineFacts) -> String {
        debug_assert!((MIN_AGE_DAYS..=MAX_AGE_DAYS).contains(&self.age_days));
        let d = format_date(self.date);
        let wdy = format_date_with_weekday(self.date);
        match self.kind {
            LookbackKind::SingleOpenStart { time, .. } if self.is_yesterday_hint() => format!(
                "warning: start on {wdy} at {} still open (yesterday)\n    fix: mlm stop HH:MM --date {d}  {}",
                time.format("%H:%M"),
                render::stop_time_hint(time, facts.next)
            ),
            LookbackKind::SingleOpenStart { time, .. } => format!(
                "warning: unclosed start on {wdy} at {} ({} days ago)\n    fix: mlm stop HH:MM --date {d}  {}\n     or: mlm delete punch --date {d}  (lists punches; rerun with the start's number)",
                time.format("%H:%M"),
                self.age_days,
                render::stop_time_hint(time, facts.next)
            ),
            LookbackKind::MultipleOpenStints { count } => format!(
                "warning: {count} open stints on {wdy} {}; see mlm status {d}",
                age_phrase(self.age_days)
            ),
        }
    }

    /// True for the age-1 single open start, which renders as the `[i]`
    /// hint instead of an anomaly line.
    pub fn is_yesterday_hint(&self) -> bool {
        self.age_days == 1 && matches!(self.kind, LookbackKind::SingleOpenStart { .. })
    }
}

/// `(yesterday)` at age 1, `(N days ago)` otherwise.
fn age_phrase(age_days: i64) -> String {
    if age_days == 1 {
        "(yesterday)".to_string()
    } else {
        format!("({age_days} days ago)")
    }
}

fn unclosed_start_lines(
    wdy: &str,
    d: &str,
    time: NaiveTime,
    age: i64,
    facts: LineFacts,
) -> Vec<String> {
    vec![
        format!(
            "[!] unclosed start on {wdy} at {} ({age} days ago)",
            time.format("%H:%M")
        ),
        format!(
            "    fix: mlm stop HH:MM --date {d}  {}",
            render::stop_time_hint(time, facts.next)
        ),
        format!(
            "     or: mlm delete punch --date {d}  (lists punches; rerun with the start's number)"
        ),
    ]
}

fn yesterday_hint_lines(wdy: &str, d: &str, time: NaiveTime, facts: LineFacts) -> Vec<String> {
    let mut lines = vec![
        format!(
            "[i] start on {wdy} at {} still open (yesterday; not counted until stopped)",
            time.format("%H:%M")
        ),
        format!(
            "    fix: mlm stop HH:MM --date {d}  {}",
            render::stop_time_hint(time, facts.next)
        ),
    ];
    if facts.next.is_none() && !facts.today_has_punches {
        lines.push("     or: mlm stop  (still working: ends it now, across midnight)".to_string());
    }
    lines
}

/// The overnight stint: yesterday's one open start, not joined to today by
/// the splice, with no refused join naming yesterday on today's page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OvernightStint {
    /// Yesterday.
    pub date: NaiveDate,
    /// Local time of the open start.
    pub start: NaiveTime,
    /// Whole minutes from the start to `now_utc`.
    pub minutes_so_far: i64,
}

/// `buckets` must hold yesterday and today (the `status` and `week` loaders
/// both do).
pub fn overnight_stint(
    buckets: &PunchBuckets,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
) -> Option<OvernightStint> {
    let date = today.pred_opt()?;
    let trio = buckets.trio(date);
    if trio.day.is_empty() {
        return None;
    }
    let ctx = anomalies::classify_context(&trio, now_utc);
    let [only] = ctx.day.open.as_slice() else {
        return None;
    };
    if stint::is_refused_join(&ctx.day_raw, &ctx.next_raw, trio.next) {
        return None;
    }
    Some(OvernightStint {
        date,
        start: render::local_time(only.start.at_utc),
        minutes_so_far: only.minutes_so_far,
    })
}

fn load_buckets(conn: &Connection, today: NaiveDate) -> anyhow::Result<PunchBuckets> {
    let from = today - ChronoDuration::days(MAX_AGE_DAYS + 1);
    let punches = storage::punches_in_range(conn, from, today)?;
    Ok(PunchBuckets::from_punches(&punches))
}

fn scan(buckets: &PunchBuckets, today: NaiveDate, now_utc: DateTime<Utc>) -> Vec<LookbackEntry> {
    let today_has_punches = !buckets.day(today).is_empty();
    (MIN_AGE_DAYS..=MAX_AGE_DAYS)
        .rev()
        .filter_map(|age| {
            let date = today - ChronoDuration::days(age);
            entry_for_date(date, age, buckets, today_has_punches, now_utc)
        })
        .collect()
}

/// One entry per date with open starts, ages 1-14, oldest first. One ranged
/// read over `[today - 15, today]` covers the 14 window dates, the previous
/// day of age 14, and today (the next day of age 1 and the source of
/// `today_has_punches`). `now_utc` is the caller's resolved clock instant.
///
/// Called by `commands::start` after every successful write (§7.4) and,
/// through [`today_page_lines`], by today's `status` (§7.1, §3.1).
pub fn lookback(
    conn: &Connection,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
) -> anyhow::Result<Vec<LookbackEntry>> {
    Ok(scan(&load_buckets(conn, today)?, today, now_utc))
}

/// The entry for one window date, or `None` when it has no open start after
/// splice resolution (idle, or an orphaned end only, §1.2's residual gap).
fn entry_for_date(
    date: NaiveDate,
    age: i64,
    buckets: &PunchBuckets,
    today_has_punches: bool,
    now_utc: DateTime<Utc>,
) -> Option<LookbackEntry> {
    let trio = buckets.trio(date);
    if trio.day.is_empty() {
        return None;
    }
    let day = stint::classify_at(trio.prev, trio.day, trio.next, now_utc);
    let (kind, next) = match day.open.as_slice() {
        [] => return None,
        [only] => (
            LookbackKind::SingleOpenStart {
                time: render::local_time(only.start.at_utc),
                punch_id: only.start.id,
            },
            render::next_punch_after(trio.day, &only.start),
        ),
        many => (LookbackKind::MultipleOpenStints { count: many.len() }, None),
    };
    Some(LookbackEntry {
        finding: LookbackFinding {
            date,
            age_days: age,
            kind,
        },
        facts: LineFacts {
            next,
            today_has_punches,
        },
    })
}

/// Finished lookback lines for today's page, oldest first, with the age-1
/// hint last. The hint prints exactly when `overnight_stint` finds a stint.
pub fn today_page_lines(
    conn: &Connection,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
) -> anyhow::Result<Vec<String>> {
    let buckets = load_buckets(conn, today)?;
    let overnight = overnight_stint(&buckets, today, now_utc).is_some();
    Ok(scan(&buckets, today, now_utc)
        .iter()
        .filter(|e| overnight || !e.finding.is_yesterday_hint())
        .flat_map(LookbackEntry::detail_lines)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::storage::{Punch, PunchKind};
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

    // ---- age-aware rendering ----

    fn single(age: i64) -> LookbackFinding {
        let date = today() - ChronoDuration::days(age);
        LookbackFinding {
            date,
            age_days: age,
            kind: LookbackKind::SingleOpenStart {
                time: t(18, 2),
                punch_id: 1,
            },
        }
    }

    fn multi(age: i64) -> LookbackFinding {
        LookbackFinding {
            date: today() - ChronoDuration::days(age),
            age_days: age,
            kind: LookbackKind::MultipleOpenStints { count: 2 },
        }
    }

    fn facts(next: Option<NaiveTime>, today_has_punches: bool) -> LineFacts {
        LineFacts {
            next,
            today_has_punches,
        }
    }

    #[test]
    fn yesterday_hint_lines_with_or_line_when_today_is_empty() {
        assert_eq!(
            single(1).detail_lines_with(facts(None, false)),
            vec![
                "[i] start on Wed 2026-02-11 at 18:02 still open (yesterday; not counted until stopped)",
                "    fix: mlm stop HH:MM --date 2026-02-11  (a time after 18:02)",
                "     or: mlm stop  (still working: ends it now, across midnight)",
            ]
        );
    }

    #[test]
    fn yesterday_hint_lines_drop_or_line_when_today_has_punches() {
        let lines = single(1).detail_lines_with(facts(Some(t(19, 0)), true));
        assert_eq!(
            lines,
            vec![
                "[i] start on Wed 2026-02-11 at 18:02 still open (yesterday; not counted until stopped)",
                "    fix: mlm stop HH:MM --date 2026-02-11  (a time between 18:02 and 19:00)",
            ]
        );
    }

    #[test]
    fn yesterday_hint_lines_drop_or_line_when_a_later_punch_follows_the_start() {
        let lines = single(1).detail_lines_with(facts(Some(t(19, 0)), false));
        assert_eq!(
            lines,
            vec![
                "[i] start on Wed 2026-02-11 at 18:02 still open (yesterday; not counted until stopped)",
                "    fix: mlm stop HH:MM --date 2026-02-11  (a time between 18:02 and 19:00)",
            ]
        );
    }

    #[test]
    fn page_drops_or_line_when_yesterday_has_a_later_closed_stint() {
        let conn = test_db();
        insert_start(&conn, 1, 9, 0);
        insert_start(&conn, 1, 14, 0);
        ins(&conn, PunchKind::End, 1, 15, 0);
        let lines = today_page_lines(&conn, today(), now()).expect("lines");
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(lines[1].contains("a time between 09:00 and 14:00"));
    }

    #[test]
    fn older_single_start_lines_carry_the_window_phrase() {
        let lines = single(5).detail_lines_with(facts(Some(t(19, 0)), true));
        assert_eq!(
            lines[1],
            "    fix: mlm stop HH:MM --date 2026-02-07  (a time between 18:02 and 19:00)"
        );
        assert!(lines[0].ends_with("(5 days ago)"));
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn yesterday_multi_lines_say_yesterday() {
        assert_eq!(
            multi(1).detail_lines(),
            vec![
                "[!] 2 open stints on Wed 2026-02-11 (yesterday)",
                "    fix: see mlm status 2026-02-11",
            ]
        );
    }

    #[test]
    fn yesterday_stderr_forms() {
        assert_eq!(
            single(1).stderr_line_with(facts(None, false)),
            "warning: start on Wed 2026-02-11 at 18:02 still open (yesterday)\n    fix: mlm stop HH:MM --date 2026-02-11  (a time after 18:02)"
        );
        assert_eq!(
            multi(1).stderr_line(),
            "warning: 2 open stints on Wed 2026-02-11 (yesterday); see mlm status 2026-02-11"
        );
    }

    #[test]
    fn older_stderr_single_carries_the_window_phrase() {
        let line = single(5).stderr_line_with(facts(Some(t(19, 0)), false));
        assert!(
            line.contains("--date 2026-02-07  (a time between 18:02 and 19:00)\n     or: "),
            "{line}"
        );
    }

    #[test]
    fn is_yesterday_hint_only_for_single_age_one() {
        assert!(single(1).is_yesterday_hint());
        assert!(!single(2).is_yesterday_hint());
        assert!(!multi(1).is_yesterday_hint());
    }

    // ---- the scan ----

    fn ins(conn: &Connection, kind: PunchKind, days_ago: i64, h: u32, m: u32) {
        storage::insert_punch(
            conn,
            kind,
            today() - ChronoDuration::days(days_ago),
            t(h, m),
            &Local,
        )
        .expect("insert");
    }

    #[test]
    fn lookback_returns_entries_oldest_first() {
        let conn = test_db();
        insert_start(&conn, 10, 9, 0);
        insert_start(&conn, 4, 9, 0);

        let entries = lookback(&conn, today(), now()).expect("lookback");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].finding.date, today() - ChronoDuration::days(10));
        assert_eq!(entries[1].finding.date, today() - ChronoDuration::days(4));
        assert_eq!(entries[0].finding.age_days, 10);
        assert_eq!(entries[1].finding.age_days, 4);
    }

    // Yesterday is inside the window (SPEC §3.1).
    #[test]
    fn lookback_includes_age_one_single_open_start() {
        let conn = test_db();
        insert_start(&conn, 1, 9, 0);
        insert_start(&conn, 4, 9, 0);

        let entries = lookback(&conn, today(), now()).expect("lookback");
        let ages: Vec<i64> = entries.iter().map(|e| e.finding.age_days).collect();
        assert_eq!(ages, vec![4, 1]);
    }

    #[test]
    fn lookback_excludes_age_fifteen_and_keeps_fourteen() {
        let conn = test_db();
        insert_start(&conn, 15, 9, 0);
        insert_start(&conn, 14, 9, 0);
        insert_start(&conn, 2, 9, 0);

        let entries = lookback(&conn, today(), now()).expect("lookback");
        let ages: Vec<i64> = entries.iter().map(|e| e.finding.age_days).collect();
        assert_eq!(ages, vec![14, 2]);
    }

    #[test]
    fn lookback_age_one_joined_by_todays_first_stop_is_absent() {
        let conn = test_db();
        insert_start(&conn, 1, 23, 0);
        ins(&conn, PunchKind::End, 0, 0, 10);

        assert!(
            lookback(&conn, today(), now())
                .expect("lookback")
                .is_empty()
        );
    }

    #[test]
    fn lookback_age_one_not_joined_when_today_has_two_orphan_stops() {
        let conn = test_db();
        insert_start(&conn, 1, 18, 2);
        ins(&conn, PunchKind::End, 0, 0, 10);
        ins(&conn, PunchKind::End, 0, 0, 20);

        let entries = lookback(&conn, today(), now()).expect("lookback");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].finding.age_days, 1);
        assert!(entries[0].facts.today_has_punches);
    }

    #[test]
    fn lookback_entry_facts_carry_next_punch_and_today_flag() {
        let conn = test_db();
        insert_start(&conn, 5, 9, 0);
        insert_start(&conn, 5, 10, 0);
        ins(&conn, PunchKind::End, 5, 10, 30);

        let entries = lookback(&conn, today(), now()).expect("lookback");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].facts.next, Some(t(10, 0)));
        assert!(!entries[0].facts.today_has_punches);
    }

    #[test]
    fn lookback_entry_facts_have_no_next_without_a_later_punch() {
        let conn = test_db();
        insert_start(&conn, 5, 9, 0);
        insert_start(&conn, 3, 9, 0);
        insert_start(&conn, 3, 10, 0);
        insert_start(&conn, 0, 8, 0);

        let entries = lookback(&conn, today(), now()).expect("lookback");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].facts, facts(None, true));
        assert!(matches!(
            entries[1].finding.kind,
            LookbackKind::MultipleOpenStints { count: 2 }
        ));
        assert_eq!(entries[1].facts.next, None);
    }

    // ---- today's page entry point ----

    #[test]
    fn today_page_lines_equals_flattened_entries() {
        let conn = test_db();
        insert_start(&conn, 10, 9, 0);
        insert_start(&conn, 4, 9, 0);
        insert_start(&conn, 4, 11, 0);

        let expected: Vec<String> = lookback(&conn, today(), now())
            .expect("lookback")
            .iter()
            .flat_map(LookbackEntry::detail_lines)
            .collect();
        assert!(!expected.is_empty());
        assert!(expected[0].contains("(10 days ago)"));
        let lines = today_page_lines(&conn, today(), now()).expect("lines");
        assert_eq!(lines, expected);
    }

    #[test]
    fn today_page_lines_is_empty_without_findings() {
        let conn = test_db();
        let lines = today_page_lines(&conn, today(), now()).expect("lines");
        assert!(lines.is_empty());
    }

    const HINT: [&str; 3] = [
        "[i] start on Wed 2026-02-11 at 18:02 still open (yesterday; not counted until stopped)",
        "    fix: mlm stop HH:MM --date 2026-02-11  (a time after 18:02)",
        "     or: mlm stop  (still working: ends it now, across midnight)",
    ];

    #[test]
    fn page_shows_three_line_hint_when_today_is_empty() {
        let conn = test_db();
        insert_start(&conn, 1, 18, 2);
        let lines = today_page_lines(&conn, today(), now()).expect("lines");
        assert_eq!(lines, HINT);
    }

    #[test]
    fn page_drops_or_line_when_today_has_a_start_or_a_closed_pair() {
        let conn = test_db();
        insert_start(&conn, 1, 18, 2);
        insert_start(&conn, 0, 9, 0);
        let lines = today_page_lines(&conn, today(), now()).expect("lines");
        assert_eq!(lines, HINT[..2]);

        ins(&conn, PunchKind::End, 0, 10, 0);
        let lines = today_page_lines(&conn, today(), now()).expect("lines");
        assert_eq!(lines, HINT[..2]);
    }

    #[test]
    fn page_has_no_hint_when_todays_first_stop_joins() {
        let conn = test_db();
        insert_start(&conn, 1, 18, 2);
        ins(&conn, PunchKind::End, 0, 0, 30);
        assert!(
            today_page_lines(&conn, today(), now())
                .expect("lines")
                .is_empty()
        );
    }

    #[test]
    fn page_has_no_hint_after_following_the_hints_own_fix() {
        let conn = test_db();
        insert_start(&conn, 1, 18, 2);
        ins(&conn, PunchKind::End, 1, 19, 0);
        assert!(
            today_page_lines(&conn, today(), now())
                .expect("lines")
                .is_empty()
        );
    }

    #[test]
    fn refused_join_drops_only_the_hint() {
        let conn = test_db();
        insert_start(&conn, 1, 18, 2);
        ins(&conn, PunchKind::End, 0, 0, 10);
        ins(&conn, PunchKind::End, 0, 0, 20);

        assert!(
            today_page_lines(&conn, today(), now())
                .expect("lines")
                .is_empty()
        );
        // `mlm start` reads the unfiltered scan and keeps its warning.
        let entries = lookback(&conn, today(), now()).expect("lookback");
        assert_eq!(entries.len(), 1);
        assert!(entries[0].finding.is_yesterday_hint());
    }

    #[test]
    fn yesterday_multi_line_prints_without_a_hint() {
        let conn = test_db();
        insert_start(&conn, 1, 9, 0);
        insert_start(&conn, 1, 10, 0);
        let lines = today_page_lines(&conn, today(), now()).expect("lines");
        assert_eq!(
            lines,
            vec![
                "[!] 2 open stints on Wed 2026-02-11 (yesterday)",
                "    fix: see mlm status 2026-02-11",
            ]
        );
    }

    #[test]
    fn hint_is_last_after_older_findings() {
        let conn = test_db();
        insert_start(&conn, 14, 9, 0);
        insert_start(&conn, 5, 9, 0);
        insert_start(&conn, 1, 18, 2);
        let lines = today_page_lines(&conn, today(), now()).expect("lines");
        assert_eq!(lines.len(), 9);
        assert!(lines[0].contains("(14 days ago)"));
        assert!(lines[3].contains("(5 days ago)"));
        assert_eq!(lines[6..], HINT);
    }

    #[test]
    fn splice_through_today_uses_classify_at() {
        let conn = test_db();
        insert_start(&conn, 1, 23, 0);
        ins(&conn, PunchKind::End, 0, 0, 10);
        assert!(lookback(&conn, today(), now()).expect("scan").is_empty());
    }

    // ---- clean-pair window on every surface ----

    fn window_fixture(conn: &Connection, age: i64, second_start: (u32, u32)) {
        ins(conn, PunchKind::Start, age, 9, 0);
        ins(conn, PunchKind::Start, age, second_start.0, second_start.1);
        ins(conn, PunchKind::End, age, 10, 30);
    }

    #[test]
    fn every_surface_names_the_window_when_a_later_punch_exists() {
        for age in [5, 1] {
            let conn = test_db();
            window_fixture(&conn, age, (10, 0));
            let entries = lookback(&conn, today(), now()).expect("scan");
            assert_eq!(entries.len(), 1, "age {age}");
            let phrase = "(a time between 09:00 and 10:00)";
            assert!(
                entries[0].detail_lines().iter().any(|l| l.contains(phrase)),
                "age {age} detail"
            );
            assert!(
                entries[0].stderr_line().contains(phrase),
                "age {age} stderr"
            );
        }
    }

    #[test]
    fn every_surface_keeps_the_open_phrase_without_a_later_punch() {
        for age in [5, 1] {
            let conn = test_db();
            ins(&conn, PunchKind::Start, age, 9, 0);
            let entries = lookback(&conn, today(), now()).expect("scan");
            let phrase = "(a time after 09:00)";
            assert!(entries[0].detail_lines().iter().any(|l| l.contains(phrase)));
            assert!(entries[0].stderr_line().contains(phrase));
        }
    }

    #[test]
    fn window_is_one_minute_wide_when_next_is_adjacent() {
        let conn = test_db();
        ins(&conn, PunchKind::Start, 5, 9, 0);
        ins(&conn, PunchKind::Start, 5, 9, 1);
        ins(&conn, PunchKind::End, 5, 9, 30);
        let entries = lookback(&conn, today(), now()).expect("scan");
        assert!(
            entries[0]
                .stderr_line()
                .contains("(a time between 09:00 and 09:01)")
        );
    }

    fn day_punches(conn: &Connection, age: i64) -> Vec<Punch> {
        let date = today() - ChronoDuration::days(age);
        storage::punches_in_range(conn, date, date).expect("read")
    }

    #[test]
    fn stop_inside_the_window_pairs_cleanly_for_every_edge() {
        // The overlap assertion is trivially true until the overlap anomaly
        // lands in `anomalies`; the stint assertions carry the check now.
        for age in [5, 1] {
            for stop in [(9, 30), (10, 0), (9, 1)] {
                let conn = test_db();
                window_fixture(&conn, age, (10, 0));
                ins(&conn, PunchKind::End, age, stop.0, stop.1);

                let punches = day_punches(&conn, age);
                let day = stint::classify(&punches, now());
                assert!(day.open.is_empty(), "age {age} stop {stop:?}");
                let mut pairs: Vec<(NaiveTime, NaiveTime)> = day
                    .completed
                    .iter()
                    .map(|s| {
                        (
                            render::local_time(s.start.at_utc),
                            render::local_time(s.end.at_utc),
                        )
                    })
                    .collect();
                pairs.sort();
                assert_eq!(
                    pairs,
                    vec![(t(9, 0), t(stop.0, stop.1)), (t(10, 0), t(10, 30))],
                    "age {age} stop {stop:?}"
                );

                let buckets = PunchBuckets::from_punches(&punches);
                let date = today() - ChronoDuration::days(age);
                let facts = crate::anomalies::date_facts(&buckets.trio(date), date, today(), now());
                assert!(!facts.has_any(), "age {age} stop {stop:?}");
            }
        }
    }

    #[test]
    fn stop_past_the_window_nests() {
        let conn = test_db();
        window_fixture(&conn, 5, (10, 0));
        ins(&conn, PunchKind::End, 5, 10, 1);
        let day = stint::classify(&day_punches(&conn, 5), now());
        let outer = day
            .completed
            .iter()
            .find(|s| render::local_time(s.start.at_utc) == t(9, 0))
            .expect("outer stint");
        assert_eq!(render::local_time(outer.end.at_utc), t(10, 30));
    }

    // ---- through status::resolve ----

    fn now_local() -> chrono::DateTime<Local> {
        Local
            .from_local_datetime(&today().and_hms_opt(18, 0, 0).unwrap())
            .unwrap()
    }

    #[test]
    fn resolve_shows_refused_join_line_and_no_hint() {
        let conn = test_db();
        insert_start(&conn, 1, 18, 2);
        ins(&conn, PunchKind::End, 0, 0, 10);
        ins(&conn, PunchKind::End, 0, 0, 20);

        let view = crate::status::resolve(None, now_local(), &conn).expect("resolve");
        assert!(
            view.anomaly_lines
                .iter()
                .any(|l| l.contains("not joined to")),
            "{:?}",
            view.anomaly_lines
        );
        assert!(!view.anomaly_lines.iter().any(|l| l.starts_with("[i]")));
    }

    #[test]
    fn resolve_orders_today_block_then_older_findings_then_hint() {
        let conn = test_db();
        insert_start(&conn, 5, 9, 0);
        insert_start(&conn, 1, 18, 2);
        insert_start(&conn, 0, 8, 0);
        insert_start(&conn, 0, 9, 0);

        let view = crate::status::resolve(None, now_local(), &conn).expect("resolve");
        let lines = &view.anomaly_lines;
        let pos = |needle: &str| {
            lines
                .iter()
                .position(|l| l.contains(needle))
                .unwrap_or_else(|| panic!("no {needle:?} in {lines:?}"))
        };
        let todays = pos("2 open stints for this date");
        let older = pos("unclosed start on Sat 2026-02-07");
        let hint = pos("[i] start on Wed 2026-02-11");
        assert!(todays < older && older < hint, "{lines:?}");
        assert_eq!(lines.len(), hint + 3, "{lines:?}");
        assert_eq!(
            lines[hint + 2],
            "[!] carry-in includes anomalies from 1 earlier week, 2026-06 (see mlm week 2026-06)"
        );
    }

    // ---- week rows ----

    #[test]
    fn hint_marks_the_yesterday_week_row() {
        let conn = test_db();
        insert_start(&conn, 1, 18, 2);
        let now_local = Local
            .from_local_datetime(&today().and_hms_opt(18, 0, 0).unwrap())
            .unwrap();
        let yesterday = today() - ChronoDuration::days(1);
        let week = crate::date::WeekId::from_date(yesterday);
        let view = crate::week_view::build_week_view(&conn, week, now_local).expect("view");
        assert!(view.rows.iter().all(|r| !r.has_anomaly));
        for row in &view.rows {
            let is_yesterday = row.date_display == format_date(yesterday);
            assert_eq!(row.has_overnight, is_yesterday, "{}", row.date_display);
        }
    }

    // ---- overnight stint decision ----

    fn buckets_of(conn: &Connection) -> PunchBuckets {
        let punches = storage::punches_in_range(conn, today() - ChronoDuration::days(15), today())
            .expect("range");
        PunchBuckets::from_punches(&punches)
    }

    fn overnight_in(conn: &Connection) -> Option<OvernightStint> {
        overnight_stint(&buckets_of(conn), today(), now())
    }

    #[test]
    fn overnight_stint_lone_start_yesterday() {
        let conn = test_db();
        insert_start(&conn, 1, 18, 2);
        let o = overnight_in(&conn).expect("stint");
        assert_eq!(o.date, d(2026, 2, 11));
        assert_eq!(o.start, t(18, 2));
        assert_eq!(o.minutes_so_far, 23 * 60 + 58);
    }

    #[test]
    fn overnight_stint_none_when_joined() {
        let conn = test_db();
        insert_start(&conn, 1, 23, 0);
        ins(&conn, PunchKind::End, 0, 0, 10);
        assert_eq!(overnight_in(&conn), None);
    }

    #[test]
    fn overnight_stint_none_on_refused_join() {
        let conn = test_db();
        insert_start(&conn, 1, 18, 2);
        ins(&conn, PunchKind::End, 0, 0, 10);
        ins(&conn, PunchKind::End, 0, 0, 20);
        assert_eq!(overnight_in(&conn), None);
    }

    #[test]
    fn overnight_stint_none_with_two_opens() {
        let conn = test_db();
        insert_start(&conn, 1, 9, 0);
        insert_start(&conn, 1, 10, 0);
        assert_eq!(overnight_in(&conn), None);
    }

    #[test]
    fn overnight_stint_none_at_age_two() {
        let conn = test_db();
        insert_start(&conn, 2, 9, 0);
        assert_eq!(overnight_in(&conn), None);
    }

    #[test]
    fn overnight_stint_ignores_an_older_open_start() {
        let conn = test_db();
        insert_start(&conn, 2, 9, 0);
        insert_start(&conn, 1, 9, 0);
        let o = overnight_in(&conn).expect("stint");
        assert_eq!(o.date, d(2026, 2, 11));
    }

    #[test]
    fn overnight_stint_none_without_punches() {
        let conn = test_db();
        assert_eq!(overnight_in(&conn), None);
    }
}
