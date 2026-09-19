//! `status` command: resolution (I/O + arithmetic) and rendering (pure)
//! (SPEC.md §3.5, §7.1, §7.3, §7.4).
//!
//! Split into three layers (plans/milestone-10-status-command.md §1):
//! - [`resolve`] does all I/O and arithmetic, producing an owned
//!   [`StatusView`].
//! - [`render`] is a pure `&StatusView -> String` function with no
//!   clock, no DB, no parsing — the golden-test surface.
//! - [`run`] is the CLI entry point: calls `resolve`, prints on `Ok`,
//!   propagates the error on `Err` (`main`'s existing `exit_code` maps
//!   that to a nonzero exit and an stderr message, SPEC.md §6.3).
//!
//! Two landmines from Milestone 9's independent review are handled
//! deliberately here, not incidentally:
//! - `render::status_week_line`/`week_headline` take several adjacent
//!   `i64` minute parameters. Every call in this module goes through
//!   the single [`week_line`] helper below, which takes a
//!   `&week::WeekAccounting` (named fields) instead of positional
//!   minutes, so there is exactly one place a transposition could hide
//!   and it is directly tested (see `week_line_matches_status_week_line`).
//! - `stint::DayStints` exposes its own `has_anomaly()`/`anomalies()`,
//!   computed by a formula that duplicates `render::Anomalies` but is
//!   not bound to it by any test. This module never calls those two
//!   methods; [`to_anomalies`] builds a `render::Anomalies` directly
//!   from `DayStints`'s raw `open`/`orphaned_ends` fields instead, so
//!   anomaly rendering goes through `render::Anomalies` exclusively.

use chrono::{DateTime, Datelike, Duration as ChronoDuration, Local, NaiveDate, NaiveTime, Utc};
use rusqlite::Connection;
use std::collections::{BTreeMap, BTreeSet};

use crate::date::{self, WeekId};
use crate::render::{self, Anomalies};
use crate::stint::{self, DayStints};
use crate::storage;
use crate::time::format_minutes;
use crate::week::{self, WeekAccounting, WeekLedger};
use crate::week_target;

// ---------------------------------------------------------------------------
// StatusView and its component types
// ---------------------------------------------------------------------------

/// The §7.1 estimated-EOD line's state (F9's three cases; `None` at the
/// `StatusView` level is the third — "no open stint", line omitted).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EodState {
    /// `est. EOD HH:MM` — quota not yet met, `now + gap`.
    At(NaiveTime),
    /// `target already met` — quota already met or exceeded (gap <= 0).
    TargetAlreadyMet,
}

/// The §7.1 "X left to `<required>` required by end of `<weekday>`"
/// hint. `None` at the `StatusView` level whenever `DATE` is not today
/// (§3.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DailyTargetHint {
    /// `daily target × min(today's ISO weekday number, 5)` (§2.4).
    pub required_minutes: i64,
    /// Signed: `required_minutes - fulfillment` (`carry_in + worked`,
    /// §2.4/§5) — NOT `day_total_minutes`.
    pub gap_minutes: i64,
}

/// A stint line's end (§7.1: `HH:MM-HH:MM` or `HH:MM-now`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StintEnd {
    At(NaiveTime),
    Now,
}

/// One rendered stint row's data (§7.1/§7.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StintLine {
    pub start: NaiveTime,
    pub end: StintEnd,
    pub duration_minutes: i64,
}

/// Everything `render` needs, already computed and in final form
/// (plans/milestone-10-status-command.md §1). `render` makes no
/// decisions beyond layout and section omission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusView {
    /// Pre-formatted L1 header, e.g. `"Thu 2026-02-12"` (date::format_date_with_weekday).
    pub header: String,
    /// Completed-stints-only total for `DATE` (§2.4).
    pub day_total_minutes: i64,
    pub has_open_stint: bool,
    /// `Some` only when `DATE == today` (§3.5).
    pub daily_target: Option<DailyTargetHint>,
    /// Today's full weekday name (`date::format_weekday_full`), e.g.
    /// `"Thursday"` — used only by the day-total pace-hint line. `Some`
    /// under exactly the same condition as `daily_target` (`is_today`).
    pub weekday_name: Option<String>,
    /// `Some` only when `DATE == today && has_open_stint` (§7.1).
    pub eod: Option<EodState>,
    /// The complete, pre-rendered §7.1 week line (label, headline, and
    /// the fulfillment/target parenthetical when applicable) — verbatim
    /// from `render::status_week_line` via the [`week_line`] wrapper.
    pub week_line: String,
    /// §7.3 anomaly detail lines, already `[!] `-prefixed, verbatim from
    /// `render::Anomalies::detail_lines()`.
    pub anomaly_lines: Vec<String>,
    /// Stint rows, already sorted by start time ascending.
    pub stints: Vec<StintLine>,
    /// Note bodies, in insertion order, verbatim (already trimmed at
    /// storage, §2.3).
    pub notes: Vec<String>,
}

// ---------------------------------------------------------------------------
// render: pure StatusView -> String
// ---------------------------------------------------------------------------

/// Left-aligned label column width shared with `render::LABEL_WIDTH`
/// (§7.1's "Day total:" / "Week NN:" lines line up).
const LABEL_WIDTH: usize = render::LABEL_WIDTH;

/// L3: the day-total line (plans/milestone-10-status-command.md §3.4).
fn day_total_line(view: &StatusView) -> String {
    let mut s = format!(
        "{:<w$}{}",
        "Day total:",
        format_minutes(view.day_total_minutes),
        w = LABEL_WIDTH
    );
    if view.has_open_stint {
        s.push_str(" (+ ongoing)");
    }
    if let Some(hint) = &view.daily_target {
        let (word, magnitude) = if hint.gap_minutes >= 0 {
            ("left to", hint.gap_minutes)
        } else {
            ("over", -hint.gap_minutes)
        };
        s.push_str(&format!(
            ", {} {} {} required by end of {}",
            format_minutes(magnitude),
            word,
            format_minutes(hint.required_minutes),
            view.weekday_name.as_deref().unwrap_or_default()
        ));
    }
    match &view.eod {
        Some(EodState::At(t)) => s.push_str(&format!(", est. EOD {}", t.format("%H:%M"))),
        Some(EodState::TargetAlreadyMet) => s.push_str(", target already met"),
        None => {}
    }
    s
}

