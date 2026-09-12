//! Punch and note storage (SPEC.md §2.1, §2.3, §6.1).
//!
//! Owns the `Punch`/`PunchKind`/`Note` types (PLAN.md interface contract
//! 8) and every read/write over the `punches`/`notes` tables. Every
//! local-to-UTC conversion is generic over `chrono::TimeZone` and takes
//! the timezone as an explicit parameter — there is no hidden
//! `chrono::Local` reference and no hidden `Utc::now()` call anywhere in
//! this module (PLAN.md interface contract 6).

// Milestone 4 is built ahead of its consumers (Milestones 5/7/10/11), so
// several public items here have no in-crate caller yet outside tests.
#![allow(dead_code)]

use std::fmt;

use chrono::{DateTime, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike, Utc};
use rusqlite::Connection;

/// Canonical UTC-instant text format (§2.3/§4.3): always
/// `2026-09-12T13:05:00Z` — fixed-width, zero-padded, `Z`-suffixed.
/// `punches_for_date`'s `ORDER BY at_utc` (a lexical comparison on a
/// `TEXT` column) is only chronologically correct because every row uses
/// this exact format. Never call `to_rfc3339()` or `DateTime`'s default
/// `Display`/`to_string()` on the write path.
const UTC_FMT: &str = "%Y-%m-%dT%H:%M:%SZ";

fn fmt_utc(dt: DateTime<Utc>) -> String {
    dt.format(UTC_FMT).to_string()
}

fn parse_utc(s: &str, column: &'static str) -> Result<DateTime<Utc>, StorageError> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| StorageError::CorruptRow {
            column,
            value: s.to_string(),
        })
}

fn parse_date(s: &str, column: &'static str) -> Result<NaiveDate, StorageError> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| StorageError::CorruptRow {
        column,
        value: s.to_string(),
    })
}

/// Which side of a stint a punch is (§1.3).
///
/// `Ord` is load-bearing beyond this module: `Start` must order before
/// `End` so Milestone 5's §4.3 step-1 tie-break at an identical instant
/// pairs cleanly (E14), and Milestone 5 imports this type directly
/// (PLAN.md contract 8) rather than declaring its own copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PunchKind {
    Start,
    End,
}

impl PunchKind {
    /// The exact text stored in `punches.kind` (matches §2.3's CHECK).
    pub fn as_str(self) -> &'static str {
        match self {
            PunchKind::Start => "start",
            PunchKind::End => "end",
        }
    }

    /// Inverse of `as_str`; unknown text is a data-integrity error rather
    /// than a panic, since the CHECK constraint could in principle be
    /// bypassed by an externally-edited DB file.
    #[allow(
        clippy::should_implement_trait,
        reason = "named from_str per PLAN.md's pinned public API, not std::str::FromStr"
    )]
    pub fn from_str(s: &str) -> Result<Self, StorageError> {
        match s {
            "start" => Ok(PunchKind::Start),
            "end" => Ok(PunchKind::End),
            other => Err(StorageError::CorruptRow {
                column: "kind",
                value: other.to_string(),
            }),
        }
    }
}

/// One stored start/end event, read back (PLAN.md interface contract 1).
///
/// The full row, typed: an ordered instant (UTC), the local calendar date
/// it was recorded against, a start/end kind, and an insertion-order
/// tiebreaker (`id`). `Copy` because every field already is, and
/// Milestone 5's pairing wants to move these around freely.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Punch {
    /// `punches.id`. Surrogate key, monotonically increasing with
    /// insertion (AUTOINCREMENT) — the insertion-order tiebreaker used
    /// when two punches share an identical `at_utc` (§4.3 step 1, E14).
    pub id: i64,

    /// The instant, in UTC. Minute-granular: seconds and nanoseconds are
    /// always zero (§4.1).
    pub at_utc: DateTime<Utc>,

    /// The local calendar date this punch belongs to (§2.1/§2.3). Not
    /// derivable from `at_utc` by string slicing — its own stored column,
    /// computed app-side at insert.
    pub date: NaiveDate,

    /// Whether this is a `start` or an `end` event (§1.3).
    pub kind: PunchKind,
}

/// One stored work-log entry, read back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    /// `notes.id`, AUTOINCREMENT. Secondary insertion-order tiebreaker.
    pub id: i64,

    /// The local calendar date this note is attached to (§2.3) — a note
    /// is attached to a day, not an instant, so there is no `at_utc`.
    pub date: NaiveDate,

    /// The note text, already trimmed of leading/trailing whitespace
    /// (§2.3). Never empty (§6.1 rejects that at insert).
    pub body: String,

    /// `notes.created_at_utc` — the primary insertion-order sort key for
    /// same-day notes (§2.3). Not displayed anywhere in MVP output.
    pub created_at_utc: DateTime<Utc>,
}

/// Failures surfaced by this module (PLAN.md interface contract 7: each
/// wave-1 milestone owns its own concrete error enum).
#[derive(Debug)]
pub enum StorageError {
    /// A `rusqlite` failure (constraint violation, closed connection,
    /// missing table, ...) propagated verbatim.
    Db(rusqlite::Error),

    /// A stored column failed to parse back into its typed form —
    /// data-integrity issue, not something a normal insert can produce.
    CorruptRow { column: &'static str, value: String },

    /// §6.1: a note body that is empty, or entirely whitespace, was
    /// rejected before the trim. Nothing was written.
    EmptyNote,

    /// §2.1's spring-forward gap: the given local wall-clock time does
    /// not correspond to any real instant on that date in `tz`. Nothing
    /// was written.
    NonexistentLocalTime { local: NaiveDateTime },
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::Db(source) => write!(f, "database error: {source}"),
            StorageError::CorruptRow { column, value } => {
                write!(f, "corrupt stored value in column {column}: {value:?}")
            }
            StorageError::EmptyNote => {
                write!(f, "note body is empty or whitespace-only")
            }
            StorageError::NonexistentLocalTime { local } => write!(
                f,
                "local time {local} does not exist (spring-forward DST gap)"
            ),
        }
    }
}

