//! SQLite storage layer: connection bootstrap, schema, and migrations.
//!
//! The database lives in the platform app-data dir (via
//! `directories::ProjectDirs`), e.g. `~/.local/share/mlm/mlm.db` on Linux
//! and `%APPDATA%\mlm\data\mlm.db` on Windows, unless the `MLM_DB_PATH`
//! environment variable overrides it.
//!
//! First run is not a special case: the data directory is created if
//! absent, SQLite creates the file if absent, and the embedded migration
//! set applies whatever is pending from schema version 0. A failure is
//! defined purely as one of those steps returning `Err` — this module
//! never panics.

use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};

/// Qualifier/org/app triple used to locate the platform data dir.
const QUALIFIER: &str = "";
const ORG: &str = "";
const APP: &str = "mlm";
const DB_FILENAME: &str = "mlm.db";

/// Environment variable that, when set and non-empty, overrides the
/// resolved database path verbatim. Exists so integration tests can point
/// the compiled binary at a temp file instead of the real database.
pub const DB_PATH_ENV: &str = "MLM_DB_PATH";

/// Baseline schema (migration 0001).
///
/// INVARIANT: once a release ships, this string is FROZEN. Schema changes
/// are appended to `MIGRATION_SLICE` as new migrations, never edits here.
const MIGRATION_0001_BASELINE: &str = r#"
-- Pre-release cleanup: the scaffold shipped an `entries` table on
-- developer machines with user_version still 0, so this migration runs
-- against those files and should not leave the dead table behind.
-- Harmless no-op on a fresh database.
DROP TABLE IF EXISTS entries;

CREATE TABLE punches (
    id      INTEGER PRIMARY KEY AUTOINCREMENT,
    at_utc  TEXT NOT NULL,
    "date"  TEXT NOT NULL,
    kind    TEXT NOT NULL CHECK (kind IN ('start', 'end'))
);

CREATE INDEX idx_punches_date_at_utc_id
    ON punches ("date", at_utc, id);

CREATE TABLE notes (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    "date"         TEXT NOT NULL,
    body           TEXT NOT NULL,
    created_at_utc TEXT NOT NULL
);

CREATE INDEX idx_notes_date_created_at_utc_id
    ON notes ("date", created_at_utc, id);

CREATE TABLE week_targets (
    week_id        TEXT PRIMARY KEY,
    target_minutes INTEGER NOT NULL CHECK (target_minutes >= 0)
);
"#;

/// Ordered, embedded migration set. Index 0 == schema version 1.
///
/// INVARIANT: once a release ships, an entry in this slice is FROZEN.
/// Schema changes are appended as new entries, never edits.
const MIGRATION_SLICE: &[M<'static>] =
    &[M::up(MIGRATION_0001_BASELINE).comment("baseline: punches, notes, week_targets")];

/// Borrowing view over `MIGRATION_SLICE`. Cheap (no allocation).
fn migrations() -> Migrations<'static> {
    Migrations::from_slice(MIGRATION_SLICE)
}

/// Failures surfaced by this module. Every variant is a hard error in
/// SPEC.md §6.1 terms; none of them is a "first run" signal.
#[derive(Debug)]
pub enum DbError {
    /// `ProjectDirs::from(...)` returned `None` — no home/app-data dir on
    /// this platform.
    DataDirUnavailable,
    /// Creating the data directory failed (permissions, a file where a
    /// directory must go, read-only filesystem).
    CreateDataDir {
        path: PathBuf,
        source: std::io::Error,
    },
    /// `Connection::open` failed.
    Open {
        path: PathBuf,
        source: rusqlite::Error,
    },
    /// The migration set failed to apply. `path` is `None` when migrating
    /// a connection that has no filesystem path (e.g. in-memory).
    Migrate {
        path: Option<PathBuf>,
        source: rusqlite_migration::Error,
    },
    /// Statement execution failure. Reserved for later milestones'
    /// queries, declared here so they extend rather than redefine
    /// `DbError` (contract 7). Constructed from Milestone 4 onward.
    #[allow(dead_code, reason = "declared for Milestone 4+ query call sites")]
    Query(rusqlite::Error),
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DbError::DataDirUnavailable => {
                write!(f, "could not determine the application data directory")
            }
            DbError::CreateDataDir { path, source } => write!(
                f,
                "could not create data directory {}: {source}",
                path.display()
            ),
            DbError::Open { path, source } => {
                write!(f, "could not open database {}: {source}", path.display())
            }
            DbError::Migrate {
                path: Some(path),
                source,
            } => write!(f, "could not migrate database {}: {source}", path.display()),
            DbError::Migrate { path: None, source } => {
                write!(f, "could not migrate database: {source}")
            }
            DbError::Query(source) => write!(f, "database query failed: {source}"),
        }
    }
}

