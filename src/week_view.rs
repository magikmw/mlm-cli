//! `mlm week [WEEK_ID]` — the rendering half of Milestone 11 (SPEC.md
//! §3.6, §7.2, §7.3).
//!
//! Split per the milestone plan: [`render_week`] is a pure function
//! (`WeekView` in, `String` out — no DB, no clock) so every §7.2/§7.3
//! layout rule is a byte-exact unit test; [`run`] is the thin command
//! wiring that resolves the week id, builds the view from storage +
//! Milestone 5/6/9, and prints it.
//!
//! Landmine notes (plans/reports/milestone-9-review.md):
//! - `render::week_headline` takes the whole `WeekAccounting`, so the
//!   current week's headline here and in `status` come from one function.
//! - Per-row anomaly detection goes through `anomalies::build_facts`
//!   exclusively (`has_any()`), never `DayStints::has_anomaly()` /
//!   `DayStints::anomalies()` directly, even though the latter are public
//!   and would compile.
//! - Accounting comes from `history::load_history`, the same loader
//!   `status` uses. A failed storage read is a hard error.

use chrono::{DateTime, Local, NaiveDate, Utc};
use rusqlite::Connection;

use crate::anomalies;
use crate::carry_scan;
use crate::cli::WeekArgs;
use crate::date::{WeekId, format_date, parse_week_id};
use crate::history::{self, PunchBuckets};
use crate::lookback;
use crate::render::{self, CarryMarker};
use crate::time::format_minutes;

/// Left-hand label column width shared with the trailing block (§4.2).
/// Mirrors `render::LABEL_WIDTH`; kept as a local constant so this
/// module's formatting doesn't depend on `render`'s constant staying at
/// this exact value for a reason unrelated to week's own layout.
const LABEL_WIDTH: usize = 15;

const WEEKDAY_ABBREVS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/// One row of the §7.2 table. Built once per date in the week's Mon..Sun
/// span — see `build_rows`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeekRow {
    pub weekday_abbrev: &'static str,
    pub date_display: String,
    /// Completed-stint minutes (§2.4), plus the running stint's live
    /// minutes on today's row only. Always >= 0.
    pub minutes: i64,
    /// `(incl. ongoing)` marker: an open stint AND this date is today.
    pub is_ongoing: bool,
    /// `[!]` marker, via `render::DateAnomalies::has_any()` exclusively.
    pub has_anomaly: bool,
    /// `[i]` marker: this row's date is the overnight stint's date. Never
    /// feeds `has_any`, the `[!]` footer, the carry marker or carry
    /// accounting.
    pub has_overnight: bool,
}

/// Everything `render_week` needs, and nothing it can compute itself
/// (plan §2.1). `owed` is deliberately absent: the headline already
/// states the week's standing, so the renderer cannot accidentally
/// re-render it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeekView {
    pub week_id_display: String,
    pub span_start_display: String,
    pub span_end_display: String,
    pub headline: String,
    pub rows: [WeekRow; 7],
    pub carry_in_minutes: i64,
    pub worked_minutes: i64,
    pub fulfillment_minutes: i64,
    pub target_minutes: i64,
    /// Set when earlier weeks hold anomalies that feed this week's
    /// carry-in (SPEC §3.3). Adds a line above the totals and a `[!]` on
    /// the Carry-in figure.
    pub carry_marker: Option<CarryMarker>,
    /// Monday case: the overnight stint's date when the shown week contains
    /// today and that date is in the previous week. Names Sunday in the
    /// `[i]` footer slot because no row of this week is marked.
    pub overnight_last_week: Option<NaiveDate>,
}

/// The §7.2/§7.3 layout, byte-exact. Pure: no I/O, no clock.
pub fn render_week(view: &WeekView) -> String {
    let mut out = String::new();
    push_header(&mut out, view);
    push_rows(&mut out, view);
    push_notices(&mut out, view);
    push_totals(&mut out, view);
    out
}

fn push_header(out: &mut String, view: &WeekView) {
    out.push_str(&format!(
        "Week {} ({} - {})\n",
        view.week_id_display, view.span_start_display, view.span_end_display
    ));
    out.push('\n');
    out.push_str(&view.headline);
    out.push('\n');
    out.push('\n');
}

/// The seven day rows, then one blank line.
fn push_rows(out: &mut String, view: &WeekView) {
    for row in &view.rows {
        out.push_str("  ");
        out.push_str(row.weekday_abbrev);
        out.push(' ');
        out.push_str(&row.date_display);
        out.push_str("   ");
        out.push_str(&format_minutes(row.minutes));
        if row.is_ongoing {
            out.push_str(" (incl. ongoing)");
        }
        out.push_str(row_markers(row));
        out.push('\n');
    }
    out.push('\n');
}

/// `  [!]`, `  [i]` or `  [!] [i]`, or nothing.
fn row_markers(row: &WeekRow) -> &'static str {
    match (row.has_anomaly, row.has_overnight) {
        (true, true) => "  [!] [i]",
        (true, false) => "  [!]",
        (false, true) => "  [i]",
        (false, false) => "",
    }
}

/// The footer slot above the totals, followed by one blank line whenever
/// any line printed.
fn push_notices(out: &mut String, view: &WeekView) {
    let lines = notice_lines(view);
    for line in &lines {
        out.push_str(line);
        out.push('\n');
    }
    if !lines.is_empty() {
        out.push('\n');
    }
}

/// Footer order: `[!]` footer, `[i]` footer or Monday line, carry marker line.
fn notice_lines(view: &WeekView) -> Vec<String> {
    let mut lines = Vec::new();
    if view.rows.iter().any(|r| r.has_anomaly) {
        lines.push(render::WEEK_ANOMALY_FOOTER.to_string());
    }
    lines.extend(overnight_footer(view));
    lines.extend(view.carry_marker.as_ref().map(render::carry_marker_line));
    lines
}

/// The plain footer when any row shows `[i]`, else the Monday line.
fn overnight_footer(view: &WeekView) -> Option<String> {
    if let Some(row) = view.rows.iter().find(|r| r.has_overnight) {
        let label = format!("{} {}", row.weekday_abbrev, row.date_display);
        return Some(render::week_overnight_footer(&label));
    }
    view.overnight_last_week
        .map(render::week_overnight_monday_footer)
}

fn push_totals(out: &mut String, view: &WeekView) {
    for (label, minutes) in [
        ("Carry-in:", view.carry_in_minutes),
        ("Worked:", view.worked_minutes),
        ("Fulfillment:", view.fulfillment_minutes),
        ("Target:", view.target_minutes),
    ] {
        out.push_str(&format!(
            "{:<width$}{}",
            label,
            format_minutes(minutes),
            width = LABEL_WIDTH
        ));
        if label == "Carry-in:" && view.carry_marker.is_some() {
            out.push_str("  [!]");
        }
        out.push('\n');
    }
}

/// One row per date of the week. Each date is classified against its own
/// previous and next buckets, so a midnight-spanning stint that starts or
/// ends just outside the week's seven dates still resolves at the
/// boundary date it touches. `buckets` must cover one day past each end of
/// the week, which `history::load_history` guarantees.
///
/// Anomalies go through `anomalies::build_facts` exclusively, never
/// `day.has_anomaly()`/`day.anomalies()` directly.
fn build_rows_from(
    dates: [NaiveDate; 7],
    buckets: &PunchBuckets,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
    overnight: Option<NaiveDate>,
) -> [WeekRow; 7] {
    std::array::from_fn(|i| {
        let date = dates[i];
        let trio = buckets.trio(date);
        let ctx = anomalies::classify_context(&trio, now_utc);
        let facts = anomalies::build_facts(&ctx, &trio, date, today, now_utc);
        let is_today = date == today;
        WeekRow {
            weekday_abbrev: WEEKDAY_ABBREVS[i],
            date_display: format_date(date),
            minutes: ctx.day.completed_minutes()
                + if is_today { ctx.day.live_minutes() } else { 0 },
            is_ongoing: ctx.day.is_ongoing() && is_today,
            has_anomaly: facts.has_any(),
            has_overnight: Some(date) == overnight,
        }
    })
}

#[cfg(test)]
fn build_rows(
    dates: [NaiveDate; 7],
    punches: &[crate::storage::Punch],
    today: NaiveDate,
    now_utc: DateTime<Utc>,
) -> [WeekRow; 7] {
    let buckets = PunchBuckets::from_punches(punches);
    let overnight = lookback::overnight_stint(&buckets, today, now_utc).map(|o| o.date);
    build_rows_from(dates, &buckets, today, now_utc, overnight)
}

