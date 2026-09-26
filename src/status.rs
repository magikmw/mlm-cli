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
use crate::render::{self, DateAnomalies};
use crate::stint::{self, DayStints};
use crate::storage::{self, Punch};
use crate::time::format_minutes;
use crate::week::{self, WeekAccounting, WeekLedger};
use crate::week_target;

/// One dated finding from the §3.3/§3.6 14-day lookback scan. Defined in
/// `render.rs` (Task 2); re-exported here so Task 4 can call
/// `status::lookback` and name its return type without importing `render`
/// directly.
pub use crate::render::{LookbackFinding, LookbackKind};

// ---------------------------------------------------------------------------
// StatusView and its component types
// ---------------------------------------------------------------------------

/// The §7.1 estimated-EOD line's state (F9's three cases; `None` at the
/// `StatusView` level is the third — "no open stint", line omitted).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EodState {
    /// `est. EOD HH:MM` — quota not yet met, `now + gap`. The bool is
    /// true exactly when the estimate falls on a later calendar date
    /// than `today`, rendered as a `(tomorrow)` suffix.
    At(NaiveTime, bool),
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
    /// True when `weekday_number.min(5) == 5` — i.e. `required_minutes`
    /// has plateaued at the week's full target (Friday, Saturday, and
    /// Sunday alike under the default 5-day work week). Drives
    /// `day_total_line`'s "required today" vs "required by end of
    /// {weekday}" choice; computed once in `resolve()`, where `today`
    /// and `weekday_number` are in scope — `day_total_line` itself
    /// never sees either.
    pub day_reaches_week_cap: bool,
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
fn day_total_line(view: &StatusView) -> String {
    let mut s = format!(
        "{:<w$}{}",
        "Day total:",
        format_minutes(view.day_total_minutes),
        w = LABEL_WIDTH
    );
    if view.has_open_stint {
        match view.open_stint_age {
            OpenStintAge::Today | OpenStintAge::Yesterday => s.push_str(" (+ ongoing)"),
            OpenStintAge::TwoOrMoreDaysBack => s.push_str(" (+ unclosed)"),
        }
    }
    if let Some(hint) = &view.daily_target {
        let (word, magnitude) = if hint.gap_minutes >= 0 {
            ("left to", hint.gap_minutes)
        } else {
            ("over", -hint.gap_minutes)
        };
        if hint.day_reaches_week_cap {
            s.push_str(&format!(
                ", {} {} {} required today",
                format_minutes(magnitude),
                word,
                format_minutes(hint.required_minutes),
            ));
        } else {
            s.push_str(&format!(
                ", {} {} {} required by end of {}",
                format_minutes(magnitude),
                word,
                format_minutes(hint.required_minutes),
                view.weekday_name.as_deref().unwrap_or_default()
            ));
        }
    }
    match &view.eod {
        Some(EodState::At(t, false)) => s.push_str(&format!(", est. EOD {}", t.format("%H:%M"))),
        Some(EodState::At(t, true)) => {
            s.push_str(&format!(", est. EOD {} (tomorrow)", t.format("%H:%M")))
        }
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

/// The single call site for `render::status_week_line`, guarded against
/// Milestone 9 review finding 1 (adjacent same-typed `i64` minute
/// params): takes the whole `WeekAccounting` by reference so the
/// owed/fulfillment/target fields can never be transposed by a typo at
/// a call site, since there is only one call site and its argument
/// order is named here, not positional at every use.
fn week_line(acct: &WeekAccounting, today: NaiveDate) -> String {
    render::status_week_line(
        acct.week,
        acct.owed,
        acct.fulfillment,
        acct.target,
        acct.carry_in,
        today,
    )
}

/// `at_utc`'s local wall-clock time, factoring out the
/// `.with_timezone(&Local).time()` pattern repeated across this module's
/// stint/anomaly/lookback fact-building.
fn local_time(at_utc: DateTime<Utc>) -> NaiveTime {
    at_utc.with_timezone(&Local).time()
}

/// Every classification `resolve()` needs for one target date: the
/// post-splice view (`day`) plus the three raw, pre-splice views
/// (`prev_raw`/`day_raw`/`next_raw`) the refused-join predicate requires
/// (Contract B: "raw single-date classifications ... never
/// classify_at-resolved ones").
struct DateContext {
    day: DayStints,
    prev_raw: DayStints,
    day_raw: DayStints,
    next_raw: DayStints,
}

fn classify_date_context(
    prev_punches: &[Punch],
    punches: &[Punch],
    next_punches: &[Punch],
    now_utc: DateTime<Utc>,
) -> DateContext {
    DateContext {
        day: stint::classify_at(prev_punches, punches, next_punches, now_utc),
        prev_raw: stint::classify(prev_punches, now_utc),
        day_raw: stint::classify(punches, now_utc),
        next_raw: stint::classify(next_punches, now_utc),
    }
}

/// Builds `target_date`'s `render::DateAnomalies` from its `DateContext`
/// plus Task 1's predicates. `stale_open_start` reads `ctx.day`
/// (post-splice, matching §2's "open start" definition); `is_refused_join`
/// reads the raw pair, per Contract B.
fn anomaly_facts_for_date(
    ctx: &DateContext,
    target_date: NaiveDate,
    today: NaiveDate,
    punches: &[Punch],
    next_punches: &[Punch],
) -> DateAnomalies {
    DateAnomalies {
        date: target_date,
        open_stint_count: ctx.day.open.len(),
        latest_open_start: ctx
            .day
            .open
            .iter()
            .map(|o| local_time(o.start.at_utc))
            .max(),
        orphaned_end_times: ctx
            .day
            .orphaned_ends
            .iter()
            .map(|o| local_time(o.punch.at_utc))
            .collect(),
        stale_open_start: stint::stale_open_start(&ctx.day, target_date, today).map(|s| {
            render::StaleOpenStart {
                time: local_time(s.at),
                age_days: s.age_days,
            }
        }),
        refused_join_with_previous: stint::is_refused_join(&ctx.prev_raw, &ctx.day_raw, punches)
            .then(|| target_date.pred_opt().unwrap()),
        refused_join_with_next: stint::is_refused_join(&ctx.day_raw, &ctx.next_raw, next_punches)
            .then(|| target_date.succ_opt().unwrap()),
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
            time: local_time(day.open[0].start.at_utc),
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
/// Called by `status::resolve` for today's own `status` (§3.3) and by
/// `commands::start` after every successful write (§3.6) -- this is the
/// one place the scan is implemented, so the two callers can never
/// disagree about what counts as a stale window date.
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
        // (smallest calendar date) first -- oldest-first per §3.3.
        let date = today - ChronoDuration::days(age);
        if let Some(finding) = lookback_finding_for_date(date, age, &by_date, now_utc) {
            findings.push(finding);
        }
    }
    Ok(findings)
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
    let open_stint_age = if target_date == today {
        OpenStintAge::Today
    } else if target_date == today - ChronoDuration::days(1) {
        OpenStintAge::Yesterday
    } else {
        OpenStintAge::TwoOrMoreDaysBack
    };

    let punches = storage::punches_for_date(conn, target_date)?;
    let notes = storage::notes_for_date(conn, target_date)?;
    let prev_punches = storage::punches_for_date(conn, target_date.pred_opt().unwrap())?;
    let next_punches = storage::punches_for_date(conn, target_date.succ_opt().unwrap())?;

    let now_utc = now.with_timezone(&Utc);
    let ctx = classify_date_context(&prev_punches, &punches, &next_punches, now_utc);

    let receiving_end_time =
        stint::splice_candidate(ctx.prev_raw.open.len(), &ctx.day_raw, &punches)
            .then(|| local_time(ctx.day_raw.orphaned_ends[0].punch.at_utc));
    let receiving_date_suffix = receiving_end_time
        .map(|t| format!("  ({} continues previous day's stint)", t.format("%H:%M")));

    let day_total_minutes = ctx.day.completed_minutes();
    let has_open_stint = ctx.day.is_ongoing();

    let target_week = WeekId::from_date(target_date);
    let ledger = build_ledger(conn, target_week, now_utc)?;
    let acct = week::week_accounting(target_week, &ledger, &ledger);
    let week_line_str = week_line(&acct, today);

    let facts = anomaly_facts_for_date(&ctx, target_date, today, &punches, &next_punches);
    let mut anomaly_lines = facts.detail_lines();
    if is_today {
        for finding in lookback(conn, today, now_utc)? {
            anomaly_lines.extend(finding.detail_lines());
        }
    }

    let (daily_target, eod, weekday_name) = if is_today {
        let daily_target_minutes = week::daily_target_minutes(acct.target);
        let weekday_number = i64::from(today.weekday().number_from_monday());
        let required_minutes = daily_target_minutes * weekday_number.min(5);
        let gap_minutes = required_minutes - acct.fulfillment;
        let day_reaches_week_cap = weekday_number.min(5) == 5;
        let hint = DailyTargetHint {
            required_minutes,
            gap_minutes,
            day_reaches_week_cap,
        };
        let eod = if has_open_stint {
            if gap_minutes > 0 {
                let eod_datetime = now + ChronoDuration::minutes(gap_minutes);
                let eod_time = eod_datetime.time();
                let is_tomorrow = eod_datetime.date_naive() != today;
                Some(EodState::At(eod_time, is_tomorrow))
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

    let stints = build_stint_lines(&ctx.day, open_stint_age);
    let note_bodies = notes.into_iter().map(|n| n.body).collect();

    Ok(StatusView {
        header: date::format_date_with_weekday(target_date),
        receiving_date_suffix,
        day_total_minutes,
        has_open_stint,
        daily_target,
        weekday_name,
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
            receiving_date_suffix: None,
            day_total_minutes: 0,
            has_open_stint: false,
            daily_target: None,
            weekday_name: None,
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
        view.daily_target = Some(DailyTargetHint {
            required_minutes: 480,
            gap_minutes: 480,
            day_reaches_week_cap: false,
        });
        view.weekday_name = Some("Thursday".to_string());
        view.eod = Some(EodState::At(t(18, 0) + ChronoDuration::minutes(480), false));
        view.stints = vec![StintLine {
            start: t(9, 0),
            end: StintEnd::Now,
            duration_minutes: 0,
            spans_to_next_day: false,
            open_stint_age: None,
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
            spans_to_next_day: false,
            open_stint_age: None,
        }];
        view.daily_target = Some(DailyTargetHint {
            required_minutes: 480,
            gap_minutes: -30,
            day_reaches_week_cap: false,
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

    // §4 -- today's open stint is byte-identical to today's current
    // output: no new literal in either the stint line or Day total.
    #[test]
    fn open_stint_today_is_byte_identical_to_current_output() {
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
        assert!(out.contains("Day total:     00h 00m (+ ongoing)"));
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
        view.daily_target = Some(DailyTargetHint {
            required_minutes: 1920,
            gap_minutes: 165,
            day_reaches_week_cap: false,
        });
        view.weekday_name = Some("Thursday".to_string());
        view.eod = Some(EodState::At(t(20, 45), false));
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
                day_reaches_week_cap: false,
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
            day_reaches_week_cap: false,
        });
        view.weekday_name = Some("Thursday".to_string());
        view.eod = None;
        let out = render(&view);
        assert!(!out.contains("est. EOD"));
        assert!(!out.contains("target already met"));
        assert!(!out.contains("(+ ongoing)"));
        assert!(out.contains("left to 08h 00m required by end of Thursday"));
    }

    // New -- Fix B: est. EOD carries a (tomorrow) suffix when the estimate
    // falls on a later calendar date than today.
    #[test]
    fn fix_b_eod_estimate_tomorrow_gets_suffix() {
        let mut view = base_view();
        view.day_total_minutes = 60;
        view.has_open_stint = true;
        view.daily_target = Some(DailyTargetHint {
            required_minutes: 480,
            gap_minutes: 420,
            day_reaches_week_cap: false,
        });
        view.weekday_name = Some("Thursday".to_string());
        view.eod = Some(EodState::At(t(1, 30), true));
        let out = render(&view);
        assert!(out.contains(", est. EOD 01:30 (tomorrow)"), "{out}");
    }

    // Companion negative case: same clock time, false bool, no suffix --
    // confirms the two arms are actually distinct, not that "true" always
    // wins on any input.
    #[test]
    fn fix_b_eod_estimate_same_day_has_no_tomorrow_suffix() {
        let mut view = base_view();
        view.day_total_minutes = 60;
        view.has_open_stint = true;
        view.daily_target = Some(DailyTargetHint {
            required_minutes: 480,
            gap_minutes: 420,
            day_reaches_week_cap: false,
        });
        view.weekday_name = Some("Thursday".to_string());
        view.eod = Some(EodState::At(t(1, 30), false));
        let out = render(&view);
        assert!(out.contains(", est. EOD 01:30"), "{out}");
        assert!(!out.contains("(tomorrow)"), "{out}");
    }

    // New -- Fix C: Friday reaches the week cap, day-total clause says
    // "required today", not "required by end of Friday".
    #[test]
    fn fix_c_friday_reaches_week_cap_says_required_today() {
        let mut view = base_view();
        view.day_total_minutes = 445;
        view.daily_target = Some(DailyTargetHint {
            required_minutes: 2400,
            gap_minutes: 100,
            day_reaches_week_cap: true,
        });
        view.weekday_name = Some("Friday".to_string());
        let out = render(&view);
        assert!(out.contains("required today"), "{out}");
        assert!(!out.contains("required by end of Friday"), "{out}");
        assert!(
            out.contains("01h 40m left to 40h 00m required today"),
            "{out}"
        );
    }

    // New -- Fix C: Saturday also reaches the week cap (not "the one day" --
    // spec §3's correction).
    #[test]
    fn fix_c_saturday_reaches_week_cap_says_required_today() {
        let mut view = base_view();
        view.day_total_minutes = 0;
        view.daily_target = Some(DailyTargetHint {
            required_minutes: 2400,
            gap_minutes: 2400,
            day_reaches_week_cap: true,
        });
        view.weekday_name = Some("Saturday".to_string());
        let out = render(&view);
        assert!(out.contains("required today"), "{out}");
        assert!(!out.contains("required by end of Saturday"), "{out}");
        // Same required_minutes figure as the adjacent Friday case above --
        // both plateau at the full week target.
        assert!(out.contains("40h 00m required today"), "{out}");
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
        let acct = current_week_acct(645, 1755, 2400);
        let view = StatusView {
            header: "Thu 2026-02-12".to_string(),
            day_total_minutes: 445,
            has_open_stint: true,
            daily_target: Some(DailyTargetHint {
                required_minutes: 1920,
                gap_minutes: 165,
                day_reaches_week_cap: false,
            }),
            weekday_name: Some("Thursday".to_string()),
            eod: Some(EodState::At(t(20, 45), false)),
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
                day_reaches_week_cap: false,
            }),
            weekday_name: Some("Thursday".to_string()),
            eod: Some(EodState::At(t(20, 45), false)),
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
            daily_target: None,
            weekday_name: None,
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
        t6b_view.daily_target = Some(DailyTargetHint {
            required_minutes: 480,
            gap_minutes: -20,
            day_reaches_week_cap: false,
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
                day_reaches_week_cap: false,
            }),
            weekday_name: Some("Thursday".to_string()),
            eod: Some(EodState::At(t(20, 45), false)),
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

    // Current week whose fulfillment already exceeds target: the week
    // line drops the weekday deadline and reads "ahead of target".
    #[test]
    fn resolve_current_week_ahead_of_target_renders_ahead_of_target() {
        let conn = test_db();
        week_target::set_week_target(&conn, &wk(2026, 7), 5).expect("set target"); // tiny target
        storage::insert_punch(&conn, PunchKind::Start, d(2026, 2, 9), t(8, 0), &Local)
            .expect("insert");
        storage::insert_punch(&conn, PunchKind::End, d(2026, 2, 9), t(17, 0), &Local)
            .expect("insert");

        let view = resolve(None, now_thu_1800(), &conn).expect("resolve");
        assert!(
            view.week_line.contains("ahead of target (fulfillment "),
            "week_line: {}",
            view.week_line
        );
        let out = render(&view);
        assert!(out.contains("ahead of target (fulfillment "), "{out}");
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

    // Direct regression companion: confirms the new call site doesn't
    // accidentally always splice -- only ever mechanically, per Task 1's
    // 1:1 gate. `D` has two trailing opens (E7, not 1:1), so `D+1`'s
    // single `End 00:30` cannot close either of them and stays a genuine
    // orphaned end. Since the pair isn't 1:1, it's also a refused join
    // (§3.2): both dates carry their own headline plus a line naming the
    // other.
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
        let hint = view.daily_target.expect("is_today");
        assert!(hint.gap_minutes <= 0, "gap must be <= 0: {hint:?}");
        assert_eq!(view.eod, Some(EodState::TargetAlreadyMet));
        let out = render(&view);
        assert!(out.contains(", target already met"), "{out}");
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
        assert!(hint.day_reaches_week_cap, "Saturday reaches the week cap");
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
        assert!(out.contains("33h 30m required today"));
        assert!(!out.contains("required by end of Sunday"));
        assert!(hint.day_reaches_week_cap, "Sunday reaches the week cap");
    }

    // New -- Fix B, resolve()-level: a gap_minutes large enough to push
    // `now + gap` past midnight sets EodState::At's bool to true and
    // day_total_line renders the (tomorrow) suffix.
    #[test]
    fn resolve_fix_b_eod_estimate_crossing_midnight_is_marked_tomorrow() {
        let conn = test_db();
        // No punches at all today: day_total_minutes stays 0, has_open_stint
        // is false by default -- so first give the day an open stint to
        // satisfy the `has_open_stint` precondition for `eod` to be `Some`.
        storage::insert_punch(&conn, PunchKind::Start, today(), t(9, 0), &Local).expect("insert");
        let view = resolve(None, now_thu_1800(), &conn).expect("resolve");
        let hint = view.daily_target.expect("is_today");
        assert!(
            hint.gap_minutes > 360,
            "gap must push well past midnight: {hint:?}"
        );
        match view.eod {
            Some(EodState::At(_, is_tomorrow)) => assert!(is_tomorrow, "expected tomorrow marker"),
            other => panic!("expected EodState::At, got {other:?}"),
        }
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
            "carry-in breakdown should survive build_ledger -> week_accounting -> \
             week_line -> status_week_line: {out}"
        );
    }

    // -----------------------------------------------------------------
    // week_line helper sanity: confirms the wrapper never transposes.
    // -----------------------------------------------------------------

    #[test]
    fn week_line_matches_status_week_line_field_for_field() {
        let acct = current_week_acct(645, 1755, 2400);
        let want = render::status_week_line(
            acct.week,
            acct.owed,
            acct.fulfillment,
            acct.target,
            acct.carry_in,
            today(),
        );
        assert_eq!(week_line(&acct, today()), want);
    }

    // -----------------------------------------------------------------
    // open-punch-visibility Task 3: stale open start, refused join, and
    // the shared 14-day lookback (spec §3.1-§3.4, §5).
    // -----------------------------------------------------------------

    // §3.1 -- a lone open start at age >= 2 gets the new stale-open-start
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

    // §3.1 -- age 0 and age 1 stay non-anomalies.
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

    // §3.1 -- ages 2, 14, and 15 are all stale (age doesn't stop being
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

    // §3.1 -- two or more open starts on a date show only the E7 line;
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

    // §3.2 -- refused join shape 1: two opens on the earlier date. Both
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
    }

    // §3.2 -- refused join shape 2: the orphan isn't the later date's
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

    // §3.2 -- refused join shape 3: two orphans on the later date.
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

    // §3.2 -- regression: the clean-splice shape produces no refused
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

    // §3.2 -- a refused join between yesterday and today shows on
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

    // §3.4 -- ordering: E7 lines precede the refused-join-with-next
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

    // §3.4 -- ordering: stale, then E8 (ascending instant), then the
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

    // §3.3 -- today's status lists stale starts from the lookback
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

    // §3.3 -- a window date with two or more open starts uses the
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

    // §3.3 -- status for a non-today date runs no lookback at all, even
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
        assert!(
            view.anomaly_lines.is_empty(),
            "no lookback for a non-today date: {:?}",
            view.anomaly_lines
        );
    }

    // §3.3 -- an explicit today's-own-date string still runs the
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

    // §3.1/§3.3 boundary -- age 15 is stale on its own status DATE but
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

    // §3.4/§5 -- every fix/or line carries the anomaly's own real date,
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

    // Published interface: `lookback` itself, independent of `resolve()`
    // wiring -- Task 4 (`commands::start`) calls this directly.
    #[test]
    fn lookback_returns_findings_oldest_first() {
        let conn = test_db();
        storage::insert_punch(
            &conn,
            PunchKind::Start,
            today() - ChronoDuration::days(10),
            t(9, 0),
            &Local,
        )
        .expect("insert");
        storage::insert_punch(
            &conn,
            PunchKind::Start,
            today() - ChronoDuration::days(4),
            t(9, 0),
            &Local,
        )
        .expect("insert");

        let findings =
            lookback(&conn, today(), now_thu_1800().with_timezone(&Utc)).expect("lookback");
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].date, today() - ChronoDuration::days(10));
        assert_eq!(findings[1].date, today() - ChronoDuration::days(4));
        assert_eq!(findings[0].age_days, 10);
        assert_eq!(findings[1].age_days, 4);
    }
}
