//! Scan of earlier weeks for anomalies feeding the carry-in marker
//! (SPEC §3.3).

use std::collections::BTreeSet;

use chrono::{DateTime, NaiveDate, Utc};

use crate::anomalies;
use crate::date::WeekId;
use crate::history::History;
use crate::render::CarryMarker;

/// Counts the earlier weeks that hold at least one date whose `mlm week`
/// row would carry `[!]`. Scans the dates with punches from
/// `history.earliest_date` through the day before `shown.start()`, asking
/// `anomalies::date_facts(..).has_any()` of each date's
/// `history.buckets.trio(date)`. Makes no storage reads.
///
/// Returns `None` when no such week exists, when there is no data, and
/// when `shown` is the earliest week or precedes all data.
pub fn carry_marker(
    history: &History,
    shown: WeekId,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
) -> Option<CarryMarker> {
    let earliest = history.earliest_date?;
    let last = shown.start().pred_opt()?;
    let weeks = anomalous_weeks(history, earliest, last, today, now_utc);
    let oldest = *weeks.first()?;
    Some(CarryMarker {
        weeks: weeks.len(),
        oldest,
    })
}

/// Distinct weeks with an anomalous date in `[from, to]`. Dates run in
/// ascending order, so a week already flagged skips its remaining dates.
fn anomalous_weeks(
    history: &History,
    from: NaiveDate,
    to: NaiveDate,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
) -> BTreeSet<WeekId> {
    let mut weeks = BTreeSet::new();
    for date in history.buckets.dates_in(from, to) {
        let week = WeekId::from_date(date);
        if !weeks.contains(&week) && date_is_anomalous(history, date, today, now_utc) {
            weeks.insert(week);
        }
    }
    weeks
}

