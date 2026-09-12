//! SQLite storage layer.
//!
//! DB lives in the platform app-data dir (via `directories::ProjectDirs`),
//! e.g. `~/.local/share/mlm/mlm.db` on Linux, `%APPDATA%\mlm\mlm.db` on Windows.

use std::path::PathBuf;

use directories::ProjectDirs;
use rusqlite::Connection;

/// Qualifier/org/app triple used to locate the platform data dir.
const QUALIFIER: &str = "";
const ORG: &str = "";
const APP: &str = "mlm";

pub fn data_dir() -> PathBuf {
    ProjectDirs::from(QUALIFIER, ORG, APP)
        .expect("could not determine application data directory")
        .data_dir()
        .to_path_buf()
}

pub fn db_path() -> PathBuf {
    data_dir().join("mlm.db")
}

pub fn connect() -> rusqlite::Result<Connection> {
    let dir = data_dir();
    std::fs::create_dir_all(&dir).expect("could not create application data directory");
    let conn = Connection::open(db_path())?;
    init_schema(&conn)?;
    Ok(conn)
}

fn init_schema(conn: &Connection) -> rusqlite::Result<()> {
    // TODO: real schema (entries table: date, start, stop, note). Placeholder for now.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS entries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL,
            start TEXT,
            stop TEXT,
            note TEXT
        );",
    )
}