/// Build the full `WeekView` for `week`. Every storage read goes through
/// `history::load_history`, so a failed read is an error.
pub fn build_week_view(
    conn: &Connection,
    week: WeekId,
    now: DateTime<Local>,
) -> anyhow::Result<WeekView> {
    let today = now.date_naive();
    let now_utc = now.with_timezone(&Utc);
    let (start, end) = week.span();

    let history = history::load_history(conn, week, today, now_utc)?;
    let overnight = lookback::overnight_stint(&history.buckets, today, now_utc).map(|o| o.date);
    let rows = build_rows_from(week.dates(), &history.buckets, today, now_utc, overnight);
    let acct = history.accounting(week);
    let carry_marker = carry_scan::carry_marker(&history, week, today, now_utc);

    Ok(WeekView {
        week_id_display: week.to_string(),
        span_start_display: format_date(start),
        span_end_display: format_date(end),
        headline: render::week_headline(&acct, today),
        rows,
        carry_in_minutes: acct.carry_in,
        worked_minutes: acct.worked,
        fulfillment_minutes: acct.fulfillment,
        target_minutes: acct.target,
        carry_marker,
        overnight_last_week: monday_overnight(week, today, overnight),
    })
}

/// The overnight date when the shown week holds today but not that date.
fn monday_overnight(
    week: WeekId,
    today: NaiveDate,
    overnight: Option<NaiveDate>,
) -> Option<NaiveDate> {
    let (start, end) = week.span();
    let holds = |d: NaiveDate| (start..=end).contains(&d);
    overnight.filter(|&d| holds(today) && !holds(d))
}

/// `mlm week [WEEK_ID]` (the `action: None` arm; `week target` is
/// Milestone 8's `week_target::run`, dispatched separately by `main`).
pub fn run(conn: &Connection, now: DateTime<Local>, args: &WeekArgs) -> anyhow::Result<()> {
    let today = now.date_naive();
    let week = match &args.week_id {
        Some(s) => parse_week_id(s, today)?,
        None => WeekId::current(today),
    };

    let view = build_week_view(conn, week, now)?;
    print!("{}", render_week(&view));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::storage::{self, Punch, PunchKind, punch_from_local};
    use crate::week::WeekData;
    use crate::week_target;
    use chrono::{NaiveTime, TimeZone};
    use chrono_tz::UTC as TZ_UTC;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("test fixture is a real date")
    }

    fn t(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).expect("test fixture is a real time")
    }

    fn wk(y: i32, w: u32) -> WeekId {
        WeekId::new(y, w, "test").expect("test fixture is a real ISO week")
    }

    fn row(weekday: &'static str, date: NaiveDate, minutes: i64) -> WeekRow {
        WeekRow {
            weekday_abbrev: weekday,
            date_display: format_date(date),
            minutes,
            is_ongoing: false,
            has_anomaly: false,
            has_overnight: false,
        }
    }

    // ---- Group A fixtures: SPEC.md §7.2 worked examples ----

    /// §7.2 example A: current/ongoing week, `now` 2026-02-12 (Thu) 18:00.
    fn example_a() -> WeekView {
        let mut rows = [
            row("Mon", d(2026, 2, 9), 490),
            row("Tue", d(2026, 2, 10), 470),
            row("Wed", d(2026, 2, 11), 480),
            row("Thu", d(2026, 2, 12), 445),
            row("Fri", d(2026, 2, 13), 0),
            row("Sat", d(2026, 2, 14), 0),
            row("Sun", d(2026, 2, 15), 0),
        ];
        rows[3].is_ongoing = true;

        WeekView {
            week_id_display: "2026-07".to_string(),
            span_start_display: "2026-02-09".to_string(),
            span_end_display: "2026-02-15".to_string(),
            headline: "02h 45m left to 32h 00m by end of Thursday".to_string(),
            rows,
            carry_in_minutes: -130,
            worked_minutes: 1885,
            fulfillment_minutes: 1755,
            target_minutes: 2400,
            carry_marker: None,
            overnight_last_week: None,
        }
    }

    /// §7.2 example B: past/closed week.
    fn example_b() -> WeekView {
        let rows = [
            row("Mon", d(2026, 2, 2), 450),
            row("Tue", d(2026, 2, 3), 480),
            row("Wed", d(2026, 2, 4), 465),
            row("Thu", d(2026, 2, 5), 490),
            row("Fri", d(2026, 2, 6), 325),
            row("Sat", d(2026, 2, 7), 0),
            row("Sun", d(2026, 2, 8), 0),
        ];

        WeekView {
            week_id_display: "2026-06".to_string(),
            span_start_display: "2026-02-02".to_string(),
            span_end_display: "2026-02-08".to_string(),
            headline: "Total behind: 03h 10m".to_string(),
            rows,
            carry_in_minutes: 0,
            worked_minutes: 2210,
            fulfillment_minutes: 2210,
            target_minutes: 2400,
            carry_marker: None,
            overnight_last_week: None,
        }
    }

    const EXAMPLE_A_GOLDEN: &str = "\
Week 2026-07 (2026-02-09 - 2026-02-15)

02h 45m left to 32h 00m by end of Thursday

  Mon 2026-02-09   08h 10m
  Tue 2026-02-10   07h 50m
  Wed 2026-02-11   08h 00m
  Thu 2026-02-12   07h 25m (incl. ongoing)
  Fri 2026-02-13   00h 00m
  Sat 2026-02-14   00h 00m
  Sun 2026-02-15   00h 00m

Carry-in:      -02h 10m
Worked:        31h 25m
Fulfillment:   29h 15m
Target:        40h 00m
";

    const EXAMPLE_B_GOLDEN: &str = "\
Week 2026-06 (2026-02-02 - 2026-02-08)

Total behind: 03h 10m

  Mon 2026-02-02   07h 30m
  Tue 2026-02-03   08h 00m
  Wed 2026-02-04   07h 45m
  Thu 2026-02-05   08h 10m
  Fri 2026-02-06   05h 25m
  Sat 2026-02-07   00h 00m
  Sun 2026-02-08   00h 00m

