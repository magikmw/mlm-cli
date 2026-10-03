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
//! - `render::status_week_line`/`week_headline` take a
//!   `&week::WeekAccounting` (named fields) instead of positional
//!   minutes. Every call in this module goes through the single
//!   [`week_line`] helper below, which is directly tested (see
//!   `week_line_matches_status_week_line`).
//! - `stint::DayStints` exposes its own `has_anomaly()`/`anomalies()`,
//!   computed by a formula that duplicates `anomalies::DateAnomalies` but
//!   is not bound to it by any test. This module never calls those two
//!   methods; `anomalies::build_facts` builds a `DateAnomalies` directly
//!   from `DayStints`'s raw `open`/`orphaned_ends` fields instead, so
//!   anomaly rendering goes through `DateAnomalies` exclusively.
//!
//! Storage reads for the week line come from `history::load_history`,
//! the same loader `mlm week` uses, so both commands account a week
//! identically and a failed read is an error.

use chrono::{DateTime, Duration as ChronoDuration, Local, NaiveDate, NaiveTime, Utc};
use rusqlite::Connection;

use crate::anomalies::{self, DateAnomalies};
use crate::carry_scan;
use crate::date::{self, WeekId};
use crate::history;
use crate::lookback;
use crate::render::{self, CarryMarker, local_time};
use crate::stint::{self, DayStints};
use crate::storage;
use crate::time::format_minutes;
use crate::week::{self, WeekAccounting};

/// The lookback scan lives in `lookback.rs`. These re-exports keep the
/// old `status::lookback` paths valid for `commands.rs`.
pub use crate::lookback::{LookbackFinding, LookbackKind, lookback};

// ---------------------------------------------------------------------------
// StatusView and its component types
// ---------------------------------------------------------------------------

/// The day an `est. EOD` estimate lands on, relative to today.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EodDay {
    /// No suffix.
    Today,
    /// ` (tomorrow)`.
    Tomorrow,
    /// 2-6 days later: ` (<weekday abbrev>)`.
    Weekday(NaiveDate),
    /// 7+ days later: ` (<weekday abbrev> <YYYY-MM-DD>)`.
    Dated(NaiveDate),
}

impl EodDay {
    fn classify(today: NaiveDate, lands: NaiveDate) -> Self {
        match (lands - today).num_days() {
            i64::MIN..=0 => Self::Today,
            1 => Self::Tomorrow,
            2..=6 => Self::Weekday(lands),
            _ => Self::Dated(lands),
        }
    }

    fn suffix(self) -> String {
        match self {
            Self::Today => String::new(),
            Self::Tomorrow => " (tomorrow)".to_string(),
            Self::Weekday(d) => {
                let full = date::format_date_with_weekday(d);
                format!(" ({})", full.split(' ').next().unwrap_or_default())
            }
            Self::Dated(d) => format!(" ({})", date::format_date_with_weekday(d)),
        }
    }
}

/// The §7.1 estimated-EOD line's state (F9's three cases; `None` at the
/// `StatusView` level is the third — "no open stint", line omitted).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EodState {
    /// `est. EOD HH:MM` — quota not yet met. The time is the open stint's
    /// start plus `required - (fulfillment without the running stint's live
    /// minutes)`, which equals now plus the gap. The `EodDay` says which day
    /// it lands on and drives the suffix (SPEC §7.1).
    At(NaiveTime, EodDay),
    /// `done for today` — quota already met or exceeded (gap <= 0).
    TargetAlreadyMet,
}

/// A stint line's end (§7.1: `HH:MM-HH:MM` or `HH:MM-now`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StintEnd {
    At(NaiveTime),
    Now,
}

/// §4's three-way classification of how stale an open stint's date is,
/// relative to `today`. Threaded into every `StintLine` via
/// `build_stint_lines`'s parameter, and stored again, unchanged, on
/// `StatusView` for `day_total_line()` to read directly (§4's Day-total
/// match requirement).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenStintAge {
    /// `target_date == today` — today's own open stint (§4, unchanged
    /// rendering).
    Today,
    /// `target_date == today - 1 day` — keeps its duration, gains a
    /// caption.
    Yesterday,
    /// `target_date <= today - 2 days` — duration dropped, `(unclosed)`.
    TwoOrMoreDaysBack,
}

/// One rendered stint row's data (§7.1/§7.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StintLine {
    pub start: NaiveTime,
    pub end: StintEnd,
    pub duration_minutes: i64,
    /// True for a completed stint whose `start.date != end.date`
    /// (§2). Always `false` for an open stint (`StintEnd::Now`) —
    /// §2's span cue applies to completed stints only.
    pub spans_to_next_day: bool,
    /// Only meaningful when `end == StintEnd::Now`; `None` for a
    /// completed stint. §4's three-way open-stint age classification.
    pub open_stint_age: Option<OpenStintAge>,
}

/// Everything `render` needs, already computed and in final form
/// (plans/milestone-10-status-command.md §1). `render` makes no
/// decisions beyond layout and section omission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusView {
    /// Pre-formatted L1 header, e.g. `"Thu 2026-02-12"` (date::format_date_with_weekday).
    pub header: String,
    /// §3 header suffix, pre-rendered, e.g. `"  (00:45 continues
    /// previous day's stint)"`. `None` when no splice consumed
    /// `target_date`'s first punch.
    pub receiving_date_suffix: Option<String>,
    /// Total for `DATE` (§2.4): its completed stints, plus live minutes
    /// when `DATE` is today.
    pub day_total_minutes: i64,
    pub has_open_stint: bool,
    /// `Some` only when `DATE == today && has_open_stint` (§7.1).
    pub eod: Option<EodState>,
    /// The complete, pre-rendered §7.1 week line (label, headline, and
    /// the fulfillment/target parenthetical when applicable) — verbatim
    /// from `render::status_week_line` via the [`week_line`] wrapper. It
    /// always describes now, whichever `DATE` the page shows.
    pub week_line: String,
    /// §7.3 anomaly detail lines, already `[!] `-prefixed, verbatim from
    /// `render::DateAnomalies::detail_lines()`, plus any lookback lines
    /// appended for today (§7.1).
    pub anomaly_lines: Vec<String>,
    /// Stint rows, already sorted by start time ascending.
    pub stints: Vec<StintLine>,
    /// Note bodies, in insertion order, verbatim (already trimmed at
    /// storage, §2.3).
    pub notes: Vec<String>,
    /// §4's three-way open-stint age, stored unchanged from the same
    /// local `resolve()` computes for `build_stint_lines` — read
    /// directly by `day_total_line()` since it is a per-date fact, not
    /// per-stint. NOT derived from `stints` — it is set even when
    /// `has_open_stint` is false, in which case `day_total_line()`
    /// never reads it.
    pub open_stint_age: OpenStintAge,
}

// ---------------------------------------------------------------------------
// render: pure StatusView -> String
// ---------------------------------------------------------------------------

/// Left-aligned label column width shared with `render::LABEL_WIDTH`
/// (§7.1's "Day total:" / "Week NN:" lines line up).
const LABEL_WIDTH: usize = render::LABEL_WIDTH;

