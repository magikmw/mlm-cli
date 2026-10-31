//! Shared history loader for `status` and `mlm week` (SPEC §2.4).
//!
//! One earliest-date read, one ranged punch read and one ranged target
//! read feed the week ledger and hand back the bucketed punches, so every
//! consumer works from the same data. The loader visits only dates that
//! have punches, so a far-future week id costs nothing extra. Any storage
//! failure is an error, never a default. `mlm week --notes` makes one
//! ranged note read of its own, outside this loader.

use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::Connection;

use crate::anomalies::DateBuckets;
use crate::date::WeekId;
use crate::stint;
use crate::storage::{self, Punch};
use crate::week::{self, WeekAccounting, WeekLedger};

/// Punches bucketed by local date. Only dates that have punches are keys.
#[derive(Debug, Clone, Default)]
pub struct PunchBuckets {
    by_date: BTreeMap<NaiveDate, Vec<Punch>>,
}

impl PunchBuckets {
    pub fn from_punches(punches: &[Punch]) -> Self {
        let mut by_date: BTreeMap<NaiveDate, Vec<Punch>> = BTreeMap::new();
        for p in punches {
            by_date.entry(p.date).or_default().push(*p);
        }
        Self { by_date }
    }

    /// The date's punches, an empty slice when it has none.
    pub fn day(&self, date: NaiveDate) -> &[Punch] {
        self.by_date.get(&date).map_or(&[], Vec::as_slice)
    }

    /// The previous, own and next day buckets for `date`.
    pub fn trio(&self, date: NaiveDate) -> DateBuckets<'_> {
        DateBuckets {
            prev: date.pred_opt().map_or(&[], |d| self.day(d)),
            day: self.day(date),
            next: date.succ_opt().map_or(&[], |d| self.day(d)),
        }
    }

    /// Dates that have punches within `[from, to]`, ascending. Empty when
    /// `from > to`.
    pub fn dates_in(&self, from: NaiveDate, to: NaiveDate) -> impl Iterator<Item = NaiveDate> + '_ {
        let range = (from <= to).then(|| self.by_date.range(from..=to));
        range.into_iter().flatten().map(|(date, _)| *date)
    }
}

/// What `status` and `mlm week` both read.
#[derive(Debug, Clone)]
pub struct History {
    /// Worked minutes per week (live added to the current week only),
    /// data weeks and target overrides.
    pub ledger: WeekLedger,
    /// Every punch from one day before the walk start through one day
    /// after the shown week's end.
    pub buckets: PunchBuckets,
    /// Earliest date with a punch or a note.
    pub earliest_date: Option<NaiveDate>,
    /// Today's running-stint minutes. Zero unless today's week is the
    /// shown week or an earlier one.
    pub live_minutes: i64,
}

impl History {
    /// One week's accounting, walked over the ledger.
    pub fn accounting(&self, week: WeekId) -> WeekAccounting {
        week::week_accounting(week, &self.ledger, &self.ledger)
    }
}

/// Loads the history needed to account `shown`. The walk starts at
/// `min(earliest data week, shown)`. `now_utc` decides open-stint minutes.
pub fn load_history(
    conn: &Connection,
    shown: WeekId,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
) -> anyhow::Result<History> {
    let earliest_date = storage::earliest_data_date(conn)?;
    let start_week = walk_start(earliest_date, shown);
    let (from, to) = read_range(start_week, shown);
    let punches = storage::punches_in_range(conn, from, to)?;
    let targets = storage::week_targets_in_range(conn, start_week, shown)?;

    let buckets = PunchBuckets::from_punches(&punches);
    let live_minutes = live_for(&buckets, today, shown, now_utc);
    let mut worked = worked_by_week(&buckets, start_week.start(), shown.end(), now_utc);
    credit_live(&mut worked, today, shown, live_minutes);

    Ok(History {
        ledger: build_ledger(earliest_date, worked, targets),
        buckets,
        earliest_date,
        live_minutes,
    })
}

fn walk_start(earliest: Option<NaiveDate>, shown: WeekId) -> WeekId {
    earliest
        .map(WeekId::from_date)
        .map_or(shown, |week| week.min(shown))
}