/// §3.7: `"  " + pad(range, 11) + "  (" + duration [+ ", ongoing"] + ")"`.
fn stint_line(line: &StintLine) -> String {
    let range = match line.end {
        StintEnd::At(end) => format!("{}-{}", line.start.format("%H:%M"), end.format("%H:%M")),
        StintEnd::Now => format!("{}-now", line.start.format("%H:%M")),
    };
    let ongoing = matches!(line.end, StintEnd::Now);
    format!(
        "  {:<11}  ({}{})",
        range,
        format_minutes(line.duration_minutes),
        if ongoing { ", ongoing" } else { "" }
    )
}

/// The complete rendered status view (plans/milestone-10-status-command.md
/// §3.1): pure, no I/O, no clock. Lines joined by `\n`, exactly one
/// trailing `\n`, no trailing blank line.
pub fn render(view: &StatusView) -> String {
    let mut lines: Vec<String> = vec![
        view.header.clone(),
        String::new(),
        day_total_line(view),
        view.week_line.clone(),
    ];

    if !view.anomaly_lines.is_empty() {
        lines.push(String::new());
        lines.extend(view.anomaly_lines.iter().cloned());
    }

    if !view.stints.is_empty() {
        lines.push(String::new());
        lines.extend(view.stints.iter().map(stint_line));
    }

    if !view.notes.is_empty() {
        lines.push(String::new());
        lines.push("Notes:".to_string());
        lines.extend(view.notes.iter().map(|body| format!("  - {body}")));
    }

    let mut out = lines.join("\n");
    out.push('\n');
    out
}

// ---------------------------------------------------------------------------
// resolve: I/O + arithmetic -> StatusView
// ---------------------------------------------------------------------------

/// The single call site for `render::status_week_line`, guarded against
/// Milestone 9 review finding 1 (adjacent same-typed `i64` minute
/// params): takes the whole `WeekAccounting` by reference so the
/// owed/fulfillment/target fields can never be transposed by a typo at
/// a call site, since there is only one call site and its argument
/// order is named here, not positional at every use.
fn week_line(acct: &WeekAccounting, today: NaiveDate) -> String {
    render::status_week_line(acct.week, acct.owed, acct.fulfillment, acct.target, today)
}

/// Builds a `render::Anomalies` from `DayStints`'s raw fields.
///
/// Deliberately does NOT call `DayStints::has_anomaly()` or
/// `DayStints::anomalies()` (Milestone 9 review finding 3): those two
/// methods duplicate `render::Anomalies`'s formula without being bound
/// to it by any test. Going through the raw `open`/`orphaned_ends`
/// fields and `render::Anomalies` itself keeps anomaly rendering behind
/// exactly one implementation.
fn to_anomalies(day: &DayStints) -> Anomalies {
    Anomalies {
        open_stint_count: day.open.len(),
        orphaned_end_times: day
            .orphaned_ends
            .iter()
            .map(|o| o.punch.at_utc.with_timezone(&Local).time())
            .collect(),
    }
}

/// Builds stint rows from `DayStints`, sorted by start time ascending
/// (plans/milestone-10-status-command.md §3.7). A stable sort preserves
/// each side's own relative order (completed by end instant, open by
/// start instant) for the rare tie at an identical start time.
fn build_stint_lines(day: &DayStints) -> Vec<StintLine> {
    let mut rows: Vec<StintLine> = Vec::new();
    for s in &day.completed {
        rows.push(StintLine {
            start: s.start.at_utc.with_timezone(&Local).time(),
            end: StintEnd::At(s.end.at_utc.with_timezone(&Local).time()),
            duration_minutes: s.minutes,
        });
    }
    for o in &day.open {
        rows.push(StintLine {
            start: o.start.at_utc.with_timezone(&Local).time(),
            end: StintEnd::Now,
            duration_minutes: o.minutes_so_far,
        });
    }
    rows.sort_by_key(|r| r.start);
    rows
}

/// Loads a `WeekLedger` covering exactly the weeks Milestone 6's walk
/// would visit to account `through` (SPEC.md §2.4): from the earliest
/// data week (or `through` itself, if there is no data at all) through
/// `through`, inclusive. `now_utc` is injected (contract 6) and used
/// only as `stint::classify`'s "now" parameter for completed-minutes
/// arithmetic, which never depends on it — no clock is read here.
fn build_ledger(
    conn: &Connection,
    through: WeekId,
    now_utc: DateTime<Utc>,
) -> anyhow::Result<WeekLedger> {
    let earliest_date = storage::earliest_data_date(conn)?;
    let start_week = match earliest_date.map(WeekId::from_date) {
        Some(earliest) => earliest.min(through),
        None => through,
    };

    let mut ledger = WeekLedger::new();

    // Target overrides: only weeks in the walked span matter, since
    // week::week_series never visits anything outside it.
    for week in date::week_range(start_week, through) {
        if let Some(minutes) = week_target::get_week_target(conn, &week)? {
            ledger = ledger.with_target(week, minutes);
        }
    }

    // Worked minutes and data-week markers, accumulated per week before
    // being written into the ledger (WeekLedger::with_worked replaces
    // rather than adds, so per-day contributions must be summed first).
    let mut worked_by_week: BTreeMap<WeekId, i64> = BTreeMap::new();
    let mut data_weeks: BTreeSet<WeekId> = BTreeSet::new();

    let mut date = start_week.start();
    let last_date = through.end();
    while date <= last_date {
        let day_punches = storage::punches_for_date(conn, date)?;
        let day_notes = storage::notes_for_date(conn, date)?;
        if !day_punches.is_empty() || !day_notes.is_empty() {
            let week = WeekId::from_date(date);
            data_weeks.insert(week);
            if !day_punches.is_empty() {
                let prev_punches = storage::punches_for_date(conn, date.pred_opt().unwrap())?;
                let next_punches = storage::punches_for_date(conn, date.succ_opt().unwrap())?;
                let classified =
                    stint::classify_at(&prev_punches, &day_punches, &next_punches, now_utc);
                *worked_by_week.entry(week).or_insert(0) += classified.completed_minutes();
            }
        }
        date += ChronoDuration::days(1);
    }

    for week in data_weeks {
        ledger = ledger.with_data_week(week);
    }
    for (week, minutes) in worked_by_week {
        ledger = ledger.with_worked(week, minutes);
    }

    Ok(ledger)
}

