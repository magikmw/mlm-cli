# Milestone 3 — Schema and migrations: completion report

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.


**Branch**: `milestone-3`. **Files touched**: `src/db.rs` (full rewrite),
`Cargo.toml` (`[dev-dependencies]` only), `Cargo.lock` (dep resolution).
Nothing else — `cli.rs`, `time.rs`, `main.rs` untouched.

## What landed

`src/db.rs`, replacing the scaffold `entries` placeholder:

- **Migration set**: one migration, schema version 1, as
  `const MIGRATION_SLICE: &[M<'static>]` + `fn migrations() -> Migrations<'static>`
  (`Migrations::from_slice`, no allocation). Baseline DDL is exactly the
  plan's §2: `punches` (id AUTOINCREMENT, at_utc, "date",
  kind CHECK IN ('start','end')), `notes` (id, "date", body,
  created_at_utc), `week_targets` (week_id PK, target_minutes CHECK >= 0),
  plus `idx_punches_date_at_utc_id ("date", at_utc, id)` and
  `idx_notes_date_created_at_utc_id ("date", created_at_utc, id)`, and the
  leading `DROP TABLE IF EXISTS entries` (DECISION-A, kept as recommended).
  No extra CHECKs, no FKs, no WAL/busy_timeout, no `down` migrations.
- **`DbError`** (contract 7): local concrete enum with `DataDirUnavailable`,
  `CreateDataDir { path, source }`, `Open { path, source }`,
  `Migrate { path, source }`, `Query(rusqlite::Error)`. One-line `Display`
  with the offending path, no `Error:` prefix, no trailing newline;
  `source()` returns the wrapped error for every wrapping variant. No
  blanket `From<rusqlite::Error>`. Not `PartialEq`.
- **Public API** (the part downstream milestones depend on):

```rust
pub const DB_PATH_ENV: &str = "MLM_DB_PATH";
pub fn data_dir() -> Result<PathBuf, DbError>;          // pure
pub fn default_db_path() -> Result<PathBuf, DbError>;   // pure; honors MLM_DB_PATH
pub fn apply_migrations(conn: &mut Connection) -> Result<(), DbError>;
pub fn connect_at(path: &Path) -> Result<Connection, DbError>;
pub fn connect() -> Result<Connection, DbError>;        // connect_at(&default_db_path()?)
```

  `connect_at` = `create_dir_all(parent)` → `Connection::open` →
  `apply_migrations`, each mapped to its own `DbError`. No first-run
  predicate, no `path.exists()` branching, and **no `unwrap`/`expect`/
  `panic!` anywhere in the non-test half of the module** (verified by grep).
- The scaffold's `db_path()` is gone (renamed `default_db_path()`); nothing
  referenced it.

## Verification

