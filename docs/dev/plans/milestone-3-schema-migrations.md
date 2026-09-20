# Milestone 3 — Schema and migrations: implementation plan

> **Archived — historical planning record.** Not authoritative. Current spec: `docs/dev/SPEC.md`. Design/process history: `docs/dev/NOTES.md`.


**Status**: ready to hand to a TDD subagent.
**Spec basis**: SPEC.md §2.1, §2.2, §2.3, §6.1, §6.2, §8.2 (E6).
**Plan basis**: PLAN.md "Milestone 3", interface contracts 1 and 7.
**Files this milestone touches**: `src/db.rs` (rewrite), `Cargo.toml`
(`[dev-dependencies]` only, see §6.1). Nothing else — schema and
migrations only; `Punch`/`PunchKind`/`Note` belong solely to Milestone
4 (contract 8, §3.4 below). `src/cli.rs` / `src/main.rs` keep their
scaffold shape here — Milestones 7/10 replace those.

Unlike PLAN.md, this document deliberately names concrete SQL, types,
and function signatures. Where it completes something SPEC.md or
PLAN.md left underspecified, that is called out inline as **[DECISION]**
and collected in §7.

---

## 0. Verified facts about `rusqlite_migration` 2.6.0

These were read out of the vendored crate source
(`~/.cargo/registry/src/index.crates.io-*/rusqlite_migration-2.6.0/src/lib.rs`),
not recalled — PLAN.md asked for this to be confirmed rather than
assumed.

- Public API surface used here:
  - `M::up(sql: &'u str) -> M<'u>` — **`const fn`**, so a migration set
    can live in a `const`/`static` slice.
  - `M::comment(self, &str) -> M` — `const fn`, optional documentation.
  - `M::down(self, &str)`, `M::up_with_hook`, `M::foreign_key_check` —
    **not used** in this milestone (see §7.4).
  - `Migrations::from_slice(ms: &'m [M<'m>]) -> Migrations<'m>` —
    `const fn`, borrows; no allocation. Preferred over
    `Migrations::new(Vec<M>)` here.
  - `Migrations::to_latest(&self, conn: &mut Connection) -> Result<()>`
    — **takes `&mut Connection`**. This is the single most common
    compile-time trip-up; `connect()` must own a `mut` binding.
  - `Migrations::validate(&self) -> Result<()>` — self-consistency
    check on the migration set; cheap, worth one test (§5, T5).
  - `Migrations::current_version(&self, &Connection) -> Result<SchemaVersion>`
    and `pending_migrations(&self, &Connection) -> Result<i32>` — used
    by the idempotency tests.
- `SchemaVersion` is an enum: `NoneSet` | `Inside(NonZeroUsize)` |
  `Outside(NonZeroUsize)`, with `impl From<SchemaVersion> for usize`
  (`NoneSet` → `0`). Assert against the `usize` form, not the enum
  Display string.
- **Bookkeeping is `PRAGMA user_version`, not a table.** The crate
  stores the applied version in the SQLite file header via
  `PRAGMA user_version` / `pragma_update`. There is **no
  `schema_migrations` table**, and none will ever appear in
  `sqlite_master`. SPEC.md §2.2 says "A `schema_migrations` table (**or
  equivalent**)" — `user_version` is that equivalent, so this is
  compliant, but see §7.1: **tests must not assert a
  `schema_migrations` table exists.**
- Each migration is applied inside a transaction the crate opens
  itself, and `user_version` is set inside that same transaction. A
  migration that errors partway therefore rolls back — this is what
  makes §6.1's "a migration erroring partway" a clean hard-error rather
  than a half-applied schema.
- `rusqlite_migration::Error` variants (from `src/errors.rs`):
  `RusqliteError { query, err }`, `SpecifiedSchemaVersion(_)`,
  `InvalidUserVersion`, `MigrationDefinition(_)`, `ForeignKeyCheck(_)`,
  `Hook(_)`, `FileLoad(_)`, `Unrecognized(_)`. It implements
  `std::error::Error`. Do not match on variants in app code — wrap the
  whole thing (§3.2).

**Verdict on PLAN.md's assumption**: PLAN.md's phrasing ("an ordered,
embedded migration set applied on connect", `Migrations::new(...)` /
`to_latest`) matches the real API. The only correction is
`schema_migrations`-vs-`user_version` and the `&mut Connection`
requirement.

---

## 1. Migration set structure

**One migration, version 1**, containing the entire MVP baseline
schema. Rationale: nothing has shipped, so there is no deployed v0 to
step through; three separate migrations would buy nothing and only
multiply the version numbers a future change has to reason about.