/// The walked dates padded by one day each side, so a stint crossing
/// midnight at either edge still resolves.
fn read_range(start_week: WeekId, shown: WeekId) -> (NaiveDate, NaiveDate) {
    let (start, end) = (start_week.start(), shown.end());
    (
        start.pred_opt().unwrap_or(start),
        end.succ_opt().unwrap_or(end),
    )
}

/// Completed minutes per week over the dates with punches in
/// `[from, to]`. Padding days outside that span never count.
fn worked_by_week(
    buckets: &PunchBuckets,
    from: NaiveDate,
    to: NaiveDate,
    now_utc: DateTime<Utc>,
) -> BTreeMap<WeekId, i64> {
    let mut worked: BTreeMap<WeekId, i64> = BTreeMap::new();
    for date in buckets.dates_in(from, to) {
        let trio = buckets.trio(date);
        let day = stint::classify_at(trio.prev, trio.day, trio.next, now_utc);
        *worked.entry(WeekId::from_date(date)).or_insert(0) += day.completed_minutes();
    }
    worked
}

/// Today's running-stint minutes, zero when today's week is after `shown`.
/// The most recent open start dated today, classified with its neighbours
/// so a splice with tomorrow's orphaned end applies.
fn live_for(
    buckets: &PunchBuckets,
    today: NaiveDate,
    shown: WeekId,
    now_utc: DateTime<Utc>,
) -> i64 {
    if WeekId::from_date(today) > shown {
        return 0;
    }
    let trio = buckets.trio(today);
    stint::classify_at(trio.prev, trio.day, trio.next, now_utc).live_minutes()
}

/// Adds `live` to the current week's worked minutes, but only when the
/// walk reaches the current week (`through` is that week or later). Live
/// enters before carry is walked, so a later week's carry-in reflects it.
fn credit_live(
    worked_by_week: &mut BTreeMap<WeekId, i64>,
    today: NaiveDate,
    through: WeekId,
    live: i64,
) {
    let current = WeekId::from_date(today);
    if live > 0 && current <= through {
        *worked_by_week.entry(current).or_insert(0) += live;
    }
}

