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
//! - `render::week_headline`/`status_week_line` take same-typed adjacent
//!   `i64` minute parameters; every call site here is written and
//!   commented to make the argument identity obvious at the call.
//! - Per-row anomaly detection goes through `render::Anomalies`
//!   exclusively (`has_any()`), never `DayStints::has_anomaly()` /
//!   `DayStints::anomalies()` directly, even though the latter are public
//!   and would compile.

use std::collections::HashMap;

use chrono::{DateTime, Local, NaiveDate, Utc};
use rusqlite::Connection;

use crate::cli::WeekArgs;
use crate::date::{WeekId, format_date, parse_week_id};
use crate::render::{self, Anomalies};
use crate::stint;
use crate::storage::{self, Punch};
use crate::time::format_minutes;
use crate::week::{self, WeekData, WeekTargets};
use crate::week_target;

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
    /// Completed-stint minutes only (§2.4, NOTES 37/38). Always >= 0.
    pub minutes: i64,
    /// `(ongoing)` marker: an open stint AND this date is today.
    pub is_ongoing: bool,
    /// `[!]` marker, via `render::Anomalies::has_any()` exclusively.
    pub has_anomaly: bool,
}

/// Everything `render_week` needs, and nothing it can compute itself
/// (plan §2.1). `owed` is deliberately absent: the headline already
/// states it, so the renderer cannot accidentally re-render it.
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
}

/// The §7.2/§7.3 16-line layout, byte-exact. Pure: no I/O, no clock.
pub fn render_week(view: &WeekView) -> String {
    let mut out = String::new();

    out.push_str(&format!(
        "Week {} ({} - {})\n",
        view.week_id_display, view.span_start_display, view.span_end_display
    ));
    out.push('\n');
    out.push_str(&view.headline);
    out.push('\n');
    out.push('\n');

    for row in &view.rows {
        out.push_str("  ");
        out.push_str(row.weekday_abbrev);
        out.push(' ');
        out.push_str(&row.date_display);
        out.push_str("   ");
        out.push_str(&format_minutes(row.minutes));
        if row.is_ongoing {
            out.push_str(" (ongoing)");
        }
        if row.has_anomaly {
            out.push_str("  [!]");
        }
        out.push('\n');
    }
    out.push('\n');

    for (label, minutes) in [
        ("Carry-in:", view.carry_in_minutes),
        ("Worked:", view.worked_minutes),
        ("Fulfillment:", view.fulfillment_minutes),
        ("Target:", view.target_minutes),
    ] {
        out.push_str(&format!(
            "{:<width$}{}\n",
            label,
            format_minutes(minutes),
            width = LABEL_WIDTH
        ));
    }

    out
}

/// Contract 4: bucket `punches` by their `date` column, then pair each of
/// the week's 7 dates independently via `stint::classify` — one call per
/// date, never a whole week's punches in one call, which is what keeps
/// cross-midnight pairing out of scope (E15).
fn build_rows(
    dates: [NaiveDate; 7],
    punches: &[Punch],
    today: NaiveDate,
    now_utc: DateTime<Utc>,
) -> [WeekRow; 7] {
    let mut by_date: HashMap<NaiveDate, Vec<Punch>> = HashMap::new();
    for p in punches {
        by_date.entry(p.date).or_default().push(*p);
    }

    std::array::from_fn(|i| {
        let date = dates[i];
        let bucket = by_date.get(&date).cloned().unwrap_or_default();
        let day = stint::classify(&bucket, now_utc);

        // Landmine 2: go through `render::Anomalies` exclusively, never
        // `day.has_anomaly()`/`day.anomalies()` directly.
        let anomalies = Anomalies {
            open_stint_count: day.open.len(),
            orphaned_end_times: day
                .orphaned_ends
                .iter()
                .map(|o| o.punch.at_utc.time())
                .collect(),
        };

        WeekRow {
            weekday_abbrev: WEEKDAY_ABBREVS[i],
            date_display: format_date(date),
            minutes: day.completed_minutes(),
            is_ongoing: day.is_ongoing() && date == today,
            has_anomaly: anomalies.has_any(),
        }
    })
}

/// `WeekData`/`WeekTargets` adapter over live storage (contract 4/10's
/// wave-3 integration seam, done here since Milestone 6 declares the
/// traits but not this binding). `now_utc` is injected, never read from a
/// system clock inside this module.
///
/// Both trait methods collapse a storage/DB error to "no data" rather
/// than propagating `Result` (the traits are infallible by design) —
/// acceptable because a DB already opened and migrated successfully by
/// `main` is not expected to fail on a read here; see the milestone
/// report's risk notes.
struct DbWeekData<'a> {
    conn: &'a Connection,
    now_utc: DateTime<Utc>,
}

impl WeekData for DbWeekData<'_> {
    fn earliest_data_week(&self) -> Option<WeekId> {
        storage::earliest_data_date(self.conn)
            .ok()
            .flatten()
            .map(WeekId::from_date)
    }

    fn worked_minutes(&self, week: WeekId) -> i64 {
        let (start, end) = week.span();
        let punches = storage::punches_in_range(self.conn, start, end).unwrap_or_default();
        let mut by_date: HashMap<NaiveDate, Vec<Punch>> = HashMap::new();
        for p in punches {
            by_date.entry(p.date).or_default().push(p);
        }
        by_date
            .values()
            .map(|bucket| stint::classify(bucket, self.now_utc).completed_minutes())
            .sum()
    }
}