- `cargo build`: **pass**.
- `cargo test`: **18 passed, 0 failed** (all in `src/db.rs`'s `mod tests`).
  T1–T16 exactly as the plan lists them, plus two extra discharging PLAN.md
  Milestone 3 ACs the milestone plan doc predated: `apply_migrations`
  against `Connection::open_in_memory()` (idempotent, reaches
  `user_version = 1`), and `MLM_DB_PATH` precedence.
- `cargo fmt`: applied, clean.
- `cargo clippy --all-targets -- -D warnings`: **clean for `src/db.rs`** —
  zero findings in this milestone's code. The command as a whole still
  fails on **three pre-existing `dead_code` errors in `src/time.rs`**
  (`parse_hm`, `between`, `format_duration` never used). Confirmed
  pre-existing by stashing this milestone's diff and re-running on the
  base commit — identical three errors. `time.rs` is Milestone 1's file and
  explicitly out of this milestone's scope, so they were left alone;
  Milestone 1 clears them by giving those functions real callers/tests.
- **Tests have teeth** (mutation-checked, then reverted): removing the
  `kind` CHECK, the `target_minutes` CHECK, and the punches index made
  exactly the three corresponding tests fail.

## Deviations from the plan (all deliberate)

1. **`MLM_DB_PATH` and `apply_migrations` were implemented**, though
   `plans/milestone-3-schema-migrations.md` §7.3 says the env override
   needs reviewer sign-off and never mentions `apply_migrations` at all.
   Justification: PLAN.md's own Milestone 3 scope and acceptance criteria
   (lines ~316–341) mandate **both** by name — the milestone plan doc is
   simply older than PLAN.md here. PLAN.md is the more authoritative,
   more recently reconciled document, so it won. Note SPEC.md still does
   not mention `MLM_DB_PATH`; a one-line SPEC.md note is owed (out of this
   milestone's file scope).
2. **`DbError::Migrate.path` is `Option<PathBuf>`, not `PathBuf`.**
   Forced by `apply_migrations`, which has no path by construction.
   `connect_at` calls `apply_migrations` and re-stamps `Some(path)` onto
   the error, so file-backed failures still name the file; the path-free
   `Display` drops just the path clause. This is the only signature
   divergence from the plan's §3.2.
3. **`DbError::Query` carries `#[allow(dead_code, reason = ...)]`**, since
   it is declared for Milestone 4+ but constructed nowhere in this
   milestone and `-D warnings` rejects it otherwise. Milestone 4 should
   delete the attribute once it has real call sites.
4. `main.rs` was **not** touched: its `db::connect().expect(...)` still
   compiles unchanged against `Result<Connection, DbError>` (`DbError:
   Debug`). The plan's §4 step 7 anticipated needing an edit; none was
   needed. That `.expect` remains a §6.1 violation for Milestone 7 to fix.
5. TDD ordering was honest but batched: T1–T5 were written and watched fail
   (compile-error red: `connect_at`/`migrations` absent) before any
   implementation. T6–T16 were written after the DDL existed (T2 required
   it), so they passed on first run — the mutation check above is what
   substitutes for a red phase there.

## Notes for the reviewer / downstream milestones

- **`connect_at(&Path)` is the only path tests may use.** `connect()`
  resolves the real user data dir; no test in any milestone should call it.
- **`MLM_DB_PATH` semantics**: set and non-empty → used verbatim (no
  `mlm.db` appended, no parent-dir assumptions beyond `connect_at`'s
  `create_dir_all`); unset or empty → `data_dir()/mlm.db`. Integration
  tests in Milestones 7/8/10/11 should set it on the **child process**
  (`Command::env`), which is safe; the in-process test here exercises the
  private pure `resolve_db_path(Option<OsString>)` instead of mutating the
  test process's environment (`std::env::set_var` is `unsafe` in edition
  2024 and unsound with a multithreaded test harness).
- **Bookkeeping is `PRAGMA user_version`.** There is no and will never be a
  `schema_migrations` table; a test asserts its absence on purpose. Do not
  "fix" this. Related caveat: a `.dump`/`.read`-reconstructed database loses
  `user_version` and would fail re-migration — don't document `.dump` as a
  backup path.
- **`AUTOINCREMENT` materializes `sqlite_sequence`** — no test may ever
  assert an exact table count in `sqlite_master`.
- **DECISION-C (`notes.created_at_utc` seconds) is now moot for Milestone 3
  and appears already resolved against the plan**: the plan's §7.8 proposes
  true seconds, but current SPEC.md §2.3 explicitly says the column is
  "minute-granular like every other stored instant" with "`id ASC` as the
  actual tiebreaker". Schema-wise it is `TEXT NOT NULL` either way, so
  nothing in this milestone depends on it. **Milestone 4 must follow
  SPEC.md (minute-granular, order by `created_at_utc, id`)**, not the plan
  doc's §7.8 — flagging the contradiction rather than silently resolving it.
- `Cargo.toml` `[dev-dependencies]`: `tempfile = "3.27.0"`,
  `chrono-tz = "0.10.4"` (the latter is unused by this milestone, added per
  PLAN.md's consolidated dev-dep list so later milestones don't collide on
  `Cargo.lock`). Clippy/build do not complain about the unused dev-dep.
- Concurrency (two `mlm` processes racing migration 0001) still surfaces as
  `DbError::Migrate` on `SQLITE_BUSY`, per SPEC.md §2.2. Deliberate; do not
  add a busy timeout to quiet a flaky test.