/// The earliest data week anchors the walk (notes count), each override
/// is recorded, and every week with punches gets its worked minutes.
fn build_ledger(
    earliest: Option<NaiveDate>,
    worked: BTreeMap<WeekId, i64>,
    targets: Vec<(WeekId, i64)>,
) -> WeekLedger {
    let mut ledger = WeekLedger::new();
    if let Some(date) = earliest {
        ledger = ledger.with_data_week(WeekId::from_date(date));
    }
    for (week, minutes) in targets {
        ledger = ledger.with_target(week, minutes);
    }
    for (week, minutes) in worked {
        ledger = ledger.with_worked(week, minutes);
    }
    ledger
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::storage::{PunchKind, punch_from_local};
    use crate::week::WeekData;
    use chrono::{Local, NaiveTime, TimeZone};
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

    fn test_db() -> Connection {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");
        db::apply_migrations(&mut conn).expect("apply migrations");
        conn
    }

    fn now_utc(y: i32, m: u32, day: u32, h: u32, mi: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, day, h, mi, 0).unwrap()
    }

    fn put(conn: &Connection, kind: PunchKind, date: NaiveDate, h: u32, m: u32) {
        storage::insert_punch(conn, kind, date, t(h, m), &TZ_UTC).expect("insert punch");
    }

    fn span(conn: &Connection, date: NaiveDate, from: (u32, u32), to: (u32, u32)) {
        put(conn, PunchKind::Start, date, from.0, from.1);
        put(conn, PunchKind::End, date, to.0, to.1);
    }

    fn fixture_punch(id: i64, date: NaiveDate, h: u32, m: u32) -> Punch {
        punch_from_local(id, PunchKind::Start, date, t(h, m), &Local).expect("fixture punch")
    }

    // ---- PunchBuckets ----

    #[test]
    fn buckets_day_is_empty_for_a_date_without_punches() {
        let b = PunchBuckets::from_punches(&[fixture_punch(1, d(2026, 2, 10), 9, 0)]);
        assert!(b.day(d(2026, 2, 11)).is_empty());
        assert_eq!(b.day(d(2026, 2, 10)).len(), 1);
    }

    #[test]
    fn buckets_trio_holds_the_neighbour_days() {
        let punches = [
            fixture_punch(1, d(2026, 2, 9), 9, 0),
            fixture_punch(2, d(2026, 2, 10), 9, 0),
            fixture_punch(3, d(2026, 2, 10), 11, 0),
            fixture_punch(4, d(2026, 2, 11), 9, 0),
        ];
        let b = PunchBuckets::from_punches(&punches);

        let mid = b.trio(d(2026, 2, 10));
        assert_eq!((mid.prev.len(), mid.day.len(), mid.next.len()), (1, 2, 1));

        let first = b.trio(d(2026, 2, 9));
        assert_eq!(
            (first.prev.len(), first.day.len(), first.next.len()),
            (0, 1, 2)
        );

        let last = b.trio(d(2026, 2, 11));
        assert_eq!(
            (last.prev.len(), last.day.len(), last.next.len()),
            (2, 1, 0)
        );
    }

    #[test]
    fn buckets_dates_in_is_ascending_inclusive_and_skips_empty_dates() {
        let punches = [
            fixture_punch(1, d(2026, 2, 12), 9, 0),
            fixture_punch(2, d(2026, 2, 9), 9, 0),
            fixture_punch(3, d(2026, 2, 20), 9, 0),
            fixture_punch(4, d(2026, 2, 9), 11, 0),
        ];
        let b = PunchBuckets::from_punches(&punches);
        let got: Vec<NaiveDate> = b.dates_in(d(2026, 2, 9), d(2026, 2, 12)).collect();
        assert_eq!(got, vec![d(2026, 2, 9), d(2026, 2, 12)]);
    }

    #[test]
    fn buckets_dates_in_with_from_after_to_is_empty() {
        let b = PunchBuckets::from_punches(&[fixture_punch(1, d(2026, 2, 12), 9, 0)]);
        assert_eq!(b.dates_in(d(2026, 2, 13), d(2026, 2, 9)).count(), 0);
    }

    #[test]
    fn buckets_default_is_empty() {
        let b = PunchBuckets::default();
        assert_eq!(b.dates_in(d(2000, 1, 1), d(2999, 1, 1)).count(), 0);
    }

    // ---- load_history ----

    #[test]
    fn empty_database_gives_an_empty_history() {
        let conn = test_db();
        let h = load_history(
            &conn,
            wk(2026, 7),
            d(2026, 2, 12),
            now_utc(2026, 2, 12, 18, 0),
        )
        .expect("load");
        assert_eq!(h.earliest_date, None);
        assert_eq!(h.live_minutes, 0);
        assert_eq!(
            h.buckets.dates_in(d(1000, 1, 1), d(9999, 12, 31)).count(),
            0
        );
        let acct = h.accounting(wk(2026, 7));
        assert_eq!((acct.carry_in, acct.worked, acct.target), (0, 0, 2400));
    }

    #[test]
    fn two_weeks_of_data_give_worked_per_week() {
        let conn = test_db();
        span(&conn, d(2026, 2, 3), (9, 0), (17, 0));
        span(&conn, d(2026, 2, 4), (9, 0), (10, 30));
        span(&conn, d(2026, 2, 9), (8, 0), (12, 0));
        let h = load_history(
            &conn,
            wk(2026, 7),
            d(2026, 2, 15),
            now_utc(2026, 2, 15, 20, 0),
        )
        .expect("load");
        assert_eq!(h.earliest_date, Some(d(2026, 2, 3)));
        assert_eq!(h.ledger.worked_minutes(wk(2026, 6)), 570);
        assert_eq!(h.ledger.worked_minutes(wk(2026, 7)), 240);
        let acct = h.accounting(wk(2026, 7));
        assert_eq!(acct.carry_in, 570 - 2400);
        assert_eq!(acct.worked, 240);
    }

    #[test]
    fn padding_days_count_only_in_their_own_week() {
        let conn = test_db();
        let week = wk(2026, 7);
        let dates = week.dates();
        span(&conn, dates[0].pred_opt().unwrap(), (9, 0), (11, 0));
        span(&conn, dates[6].succ_opt().unwrap(), (9, 0), (10, 30));
        span(&conn, dates[2], (9, 0), (17, 0));

        let h = load_history(
            &conn,
            wk(2026, 8),
            d(2026, 2, 20),
            now_utc(2026, 2, 20, 20, 0),
        )
        .expect("load");
        let (before, mid, after) = (
            h.ledger.worked_minutes(wk(2026, 6)),
            h.ledger.worked_minutes(week),
            h.ledger.worked_minutes(wk(2026, 8)),
        );
        assert_eq!((before, mid, after), (120, 480, 90));
        assert_eq!(before + mid + after, 120 + 480 + 90);
    }

    #[test]
    fn padding_days_outside_the_walk_are_read_but_not_counted() {
        let conn = test_db();
        // Shown week 2026-06 ends Sun 2026-02-08. The Monday after it
        // holds a stint that belongs to week 2026-07 only.
        span(&conn, d(2026, 2, 3), (9, 0), (10, 0));
        span(&conn, d(2026, 2, 9), (9, 0), (11, 0));
        let h = load_history(
            &conn,
            wk(2026, 6),
            d(2026, 2, 20),
            now_utc(2026, 2, 20, 20, 0),
        )
        .expect("load");
        assert_eq!(h.ledger.worked_minutes(wk(2026, 6)), 60);
        assert_eq!(h.ledger.worked_minutes(wk(2026, 7)), 0);
        assert_eq!(h.buckets.day(d(2026, 2, 9)).len(), 2);
    }

    #[test]
    fn live_is_credited_to_the_current_week_when_shown_is_that_week_or_later() {
        let conn = test_db();
        put(&conn, PunchKind::Start, d(2026, 2, 15), 18, 0);
        let now = now_utc(2026, 2, 15, 20, 0);
        for shown in [wk(2026, 7), wk(2026, 8)] {
            let h = load_history(&conn, shown, d(2026, 2, 15), now).expect("load");
            assert_eq!(h.live_minutes, 120, "{shown}");
            assert_eq!(h.ledger.worked_minutes(wk(2026, 7)), 120, "{shown}");
        }
    }

    #[test]
    fn live_is_not_credited_when_shown_is_an_earlier_week() {
        let conn = test_db();
        span(&conn, d(2026, 2, 3), (9, 0), (10, 0));
        put(&conn, PunchKind::Start, d(2026, 2, 15), 18, 0);
        let h = load_history(
            &conn,
            wk(2026, 6),
            d(2026, 2, 15),
            now_utc(2026, 2, 15, 20, 0),
        )
        .expect("load");
        assert_eq!(h.live_minutes, 0);
        assert_eq!(h.ledger.worked_minutes(wk(2026, 7)), 0);
    }

    #[test]
    fn a_notes_only_week_anchors_the_walk() {
        let conn = test_db();
        storage::insert_note(&conn, d(2026, 2, 3), "hi", now_utc(2026, 2, 3, 9, 0)).unwrap();
        let h = load_history(
            &conn,
            wk(2026, 7),
            d(2026, 2, 12),
            now_utc(2026, 2, 12, 18, 0),
        )
        .expect("load");
        assert_eq!(h.earliest_date, Some(d(2026, 2, 3)));
        // Weeks 06 and 07 idle: carry into 07 is one full idle week.
        assert_eq!(h.accounting(wk(2026, 7)).carry_in, -2400);
    }

    #[test]
    fn target_overrides_in_the_walked_range_apply() {
        let conn = test_db();
        span(&conn, d(2026, 2, 3), (9, 0), (10, 0));
        conn.execute(
            "INSERT INTO week_targets (week_id, target_minutes) VALUES ('2026-06', 0), ('2026-30', 600)",
            [],
        )
        .unwrap();
        let h = load_history(
            &conn,
            wk(2026, 7),
            d(2026, 2, 12),
            now_utc(2026, 2, 12, 18, 0),
        )
        .expect("load");
        assert_eq!(h.accounting(wk(2026, 6)).target, 0);
        assert_eq!(h.accounting(wk(2026, 7)).target, 2400);
    }

    #[test]
    fn a_far_future_week_only_buckets_dates_with_punches() {
        let conn = test_db();
        span(&conn, d(2026, 2, 3), (9, 0), (17, 0));
        let h = load_history(
            &conn,
            wk(9000, 1),
            d(2026, 2, 12),
            now_utc(2026, 2, 12, 18, 0),
        )
        .expect("load");
        let dates: Vec<NaiveDate> = h.buckets.dates_in(d(1000, 1, 1), d(9999, 12, 31)).collect();
        assert_eq!(dates, vec![d(2026, 2, 3)]);
        assert!(h.accounting(wk(9000, 1)).carry_in < 0);
    }

    #[test]
    fn a_week_before_all_data_still_loads() {
        let conn = test_db();
        span(&conn, d(2026, 2, 3), (9, 0), (17, 0));
        let h = load_history(
            &conn,
            wk(2025, 10),
            d(2026, 2, 12),
            now_utc(2026, 2, 12, 18, 0),
        )
        .expect("load");
        let acct = h.accounting(wk(2025, 10));
        assert_eq!((acct.carry_in, acct.worked), (0, 0));
    }

    #[test]
    fn accounting_gives_concrete_carry_worked_and_fulfillment_across_weeks() {
        let conn = test_db();
        span(&conn, d(2026, 2, 3), (9, 0), (17, 0));
        span(&conn, d(2026, 2, 9), (9, 0), (11, 0));
        span(&conn, d(2026, 2, 17), (9, 0), (12, 0));
        conn.execute(
            "INSERT INTO week_targets (week_id, target_minutes) VALUES ('2026-06', 600), ('2026-08', 300)",
            [],
        )
        .unwrap();
        let h = load_history(
            &conn,
            wk(2026, 8),
            d(2026, 2, 20),
            now_utc(2026, 2, 20, 20, 0),
        )
        .expect("load");

        let w6 = h.accounting(wk(2026, 6));
        assert_eq!(
            (w6.target, w6.carry_in, w6.worked, w6.fulfillment),
            (600, 0, 480, 480)
        );
        let w7 = h.accounting(wk(2026, 7));
        assert_eq!(
            (w7.target, w7.carry_in, w7.worked, w7.fulfillment),
            (2400, -120, 120, 0)
        );
        let w8 = h.accounting(wk(2026, 8));
        assert_eq!(
            (w8.target, w8.carry_in, w8.worked, w8.fulfillment),
            (300, -2400, 180, -2220)
        );
        assert_eq!((w8.owed, w8.carry_out), (2520, -2520));
    }

    // ---- failures are hard errors ----

    fn load_err(conn: &Connection) -> anyhow::Error {
        load_history(
            conn,
            wk(2026, 7),
            d(2026, 2, 12),
            now_utc(2026, 2, 12, 18, 0),
        )
        .expect_err("a failed read must be an error")
    }

    #[test]
    fn dropped_notes_table_is_an_error() {
        let conn = test_db();
        span(&conn, d(2026, 2, 3), (9, 0), (17, 0));
        conn.execute("DROP TABLE notes", []).unwrap();
        assert!(load_err(&conn).to_string().contains("database error"));
    }

    #[test]
    fn dropped_week_targets_table_is_an_error() {
        let conn = test_db();
        span(&conn, d(2026, 2, 3), (9, 0), (17, 0));
        conn.execute("DROP TABLE week_targets", []).unwrap();
        assert!(load_err(&conn).to_string().contains("database error"));
    }

    #[test]
    fn only_the_ranged_punch_read_failing_is_an_error() {
        let conn = test_db();
        conn.execute(
            "INSERT INTO punches (at_utc, \"date\", kind) VALUES ('garbage', '2026-02-03', 'start')",
            [],
        )
        .unwrap();
        // The earliest-date read parses only the date, so it still passes.
        assert_eq!(
            storage::earliest_data_date(&conn).unwrap(),
            Some(d(2026, 2, 3))
        );
        assert!(load_err(&conn).to_string().contains("at_utc"));
    }
}