/// Resolution flow (plans/milestone-10-status-command.md §2.2, exact
/// order): parses `date_arg` against `now`'s local date, reads
/// `target_date`'s punches/notes, classifies them, accounts
/// `target_date`'s own week (not today's, §3.5/NOTES decision 26), and
/// computes the daily-target/EOD hints only when `target_date` is
/// today.
pub fn resolve(
    date_arg: Option<&str>,
    now: DateTime<Local>,
    conn: &Connection,
) -> anyhow::Result<StatusView> {
    let today = now.date_naive();
    let target_date = match date_arg {
        None => today,
        Some(s) => date::resolve_date(s, today)?,
    };
    let is_today = target_date == today;

    let punches = storage::punches_for_date(conn, target_date)?;
    let notes = storage::notes_for_date(conn, target_date)?;
    let prev_punches = storage::punches_for_date(conn, target_date.pred_opt().unwrap())?;
    let next_punches = storage::punches_for_date(conn, target_date.succ_opt().unwrap())?;

    let now_utc = now.with_timezone(&Utc);
    let day = stint::classify_at(&prev_punches, &punches, &next_punches, now_utc);

    let day_total_minutes = day.completed_minutes();
    let has_open_stint = day.is_ongoing();

    let target_week = WeekId::from_date(target_date);
    let ledger = build_ledger(conn, target_week, now_utc)?;
    let acct = week::week_accounting(target_week, &ledger, &ledger);
    let week_line_str = week_line(&acct, today);

    let anomalies = to_anomalies(&day);
    let anomaly_lines = anomalies.detail_lines();

    let (daily_target, eod, weekday_name) = if is_today {
        let daily_target_minutes = week::daily_target_minutes(acct.target);
        let weekday_number = i64::from(today.weekday().number_from_monday());
        let required_minutes = daily_target_minutes * weekday_number.min(5);
        let gap_minutes = required_minutes - acct.fulfillment;
        let hint = DailyTargetHint {
            required_minutes,
            gap_minutes,
        };
        let eod = if has_open_stint {
            if gap_minutes > 0 {
                let eod_time = (now + ChronoDuration::minutes(gap_minutes)).time();
                Some(EodState::At(eod_time))
            } else {
                Some(EodState::TargetAlreadyMet)
            }
        } else {
            None
        };
        (Some(hint), eod, Some(date::format_weekday_full(today)))
    } else {
        (None, None, None)
    };

    let stints = build_stint_lines(&day);
    let note_bodies = notes.into_iter().map(|n| n.body).collect();

    Ok(StatusView {
        header: date::format_date_with_weekday(target_date),
        day_total_minutes,
        has_open_stint,
        daily_target,
        weekday_name,
        eod,
        week_line: week_line_str,
        anomaly_lines,
        stints,
        notes: note_bodies,
    })
}

// ---------------------------------------------------------------------------
// run: CLI entry point
// ---------------------------------------------------------------------------

/// `mlm status [DATE]` (SPEC.md §3.5). Prints the rendered view to
/// stdout on success (§7.4: only `status`/`week` produce stdout output);
/// a hard error propagates for `main`'s shared `exit_code` path to print
/// to stderr and exit nonzero (§6.3).
pub fn run(conn: &Connection, now: DateTime<Local>, date_arg: Option<&str>) -> anyhow::Result<()> {
    let view = resolve(date_arg, now, conn)?;
    print!("{}", render(&view));
    Ok(())
}