/// The predicate a `mlm week` row uses for its `[!]`.
fn date_is_anomalous(
    history: &History,
    date: NaiveDate,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
) -> bool {
    anomalies::date_facts(&history.buckets.trio(date), date, today, now_utc).has_any()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anomalies;
    use crate::history::PunchBuckets;
    use crate::storage::{Punch, PunchKind, punch_from_local};
    use crate::week::WeekLedger;
    use chrono::{NaiveTime, TimeZone};

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("test fixture is a real date")
    }

    fn wk(y: i32, w: u32) -> WeekId {
        WeekId::new(y, w, "t").expect("test fixture is a real ISO week")
    }

    fn p(id: i64, kind: PunchKind, date: NaiveDate, hhmm: (u32, u32)) -> Punch {
        let time = NaiveTime::from_hms_opt(hhmm.0, hhmm.1, 0).expect("real time");
        punch_from_local(id, kind, date, time, &Utc).expect("punch")
    }

    fn history_of(punches: &[Punch], earliest: Option<NaiveDate>) -> History {
        History {
            ledger: WeekLedger::new(),
            buckets: PunchBuckets::from_punches(punches),
            earliest_date: earliest,
            live_minutes: 0,
        }
    }

    fn today() -> NaiveDate {
        d(2026, 2, 12)
    }

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 2, 12, 18, 0, 0).unwrap()
    }

    fn scan(punches: &[Punch], shown: WeekId) -> Option<CarryMarker> {
        let earliest = punches.iter().map(|x| x.date).min();
        carry_marker(&history_of(punches, earliest), shown, today(), now())
    }

    fn one(weeks: usize, y: i32, w: u32) -> Option<CarryMarker> {
        Some(CarryMarker {
            weeks,
            oldest: wk(y, w),
        })
    }

    /// Two starts on one date (E7).
    fn e7(date: NaiveDate) -> Vec<Punch> {
        vec![
            p(1, PunchKind::Start, date, (9, 0)),
            p(2, PunchKind::Start, date, (11, 0)),
        ]
    }

    fn clean(date: NaiveDate) -> Vec<Punch> {
        vec![
            p(1, PunchKind::Start, date, (9, 0)),
            p(2, PunchKind::End, date, (17, 0)),
        ]
    }

    #[test]
    fn anomaly_far_in_past_raises_marker() {
        assert_eq!(scan(&e7(d(2026, 1, 6)), wk(2026, 7)), one(1, 2026, 2));
    }

    #[test]
    fn clean_history_gives_none() {
        assert_eq!(scan(&clean(d(2026, 1, 6)), wk(2026, 7)), None);
    }

    #[test]
    fn empty_history_gives_none() {
        let h = history_of(&[], None);
        assert_eq!(carry_marker(&h, wk(2026, 7), today(), now()), None);
    }

    #[test]
    fn notes_only_earlier_week_gives_none() {
        let h = history_of(&[], Some(d(2026, 1, 5)));
        assert_eq!(carry_marker(&h, wk(2026, 7), today(), now()), None);
    }

    #[test]
    fn counts_distinct_weeks_not_dates() {
        let mut v = e7(d(2026, 1, 6));
        v.extend(e7(d(2026, 1, 7)));
        v.extend(e7(d(2026, 1, 8)));
        v.extend(e7(d(2026, 1, 21)));
        assert_eq!(scan(&v, wk(2026, 7)), one(2, 2026, 2));
    }

    #[test]
    fn several_anomalous_weeks_oldest_is_earliest() {
        let mut v = e7(d(2026, 1, 20));
        v.extend(e7(d(2026, 1, 6)));
        v.extend(e7(d(2026, 1, 27)));
        assert_eq!(scan(&v, wk(2026, 7)), one(3, 2026, 2));
    }

    #[test]
    fn kind_e7_raises_marker() {
        assert_eq!(scan(&e7(d(2026, 1, 6)), wk(2026, 7)), one(1, 2026, 2));
    }

    #[test]
    fn kind_stale_raises_marker() {
        let v = vec![p(1, PunchKind::Start, d(2026, 1, 13), (9, 0))];
        assert_eq!(scan(&v, wk(2026, 7)), one(1, 2026, 3));
    }

    #[test]
    fn kind_e8_raises_marker() {
        let v = vec![p(1, PunchKind::End, d(2026, 1, 20), (10, 0))];
        assert_eq!(scan(&v, wk(2026, 7)), one(1, 2026, 4));
    }

    #[test]
    fn kind_refused_join_raises_marker() {
        let v = vec![
            p(1, PunchKind::Start, d(2026, 1, 27), (9, 0)),
            p(2, PunchKind::End, d(2026, 1, 28), (8, 0)),
            p(3, PunchKind::End, d(2026, 1, 28), (9, 0)),
        ];
        assert_eq!(scan(&v, wk(2026, 7)), one(1, 2026, 5));
    }

    #[test]
    fn refused_join_with_recent_open_start_flags_later_date() {
        let v = vec![
            p(1, PunchKind::Start, d(2026, 1, 27), (9, 0)),
            p(2, PunchKind::End, d(2026, 1, 28), (8, 0)),
            p(3, PunchKind::End, d(2026, 1, 28), (9, 0)),
        ];
        let h = history_of(&v, Some(d(2026, 1, 27)));
        let date = d(2026, 1, 28);
        let facts = anomalies::date_facts(&h.buckets.trio(date), date, date, now());
        assert!(facts.has_any());
    }

    #[test]
    fn scan_agrees_with_has_any_per_date() {
        let mut v = clean(d(2026, 1, 5));
        v.extend(e7(d(2026, 1, 13)));
        v.push(p(10, PunchKind::End, d(2026, 1, 20), (10, 0)));
        let h = history_of(&v, Some(d(2026, 1, 5)));
        let any = h
            .buckets
            .dates_in(d(2026, 1, 1), d(2026, 1, 31))
            .any(|date| {
                anomalies::date_facts(&h.buckets.trio(date), date, today(), now()).has_any()
            });
        let got = carry_marker(&h, wk(2026, 5), today(), now());
        assert_eq!(got.is_some(), any);
        assert_eq!(got, one(2, 2026, 3));
    }

    #[test]
    fn shown_week_anomaly_excluded() {
        assert_eq!(scan(&e7(d(2026, 1, 6)), wk(2026, 2)), None);
    }

    #[test]
    fn only_dates_before_shown_week_start_scanned() {
        // Sunday before the shown Monday (2026-01-12).
        let sunday = vec![p(1, PunchKind::End, d(2026, 1, 11), (10, 0))];
        assert_eq!(scan(&sunday, wk(2026, 3)), one(1, 2026, 2));
        let monday = vec![p(1, PunchKind::End, d(2026, 1, 12), (10, 0))];
        assert_eq!(scan(&monday, wk(2026, 3)), None);
    }

    #[test]
    fn future_shown_week_scans_all_history() {
        assert_eq!(scan(&e7(d(2026, 1, 6)), wk(9000, 1)), one(1, 2026, 2));
    }

    #[test]
    fn current_week_anomaly_is_counted_for_a_future_shown_week() {
        let v = e7(today());
        assert_eq!(scan(&v, wk(2026, 8)), one(1, 2026, 7));
    }

    #[test]
    fn shown_before_all_data_gives_none() {
        assert_eq!(scan(&e7(d(2026, 1, 13)), wk(2026, 1)), None);
    }

    #[test]
    fn shown_is_earliest_week_gives_none() {
        assert_eq!(scan(&e7(d(2026, 1, 6)), wk(2026, 2)), None);
    }
}