```rust
// src/db.rs
use rusqlite_migration::{M, Migrations};

/// Ordered, embedded migration set. Index 0 == schema version 1.
///
/// INVARIANT: once a release ships, an entry in this slice is
/// FROZEN. Schema changes are appended as new entries, never edits.
const MIGRATION_SLICE: &[M<'static>] = &[
    M::up(MIGRATION_0001_BASELINE).comment("baseline: punches, notes, week_targets"),
];

/// Borrowing view over `MIGRATION_SLICE`. Cheap (no allocation) —
/// construct per call rather than caching in a static.
fn migrations() -> Migrations<'static> {
    Migrations::from_slice(MIGRATION_SLICE)
}
```

`MIGRATION_0001_BASELINE` is a `const &str` holding the SQL in §2. It
is executed as a batch, so multiple statements separated by `;` in one
string are fine and run in one transaction.

---

## 2. Exact SQL DDL (migration 0001, baseline)

```sql
-- Pre-release cleanup: the scaffold shipped an `entries` table on
-- developer machines with user_version still 0, so this migration will
-- run against those files and should not leave the dead table behind.
-- Harmless no-op on a fresh database. [DECISION-A, see §7.5]
DROP TABLE IF EXISTS entries;

-- SPEC.md §2.3: one row per timestamped start/end event.
CREATE TABLE punches (
    id      INTEGER PRIMARY KEY AUTOINCREMENT,
    at_utc  TEXT NOT NULL,
    "date"  TEXT NOT NULL,
    kind    TEXT NOT NULL CHECK (kind IN ('start', 'end'))
);

-- SPEC.md §2.3: "Index on `date` (and probably `at_utc` for ordering
-- within a date)". One composite covers both: `date` is the leftmost
-- prefix (so a standalone date index would be redundant), and the
-- trailing `at_utc, id` columns give Milestone 5 its exact required
-- sort order (§4.3 step 1) straight out of the index, plus a range
-- scan for Milestone 6's per-week date spans. [DECISION-B, §7.6]
CREATE INDEX idx_punches_date_at_utc_id
    ON punches ("date", at_utc, id);

-- SPEC.md §2.3: one row per work-log entry. No `at_utc` here by
-- design — a note attaches to a day, not an instant.
CREATE TABLE notes (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    "date"         TEXT NOT NULL,
    body           TEXT NOT NULL,
    created_at_utc TEXT NOT NULL
);

-- Supports Milestone 4's "notes for a date, in insertion order" read.
CREATE INDEX idx_notes_date_created_at_utc_id
    ON notes ("date", created_at_utc, id);

-- SPEC.md §2.3: sparse overrides only. A week with no row uses the
-- default 2400-minute target (that default lives in Milestone 6, NOT
-- as a SQL DEFAULT here — there is no row to default).
CREATE TABLE week_targets (
    week_id        TEXT PRIMARY KEY,
    target_minutes INTEGER NOT NULL CHECK (target_minutes >= 0)
);
```

### 2.1 Column-by-column notes for the implementer

| table.column | type | constraint | value contract |
|---|---|---|---|
| `punches.id` | `INTEGER PRIMARY KEY AUTOINCREMENT` | implicit NOT NULL/UNIQUE | surrogate; also the §4.3 insertion-order tie-break. NOTES.md decision 9 mandates `AUTOINCREMENT` explicitly — do not "optimize" it to a bare `INTEGER PRIMARY KEY`. |
| `punches.at_utc` | `TEXT` | `NOT NULL` | canonical fixed-width RFC 3339 UTC: `%Y-%m-%dT%H:%M:%SZ` (exactly 20 chars, e.g. `2026-09-12T13:05:00Z`). Seconds are always `00` (§4.1 minute granularity). Fixed width is what makes it lexically sortable. |
| `punches."date"` | `TEXT` | `NOT NULL` | local calendar date `YYYY-MM-DD`, computed app-side from `at_utc` + the local tz **at that instant** (§2.1). Never derived in SQL. |
| `punches.kind` | `TEXT` | `NOT NULL CHECK (kind IN ('start','end'))` | lowercase only. The CHECK is case-sensitive (default `BINARY` collation), so `'START'` is rejected — that is intended, and is a test case. |
| `notes.id` | `INTEGER PRIMARY KEY AUTOINCREMENT` | | |
| `notes."date"` | `TEXT` | `NOT NULL` | same shape as `punches."date"`. |
| `notes.body` | `TEXT` | `NOT NULL` | already trimmed by the caller; no length cap, no charset restriction (§2.3, NOTES.md 35). Emptiness is rejected **in app code** (Milestone 4 / §6.1), not by a CHECK — see §7.7. |
| `notes.created_at_utc` | `TEXT` | `NOT NULL` | same canonical format; **real seconds, not forced to `:00`** — see [DECISION-C], §7.8. |
| `week_targets.week_id` | `TEXT PRIMARY KEY` | | normalized `YYYY-WW` (zero-padded, no `W`), per Milestone 2's formatter. The PK is Milestone 8's upsert conflict target: `INSERT INTO week_targets (week_id, target_minutes) VALUES (?1, ?2) ON CONFLICT(week_id) DO UPDATE SET target_minutes = excluded.target_minutes`. |
| `week_targets.target_minutes` | `INTEGER` | `NOT NULL CHECK (target_minutes >= 0)` | `0` legal (§2.3, F7b); negative rejected at both the parser (§6.1/E9) and here. |