/// Suppress an unused-import warning for `Datelike` pulled in only for
/// doc-comment cross-referencing clarity in a couple of call sites.
#[allow(dead_code)]
fn _use_datelike(d: NaiveDate) -> i32 {
    d.year()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::storage::PunchKind;
    use chrono::TimeZone;

    // -----------------------------------------------------------------
    // Fixtures
    // -----------------------------------------------------------------

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("test fixture is a real date")
    }

    fn t(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).expect("test fixture is a real time")
    }

    fn wk(y: i32, w: u32) -> WeekId {
        WeekId::new(y, w, "test").expect("test fixture is a real ISO week")
    }

    /// SPEC.md §7.1's "today": Thu 2026-02-12, week 2026-07.
    fn today() -> NaiveDate {
        d(2026, 2, 12)
    }

    fn now_thu_1800() -> DateTime<Local> {
        Local
            .from_local_datetime(&today().and_hms_opt(18, 0, 0).unwrap())
            .unwrap()
    }

    fn current_week_acct(owed: i64, fulfillment: i64, target: i64) -> WeekAccounting {
        WeekAccounting {
            week: wk(2026, 7),
            target,
            carry_in: 0,
            worked: fulfillment,
            fulfillment,
            owed,
            carry_out: fulfillment - target,
        }
    }

    fn closed_week_acct(week: WeekId, owed: i64) -> WeekAccounting {
        WeekAccounting {
            week,
            target: 2400,
            carry_in: 0,
            worked: 2400 - owed,
            fulfillment: 2400 - owed,
            owed,
            carry_out: owed.checked_neg().unwrap_or(0),
        }
    }

    fn base_view() -> StatusView {
        StatusView {
            header: "Thu 2026-02-12".to_string(),
            day_total_minutes: 0,
            has_open_stint: false,
            daily_target: None,
            weekday_name: None,
            eod: None,
            week_line: week_line(&current_week_acct(645, 1755, 2400), today()),
            anomaly_lines: vec![],
            stints: vec![],
            notes: vec![],
        }
    }

    fn test_db() -> Connection {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");
        db::apply_migrations(&mut conn).expect("apply migrations");
        conn
    }

    // -----------------------------------------------------------------
    // render golden tests
    // -----------------------------------------------------------------

    // T5 -- E11: zero punches and zero notes.
    #[test]
    fn t5_e11_zero_punches_and_notes_is_exactly_four_lines() {
        let view = base_view();
        let out = render(&view);
        let expected = format!(
            "Thu 2026-02-12\n\nDay total:     00h 00m\n{}\n",
            week_line(&current_week_acct(645, 1755, 2400), today())
        );
        assert_eq!(out, expected);
        assert_eq!(out.lines().count(), 4);
        assert!(out.ends_with('\n'));
        assert!(!out.ends_with("\n\n"));
    }

    // T4 -- F4: note-only day, stint section entirely absent.
    #[test]
    fn t4_f4_note_only_day_omits_stint_section() {
        let mut view = base_view();
        view.notes = vec![
            "fixed migration runner bug".to_string(),
            "started punch pairing tests".to_string(),
        ];
        let out = render(&view);
        assert!(out.contains("Notes:"));
        assert!(out.contains("  - fixed migration runner bug"));
        assert!(out.contains("  - started punch pairing tests"));
        // No stint line: a stint line is the only kind of line indented by
        // exactly two spaces that also carries a parenthesised duration
        // (the week line's own "(fulfillment .. / target ..)" parenthetical
        // starts at column 0, not column 2).
        assert!(
            !out.lines().any(|l| l.starts_with("  ") && l.contains('(')),
            "no stint/duration line should render: {out}"
        );
        assert!(!out.contains("\n\n\n"), "no doubled blank line");
    }

    // T1 -- F1: fresh day, one open stint only.
    #[test]
    fn t1_f1_single_open_stint_no_completed() {
        let mut view = base_view();
        view.has_open_stint = true;
        view.daily_target = Some(DailyTargetHint {
            required_minutes: 480,
            gap_minutes: 480,
        });
        view.weekday_name = Some("Thursday".to_string());
        view.eod = Some(EodState::At(t(18, 0) + ChronoDuration::minutes(480)));
        view.stints = vec![StintLine {
            start: t(9, 0),
            end: StintEnd::Now,
            duration_minutes: 0,
        }];
        let out = render(&view);
        assert!(out.contains("Day total:     00h 00m (+ ongoing)"));
        assert!(out.contains("  09:00-now    (00h 00m, ongoing)"));
        assert!(!out.contains("[!]"));
        assert!(!out.contains("Notes:"));
    }

    // T2 -- F2: one closed stint, daily-target hint present, no EOD line.
    #[test]
    fn t2_f2_ordinary_day_no_eod_line_without_open_stint() {
        let mut view = base_view();
        view.day_total_minutes = 510; // 08h 30m
        view.stints = vec![StintLine {
            start: t(9, 0),
            end: StintEnd::At(t(17, 30)),
            duration_minutes: 510,
        }];
        view.daily_target = Some(DailyTargetHint {
            required_minutes: 480,
            gap_minutes: -30,
        });
        view.weekday_name = Some("Thursday".to_string());
        let out = render(&view);
        assert!(
            out.contains(
                "Day total:     08h 30m, 00h 30m over 08h 00m required by end of Thursday"
            )
        );
        assert!(!out.contains("(+ ongoing)"));
        assert!(!out.contains("est. EOD"));
        assert!(!out.contains("target already met"));
        assert!(out.contains("  09:00-17:30  (08h 30m)"));
    }

    // T3 -- F3: out-of-order entry pairs by time, never leaks entry order.
    #[test]
    fn t3_f3_stint_order_reflects_time_not_entry_order() {
        let mut view = base_view();
        view.day_total_minutes = 480;
        view.stints = vec![
            StintLine {
                start: t(9, 0),
                end: StintEnd::At(t(13, 0)),
                duration_minutes: 240,
            },
            StintLine {
                start: t(14, 0),
                end: StintEnd::At(t(18, 0)),
                duration_minutes: 240,
            },
        ];
        let out = render(&view);
        let idx1 = out.find("09:00-13:00").unwrap();
        let idx2 = out.find("14:00-18:00").unwrap();
        assert!(idx1 < idx2);
        assert!(!out.contains("[!]"));
    }

    // T6a -- F9 state 1: open stint, gap > 0.
    #[test]
    fn t6a_f9_open_stint_with_gap_shows_est_eod() {
        let mut view = base_view();
        view.day_total_minutes = 445; // 07h 25m
        view.has_open_stint = true;
        view.daily_target = Some(DailyTargetHint {
            required_minutes: 1920,
            gap_minutes: 165,
        });
        view.weekday_name = Some("Thursday".to_string());
        view.eod = Some(EodState::At(t(20, 45)));
        let out = render(&view);
        assert!(out.contains(
            "Day total:     07h 25m (+ ongoing), 02h 45m left to 32h 00m required by end of Thursday, est. EOD 20:45"
        ));
    }

    // T6b -- F9 state 2: gap == 0 and gap < 0 both take the "met" branch.
    #[test]
    fn t6b_f9_open_stint_gap_zero_or_negative_is_target_already_met() {
        for gap in [0i64, -20i64] {
            let mut view = base_view();
            view.day_total_minutes = 480 - gap;
            view.has_open_stint = true;
            view.daily_target = Some(DailyTargetHint {
                required_minutes: 480,
                gap_minutes: gap,
            });
            view.weekday_name = Some("Thursday".to_string());
            view.eod = Some(EodState::TargetAlreadyMet);
            let out = render(&view);
            assert!(out.contains(", target already met"), "gap {gap}: {out}");
            assert!(!out.contains("est. EOD"), "gap {gap}: {out}");
            if gap >= 0 {
                assert!(
                    out.contains("00h 00m left to 08h 00m required by end of Thursday"),
                    "gap {gap}: {out}"
                );
                assert!(!out.contains("over 08h 00m required"), "gap {gap}: {out}");
            } else {
                assert!(
                    out.contains(&format!(
                        "{} over 08h 00m required by end of Thursday",
                        format_minutes(-gap)
                    )),
                    "gap {gap}: {out}"
                );
                assert!(
                    !out.contains("left to 08h 00m required"),
                    "gap {gap}: {out}"
                );
            }
        }
    }

    // T6c -- F9 state 3: no open stint, EOD segment entirely absent.
    #[test]
    fn t6c_f9_no_open_stint_omits_eod_segment_entirely() {
        let mut view = base_view();
        view.day_total_minutes = 480;
        view.has_open_stint = false;
        view.daily_target = Some(DailyTargetHint {
            required_minutes: 480,
            gap_minutes: 0,
        });
        view.weekday_name = Some("Thursday".to_string());
        view.eod = None;
        let out = render(&view);
        assert!(!out.contains("est. EOD"));
        assert!(!out.contains("target already met"));
        assert!(!out.contains("(+ ongoing)"));
        assert!(out.contains("left to 08h 00m required by end of Thursday"));
    }

    // T7 -- F10: past date, closed week, no daily-target/EOD lines.
    // Golden-matches SPEC.md §7.1's second worked example verbatim.
    #[test]
    fn t7_f10_golden_second_spec_example() {
        let view = StatusView {
            header: "Mon 2026-01-05".to_string(),
            day_total_minutes: 375,
            has_open_stint: false,
            daily_target: None,
            weekday_name: None,
            eod: None,
            week_line: week_line(&closed_week_acct(wk(2026, 2), 100), today()),
            anomaly_lines: vec![],
            stints: vec![StintLine {
                start: t(8, 30),
                end: StintEnd::At(t(14, 45)),
                duration_minutes: 375,
            }],
            notes: vec![],
        };
        let out = render(&view);
        let expected = "Mon 2026-01-05\n\
             \n\
             Day total:     06h 15m\n\
             Week 2026-02:  Total still owed: 01h 40m\n\
             \n\
             \x20\x2008:30-14:45  (06h 15m)\n";
        assert_eq!(out, expected);
        assert!(!out.contains("daily target"));
        assert!(!out.contains("required by end of"));
        assert!(!out.contains("est. EOD"));
        assert!(!out.contains("target already met"));
        assert!(out.contains("Week 2026-02:"));
        assert!(!out.contains("fulfillment"));
    }

    // T8 -- F11: different day within the current week keeps deadline
    // framing keyed to today's weekday.
    #[test]
    fn t8_f11_different_day_in_current_week_has_no_daily_target_but_deadline_week_line() {
        let mut view = base_view();
        view.header = "Mon 2026-02-09".to_string();
        // daily_target/eod stay None: DATE isn't today.
        let out = render(&view);
        assert!(!out.contains("Monday"));
        assert!(out.contains("left by end of Thursday"));
        assert!(!out.contains("daily target"));
        assert!(!out.contains("required by end of"));
        assert!(!out.contains("est. EOD"));
        assert!(!out.contains("target already met"));
    }

    // T9 -- E7: three unmatched starts.
    #[test]
    fn t9_e7_multi_open_anomaly_rendering() {
        let mut view = base_view();
        view.has_open_stint = true;
        view.anomaly_lines = vec!["[!] 3 open stints for this date (unmatched starts)".to_string()];
        view.stints = vec![
            StintLine {
                start: t(9, 0),
                end: StintEnd::Now,
                duration_minutes: 540,
            },
            StintLine {
                start: t(10, 0),
                end: StintEnd::Now,
                duration_minutes: 480,
            },
            StintLine {
                start: t(11, 0),
                end: StintEnd::Now,
                duration_minutes: 420,
            },
        ];
        let out = render(&view);
        assert_eq!(
            out.matches("[!]").count(),
            1,
            "exactly one anomaly line: {out}"
        );
        assert!(out.contains("[!] 3 open stints for this date (unmatched starts)"));
        let idx_anomaly = out.find("[!]").unwrap();
        let idx_first_stint = out.find("09:00-now").unwrap();
        assert!(idx_anomaly < idx_first_stint);
        for s in ["09:00-now", "10:00-now", "11:00-now"] {
            assert!(out.contains(s), "{s} missing: {out}");
        }
        assert_eq!(
            view.day_total_minutes, 0,
            "open time never affects day total"
        );
    }

    // T10 -- E8: two orphaned ends plus one clean pair.
    #[test]
    fn t10_e8_orphaned_ends_produce_no_stint_lines() {
        let mut view = base_view();
        view.day_total_minutes = 480;
        view.anomaly_lines = vec![
            "[!] orphaned end at 18:00 (no matching start)".to_string(),
            "[!] orphaned end at 19:30 (no matching start)".to_string(),
        ];
        view.stints = vec![StintLine {
            start: t(9, 0),
            end: StintEnd::At(t(17, 0)),
            duration_minutes: 480,
        }];
        let out = render(&view);
        assert_eq!(out.matches("[!]").count(), 2);
        assert!(out.contains("orphaned end at 18:00"));
        assert!(out.contains("orphaned end at 19:30"));
        // 1 paren from the stint line, plus 1 from each "(no matching
        // start)" anomaly line -- none from an orphan-shaped stint line,
        // since orphans never produce one.
        assert_eq!(out.matches('(').count(), 4);
        assert!(out.contains("09:00-17:00  (08h 00m)"));
    }

    // T11 -- the full golden test: SPEC.md §7.1's FIRST worked example.
    #[test]
    fn t11_golden_full_first_spec_example() {
        let acct = current_week_acct(645, 1755, 2400);
        let view = StatusView {
            header: "Thu 2026-02-12".to_string(),
            day_total_minutes: 445,
            has_open_stint: true,
            daily_target: Some(DailyTargetHint {
                required_minutes: 1920,
                gap_minutes: 165,
            }),
            weekday_name: Some("Thursday".to_string()),
            eod: Some(EodState::At(t(20, 45))),
            week_line: week_line(&acct, today()),
            anomaly_lines: vec![],
            stints: vec![
                StintLine {
                    start: t(9, 0),
                    end: StintEnd::At(t(13, 0)),
                    duration_minutes: 240,
                },
                StintLine {
                    start: t(14, 5),
                    end: StintEnd::At(t(17, 30)),
                    duration_minutes: 205,
                },
                StintLine {
                    start: t(17, 45),
                    end: StintEnd::Now,
                    duration_minutes: 15,
                },
            ],
            notes: vec![
                "fixed migration runner bug".to_string(),
                "started punch pairing tests".to_string(),
            ],
        };
        let out = render(&view);
        let expected = "Thu 2026-02-12\n\
            \n\
            Day total:     07h 25m (+ ongoing), 02h 45m left to 32h 00m required by end of Thursday, est. EOD 20:45\n\
            Week 2026-07:  10h 45m left by end of Thursday (fulfillment 29h 15m / target 40h 00m)\n\
            \n\
            \x20\x2009:00-13:00  (04h 00m)\n\
            \x20\x2014:05-17:30  (03h 25m)\n\
            \x20\x2017:45-now    (00h 15m, ongoing)\n\
            \n\
            Notes:\n\
            \x20\x20- fixed migration runner bug\n\
            \x20\x20- started punch pairing tests\n";
        assert_eq!(out, expected);
    }

    // T12 -- §4.2 duration-format consistency, over T11/T7/T9/T6b outputs.
    fn assert_durations_well_formed(s: &str) {
        let words: Vec<&str> = s.split_whitespace().collect();
        for i in 0..words.len() {
            let w = words[i];
            let unsigned = w.strip_prefix('-').unwrap_or(w);
            let Some(hour_digits) = unsigned.strip_suffix('h') else {
                continue;
            };
            if hour_digits.is_empty() || !hour_digits.bytes().all(|b| b.is_ascii_digit()) {
                continue; // not actually an "Hh" token (e.g. "Week", punctuation)
            }
            assert!(
                hour_digits.len() >= 2,
                "hour part not zero-padded: {w:?} in {s:?}"
            );
            let next = words
                .get(i + 1)
                .unwrap_or_else(|| panic!("hour token {w:?} has no following minute token"));
            let minute_run: String = next
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == 'm')
                .collect();
            assert!(
                minute_run.ends_with('m'),
                "minute token malformed: {next:?} in {s:?}"
            );
            let minute_digits = &minute_run[..minute_run.len() - 1];
            assert_eq!(
                minute_digits.len(),
                2,
                "minute part not exactly 2 digits: {next:?} in {s:?}"
            );
            assert!(minute_digits.bytes().all(|b| b.is_ascii_digit()));
        }
    }

    #[test]
    fn t12_every_duration_matches_the_canonical_format() {
        let acct = current_week_acct(645, 1755, 2400);
        let t11 = render(&StatusView {
            header: "Thu 2026-02-12".to_string(),
            day_total_minutes: 445,
            has_open_stint: true,
            daily_target: Some(DailyTargetHint {
                required_minutes: 1920,
                gap_minutes: 165,
            }),
            weekday_name: Some("Thursday".to_string()),
            eod: Some(EodState::At(t(20, 45))),
            week_line: week_line(&acct, today()),
            anomaly_lines: vec![],
            stints: vec![StintLine {
                start: t(9, 0),
                end: StintEnd::At(t(13, 0)),
                duration_minutes: 240,
            }],
            notes: vec![],
        });
        let t7 = render(&StatusView {
            header: "Mon 2026-01-05".to_string(),
            day_total_minutes: 375,
            has_open_stint: false,
            daily_target: None,
            weekday_name: None,
            eod: None,
            week_line: week_line(&closed_week_acct(wk(2026, 2), 100), today()),
            anomaly_lines: vec![],
            stints: vec![StintLine {
                start: t(8, 30),
                end: StintEnd::At(t(14, 45)),
                duration_minutes: 375,
            }],
            notes: vec![],
        });
        let mut t6b_view = base_view();
        t6b_view.day_total_minutes = 500;
        t6b_view.has_open_stint = true;
        t6b_view.daily_target = Some(DailyTargetHint {
            required_minutes: 480,
            gap_minutes: -20,
        });
        t6b_view.weekday_name = Some("Thursday".to_string());
        t6b_view.eod = Some(EodState::TargetAlreadyMet);
        let t6b = render(&t6b_view);

        for out in [t11, t7, t6b] {
            assert_durations_well_formed(&out);
        }
    }

    // T13 -- §7 plain ASCII, over ASCII-only fixtures; non-ASCII note
    // bodies pass through unmangled (companion assertion).
    #[test]
    fn t13_plain_ascii_output_and_non_ascii_notes_pass_through() {
        let acct = current_week_acct(645, 1755, 2400);
        let ascii_view = StatusView {
            header: "Thu 2026-02-12".to_string(),
            day_total_minutes: 445,
            has_open_stint: true,
            daily_target: Some(DailyTargetHint {
                required_minutes: 1920,
                gap_minutes: 165,
            }),
            weekday_name: Some("Thursday".to_string()),
            eod: Some(EodState::At(t(20, 45))),
            week_line: week_line(&acct, today()),
            anomaly_lines: vec!["[!] orphaned end at 18:00 (no matching start)".to_string()],
            stints: vec![StintLine {
                start: t(9, 0),
                end: StintEnd::At(t(13, 0)),
                duration_minutes: 240,
            }],
            notes: vec!["ascii only note".to_string()],
        };
        let out = render(&ascii_view);
        assert!(
            out.bytes()
                .all(|b| b == b'\n' || (0x20..=0x7E).contains(&b)),
            "non-plain-ASCII byte found: {out:?}"
        );

        let mut non_ascii_view = ascii_view;
        non_ascii_view.notes = vec!["caf\u{e9} r\u{e9}sum\u{e9}".to_string()];
        let out2 = render(&non_ascii_view);
        assert!(
            out2.contains("caf\u{e9} r\u{e9}sum\u{e9}"),
            "note body must pass through unmangled: {out2:?}"
        );
    }

    // Guards the render function itself never emits a doubled blank line
    // or a trailing blank line for any of the section-omission cases.
    #[test]
    fn render_never_produces_a_trailing_or_doubled_blank_line() {
        for view in [base_view(), {
            let mut v = base_view();
            v.notes = vec!["only a note".to_string()];
            v
        }] {
            let out = render(&view);
            assert!(!out.contains("\n\n\n"));
            assert!(out.ends_with('\n') && !out.ends_with("\n\n"));
        }
    }

    // -----------------------------------------------------------------
    // resolve tests (in-memory DB)
    // -----------------------------------------------------------------

    // T7 (resolve half) -- F10: status for a past date in a closed week.
    #[test]
    fn resolve_f10_past_date_different_closed_week() {
        let conn = test_db();
        storage::insert_punch(&conn, PunchKind::Start, d(2026, 1, 5), t(8, 30), &Local)
            .expect("insert");
        storage::insert_punch(&conn, PunchKind::End, d(2026, 1, 5), t(14, 45), &Local)
            .expect("insert");

        let view = resolve(Some("2026-01-05"), now_thu_1800(), &conn).expect("resolve");
        assert_eq!(view.header, "Mon 2026-01-05");
        assert_eq!(view.day_total_minutes, 375);
        assert!(view.daily_target.is_none());
        assert!(view.eod.is_none());
        assert!(view.week_line.starts_with("Week 2026-02:"));
        assert!(view.week_line.contains("Total"));
        assert!(!view.week_line.contains("fulfillment"));
    }

    // T8 (resolve half) -- F11: is_today is false, but the week the
    // displayed date belongs to IS the actual current week; the two
    // must not be conflated.
    #[test]
    fn resolve_f11_is_today_false_but_week_is_current() {
        let conn = test_db();
        storage::insert_punch(&conn, PunchKind::Start, d(2026, 2, 9), t(9, 0), &Local)
            .expect("insert");
        storage::insert_punch(&conn, PunchKind::End, d(2026, 2, 9), t(17, 0), &Local)
            .expect("insert");

        let view = resolve(Some("2026-02-09"), now_thu_1800(), &conn).expect("resolve");
        assert_eq!(view.header, "Mon 2026-02-09");
        assert!(view.daily_target.is_none(), "DATE is not today");
        assert!(view.eod.is_none());
        assert!(
            view.week_line.contains("left by end of Thursday"),
            "week_line: {}",
            view.week_line
        );
        assert!(!view.week_line.contains("Monday"));

        // Independently: week_framing itself says this week IS current.
        let week = WeekId::from_date(d(2026, 2, 9));
        assert_eq!(
            render::week_framing(week, today()),
            render::WeekFraming::Current
        );
    }

    #[test]
    fn resolve_e7_multi_open_and_e8_orphaned_end_via_anomalies_only() {
        let conn = test_db();
        storage::insert_punch(&conn, PunchKind::Start, today(), t(9, 0), &Local).expect("insert");
        storage::insert_punch(&conn, PunchKind::Start, today(), t(10, 0), &Local).expect("insert");
        storage::insert_punch(&conn, PunchKind::End, today(), t(8, 0), &Local).expect("insert");

        let view = resolve(None, now_thu_1800(), &conn).expect("resolve");
        assert_eq!(view.anomaly_lines.len(), 2);
        assert!(view.anomaly_lines[0].contains("2 open stints"));
        assert!(view.anomaly_lines[1].contains("orphaned end at 08:00"));
        assert_eq!(view.stints.len(), 2, "the orphan produces no stint line");
        assert_eq!(view.weekday_name, Some("Thursday".to_string()));
        assert!(view.daily_target.is_some());
    }

    #[test]
    fn resolve_malformed_date_is_a_hard_error() {
        let conn = test_db();
        assert!(resolve(Some("2026-02-30"), now_thu_1800(), &conn).is_err());
        assert!(resolve(Some("13/02/2026"), now_thu_1800(), &conn).is_err());
    }

    #[test]
    fn resolve_shorthand_matches_equivalent_absolute_date() {
        let conn = test_db();
        storage::insert_punch(&conn, PunchKind::Start, d(2026, 1, 5), t(8, 30), &Local)
            .expect("insert");
        storage::insert_punch(&conn, PunchKind::End, d(2026, 1, 5), t(14, 45), &Local)
            .expect("insert");

        // now_thu_1800() is 2026-02-12; 2026-01-05 is 38 days earlier.
        let via_absolute = resolve(Some("2026-01-05"), now_thu_1800(), &conn).expect("resolve");
        let via_shorthand = resolve(Some("-38"), now_thu_1800(), &conn).expect("resolve");
        assert_eq!(via_absolute.header, via_shorthand.header);
        assert_eq!(
            via_absolute.day_total_minutes,
            via_shorthand.day_total_minutes
        );
        assert_eq!(via_shorthand.header, "Mon 2026-01-05");
    }

    #[test]
    fn resolve_future_absolute_date_still_succeeds_and_renders_empty() {
        let conn = test_db();
        // now_thu_1800() is 2026-02-12; this stays permissive, unlike
        // start/stop/note's future-date rejection.
        let view = resolve(Some("2026-03-01"), now_thu_1800(), &conn).expect("resolve");
        assert_eq!(view.header, date::format_date_with_weekday(d(2026, 3, 1)));
        assert!(view.stints.is_empty());
        assert!(view.notes.is_empty());
        assert_eq!(view.day_total_minutes, 0);
    }

    #[test]
    fn resolve_note_only_day_has_no_stint_section_but_has_notes() {
        let conn = test_db();
        storage::insert_note(
            &conn,
            today(),
            "just a note",
            now_thu_1800().with_timezone(&Utc),
        )
        .expect("insert");
        let view = resolve(None, now_thu_1800(), &conn).expect("resolve");
        assert!(view.stints.is_empty());
        assert_eq!(view.notes, vec!["just a note".to_string()]);
        let out = render(&view);
        assert!(out.contains("Notes:"));
        assert!(
            !out.lines().any(|l| l.starts_with("  ") && l.contains('(')),
            "no stint/duration line should render: {out}"
        );
    }

    // -----------------------------------------------------------------
    // midnight-spanning splice (boundary-stint-pairing spec §4.3):
    // `resolve`/`build_ledger` now fetch neighbor-date punches and call
    // `stint::classify_at` instead of `stint::classify`.
    // -----------------------------------------------------------------

    #[test]
    fn resolve_midnight_splice_earlier_date_shows_completed_stint_no_anomaly() {
        let conn = test_db();
        let d1 = d(2026, 2, 9);
        let d2 = d(2026, 2, 10);
        storage::insert_punch(&conn, PunchKind::Start, d1, t(23, 30), &Local).expect("insert");
        storage::insert_punch(&conn, PunchKind::End, d2, t(0, 45), &Local).expect("insert");

        let view = resolve(Some("2026-02-09"), now_thu_1800(), &conn).expect("resolve");
        assert_eq!(view.day_total_minutes, 75);
        assert!(view.anomaly_lines.is_empty());
        assert_eq!(view.stints.len(), 1);
        assert_eq!(view.stints[0].start, t(23, 30));
        assert_eq!(view.stints[0].end, StintEnd::At(t(0, 45)));
    }

    #[test]
    fn resolve_midnight_splice_later_date_shows_no_orphan_anomaly() {
        let conn = test_db();
        let d1 = d(2026, 2, 9);
        let d2 = d(2026, 2, 10);
        storage::insert_punch(&conn, PunchKind::Start, d1, t(23, 30), &Local).expect("insert");
        storage::insert_punch(&conn, PunchKind::End, d2, t(0, 45), &Local).expect("insert");

        let view = resolve(Some("2026-02-10"), now_thu_1800(), &conn).expect("resolve");
        assert!(
            view.anomaly_lines.is_empty(),
            "the 00:45 orphan is consumed by the splice: {:?}",
            view.anomaly_lines
        );
        assert!(
            view.stints.is_empty(),
            "the spliced stint belongs to D, never D+1's own list"
        );
        assert_eq!(view.day_total_minutes, 0);
    }

    // Direct regression companion: confirms the new call site doesn't
    // accidentally always splice -- only ever mechanically, per Task 1's
    // 1:1 gate. `D` has two trailing opens (E7, not 1:1), so `D+1`'s
    // single `End 00:30` cannot close either of them and stays a genuine
    // orphaned end (note: this is a deliberate deviation from the task
    // plan's stated expectation of "no anomaly on that side either" --
    // `render::Anomalies::has_any()`/`detail_lines()` flag ANY orphaned
    // end regardless of open-stint count, as
    // `resolve_e7_multi_open_and_e8_orphaned_end_via_anomalies_only`
    // above already establishes for the same-date case; see the task 2
    // completion report for the full explanation).
    #[test]
    fn resolve_midnight_boundary_still_flags_non_1to1_shapes() {
        let conn = test_db();
        let d1 = d(2026, 2, 9);
        let d2 = d(2026, 2, 10);
        storage::insert_punch(&conn, PunchKind::Start, d1, t(22, 0), &Local).expect("insert");
        storage::insert_punch(&conn, PunchKind::Start, d1, t(23, 0), &Local).expect("insert");
        storage::insert_punch(&conn, PunchKind::End, d2, t(0, 30), &Local).expect("insert");

        let view_d1 = resolve(Some("2026-02-09"), now_thu_1800(), &conn).expect("resolve");
        assert_eq!(view_d1.anomaly_lines.len(), 1);
        assert!(view_d1.anomaly_lines[0].contains("2 open stints"));
        assert_eq!(view_d1.day_total_minutes, 0);

        let view_d2 = resolve(Some("2026-02-10"), now_thu_1800(), &conn).expect("resolve");
        assert_eq!(view_d2.day_total_minutes, 0);
        assert_eq!(view_d2.anomaly_lines.len(), 1);
        assert!(view_d2.anomaly_lines[0].contains("orphaned end at 00:30"));
    }

    #[test]
    fn resolve_midnight_splice_reflected_in_week_line() {
        let conn = test_db();
        // Both dates fall inside week 2026-07 (Mon 2026-02-09 .. Sun
        // 2026-02-15), the same week `today()` (Thu 2026-02-12) is in.
        let d1 = d(2026, 2, 9);
        let d2 = d(2026, 2, 10);
        storage::insert_punch(&conn, PunchKind::Start, d1, t(23, 30), &Local).expect("insert");
        storage::insert_punch(&conn, PunchKind::End, d2, t(0, 45), &Local).expect("insert");

        let view = resolve(Some("2026-02-09"), now_thu_1800(), &conn).expect("resolve");
        // The recovered 75 minutes is the week's entire `worked`/
        // `fulfillment` -- no other data seeded, no prior week's carry.
        let expected = current_week_acct(2400 - 75, 75, 2400);
        assert_eq!(view.week_line, week_line(&expected, today()));
    }

    // -----------------------------------------------------------------
    // F9b: carry-inclusive required-by-day (decision 52/53), resolve-level.
    // -----------------------------------------------------------------

    #[test]
    fn resolve_f9b_large_carry_in_negative_pace_on_day_one() {
        let conn = test_db();
        week_target::set_week_target(&conn, &wk(2026, 6), 0).expect("set target");
        for day in [2, 3, 4, 5, 6] {
            storage::insert_punch(&conn, PunchKind::Start, d(2026, 2, day), t(8, 0), &Local)
                .expect("insert");
            storage::insert_punch(&conn, PunchKind::End, d(2026, 2, day), t(14, 40), &Local)
                .expect("insert");
        }
        let monday_0900 = Local
            .from_local_datetime(&d(2026, 2, 9).and_hms_opt(9, 0, 0).unwrap())
            .unwrap();
        let view = resolve(None, monday_0900, &conn).expect("resolve");
        assert_eq!(view.header, "Mon 2026-02-09");
        assert_eq!(
            view.day_total_minutes, 0,
            "day total unaffected by carry-in"
        );
        let hint = view.daily_target.expect("is_today");
        assert_eq!(hint.required_minutes, 480, "daily_target(2400) * min(1,5)");
        assert_eq!(hint.gap_minutes, -1520, "480 - fulfillment(2000)");
        assert_eq!(view.weekday_name, Some("Monday".to_string()));
        let out = render(&view);
        assert!(
            out.contains("Day total:     00h 00m,"),
            "day total line: {out}"
        );
        assert!(out.contains("25h 20m over 08h 00m required by end of Monday"));
    }

    #[test]
    fn resolve_f9b_saturday_pin_multiple_of_five_target() {
        let conn = test_db();
        storage::insert_punch(&conn, PunchKind::Start, d(2026, 2, 14), t(9, 0), &Local)
            .expect("insert");
        storage::insert_punch(&conn, PunchKind::End, d(2026, 2, 14), t(11, 20), &Local)
            .expect("insert"); // 140 min, arbitrary day total, not load-bearing
        let saturday_1200 = Local
            .from_local_datetime(&d(2026, 2, 14).and_hms_opt(12, 0, 0).unwrap())
            .unwrap();
        let view = resolve(None, saturday_1200, &conn).expect("resolve");
        assert_eq!(view.header, "Sat 2026-02-14");
        let hint = view.daily_target.expect("is_today");
        assert_eq!(
            hint.required_minutes, 2400,
            "5 * daily_target_minutes(2400) == target itself here"
        );
        assert_eq!(view.weekday_name, Some("Saturday".to_string()));
    }

    #[test]
    fn resolve_f9b_sunday_pin_non_multiple_of_five_target_override() {
        let conn = test_db();
        week_target::set_week_target(&conn, &wk(2026, 7), 2011).expect("set target"); // 33h 31m
        let sunday_1200 = Local
            .from_local_datetime(&d(2026, 2, 15).and_hms_opt(12, 0, 0).unwrap())
            .unwrap();
        let view = resolve(None, sunday_1200, &conn).expect("resolve");
        assert_eq!(view.header, "Sun 2026-02-15");
        let hint = view.daily_target.expect("is_today");
        assert_eq!(
            hint.required_minutes, 2010,
            "5 * floor(2011/5) = 2010, NOT 2011"
        );
        assert_ne!(
            hint.required_minutes, 2011,
            "must not silently round up to the raw override"
        );
        assert_eq!(view.weekday_name, Some("Sunday".to_string()));
        let out = render(&view);
        assert!(out.contains("33h 30m required by end of Sunday"));
    }

    // -----------------------------------------------------------------
    // week_line helper sanity: confirms the wrapper never transposes.
    // -----------------------------------------------------------------

    #[test]
    fn week_line_matches_status_week_line_field_for_field() {
        let acct = current_week_acct(645, 1755, 2400);
        let want =
            render::status_week_line(acct.week, acct.owed, acct.fulfillment, acct.target, today());
        assert_eq!(week_line(&acct, today()), want);
    }
}