impl std::error::Error for DbError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DbError::DataDirUnavailable => None,
            DbError::CreateDataDir { source, .. } => Some(source),
            DbError::Open { source, .. } => Some(source),
            DbError::Migrate { source, .. } => Some(source),
            DbError::Query(source) => Some(source),
        }
    }
}

/// Platform app-data directory. Pure: no filesystem side effects.
pub fn data_dir() -> Result<PathBuf, DbError> {
    ProjectDirs::from(QUALIFIER, ORG, APP)
        .map(|dirs| dirs.data_dir().to_path_buf())
        .ok_or(DbError::DataDirUnavailable)
}

/// Resolve the database path given an explicit `MLM_DB_PATH` value.
/// Set and non-empty wins verbatim; otherwise `data_dir()/mlm.db`.
fn resolve_db_path(env_override: Option<OsString>) -> Result<PathBuf, DbError> {
    match env_override {
        Some(value) if !value.is_empty() => Ok(PathBuf::from(value)),
        _ => Ok(data_dir()?.join(DB_FILENAME)),
    }
}

/// The database path this binary uses. Pure: no filesystem side effects.
pub fn default_db_path() -> Result<PathBuf, DbError> {
    resolve_db_path(std::env::var_os(DB_PATH_ENV))
}

/// Apply every pending migration to an already-open connection. No path
/// or directory logic — usable against `Connection::open_in_memory()`.
pub fn apply_migrations(conn: &mut Connection) -> Result<(), DbError> {
    migrations()
        .to_latest(conn)
        .map_err(|source| DbError::Migrate { path: None, source })
}

/// Open (creating if absent) the database at `path`, creating `path`'s
/// parent directory if absent, and apply all pending migrations.
pub fn connect_at(path: &Path) -> Result<Connection, DbError> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|source| DbError::CreateDataDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let mut conn = Connection::open(path).map_err(|source| DbError::Open {
        path: path.to_path_buf(),
        source,
    })?;

    apply_migrations(&mut conn).map_err(|err| match err {
        DbError::Migrate { source, .. } => DbError::Migrate {
            path: Some(path.to_path_buf()),
            source,
        },
        other => other,
    })?;

    Ok(conn)
}