impl WeekTargets for DbWeekData<'_> {
    fn target_override(&self, week: WeekId) -> Option<i64> {
        week_target::get_week_target(self.conn, &week)
            .ok()
            .flatten()
    }
}

/// Build the full `WeekView` for `week` (resolution step 2-6 of the plan).
pub fn build_week_view(
    conn: &Connection,
    week: WeekId,
    now: DateTime<Local>,
) -> anyhow::Result<WeekView> {
    let today = now.date_naive();
    let now_utc = now.with_timezone(&Utc);
    let (start, end) = week.span();
    let dates = week.dates();

    let punches = storage::punches_in_range(conn, start, end)?;
    let rows = build_rows(dates, &punches, today, now_utc);

    let data = DbWeekData { conn, now_utc };
    let acct = week::week_accounting(week, &data, &data);

    // `week_headline(week, owed_minutes, today)` — argument order matters
    // (landmine 1): `owed` here, never `fulfillment`/`target`.
    let headline = render::week_headline(week, acct.owed, today);

    Ok(WeekView {
        week_id_display: week.to_string(),
        span_start_display: format_date(start),
        span_end_display: format_date(end),
        headline,
        rows,
        carry_in_minutes: acct.carry_in,
        worked_minutes: acct.worked,
        fulfillment_minutes: acct.fulfillment,
        target_minutes: acct.target,
    })
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
    use crate::storage::{PunchKind, punch_from_local};
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
            headline: "10h 45m left by end of Thursday".to_string(),
            rows,
            carry_in_minutes: -130,
            worked_minutes: 1885,
            fulfillment_minutes: 1755,
            target_minutes: 2400,
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
            headline: "Total still owed: 03h 10m".to_string(),
            rows,
            carry_in_minutes: 0,
            worked_minutes: 2210,
            fulfillment_minutes: 2210,
            target_minutes: 2400,
        }
    }

    const EXAMPLE_A_GOLDEN: &str = "\
Week 2026-07 (2026-02-09 - 2026-02-15)

10h 45m left by end of Thursday

  Mon 2026-02-09   08h 10m
  Tue 2026-02-10   07h 50m
  Wed 2026-02-11   08h 00m
  Thu 2026-02-12   07h 25m (ongoing)
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

Total still owed: 03h 10m

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
        let ongoing_lines: Vec<&&str> = lines.iter().filter(|l| l.contains("(ongoing)")).collect();
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
            headline: "Total still owed: 40h 00m".to_string(),
            rows,
            carry_in_minutes: 0,
            worked_minutes: 0,
            fulfillment_minutes: 0,
            target_minutes: 2400,
        };
        let out = render_week(&view);
        assert!(!out.contains("(ongoing)"));
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
            headline: "Total still owed: 43h 12m".to_string(),
            rows,
            carry_in_minutes: -1872,
            worked_minutes: 0,
            fulfillment_minutes: -1872,
            target_minutes: 2400,
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
        let marker_lines: Vec<&str> = out.lines().filter(|l| l.contains("[!]")).collect();
        assert_eq!(marker_lines.len(), 1);
        assert_eq!(marker_lines[0], "  Wed 2026-02-04   07h 45m  [!]");
    }

    // A7
    #[test]
    fn a7_ongoing_and_anomaly_ordering() {
        let mut view = example_a();
        view.rows[3].has_anomaly = true; // Thu, already ongoing
        let out = render_week(&view);
        assert!(out.contains("07h 25m (ongoing)  [!]"));
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

    // B2 (E15)
    #[test]
    fn b2_bucketing_does_not_pair_across_midnight() {
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
        assert_eq!(tue_row.minutes, 0);
        assert!(
            !tue_row.has_anomaly,
            "a single open stint is not an anomaly"
        );
        assert_eq!(wed_row.minutes, 0);
        assert!(wed_row.has_anomaly, "orphaned end must flag an anomaly");
    }

    // B3
    #[test]
    fn b3_completed_stints_only_totals() {
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
        assert_eq!(thu.minutes, 180 + 240);
        assert!(thu.is_ongoing);
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
        assert_eq!(thu.minutes, 0);
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

        let punches = storage::punches_in_range(&conn, dates[0], dates[6]).unwrap();
        let now = now_utc(2026, 2, 15, 20, 0);
        let rows = build_rows(dates, &punches, d(2026, 2, 15), now);
        let row_sum: i64 = rows.iter().map(|r| r.minutes).sum();

        let data = DbWeekData {
            conn: &conn,
            now_utc: now,
        };
        let acct = week::week_accounting(week, &data, &data);
        assert_eq!(row_sum, acct.worked);
        assert_eq!(row_sum, 480 + 510);
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

    // C10
    #[test]
    fn c10_current_vs_closed_headline_end_to_end() {
        let conn = new_test_db();
        let current = build_week_view(&conn, wk(2026, 7), fixed_now()).unwrap();
        assert!(current.headline.contains("left by end of Thursday"));

        let closed = build_week_view(&conn, wk(2026, 6), fixed_now()).unwrap();
        assert!(!closed.headline.contains("Thursday"));
        assert!(!closed.headline.contains("left by end of"));
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
}