### 2.2 Deliberately NOT in the DDL

Do not add any of these in Milestone 3; each is listed so the
implementer doesn't "helpfully" include it and break a test:

- No format CHECKs on `at_utc` / `date` / `week_id` (e.g. `LIKE` globs).
  SPEC.md §2.3 specifies exactly two CHECK clauses; adding more widens
  the schema contract beyond the spec and beyond PLAN.md's acceptance
  criteria. (§7.7 records this as a deliberate call.)
- No `CHECK (length(trim(body)) > 0)` on `notes.body` — E5 is Milestone
  4's, checked *before* the trim, which SQL cannot express.
- No foreign keys anywhere (no relationships exist), therefore no
  `PRAGMA foreign_keys` and no `M::foreign_key_check()`.
- No `DEFAULT` on `week_targets.target_minutes` — the 40h default is a
  *missing-row* semantic, not a column default (§2.3).
- No `PRAGMA journal_mode = WAL`, no `busy_timeout`. SPEC.md §2.2
  explicitly declines both.
- No `down` migrations (§7.4).
- No `WITHOUT ROWID` on `week_targets` — `AUTOINCREMENT` on the other
  two forbids it there anyway, and consistency is worth more than the
  micro-optimisation.

---

## 3. Rust API

### 3.1 Module layout