/// Production entry point: connect to the resolved database path.
pub fn connect() -> Result<Connection, DbError> {
    connect_at(&default_db_path()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Isolated temp directory plus a db path inside a not-yet-created
    /// subdirectory of it. The caller MUST bind the `TempDir` — dropping
    /// it deletes the tree.
    fn temp_db() -> (TempDir, PathBuf) {
        let dir = TempDir::new().expect("create temp dir");
        let path = dir.path().join("nested").join("mlm.db");
        (dir, path)
    }

    fn user_version(conn: &Connection) -> i64 {
        conn.query_row("PRAGMA user_version", [], |r| r.get(0))
            .expect("read user_version")
    }

    // T1
    #[test]
    fn connect_at_creates_missing_parent_directory_and_file() {
        let (_dir, path) = temp_db();
        assert!(!path.parent().unwrap().exists());

        let conn = connect_at(&path).expect("connect_at should succeed");
        drop(conn);

        assert!(path.parent().unwrap().is_dir());
        assert!(path.is_file());
    }

    // T2
    #[test]
    fn baseline_schema_has_expected_tables_and_columns() {
        let (_dir, path) = temp_db();
        let conn = connect_at(&path).expect("connect");

        /// `(name, declared type, NOT NULL)` for one column.
        type ColumnSpec = (&'static str, &'static str, bool);
        /// `(table, its expected columns)`.
        type TableSpec = (&'static str, &'static [ColumnSpec]);

        let expected: &[TableSpec] = &[
            (
                "punches",
                &[
                    ("id", "INTEGER", false),
                    ("at_utc", "TEXT", true),
                    ("date", "TEXT", true),
                    ("kind", "TEXT", true),
                ],
            ),
            (
                "notes",
                &[
                    ("id", "INTEGER", false),
                    ("date", "TEXT", true),
                    ("body", "TEXT", true),
                    ("created_at_utc", "TEXT", true),
                ],
            ),
            (
                "week_targets",
                &[
                    ("week_id", "TEXT", false),
                    ("target_minutes", "INTEGER", true),
                ],
            ),
        ];

        for (table, columns) in expected {
            let mut stmt = conn
                .prepare(&format!("PRAGMA table_info({table})"))
                .expect("prepare table_info");
            let found: Vec<(String, String, bool)> = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)? != 0,
                    ))
                })
                .expect("query table_info")
                .collect::<Result<_, _>>()
                .expect("collect table_info");

            assert!(!found.is_empty(), "table {table} does not exist");

            for (name, decl_type, not_null) in *columns {
                let col = found
                    .iter()
                    .find(|(n, _, _)| n == name)
                    .unwrap_or_else(|| panic!("{table}.{name} missing"));
                assert_eq!(&col.1, decl_type, "{table}.{name} declared type");
                assert_eq!(col.2, *not_null, "{table}.{name} NOT NULL flag");
            }
        }
    }

    // T3
    #[test]
    fn fresh_connect_lands_on_schema_version_one_without_a_migrations_table() {
        let (_dir, path) = temp_db();
        let conn = connect_at(&path).expect("connect");

        assert_eq!(user_version(&conn), 1);
        assert_eq!(migrations().pending_migrations(&conn).expect("pending"), 0);
        assert_eq!(
            usize::from(migrations().current_version(&conn).expect("current")),
            1
        );

        // Bookkeeping is PRAGMA user_version; no table is ever created.
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'schema_migrations'",
                [],
                |r| r.get(0),
            )
            .expect("query sqlite_master");
        assert_eq!(count, 0);
    }

    // T4
    #[test]
    fn reconnecting_is_idempotent_and_preserves_data() {
        let (_dir, path) = temp_db();
        {
            let conn = connect_at(&path).expect("first connect");
            conn.execute(
                "INSERT INTO punches (at_utc, \"date\", kind) VALUES (?1, ?2, ?3)",
                ("2026-09-12T13:05:00Z", "2026-09-12", "start"),
            )
            .expect("insert punch");
        }

        let conn = connect_at(&path).expect("second connect");
        assert_eq!(user_version(&conn), 1);
        assert_eq!(migrations().pending_migrations(&conn).expect("pending"), 0);

        let row: (String, String, String) = conn
            .query_row("SELECT at_utc, \"date\", kind FROM punches", [], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })
            .expect("row survives");
        assert_eq!(
            row,
            (
                "2026-09-12T13:05:00Z".to_string(),
                "2026-09-12".to_string(),
                "start".to_string()
            )
        );
    }

    // T5
    #[test]
    fn migration_set_is_self_consistent() {
        migrations().validate().expect("migration set validates");
    }

    fn is_constraint_violation(err: &rusqlite::Error) -> bool {
        matches!(
            err,
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error {
                    code: rusqlite::ErrorCode::ConstraintViolation,
                    ..
                },
                _
            )
        )
    }

    fn insert_punch(
        conn: &Connection,
        at_utc: &str,
        date: &str,
        kind: &str,
    ) -> rusqlite::Result<usize> {
        conn.execute(
            "INSERT INTO punches (at_utc, \"date\", kind) VALUES (?1, ?2, ?3)",
            (at_utc, date, kind),
        )
    }

    fn count(conn: &Connection, table: &str) -> i64 {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .expect("count rows")
    }

    // T6
    #[test]
    fn punch_kind_check_rejects_anything_but_lowercase_start_and_end() {
        let (_dir, path) = temp_db();
        let conn = connect_at(&path).expect("connect");

        for kind in ["START", "Start", "pause", "", "starts", "END"] {
            let err = insert_punch(&conn, "2026-09-12T13:05:00Z", "2026-09-12", kind)
                .expect_err(&format!("kind {kind:?} must be rejected"));
            assert!(
                is_constraint_violation(&err),
                "kind {kind:?} produced {err:?}, expected a constraint violation"
            );
        }
        assert_eq!(count(&conn, "punches"), 0);
    }

    // T7
    #[test]
    fn punch_kind_check_accepts_start_and_end() {
        let (_dir, path) = temp_db();
        let conn = connect_at(&path).expect("connect");

        insert_punch(&conn, "2026-09-12T09:00:00Z", "2026-09-12", "start").expect("start");
        insert_punch(&conn, "2026-09-12T17:00:00Z", "2026-09-12", "end").expect("end");

        let mut stmt = conn
            .prepare("SELECT kind FROM punches ORDER BY id")
            .expect("prepare");
        let kinds: Vec<String> = stmt
            .query_map([], |r| r.get(0))
            .expect("query")
            .collect::<Result<_, _>>()
            .expect("collect");
        assert_eq!(kinds, vec!["start".to_string(), "end".to_string()]);
    }

    // T8
    #[test]
    fn week_target_minutes_rejects_negative_and_accepts_zero() {
        let (_dir, path) = temp_db();
        let conn = connect_at(&path).expect("connect");

        for (week_id, minutes) in [("2026-07", -1_i64), ("2026-08", -2400)] {
            let err = conn
                .execute(
                    "INSERT INTO week_targets (week_id, target_minutes) VALUES (?1, ?2)",
                    (week_id, minutes),
                )
                .expect_err("negative target_minutes must be rejected");
            assert!(is_constraint_violation(&err), "got {err:?}");
        }
        assert_eq!(count(&conn, "week_targets"), 0);

        for (week_id, minutes) in [("2026-09", 0_i64), ("2026-10", 2400)] {
            conn.execute(
                "INSERT INTO week_targets (week_id, target_minutes) VALUES (?1, ?2)",
                (week_id, minutes),
            )
            .expect("non-negative target_minutes is accepted");
            let stored: i64 = conn
                .query_row(
                    "SELECT target_minutes FROM week_targets WHERE week_id = ?1",
                    [week_id],
                    |r| r.get(0),
                )
                .expect("read back");
            assert_eq!(stored, minutes);
        }
    }

    // T9
    #[test]
    fn week_target_primary_key_supports_upsert_by_week_id() {
        let (_dir, path) = temp_db();
        let conn = connect_at(&path).expect("connect");

        conn.execute(
            "INSERT INTO week_targets (week_id, target_minutes) VALUES (?1, ?2)",
            ("2026-07", 2400_i64),
        )
        .expect("first insert");

        let err = conn
            .execute(
                "INSERT INTO week_targets (week_id, target_minutes) VALUES (?1, ?2)",
                ("2026-07", 1800_i64),
            )
            .expect_err("duplicate week_id must be rejected");
        assert!(is_constraint_violation(&err), "got {err:?}");

        conn.execute(
            "INSERT INTO week_targets (week_id, target_minutes) VALUES (?1, ?2) \
             ON CONFLICT(week_id) DO UPDATE SET target_minutes = excluded.target_minutes",
            ("2026-07", 1800_i64),
        )
        .expect("upsert");

        assert_eq!(count(&conn, "week_targets"), 1);
        let stored: i64 = conn
            .query_row(
                "SELECT target_minutes FROM week_targets WHERE week_id = '2026-07'",
                [],
                |r| r.get(0),
            )
            .expect("read back");
        assert_eq!(stored, 1800);
    }

    // T10
    #[test]
    fn not_null_columns_reject_nulls() {
        let (_dir, path) = temp_db();
        let conn = connect_at(&path).expect("connect");

        let nullable_attempts: &[(&str, [Option<&str>; 3])] = &[
            ("at_utc", [None, Some("2026-09-12"), Some("start")]),
            ("date", [Some("2026-09-12T13:05:00Z"), None, Some("start")]),
            (
                "kind",
                [Some("2026-09-12T13:05:00Z"), Some("2026-09-12"), None],
            ),
        ];
        for (column, values) in nullable_attempts {
            let err = conn
                .execute(
                    "INSERT INTO punches (at_utc, \"date\", kind) VALUES (?1, ?2, ?3)",
                    rusqlite::params![values[0], values[1], values[2]],
                )
                .expect_err(&format!("punches.{column} NULL must be rejected"));
            assert!(is_constraint_violation(&err), "punches.{column}: {err:?}");
        }

        let err = conn
            .execute(
                "INSERT INTO notes (\"date\", body, created_at_utc) VALUES (?1, ?2, ?3)",
                rusqlite::params!["2026-09-12", None::<&str>, "2026-09-12T13:05:07Z"],
            )
            .expect_err("notes.body NULL must be rejected");
        assert!(is_constraint_violation(&err), "notes.body: {err:?}");

        let err = conn
            .execute(
                "INSERT INTO week_targets (week_id, target_minutes) VALUES (?1, ?2)",
                rusqlite::params!["2026-07", None::<i64>],
            )
            .expect_err("week_targets.target_minutes NULL must be rejected");
        assert!(is_constraint_violation(&err), "target_minutes: {err:?}");

        assert_eq!(count(&conn, "punches"), 0);
        assert_eq!(count(&conn, "notes"), 0);
        assert_eq!(count(&conn, "week_targets"), 0);
    }

    // T11
    #[test]
    fn connect_at_errors_when_parent_path_is_a_regular_file() {
        let dir = TempDir::new().expect("temp dir");
        let blocker = dir.path().join("blocker");
        std::fs::write(&blocker, b"not a directory").expect("write blocker");

        let err = connect_at(&blocker.join("sub").join("mlm.db"))
            .expect_err("a file where a directory is expected must be an error");
        assert!(
            matches!(err, DbError::CreateDataDir { .. }),
            "expected CreateDataDir, got {err:?}"
        );
        let message = err.to_string();
        assert!(!message.is_empty());
        assert!(
            message.contains("blocker"),
            "message lacks the path: {message}"
        );
    }

    // T12
    #[cfg(unix)]
    #[test]
    fn connect_at_errors_on_an_unwritable_directory() {
        use std::os::unix::fs::PermissionsExt;

        // Root bypasses mode bits, so the call would succeed for the
        // wrong reason.
        if unsafe { libc_geteuid() } == 0 {
            return;
        }

        let dir = TempDir::new().expect("temp dir");
        let locked = dir.path().join("locked");
        std::fs::create_dir(&locked).expect("create dir");
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000))
            .expect("chmod 000");

        let result = connect_at(&locked.join("mlm.db"));

        // Restore permissions so TempDir can clean up.
        let _ = std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755));

        let err = result.expect_err("unwritable directory must be an error");
        assert!(
            matches!(err, DbError::CreateDataDir { .. } | DbError::Open { .. }),
            "expected CreateDataDir or Open, got {err:?}"
        );
    }

    #[cfg(unix)]
    unsafe fn libc_geteuid() -> u32 {
        unsafe extern "C" {
            fn geteuid() -> u32;
        }
        unsafe { geteuid() }
    }

    // T13
    #[test]
    fn connect_at_errors_on_a_file_that_is_not_a_database() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("garbage.db");
        std::fs::write(&path, vec![0xABu8; 4096]).expect("write garbage");

        let err = connect_at(&path).expect_err("a non-database file must be an error");
        assert!(
            matches!(err, DbError::Open { .. } | DbError::Migrate { .. }),
            "expected Open or Migrate, got {err:?}"
        );
    }

    // T14
    #[test]
    fn baseline_schema_creates_the_documented_indexes() {
        let (_dir, path) = temp_db();
        let conn = connect_at(&path).expect("connect");

        let index_names = |table: &str| -> Vec<String> {
            let mut stmt = conn
                .prepare(&format!("PRAGMA index_list({table})"))
                .expect("prepare index_list");
            stmt.query_map([], |r| r.get::<_, String>(1))
                .expect("query index_list")
                .collect::<Result<Vec<_>, _>>()
                .expect("collect index_list")
        };

        assert!(index_names("punches").contains(&"idx_punches_date_at_utc_id".to_string()));
        assert!(index_names("notes").contains(&"idx_notes_date_created_at_utc_id".to_string()));

        let mut stmt = conn
            .prepare("PRAGMA index_info(idx_punches_date_at_utc_id)")
            .expect("prepare index_info");
        let columns: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(2))
            .expect("query index_info")
            .collect::<Result<_, _>>()
            .expect("collect index_info");
        assert_eq!(columns, vec!["date", "at_utc", "id"]);
    }

    // T15
    #[test]
    fn punch_ids_autoincrement_and_materialize_sqlite_sequence() {
        let (_dir, path) = temp_db();
        let conn = connect_at(&path).expect("connect");

        for (at_utc, kind) in [
            ("2026-09-12T09:00:00Z", "start"),
            ("2026-09-12T12:00:00Z", "end"),
            ("2026-09-12T13:00:00Z", "start"),
        ] {
            insert_punch(&conn, at_utc, "2026-09-12", kind).expect("insert");
        }

        let mut stmt = conn
            .prepare("SELECT id FROM punches ORDER BY id")
            .expect("prepare");
        let ids: Vec<i64> = stmt
            .query_map([], |r| r.get(0))
            .expect("query")
            .collect::<Result<_, _>>()
            .expect("collect");
        assert_eq!(ids.len(), 3);
        assert!(
            ids.windows(2).all(|w| w[0] < w[1]),
            "ids not increasing: {ids:?}"
        );

        // AUTOINCREMENT materializes sqlite_sequence: no test may assert an
        // exact table count in sqlite_master.
        let seq: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'sqlite_sequence'",
                [],
                |r| r.get(0),
            )
            .expect("query sqlite_master");
        assert_eq!(seq, 1);
    }

    // T16
    #[test]
    fn every_db_error_renders_one_actionable_line() {
        let io_err = std::io::Error::other("permission denied");
        let sqlite_err = rusqlite::Error::InvalidQuery;
        let migrate_err = rusqlite_migration::Error::InvalidUserVersion;

        let errors = vec![
            DbError::DataDirUnavailable,
            DbError::CreateDataDir {
                path: PathBuf::from("/tmp/mlm-test/data"),
                source: io_err,
            },
            DbError::Open {
                path: PathBuf::from("/tmp/mlm-test/mlm.db"),
                source: sqlite_err,
            },
            DbError::Migrate {
                path: Some(PathBuf::from("/tmp/mlm-test/mlm.db")),
                source: migrate_err,
            },
            DbError::Migrate {
                path: None,
                source: rusqlite_migration::Error::InvalidUserVersion,
            },
            DbError::Query(rusqlite::Error::InvalidQuery),
        ];

        for err in &errors {
            let message = err.to_string();
            assert!(!message.is_empty(), "empty Display for {err:?}");
            assert!(!message.ends_with('\n'), "trailing newline: {message:?}");
            assert!(!message.contains('\n'), "not one line: {message:?}");
            assert!(
                !message.starts_with("Error:"),
                "redundant prefix: {message:?}"
            );
        }

        use std::error::Error as _;
        assert!(errors[0].source().is_none());
        for err in &errors[1..] {
            assert!(err.source().is_some(), "missing source for {err:?}");
        }
    }

    // T17 — PLAN.md Milestone 3 AC: apply_migrations works path-free.
    #[test]
    fn apply_migrations_works_against_an_in_memory_connection() {
        let mut conn = Connection::open_in_memory().expect("open in memory");
        apply_migrations(&mut conn).expect("migrate in memory");

        assert_eq!(user_version(&conn), 1);
        assert_eq!(migrations().pending_migrations(&conn).expect("pending"), 0);
        insert_punch(&conn, "2026-09-12T09:00:00Z", "2026-09-12", "start").expect("insert");

        // Idempotent second application.
        apply_migrations(&mut conn).expect("second migrate");
        assert_eq!(count(&conn, "punches"), 1);
    }

    // T18 — PLAN.md Milestone 3 AC: MLM_DB_PATH is honored.
    #[test]
    fn db_path_resolution_prefers_a_non_empty_env_override() {
        let overridden =
            resolve_db_path(Some(OsString::from("/tmp/mlm-test/custom.db"))).expect("override");
        assert_eq!(overridden, PathBuf::from("/tmp/mlm-test/custom.db"));

        let empty = resolve_db_path(Some(OsString::from(""))).expect("empty override");
        let unset = resolve_db_path(None).expect("unset override");
        assert_eq!(empty, unset);
        assert_eq!(unset, data_dir().expect("data dir").join(DB_FILENAME));
        assert!(unset.ends_with("mlm.db"));
    }
}