impl std::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StorageError::Db(source) => Some(source),
            StorageError::CorruptRow { .. }
            | StorageError::EmptyNote
            | StorageError::NonexistentLocalTime { .. } => None,
        }
    }
}

impl From<rusqlite::Error> for StorageError {
    fn from(source: rusqlite::Error) -> Self {
        StorageError::Db(source)
    }
}

/// Write-path core: local wall-clock `(date, time)` in `tz` → `(at_utc,
/// date)`, per §4.1's step-by-step conversion.
fn to_utc_and_date<Tz: TimeZone>(
    local_date: NaiveDate,
    local_time: NaiveTime,
    tz: &Tz,
) -> Result<(DateTime<Utc>, NaiveDate), StorageError> {
    // Force minute granularity defensively (§4.1): "no seconds precision
    // anywhere". Setting second/nanosecond to 0 on any valid `NaiveTime`
    // is always representable, so these are not fallible in practice.
    let local_time = local_time
        .with_second(0)
        .expect("second 0 is always a valid NaiveTime component")
        .with_nanosecond(0)
        .expect("nanosecond 0 is always a valid NaiveTime component");
    let naive_local = local_date.and_time(local_time);

    let resolved = match tz.from_local_datetime(&naive_local) {
        LocalResult::Single(dt) => dt,
        // The repeated hour on fall-back: use the earlier of the two real
        // instants (§2.1's documented, conventional choice).
        LocalResult::Ambiguous(earliest, _latest) => earliest,
        // The skipped hour on spring-forward: no real instant exists.
        LocalResult::None => {
            return Err(StorageError::NonexistentLocalTime { local: naive_local });
        }
    };

    let at_utc = resolved.with_timezone(&Utc);
    // Deliberately round-tripped through the timezone rather than reusing
    // `local_date` verbatim — §2.3 specifies `date` as "computed app-side
    // at insert from `at_utc` + local timezone".
    let date = at_utc.with_timezone(tz).date_naive();
    Ok((at_utc, date))
}

/// Build a `Punch` from a local wall-clock date/time without touching a
/// DB. Uses the same conversion path `insert_punch` uses, so hand-built
/// fixtures (Milestone 5's tests) cannot drift from what real storage
/// produces.
pub fn punch_from_local<Tz: TimeZone>(
    id: i64,
    kind: PunchKind,
    local_date: NaiveDate,
    local_time: NaiveTime,
    tz: &Tz,
) -> Result<Punch, StorageError> {
    let (at_utc, date) = to_utc_and_date(local_date, local_time, tz)?;
    Ok(Punch {
        id,
        at_utc,
        date,
        kind,
    })
}

/// Insert a start/end punch for a local wall-clock date+time (§2.1).
/// Returns the new row's `id`.
pub fn insert_punch<Tz: TimeZone>(
    conn: &Connection,
    kind: PunchKind,
    local_date: NaiveDate,
    local_time: NaiveTime,
    tz: &Tz,
) -> Result<i64, StorageError> {
    let (at_utc, date) = to_utc_and_date(local_date, local_time, tz)?;
    conn.execute(
        "INSERT INTO punches (at_utc, \"date\", kind) VALUES (?1, ?2, ?3)",
        (
            fmt_utc(at_utc),
            date.format("%Y-%m-%d").to_string(),
            kind.as_str(),
        ),
    )?;
    Ok(conn.last_insert_rowid())
}

/// Insert a work-log note for a local calendar date (§2.3/§6.1).
///
/// `body` is raw user text; rejected (§6.1) if empty or whitespace-only
/// *before* trimming, otherwise trimmed and stored. `now_utc` is the
/// injected current instant used for `created_at_utc` (never a hidden
/// `Utc::now()`). Returns the new row's `id`.
pub fn insert_note(
    conn: &Connection,
    local_date: NaiveDate,
    body: &str,
    now_utc: DateTime<Utc>,
) -> Result<i64, StorageError> {
    if body.trim().is_empty() {
        return Err(StorageError::EmptyNote);
    }
    let stored = body.trim();
    let now_utc = now_utc
        .with_second(0)
        .expect("second 0 is always valid")
        .with_nanosecond(0)
        .expect("nanosecond 0 is always valid");
    conn.execute(
        "INSERT INTO notes (\"date\", body, created_at_utc) VALUES (?1, ?2, ?3)",
        (
            local_date.format("%Y-%m-%d").to_string(),
            stored,
            fmt_utc(now_utc),
        ),
    )?;
    Ok(conn.last_insert_rowid())
}

/// Insert a punch and, if `note_body` is `Some`, a note for the same
/// local date, atomically (§6.1: a rejected note leaves no orphaned
/// punch). Validates the note body before opening any transaction, so
/// rejection costs nothing and nothing is ever written on that path.
pub fn insert_punch_with_note<Tz: TimeZone>(
    conn: &mut Connection,
    kind: PunchKind,
    local_date: NaiveDate,
    local_time: NaiveTime,
    tz: &Tz,
    note_body: Option<&str>,
    now_utc: DateTime<Utc>,
) -> Result<(i64, Option<i64>), StorageError> {
    if let Some(body) = note_body
        && body.trim().is_empty()
    {
        return Err(StorageError::EmptyNote);
    }

    let tx = conn.transaction()?;
    let punch_id = insert_punch(&tx, kind, local_date, local_time, tz)?;
    let note_id = match note_body {
        Some(body) => Some(insert_note(&tx, local_date, body, now_utc)?),
        None => None,
    };
    tx.commit()?;
    Ok((punch_id, note_id))
}