All of the below lives in `src/db.rs`, replacing the file's current
contents in full. Two `.expect()` calls in the scaffold (`data_dir()`'s
and `connect()`'s) are removed — see §3.3; they are precisely the
"panics silently instead of surfacing an error" behavior PLAN.md's last
acceptance criterion rules out.

### 3.2 Error type — PLAN.md interface contract 7

Contract 7 ("error type/shape for hard errors") is listed in PLAN.md as
unresolved. This section **completes it**; see §7.2 for what was
invented versus given.

Constraints that shaped the choice: `Cargo.toml` has no `thiserror`
and no `anyhow`, and Milestone 3 has no mandate to add runtime
dependencies. So: hand-written impls, no new deps.

```rust
// src/db.rs
use std::path::PathBuf;

#[derive(Debug)]
pub enum DbError {
    /// `ProjectDirs::from(...)` returned None — no home/app-data dir
    /// on this platform. §6.1 hard error, NOT first-run bootstrap.
    DataDirUnavailable,
    /// `create_dir_all` failed (permissions, a file where a directory
    /// must go, read-only filesystem).
    CreateDataDir { path: PathBuf, source: std::io::Error },
    /// `Connection::open` failed (unreadable file, bad permissions).
    Open { path: PathBuf, source: rusqlite::Error },
    /// The migration set failed to apply (corrupt file, a migration
    /// erroring partway — §6.1 names both).
    Migrate { path: PathBuf, source: rusqlite_migration::Error },
    /// Reserved for Milestones 4/8's statement execution. Declared
    /// here so those milestones extend rather than redefine DbError.
    Query(rusqlite::Error),
}

impl std::fmt::Display for DbError { /* one-line, user-facing */ }
impl std::error::Error for DbError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> { /* ... */ }
}
```

Rules the implementer must follow:

- `Display` produces **one line, no trailing newline, no `Error: `
  prefix**, suitable for writing straight to stderr by the eventual
  `main`. Include the offending path where one exists — an
  unactionable "database error" is exactly what §6.1's "missing
  permissions, corrupt file" cases need to avoid. Suggested wording:
  - `DataDirUnavailable` → `could not determine the application data directory`
  - `CreateDataDir` → `could not create data directory {path}: {source}`
  - `Open` → `could not open database {path}: {source}`
  - `Migrate` → `could not migrate database {path}: {source}`
- `source()` returns the wrapped error for every variant that has one,
  so a future `--verbose` can walk the chain. Do not fold the source
  into the `Display` of a *parent* wrapper later — it's already in this
  string, which is intentional (one line has to stand alone).
- **No `From<rusqlite::Error> for DbError` blanket impl.** A blanket
  `?` conversion would silently classify an `open` failure as a
  `Query`. Every call site maps explicitly with `.map_err(...)`.
- `DbError` is not `PartialEq` (the io/rusqlite sources aren't). Tests
  assert on the *variant* via `matches!(err, DbError::Open { .. })`, or
  on the `Display` string, never on equality.

**Composition with the eventual app-wide error — resolved, no shared
enum needed**: cross-plan review settled on `anyhow` as the project's
CLI-boundary error convention (PLAN.md contract 7) rather than a
hand-rolled `AppError` enum. Command handlers (Milestone 7 onward)
return `anyhow::Result<()>`; `?` on a `DbError` auto-converts via
`anyhow`'s blanket `From<E: std::error::Error>` impl, no manual
wrapper or `From` impl needed anywhere. Milestone 3's only obligation
is unchanged: `DbError` must be a well-behaved `std::error::Error`
(`Display` + `source()`), which §3.2 already satisfies.

### 3.3 Connection / bootstrap functions

```rust
use std::path::{Path, PathBuf};
use directories::ProjectDirs;
use rusqlite::Connection;

const QUALIFIER: &str = "";
const ORG: &str = "";
const APP: &str = "mlm";
const DB_FILENAME: &str = "mlm.db";

/// Platform app-data directory, e.g. `~/.local/share/mlm` (Linux),
/// `%APPDATA%\mlm\data` (Windows). Pure: no filesystem side effects.
pub fn data_dir() -> Result<PathBuf, DbError>;

/// `data_dir()/mlm.db`. Pure.
pub fn default_db_path() -> Result<PathBuf, DbError>;

/// THE testable core. Opens (creating if absent) the database at
/// `path`, creating `path`'s parent directory if absent, and applies
/// all pending migrations. Returns a ready-to-use connection.
pub fn connect_at(path: &Path) -> Result<Connection, DbError>;

/// Production entry point: `connect_at(&default_db_path()?)`.
pub fn connect() -> Result<Connection, DbError>;
```

`connect_at`'s body, exactly:

1. `if let Some(parent) = path.parent()` and `!parent.as_os_str().is_empty()`:
   `std::fs::create_dir_all(parent).map_err(|source| DbError::CreateDataDir { path: parent.to_path_buf(), source })?`
   — `create_dir_all` returns `Ok(())` when the directory already
   exists, which is the whole first-run/nth-run unification.
2. `let mut conn = Connection::open(path).map_err(|source| DbError::Open { path: path.to_path_buf(), source })?;`
   — SQLite creates the file here when it is absent. A missing file is
   never checked for and never branched on.
3. `migrations().to_latest(&mut conn).map_err(|source| DbError::Migrate { path: path.to_path_buf(), source })?;`
   — on a brand-new file `user_version` is 0, so migration 0001 applies;
   on an already-migrated file `user_version` is 1 and this is a no-op.
4. `Ok(conn)`

**How first-run bootstrap is distinguished from a genuine failure
(§6.1 / NOTES.md 33) — the key design statement:** it *isn't*, and
deliberately so. There is no "is this a first run?" predicate anywhere.
Absence is handled by construction — `create_dir_all` is idempotent,
`Connection::open` creates, `to_latest` applies whatever is pending
from version 0. A failure is defined purely as *any of those three
operations returning `Err`*. Consequences the implementer must respect:

- Never `.unwrap()`, `.expect()`, or `panic!` in this module. Every
  failure is a returned `DbError`.
- Never pre-check with `path.exists()` / `parent.is_dir()` and branch.
  That reintroduces a TOCTOU race and a second, divergent definition of
  "first run".
- Never swallow an error as "probably just first run". A
  permission-denied `create_dir_all` looks nothing like success and
  must not be treated as one.

### 3.4 Punch value shape — superseded by cross-plan reconciliation

**This section's original plan (landing `Punch`/`PunchKind`/`Note` and
the UTC-text helpers in a new `src/model.rs` owned by Milestone 3) is
superseded.** Cross-plan review found three milestones independently
claiming ownership of this same type (this plan's `src/model.rs`,
Milestone 4's `src/storage.rs`, and Milestone 5's own `src/stint.rs`
copy) — exactly the divergence contract-pinning was meant to prevent.

**Resolved (PLAN.md contract 8): Milestone 4 (`src/storage.rs`) is the
sole owner** of `Punch`, `PunchKind`, `Note`, and the UTC text-format
helpers described below. **Milestone 3's scope is schema/migrations
only — no Rust value types, no `src/model.rs`.** The DDL, `CHECK`
constraints, and indexes in §§2-3 of this plan are unaffected by this
change; only the "define the row types here" portion is removed.

The design content below is preserved as input for whoever implements
Milestone 4, since it's still correct — just relocated:

- Full row (not a lighter intermediate value): four fields on `Punch`,
  `id` included since §4.3 step 1's tie-break needs it directly.
- `PunchKind` as a two-variant enum with `as_str()`/`parse()` as the
  single source of truth for the strings the `kind` CHECK accepts,
  additionally deriving `PartialOrd, Ord` with `Start` before `End`
  (contract 8 — Milestone 5's pairing tie-break needs this ordering).
- The canonical UTC text constant, `"%Y-%m-%dT%H:%M:%SZ"`, with the
  serialization trap called out loudly: **never** use
  `DateTime::to_rfc3339()` (emits `+00:00`, breaks the fixed-width
  lexical sort §2.3 depends on) or `to_rfc3339_opts` (can emit
  sub-second digits) — always format/parse against the literal
  constant.

---

## 4. Sequence of work for the TDD agent

1. Add `tempfile` to `[dev-dependencies]` (§6.1). Write the temp-dir
   test helper first (§6.2).
2. Write T1–T5 (bootstrap + migration bookkeeping). They fail: `db.rs`
   still has the scaffold `entries` table.
3. Write the DDL const and `migrations()`; rewrite `connect_at`. T1–T5 go green.
4. Write T6–T10, T14–T15 (constraint + index + autoincrement tests)
   against the now-existing schema. Adjust DDL until green.
5. Write `DbError` + T16, then T11–T13 (failure paths). Remove the last
   `.expect()`s.
6. `Punch`/`PunchKind`/`Note` and their round-trip tests (formerly
   T17-T18 here) now belong to Milestone 4 — nothing to do for them
   in this milestone.
7. `main.rs` currently calls `db::connect().expect(...)`. Minimal
   adaptation only: keep it compiling (`match`/`eprintln!`+`exit(1)` is
   fine); the real wiring is Milestone 7's. Do not touch `cli.rs`.

---

## 5. Test cases

Location: `#[cfg(test)] mod tests` inside `src/db.rs` (these assert on
private DDL details and private error mapping, so unit tests, not
`tests/`). T17/T18 (the UTC-text round-trip and `PunchKind` parse
tests) have moved to Milestone 4's plan along with the types they
test (§3.4, contract 8).

Mapping column: SPEC.md §8 flow / §6 clause each test discharges.

| # | Test | Asserts | Maps to |
|---|---|---|---|
| **T1** | `connect_at` on a temp path whose parent directory does **not** exist | returns `Ok`; the parent directory now exists; the db file now exists | §6.1 "missing app-data directory or db file is expected and handled silently"; E6's *negative* case; PLAN AC 1 |
| **T2** | after T1's connect, `PRAGMA table_info` for each of `punches`, `notes`, `week_targets` | each table exists; the expected column **names** are present with the expected declared types and NOT NULL flags. Assert by name lookup, never by ordinal — SPEC.md fixes no column order | §2.3; PLAN AC 1 |
| **T3** | after a fresh connect, `PRAGMA user_version` and `migrations().pending_migrations(&conn)` | `user_version == 1`; `pending_migrations == 0`; `usize::from(current_version(&conn)) == 1`. **Also assert `sqlite_master` contains no table named `schema_migrations`** — encodes §0's finding so nobody "fixes" it later | §2.2 |
| **T4** | idempotent re-migration: connect, insert one valid punch row, drop the connection, `connect_at` the **same path** again | second call returns `Ok`; `user_version` still `1`; `pending_migrations` still `0`; the punch row inserted between the two connects is still readable and unmodified | PLAN AC 2 ("no duplicate migration application, no data loss") |
| **T5** | `migrations().validate()` | returns `Ok` | crate-level consistency guard; catches a malformed migration set at test time rather than on a user's first run |
| **T6** | insert into `punches` with `kind` = each of `'START'`, `'Start'`, `'pause'`, `''`, `'starts'` | every one returns `Err`; the error is a constraint violation (`rusqlite::Error::SqliteFailure` with `ErrorCode::ConstraintViolation`); `SELECT COUNT(*) FROM punches` is unchanged | §2.3 `kind` CHECK; PLAN AC 3 |
| **T7** | insert `kind = 'start'` and `kind = 'end'` | both `Ok`; both readable back with the exact stored string | §2.3 |
| **T8** | insert into `week_targets`: `target_minutes` = `-1` and `-2400` | both `Err` (constraint violation), nothing written. Then `0` and `2400` → both `Ok`, both read back exactly | §2.3 `target_minutes >= 0`; PLAN AC 4; schema-layer half of F7b and E9 |
| **T9** | insert `week_targets` row for `'2026-07'` twice | second plain `INSERT` is `Err` (PK uniqueness); the `INSERT ... ON CONFLICT(week_id) DO UPDATE SET target_minutes = excluded.target_minutes` form succeeds and leaves exactly one row with the new value | pins Milestone 8's upsert AC at the schema layer (§2.3, F7 storage half) |
| **T10** | insert a `punches` row with `at_utc` NULL; separately `"date"` NULL; separately `kind` NULL; a `notes` row with `body` NULL; a `week_targets` row with `target_minutes` NULL | every one `Err` | §2.3 NOT NULL columns |
| **T11** | `connect_at` where the **parent path is an existing regular file** (create a temp file, then ask for `<that file>/sub/mlm.db`) | returns `Err`, does **not** panic; `matches!(err, DbError::CreateDataDir { .. })`; `Display` is non-empty and names the path | §6.1 "database open failure"; **E6**; PLAN AC 5 ("a file where a directory is expected") |
| **T12** | `#[cfg(unix)]` — create a temp dir, `chmod 0o000`, then `connect_at(dir.join("mlm.db"))` | returns `Err`, no panic. Accept **either** `CreateDataDir` or `Open` (see §7.9 — which one fires is platform-dependent). Guard: skip the test when running as root, since root bypasses mode bits and the call would succeed | §6.1 "missing permissions"; **E6**; PLAN AC 5 |
| **T13** | write ~4 KiB of non-SQLite bytes to a temp file, then `connect_at` that path | returns `Err`, no panic. Accept either `Open` or `Migrate` (§7.9) | §6.1 "corrupt file"; **E6** |
| **T14** | after a fresh connect, `PRAGMA index_list(punches)` / `index_list(notes)` | `idx_punches_date_at_utc_id` and `idx_notes_date_created_at_utc_id` are present; `PRAGMA index_info` on the punches index reports columns `date, at_utc, id` in that order | PLAN AC "indexing intent on `date`/`at_utc`"; §2.3 |
| **T15** | insert three punches, read their `id`s; assert strictly increasing. Assert a table named `sqlite_sequence` exists | documents that `AUTOINCREMENT` materializes `sqlite_sequence`, so **no test may assert "sqlite_master contains exactly three tables"** | NOTES.md decision 9; guards a likely test-authoring mistake |
| **T16** | construct one of each `DbError` variant; call `Display` and `source()` | every `Display` is a single line, non-empty, with no trailing newline; every wrapping variant's `source()` is `Some` | contract 7; §6.1's "message on stderr" requirement |

(T17/T18 — the UTC-text round-trip and `PunchKind::parse` tests —
moved to Milestone 4's plan with the types they test, per §3.4 above.)

Explicitly **out of scope for this milestone's tests** (they belong to
later milestones, listed so the TDD agent doesn't over-reach): note
trimming and empty-note rejection (M4/E5), local↔UTC conversion
correctness and DST (M4/F12), stint pairing (M5), the default 2400
target (M6), CLI exit codes (M7/M8), and **E6 end-to-end through an
actual command** — PLAN.md's cross-cutting section already assigns that
re-test to the final pass, and §7.3 below flags what it will need.

---

## 6. Test infrastructure

### 6.1 `tempfile` dev-dependency

Every test above needs an isolated filesystem path, and none may touch
the real `ProjectDirs` location (a test run must never write to the
developer's actual `~/.local/share/mlm/mlm.db`). `Cargo.toml` currently
has **no** `[dev-dependencies]` section and no temp-dir crate.

Recommended: add

```toml
[dev-dependencies]
tempfile = "3"
```

This is a dev-only dependency — it does not enter the shipped binary.
It is the one `Cargo.toml` change this milestone needs; flagged here
because PLAN.md does not sanction dependency changes and the reviewer
should see it coming. Fallback if rejected: a hand-rolled guard around
`std::env::temp_dir().join(format!("mlm-test-{}-{}", std::process::id(), n))`
with a `Drop` impl that removes the tree — roughly 25 lines, and
strictly worse (leaks on panic-in-Drop, needs its own counter).

### 6.2 Helper

```rust
#[cfg(test)]
fn temp_db() -> (tempfile::TempDir, PathBuf) // dir kept alive by the caller
```
Returning the `TempDir` alongside the path matters: dropping it deletes
the directory, so tests must bind it (`let (_dir, path) = temp_db();`)
rather than `temp_db().1`.

### 6.3 Every test calls `connect_at`, never `connect`

`connect()` resolves the real user data directory. No test in this
milestone (or any later one) may call it. This is the reason
`connect_at` exists as a separate public function rather than
`connect()` doing the path resolution inline.

---

## 7. Ambiguities, risks, decisions, and disagreements

### 7.1 SPEC.md §2.2 says `schema_migrations`; the crate uses `PRAGMA user_version`
**Severity: low (wording), but a high-probability test-authoring trap.**
SPEC.md's "(or equivalent)" makes this compliant, and no spec change is
needed. But an agent writing tests from the spec text alone would very
plausibly assert a `schema_migrations` table exists and then "fix"
`db.rs` to create one. T3 asserts the opposite on purpose.

**Related operational caveat (out of MVP scope, worth knowing):**
`user_version` lives in the SQLite file header, not in
`sqlite_master`. A database reconstructed via `.dump` / `.read` loses
it, resetting to 0 — a subsequent `to_latest` would try to re-apply
migration 0001 against populated tables and fail with "table punches
already exists". Nothing to do about it now; do not build backup
tooling in this milestone. Just don't document `.dump` as a supported
backup path.

### 7.2 Contract 7 (error shape) — resolved
`DbError` (§3.2's enum, `Display`/`source` conventions, no-blanket-
`From` rule) stands as designed and needed no change. The project-wide
question this section originally deferred — hand-rolled wrapper vs.
`thiserror`/`anyhow` — is now decided at the project level (not
Milestone 3's call, correctly deferred here): **`anyhow`**, adopted as
a real dependency, used at the CLI boundary from Milestone 7 onward.
`DbError` itself is unaffected — it stays a hand-written, precise
`std::error::Error` type; `anyhow` only removes the need for a
hand-rolled `AppError` wrapper around it and the other wave-1 types.

### 7.3 No way to point the binary at a test database — blocks end-to-end E6
PLAN.md's cross-cutting section requires E6 re-tested "end-to-end
through at least one write command and one read command". With
`connect()` hard-wired to `ProjectDirs`, the only ways to do that are
(a) corrupt the developer's real database, or (b) make the real
app-data path unopenable — both hostile and non-hermetic.

**Recommendation**: add an env-var override, e.g.
`MLM_DB_PATH`, consulted by `default_db_path()`: if set and non-empty,
use it verbatim; otherwise fall back to `ProjectDirs`. Three lines, no
new dependency, and it makes every CLI-level integration test in
Milestones 7/8/10/11 hermetic — not just E6.

**But this is a surface addition SPEC.md does not mention**, so it is
flagged rather than assumed. Milestone 3 should implement it **only if
the reviewer signs off**; otherwise it must be resolved before the
first CLI milestone, because the alternative is that every later
milestone's integration tests are either non-hermetic or skipped. If
adopted, it also needs a one-line SPEC.md note, which is out of this
milestone's file scope.

### 7.4 No `down` migrations — deliberate
`M::down` exists in the crate and is unused. MVP never rolls back
(`to_version` backwards is the only consumer), and an unused, untested
`down` is worse than no `down`. If a later milestone ever wants
`to_version`, it must add `down` to *every* migration, since the crate
errors with `MigrationDefinition(DownNotDefined)` on a gap.

### 7.5 [DECISION-A] `DROP TABLE IF EXISTS entries` inside migration 0001
The scaffold creates an `entries` table and never sets `user_version`.
Any developer who has run `cargo run` already has such a file at
`user_version = 0`. Migration 0001 will therefore run against it and
*succeed* (no name collisions), silently leaving a dead `entries` table
behind forever. The `DROP TABLE IF EXISTS` is a no-op on a fresh
database and cleans up those pre-release files.

**Counter-argument the reviewer should weigh:** a migration that drops
a table is normally a red flag, and this one is only safe because
nothing has shipped and `entries` holds no spec-relevant data. If it
makes anyone uncomfortable, dropping the line costs only a vestigial
table on a handful of dev machines. Either choice is defensible; I
recommend keeping it, and it must never be imitated post-release.

### 7.6 [DECISION-B] One composite index instead of two
SPEC.md §2.3 says "Index on `date` (and probably `at_utc` for ordering
within a date)" — a hedge, not a specification. `("date", at_utc, id)`
serves both stated purposes (a standalone `date` index would be a
redundant leftmost prefix), delivers §4.3's exact sort order without a
sort step, covers Milestone 6's per-week date-range scans, and is a
covering index for Milestone 5's read. A separate `at_utc`-only index
would serve no query this application issues.

### 7.7 No format or non-empty CHECKs — deliberate, and a mild disagreement with nobody
Tempting to add `CHECK ("date" LIKE '____-__-__')` etc. as defense in
depth. Declined: SPEC.md §2.3 enumerates exactly two CHECK clauses,
PLAN.md's acceptance criteria test exactly those two, and extra
constraints would make the schema reject data the spec considers valid
if any app-side format ever legitimately shifts. The invariant is
instead enforced at the single serialization chokepoint (Milestone
4's `UTC_TEXT_FORMAT` constant and its round-trip tests, §3.4).
Recorded because it is a real fork, not an oversight.

### 7.8 [DECISION-C] `notes.created_at_utc` keeps real seconds — a genuine spec tension
SPEC.md §4.1 says "There is no seconds precision anywhere — everything
is minute-granular." SPEC.md §2.3 says `created_at_utc` is the
"insertion-order tiebreaker for same-day notes", and PLAN.md's
Milestone 4 AC demands notes come back in insertion order "using the
insertion-order tiebreaker column, not just `id` incidentally".

**These conflict.** At minute granularity, two notes added in the same
minute get identical `created_at_utc` values and the column tiebreaks
nothing.

Resolution proposed: §4.1's minute-granularity rule is about
*user-entered time* (the `TIME` grammar, punch instants, durations) —
`created_at_utc` is a machine-generated bookkeeping timestamp the user
never types or sees, so it stores true seconds. Same canonical
fixed-width format either way. Reads order by `created_at_utc, id`, so
ordering stays total even for two notes in the same second.

This affects Milestone 4 more than Milestone 3 (Milestone 3 only
declares the column), but it must be settled now because it is a
storage-format decision. **Flagging for explicit sign-off** — the
alternative (minute granularity, `id` as the real tiebreaker) is also
coherent, it just makes `created_at_utc` decorative and mildly
contradicts PLAN.md's "not just `id` incidentally".

### 7.9 Which error variant a broken path produces is platform-dependent
For T12/T13, whether the failure surfaces from `create_dir_all`,
`Connection::open`, or the first statement inside `to_latest` depends
on the OS and on SQLite's lazy file handling (`open` can succeed on a
path whose first real read then fails). Tests must therefore assert
"returns `Err` and does not panic", accepting a small set of variants,
rather than pinning one exact variant. Over-specifying here produces a
test that passes on Linux and fails on Windows.

Related: T12 is `#[cfg(unix)]` **and** must skip under root. CI that
runs tests as root inside a container will otherwise see `chmod 0o000`
have no effect and the test fail for the wrong reason.

### 7.10 Concurrency, per SPEC.md §2.2 — not a bug to fix here
Two simultaneous `mlm` invocations against a fresh database can collide
inside migration 0001's transaction; the loser gets `SQLITE_BUSY`,
which surfaces as `DbError::Migrate` and a nonzero exit. That is
exactly SPEC.md §2.2's stated, accepted behavior ("trusted to fail
safely... no explicit WAL/busy-timeout tuning is planned"). Do not add
a busy-timeout to make a flaky test pass — fix the test instead.

### 7.11 Minor notes
- `date` is a SQLite built-in *function* name but is not a reserved
  keyword; bare `date` as a column name parses fine. The DDL quotes it
  as `"date"` anyway for readability and to avoid any future parser
  ambiguity. Queries in later milestones should quote it too.
- `to_latest` takes `&mut Connection`. A `let conn = Connection::open(..)?`
  binding will not compile; it must be `let mut conn`.
- `AUTOINCREMENT` (mandated by NOTES.md decision 9) adds a
  `sqlite_sequence` row-write per insert and creates the
  `sqlite_sequence` table. Both are fine at this scale; T15 documents
  the table's existence so no test asserts an exact table count.
- PLAN.md's Milestone 3 scope says "Replace `db.rs`'s placeholder
  `entries` table" — note that `main.rs` also calls
  `db::connect().expect("failed to open database")`. That `.expect` is
  a §6.1/§6.3 violation (panic instead of a stderr message and a clean
  nonzero exit), but proper handling belongs to Milestone 7. Milestone
  3 should do the minimum to keep `main.rs` compiling against the new
  `Result<Connection, DbError>` signature and leave the rest alone.

---

## 8. Acceptance checklist (maps 1:1 to PLAN.md Milestone 3)

- [ ] Connecting against a fresh/missing app-data directory creates the
      directory and database and leaves all three tables present with
      the documented columns — no error. → **T1, T2**
- [ ] Re-connecting against an already-migrated database is a no-op, no
      duplicate application, no data loss. → **T3, T4**
- [ ] A punch with `kind` outside `start`/`end` is rejected by the
      schema itself. → **T6** (+ **T7** positive control)
- [ ] A negative `target_minutes` is rejected by the schema itself;
      zero is accepted. → **T8**
- [ ] A connection failure against an unopenable path surfaces as an
      error rather than panicking or succeeding. → **T11, T12, T13**
      (+ **T16** for the message shape)
- [ ] Indexing intent on `date`/`at_utc` is realized. → **T14**
- [ ] No `.unwrap()`/`.expect()`/`panic!` remains in `src/db.rs`.
- [ ] Contract 7 (error shape) is concretely landed for wave-1
      consumers. → **T16**. (Contract 8 — punch/note value shape — is
      Milestone 4's responsibility now, not this milestone's.)