Carry-in:      00h 00m
Worked:        36h 50m
Fulfillment:   36h 50m
Target:        40h 00m
";

    // A1
    #[test]
    fn a1_current_week_golden_byte_exact() {
        assert_eq!(render_week(&example_a()), EXAMPLE_A_GOLDEN);
    }

    // A2
    #[test]
    fn a2_ongoing_marker_only_on_thursday_row() {
        let out = render_week(&example_a());
        let lines: Vec<&str> = out.lines().collect();
        let ongoing_lines: Vec<&&str> = lines
            .iter()
            .filter(|l| l.contains("(incl. ongoing)"))
            .collect();
        assert_eq!(ongoing_lines.len(), 1);
        assert!(ongoing_lines[0].contains("Thu 2026-02-12"));
    }

    // A3
    #[test]
    fn a3_past_week_golden_full_table_not_a_one_liner() {
        let out = render_week(&example_b());
        assert_eq!(out, EXAMPLE_B_GOLDEN);
        for date in [
            "2026-02-02",
            "2026-02-03",
            "2026-02-04",
            "2026-02-05",
            "2026-02-06",
            "2026-02-07",
            "2026-02-08",
        ] {
            assert!(out.contains(date), "missing row for {date}");
        }
        for label in ["Carry-in:", "Worked:", "Fulfillment:", "Target:"] {
            assert!(out.contains(label), "missing trailing label {label}");
        }
    }

    // A4
    #[test]
    fn a4_future_week_all_zero_rows_full_block_present() {
        let rows = [
            row("Mon", d(2026, 3, 2), 0),
            row("Tue", d(2026, 3, 3), 0),
            row("Wed", d(2026, 3, 4), 0),
            row("Thu", d(2026, 3, 5), 0),
            row("Fri", d(2026, 3, 6), 0),
            row("Sat", d(2026, 3, 7), 0),
            row("Sun", d(2026, 3, 8), 0),
        ];
        let view = WeekView {
            week_id_display: "2026-10".to_string(),
            span_start_display: "2026-03-02".to_string(),
            span_end_display: "2026-03-08".to_string(),
            headline: "Total behind: 40h 00m".to_string(),
            rows,
            carry_in_minutes: 0,
            worked_minutes: 0,
            fulfillment_minutes: 0,
            target_minutes: 2400,
            carry_marker: None,
            overnight_last_week: None,
        };
        let out = render_week(&view);
        assert!(!out.contains("ongoing"));
        assert!(out.contains("Target:        40h 00m"));
        assert_eq!(out.lines().count(), 16);
    }

    // A5 (E13, render half)
    #[test]
    fn a5_never_touched_week_seven_zero_rows() {
        let rows = std::array::from_fn(|i| row(WEEKDAY_ABBREVS[i], d(2026, 5, 4 + i as u32), 0));
        let view = WeekView {
            week_id_display: "2026-20".to_string(),
            span_start_display: "2026-05-04".to_string(),
            span_end_display: "2026-05-10".to_string(),
            headline: "Total behind: 43h 12m".to_string(),
            rows,
            carry_in_minutes: -1872,
            worked_minutes: 0,
            fulfillment_minutes: -1872,
            target_minutes: 2400,
            carry_marker: None,
            overnight_last_week: None,
        };
        let out = render_week(&view);
        let zero_rows = out
            .lines()
            .filter(|l| l.starts_with("  ") && l.contains("00h 00m"))
            .count();
        assert_eq!(zero_rows, 7);
    }

    // A6
    #[test]
    fn a6_anomaly_marker_present_only_on_flagged_row() {
        let mut view = example_b();
        view.rows[2].has_anomaly = true; // Wed
        let out = render_week(&view);
        // Row markers only -- the anomaly-present footer (§3.5) also
        // contains "[!]" now, so this filters to date-row lines specifically.
        let marker_lines: Vec<&str> = out
            .lines()
            .filter(|l| l.starts_with("  ") && l.contains("[!]"))
            .collect();
        assert_eq!(marker_lines.len(), 1);
        assert_eq!(marker_lines[0], "  Wed 2026-02-04   07h 45m  [!]");
    }

    // A7
    #[test]
    fn a7_ongoing_and_anomaly_ordering() {
        let mut view = example_a();
        view.rows[3].has_anomaly = true; // Thu, already ongoing
        let out = render_week(&view);
        assert!(out.contains("07h 25m (incl. ongoing)  [!]"));
    }

    // A8
    #[test]
    fn a8_negative_carry_in_and_fulfillment_formatting() {
        let out = render_week(&example_a());
        assert!(out.contains("Carry-in:      -02h 10m"));

        let mut view = example_a();
        view.fulfillment_minutes = -50;
        let out2 = render_week(&view);
        assert!(out2.contains("Fulfillment:   -00h 50m"));
    }

    // A9
    #[test]
    fn a9_default_target_display() {
        let out = render_week(&example_a());
        assert!(out.contains("Target:        40h 00m"));
    }

    // A10
    #[test]
    fn a10_zero_target_renders_explicitly() {
        let mut view = example_a();
        view.target_minutes = 0;
        let out = render_week(&view);
        assert!(out.contains("Target:        00h 00m"));
    }

    // A11
    #[test]
    fn a11_plain_ascii_audit() {
        for out in [
            render_week(&example_a()),
            render_week(&example_b()),
            {
                let mut v = example_b();
                v.rows[2].has_anomaly = true;
                render_week(&v)
            },
            {
                let mut v = example_a();
                v.rows[3].has_anomaly = true;
                render_week(&v)
            },
        ] {
            for b in out.bytes() {
                assert!(
                    b == b'\n' || (0x20..=0x7E).contains(&b),
                    "non-ASCII-printable byte {b:#x} in {out:?}"
                );
            }
            assert!(!out.contains('\u{2013}'));
            assert!(!out.contains('\u{2014}'));
            assert!(!out.contains('\u{2500}'));
        }
    }

    // A12
    #[test]
    fn a12_no_trailing_whitespace_exact_line_count() {
        for out in [render_week(&example_a()), render_week(&example_b())] {
            assert!(out.ends_with('\n'));
            assert_eq!(out.matches('\n').count(), 16);
            let lines: Vec<&str> = out.split('\n').collect();
            // split on the final trailing \n leaves one empty tail element.
            assert_eq!(lines.len(), 17);
            assert_eq!(lines[16], "");
            for line in &lines[..16] {
                assert_eq!(*line, line.trim_end(), "trailing whitespace in {line:?}");
            }
            assert_eq!(lines[1], "");
            assert_eq!(lines[3], "");
            assert_eq!(lines[11], "");
        }
    }

    // A15: footer absent on the anomaly-free golden examples.
    #[test]
    fn a15_footer_absent_when_no_row_anomalous() {
        for out in [render_week(&example_a()), render_week(&example_b())] {
            assert!(!out.contains(render::WEEK_ANOMALY_FOOTER));
        }
    }

    // A16: footer present exactly once when any row is marked.
    #[test]
    fn a16_footer_present_when_any_row_anomalous() {
        let mut view = example_b();
        view.rows[2].has_anomaly = true;
        let out = render_week(&view);
        assert_eq!(out.matches(render::WEEK_ANOMALY_FOOTER).count(), 1);
    }

    // A17: exact line count on the anomalous path -- 16 (the
    // existing no-anomaly layout) + 2 (footer line + its separating blank).
    #[test]
    fn a17_footer_line_count_when_present() {
        let mut view = example_b();
        view.rows[2].has_anomaly = true;
        let out = render_week(&view);
        assert_eq!(out.matches('\n').count(), 18);
    }

    // Carry marker layouts: the marker line sits in the footer slot above
    // the Carry-in block, and the Carry-in figure gains `  [!]`.
    fn marker(weeks: usize, oldest: WeekId) -> Option<render::CarryMarker> {
        Some(render::CarryMarker { weeks, oldest })
    }

    #[test]
    fn carry_marker_alone_takes_the_footer_slot() {
        let mut view = example_b();
        view.carry_marker = marker(2, wk(2026, 38));
        let want = "\
Week 2026-06 (2026-02-02 - 2026-02-08)

Total behind: 03h 10m

  Mon 2026-02-02   07h 30m
  Tue 2026-02-03   08h 00m
  Wed 2026-02-04   07h 45m
  Thu 2026-02-05   08h 10m
  Fri 2026-02-06   05h 25m
  Sat 2026-02-07   00h 00m
  Sun 2026-02-08   00h 00m

[!] carry-in includes anomalies from 2 earlier weeks, oldest 2026-38 (see mlm week 2026-38)

Carry-in:      00h 00m  [!]
Worked:        36h 50m
Fulfillment:   36h 50m
Target:        40h 00m
";
        assert_eq!(render_week(&view), want);
    }

    #[test]
    fn carry_marker_sits_directly_under_the_footer() {
        let mut view = example_b();
        view.rows[2].has_anomaly = true;
        view.carry_marker = marker(2, wk(2026, 38));
        let want = "\
Week 2026-06 (2026-02-02 - 2026-02-08)

Total behind: 03h 10m

  Mon 2026-02-02   07h 30m
  Tue 2026-02-03   08h 00m
  Wed 2026-02-04   07h 45m  [!]
  Thu 2026-02-05   08h 10m
  Fri 2026-02-06   05h 25m
  Sat 2026-02-07   00h 00m
  Sun 2026-02-08   00h 00m

[!] needs a look: run mlm status DATE for details and fixes
[!] carry-in includes anomalies from 2 earlier weeks, oldest 2026-38 (see mlm week 2026-38)

Carry-in:      00h 00m  [!]
Worked:        36h 50m
Fulfillment:   36h 50m
Target:        40h 00m
";
        assert_eq!(render_week(&view), want);
    }

    #[test]
    fn carry_marker_one_week_form() {
        let mut view = example_b();
        view.carry_marker = marker(1, wk(2026, 38));
        let out = render_week(&view);
        assert!(out.contains(
            "\n[!] carry-in includes anomalies from 1 earlier week, 2026-38 (see mlm week 2026-38)\n\nCarry-in:      00h 00m  [!]\n"
        ));
    }

    #[test]
    fn no_carry_marker_leaves_the_output_unchanged() {
        let mut view = example_b();
        view.carry_marker = None;
        assert_eq!(render_week(&view), EXAMPLE_B_GOLDEN);
        assert!(!render_week(&view).contains("carry-in includes"));
    }

    // A13
    #[test]
    fn a13_headline_passthrough_verbatim() {
        let mut view = example_a();
        view.headline = "XYZZY".to_string();
        let out = render_week(&view);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[2], "XYZZY");
    }

    // A14
    #[test]
    fn a14_row_ordering_is_mon_to_sun() {
        let out = render_week(&example_a());
        let day_lines: Vec<&str> = out
            .lines()
            .filter(|l| l.starts_with("  "))
            .filter(|l| {
                WEEKDAY_ABBREVS
                    .iter()
                    .any(|a| l.trim_start().starts_with(a))
            })
            .collect();
        assert_eq!(day_lines.len(), 7);
        for (i, abbr) in WEEKDAY_ABBREVS.iter().enumerate() {
            assert!(day_lines[i].trim_start().starts_with(abbr), "row {i}");
        }
    }

    // ---- Group B: rollup construction (contract 4) ----

    fn week_dates() -> [NaiveDate; 7] {
        wk(2026, 7).dates()
    }

    fn punch(id: i64, date: NaiveDate, hhmm: (u32, u32), kind: PunchKind) -> Punch {
        punch_from_local(id, kind, date, t(hhmm.0, hhmm.1), &TZ_UTC).expect("valid fixture punch")
    }

    fn now_utc(y: i32, m: u32, d: u32, h: u32, mi: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, h, mi, 0).unwrap()
    }

    // B1
    #[test]
    fn b1_seven_rows_always_zero_punches() {
        let rows = build_rows(week_dates(), &[], d(2026, 2, 9), now_utc(2026, 2, 9, 12, 0));
        assert_eq!(rows.len(), 7);
        for r in &rows {
            assert_eq!(r.minutes, 0);
            assert!(!r.is_ongoing);
            assert!(!r.has_anomaly);
        }
    }

    // B2 (superseded E15 -- now the ordinary midnight-splice case,
    // boundary-stint-pairing spec §4.3)
    #[test]
    fn b2_bucketing_now_splices_across_midnight() {
        use PunchKind::{End, Start};
        let tue = d(2026, 2, 10);
        let wed = d(2026, 2, 11);
        let punches = vec![punch(1, tue, (23, 30), Start), punch(2, wed, (0, 45), End)];
        let rows = build_rows(
            week_dates(),
            &punches,
            d(2026, 2, 9),
            now_utc(2026, 2, 11, 1, 0),
        );
        let tue_row = &rows[1];
        let wed_row = &rows[2];
        assert_eq!(
            tue_row.minutes, 75,
            "23:30-00:45 spans midnight, lands on Tue"
        );
        assert!(!tue_row.has_anomaly, "clean splice, no anomaly");
        assert_eq!(wed_row.minutes, 0, "the spliced stint never appears on Wed");
        assert!(!wed_row.has_anomaly, "the orphan is consumed by the splice");
    }

    // B9: a splice landing exactly on the week's own boundary (Sunday ->
    // next Monday) still lands on the correct row, with no 8th row to
    // accidentally leak into.
    #[test]
    fn b9_week_boundary_splice_lands_in_correct_weeks_rows_only() {
        use PunchKind::{End, Start};
        let sun = d(2026, 2, 15);
        let next_mon = d(2026, 2, 16);
        let punches = vec![
            punch(1, sun, (23, 0), Start),
            punch(2, next_mon, (0, 20), End),
        ];
        let rows = build_rows(
            week_dates(),
            &punches,
            d(2026, 2, 9),
            now_utc(2026, 2, 16, 1, 0),
        );
        assert_eq!(rows.len(), 7);
        let sun_row = &rows[6];
        assert_eq!(
            sun_row.minutes, 80,
            "23:00-00:20 spans midnight, lands on Sun"
        );
        assert!(!sun_row.has_anomaly);
    }

    // B3: today's row adds the running stint's live minutes.
    #[test]
    fn b3_today_row_includes_live_minutes() {
        use PunchKind::{End, Start};
        let today = d(2026, 2, 12);
        let punches = vec![
            punch(1, today, (9, 0), Start),
            punch(2, today, (12, 0), End),
            punch(3, today, (13, 0), Start),
            punch(4, today, (17, 0), End),
            punch(5, today, (17, 45), Start),
        ];
        let rows = build_rows(week_dates(), &punches, today, now_utc(2026, 2, 12, 18, 0));
        let thu = &rows[3];
        assert_eq!(thu.minutes, 180 + 240 + 15, "17:45 to 18:00 is live");
        assert!(thu.is_ongoing);
    }

    // B3b: a past date's dangling start gets no live credit.
    #[test]
    fn b3b_past_date_open_start_row_gets_no_live() {
        use PunchKind::Start;
        let wed = d(2026, 2, 11);
        let punches = vec![punch(1, wed, (9, 0), Start)];
        let rows = build_rows(
            week_dates(),
            &punches,
            d(2026, 2, 12),
            now_utc(2026, 2, 12, 18, 0),
        );
        assert_eq!(rows[2].minutes, 0);
        assert!(!rows[2].is_ongoing);
    }

    // B4
    #[test]
    fn b4_orphaned_end_contributes_nothing() {
        use PunchKind::{End, Start};
        let day = d(2026, 2, 11);
        let punches = vec![
            punch(1, day, (9, 0), Start),
            punch(2, day, (12, 0), End),
            punch(3, day, (18, 0), End),
        ];
        let rows = build_rows(
            week_dates(),
            &punches,
            d(2026, 2, 9),
            now_utc(2026, 2, 11, 19, 0),
        );
        let wed = &rows[2];
        assert_eq!(wed.minutes, 180);
        assert!(wed.has_anomaly);
    }

    // B5 (Risk R2)
    #[test]
    fn b5_is_ongoing_false_on_past_date_with_dangling_start() {
        use PunchKind::Start;
        let past_week = wk(2026, 2).dates();
        let mon = past_week[0];
        let punches = vec![punch(1, mon, (9, 0), Start)];
        // "today" (now's local date) is well after this past week.
        let rows = build_rows(
            past_week,
            &punches,
            d(2026, 2, 12),
            now_utc(2026, 2, 12, 18, 0),
        );
        assert!(!rows[0].is_ongoing);
        assert_eq!(rows[0].minutes, 0);
    }

    // B6 (E7)
    #[test]
    fn b6_multi_open_flags_the_row() {
        use PunchKind::Start;
        let today = d(2026, 2, 12);
        let punches = vec![
            punch(1, today, (9, 0), Start),
            punch(2, today, (11, 0), Start),
        ];
        let rows = build_rows(week_dates(), &punches, today, now_utc(2026, 2, 12, 12, 0));
        let thu = &rows[3];
        assert!(thu.has_anomaly);
        assert!(thu.is_ongoing);
        assert_eq!(thu.minutes, 60, "only the most recent open start is live");
    }

    // B7 (E14)
    #[test]
    fn b7_zero_length_stint_no_anomaly() {
        use PunchKind::{End, Start};
        let day = d(2026, 2, 9);
        let punches = vec![punch(1, day, (9, 0), Start), punch(2, day, (9, 0), End)];
        let rows = build_rows(
            week_dates(),
            &punches,
            d(2026, 2, 9),
            now_utc(2026, 2, 9, 12, 0),
        );
        assert_eq!(rows[0].minutes, 0);
        assert!(!rows[0].has_anomaly);
    }

    // B10: a lone open start at age >= 2 gets `[!]`, and no other
    // row does.
    #[test]
    fn b10_stale_open_start_row_is_marked() {
        use PunchKind::Start;
        let today = d(2026, 2, 12); // Thu, week's own day-4
        let mon = week_dates()[0]; // 2026-02-09, age 3 from `today`
        let punches = vec![punch(1, mon, (9, 0), Start)];
        let rows = build_rows(week_dates(), &punches, today, now_utc(2026, 2, 12, 18, 0));
        assert!(rows[0].has_anomaly, "stale open start at age 3 must mark");
        for (i, r) in rows.iter().enumerate().skip(1) {
            assert!(!r.has_anomaly, "unexpected anomaly on row {i}");
        }
    }

    // B11: a lone open start at age 1 (yesterday) must NOT be
    // marked -- the stale-open-start threshold starts at age 2.
    #[test]
    fn b11_stale_open_start_at_age_one_not_marked() {
        use PunchKind::Start;
        let today = d(2026, 2, 12); // Thu
        let wed = week_dates()[2]; // 2026-02-11, age 1 from `today`
        let punches = vec![punch(1, wed, (9, 0), Start)];
        let rows = build_rows(week_dates(), &punches, today, now_utc(2026, 2, 12, 18, 0));
        assert!(!rows[2].has_anomaly, "age 1 must not be marked stale");
    }

    // B12: a refused join between two dates both inside the
    // displayed week marks both rows. Shape: one open start on Monday, an
    // ordinary start/end pair on Tuesday followed by an orphaned end whose
    // punch is not Tuesday's chronologically-first one -- `splice_candidate`
    // rejects on the "first punch" check.
    #[test]
    fn b12_refused_join_both_rows_marked_same_week() {
        use PunchKind::{End, Start};
        let mon = week_dates()[0];
        let tue = week_dates()[1];
        let punches = vec![
            punch(1, mon, (23, 0), Start),
            punch(2, tue, (9, 0), Start),
            punch(3, tue, (10, 0), End),
            punch(4, tue, (13, 0), End),
        ];
        let rows = build_rows(
            week_dates(),
            &punches,
            d(2026, 2, 9),
            now_utc(2026, 2, 10, 14, 0),
        );
        assert!(rows[0].has_anomaly, "Monday's refused-join-with-next");
        assert!(rows[1].has_anomaly, "Tuesday's refused-join-with-previous");
        for (i, r) in rows.iter().enumerate() {
            if i != 0 && i != 1 {
                assert!(!r.has_anomaly, "unexpected anomaly on row {i}");
            }
        }
    }

    // B13: a refused join across the week's own trailing
    // boundary (Sunday <-> the following Monday, outside the displayed
    // week) still marks Sunday's row, even though the other date never
    // appears among `rows`.
    #[test]
    fn b13_refused_join_across_week_boundary_still_marks_boundary_row() {
        use PunchKind::{End, Start};
        let sun = week_dates()[6];
        let next_mon = sun.succ_opt().unwrap();
        let punches = vec![
            punch(1, sun, (23, 0), Start),
            punch(2, next_mon, (9, 0), Start),
            punch(3, next_mon, (10, 0), End),
            punch(4, next_mon, (13, 0), End),
        ];
        let rows = build_rows(
            week_dates(),
            &punches,
            d(2026, 2, 9),
            now_utc(2026, 2, 16, 14, 0),
        );
        assert!(rows[6].has_anomaly, "Sunday's refused-join-with-next");
        for (i, r) in rows.iter().enumerate().take(6) {
            assert!(!r.has_anomaly, "unexpected anomaly on row {i}");
        }
    }

    // B8 (Risk R3)
    #[test]
    fn b8_rows_sum_to_week_accounting_worked() {
        use PunchKind::{End, Start};
        let conn = new_test_db();
        let week = wk(2026, 7);
        let dates = week.dates();
        storage::insert_punch(&conn, Start, dates[0], t(9, 0), &TZ_UTC).unwrap();
        storage::insert_punch(&conn, End, dates[0], t(17, 0), &TZ_UTC).unwrap();
        storage::insert_punch(&conn, Start, dates[2], t(8, 0), &TZ_UTC).unwrap();
        storage::insert_punch(&conn, End, dates[2], t(16, 30), &TZ_UTC).unwrap();

        // A running stint on Sunday, the current day: 18:00 to 20:00 live.
        storage::insert_punch(&conn, Start, dates[6], t(18, 0), &TZ_UTC).unwrap();

        let punches = storage::punches_in_range(&conn, dates[0], dates[6]).unwrap();
        let now = now_utc(2026, 2, 15, 20, 0);
        let rows = build_rows(dates, &punches, d(2026, 2, 15), now);
        let row_sum: i64 = rows.iter().map(|r| r.minutes).sum();

        let history = history::load_history(&conn, week, d(2026, 2, 15), now).unwrap();
        assert_eq!(row_sum, history.accounting(week).worked);
        assert_eq!(row_sum, 480 + 510 + 120);
    }

    // B8b: a week that does not contain today gets no live, even through
    // the same data source.
    #[test]
    fn b8b_other_week_worked_gets_no_live() {
        use PunchKind::Start;
        let conn = new_test_db();
        storage::insert_punch(&conn, Start, d(2026, 2, 15), t(18, 0), &TZ_UTC).unwrap();
        let history = history::load_history(
            &conn,
            wk(2026, 8),
            d(2026, 2, 15),
            now_utc(2026, 2, 15, 20, 0),
        )
        .unwrap();
        assert_eq!(history.ledger.worked_minutes(wk(2026, 7)), 120);
        assert_eq!(history.ledger.worked_minutes(wk(2026, 6)), 0);
        assert_eq!(history.ledger.worked_minutes(wk(2026, 8)), 0);
    }

    // Any failed read is a hard error, never a silent default.
    #[test]
    fn build_week_view_errors_when_week_targets_is_unreadable() {
        let conn = new_test_db();
        conn.execute("DROP TABLE week_targets", []).unwrap();
        let err = build_week_view(&conn, wk(2026, 7), fixed_now());
        assert!(err.is_err());
    }

    #[test]
    fn build_week_view_errors_when_notes_is_unreadable() {
        let conn = new_test_db();
        conn.execute("DROP TABLE notes", []).unwrap();
        let err = build_week_view(&conn, wk(2026, 7), fixed_now());
        assert!(err.is_err());
    }

    #[test]
    fn build_week_view_has_no_carry_marker_while_the_scan_is_a_stub() {
        let conn = new_test_db();
        let view = build_week_view(&conn, wk(2026, 7), fixed_now()).unwrap();
        assert_eq!(view.carry_marker, None);
    }

    // ---- Group C: resolver / end-to-end ----

    fn new_test_db() -> Connection {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");
        db::apply_migrations(&mut conn).expect("apply migrations");
        conn
    }

    fn fixed_now() -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 2, 12, 18, 0, 0).unwrap()
    }

    type HourMinute = (u32, u32);

    fn seed_week(
        conn: &Connection,
        week: WeekId,
        spans: &[(HourMinute, HourMinute)],
        day_index: usize,
    ) {
        let date = week.dates()[day_index];
        for (start, end) in spans {
            storage::insert_punch(conn, PunchKind::Start, date, t(start.0, start.1), &TZ_UTC)
                .unwrap();
            storage::insert_punch(conn, PunchKind::End, date, t(end.0, end.1), &TZ_UTC).unwrap();
        }
    }

    // C1
    #[test]
    fn c1_no_week_id_defaults_to_current_week() {
        let conn = new_test_db();
        let view = build_week_view(
            &conn,
            WeekId::current(fixed_now().date_naive()),
            fixed_now(),
        )
        .unwrap();
        assert_eq!(view.week_id_display, "2026-07");
        let out = render_week(&view);
        assert!(out.starts_with("Week 2026-07 (2026-02-09 - 2026-02-15)\n"));
    }

    // C2 (F6)
    #[test]
    fn c2_bare_week_number_defaults_to_current_year() {
        let conn = new_test_db();
        let by_bare = parse_week_id("7", fixed_now().date_naive()).unwrap();
        let by_full = parse_week_id("2026-07", fixed_now().date_naive()).unwrap();
        assert_eq!(by_bare, by_full);
        let view = build_week_view(&conn, by_bare, fixed_now()).unwrap();
        assert_eq!(view.week_id_display, "2026-07");
    }

    // C3
    #[test]
    fn c3_unpadded_full_id_matches_padded() {
        let conn = new_test_db();
        let a = parse_week_id("2026-7", fixed_now().date_naive()).unwrap();
        let b = parse_week_id("2026-07", fixed_now().date_naive()).unwrap();
        let view_a = build_week_view(&conn, a, fixed_now()).unwrap();
        let view_b = build_week_view(&conn, b, fixed_now()).unwrap();
        assert_eq!(render_week(&view_a), render_week(&view_b));
    }

    // C4 (E3)
    #[test]
    fn c4_malformed_week_id_is_a_hard_error() {
        let conn = new_test_db();
        let today = fixed_now();
        for bad in ["0", "abcd", "2027-53"] {
            let args = WeekArgs {
                action: None,
                week_id: Some(bad.to_string()),
            };
            let result = run(&conn, today, &args);
            assert!(result.is_err(), "expected error for {bad}");
        }
    }

    // C5 (F6)
    #[test]
    fn c5_default_target_end_to_end() {
        let conn = new_test_db();
        let view = build_week_view(&conn, wk(2026, 7), fixed_now()).unwrap();
        assert_eq!(view.target_minutes, 2400);
    }

    // C6 (F7)
    #[test]
    fn c6_target_override_end_to_end() {
        let conn = new_test_db();
        week_target::set_week_target(&conn, &wk(2026, 7), 2010).unwrap();
        let view = build_week_view(&conn, wk(2026, 7), fixed_now()).unwrap();
        assert_eq!(view.target_minutes, 2010);
        let out = render_week(&view);
        assert!(out.contains("Target:        33h 30m"));
    }

    // C7 (E13)
    #[test]
    fn c7_never_touched_week_carries_forward() {
        let conn = new_test_db();
        seed_week(&conn, wk(2026, 1), &[((9, 0), (17, 0))], 0);
        let view = build_week_view(&conn, wk(2026, 6), fixed_now()).unwrap();
        for r in &view.rows {
            assert_eq!(r.minutes, 0);
        }
        assert_eq!(view.target_minutes, 2400);
        assert_ne!(view.carry_in_minutes, 0);
        // Not week 1's own carry-out in isolation; it is the accumulated
        // deficit of every intervening idle week (2..=5), four weeks'
        // worth of a 480-minute-short 2026-01 plus three fully-idle weeks.
        let week1_carry_out = -(2400 - 480);
        assert_ne!(view.carry_in_minutes, week1_carry_out);
    }

    // C8 (F8b)
    #[test]
    fn c8_multi_week_idle_gap_carry_end_to_end() {
        let conn = new_test_db();
        seed_week(&conn, wk(2026, 1), &[((9, 0), (17, 0))], 0); // 480m, week N
        // week N+1 (2026-02) intentionally empty
        seed_week(&conn, wk(2026, 3), &[((9, 0), (17, 0))], 0); // week N+2

        let view_n1 = build_week_view(&conn, wk(2026, 2), fixed_now()).unwrap();
        for r in &view_n1.rows {
            assert_eq!(r.minutes, 0);
        }
        assert_eq!(view_n1.carry_in_minutes, -(2400 - 480));

        let view_n2 = build_week_view(&conn, wk(2026, 3), fixed_now()).unwrap();
        // Full deficit of the idle week N+1 must be visible: carry_in of
        // N+2 is week N+1's own carry_out, not a naive skip.
        assert_eq!(view_n2.carry_in_minutes, view_n1.carry_in_minutes - 2400);
    }

    // C9
    #[test]
    fn c9_anomaly_marker_end_to_end_and_exit_zero() {
        let conn = new_test_db();
        let week = wk(2026, 7);
        let wed = week.dates()[2];
        storage::insert_punch(&conn, PunchKind::End, wed, t(18, 0), &TZ_UTC).unwrap();

        let args = WeekArgs {
            action: None,
            week_id: Some("2026-07".to_string()),
        };
        let result = run(&conn, fixed_now(), &args);
        assert!(result.is_ok());
        assert_eq!(crate::exit_code(&result), 0);

        let view = build_week_view(&conn, week, fixed_now()).unwrap();
        assert!(view.rows[2].has_anomaly);
        for (i, r) in view.rows.iter().enumerate() {
            if i != 2 {
                assert!(!r.has_anomaly, "unexpected anomaly on row {i}");
            }
        }
    }

    // C13: a splice landing exactly on a week boundary attributes minutes
    // to the starting week's view only, end to end through `build_week_view`
    // (real `punches_in_range`-widened fetch, not just the unit-level
    // `build_rows` call `b9` above already covers).
    #[test]
    fn c13_week_boundary_splice_end_to_end_lands_in_starting_weeks_view_only() {
        let conn = new_test_db();
        let week7 = wk(2026, 7); // Mon 2026-02-09 .. Sun 2026-02-15
        let week8 = wk(2026, 8); // Mon 2026-02-16 .. Sun 2026-02-22
        let sun = week7.dates()[6];
        let next_mon = week8.dates()[0];
        storage::insert_punch(&conn, PunchKind::Start, sun, t(23, 0), &TZ_UTC).unwrap();
        storage::insert_punch(&conn, PunchKind::End, next_mon, t(0, 20), &TZ_UTC).unwrap();

        let now = now_utc(2026, 2, 16, 1, 0);
        let view7 =
            build_week_view(&conn, week7, Local.from_utc_datetime(&now.naive_utc())).unwrap();
        assert_eq!(
            view7.rows[6].minutes, 80,
            "Sunday row carries the spliced minutes"
        );
        assert!(!view7.rows[6].has_anomaly);
        assert_eq!(view7.worked_minutes, 80);

        let view8 =
            build_week_view(&conn, week8, Local.from_utc_datetime(&now.naive_utc())).unwrap();
        assert_eq!(
            view8.rows[0].minutes, 0,
            "the spliced stint never lands on next Monday"
        );
        assert!(
            !view8.rows[0].has_anomaly,
            "the orphan is consumed by the splice"
        );
        assert_eq!(view8.worked_minutes, 0);
    }

    // C14: `history::load_history` padding-day isolation -- a real,
    // unrelated (non-spliced) stint on the calendar date just outside the
    // week's own 7 dates, but inside the loader's widened fetch, must
    // appear in only its own week's figures, never leaked into or
    // double-counted with a neighboring week. `c7`/`c8` above seed only
    // fully-idle padding weeks, so only this fixture can tell the two
    // apart.
    #[test]
    fn c14_worked_minutes_padding_day_isolation() {
        let conn = new_test_db();
        let week = wk(2026, 7); // Mon 2026-02-09 .. Sun 2026-02-15
        let dates = week.dates();

        // An ordinary, unrelated, fully self-contained stint on the
        // padding day immediately BEFORE the week's own span (outside
        // `week`'s own 7 dates, but inside the loader's widened fetch).
        // No splice involved.
        let padding_before = dates[0].pred_opt().unwrap(); // 2026-02-08
        storage::insert_punch(&conn, PunchKind::Start, padding_before, t(9, 0), &TZ_UTC).unwrap();
        storage::insert_punch(&conn, PunchKind::End, padding_before, t(11, 0), &TZ_UTC).unwrap(); // 120m

        // Same on the padding day immediately AFTER the week's span.
        let padding_after = dates[6].succ_opt().unwrap(); // 2026-02-16
        storage::insert_punch(&conn, PunchKind::Start, padding_after, t(9, 0), &TZ_UTC).unwrap();
        storage::insert_punch(&conn, PunchKind::End, padding_after, t(10, 30), &TZ_UTC).unwrap(); // 90m

        // One ordinary in-week stint too, so the assertion isn't just "0
        // in, 0 out" -- confirms the real week total is exactly the
        // in-week figure, neither padding day's minutes folded in.
        storage::insert_punch(&conn, PunchKind::Start, dates[2], t(9, 0), &TZ_UTC).unwrap(); // Wed
        storage::insert_punch(&conn, PunchKind::End, dates[2], t(17, 0), &TZ_UTC).unwrap(); // 480m

        let now = now_utc(2026, 2, 15, 20, 0);
        let history = history::load_history(&conn, wk(2026, 8), d(2026, 2, 15), now).unwrap();
        let ledger = &history.ledger;
        let this_week = ledger.worked_minutes(week);
        assert_eq!(this_week, 480, "padding-day stints must not leak in");

        // And the mirror check: each padding day's own week sees ITS
        // stint, undiminished and un-doubled.
        let week_before = WeekId::from_date(padding_before); // wk 2026-06
        let week_after = WeekId::from_date(padding_after); // wk 2026-08
        assert_eq!(ledger.worked_minutes(week_before), 120);
        assert_eq!(ledger.worked_minutes(week_after), 90);

        // Total across the three adjacent weeks equals the sum of the
        // three independent stints exactly once each -- the direct
        // anti-double-count assertion.
        assert_eq!(
            ledger.worked_minutes(week_before) + this_week + ledger.worked_minutes(week_after),
            120 + 480 + 90
        );
    }

    // C10
    #[test]
    fn c10_current_vs_closed_headline_end_to_end() {
        let conn = new_test_db();
        let current = build_week_view(&conn, wk(2026, 7), fixed_now()).unwrap();
        assert_eq!(
            current.headline,
            "32h 00m left to 32h 00m by end of Thursday"
        );

        let closed = build_week_view(&conn, wk(2026, 6), fixed_now()).unwrap();
        assert!(!closed.headline.contains("Thursday"));
        assert!(!closed.headline.contains("by end of"));
    }

    // Current week whose fulfillment already exceeds today's required
    // slice: the headline reads the `over` form, never "ahead of target".
    #[test]
    fn c10b_current_week_over_required_end_to_end() {
        let conn = new_test_db();
        week_target::set_week_target(&conn, &wk(2026, 7), 5).unwrap();
        seed_week(&conn, wk(2026, 7), &[((9, 0), (17, 0))], 0);

        let view = build_week_view(&conn, wk(2026, 7), fixed_now()).unwrap();
        // required = floor(5 / 5) * 4 = 4 minutes; fulfillment 480.
        assert_eq!(view.headline, "07h 56m over 00h 04m by end of Thursday");

        let out = render_week(&view);
        assert!(!out.contains("ahead of target"), "{out}");
    }

    // C12
    #[test]
    fn c12_plain_ascii_over_real_output() {
        let conn = new_test_db();
        seed_week(&conn, wk(2026, 1), &[((9, 0), (17, 0))], 0);
        let view = build_week_view(&conn, wk(2026, 7), fixed_now()).unwrap();
        let out = render_week(&view);
        for b in out.bytes() {
            assert!(b == b'\n' || (0x20..=0x7E).contains(&b));
        }
    }

    // ---- point-in-time-week: live-inclusive current week ----

    fn local_punch(conn: &Connection, kind: PunchKind, date: NaiveDate, time: NaiveTime) {
        storage::insert_punch(conn, kind, date, time, &Local).unwrap();
    }

    /// Mon-Wed of 2026-07 plus Thursday's two closed stints (1755 minutes
    /// in all), then a running stint from 17:45.
    fn seed_running_thursday(conn: &Connection) {
        use PunchKind::{End, Start};
        local_punch(conn, Start, d(2026, 2, 9), t(9, 0));
        local_punch(conn, End, d(2026, 2, 9), t(19, 0));
        local_punch(conn, Start, d(2026, 2, 10), t(9, 0));
        local_punch(conn, End, d(2026, 2, 10), t(18, 50));
        local_punch(conn, Start, d(2026, 2, 11), t(9, 0));
        local_punch(conn, End, d(2026, 2, 11), t(11, 0));
        local_punch(conn, Start, d(2026, 2, 12), t(9, 0));
        local_punch(conn, End, d(2026, 2, 12), t(12, 0));
        local_punch(conn, Start, d(2026, 2, 12), t(13, 0));
        local_punch(conn, End, d(2026, 2, 12), t(17, 25));
        local_punch(conn, Start, d(2026, 2, 12), t(17, 45));
    }

    #[test]
    fn pit_current_week_counts_live_in_row_worked_and_fulfillment() {
        let conn = new_test_db();
        seed_running_thursday(&conn);
        let view = build_week_view(&conn, wk(2026, 7), fixed_now()).unwrap();
        // 600 + 590 + 120 + (445 + 15 live).
        assert_eq!(view.worked_minutes, 1770);
        assert_eq!(view.fulfillment_minutes, 1770);
        assert_eq!(view.rows[3].minutes, 460);
        assert!(view.rows[3].is_ongoing);
        let row_sum: i64 = view.rows.iter().map(|r| r.minutes).sum();
        assert_eq!(row_sum, view.worked_minutes);
        assert_eq!(
            view.fulfillment_minutes,
            view.carry_in_minutes + view.worked_minutes
        );
        assert_eq!(view.headline, "02h 30m left to 32h 00m by end of Thursday");
        let out = render_week(&view);
        assert!(
            out.contains("  Thu 2026-02-12   07h 40m (incl. ongoing)\n"),
            "{out}"
        );
        assert!(out.contains("Worked:        29h 30m\n"), "{out}");
        assert!(out.contains("Fulfillment:   29h 30m\n"), "{out}");
    }

    #[test]
    fn pit_week_headline_matches_the_status_week_line_headline() {
        let conn = new_test_db();
        seed_running_thursday(&conn);
        let view = build_week_view(&conn, wk(2026, 7), fixed_now()).unwrap();
        let status = crate::status::resolve(None, fixed_now(), &conn).unwrap();
        assert!(
            status.week_line.contains(&view.headline),
            "{} vs {}",
            status.week_line,
            view.headline
        );
    }

    #[test]
    fn pit_future_week_carry_in_includes_current_week_live() {
        let conn = new_test_db();
        seed_running_thursday(&conn);
        let view = build_week_view(&conn, wk(2026, 8), fixed_now()).unwrap();
        // fulfillment 1770 against a 2400 target carries -630.
        assert_eq!(view.carry_in_minutes, -630);
        assert_eq!(view.worked_minutes, 0);
    }

    #[test]
    fn pit_past_week_output_is_unchanged_by_a_running_stint_today() {
        use PunchKind::{End, Start};
        let with_live = new_test_db();
        let without = new_test_db();
        for conn in [&with_live, &without] {
            local_punch(conn, Start, d(2026, 2, 3), t(9, 0));
            local_punch(conn, End, d(2026, 2, 3), t(17, 0));
        }
        local_punch(&with_live, Start, d(2026, 2, 12), t(17, 45));
        let a = render_week(&build_week_view(&with_live, wk(2026, 6), fixed_now()).unwrap());
        let b = render_week(&build_week_view(&without, wk(2026, 6), fixed_now()).unwrap());
        assert_eq!(a, b);
        assert!(a.contains("Total behind: "), "{a}");
        assert!(!a.contains("incl. ongoing"), "{a}");
    }

    #[test]
    fn pit_yesterday_open_start_earns_no_live() {
        let conn = new_test_db();
        local_punch(&conn, PunchKind::Start, d(2026, 2, 11), t(20, 0));
        let view = build_week_view(&conn, wk(2026, 7), fixed_now()).unwrap();
        assert_eq!(view.worked_minutes, 0);
        assert!(view.rows.iter().all(|r| !r.is_ongoing));
    }

    #[test]
    fn pit_spliced_open_start_earns_no_live() {
        let conn = new_test_db();
        local_punch(&conn, PunchKind::Start, d(2026, 2, 12), t(17, 45));
        local_punch(&conn, PunchKind::End, d(2026, 2, 13), t(0, 30));
        let view = build_week_view(&conn, wk(2026, 7), fixed_now()).unwrap();
        // The spliced stint (17:45 to 00:30) is completed time on Thursday.
        assert_eq!(view.rows[3].minutes, 405);
        assert!(!view.rows[3].is_ongoing);
        assert_eq!(view.worked_minutes, 405);
    }

    // The worked example's `mlm week` block, Thursday 18:05.
    #[test]
    fn pit_worked_example_week_block() {
        use PunchKind::{End, Start};
        let conn = new_test_db();
        for day in 21..=25 {
            local_punch(&conn, Start, d(2026, 9, day), t(6, 0));
            local_punch(&conn, End, d(2026, 9, day), t(15, 23));
        }
        for (day, end) in [(28, t(10, 30)), (29, t(10, 40)), (30, t(10, 30))] {
            local_punch(&conn, Start, d(2026, 9, day), t(6, 0));
            local_punch(&conn, End, d(2026, 9, day), end);
        }
        let thu = d(2026, 10, 1);
        local_punch(&conn, Start, thu, t(9, 0));
        local_punch(&conn, End, thu, t(13, 0));
        local_punch(&conn, Start, thu, t(13, 30));
        local_punch(&conn, End, thu, t(16, 55));
        local_punch(&conn, Start, thu, t(17, 25));
        let now = Local.with_ymd_and_hms(2026, 10, 1, 18, 5, 0).unwrap();

        let view = build_week_view(&conn, wk(2026, 40), now).unwrap();
        let want = "\
Week 2026-40 (2026-09-28 - 2026-10-04)

03h 20m left to 32h 00m by end of Thursday

  Mon 2026-09-28   04h 30m
  Tue 2026-09-29   04h 40m
  Wed 2026-09-30   04h 30m
  Thu 2026-10-01   08h 05m (incl. ongoing)
  Fri 2026-10-02   00h 00m
  Sat 2026-10-03   00h 00m
  Sun 2026-10-04   00h 00m

Carry-in:      06h 55m
Worked:        21h 45m
Fulfillment:   28h 40m
Target:        40h 00m
";
        assert_eq!(render_week(&view), want);
    }

    // ---- overnight stint marker ----

    fn seed_at(conn: &Connection, kind: PunchKind, date: NaiveDate, h: u32, m: u32) {
        storage::insert_punch(conn, kind, date, t(h, m), &Local).expect("insert");
    }

    fn mon_0900() -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 2, 16, 9, 0, 0).unwrap()
    }

    fn overnight_rows(view: &WeekView) -> Vec<usize> {
        (0..7).filter(|&i| view.rows[i].has_overnight).collect()
    }

    #[test]
    fn build_week_view_marks_yesterday_row() {
        let conn = new_test_db();
        seed_at(&conn, PunchKind::Start, d(2026, 2, 11), 18, 2);
        let view = build_week_view(&conn, wk(2026, 7), fixed_now()).unwrap();
        assert_eq!(overnight_rows(&view), vec![2]);
        assert_eq!(view.overnight_last_week, None);
        assert!(view.rows.iter().all(|r| !r.has_anomaly));
        let out = render_week(&view);
        assert!(out.contains("  Wed 2026-02-11   00h 00m  [i]\n"), "{out}");
        let foot = "[i] open stint on Wed 2026-02-11: mlm status shows how to stop it";
        assert_eq!(out.matches(foot).count(), 1, "{out}");
        assert!(!out.contains("needs a look"), "{out}");
    }

    #[test]
    fn past_week_by_id_marks_sunday_row() {
        let conn = new_test_db();
        seed_at(&conn, PunchKind::Start, d(2026, 2, 15), 22, 0);
        let view = build_week_view(&conn, wk(2026, 7), mon_0900()).unwrap();
        assert_eq!(overnight_rows(&view), vec![6]);
        assert_eq!(view.overnight_last_week, None);
        let out = render_week(&view);
        let foot = "[i] open stint on Sun 2026-02-15: mlm status shows how to stop it";
        assert_eq!(out.matches(foot).count(), 1, "{out}");
        assert!(!out.contains("(last week)"), "{out}");
    }

    #[test]
    fn row_mark_follows_the_trigger() {
        type Seeder = Box<dyn Fn(&Connection)>;
        let fixtures: Vec<Seeder> = vec![
            Box::new(|c| {
                seed_at(c, PunchKind::Start, d(2026, 2, 11), 23, 0);
                seed_at(c, PunchKind::End, d(2026, 2, 12), 0, 10);
            }),
            Box::new(|c| {
                seed_at(c, PunchKind::Start, d(2026, 2, 11), 18, 2);
                seed_at(c, PunchKind::End, d(2026, 2, 12), 0, 10);
                seed_at(c, PunchKind::End, d(2026, 2, 12), 0, 20);
            }),
            Box::new(|c| {
                seed_at(c, PunchKind::Start, d(2026, 2, 11), 9, 0);
                seed_at(c, PunchKind::Start, d(2026, 2, 11), 10, 0);
            }),
            Box::new(|c| seed_at(c, PunchKind::Start, d(2026, 2, 10), 9, 0)),
        ];
        for (i, seed) in fixtures.iter().enumerate() {
            let conn = new_test_db();
            seed(&conn);
            let view = build_week_view(&conn, wk(2026, 7), fixed_now()).unwrap();
            assert!(view.rows.iter().all(|r| !r.has_overnight), "fixture {i}");
        }
        let conn = new_test_db();
        seed_at(&conn, PunchKind::Start, d(2026, 2, 10), 9, 0);
        let view = build_week_view(&conn, wk(2026, 7), fixed_now()).unwrap();
        assert!(view.rows[1].has_anomaly);
        let out = render_week(&view);
        assert!(out.contains("  Tue 2026-02-10   00h 00m  [!]\n"), "{out}");
        assert!(!out.contains("[i]"), "{out}");
    }

    #[test]
    fn monday_current_week_uses_loaded_buckets() {
        let conn = new_test_db();
        seed_at(&conn, PunchKind::Start, d(2026, 2, 15), 22, 0);
        let view = build_week_view(&conn, wk(2026, 8), mon_0900()).unwrap();
        assert_eq!(view.overnight_last_week, Some(d(2026, 2, 15)));
        assert!(overnight_rows(&view).is_empty());
    }

    #[test]
    fn monday_footer_names_sunday_once() {
        let conn = new_test_db();
        seed_at(&conn, PunchKind::Start, d(2026, 2, 15), 22, 0);
        let out = render_week(&build_week_view(&conn, wk(2026, 8), mon_0900()).unwrap());
        let line = "[i] open stint on Sun 2026-02-15 (last week): mlm status shows how to stop it";
        assert_eq!(out.matches(line).count(), 1, "{out}");
        assert_eq!(out.matches("[i] open stint").count(), 1, "{out}");
    }

    #[test]
    fn monday_other_weeks_show_nothing() {
        let with = new_test_db();
        let without = new_test_db();
        // An earlier closed pair in both keeps the ledger start equal.
        for conn in [&with, &without] {
            seed_at(conn, PunchKind::Start, d(2026, 2, 2), 9, 0);
            seed_at(conn, PunchKind::End, d(2026, 2, 2), 17, 0);
        }
        seed_at(&with, PunchKind::Start, d(2026, 2, 15), 22, 0);
        for week in [wk(2026, 6), wk(2026, 9)] {
            let a = build_week_view(&with, week, mon_0900()).unwrap();
            let b = build_week_view(&without, week, mon_0900()).unwrap();
            assert_eq!(render_week(&a), render_week(&b), "{week}");
            assert!(overnight_rows(&a).is_empty());
            assert_eq!(a.overnight_last_week, None);
        }
    }

    #[test]
    fn monday_sunday_row_with_anomaly_shows_both_markers() {
        let conn = new_test_db();
        seed_at(&conn, PunchKind::End, d(2026, 2, 15), 8, 0);
        seed_at(&conn, PunchKind::Start, d(2026, 2, 15), 22, 0);
        let current = build_week_view(&conn, wk(2026, 8), mon_0900()).unwrap();
        assert_eq!(current.overnight_last_week, Some(d(2026, 2, 15)));
        let past = render_week(&build_week_view(&conn, wk(2026, 7), mon_0900()).unwrap());
        assert!(
            past.contains("  Sun 2026-02-15   00h 00m  [!] [i]\n"),
            "{past}"
        );
    }

    #[test]
    fn monday_footer_hand_built_golden() {
        let mut view = example_a();
        view.overnight_last_week = Some(d(2026, 2, 15));
        let line = "[i] open stint on Sun 2026-02-15 (last week): mlm status shows how to stop it";
        let want = EXAMPLE_A_GOLDEN.replace("\n\nCarry-in:", &format!("\n\n{line}\n\nCarry-in:"));
        assert_eq!(render_week(&view), want);
    }

    #[test]
    fn shared_row_prints_both_markers_and_footers_in_order() {
        let mut view = example_b();
        view.rows[2].has_anomaly = true;
        view.rows[2].has_overnight = true;
        view.carry_marker = marker(2, wk(2026, 38));
        let want = "\
Week 2026-06 (2026-02-02 - 2026-02-08)

Total behind: 03h 10m

  Mon 2026-02-02   07h 30m
  Tue 2026-02-03   08h 00m
  Wed 2026-02-04   07h 45m  [!] [i]
  Thu 2026-02-05   08h 10m
  Fri 2026-02-06   05h 25m
  Sat 2026-02-07   00h 00m
  Sun 2026-02-08   00h 00m

[!] needs a look: run mlm status DATE for details and fixes
[i] open stint on Wed 2026-02-04: mlm status shows how to stop it
[!] carry-in includes anomalies from 2 earlier weeks, oldest 2026-38 (see mlm week 2026-38)

Carry-in:      00h 00m  [!]
Worked:        36h 50m
Fulfillment:   36h 50m
Target:        40h 00m
";
        assert_eq!(render_week(&view), want);
    }

    #[test]
    fn ongoing_anomaly_and_overnight_marker_order() {
        let mut view = example_a();
        view.rows[3].has_anomaly = true;
        view.rows[3].has_overnight = true;
        assert!(render_week(&view).contains("07h 25m (incl. ongoing)  [!] [i]\n"));
    }

    #[test]
    fn overnight_only_row_prints_no_anomaly_footer() {
        let mut view = example_b();
        view.rows[2].has_overnight = true;
        let out = render_week(&view);
        assert!(!out.contains(render::WEEK_ANOMALY_FOOTER));
        assert!(!out.contains("carry-in includes"));
        assert!(out.contains("Carry-in:      00h 00m\n"));
        assert_eq!(out.lines().count(), 18);
    }

    #[test]
    fn overnight_does_not_change_any_figure() {
        let with = new_test_db();
        let without = new_test_db();
        for conn in [&with, &without] {
            seed_at(conn, PunchKind::Start, d(2026, 2, 9), 9, 0);
            seed_at(conn, PunchKind::End, d(2026, 2, 9), 17, 0);
        }
        seed_at(&with, PunchKind::Start, d(2026, 2, 11), 18, 2);
        let a = build_week_view(&with, wk(2026, 7), fixed_now()).unwrap();
        let b = build_week_view(&without, wk(2026, 7), fixed_now()).unwrap();
        for i in 0..7 {
            assert_eq!(a.rows[i].minutes, b.rows[i].minutes);
            assert_eq!(a.rows[i].is_ongoing, b.rows[i].is_ongoing);
            assert_eq!(a.rows[i].has_anomaly, b.rows[i].has_anomaly);
        }
        assert_eq!(a.carry_in_minutes, b.carry_in_minutes);
        assert_eq!(a.worked_minutes, b.worked_minutes);
        assert_eq!(a.fulfillment_minutes, b.fulfillment_minutes);
        assert_eq!(a.target_minutes, b.target_minutes);
        assert_eq!(a.headline, b.headline);
        assert_eq!(a.carry_marker, b.carry_marker);
    }

    #[test]
    fn blank_line_follows_footer_block_only_when_a_line_printed() {
        for mask in 0..16u32 {
            let anomaly = mask & 1 != 0;
            let overnight = mask & 2 != 0;
            let last_week = mask & 4 != 0;
            let carry = mask & 8 != 0;
            let mut view = example_b();
            view.rows[2].has_anomaly = anomaly;
            view.rows[2].has_overnight = overnight;
            if last_week {
                view.overnight_last_week = Some(d(2026, 2, 1));
            }
            if carry {
                view.carry_marker = marker(2, wk(2026, 38));
            }
            let lines =
                usize::from(anomaly) + usize::from(overnight || last_week) + usize::from(carry);
            let out = render_week(&view);
            let extra = if lines > 0 { lines + 1 } else { 0 };
            assert_eq!(out.matches('\n').count(), 16 + extra, "mask {mask}");
        }
    }
}