/// L3: the day-total line (plans/milestone-10-status-command.md §3.4).
/// Today's total includes live minutes and reads `(incl. ongoing)`. An
/// earlier date keeps `(+ ongoing)` or `(+ unclosed)` with no live credit.
fn day_total_line(view: &StatusView) -> String {
    let mut s = format!(
        "{:<w$}{}",
        "Day total:",
        format_minutes(view.day_total_minutes),
        w = LABEL_WIDTH
    );
    if view.has_open_stint {
        match view.open_stint_age {
            OpenStintAge::Today => s.push_str(" (incl. ongoing)"),
            OpenStintAge::Yesterday => s.push_str(" (+ ongoing)"),
            OpenStintAge::TwoOrMoreDaysBack => s.push_str(" (+ unclosed)"),
        }
    }
    match &view.eod {
        Some(EodState::At(t, day)) => {
            s.push_str(&format!(", est. EOD {}{}", t.format("%H:%M"), day.suffix()))
        }
        Some(EodState::TargetAlreadyMet) => s.push_str(", done for today"),
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
    match (line.end, line.open_stint_age) {
        (StintEnd::At(_), _) => {
            let suffix = if line.spans_to_next_day {
                ", spans to next day"
            } else {
                ""
            };
            format!(
                "  {:<11}  ({}{})",
                range,
                format_minutes(line.duration_minutes),
                suffix
            )
        }
        (StintEnd::Now, Some(OpenStintAge::Today)) | (StintEnd::Now, None) => {
            format!(
                "  {:<11}  ({}, ongoing)",
                range,
                format_minutes(line.duration_minutes)
            )
        }
        (StintEnd::Now, Some(OpenStintAge::Yesterday)) => {
            format!(
                "  {:<11}  ({}, ongoing - duration as of right now, not a running total)",
                range,
                format_minutes(line.duration_minutes)
            )
        }
        (StintEnd::Now, Some(OpenStintAge::TwoOrMoreDaysBack)) => {
            format!("  {:<11}  (unclosed)", range)
        }
    }
}

/// The complete rendered status view (plans/milestone-10-status-command.md
/// §3.1): pure, no I/O, no clock. Lines joined by `\n`, exactly one
/// trailing `\n`, no trailing blank line.
pub fn render(view: &StatusView) -> String {
    let mut header_line = view.header.clone();
    if let Some(suffix) = &view.receiving_date_suffix {
        header_line.push_str(suffix);
    }
    let mut lines: Vec<String> = vec![
        header_line,
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

/// The single call site for `render::status_week_line`. It takes the
/// whole `WeekAccounting` by reference so the fulfillment/target/owed
/// fields can never be transposed by a typo at a call site.
fn week_line(acct: &WeekAccounting, today: NaiveDate) -> String {
    render::status_week_line(acct, today)
}

/// Builds stint rows from `DayStints`, sorted by start time ascending
/// (plans/milestone-10-status-command.md §3.7). A stable sort preserves
/// each side's own relative order (completed by end instant, open by
/// start instant) for the rare tie at an identical start time.
fn build_stint_lines(day: &DayStints, open_stint_age: OpenStintAge) -> Vec<StintLine> {
    let mut rows: Vec<StintLine> = Vec::new();
    for s in &day.completed {
        rows.push(StintLine {
            start: local_time(s.start.at_utc),
            end: StintEnd::At(local_time(s.end.at_utc)),
            duration_minutes: s.minutes,
            spans_to_next_day: s.start.date != s.end.date,
            open_stint_age: None,
        });
    }
    for o in &day.open {
        rows.push(StintLine {
            start: local_time(o.start.at_utc),
            end: StintEnd::Now,
            duration_minutes: o.minutes_so_far,
            spans_to_next_day: false,
            open_stint_age: Some(open_stint_age),
        });
    }
    rows.sort_by_key(|r| r.start);
    rows
}

/// The est. EOD state for an open stint. `gap_minutes` is the live-inclusive
/// gap and decides the branch. `base_gap` is the same gap without live
/// (`required - fulfillment` before the running stint) and is added to the
/// open start, so the instant does not move while the stint runs.
fn est_eod(open: &stint::OpenStint, base_gap: i64, gap_minutes: i64, today: NaiveDate) -> EodState {
    if gap_minutes <= 0 {
        return EodState::TargetAlreadyMet;
    }
    let eod_datetime = open.start.at_utc.with_timezone(&Local) + ChronoDuration::minutes(base_gap);
    EodState::At(
        eod_datetime.time(),
        EodDay::classify(today, eod_datetime.date_naive()),
    )
}

/// The `est. EOD` state for today's open stint, if any. `acct` already
/// counts `live`.
fn today_eod(
    acct: &WeekAccounting,
    day: &DayStints,
    live: i64,
    today: NaiveDate,
) -> Option<EodState> {
    let (_, gap_minutes) = week::required_and_gap(acct, today);
    day.open
        .last()
        .map(|o| est_eod(o, gap_minutes + live, gap_minutes, today))
}

/// §4's three-way age of `target_date` relative to `today`.
fn open_stint_age_for(target_date: NaiveDate, today: NaiveDate) -> OpenStintAge {
    if target_date == today {
        OpenStintAge::Today
    } else if target_date == today - ChronoDuration::days(1) {
        OpenStintAge::Yesterday
    } else {
        OpenStintAge::TwoOrMoreDaysBack
    }
}

/// The header suffix naming the end punch that a splice consumed as
/// `target_date`'s first punch, when there is one.
fn receiving_suffix(
    ctx: &anomalies::DateContext,
    day_punches: &[storage::Punch],
) -> Option<String> {
    stint::splice_candidate(ctx.prev_raw.open.len(), &ctx.day_raw, day_punches).then(|| {
        let at = local_time(ctx.day_raw.orphaned_ends[0].punch.at_utc);
        format!("  ({} continues previous day's stint)", at.format("%H:%M"))
    })
}

/// The anomaly section, in placement order (§3.1, §3.3): the date's own
/// block, then today's lookback lines, then the carry marker line.
fn compose_anomaly_lines(
    facts: &DateAnomalies,
    lookback_lines: Vec<String>,
    marker: Option<&CarryMarker>,
) -> Vec<String> {
    let mut lines = facts.detail_lines();
    lines.extend(lookback_lines);
    lines.extend(marker.map(render::carry_marker_line));
    lines
}

/// Resolution flow (plans/milestone-10-status-command.md §2.2, exact
/// order): parses `date_arg` against `now`'s local date and rejects a
/// future date (§4.5), loads the history through `target_date`'s week,
/// classifies `target_date`'s punches, accounts `target_date`'s own week
/// (not today's, §3.5/NOTES decision 26) with today's live minutes in the
/// current week, and computes the daily-target/EOD hints only when
/// `target_date` is today.
pub fn resolve(
    date_arg: Option<&str>,
    now: DateTime<Local>,
    conn: &Connection,
) -> anyhow::Result<StatusView> {
    let today = now.date_naive();
    let target_date = match date_arg {
        None => today,
        Some(s) => date::resolve_future_checked_date(s, today)?,
    };
    let is_today = target_date == today;
    let open_stint_age = open_stint_age_for(target_date, today);

    let now_utc = now.with_timezone(&Utc);
    let target_week = WeekId::from_date(target_date);
    let notes = storage::notes_for_date(conn, target_date)?;
    let history = history::load_history(conn, target_week, today, now_utc)?;
    let buckets = history.buckets.trio(target_date);
    let ctx = anomalies::classify_context(&buckets, now_utc);

    let receiving_date_suffix = receiving_suffix(&ctx, buckets.day);
    let live = history.live_minutes;
    let day_total_minutes = ctx.day.completed_minutes() + if is_today { live } else { 0 };
    let has_open_stint = ctx.day.is_ongoing();

    let acct = history.accounting(target_week);
    let week_line_str = week_line(&acct, today);

    let facts = anomalies::build_facts(&ctx, &buckets, target_date, today, now_utc);
    let lookback_lines = if is_today {
        lookback::today_page_lines(
            conn,
            today,
            now_utc,
            facts.refused_join_with_previous.is_some(),
        )?
    } else {
        Vec::new()
    };
    let marker = carry_scan::carry_marker(&history, target_week, today, now_utc);
    let anomaly_lines = compose_anomaly_lines(&facts, lookback_lines, marker.as_ref());

    let eod = if is_today {
        today_eod(&acct, &ctx.day, live, today)
    } else {
        None
    };

    let stints = build_stint_lines(&ctx.day, open_stint_age);
    let note_bodies = notes.into_iter().map(|n| n.body).collect();

    Ok(StatusView {
        header: date::format_date_with_weekday(target_date),
        receiving_date_suffix,
        day_total_minutes,
        has_open_stint,
        eod,
        week_line: week_line_str,
        anomaly_lines,
        stints,
        notes: note_bodies,
        open_stint_age,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::storage::PunchKind;
    use crate::week_target;
    use crate::week_view;
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
            receiving_date_suffix: None,
            day_total_minutes: 0,
            has_open_stint: false,
            eod: None,
            week_line: week_line(&current_week_acct(645, 1755, 2400), today()),
            anomaly_lines: vec![],
            stints: vec![],
            notes: vec![],
            open_stint_age: OpenStintAge::Today,
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
        view.eod = Some(EodState::At(
            t(18, 0) + ChronoDuration::minutes(480),
            EodDay::Today,
        ));
        view.stints = vec![StintLine {
            start: t(9, 0),
            end: StintEnd::Now,
            duration_minutes: 0,
            spans_to_next_day: false,
            open_stint_age: None,
        }];
        let out = render(&view);
        assert!(out.contains("Day total:     00h 00m (incl. ongoing), est. EOD 02:00"));
        assert!(out.contains("  09:00-now    (00h 00m, ongoing)"));
        assert!(!out.contains("[!]"));
        assert!(!out.contains("Notes:"));
    }

    // T2 -- F2: one closed stint, no pace clause, no EOD line.
    #[test]
    fn t2_f2_ordinary_day_no_eod_line_without_open_stint() {
        let mut view = base_view();
        view.day_total_minutes = 510; // 08h 30m
        view.stints = vec![StintLine {
            start: t(9, 0),
            end: StintEnd::At(t(17, 30)),
            duration_minutes: 510,
            spans_to_next_day: false,
            open_stint_age: None,
        }];
        let out = render(&view);
        assert!(out.contains("Day total:     08h 30m\n"), "{out}");
        assert!(!out.contains("required"), "{out}");
        assert!(!out.contains("ongoing"));
        assert!(!out.contains("est. EOD"));
        assert!(!out.contains("done for today"));
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
                spans_to_next_day: false,
                open_stint_age: None,
            },
            StintLine {
                start: t(14, 0),
                end: StintEnd::At(t(18, 0)),
                duration_minutes: 240,
                spans_to_next_day: false,
                open_stint_age: None,
            },
        ];
        let out = render(&view);
        let idx1 = out.find("09:00-13:00").unwrap();
        let idx2 = out.find("14:00-18:00").unwrap();
        assert!(idx1 < idx2);
        assert!(!out.contains("[!]"));
    }

    // §2 -- a completed stint whose start.date != end.date gets ", spans
    // to next day" appended in the duration parenthetical.
    #[test]
    fn spanning_stint_gets_spans_to_next_day_suffix() {
        let mut view = base_view();
        view.stints = vec![StintLine {
            start: t(23, 30),
            end: StintEnd::At(t(0, 45)),
            duration_minutes: 75,
            spans_to_next_day: true,
            open_stint_age: None,
        }];
        let out = render(&view);
        assert!(
            out.contains("(01h 15m, spans to next day)"),
            "expected spans-to-next-day suffix: {out}"
        );
    }

    // Companion negative case: a same-date completed stint of the same
    // duration renders byte-identical to the pre-existing "(HH:MM)" form,
    // with no ", spans to next day" substring anywhere.
    #[test]
    fn non_spanning_stint_has_no_spans_to_next_day_suffix() {
        let mut view = base_view();
        view.stints = vec![StintLine {
            start: t(23, 30),
            end: StintEnd::At(t(0, 45)),
            duration_minutes: 75,
            spans_to_next_day: false,
            open_stint_age: None,
        }];
        let out = render(&view);
        assert!(!out.contains("spans to next day"), "{out}");
        assert!(out.contains("(01h 15m)"), "{out}");
    }

    // Companion negative case for open stints: build_stint_lines always
    // sets spans_to_next_day: false for opens, and stint_line() never
    // shows the suffix for StintEnd::Now regardless.
    #[test]
    fn open_stint_never_shows_spans_to_next_day_suffix_even_if_field_set() {
        let mut view = base_view();
        view.has_open_stint = true;
        view.stints = vec![StintLine {
            start: t(9, 0),
            end: StintEnd::Now,
            duration_minutes: 30,
            // Hypothetically true -- stint_line() only branches on
            // spans_to_next_day inside the StintEnd::At arm, so this must
            // never surface for an open stint.
            spans_to_next_day: true,
            open_stint_age: Some(OpenStintAge::Today),
        }];
        let out = render(&view);
        assert!(!out.contains("spans to next day"), "{out}");
    }

    // §2 -- build_stint_lines itself always sets spans_to_next_day: false
    // for every open row it builds (a build_stint_lines fact, distinct
    // from stint_line()'s rendering fact tested above).
    #[test]
    fn build_stint_lines_always_marks_open_rows_as_not_spanning() {
        let day = stint::classify(
            &[storage::Punch {
                id: 1,
                at_utc: today()
                    .and_hms_opt(9, 0, 0)
                    .unwrap()
                    .and_local_timezone(Local)
                    .unwrap()
                    .with_timezone(&Utc),
                date: today(),
                kind: PunchKind::Start,
            }],
            now_thu_1800().with_timezone(&Utc),
        );
        let rows = build_stint_lines(&day, OpenStintAge::Today);
        assert_eq!(rows.len(), 1);
        assert!(!rows[0].spans_to_next_day);
    }

    // §4 -- today's open stint shows `(incl. ongoing)` on the Day total
    // and keeps its plain stint line.
    #[test]
    fn open_stint_today_shows_incl_ongoing_and_plain_stint_line() {
        let mut view = base_view();
        view.has_open_stint = true;
        view.stints = vec![StintLine {
            start: t(9, 0),
            end: StintEnd::Now,
            duration_minutes: 0,
            spans_to_next_day: false,
            open_stint_age: Some(OpenStintAge::Today),
        }];
        let out = render(&view);
        assert!(out.contains("Day total:     00h 00m (incl. ongoing)"));
        assert!(out.contains("  09:00-now    (00h 00m, ongoing)"));
        assert!(!out.contains("unclosed"));
        assert!(!out.contains("duration as of right now"));
    }

    // §4 -- an open stint dated yesterday keeps its live duration and
    // gains a caption; Day total stays "(+ ongoing)".
    #[test]
    fn open_stint_yesterday_keeps_duration_and_gains_caption() {
        let mut view = base_view();
        view.has_open_stint = true;
        view.open_stint_age = OpenStintAge::Yesterday;
        view.stints = vec![StintLine {
            start: t(23, 10),
            end: StintEnd::Now,
            duration_minutes: 635, // 10h 35m, spec §4's own example
            spans_to_next_day: false,
            open_stint_age: Some(OpenStintAge::Yesterday),
        }];
        let out = render(&view);
        assert!(
            out.contains(
                "  23:10-now    (10h 35m, ongoing - duration as of right now, not a running total)"
            ),
            "{out}"
        );
        assert!(
            out.contains("Day total:     00h 00m (+ ongoing)"),
            "Day total unchanged for yesterday: {out}"
        );
    }

    // §4 -- an open stint two-or-more days back drops the duration figure
    // entirely and renders "(unclosed)"; Day total switches to "(+
    // unclosed)".
    #[test]
    fn open_stint_two_or_more_days_back_drops_duration_and_shows_unclosed() {
        let mut view = base_view();
        view.has_open_stint = true;
        view.open_stint_age = OpenStintAge::TwoOrMoreDaysBack;
        view.stints = vec![StintLine {
            start: t(9, 0),
            end: StintEnd::Now,
            duration_minutes: 4321, // implausible, would be very visible if leaked
            spans_to_next_day: false,
            open_stint_age: Some(OpenStintAge::TwoOrMoreDaysBack),
        }];
        let out = render(&view);
        assert!(out.contains("  09:00-now    (unclosed)"), "{out}");
        let stint_line_out = out
            .lines()
            .find(|l| l.contains("09:00-now"))
            .expect("stint line present");
        assert!(
            !stint_line_out.contains("43"),
            "no duration digits should leak: {stint_line_out}"
        );
        assert!(
            out.contains("Day total:     00h 00m (+ unclosed)"),
            "Day total switches to unclosed: {out}"
        );
        assert!(!out.contains("(+ ongoing)"));
    }

    // §4 -- boundary tightness: today - 2 days classifies as
    // TwoOrMoreDaysBack, not a fourth bucket.
    #[test]
    fn resolve_open_stint_exactly_two_days_back_is_two_or_more_days_back() {
        let conn = test_db();
        let two_days_back = today() - ChronoDuration::days(2);
        storage::insert_punch(&conn, PunchKind::Start, two_days_back, t(9, 0), &Local)
            .expect("insert");

        let view = resolve(
            Some(&two_days_back.format("%Y-%m-%d").to_string()),
            now_thu_1800(),
            &conn,
        )
        .expect("resolve");
        assert_eq!(view.open_stint_age, OpenStintAge::TwoOrMoreDaysBack);
        assert_eq!(
            view.stints[0].open_stint_age,
            Some(OpenStintAge::TwoOrMoreDaysBack)
        );
    }

    // §4 -- resolve()-level: all three age buckets, and the StintLine-level
    // field must agree with the StatusView-level field.
    #[test]
    fn resolve_open_stint_age_buckets_agree_between_view_and_stint_line() {
        let cases = [
            (today(), OpenStintAge::Today),
            (today() - ChronoDuration::days(1), OpenStintAge::Yesterday),
            (
                today() - ChronoDuration::days(3),
                OpenStintAge::TwoOrMoreDaysBack,
            ),
        ];
        for (date, expected) in cases {
            let conn2 = test_db();
            storage::insert_punch(&conn2, PunchKind::Start, date, t(9, 0), &Local).expect("insert");
            let view = resolve(
                Some(&date.format("%Y-%m-%d").to_string()),
                now_thu_1800(),
                &conn2,
            )
            .expect("resolve");
            assert_eq!(view.open_stint_age, expected, "date {date}");
            assert_eq!(view.stints[0].open_stint_age, Some(expected), "date {date}");
        }
    }

    // T6a -- F9 state 1: open stint, gap > 0.
    #[test]
    fn t6a_f9_open_stint_with_gap_shows_est_eod() {
        let mut view = base_view();
        view.day_total_minutes = 445; // 07h 25m
        view.has_open_stint = true;
        view.eod = Some(EodState::At(t(20, 30), EodDay::Today));
        let out = render(&view);
        assert!(out.contains("Day total:     07h 25m (incl. ongoing), est. EOD 20:30\n"));
        assert!(!out.contains("required"), "{out}");
    }

    // T6b -- F9 state 2: gap == 0 and gap < 0 both take the "met" branch.
    #[test]
    fn t6b_f9_open_stint_gap_zero_or_negative_is_target_already_met() {
        for gap in [0i64, -20i64] {
            let mut view = base_view();
            view.day_total_minutes = 480 - gap;
            view.has_open_stint = true;
            view.eod = Some(EodState::TargetAlreadyMet);
            let out = render(&view);
            assert!(
                out.contains("(incl. ongoing), done for today\n"),
                "gap {gap}: {out}"
            );
            assert!(!out.contains("est. EOD"), "gap {gap}: {out}");
            assert!(!out.contains("required"), "gap {gap}: {out}");
        }
    }

    // T6c -- F9 state 3: no open stint, EOD segment entirely absent.
    #[test]
    fn t6c_f9_no_open_stint_omits_eod_segment_entirely() {
        let mut view = base_view();
        view.day_total_minutes = 480;
        view.has_open_stint = false;
        view.eod = None;
        let out = render(&view);
        assert!(!out.contains("est. EOD"));
        assert!(!out.contains("done for today"));
        assert!(!out.contains("ongoing"));
        assert!(!out.contains("required"), "{out}");
        assert!(out.contains("Day total:     08h 00m\n"), "{out}");
    }

    // New -- Fix B: est. EOD carries a (tomorrow) suffix when the estimate
    // falls on a later calendar date than today.
    #[test]
    fn fix_b_eod_estimate_tomorrow_gets_suffix() {
        let mut view = base_view();
        view.day_total_minutes = 60;
        view.has_open_stint = true;
        view.eod = Some(EodState::At(t(1, 30), EodDay::Tomorrow));
        let out = render(&view);
        assert!(out.contains(", est. EOD 01:30 (tomorrow)"), "{out}");
    }

    // Companion negative case: same clock time, Today, no suffix --
    // confirms the two arms are actually distinct, not that "true" always
    // wins on any input.
    #[test]
    fn fix_b_eod_estimate_same_day_has_no_tomorrow_suffix() {
        let mut view = base_view();
        view.day_total_minutes = 60;
        view.has_open_stint = true;
        view.eod = Some(EodState::At(t(1, 30), EodDay::Today));
        let out = render(&view);
        assert!(out.contains(", est. EOD 01:30"), "{out}");
        assert!(!out.contains("(tomorrow)"), "{out}");
    }

    // Friday reaches the week cap, and the Day total line carries no
    // pace clause in either wording.
    #[test]
    fn fix_c_friday_has_no_pace_clause() {
        let mut view = base_view();
        view.day_total_minutes = 445;
        let out = render(&view);
        assert!(!out.contains("required"), "{out}");
        assert!(out.contains("Day total:     07h 25m\n"), "{out}");
    }

    // Saturday also reaches the week cap and also has no pace clause.
    #[test]
    fn fix_c_saturday_has_no_pace_clause() {
        let mut view = base_view();
        view.day_total_minutes = 0;
        let out = render(&view);
        assert!(!out.contains("required"), "{out}");
        assert!(out.contains("Day total:     00h 00m\n"), "{out}");
    }

    // T7 -- F10: past date, closed week, no daily-target/EOD lines.
    // Golden-matches SPEC.md §7.1's second worked example verbatim.
    #[test]
    fn t7_f10_golden_second_spec_example() {
        let view = StatusView {
            header: "Mon 2026-01-05".to_string(),
            day_total_minutes: 375,
            has_open_stint: false,
            eod: None,
            week_line: week_line(&closed_week_acct(wk(2026, 2), 100), today()),
            anomaly_lines: vec![],
            stints: vec![StintLine {
                start: t(8, 30),
                end: StintEnd::At(t(14, 45)),
                duration_minutes: 375,
                spans_to_next_day: false,
                open_stint_age: None,
            }],
            notes: vec![],
            receiving_date_suffix: None,
            open_stint_age: OpenStintAge::Today,
        };
        let out = render(&view);
        let expected = "Mon 2026-01-05\n\
             \n\
             Day total:     06h 15m\n\
             Week 2026-02:  Total behind: 01h 40m\n\
             \n\
             \x20\x2008:30-14:45  (06h 15m)\n";
        assert_eq!(out, expected);
        assert!(!out.contains("daily target"));
        assert!(!out.contains("required by end of"));
        assert!(!out.contains("est. EOD"));
        assert!(!out.contains("done for today"));
        assert!(out.contains("Week 2026-02:"));
        assert!(!out.contains("fulfillment"));
    }

    // T8 -- F11: different day within the current week keeps deadline
    // framing keyed to today's weekday.
    #[test]
    fn t8_f11_different_day_in_current_week_has_no_eod_but_deadline_week_line() {
        let mut view = base_view();
        view.header = "Mon 2026-02-09".to_string();
        // eod stays None: DATE isn't today.
        let out = render(&view);
        assert!(!out.contains("Monday"));
        assert!(out.contains("left to 32h 00m by end of Thursday"));
        assert!(!out.contains("daily target"));
        assert!(!out.contains("required by end of"));
        assert!(!out.contains("est. EOD"));
        assert!(!out.contains("done for today"));
    }

    // T9 -- E7: three unmatched starts.
    #[test]
    fn t9_e7_multi_open_anomaly_rendering() {
        let mut view = base_view();
        view.has_open_stint = true;
        view.anomaly_lines =
            vec!["[!] 3 open stints for this date (only one can be open at a time)".to_string()];
        view.stints = vec![
            StintLine {
                start: t(9, 0),
                end: StintEnd::Now,
                duration_minutes: 540,
                spans_to_next_day: false,
                open_stint_age: None,
            },
            StintLine {
                start: t(10, 0),
                end: StintEnd::Now,
                duration_minutes: 480,
                spans_to_next_day: false,
                open_stint_age: None,
            },
            StintLine {
                start: t(11, 0),
                end: StintEnd::Now,
                duration_minutes: 420,
                spans_to_next_day: false,
                open_stint_age: None,
            },
        ];
        let out = render(&view);
        assert_eq!(
            out.matches("[!]").count(),
            1,
            "exactly one anomaly line: {out}"
        );
        assert!(out.contains("[!] 3 open stints for this date (only one can be open at a time)"));
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
            spans_to_next_day: false,
            open_stint_age: None,
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
        let acct = current_week_acct(630, 1770, 2400);
        let view = StatusView {
            header: "Thu 2026-02-12".to_string(),
            day_total_minutes: 460,
            has_open_stint: true,
            eod: Some(EodState::At(t(20, 30), EodDay::Today)),
            week_line: week_line(&acct, today()),
            anomaly_lines: vec![],
            stints: vec![
                StintLine {
                    start: t(9, 0),
                    end: StintEnd::At(t(13, 0)),
                    duration_minutes: 240,
                    spans_to_next_day: false,
                    open_stint_age: None,
                },
                StintLine {
                    start: t(14, 5),
                    end: StintEnd::At(t(17, 30)),
                    duration_minutes: 205,
                    spans_to_next_day: false,
                    open_stint_age: None,
                },
                StintLine {
                    start: t(17, 45),
                    end: StintEnd::Now,
                    duration_minutes: 15,
                    spans_to_next_day: false,
                    open_stint_age: None,
                },
            ],
            notes: vec![
                "fixed migration runner bug".to_string(),
                "started punch pairing tests".to_string(),
            ],
            receiving_date_suffix: None,
            open_stint_age: OpenStintAge::Today,
        };
        let out = render(&view);
        let expected = "Thu 2026-02-12\n\
            \n\
            Day total:     07h 40m (incl. ongoing), est. EOD 20:30\n\
            Week 2026-07:  02h 30m left to 32h 00m by end of Thursday (fulfillment 29h 30m / target 40h 00m)\n\
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
            eod: Some(EodState::At(t(20, 30), EodDay::Today)),
            week_line: week_line(&acct, today()),
            anomaly_lines: vec![],
            stints: vec![StintLine {
                start: t(9, 0),
                end: StintEnd::At(t(13, 0)),
                duration_minutes: 240,
                spans_to_next_day: false,
                open_stint_age: None,
            }],
            notes: vec![],
            receiving_date_suffix: None,
            open_stint_age: OpenStintAge::Today,
        });
        let t7 = render(&StatusView {
            header: "Mon 2026-01-05".to_string(),
            day_total_minutes: 375,
            has_open_stint: false,
            eod: None,
            week_line: week_line(&closed_week_acct(wk(2026, 2), 100), today()),
            anomaly_lines: vec![],
            stints: vec![StintLine {
                start: t(8, 30),
                end: StintEnd::At(t(14, 45)),
                duration_minutes: 375,
                spans_to_next_day: false,
                open_stint_age: None,
            }],
            notes: vec![],
            receiving_date_suffix: None,
            open_stint_age: OpenStintAge::Today,
        });
        let mut t6b_view = base_view();
        t6b_view.day_total_minutes = 500;
        t6b_view.has_open_stint = true;
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
            eod: Some(EodState::At(t(20, 30), EodDay::Today)),
            week_line: week_line(&acct, today()),
            anomaly_lines: vec!["[!] orphaned end at 18:00 (no matching start)".to_string()],
            stints: vec![StintLine {
                start: t(9, 0),
                end: StintEnd::At(t(13, 0)),
                duration_minutes: 240,
                spans_to_next_day: false,
                open_stint_age: None,
            }],
            notes: vec!["ascii only note".to_string()],
            receiving_date_suffix: None,
            open_stint_age: OpenStintAge::Today,
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

    // Extends T13's plain-ASCII audit to the five new literals this task
    // adds inside status.rs: ", spans to next day"; ", ongoing - duration
    // as of right now, not a running total"; "(unclosed)"; "continues
    // previous day's stint" (via a non-None receiving_date_suffix); "(+
    // unclosed)".
    #[test]
    fn t13b_new_boundary_context_cue_literals_are_plain_ascii() {
        let mut spanning_view = base_view();
        spanning_view.stints = vec![StintLine {
            start: t(23, 30),
            end: StintEnd::At(t(0, 45)),
            duration_minutes: 75,
            spans_to_next_day: true,
            open_stint_age: None,
        }];
        let out = render(&spanning_view);
        assert!(out.contains(", spans to next day"));
        assert!(
            out.bytes()
                .all(|b| b == b'\n' || (0x20..=0x7E).contains(&b))
        );

        let mut yesterday_view = base_view();
        yesterday_view.has_open_stint = true;
        yesterday_view.open_stint_age = OpenStintAge::Yesterday;
        yesterday_view.stints = vec![StintLine {
            start: t(23, 10),
            end: StintEnd::Now,
            duration_minutes: 635,
            spans_to_next_day: false,
            open_stint_age: Some(OpenStintAge::Yesterday),
        }];
        let out = render(&yesterday_view);
        assert!(out.contains(", ongoing - duration as of right now, not a running total"));
        assert!(
            out.bytes()
                .all(|b| b == b'\n' || (0x20..=0x7E).contains(&b))
        );

        let mut stale_view = base_view();
        stale_view.has_open_stint = true;
        stale_view.open_stint_age = OpenStintAge::TwoOrMoreDaysBack;
        stale_view.stints = vec![StintLine {
            start: t(9, 0),
            end: StintEnd::Now,
            duration_minutes: 4321,
            spans_to_next_day: false,
            open_stint_age: Some(OpenStintAge::TwoOrMoreDaysBack),
        }];
        let out = render(&stale_view);
        assert!(out.contains("(unclosed)"));
        assert!(out.contains("(+ unclosed)"));
        assert!(
            out.bytes()
                .all(|b| b == b'\n' || (0x20..=0x7E).contains(&b))
        );

        let mut spliced_view = base_view();
        spliced_view.receiving_date_suffix =
            Some("  (00:45 continues previous day's stint)".to_string());
        let out = render(&spliced_view);
        assert!(out.contains("continues previous day's stint"));
        assert!(
            out.bytes()
                .all(|b| b == b'\n' || (0x20..=0x7E).contains(&b))
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
        assert!(view.eod.is_none());
        assert!(
            view.week_line
                .contains("24h 00m left to 32h 00m by end of Thursday"),
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

    // Current week whose fulfillment already exceeds `required`: the week
    // line reads the `over` form against today's slice, never "ahead of
    // target".
    #[test]
    fn resolve_current_week_over_required_renders_over_form() {
        let conn = test_db();
        week_target::set_week_target(&conn, &wk(2026, 7), 5).expect("set target"); // tiny target
        storage::insert_punch(&conn, PunchKind::Start, d(2026, 2, 9), t(8, 0), &Local)
            .expect("insert");
        storage::insert_punch(&conn, PunchKind::End, d(2026, 2, 9), t(17, 0), &Local)
            .expect("insert");

        let view = resolve(None, now_thu_1800(), &conn).expect("resolve");
        // required = floor(5 / 5) * 4 = 4 minutes; fulfillment 540.
        assert!(
            view.week_line
                .contains("08h 56m over 00h 04m by end of Thursday (fulfillment "),
            "week_line: {}",
            view.week_line
        );
        let out = render(&view);
        assert!(!out.contains("ahead of target"), "{out}");
    }

    #[test]
    fn resolve_e7_multi_open_and_e8_orphaned_end_via_anomalies_only() {
        let conn = test_db();
        storage::insert_punch(&conn, PunchKind::Start, today(), t(9, 0), &Local).expect("insert");
        storage::insert_punch(&conn, PunchKind::Start, today(), t(10, 0), &Local).expect("insert");
        storage::insert_punch(&conn, PunchKind::End, today(), t(8, 0), &Local).expect("insert");

        let view = resolve(None, now_thu_1800(), &conn).expect("resolve");
        assert_eq!(view.anomaly_lines.len(), 6, "{:?}", view.anomaly_lines);
        assert!(view.anomaly_lines[0].contains("2 open stints"));
        assert!(view.anomaly_lines[3].contains("orphaned end at 08:00"));
        assert_eq!(view.stints.len(), 2, "the orphan produces no stint line");
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

    // §4.5: a future DATE is rejected exactly like start/stop/note's.
    #[test]
    fn resolve_future_date_is_rejected() {
        let conn = test_db();
        // now_thu_1800() is 2026-02-12.
        for future in ["2026-03-01", "2026-02-13"] {
            let err = resolve(Some(future), now_thu_1800(), &conn).expect_err(future);
            assert_eq!(
                err.to_string(),
                format!("invalid DATE \"{future}\": date is in the future")
            );
        }
    }

    #[test]
    fn resolve_todays_date_string_is_accepted() {
        let conn = test_db();
        let view = resolve(Some("2026-02-12"), now_thu_1800(), &conn).expect("resolve");
        assert_eq!(view.header, date::format_date_with_weekday(today()));
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
    // `resolve` and `history::load_history` bucket neighbor-date punches
    // and call `stint::classify_at` instead of `stint::classify`.
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
        assert!(
            view.stints[0].spans_to_next_day,
            "23:30-00:45 crosses a calendar date"
        );
        assert!(render(&view).contains(", spans to next day"));
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
        assert_eq!(
            view.receiving_date_suffix,
            Some("  (00:45 continues previous day's stint)".to_string())
        );
        let out = render(&view);
        assert!(out.contains(&format!(
            "{}  (00:45 continues previous day's stint)",
            date::format_date_with_weekday(d2)
        )));
    }

    // §3 -- negative case: no splice consumed this date's first punch.
    #[test]
    fn resolve_no_splice_has_no_receiving_date_suffix() {
        let conn = test_db();
        storage::insert_punch(&conn, PunchKind::Start, today(), t(9, 0), &Local).expect("insert");
        storage::insert_punch(&conn, PunchKind::Start, today(), t(10, 0), &Local).expect("insert");
        storage::insert_punch(&conn, PunchKind::End, today(), t(8, 0), &Local).expect("insert");

        let view = resolve(None, now_thu_1800(), &conn).expect("resolve");
        assert!(view.receiving_date_suffix.is_none());
    }

    // Confirms this call site doesn't accidentally always splice -- only
    // when the shape is the exact 1:1, first-punch match. `D` has two
    // trailing opens (E7, not 1:1), so `D+1`'s single `End 00:30` cannot
    // close either of them and stays a genuine orphaned end. Since the
    // pair isn't 1:1, it's also a refused join: both dates carry their
    // own headline plus a line naming the other.
    #[test]
    fn resolve_midnight_boundary_still_flags_non_1to1_shapes() {
        let conn = test_db();
        let d1 = d(2026, 2, 9);
        let d2 = d(2026, 2, 10);
        storage::insert_punch(&conn, PunchKind::Start, d1, t(22, 0), &Local).expect("insert");
        storage::insert_punch(&conn, PunchKind::Start, d1, t(23, 0), &Local).expect("insert");
        storage::insert_punch(&conn, PunchKind::End, d2, t(0, 30), &Local).expect("insert");

        let view_d1 = resolve(Some("2026-02-09"), now_thu_1800(), &conn).expect("resolve");
        assert_eq!(
            view_d1.anomaly_lines.len(),
            4,
            "{:?}",
            view_d1.anomaly_lines
        );
        assert!(view_d1.anomaly_lines[0].contains("2 open stints"));
        assert!(
            view_d1
                .anomaly_lines
                .iter()
                .any(|l| l.contains("not joined to"))
        );
        assert_eq!(view_d1.day_total_minutes, 0);

        let view_d2 = resolve(Some("2026-02-10"), now_thu_1800(), &conn).expect("resolve");
        assert_eq!(view_d2.day_total_minutes, 0);
        assert_eq!(
            view_d2.anomaly_lines.len(),
            4,
            "{:?}",
            view_d2.anomaly_lines
        );
        assert!(view_d2.anomaly_lines[0].contains("orphaned end at 00:30"));
        assert!(
            view_d2
                .anomaly_lines
                .iter()
                .any(|l| l.contains("not joined to"))
        );
        assert!(
            view_d2.receiving_date_suffix.is_none(),
            "prev has 2 opens (not 1:1), so the fresh classify(prev) gate blocks the suffix"
        );
    }

    // §3 -- the consumed punch was target_date's only punch: the stint
    // list is entirely omitted (SPEC.md §7.1), but the header suffix,
    // Day total, and week line still render.
    #[test]
    fn resolve_midnight_splice_only_punch_of_the_day_still_shows_header_suffix() {
        let conn = test_db();
        let d1 = d(2026, 2, 9);
        let d2 = d(2026, 2, 10);
        storage::insert_punch(&conn, PunchKind::Start, d1, t(23, 30), &Local).expect("insert");
        storage::insert_punch(&conn, PunchKind::End, d2, t(0, 45), &Local).expect("insert");

        let view = resolve(Some("2026-02-10"), now_thu_1800(), &conn).expect("resolve");
        assert!(view.stints.is_empty());
        assert_eq!(
            view.receiving_date_suffix,
            Some("  (00:45 continues previous day's stint)".to_string())
        );
        let expected_week = current_week_acct(2400 - 75, 75, 2400);
        assert_eq!(view.week_line, week_line(&expected_week, today()));
        assert_eq!(view.day_total_minutes, 0);
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

    // Fix B/resolve()-level: an open stint whose gap is already <= 0 takes
    // the EodState::TargetAlreadyMet arm through resolve() itself, not
    // just through the render()-level t6b fixture.
    #[test]
    fn resolve_eod_target_already_met_with_open_stint() {
        let conn = test_db();
        week_target::set_week_target(&conn, &wk(2026, 7), 5).expect("set target"); // tiny target
        storage::insert_punch(&conn, PunchKind::Start, d(2026, 2, 9), t(9, 0), &Local)
            .expect("insert");
        storage::insert_punch(&conn, PunchKind::End, d(2026, 2, 9), t(9, 10), &Local)
            .expect("insert");
        storage::insert_punch(&conn, PunchKind::Start, today(), t(9, 0), &Local).expect("insert");

        let view = resolve(None, now_thu_1800(), &conn).expect("resolve");
        assert!(view.has_open_stint);
        assert!(view.week_line.contains(" over "), "{}", view.week_line);
        assert_eq!(view.eod, Some(EodState::TargetAlreadyMet));
        let out = render(&view);
        assert!(out.contains(", done for today"), "{out}");
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
        let out = render(&view);
        assert!(
            out.contains("Day total:     00h 00m\n"),
            "day total line: {out}"
        );
        assert!(
            out.contains("25h 20m over 08h 00m by end of Monday"),
            "{out}"
        );
        assert!(!out.contains("required"), "{out}");
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
        assert!(
            view.week_line.contains("left to 40h 00m target"),
            "{}",
            view.week_line
        );
    }

    #[test]
    fn resolve_f9b_sunday_required_is_full_non_multiple_of_five_target() {
        let conn = test_db();
        week_target::set_week_target(&conn, &wk(2026, 7), 2011).expect("set target"); // 33h 31m
        let sunday_1200 = Local
            .from_local_datetime(&d(2026, 2, 15).and_hms_opt(12, 0, 0).unwrap())
            .unwrap();
        let view = resolve(None, sunday_1200, &conn).expect("resolve");
        assert_eq!(view.header, "Sun 2026-02-15");
        let out = render(&view);
        assert!(!out.contains("required"), "{out}");
        assert!(
            view.week_line
                .contains("33h 31m left to 33h 31m target (fulfillment"),
            "{}",
            view.week_line
        );
    }

    // New -- Fix B, resolve()-level: an open start plus
    // `required - fulfillment` that lands past midnight sets
    // EodState::At's day to Tomorrow and day_total_line renders the
    // (tomorrow) suffix.
    #[test]
    fn resolve_fix_b_eod_estimate_crossing_midnight_is_marked_tomorrow() {
        let conn = test_db();
        // No punches at all today: day_total_minutes stays 0, has_open_stint
        // is false by default -- so first give the day an open stint to
        // satisfy the `has_open_stint` precondition for `eod` to be `Some`.
        storage::insert_punch(&conn, PunchKind::Start, today(), t(9, 0), &Local).expect("insert");
        let view = resolve(None, now_thu_1800(), &conn).expect("resolve");
        assert!(
            matches!(view.eod, Some(EodState::At(_, EodDay::Tomorrow))),
            "expected a tomorrow marker: {:?}",
            view.eod
        );
        let out = render(&view);
        assert!(out.contains("(tomorrow)"), "{out}");
    }

    // -----------------------------------------------------------------
    // §5: carry-in wiring, end-to-end through resolve()/render().
    // -----------------------------------------------------------------

    #[test]
    fn resolve_end_to_end_nonzero_carry_in_shows_worked_and_carry_in_breakdown() {
        let conn = test_db();
        // Week 2026-06 (Mon 2026-02-02 .. Sun 2026-02-08): target 8h,
        // worked 10h -- a 2h surplus carries into week 2026-07 as
        // carry_in.
        week_target::set_week_target(&conn, &wk(2026, 6), 480).expect("set target");
        storage::insert_punch(&conn, PunchKind::Start, d(2026, 2, 2), t(8, 0), &Local)
            .expect("insert");
        storage::insert_punch(&conn, PunchKind::End, d(2026, 2, 2), t(18, 0), &Local)
            .expect("insert");

        let view = resolve(None, now_thu_1800(), &conn).expect("resolve");
        let out = render(&view);
        assert!(
            out.contains(
                "(fulfillment 02h 00m = worked 00h 00m + carry-in 02h 00m / target 40h 00m)"
            ),
            "carry-in breakdown should survive load_history -> week_accounting -> \
             week_line -> status_week_line: {out}"
        );
    }

    // -----------------------------------------------------------------
    // week_line helper sanity: confirms the wrapper never transposes.
    // -----------------------------------------------------------------

    #[test]
    fn week_line_matches_status_week_line_field_for_field() {
        let acct = current_week_acct(645, 1755, 2400);
        let want = render::status_week_line(&acct, today());
        assert_eq!(week_line(&acct, today()), want);
    }

    // -----------------------------------------------------------------
    // Stale open start, refused join, and the shared 14-day lookback
    // (§4.3, §4.3.1, §7.1, §7.3).
    // -----------------------------------------------------------------

    // §4.3 -- a lone open start at age >= 2 gets the new stale-open-start
    // anomaly line, with the right time and age.
    #[test]
    fn resolve_stale_open_start_age_5_shows_stale_line_with_time_and_age() {
        let conn = test_db();
        let target = today() - ChronoDuration::days(5);
        storage::insert_punch(&conn, PunchKind::Start, target, t(9, 0), &Local).expect("insert");

        let view = resolve(
            Some(&target.format("%Y-%m-%d").to_string()),
            now_thu_1800(),
            &conn,
        )
        .expect("resolve");
        assert_eq!(view.anomaly_lines.len(), 3, "{:?}", view.anomaly_lines);
        assert_eq!(
            view.anomaly_lines[0],
            "[!] unclosed start at 09:00, 5 days old (no stop recorded)"
        );
        assert!(view.anomaly_lines[1].contains(&target.format("%Y-%m-%d").to_string()));
    }

    // §4.3 -- age 0 and age 1 stay non-anomalies.
    #[test]
    fn resolve_lone_open_start_age_zero_or_one_is_not_an_anomaly() {
        for age in [0i64, 1i64] {
            let conn = test_db();
            let target = today() - ChronoDuration::days(age);
            storage::insert_punch(&conn, PunchKind::Start, target, t(9, 0), &Local)
                .expect("insert");
            let view = resolve(
                Some(&target.format("%Y-%m-%d").to_string()),
                now_thu_1800(),
                &conn,
            )
            .expect("resolve");
            assert!(
                view.anomaly_lines.is_empty(),
                "age {age}: {:?}",
                view.anomaly_lines
            );
        }
    }

    // §4.3 -- ages 2, 14, and 15 are all stale (age doesn't stop being
    // stale past the 14-day lookback window -- that cutoff belongs to
    // the lookback scan, not this predicate).
    #[test]
    fn resolve_stale_boundary_ages_two_fourteen_fifteen_all_stale() {
        for age in [2i64, 14, 15] {
            let conn = test_db();
            let target = today() - ChronoDuration::days(age);
            storage::insert_punch(&conn, PunchKind::Start, target, t(9, 0), &Local)
                .expect("insert");
            let view = resolve(
                Some(&target.format("%Y-%m-%d").to_string()),
                now_thu_1800(),
                &conn,
            )
            .expect("resolve");
            assert!(
                view.anomaly_lines
                    .iter()
                    .any(|l| l.contains("unclosed start at 09:00")
                        && l.contains(&format!("{age} days old"))),
                "age {age}: {:?}",
                view.anomaly_lines
            );
        }
    }

    // §4.3 -- two or more open starts on a date show only the E7 line;
    // no stale line, even though one of the opens individually qualifies
    // by age.
    #[test]
    fn resolve_two_open_starts_shows_only_e7_no_stale_line() {
        let conn = test_db();
        let target = today() - ChronoDuration::days(5);
        storage::insert_punch(&conn, PunchKind::Start, target, t(9, 0), &Local).expect("insert");
        storage::insert_punch(&conn, PunchKind::Start, target, t(11, 0), &Local).expect("insert");

        let view = resolve(
            Some(&target.format("%Y-%m-%d").to_string()),
            now_thu_1800(),
            &conn,
        )
        .expect("resolve");
        assert_eq!(view.anomaly_lines.len(), 3, "{:?}", view.anomaly_lines);
        assert!(view.anomaly_lines[0].contains("2 open stints for this date"));
        assert!(!view.anomaly_lines.iter().any(|l| l.contains("days old")));
    }

    // §4.3.1 -- refused join shape 1: two opens on the earlier date. Both
    // dates' status pages name each other.
    #[test]
    fn resolve_refused_join_two_opens_on_earlier_date_flags_both_dates() {
        let conn = test_db();
        let d1 = today() - ChronoDuration::days(6);
        let d2 = d1.succ_opt().unwrap();
        storage::insert_punch(&conn, PunchKind::Start, d1, t(9, 0), &Local).expect("insert");
        storage::insert_punch(&conn, PunchKind::Start, d1, t(11, 0), &Local).expect("insert");
        storage::insert_punch(&conn, PunchKind::End, d2, t(0, 45), &Local).expect("insert");

        let view_d1 = resolve(
            Some(&d1.format("%Y-%m-%d").to_string()),
            now_thu_1800(),
            &conn,
        )
        .expect("resolve");
        assert!(
            view_d1
                .anomaly_lines
                .iter()
                .any(|l| l.contains("not joined to")),
            "{:?}",
            view_d1.anomaly_lines
        );

        let view_d2 = resolve(
            Some(&d2.format("%Y-%m-%d").to_string()),
            now_thu_1800(),
            &conn,
        )
        .expect("resolve");
        assert!(
            view_d2
                .anomaly_lines
                .iter()
                .any(|l| l.contains("not joined to")),
            "{:?}",
            view_d2.anomaly_lines
        );

        // F4: pin the full line, not just a substring, on both dates --
        // each must name the OTHER date as its neighbor, not its own.
        let expected_on_d1 = format!(
            "[!] not joined to {} across midnight (a join needs exactly one open start on {} and exactly one unmatched end, as {}'s first punch)",
            date::format_date_with_weekday(d2),
            date::format_date_with_weekday(d1),
            date::format_date_with_weekday(d2),
        );
        assert!(
            view_d1.anomaly_lines.contains(&expected_on_d1),
            "{:?}",
            view_d1.anomaly_lines
        );
        let expected_on_d2 = format!(
            "[!] not joined to {} across midnight (a join needs exactly one open start on {} and exactly one unmatched end, as {}'s first punch)",
            date::format_date_with_weekday(d1),
            date::format_date_with_weekday(d1),
            date::format_date_with_weekday(d2),
        );
        assert!(
            view_d2.anomaly_lines.contains(&expected_on_d2),
            "{:?}",
            view_d2.anomaly_lines
        );
    }

    // §4.3.1 -- refused join shape 2: the orphan isn't the later date's
    // chronologically first punch.
    #[test]
    fn resolve_refused_join_orphan_not_first_on_later_date_flags_both_dates() {
        let conn = test_db();
        let d1 = today() - ChronoDuration::days(6);
        let d2 = d1.succ_opt().unwrap();
        storage::insert_punch(&conn, PunchKind::Start, d1, t(23, 0), &Local).expect("insert");
        storage::insert_punch(&conn, PunchKind::Start, d2, t(9, 0), &Local).expect("insert");
        storage::insert_punch(&conn, PunchKind::End, d2, t(10, 0), &Local).expect("insert");
        storage::insert_punch(&conn, PunchKind::End, d2, t(13, 0), &Local).expect("insert");

        let view_d1 = resolve(
            Some(&d1.format("%Y-%m-%d").to_string()),
            now_thu_1800(),
            &conn,
        )
        .expect("resolve");
        assert!(
            view_d1
                .anomaly_lines
                .iter()
                .any(|l| l.contains("not joined to"))
        );

        let view_d2 = resolve(
            Some(&d2.format("%Y-%m-%d").to_string()),
            now_thu_1800(),
            &conn,
        )
        .expect("resolve");
        assert!(
            view_d2
                .anomaly_lines
                .iter()
                .any(|l| l.contains("not joined to"))
        );
    }

    // §4.3.1 -- refused join shape 3: two orphans on the later date.
    #[test]
    fn resolve_refused_join_two_orphans_on_later_date_flags_both_dates() {
        let conn = test_db();
        let d1 = today() - ChronoDuration::days(6);
        let d2 = d1.succ_opt().unwrap();
        storage::insert_punch(&conn, PunchKind::Start, d1, t(23, 0), &Local).expect("insert");
        storage::insert_punch(&conn, PunchKind::End, d2, t(0, 45), &Local).expect("insert");
        storage::insert_punch(&conn, PunchKind::End, d2, t(1, 0), &Local).expect("insert");

        let view_d1 = resolve(
            Some(&d1.format("%Y-%m-%d").to_string()),
            now_thu_1800(),
            &conn,
        )
        .expect("resolve");
        assert!(
            view_d1
                .anomaly_lines
                .iter()
                .any(|l| l.contains("not joined to"))
        );

        let view_d2 = resolve(
            Some(&d2.format("%Y-%m-%d").to_string()),
            now_thu_1800(),
            &conn,
        )
        .expect("resolve");
        assert!(
            view_d2
                .anomaly_lines
                .iter()
                .any(|l| l.contains("not joined to"))
        );
    }

    // §4.3.1 -- regression: the clean-splice shape produces no refused
    // join line on either date.
    #[test]
    fn resolve_clean_splice_produces_no_refused_join_line() {
        let conn = test_db();
        let d1 = today() - ChronoDuration::days(6);
        let d2 = d1.succ_opt().unwrap();
        storage::insert_punch(&conn, PunchKind::Start, d1, t(23, 30), &Local).expect("insert");
        storage::insert_punch(&conn, PunchKind::End, d2, t(0, 45), &Local).expect("insert");

        let view_d1 = resolve(
            Some(&d1.format("%Y-%m-%d").to_string()),
            now_thu_1800(),
            &conn,
        )
        .expect("resolve");
        assert!(
            !view_d1
                .anomaly_lines
                .iter()
                .any(|l| l.contains("not joined to"))
        );

        let view_d2 = resolve(
            Some(&d2.format("%Y-%m-%d").to_string()),
            now_thu_1800(),
            &conn,
        )
        .expect("resolve");
        assert!(
            !view_d2
                .anomaly_lines
                .iter()
                .any(|l| l.contains("not joined to"))
        );
    }

    // §4.3.1 -- a refused join between yesterday and today shows on
    // today's status.
    #[test]
    fn resolve_refused_join_at_yesterday_today_boundary_shows_on_todays_status() {
        let conn = test_db();
        let yesterday = today() - ChronoDuration::days(1);
        storage::insert_punch(&conn, PunchKind::Start, yesterday, t(23, 0), &Local)
            .expect("insert");
        storage::insert_punch(&conn, PunchKind::Start, yesterday, t(23, 30), &Local)
            .expect("insert");
        storage::insert_punch(&conn, PunchKind::End, today(), t(0, 15), &Local).expect("insert");

        let view = resolve(None, now_thu_1800(), &conn).expect("resolve");
        assert!(
            view.anomaly_lines
                .iter()
                .any(|l| l.contains("not joined to")),
            "{:?}",
            view.anomaly_lines
        );
    }

    // §7.3 -- ordering: E7 lines precede the refused-join-with-next
    // line.
    #[test]
    fn resolve_ordering_e7_then_refused_join_with_next() {
        let conn = test_db();
        let d1 = today() - ChronoDuration::days(6);
        let d2 = d1.succ_opt().unwrap();
        storage::insert_punch(&conn, PunchKind::Start, d1, t(9, 0), &Local).expect("insert");
        storage::insert_punch(&conn, PunchKind::Start, d1, t(11, 0), &Local).expect("insert");
        storage::insert_punch(&conn, PunchKind::End, d2, t(0, 45), &Local).expect("insert");

        let view = resolve(
            Some(&d1.format("%Y-%m-%d").to_string()),
            now_thu_1800(),
            &conn,
        )
        .expect("resolve");
        let idx_e7 = view
            .anomaly_lines
            .iter()
            .position(|l| l.contains("open stints for this date"))
            .expect("e7 line");
        let idx_refused = view
            .anomaly_lines
            .iter()
            .position(|l| l.contains("not joined to"))
            .expect("refused line");
        assert!(idx_e7 < idx_refused, "{:?}", view.anomaly_lines);
    }

    // §7.3 -- ordering: stale, then E8 (ascending instant), then the
    // refused-join-with-next line.
    #[test]
    fn resolve_ordering_stale_then_e8_then_refused_join_with_next() {
        let conn = test_db();
        let d1 = today() - ChronoDuration::days(6);
        let d2 = d1.succ_opt().unwrap();
        // d1: one stale open start (age 6) plus an unrelated orphaned end.
        storage::insert_punch(&conn, PunchKind::Start, d1, t(9, 0), &Local).expect("insert");
        storage::insert_punch(&conn, PunchKind::End, d1, t(8, 0), &Local).expect("insert");
        // d1/d2 boundary: two orphans on d2 refuse the join.
        storage::insert_punch(&conn, PunchKind::End, d2, t(0, 45), &Local).expect("insert");
        storage::insert_punch(&conn, PunchKind::End, d2, t(1, 0), &Local).expect("insert");

        let view = resolve(
            Some(&d1.format("%Y-%m-%d").to_string()),
            now_thu_1800(),
            &conn,
        )
        .expect("resolve");
        let idx_stale = view
            .anomaly_lines
            .iter()
            .position(|l| l.contains("days old"))
            .expect("stale line");
        let idx_e8 = view
            .anomaly_lines
            .iter()
            .position(|l| l.contains("orphaned end at 08:00"))
            .expect("e8 line");
        let idx_refused = view
            .anomaly_lines
            .iter()
            .position(|l| l.contains("not joined to"))
            .expect("refused line");
        assert!(
            idx_stale < idx_e8 && idx_e8 < idx_refused,
            "{:?}",
            view.anomaly_lines
        );
    }

    // §7.1 -- today's status lists stale starts from the lookback
    // window, oldest first.
    #[test]
    fn resolve_lookback_lists_stale_starts_oldest_first() {
        let conn = test_db();
        for age in [3i64, 7, 14] {
            let d = today() - ChronoDuration::days(age);
            storage::insert_punch(&conn, PunchKind::Start, d, t(9, 0), &Local).expect("insert");
        }
        let view = resolve(None, now_thu_1800(), &conn).expect("resolve");
        let idx14 = view
            .anomaly_lines
            .iter()
            .position(|l| l.contains("14 days ago"))
            .expect("age 14 line");
        let idx7 = view
            .anomaly_lines
            .iter()
            .position(|l| l.contains("7 days ago"))
            .expect("age 7 line");
        let idx3 = view
            .anomaly_lines
            .iter()
            .position(|l| l.contains("3 days ago"))
            .expect("age 3 line");
        assert!(idx14 < idx7 && idx7 < idx3, "{:?}", view.anomaly_lines);
    }

    // §7.1 -- a window date with two or more open starts uses the
    // multi-open lookback form.
    #[test]
    fn resolve_lookback_multi_open_form_for_two_opens_at_one_age() {
        let conn = test_db();
        let d = today() - ChronoDuration::days(5);
        storage::insert_punch(&conn, PunchKind::Start, d, t(9, 0), &Local).expect("insert");
        storage::insert_punch(&conn, PunchKind::Start, d, t(11, 0), &Local).expect("insert");

        let view = resolve(None, now_thu_1800(), &conn).expect("resolve");
        assert!(
            view.anomaly_lines
                .iter()
                .any(|l| l.contains("2 open stints on") && l.contains("5 days ago")),
            "{:?}",
            view.anomaly_lines
        );
        assert!(
            view.anomaly_lines
                .iter()
                .any(|l| l.contains(&format!("see mlm status {}", d.format("%Y-%m-%d")))),
            "{:?}",
            view.anomaly_lines
        );
    }

    // §7.1 -- status for a non-today date runs no lookback at all, even
    // though another window-aged date has an open start.
    #[test]
    fn resolve_status_for_non_today_date_runs_no_lookback() {
        let conn = test_db();
        let stale_date = today() - ChronoDuration::days(5);
        storage::insert_punch(&conn, PunchKind::Start, stale_date, t(9, 0), &Local)
            .expect("insert");
        let other_date = today() - ChronoDuration::days(1);

        let view = resolve(
            Some(&other_date.format("%Y-%m-%d").to_string()),
            now_thu_1800(),
            &conn,
        )
        .expect("resolve");
        // The stale date sits in an earlier week, so the carry-in marker
        // line is expected. Only lookback lines are ruled out.
        let lookback: Vec<_> = view
            .anomaly_lines
            .iter()
            .filter(|l| !l.starts_with("[!] carry-in includes anomalies"))
            .collect();
        assert!(
            lookback.is_empty(),
            "no lookback for a non-today date: {:?}",
            view.anomaly_lines
        );
    }

    // §7.1 -- an explicit today's-own-date string still runs the
    // lookback (a distinct code path from DATE omitted).
    #[test]
    fn resolve_explicit_todays_date_string_still_runs_lookback() {
        let conn = test_db();
        let stale_date = today() - ChronoDuration::days(5);
        storage::insert_punch(&conn, PunchKind::Start, stale_date, t(9, 0), &Local)
            .expect("insert");

        let view = resolve(
            Some(&today().format("%Y-%m-%d").to_string()),
            now_thu_1800(),
            &conn,
        )
        .expect("resolve");
        assert!(
            view.anomaly_lines.iter().any(|l| l.contains("5 days ago")),
            "{:?}",
            view.anomaly_lines
        );
    }

    // §1.2 -- an orphaned end alone at a window age produces no lookback
    // line for that date (the scan counts open starts only).
    #[test]
    fn resolve_lookback_orphan_alone_produces_no_lookback_line() {
        let conn = test_db();
        let d = today() - ChronoDuration::days(5);
        storage::insert_punch(&conn, PunchKind::End, d, t(18, 0), &Local).expect("insert");

        let view = resolve(None, now_thu_1800(), &conn).expect("resolve");
        assert!(
            !view.anomaly_lines.iter().any(|l| l.contains("days ago")),
            "{:?}",
            view.anomaly_lines
        );
    }

    // §4.3/§7.1 boundary -- age 15 is stale on its own status DATE but
    // absent from today's lookback (window is ages 2-14 only).
    #[test]
    fn resolve_age_fifteen_stale_on_own_date_but_absent_from_lookback() {
        let conn = test_db();
        let d = today() - ChronoDuration::days(15);
        storage::insert_punch(&conn, PunchKind::Start, d, t(9, 0), &Local).expect("insert");

        let own_view = resolve(
            Some(&d.format("%Y-%m-%d").to_string()),
            now_thu_1800(),
            &conn,
        )
        .expect("resolve");
        assert!(
            own_view
                .anomaly_lines
                .iter()
                .any(|l| l.contains("15 days old")),
            "{:?}",
            own_view.anomaly_lines
        );

        let today_view = resolve(None, now_thu_1800(), &conn).expect("resolve");
        assert!(
            !today_view
                .anomaly_lines
                .iter()
                .any(|l| l.contains("days ago")),
            "age 15 must not appear in today's lookback: {:?}",
            today_view.anomaly_lines
        );
    }

    // §5 -- fix-command round trip: a stop after the stale start's time
    // clears the anomaly on re-resolve.
    #[test]
    fn resolve_stale_start_fix_round_trip_via_stop_clears_anomaly() {
        let conn = test_db();
        let d = today() - ChronoDuration::days(5);
        storage::insert_punch(&conn, PunchKind::Start, d, t(9, 0), &Local).expect("insert");

        let before = resolve(
            Some(&d.format("%Y-%m-%d").to_string()),
            now_thu_1800(),
            &conn,
        )
        .expect("resolve");
        assert!(!before.anomaly_lines.is_empty());

        storage::insert_punch(&conn, PunchKind::End, d, t(17, 0), &Local).expect("insert");

        let after = resolve(
            Some(&d.format("%Y-%m-%d").to_string()),
            now_thu_1800(),
            &conn,
        )
        .expect("resolve");
        assert!(after.anomaly_lines.is_empty(), "{:?}", after.anomaly_lines);
    }

    // §5 -- fix-command round trip: deleting the offending orphaned end
    // clears the E8 anomaly on re-resolve.
    #[test]
    fn resolve_orphaned_end_fix_round_trip_via_delete_clears_anomaly() {
        let conn = test_db();
        let id = storage::insert_punch(&conn, PunchKind::End, today(), t(18, 0), &Local)
            .expect("insert");

        let before = resolve(None, now_thu_1800(), &conn).expect("resolve");
        assert!(
            before
                .anomaly_lines
                .iter()
                .any(|l| l.contains("orphaned end at 18:00"))
        );

        storage::delete_punch(&conn, id).expect("delete");

        let after = resolve(None, now_thu_1800(), &conn).expect("resolve");
        assert!(after.anomaly_lines.is_empty(), "{:?}", after.anomaly_lines);
    }

    // §7.3 -- every fix/or line carries the anomaly's own real date,
    // never a placeholder, for E7, stale, E8, and the lookback form.
    #[test]
    fn resolve_fix_lines_carry_the_real_date_for_every_anomaly_kind() {
        // Each kind gets its own connection so adjacent-date splice/
        // refused-join checks from one fixture can never interact with
        // another's.
        let e7_conn = test_db();
        let e7_date = today() - ChronoDuration::days(6);
        storage::insert_punch(&e7_conn, PunchKind::Start, e7_date, t(9, 0), &Local)
            .expect("insert");
        storage::insert_punch(&e7_conn, PunchKind::Start, e7_date, t(11, 0), &Local)
            .expect("insert");
        let e7_str = e7_date.format("%Y-%m-%d").to_string();
        let e7_view = resolve(Some(&e7_str), now_thu_1800(), &e7_conn).expect("resolve");
        assert!(
            e7_view
                .anomaly_lines
                .iter()
                .any(|l| l.contains(&format!("--date {e7_str}")))
        );

        let stale_conn = test_db();
        let stale_date = today() - ChronoDuration::days(5);
        storage::insert_punch(&stale_conn, PunchKind::Start, stale_date, t(9, 0), &Local)
            .expect("insert");
        let stale_str = stale_date.format("%Y-%m-%d").to_string();
        let stale_view = resolve(Some(&stale_str), now_thu_1800(), &stale_conn).expect("resolve");
        assert!(
            stale_view
                .anomaly_lines
                .iter()
                .any(|l| l.contains(&format!("--date {stale_str}")))
        );

        let e8_conn = test_db();
        let e8_date = today() - ChronoDuration::days(4);
        storage::insert_punch(&e8_conn, PunchKind::End, e8_date, t(18, 0), &Local).expect("insert");
        let e8_str = e8_date.format("%Y-%m-%d").to_string();
        let e8_view = resolve(Some(&e8_str), now_thu_1800(), &e8_conn).expect("resolve");
        assert!(
            e8_view
                .anomaly_lines
                .iter()
                .any(|l| l.contains(&format!("--date {e8_str}")))
        );

        let lookback_conn = test_db();
        let lookback_date = today() - ChronoDuration::days(9);
        storage::insert_punch(
            &lookback_conn,
            PunchKind::Start,
            lookback_date,
            t(9, 0),
            &Local,
        )
        .expect("insert");
        let lookback_str = lookback_date.format("%Y-%m-%d").to_string();
        let today_view = resolve(None, now_thu_1800(), &lookback_conn).expect("resolve");
        assert!(
            today_view
                .anomaly_lines
                .iter()
                .any(|l| l.contains(&format!("--date {lookback_str}"))),
            "{:?}",
            today_view.anomaly_lines
        );
    }

    // -----------------------------------------------------------------
    // live-eod: the open stint's live minutes count in the week line
    // and est. EOD projects from the open start.
    // -----------------------------------------------------------------

    fn punch(conn: &Connection, kind: PunchKind, date: NaiveDate, time: NaiveTime) {
        storage::insert_punch(conn, kind, date, time, &Local).expect("insert");
    }

    fn now_at(date: NaiveDate, h: u32, m: u32, s: u32) -> DateTime<Local> {
        Local
            .from_local_datetime(&date.and_hms_opt(h, m, s).unwrap())
            .unwrap()
    }

    /// Mon-Wed of week 2026-07 (1310 min) plus 445 completed minutes
    /// today (Thu), so fulfillment is 1755 and `required - fulfillment`
    /// is 165 on Thursday (1920 required).
    fn seed_thursday_completed(conn: &Connection) {
        punch(conn, PunchKind::Start, d(2026, 2, 9), t(9, 0));
        punch(conn, PunchKind::End, d(2026, 2, 9), t(19, 0));
        punch(conn, PunchKind::Start, d(2026, 2, 10), t(9, 0));
        punch(conn, PunchKind::End, d(2026, 2, 10), t(18, 50));
        punch(conn, PunchKind::Start, d(2026, 2, 11), t(9, 0));
        punch(conn, PunchKind::End, d(2026, 2, 11), t(11, 0));
        punch(conn, PunchKind::Start, today(), t(9, 0));
        punch(conn, PunchKind::End, today(), t(12, 0));
        punch(conn, PunchKind::Start, today(), t(13, 0));
        punch(conn, PunchKind::End, today(), t(17, 25));
    }

    // T1 + F6 -- week line and est. EOD use the post-live gap, and the
    // pinned SPEC §7.1 sample line is rebuilt from punches.
    #[test]
    fn live_eod_t1_open_stint_reduces_gap_and_pins_sample_line() {
        let conn = test_db();
        seed_thursday_completed(&conn);
        punch(&conn, PunchKind::Start, today(), t(17, 45));

        let view = resolve(None, now_thu_1800(), &conn).expect("resolve");
        assert_eq!(view.eod, Some(EodState::At(t(20, 30), EodDay::Today)));
        let out = render(&view);
        assert!(
            out.contains("Day total:     07h 40m (incl. ongoing), est. EOD 20:30\n"),
            "{out}"
        );
        assert!(
            view.week_line
                .contains("02h 30m left to 32h 00m by end of Thursday"),
            "{}",
            view.week_line
        );

        let later = resolve(None, now_at(today(), 18, 30, 0), &conn).expect("resolve");
        assert!(
            later
                .week_line
                .contains("02h 00m left to 32h 00m by end of Thursday")
        );
        assert_eq!(later.eod, Some(EodState::At(t(20, 30), EodDay::Today)));
    }

    // T2 -- est. EOD is stable across reads, the week line counts down.
    #[test]
    fn live_eod_t2_est_eod_stable_across_reads_week_line_counts_down() {
        let conn = test_db();
        seed_thursday_completed(&conn);
        punch(&conn, PunchKind::Start, today(), t(17, 45));

        let a = resolve(None, now_at(today(), 18, 0, 20), &conn).expect("resolve");
        let b = resolve(None, now_at(today(), 18, 30, 45), &conn).expect("resolve");
        assert_eq!(a.eod, b.eod);
        assert_eq!(a.eod, Some(EodState::At(t(20, 30), EodDay::Today)));
        assert!(
            a.week_line.contains("02h 30m left to 32h 00m"),
            "{}",
            a.week_line
        );
        assert!(
            b.week_line.contains("02h 00m left to 32h 00m"),
            "{}",
            b.week_line
        );
    }

    // T3 -- live tips the gap negative.
    #[test]
    fn live_eod_t3_live_tips_gap_negative() {
        let conn = test_db();
        seed_thursday_completed(&conn);
        punch(&conn, PunchKind::Start, today(), t(17, 45));

        let view = resolve(None, now_at(today(), 21, 0, 0), &conn).expect("resolve");
        assert_eq!(view.eod, Some(EodState::TargetAlreadyMet));
        let out = render(&view);
        assert!(
            view.week_line
                .contains("00h 30m over 32h 00m by end of Thursday"),
            "{}",
            view.week_line
        );
        assert!(out.contains(", done for today"), "{out}");
        assert!(!out.contains("est. EOD"), "{out}");
    }

    // T4 -- live makes the gap exactly zero.
    #[test]
    fn live_eod_t4_live_makes_gap_exactly_zero() {
        let conn = test_db();
        seed_thursday_completed(&conn);
        punch(&conn, PunchKind::Start, today(), t(17, 45));

        let view = resolve(None, now_at(today(), 20, 30, 0), &conn).expect("resolve");
        assert_eq!(view.eod, Some(EodState::TargetAlreadyMet));
        let out = render(&view);
        assert!(
            view.week_line
                .contains("00h 00m left to 32h 00m by end of Thursday"),
            "{}",
            view.week_line
        );
        assert!(out.contains(", done for today"), "{out}");
    }

    // T5 -- no open stint, no live credit and no est. EOD.
    #[test]
    fn live_eod_t5_no_open_stint_gap_equals_baseline() {
        let conn = test_db();
        seed_thursday_completed(&conn);

        let view = resolve(None, now_thu_1800(), &conn).expect("resolve");
        assert!(!view.has_open_stint);
        assert_eq!(view.eod, None);
        assert!(
            view.week_line
                .contains("02h 45m left to 32h 00m by end of Thursday")
        );
    }

    // T6 -- two open starts today, the later one supplies live and the
    // est. EOD projection.
    #[test]
    fn live_eod_t6_two_open_starts_use_the_later_one() {
        let conn = test_db();
        seed_thursday_completed(&conn);
        punch(&conn, PunchKind::Start, today(), t(17, 30));
        punch(&conn, PunchKind::Start, today(), t(17, 45));

        let view = resolve(None, now_thu_1800(), &conn).expect("resolve");
        assert!(
            view.week_line.contains("02h 30m left to 32h 00m"),
            "{}",
            view.week_line
        );
        assert_eq!(view.eod, Some(EodState::At(t(20, 30), EodDay::Today)));
    }

    // T7 -- est. EOD past midnight still carries the (tomorrow) suffix.
    #[test]
    fn live_eod_t7_tomorrow_suffix_still_fires() {
        let conn = test_db();
        seed_thursday_completed(&conn);
        punch(&conn, PunchKind::Start, today(), t(22, 0));

        let view = resolve(None, now_at(today(), 22, 10, 0), &conn).expect("resolve");
        assert!(
            view.week_line.contains("02h 35m left to 32h 00m"),
            "{}",
            view.week_line
        );
        assert_eq!(view.eod, Some(EodState::At(t(0, 45), EodDay::Tomorrow)));
        assert!(render(&view).contains("est. EOD 00:45 (tomorrow)"));
    }

    /// Renders the est. EOD clause for a Thursday open start at `h:m` with
    /// `now` equal to the start (live 0), under a week target of `target`
    /// minutes (Thursday `required` is 4/5 of it).
    fn eod_clause(target: i64, h: u32, m: u32) -> String {
        let conn = test_db();
        week_target::set_week_target(&conn, &wk(2026, 7), target).expect("set target");
        punch(&conn, PunchKind::Start, today(), t(h, m));
        let view = resolve(None, now_at(today(), h, m, 0), &conn).expect("resolve");
        let out = render(&view);
        let at = out
            .find("est. EOD")
            .unwrap_or_else(|| panic!("no EOD: {out}"));
        out[at..].lines().next().unwrap().to_string()
    }

    // 3.4 -- day suffix on est. EOD: none when it lands today.
    #[test]
    fn eod_day_suffix_same_day_has_none() {
        // Thu required 480: 09:00 + 8h = 17:00 today.
        assert_eq!(eod_clause(600, 9, 0), "est. EOD 17:00");
    }

    // 3.4 -- one day later is the bare word; the 1->2 day boundary.
    #[test]
    fn eod_day_suffix_one_day_is_tomorrow_and_two_days_is_weekday() {
        // Thu required 1920: 15:59 + 32h = Fri 23:59, 16:00 + 32h = Sat 00:00.
        assert_eq!(eod_clause(2400, 15, 59), "est. EOD 23:59 (tomorrow)");
        assert_eq!(eod_clause(2400, 16, 0), "est. EOD 00:00 (Sat)");
    }

    // 3.4 -- the 6->7 day boundary: weekday alone, then weekday plus date.
    #[test]
    fn eod_day_suffix_six_days_is_weekday_and_seven_days_adds_date() {
        // Thu required 10076: 00:03 + 10076 min = Wed 2026-02-18 23:59.
        assert_eq!(eod_clause(12595, 0, 3), "est. EOD 23:59 (Wed)");
        // Thu required 10080: 00:00 + 10080 min = Thu 2026-02-19 00:00.
        assert_eq!(eod_clause(12600, 0, 0), "est. EOD 00:00 (Thu 2026-02-19)");
    }

    // T8 -- an open start dated yesterday earns no live credit.
    #[test]
    fn live_eod_t8_yesterday_open_start_earns_no_credit() {
        let conn = test_db();
        punch(&conn, PunchKind::Start, d(2026, 2, 11), t(20, 0));

        let view = resolve(None, now_thu_1800(), &conn).expect("resolve");
        assert!(!view.has_open_stint);
        assert_eq!(view.eod, None);
        let baseline = resolve(None, now_thu_1800(), &test_db()).expect("resolve");
        assert_eq!(view.week_line, baseline.week_line);
    }

    // T9 -- an open start later than now projects from its own time.
    #[test]
    fn live_eod_t9_future_open_start_projects_from_start() {
        let conn = test_db();
        seed_thursday_completed(&conn);
        punch(&conn, PunchKind::Start, today(), t(19, 0));

        let view = resolve(None, now_thu_1800(), &conn).expect("resolve");
        assert!(
            view.week_line.contains("02h 45m left to 32h 00m"),
            "{}",
            view.week_line
        );
        assert_eq!(view.eod, Some(EodState::At(t(21, 45), EodDay::Today)));
    }

    // T9 variant -- future open start with the quota already met.
    #[test]
    fn live_eod_t9_future_open_start_quota_met_is_target_already_met() {
        let conn = test_db();
        week_target::set_week_target(&conn, &wk(2026, 7), 5).expect("set target");
        seed_thursday_completed(&conn);
        punch(&conn, PunchKind::Start, today(), t(19, 0));

        let view = resolve(None, now_thu_1800(), &conn).expect("resolve");
        assert!(view.week_line.contains(" over "), "{}", view.week_line);
        assert_eq!(view.eod, Some(EodState::TargetAlreadyMet));
    }

    // T10 -- Friday counts live and names the whole target on the week line.
    #[test]
    fn live_eod_t10_friday_counts_live_against_the_whole_target() {
        let conn = test_db();
        for day in 9..=12 {
            punch(&conn, PunchKind::Start, d(2026, 2, day), t(9, 0));
            punch(&conn, PunchKind::End, d(2026, 2, day), t(17, 0));
        }
        let friday = d(2026, 2, 13);
        punch(&conn, PunchKind::Start, friday, t(9, 0));

        let view = resolve(None, now_at(friday, 18, 0, 0), &conn).expect("resolve");
        let out = render(&view);
        assert!(
            view.week_line.contains("01h 00m over 40h 00m target"),
            "{}",
            view.week_line
        );
        assert!(out.contains(", done for today"), "{out}");
    }

    // F7 -- tomorrow's orphan end consumes today's open start (splice),
    // so ctx.day has no open stint and nothing is credited.
    #[test]
    fn live_eod_spliced_open_start_earns_no_credit() {
        let conn = test_db();
        punch(&conn, PunchKind::Start, today(), t(17, 45));
        punch(&conn, PunchKind::End, d(2026, 2, 13), t(0, 30));

        let view = resolve(None, now_thu_1800(), &conn).expect("resolve");
        assert!(!view.has_open_stint);
        assert_eq!(view.eod, None);
        // The spliced stint (17:45 to 00:30, 405 min) is completed time,
        // so the gap is 1920 - 405 with no live term.
        assert!(
            view.week_line
                .contains("25h 15m left to 32h 00m by end of Thursday"),
            "{}",
            view.week_line
        );
    }

    // -----------------------------------------------------------------
    // point-in-time-week: the week line is `required - fulfillment`, and
    // fulfillment, worked and today's day total all include live.
    // -----------------------------------------------------------------

    /// One completed stint per listed minute count, on consecutive days
    /// starting Monday 2026-02-09 (week 2026-07).
    fn seed_days(conn: &Connection, minutes: &[u32]) {
        for (i, m) in minutes.iter().enumerate() {
            let day = d(2026, 2, 9 + i as u32);
            punch(conn, PunchKind::Start, day, t(6, 0));
            punch(conn, PunchKind::End, day, t(6 + m / 60, m % 60));
        }
    }

    fn week_line_for(conn: &Connection, now: DateTime<Local>) -> String {
        resolve(None, now, conn).expect("resolve").week_line
    }

    // Mon-Thu: `left to` at gap 0 and +1, `over` at gap -1.
    #[test]
    fn pit_thursday_week_line_gap_zero_plus_one_minus_one() {
        for (thu, want) in [
            (
                480,
                "Week 2026-07:  00h 00m left to 32h 00m by end of Thursday (fulfillment 32h 00m / target 40h 00m)",
            ),
            (
                479,
                "Week 2026-07:  00h 01m left to 32h 00m by end of Thursday (fulfillment 31h 59m / target 40h 00m)",
            ),
            (
                481,
                "Week 2026-07:  00h 01m over 32h 00m by end of Thursday (fulfillment 32h 01m / target 40h 00m)",
            ),
        ] {
            let conn = test_db();
            seed_days(&conn, &[480, 480, 480, thu]);
            assert_eq!(week_line_for(&conn, now_thu_1800()), want, "thu={thu}");
        }
    }

    // Fri-Sun: `left to` / `over` the whole target, no weekday.
    #[test]
    fn pit_friday_week_line_gap_zero_plus_one_minus_one() {
        for (fri, want) in [
            (
                480,
                "Week 2026-07:  00h 00m left to 40h 00m target (fulfillment 40h 00m / target 40h 00m)",
            ),
            (
                479,
                "Week 2026-07:  00h 01m left to 40h 00m target (fulfillment 39h 59m / target 40h 00m)",
            ),
            (
                481,
                "Week 2026-07:  00h 01m over 40h 00m target (fulfillment 40h 01m / target 40h 00m)",
            ),
        ] {
            let conn = test_db();
            seed_days(&conn, &[480, 480, 480, 480, fri]);
            assert_eq!(
                week_line_for(&conn, now_at(d(2026, 2, 13), 18, 0, 0)),
                want,
                "fri={fri}"
            );
        }
    }

    // Fri-Sun required is the target itself, even when it is not a
    // multiple of 5 minutes.
    #[test]
    fn pit_sunday_required_is_the_target_not_a_multiple_of_five() {
        let conn = test_db();
        week_target::set_week_target(&conn, &wk(2026, 7), 2011).expect("set target");
        punch(&conn, PunchKind::Start, d(2026, 2, 15), t(12, 0));
        let view = resolve(None, now_at(d(2026, 2, 15), 12, 0, 0), &conn).expect("resolve");
        assert_eq!(
            view.week_line,
            "Week 2026-07:  33h 31m left to 33h 31m target (fulfillment 00h 00m / target 33h 31m)"
        );
        // 12:00 + 33h 31m lands Monday 21:31.
        assert_eq!(view.eod, Some(EodState::At(t(21, 31), EodDay::Tomorrow)));
        let out = render(&view);
        assert!(!out.contains("required"), "{out}");
    }

    // The target-already-met threshold moves with the new Sunday required.
    // Fulfillment 2010 is met under the old 2010 slice and short of 2011.
    #[test]
    fn pit_sunday_target_already_met_uses_full_target() {
        let conn = test_db();
        week_target::set_week_target(&conn, &wk(2026, 7), 2011).expect("set target");
        seed_days(&conn, &[480, 480, 480, 480]);
        punch(&conn, PunchKind::Start, d(2026, 2, 15), t(0, 0));

        let short = resolve(None, now_at(d(2026, 2, 15), 1, 30, 0), &conn).expect("resolve");
        assert!(
            short.week_line.contains("00h 01m left to 33h 31m target"),
            "{}",
            short.week_line
        );
        assert_eq!(short.eod, Some(EodState::At(t(1, 31), EodDay::Today)));

        let met = resolve(None, now_at(d(2026, 2, 15), 1, 31, 0), &conn).expect("resolve");
        assert!(
            met.week_line.contains("00h 00m left to 33h 31m target"),
            "{}",
            met.week_line
        );
        assert_eq!(met.eod, Some(EodState::TargetAlreadyMet));
    }

    // Zero-target week.
    #[test]
    fn pit_zero_target_week_line() {
        let conn = test_db();
        week_target::set_week_target(&conn, &wk(2026, 7), 0).expect("set target");
        let monday = d(2026, 2, 9);
        assert_eq!(
            week_line_for(&conn, now_at(monday, 9, 0, 0)),
            "Week 2026-07:  00h 00m left to 00h 00m by end of Monday (fulfillment 00h 00m / target 00h 00m)"
        );
        punch(&conn, PunchKind::Start, monday, t(9, 0));
        punch(&conn, PunchKind::End, monday, t(11, 0));
        assert_eq!(
            week_line_for(&conn, now_at(monday, 12, 0, 0)),
            "Week 2026-07:  02h 00m over 00h 00m by end of Monday (fulfillment 02h 00m / target 00h 00m)"
        );
    }

    // No pace clause on any weekday, with or without an open stint.
    #[test]
    fn pit_no_pace_clause_anywhere() {
        for day in 9..=15 {
            let now = now_at(d(2026, 2, day), 10, 0, 0);
            let conn = test_db();
            punch(&conn, PunchKind::Start, d(2026, 2, day), t(9, 0));
            let out = render(&resolve(None, now, &conn).unwrap());
            assert!(!out.contains("required"), "day {day}: {out}");
            let out = render(&resolve(None, now, &test_db()).unwrap());
            assert!(!out.contains("required"), "day {day}: {out}");
        }
    }

    // Day total counts live and carries `(incl. ongoing)`.
    #[test]
    fn pit_day_total_includes_live_with_incl_ongoing_flag() {
        let conn = test_db();
        seed_thursday_completed(&conn);
        punch(&conn, PunchKind::Start, today(), t(17, 45));
        let view = resolve(None, now_thu_1800(), &conn).expect("resolve");
        assert_eq!(view.day_total_minutes, 445 + 15);
        let out = render(&view);
        assert!(
            out.contains("Day total:     07h 40m (incl. ongoing), est. EOD 20:30"),
            "{out}"
        );
        assert!(!out.contains("(+ ongoing)"), "{out}");
    }

    // A start typed later than now: flag shows, nothing added.
    #[test]
    fn pit_future_dated_start_is_incl_ongoing_with_live_zero() {
        let conn = test_db();
        seed_thursday_completed(&conn);
        punch(&conn, PunchKind::Start, today(), t(19, 0));
        let view = resolve(None, now_thu_1800(), &conn).expect("resolve");
        assert_eq!(view.day_total_minutes, 445);
        let out = render(&view);
        assert!(
            out.contains("Day total:     07h 25m (incl. ongoing), est. EOD 21:45"),
            "{out}"
        );
    }

    // Earlier pages keep `(+ ongoing)` / `(+ unclosed)` and get no live.
    #[test]
    fn pit_earlier_page_flags_unchanged_and_get_no_live() {
        let conn = test_db();
        punch(&conn, PunchKind::Start, d(2026, 2, 11), t(20, 0));
        let out = render(&resolve(Some("2026-02-11"), now_thu_1800(), &conn).unwrap());
        assert!(
            out.contains("Day total:     00h 00m (+ ongoing)\n"),
            "{out}"
        );

        let conn = test_db();
        punch(&conn, PunchKind::Start, d(2026, 2, 7), t(20, 0));
        let out = render(&resolve(Some("2026-02-07"), now_thu_1800(), &conn).unwrap());
        assert!(
            out.contains("Day total:     00h 00m (+ unclosed)\n"),
            "{out}"
        );
    }

    // status DATE for an earlier date in the current week: the week line
    // counts today's live and uses today's weekday and required, while the
    // Day total for DATE has no live and no ongoing flag.
    #[test]
    fn pit_earlier_date_week_line_counts_live_day_total_does_not() {
        let conn = test_db();
        seed_thursday_completed(&conn);
        punch(&conn, PunchKind::Start, today(), t(17, 45));

        let today_view = resolve(None, now_thu_1800(), &conn).expect("resolve");
        let monday = resolve(Some("2026-02-09"), now_thu_1800(), &conn).expect("resolve");
        assert_eq!(monday.week_line, today_view.week_line);
        assert!(
            monday
                .week_line
                .starts_with("Week 2026-07:  02h 30m left to 32h 00m by end of Thursday"),
            "{}",
            monday.week_line
        );
        assert_eq!(monday.day_total_minutes, 600);
        let out = render(&monday);
        assert!(out.contains("Day total:     10h 00m\n"), "{out}");
        assert!(!out.contains("ongoing"), "{out}");
    }

    // A today-open start consumed by tomorrow's orphan end earns no live,
    // on `status` and on `status DATE` alike.
    #[test]
    fn pit_spliced_open_start_earns_no_live_for_earlier_date_either() {
        let conn = test_db();
        punch(&conn, PunchKind::Start, today(), t(17, 45));
        punch(&conn, PunchKind::End, d(2026, 2, 13), t(0, 30));
        let today_view = resolve(None, now_thu_1800(), &conn).expect("resolve");
        let monday = resolve(Some("2026-02-09"), now_thu_1800(), &conn).expect("resolve");
        assert_eq!(monday.week_line, today_view.week_line);
        assert!(
            monday.week_line.contains("(fulfillment 06h 45m / target"),
            "{}",
            monday.week_line
        );
    }

    // A future week's carry-in (seen here as its owed total) takes the
    // current week's live-inclusive fulfillment.
    #[test]
    fn pit_future_week_carry_in_includes_live() {
        let conn = test_db();
        seed_thursday_completed(&conn);
        punch(&conn, PunchKind::Start, today(), t(17, 45));
        let view =
            week_view::build_week_view(&conn, wk(2026, 8), now_thu_1800()).expect("build view");
        // fulfillment 1755 + 15 live; owed = 2400 - (1770 - 2400) = 3030.
        assert_eq!(view.headline, "Total behind: 50h 30m");
        assert_eq!(view.carry_in_minutes, 1770 - 2400);
    }

    // Open starts dated before today earn no live credit.
    #[test]
    fn pit_old_open_starts_earn_no_live() {
        let baseline = week_line_for(&test_db(), now_thu_1800());
        for day in [d(2026, 2, 11), d(2026, 2, 10)] {
            let conn = test_db();
            punch(&conn, PunchKind::Start, day, t(9, 0));
            assert_eq!(week_line_for(&conn, now_thu_1800()), baseline, "{day}");
        }
    }

    /// Week 2026-39 worked 2815 minutes (5 x 563), so week 2026-40 opens
    /// with carry-in +06h 55m.
    fn seed_carry_in_06h55(conn: &Connection) {
        for day in 21..=25 {
            punch(conn, PunchKind::Start, d(2026, 9, day), t(6, 0));
            punch(conn, PunchKind::End, d(2026, 9, day), t(15, 23));
        }
    }

    // The worked example: Thursday 18:05, carry-in +06h 55m.
    #[test]
    fn pit_worked_example_status_block() {
        let conn = test_db();
        seed_carry_in_06h55(&conn);
        for (day, end) in [(28, t(10, 30)), (29, t(10, 40)), (30, t(10, 30))] {
            punch(&conn, PunchKind::Start, d(2026, 9, day), t(6, 0));
            punch(&conn, PunchKind::End, d(2026, 9, day), end);
        }
        let thu = d(2026, 10, 1);
        punch(&conn, PunchKind::Start, thu, t(9, 0));
        punch(&conn, PunchKind::End, thu, t(13, 0));
        punch(&conn, PunchKind::Start, thu, t(13, 30));
        punch(&conn, PunchKind::End, thu, t(16, 55));
        punch(&conn, PunchKind::Start, thu, t(17, 25));

        let out = render(&resolve(None, now_at(thu, 18, 5, 0), &conn).expect("resolve"));
        let want = "Thu 2026-10-01\n\
\n\
Day total:     08h 05m (incl. ongoing), est. EOD 21:25\n\
Week 2026-40:  03h 20m left to 32h 00m by end of Thursday (fulfillment 28h 40m = worked 21h 45m + carry-in 06h 55m / target 40h 00m)\n\
\n\
\x20\x2009:00-13:00  (04h 00m)\n\
\x20\x2013:30-16:55  (03h 25m)\n\
\x20\x2017:25-now    (00h 40m, ongoing)\n";
        assert_eq!(out, want);
    }

    // The Friday line of the worked example.
    #[test]
    fn pit_worked_example_friday_line() {
        let conn = test_db();
        seed_carry_in_06h55(&conn);
        for day in [28, 29, 30, 1] {
            let date = if day == 1 {
                d(2026, 10, 1)
            } else {
                d(2026, 9, day)
            };
            punch(&conn, PunchKind::Start, date, t(6, 0));
            punch(&conn, PunchKind::End, date, t(14, 0));
        }
        // Closed 1935 (4 x 480 + 15), live 120: worked 34h 15m.
        let fri = d(2026, 10, 2);
        punch(&conn, PunchKind::Start, fri, t(6, 0));
        punch(&conn, PunchKind::End, fri, t(6, 15));
        punch(&conn, PunchKind::Start, fri, t(16, 0));
        let view = resolve(None, now_at(fri, 18, 0, 0), &conn).expect("resolve");
        assert_eq!(
            view.week_line,
            "Week 2026-40:  01h 10m over 40h 00m target (fulfillment 41h 10m = worked 34h 15m + carry-in 06h 55m / target 40h 00m)"
        );
    }

    // -----------------------------------------------------------------
    // carry marker composition and rendering
    // -----------------------------------------------------------------

    const MARKER_LINE: &str = "[!] carry-in includes anomalies from 2 earlier weeks, oldest 2026-38 (see mlm week 2026-38)";

    fn marker_two_weeks() -> render::CarryMarker {
        render::CarryMarker {
            weeks: 2,
            oldest: wk(2026, 38),
        }
    }

    fn e8_facts() -> DateAnomalies {
        DateAnomalies {
            orphaned_end_times: vec![t(18, 0)],
            ..DateAnomalies::none(today())
        }
    }

    #[test]
    fn compose_anomaly_lines_marker_only() {
        let lines = compose_anomaly_lines(
            &DateAnomalies::none(today()),
            Vec::new(),
            Some(&marker_two_weeks()),
        );
        assert_eq!(lines, vec![MARKER_LINE.to_string()]);
    }

    #[test]
    fn compose_anomaly_lines_orders_block_then_lookback_then_marker() {
        let lookback = vec!["[!] lookback line".to_string()];
        let lines = compose_anomaly_lines(&e8_facts(), lookback, Some(&marker_two_weeks()));
        let block = e8_facts().detail_lines();
        let mut want = block.clone();
        want.push("[!] lookback line".to_string());
        want.push(MARKER_LINE.to_string());
        assert_eq!(lines, want);
        assert_eq!(lines.len(), block.len() + 2);
    }

    #[test]
    fn compose_anomaly_lines_without_marker_is_todays_lines() {
        let lookback = vec!["[!] lookback line".to_string()];
        let lines = compose_anomaly_lines(&e8_facts(), lookback, None);
        let mut want = e8_facts().detail_lines();
        want.push("[!] lookback line".to_string());
        assert_eq!(lines, want);
        assert!(compose_anomaly_lines(&DateAnomalies::none(today()), Vec::new(), None).is_empty());
    }

    #[test]
    fn marker_renders_after_the_week_line_and_one_blank_line() {
        let mut view = base_view();
        view.anomaly_lines = vec![MARKER_LINE.to_string()];
        let out = render(&view);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[3], view.week_line);
        assert_eq!(lines[4], "");
        assert_eq!(lines[5], MARKER_LINE);
        assert_eq!(lines.len(), 6);
    }

    #[test]
    fn resolve_has_no_marker_line_while_the_scan_is_a_stub() {
        let conn = test_db();
        let view = resolve(None, now_thu_1800(), &conn).expect("resolve");
        assert!(view.anomaly_lines.is_empty());
    }

    // -----------------------------------------------------------------
    // status and `mlm week` account every week identically
    // -----------------------------------------------------------------

    /// A note-only first week (2026-02), worked weeks with carry, a target
    /// override, and a running Thursday in the current week (2026-07).
    fn seed_ledger_fixture(conn: &Connection) {
        storage::insert_note(
            conn,
            d(2026, 1, 5),
            "kickoff",
            now_thu_1800().with_timezone(&Utc),
        )
        .expect("insert note");
        punch(conn, PunchKind::Start, d(2026, 1, 19), t(9, 0));
        punch(conn, PunchKind::End, d(2026, 1, 19), t(17, 0));
        conn.execute(
            "INSERT INTO week_targets (week_id, target_minutes) VALUES ('2026-05', 1200)",
            [],
        )
        .expect("insert target");
        punch(conn, PunchKind::Start, d(2026, 2, 2), t(9, 0));
        punch(conn, PunchKind::End, d(2026, 2, 2), t(12, 0));
        seed_thursday_completed(conn);
        punch(conn, PunchKind::Start, today(), t(17, 45));
    }

    #[test]
    fn status_week_line_equals_week_view_headline_for_every_closed_week() {
        let conn = test_db();
        seed_ledger_fixture(&conn);
        for week in [
            wk(2026, 2),
            wk(2026, 3),
            wk(2026, 4),
            wk(2026, 5),
            wk(2026, 6),
        ] {
            let status = resolve(Some(&week.start().to_string()), now_thu_1800(), &conn)
                .expect("resolve status");
            let view = week_view::build_week_view(&conn, week, now_thu_1800()).expect("view");
            assert_eq!(
                status.week_line,
                format!("Week {week}:  {}", view.headline),
                "{week}"
            );
        }
    }

    #[test]
    fn status_current_week_line_carries_the_week_view_figures() {
        let conn = test_db();
        seed_ledger_fixture(&conn);
        let status = resolve(None, now_thu_1800(), &conn).expect("resolve status");
        let view = week_view::build_week_view(&conn, wk(2026, 7), now_thu_1800()).expect("view");
        assert!(
            status.week_line.contains(&view.headline),
            "{}",
            status.week_line
        );
        assert_ne!(view.carry_in_minutes, 0);
        let want = format!(
            "(fulfillment {} = worked {} + carry-in {} / target {})",
            format_minutes(view.fulfillment_minutes),
            format_minutes(view.worked_minutes),
            format_minutes(view.carry_in_minutes),
            format_minutes(view.target_minutes)
        );
        assert!(status.week_line.ends_with(&want), "{}", status.week_line);
    }

    #[test]
    fn history_accounting_matches_between_two_loads() {
        let conn = test_db();
        seed_ledger_fixture(&conn);
        let now_utc = now_thu_1800().with_timezone(&Utc);
        for week in [wk(2026, 2), wk(2026, 5), wk(2026, 7)] {
            let first = history::load_history(&conn, week, today(), now_utc).expect("load");
            let second = history::load_history(&conn, week, today(), now_utc).expect("load");
            assert_eq!(first.accounting(week), second.accounting(week), "{week}");
        }
    }
}