fn map_punch_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<(i64, String, String, String)> {
    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
}

fn punch_from_row(
    id: i64,
    at_utc: String,
    date: String,
    kind: String,
) -> Result<Punch, StorageError> {
    Ok(Punch {
        id,
        at_utc: parse_utc(&at_utc, "at_utc")?,
        date: parse_date(&date, "date")?,
        kind: PunchKind::from_str(&kind)?,
    })
}

/// All punches for a local calendar date, sorted by instant; ties at an
/// identical instant are broken first by kind (`start` before `end`), then
/// by `id` (insertion order) — SPEC.md §4.3 step 1 / PLAN.md interface
/// contract 10. Empty `Vec`, never an error, for a date with no rows.
pub fn punches_for_date(conn: &Connection, date: NaiveDate) -> Result<Vec<Punch>, StorageError> {
    let mut stmt = conn.prepare(
        "SELECT id, at_utc, \"date\", kind FROM punches WHERE \"date\" = ?1 \
         ORDER BY at_utc ASC, CASE kind WHEN 'start' THEN 0 ELSE 1 END ASC, id ASC",
    )?;
    let rows = stmt.query_map((date.format("%Y-%m-%d").to_string(),), map_punch_row)?;
    let mut out = Vec::new();
    for row in rows {
        let (id, at_utc, date, kind) = row?;
        out.push(punch_from_row(id, at_utc, date, kind)?);
    }
    Ok(out)
}

