//! Scan of earlier weeks for anomalies feeding the carry-in marker
//! (SPEC §3.3).

use chrono::{DateTime, NaiveDate, Utc};

use crate::date::WeekId;
use crate::history::History;
use crate::render::CarryMarker;

/// Scans the dates with punches from `history.earliest_date` through the
/// day before `shown.start()`, asking `anomalies::date_facts(..).has_any()`
/// of each date's `history.buckets.trio(date)`. Makes no storage reads.
/// The scan itself is not built yet, so this reports no marker.
pub fn carry_marker(
    history: &History,
    shown: WeekId,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
) -> Option<CarryMarker> {
    let _ = (history, shown, today, now_utc);
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::{PunchBuckets, load_history};
    use chrono::TimeZone;
    use rusqlite::Connection;

    #[test]
    fn stub_returns_none() {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");
        crate::db::apply_migrations(&mut conn).expect("apply migrations");
        let today = NaiveDate::from_ymd_opt(2026, 2, 12).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 2, 12, 18, 0, 0).unwrap();
        let shown = WeekId::from_date(today);
        let history = load_history(&conn, shown, today, now).expect("load");
        assert_eq!(carry_marker(&history, shown, today, now), None);
        assert!(PunchBuckets::default().day(today).is_empty());
    }
}