/// All punches whose local `date` falls within `[from, to]` inclusive,
/// same ordering contract as `punches_for_date` extended across dates
/// (PLAN.md interface contract 10, for Milestones 6/11).
pub fn punches_in_range(
    conn: &Connection,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<Punch>, StorageError> {
    let mut stmt = conn.prepare(
        "SELECT id, at_utc, \"date\", kind FROM punches \
         WHERE \"date\" BETWEEN ?1 AND ?2 \
         ORDER BY \"date\" ASC, at_utc ASC, CASE kind WHEN 'start' THEN 0 ELSE 1 END ASC, id ASC",
    )?;
    let rows = stmt.query_map(
        (
            from.format("%Y-%m-%d").to_string(),
            to.format("%Y-%m-%d").to_string(),
        ),
        map_punch_row,
    )?;
    let mut out = Vec::new();
    for row in rows {
        let (id, at_utc, date, kind) = row?;
        out.push(punch_from_row(id, at_utc, date, kind)?);
    }
    Ok(out)
}

/// All notes for a local calendar date, in insertion order
/// (`created_at_utc ASC, id ASC`). Empty `Vec`, never an error, for a
/// date with no rows.
pub fn notes_for_date(conn: &Connection, date: NaiveDate) -> Result<Vec<Note>, StorageError> {
    let mut stmt = conn.prepare(
        "SELECT id, \"date\", body, created_at_utc FROM notes \
         WHERE \"date\" = ?1 ORDER BY created_at_utc ASC, id ASC",
    )?;
    let rows = stmt.query_map((date.format("%Y-%m-%d").to_string(),), |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (id, date, body, created_at_utc) = row?;
        out.push(Note {
            id,
            date: parse_date(&date, "date")?,
            body,
            created_at_utc: parse_utc(&created_at_utc, "created_at_utc")?,
        });
    }
    Ok(out)
}

/// The earliest local date with any punch or note data, across all
/// history. `None` if the database is empty (PLAN.md interface contract
/// 10, for Milestones 6/11).
pub fn earliest_data_date(conn: &Connection) -> Result<Option<NaiveDate>, StorageError> {
    let min: Option<String> = conn.query_row(
        "SELECT MIN(d) FROM ( \
            SELECT \"date\" AS d FROM punches \
            UNION ALL \
            SELECT \"date\" AS d FROM notes \
         )",
        [],
        |row| row.get(0),
    )?;
    match min {
        Some(s) => Ok(Some(parse_date(&s, "date")?)),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use chrono::TimeZone;
    use chrono_tz::UTC as TZ_UTC;
    use chrono_tz::{America::New_York as TZ_NY, Europe::Warsaw as TZ_WARSAW};

    fn test_db() -> Connection {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");
        db::apply_migrations(&mut conn).expect("apply migrations");
        conn
    }

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("valid date")
    }

    fn t(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).expect("valid time")
    }

    fn utc(y: i32, mo: u32, day: u32, h: u32, mi: u32, s: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, mo, day, h, mi, s).unwrap()
    }

    fn raw_kind(conn: &Connection, id: i64) -> String {
        conn.query_row("SELECT kind FROM punches WHERE id = ?1", (id,), |r| {
            r.get(0)
        })
        .expect("row exists")
    }

    fn raw_at_utc(conn: &Connection, id: i64) -> String {
        conn.query_row("SELECT at_utc FROM punches WHERE id = ?1", (id,), |r| {
            r.get(0)
        })
        .expect("row exists")
    }

    fn raw_date(conn: &Connection, id: i64) -> String {
        conn.query_row("SELECT \"date\" FROM punches WHERE id = ?1", (id,), |r| {
            r.get(0)
        })
        .expect("row exists")
    }

    // --- 7.1 Punch round-trip ------------------------------------------

    // P1
    #[test]
    fn start_punch_round_trips_in_winter_offset() {
        let conn = test_db();
        let id = insert_punch(&conn, PunchKind::Start, d(2026, 1, 15), t(9, 5), &TZ_WARSAW)
            .expect("insert");
        let punches = punches_for_date(&conn, d(2026, 1, 15)).expect("read");
        assert_eq!(punches.len(), 1);
        assert_eq!(punches[0].id, id);
        assert_eq!(punches[0].kind, PunchKind::Start);
        assert_eq!(punches[0].at_utc, utc(2026, 1, 15, 8, 5, 0));
        assert_eq!(punches[0].date, d(2026, 1, 15));
    }

    // P2
    #[test]
    fn end_punch_round_trips_in_summer_offset() {
        let conn = test_db();
        insert_punch(&conn, PunchKind::End, d(2026, 7, 15), t(17, 30), &TZ_WARSAW).expect("insert");
        let punches = punches_for_date(&conn, d(2026, 7, 15)).expect("read");
        assert_eq!(punches.len(), 1);
        assert_eq!(punches[0].kind, PunchKind::End);
        assert_eq!(punches[0].at_utc, utc(2026, 7, 15, 15, 30, 0));
        assert_eq!(punches[0].date, d(2026, 7, 15));
    }

    // P3
    #[test]
    fn hour_form_time_defaults_minute_to_zero() {
        let conn = test_db();
        insert_punch(
            &conn,
            PunchKind::Start,
            d(2026, 1, 15),
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            &TZ_WARSAW,
        )
        .expect("insert");
        let punches = punches_for_date(&conn, d(2026, 1, 15)).expect("read");
        assert_eq!(punches[0].at_utc.format("%M").to_string(), "00");
    }

    // P4
    #[test]
    fn midnight_boundary_local_date_differs_from_utc_date() {
        let conn = test_db();
        insert_punch(
            &conn,
            PunchKind::Start,
            d(2026, 7, 15),
            t(0, 30),
            &TZ_WARSAW,
        )
        .expect("insert");
        let punches = punches_for_date(&conn, d(2026, 7, 15)).expect("read");
        assert_eq!(punches.len(), 1);
        assert_eq!(punches[0].at_utc, utc(2026, 7, 14, 22, 30, 0));
        assert_eq!(punches[0].date, d(2026, 7, 15));
    }

    // P5
    #[test]
    fn midnight_boundary_negative_offset() {
        let conn = test_db();
        insert_punch(&conn, PunchKind::Start, d(2026, 7, 15), t(23, 30), &TZ_NY).expect("insert");
        let punches = punches_for_date(&conn, d(2026, 7, 15)).expect("read");
        assert_eq!(punches.len(), 1);
        assert_eq!(punches[0].at_utc, utc(2026, 7, 16, 3, 30, 0));
        assert_eq!(punches[0].date, d(2026, 7, 15));
    }

    // P6
    #[test]
    fn read_filters_by_local_date_not_utc_date() {
        let conn = test_db();
        // 2026-07-15 00:30 Warsaw -> 2026-07-14T22:30:00Z, local date 07-15.
        insert_punch(
            &conn,
            PunchKind::Start,
            d(2026, 7, 15),
            t(0, 30),
            &TZ_WARSAW,
        )
        .expect("insert");
        // 2026-07-14 23:00 Warsaw -> 2026-07-14T21:00:00Z, local date 07-14.
        insert_punch(&conn, PunchKind::End, d(2026, 7, 14), t(23, 0), &TZ_WARSAW).expect("insert");

        let on_15 = punches_for_date(&conn, d(2026, 7, 15)).expect("read");
        let on_14 = punches_for_date(&conn, d(2026, 7, 14)).expect("read");
        assert_eq!(on_15.len(), 1);
        assert_eq!(on_15[0].at_utc, utc(2026, 7, 14, 22, 30, 0));
        assert_eq!(on_14.len(), 1);
        assert_eq!(on_14[0].at_utc, utc(2026, 7, 14, 21, 0, 0));
    }

    // P7
    #[test]
    fn stored_date_equals_requested_local_date_plain_case() {
        let conn = test_db();
        insert_punch(&conn, PunchKind::Start, d(2026, 1, 15), t(9, 5), &TZ_WARSAW).expect("insert");
        let punches = punches_for_date(&conn, d(2026, 1, 15)).expect("read");
        assert_eq!(punches[0].date, d(2026, 1, 15));
    }

    // P8
    #[test]
    fn empty_date_returns_empty_vec_not_error() {
        let conn = test_db();
        let punches = punches_for_date(&conn, d(2026, 1, 1)).expect("read");
        assert!(punches.is_empty());
    }

    // P9
    #[test]
    fn stored_at_utc_and_date_are_exact_byte_strings() {
        let conn = test_db();
        let id = insert_punch(&conn, PunchKind::Start, d(2026, 1, 15), t(9, 5), &TZ_WARSAW)
            .expect("insert");
        assert_eq!(raw_at_utc(&conn, id), "2026-01-15T08:05:00Z");
        assert_eq!(raw_date(&conn, id), "2026-01-15");
    }

    // P10
    #[test]
    fn kind_round_trips_to_exact_lowercase_literal() {
        let conn = test_db();
        let start_id = insert_punch(&conn, PunchKind::Start, d(2026, 1, 15), t(9, 0), &TZ_UTC)
            .expect("insert");
        let end_id =
            insert_punch(&conn, PunchKind::End, d(2026, 1, 15), t(10, 0), &TZ_UTC).expect("insert");
        assert_eq!(raw_kind(&conn, start_id), "start");
        assert_eq!(raw_kind(&conn, end_id), "end");
    }

    // --- 7.2 Punches come back in sorted order --------------------------

    // P11
    #[test]
    fn punches_sorted_by_instant_regardless_of_insertion_order() {
        let conn = test_db();
        insert_punch(&conn, PunchKind::Start, d(2026, 1, 15), t(9, 0), &TZ_UTC).expect("insert");
        insert_punch(&conn, PunchKind::Start, d(2026, 1, 15), t(14, 0), &TZ_UTC).expect("insert");
        insert_punch(&conn, PunchKind::End, d(2026, 1, 15), t(18, 0), &TZ_UTC).expect("insert");
        insert_punch(&conn, PunchKind::End, d(2026, 1, 15), t(13, 0), &TZ_UTC).expect("insert");

        let punches = punches_for_date(&conn, d(2026, 1, 15)).expect("read");
        let seq: Vec<(PunchKind, u32)> = punches
            .iter()
            .map(|p| (p.kind, p.at_utc.format("%H").to_string().parse().unwrap()))
            .collect();
        assert_eq!(
            seq,
            vec![
                (PunchKind::Start, 9),
                (PunchKind::End, 13),
                (PunchKind::Start, 14),
                (PunchKind::End, 18),
            ]
        );
    }

    // P12
    #[test]
    fn identical_instants_tiebreak_by_kind_then_insertion_order() {
        let conn = test_db();
        let start_id = insert_punch(&conn, PunchKind::Start, d(2026, 1, 15), t(12, 0), &TZ_UTC)
            .expect("insert");
        let end_id =
            insert_punch(&conn, PunchKind::End, d(2026, 1, 15), t(12, 0), &TZ_UTC).expect("insert");
        let punches = punches_for_date(&conn, d(2026, 1, 15)).expect("read");
        assert_eq!(
            punches.iter().map(|p| p.kind).collect::<Vec<_>>(),
            vec![PunchKind::Start, PunchKind::End]
        );
        assert!(punches[0].id < punches[1].id);
        assert_eq!(punches[0].id, start_id);
        assert_eq!(punches[1].id, end_id);

        // Same instant, inserted in the opposite order (`End` before
        // `Start`): the read must still put `Start` first — kind is the
        // tiebreak, not insertion order (SPEC.md §4.3 step 1).
        let conn2 = test_db();
        let end_id2 = insert_punch(&conn2, PunchKind::End, d(2026, 1, 15), t(12, 0), &TZ_UTC)
            .expect("insert");
        let start_id2 = insert_punch(&conn2, PunchKind::Start, d(2026, 1, 15), t(12, 0), &TZ_UTC)
            .expect("insert");
        let punches2 = punches_for_date(&conn2, d(2026, 1, 15)).expect("read");
        assert_eq!(
            punches2.iter().map(|p| p.kind).collect::<Vec<_>>(),
            vec![PunchKind::Start, PunchKind::End]
        );
        assert_eq!(punches2[0].id, start_id2);
        assert_eq!(punches2[1].id, end_id2);
    }

    // P13 -- see dst_order_holds_across_transition_within_one_date (D2 twin)

    // --- 7.3 Note trim-and-store -----------------------------------------

    // N1
    #[test]
    fn padded_note_is_trimmed() {
        let conn = test_db();
        insert_note(
            &conn,
            d(2026, 1, 15),
            "  did a thing  ",
            utc(2026, 1, 15, 9, 0, 0),
        )
        .expect("insert");
        let notes = notes_for_date(&conn, d(2026, 1, 15)).expect("read");
        assert_eq!(notes[0].body, "did a thing");
    }

    // N2
    #[test]
    fn mixed_whitespace_kinds_are_trimmed() {
        let conn = test_db();
        insert_note(
            &conn,
            d(2026, 1, 15),
            "\t\n  fixed the bug \r\n",
            utc(2026, 1, 15, 9, 0, 0),
        )
        .expect("insert");
        let notes = notes_for_date(&conn, d(2026, 1, 15)).expect("read");
        assert_eq!(notes[0].body, "fixed the bug");
    }

    // N3
    #[test]
    fn interior_whitespace_is_preserved() {
        let conn = test_db();
        insert_note(
            &conn,
            d(2026, 1, 15),
            "  did   a   thing  ",
            utc(2026, 1, 15, 9, 0, 0),
        )
        .expect("insert");
        let notes = notes_for_date(&conn, d(2026, 1, 15)).expect("read");
        assert_eq!(notes[0].body, "did   a   thing");
    }

    // N4
    #[test]
    fn already_clean_note_round_trips_byte_identical() {
        let conn = test_db();
        insert_note(
            &conn,
            d(2026, 1, 15),
            "mlm: fixed migration runner bug",
            utc(2026, 1, 15, 9, 0, 0),
        )
        .expect("insert");
        let notes = notes_for_date(&conn, d(2026, 1, 15)).expect("read");
        assert_eq!(notes[0].body, "mlm: fixed migration runner bug");
    }

    // --- 7.4 Empty / whitespace note rejection (E5) -----------------------

    // N5
    #[test]
    fn whitespace_only_notes_are_rejected_and_nothing_is_written() {
        let cases = ["", " ", "   ", "\t", "\n", "\t \n \r ", "\u{00A0}"];
        for body in cases {
            let conn = test_db();
            let err = insert_note(&conn, d(2026, 1, 15), body, utc(2026, 1, 15, 9, 0, 0));
            assert!(
                matches!(err, Err(StorageError::EmptyNote)),
                "expected EmptyNote for {body:?}, got {err:?}"
            );
            let notes = notes_for_date(&conn, d(2026, 1, 15)).expect("read");
            assert!(notes.is_empty(), "expected no rows written for {body:?}");
        }
    }

    // N6
    #[test]
    fn single_non_whitespace_character_notes_are_accepted() {
        let conn = test_db();
        insert_note(&conn, d(2026, 1, 15), ".", utc(2026, 1, 15, 9, 0, 0)).expect("insert");
        insert_note(&conn, d(2026, 1, 15), " x ", utc(2026, 1, 15, 9, 1, 0)).expect("insert");
        let notes = notes_for_date(&conn, d(2026, 1, 15)).expect("read");
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].body, ".");
        assert_eq!(notes[1].body, "x");
    }

    // N7
    #[test]
    fn insert_punch_with_note_rejects_empty_note_leaving_no_orphaned_punch() {
        let mut conn = test_db();
        let err = insert_punch_with_note(
            &mut conn,
            PunchKind::Start,
            d(2026, 1, 15),
            t(9, 0),
            &TZ_UTC,
            Some("   "),
            utc(2026, 1, 15, 9, 0, 0),
        );
        assert!(matches!(err, Err(StorageError::EmptyNote)));
        assert!(punches_for_date(&conn, d(2026, 1, 15)).unwrap().is_empty());
        assert!(notes_for_date(&conn, d(2026, 1, 15)).unwrap().is_empty());
    }

    #[test]
    fn insert_punch_with_note_commits_both_rows_together() {
        let mut conn = test_db();
        let (punch_id, note_id) = insert_punch_with_note(
            &mut conn,
            PunchKind::Start,
            d(2026, 1, 15),
            t(9, 0),
            &TZ_UTC,
            Some("  hello  "),
            utc(2026, 1, 15, 9, 0, 0),
        )
        .expect("insert");
        assert!(note_id.is_some());
        let punches = punches_for_date(&conn, d(2026, 1, 15)).unwrap();
        let notes = notes_for_date(&conn, d(2026, 1, 15)).unwrap();
        assert_eq!(punches.len(), 1);
        assert_eq!(punches[0].id, punch_id);
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].body, "hello");
    }

    #[test]
    fn insert_punch_with_note_none_leaves_notes_empty() {
        let mut conn = test_db();
        let (punch_id, note_id) = insert_punch_with_note(
            &mut conn,
            PunchKind::End,
            d(2026, 1, 15),
            t(17, 0),
            &TZ_UTC,
            None,
            utc(2026, 1, 15, 17, 0, 0),
        )
        .expect("insert");
        assert!(note_id.is_none());
        let punches = punches_for_date(&conn, d(2026, 1, 15)).unwrap();
        assert_eq!(punches.len(), 1);
        assert_eq!(punches[0].id, punch_id);
        assert!(notes_for_date(&conn, d(2026, 1, 15)).unwrap().is_empty());
    }

    // --- 7.5 Notes in insertion order -------------------------------------

    // N8
    #[test]
    fn notes_in_plain_insertion_order() {
        let conn = test_db();
        insert_note(&conn, d(2026, 1, 15), "first", utc(2026, 1, 15, 9, 0, 0)).expect("insert");
        insert_note(&conn, d(2026, 1, 15), "second", utc(2026, 1, 15, 10, 0, 0)).expect("insert");
        insert_note(&conn, d(2026, 1, 15), "third", utc(2026, 1, 15, 11, 0, 0)).expect("insert");
        let notes = notes_for_date(&conn, d(2026, 1, 15)).expect("read");
        assert_eq!(
            notes.iter().map(|n| n.body.as_str()).collect::<Vec<_>>(),
            vec!["first", "second", "third"]
        );
    }

    // N9
    #[test]
    fn same_minute_notes_tiebreak_by_id() {
        let conn = test_db();
        let same_instant = utc(2026, 1, 15, 9, 0, 0);
        insert_note(&conn, d(2026, 1, 15), "a", same_instant).expect("insert");
        insert_note(&conn, d(2026, 1, 15), "b", same_instant).expect("insert");
        let notes = notes_for_date(&conn, d(2026, 1, 15)).expect("read");
        assert_eq!(
            notes.iter().map(|n| n.body.as_str()).collect::<Vec<_>>(),
            vec!["a", "b"]
        );
    }

    // N10
    #[test]
    fn created_at_utc_is_the_primary_sort_key_not_id() {
        let conn = test_db();
        insert_note(&conn, d(2026, 1, 15), "A", utc(2026, 1, 15, 12, 5, 0)).expect("insert");
        insert_note(&conn, d(2026, 1, 15), "B", utc(2026, 1, 15, 12, 1, 0)).expect("insert");
        let notes = notes_for_date(&conn, d(2026, 1, 15)).expect("read");
        assert_eq!(
            notes.iter().map(|n| n.body.as_str()).collect::<Vec<_>>(),
            vec!["B", "A"]
        );
    }

    // N11
    #[test]
    fn notes_are_scoped_to_their_date() {
        let conn = test_db();
        insert_note(&conn, d(2026, 1, 15), "on 15", utc(2026, 1, 15, 9, 0, 0)).expect("insert");
        insert_note(&conn, d(2026, 1, 16), "on 16", utc(2026, 1, 16, 9, 0, 0)).expect("insert");
        assert_eq!(notes_for_date(&conn, d(2026, 1, 15)).unwrap().len(), 1);
        assert_eq!(notes_for_date(&conn, d(2026, 1, 16)).unwrap().len(), 1);
        assert!(notes_for_date(&conn, d(2026, 1, 17)).unwrap().is_empty());
    }

    // N12
    #[test]
    fn notes_and_punches_are_independent() {
        let conn = test_db();
        insert_note(
            &conn,
            d(2026, 1, 15),
            "just a note",
            utc(2026, 1, 15, 9, 0, 0),
        )
        .expect("insert");
        let notes = notes_for_date(&conn, d(2026, 1, 15)).unwrap();
        let punches = punches_for_date(&conn, d(2026, 1, 15)).unwrap();
        assert_eq!(notes.len(), 1);
        assert!(punches.is_empty());

        let conn2 = test_db();
        insert_punch(&conn2, PunchKind::Start, d(2026, 1, 15), t(9, 0), &TZ_UTC).expect("insert");
        let notes2 = notes_for_date(&conn2, d(2026, 1, 15)).unwrap();
        let punches2 = punches_for_date(&conn2, d(2026, 1, 15)).unwrap();
        assert!(notes2.is_empty());
        assert_eq!(punches2.len(), 1);
    }

    // --- 7.6 DST-transition round-trip (F12) ------------------------------

    // D1
    #[test]
    fn dst_spring_forward_different_dates_use_own_offsets() {
        let conn = test_db();
        insert_punch(
            &conn,
            PunchKind::Start,
            d(2026, 3, 28),
            t(12, 0),
            &TZ_WARSAW,
        )
        .expect("insert");
        insert_punch(
            &conn,
            PunchKind::Start,
            d(2026, 3, 30),
            t(12, 0),
            &TZ_WARSAW,
        )
        .expect("insert");

        let before = punches_for_date(&conn, d(2026, 3, 28)).unwrap();
        let after = punches_for_date(&conn, d(2026, 3, 30)).unwrap();
        assert_eq!(before[0].at_utc, utc(2026, 3, 28, 11, 0, 0));
        assert_eq!(after[0].at_utc, utc(2026, 3, 30, 10, 0, 0));
        assert_eq!(before[0].date, d(2026, 3, 28));
        assert_eq!(after[0].date, d(2026, 3, 30));

        // 2026-03-28T11:00Z -> 2026-03-30T10:00Z: 48 hours minus the 1-hour
        // DST offset change = 47, not the plan's stated 46 (arithmetic slip
        // in plans/milestone-4-punch-note-storage.md §7.6 D1 — verified by
        // direct subtraction of the two asserted instants above).
        let gap = after[0].at_utc - before[0].at_utc;
        assert_eq!(gap.num_hours(), 47);
    }

    // D2
    #[test]
    fn dst_order_holds_across_transition_within_one_date() {
        let conn = test_db();
        // Still UTC+1: 2026-03-29 01:30 Warsaw -> 2026-03-29T00:30:00Z.
        insert_punch(
            &conn,
            PunchKind::Start,
            d(2026, 3, 29),
            t(1, 30),
            &TZ_WARSAW,
        )
        .expect("insert");
        // Already UTC+2: 2026-03-29 03:30 Warsaw -> 2026-03-29T01:30:00Z.
        insert_punch(&conn, PunchKind::End, d(2026, 3, 29), t(3, 30), &TZ_WARSAW).expect("insert");

        let punches = punches_for_date(&conn, d(2026, 3, 29)).expect("read");
        assert_eq!(punches.len(), 2);
        assert_eq!(punches[0].kind, PunchKind::Start);
        assert_eq!(punches[1].kind, PunchKind::End);
        assert_eq!(punches[0].at_utc, utc(2026, 3, 29, 0, 30, 0));
        assert_eq!(punches[1].at_utc, utc(2026, 3, 29, 1, 30, 0));
        assert!(punches.iter().all(|p| p.date == d(2026, 3, 29)));

        let gap = punches[1].at_utc - punches[0].at_utc;
        assert_eq!(gap.num_minutes(), 60);
    }

    // D3
    #[test]
    fn dst_fall_back_ambiguity_resolves_to_earlier_instant() {
        let conn = test_db();
        let id = insert_punch(
            &conn,
            PunchKind::Start,
            d(2026, 10, 25),
            t(2, 30),
            &TZ_WARSAW,
        )
        .expect("ambiguous time should resolve, not error");
        let punches = punches_for_date(&conn, d(2026, 10, 25)).unwrap();
        assert_eq!(punches.len(), 1);
        assert_eq!(punches[0].id, id);
        assert_eq!(punches[0].at_utc, utc(2026, 10, 25, 0, 30, 0));
        assert_eq!(punches[0].date, d(2026, 10, 25));
    }

    // D4
    #[test]
    fn dst_fall_back_different_dates_use_own_offsets() {
        let conn = test_db();
        insert_punch(
            &conn,
            PunchKind::Start,
            d(2026, 10, 24),
            t(12, 0),
            &TZ_WARSAW,
        )
        .expect("insert");
        insert_punch(
            &conn,
            PunchKind::Start,
            d(2026, 10, 26),
            t(12, 0),
            &TZ_WARSAW,
        )
        .expect("insert");

        let before = punches_for_date(&conn, d(2026, 10, 24)).unwrap();
        let after = punches_for_date(&conn, d(2026, 10, 26)).unwrap();
        assert_eq!(before[0].at_utc, utc(2026, 10, 24, 10, 0, 0));
        assert_eq!(after[0].at_utc, utc(2026, 10, 26, 11, 0, 0));
    }

    // D5
    #[test]
    fn dst_spring_forward_gap_is_a_hard_error_and_writes_nothing() {
        let conn = test_db();
        let err = insert_punch(
            &conn,
            PunchKind::Start,
            d(2026, 3, 29),
            t(2, 30),
            &TZ_WARSAW,
        );
        assert!(
            matches!(err, Err(StorageError::NonexistentLocalTime { .. })),
            "expected NonexistentLocalTime, got {err:?}"
        );
        assert!(punches_for_date(&conn, d(2026, 3, 29)).unwrap().is_empty());
    }

    // D6
    #[test]
    fn dst_zero_offset_timezone_is_unaffected() {
        let conn = test_db();
        insert_punch(&conn, PunchKind::Start, d(2026, 3, 29), t(2, 30), &TZ_UTC)
            .expect("no gap under UTC");
        insert_punch(&conn, PunchKind::Start, d(2026, 10, 25), t(2, 30), &TZ_UTC)
            .expect("no ambiguity under UTC");
        let spring = punches_for_date(&conn, d(2026, 3, 29)).unwrap();
        let fall = punches_for_date(&conn, d(2026, 10, 25)).unwrap();
        assert_eq!(spring[0].at_utc, utc(2026, 3, 29, 2, 30, 0));
        assert_eq!(fall[0].at_utc, utc(2026, 10, 25, 2, 30, 0));
        assert_eq!(spring[0].date, d(2026, 3, 29));
        assert_eq!(fall[0].date, d(2026, 10, 25));
    }

    // D7
    #[test]
    fn notes_are_dst_agnostic() {
        let conn = test_db();
        insert_note(
            &conn,
            d(2026, 3, 29),
            "note on transition day",
            utc(2026, 3, 29, 12, 0, 0),
        )
        .expect("insert");
        let notes = notes_for_date(&conn, d(2026, 3, 29)).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].date, d(2026, 3, 29));
    }

    // --- 7.7 Error surface -------------------------------------------------

    // X1
    #[test]
    fn db_error_on_punches_propagates_not_panics() {
        let conn = test_db();
        conn.execute("DROP TABLE punches", []).expect("drop table");
        let insert_err = insert_punch(&conn, PunchKind::Start, d(2026, 1, 1), t(9, 0), &TZ_UTC);
        assert!(matches!(insert_err, Err(StorageError::Db(_))));
        let read_err = punches_for_date(&conn, d(2026, 1, 1));
        assert!(matches!(read_err, Err(StorageError::Db(_))));
    }

    #[test]
    fn db_error_on_notes_propagates_not_panics() {
        let conn = test_db();
        conn.execute("DROP TABLE notes", []).expect("drop table");
        let insert_err = insert_note(&conn, d(2026, 1, 1), "hi", utc(2026, 1, 1, 9, 0, 0));
        assert!(matches!(insert_err, Err(StorageError::Db(_))));
        let read_err = notes_for_date(&conn, d(2026, 1, 1));
        assert!(matches!(read_err, Err(StorageError::Db(_))));
    }

    // X2
    #[test]
    fn corrupt_at_utc_column_is_reported_not_panicked() {
        let conn = test_db();
        conn.execute(
            "INSERT INTO punches (at_utc, \"date\", kind) VALUES ('not-a-timestamp', '2026-01-01', 'start')",
            [],
        )
        .expect("raw insert bypassing the typed path");
        let err = punches_for_date(&conn, d(2026, 1, 1));
        assert!(matches!(
            err,
            Err(StorageError::CorruptRow {
                column: "at_utc",
                ..
            })
        ));
    }

    // X3
    #[test]
    fn schema_check_rejects_wrong_case_kind_under_raw_insert() {
        let conn = test_db();
        let result = conn.execute(
            "INSERT INTO punches (at_utc, \"date\", kind) VALUES ('2026-01-01T09:00:00Z', '2026-01-01', 'START')",
            [],
        );
        assert!(result.is_err(), "CHECK constraint should reject 'START'");
    }

    // --- earliest_data_date / punches_in_range (contract 10) --------------

    #[test]
    fn earliest_data_date_is_none_for_empty_database() {
        let conn = test_db();
        assert_eq!(earliest_data_date(&conn).unwrap(), None);
    }

    #[test]
    fn earliest_data_date_finds_minimum_across_punches_and_notes() {
        let conn = test_db();
        insert_punch(&conn, PunchKind::Start, d(2026, 3, 10), t(9, 0), &TZ_UTC).expect("insert");
        insert_note(&conn, d(2026, 2, 1), "earliest", utc(2026, 2, 1, 9, 0, 0)).expect("insert");
        insert_punch(&conn, PunchKind::Start, d(2026, 5, 1), t(9, 0), &TZ_UTC).expect("insert");
        assert_eq!(earliest_data_date(&conn).unwrap(), Some(d(2026, 2, 1)));
    }

    #[test]
    fn punches_in_range_matches_ordering_contract_across_dates() {
        let conn = test_db();
        insert_punch(&conn, PunchKind::Start, d(2026, 1, 10), t(9, 0), &TZ_UTC).expect("insert");
        insert_punch(&conn, PunchKind::End, d(2026, 1, 10), t(17, 0), &TZ_UTC).expect("insert");
        insert_punch(&conn, PunchKind::Start, d(2026, 1, 11), t(9, 0), &TZ_UTC).expect("insert");
        insert_punch(&conn, PunchKind::Start, d(2026, 1, 12), t(9, 0), &TZ_UTC).expect("insert");

        let range = punches_in_range(&conn, d(2026, 1, 10), d(2026, 1, 11)).unwrap();
        assert_eq!(range.len(), 3);
        assert_eq!(range[0].date, d(2026, 1, 10));
        assert_eq!(range[1].date, d(2026, 1, 10));
        assert_eq!(range[2].date, d(2026, 1, 11));
        assert_eq!(range[0].kind, PunchKind::Start);
        assert_eq!(range[1].kind, PunchKind::End);
    }

    #[test]
    fn punches_in_range_empty_span_returns_empty_vec() {
        let conn = test_db();
        insert_punch(&conn, PunchKind::Start, d(2026, 1, 10), t(9, 0), &TZ_UTC).expect("insert");
        let range = punches_in_range(&conn, d(2026, 2, 1), d(2026, 2, 28)).unwrap();
        assert!(range.is_empty());
    }
}
