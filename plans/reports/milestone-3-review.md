# Milestone 3 — independent adversarial review

**Reviewer**: fresh read of `src/db.rs` @ `433b456` (branch `milestone-3`),
against `plans/milestone-3-schema-migrations.md`, `PLAN.md` (Milestone 3 +
contracts 7/8/10), and `SPEC.md` §2.1/§2.2/§2.3/§6.1/§6.2. Nothing was
modified in the worktree except this file. Probing was done on a throwaway
`git archive` copy in a scratch directory.

## Verification actually run

- `cargo build` — **pass** (3 pre-existing `dead_code` *warnings* from
  `src/time.rs`).
- `cargo test` — **18 passed, 0 failed**, all in `db::tests`.
- `cargo fmt --check` — **clean**.
- `cargo clippy --all-targets -- -D warnings` — fails with exactly **3
  errors**, all `dead_code` on `src/time.rs`'s `parse_hm` / `between` /
  `format_duration`. The report's "pre-existing" claim is **confirmed
  independently**: `src/time.rs`, `src/cli.rs`, `src/main.rs` are byte-identical
  between the base commit `638f2d6` and `433b456` (`git diff --stat` empty),
  and a clean extraction of `638f2d6` into a scratch dir produces the same
  three errors. `cargo clippy --all-targets -- -D warnings -A dead_code` is
  **clean**, i.e. this milestone's code contributes zero clippy findings.
- Mutation check re-run independently: deleting the `kind` CHECK, the
  `target_minutes` CHECK, and the punches index made exactly the three
  corresponding tests fail (`punch_kind_check_rejects_...`,
  `week_target_minutes_rejects_...`, `baseline_schema_creates_the_documented_indexes`).
  The tests have teeth.

## Acceptance criteria — all met

Each PLAN.md Milestone 3 AC is discharged by a test that genuinely exercises it:

- Fresh-DB bootstrap creates dir + file + all three tables, no error — T1/T2,
  verified (`temp_db()` deliberately points at a *not yet created* `nested/`
  subdirectory, so the `create_dir_all` path is really taken).
- Idempotent re-migration with no data loss — T4 (+ T3 for `user_version == 1`
  / `pending_migrations == 0`).
- `kind` CHECK rejects `START`/`Start`/`pause`/``/`starts`/`END` and accepts
  `start`/`end` — T6/T7. Case-sensitivity is real (BINARY collation).
- `target_minutes >= 0`: `-1`/`-2400` rejected, `0`/`2400` accepted and read
  back — T8.
- Unopenable paths surface as `Err`, never panic — T11 (file where a directory
  must be → `CreateDataDir`), T12 (`chmod 000`, unix, root-guarded), T13
  (4 KiB of garbage → `Open`/`Migrate`).
- `MLM_DB_PATH` honored — T18 (but see finding 4: the env *plumbing* itself is
  untested).
- `apply_migrations` against `Connection::open_in_memory()` with no path — T17,
  including an idempotent second application.
- No `unwrap`/`expect`/`panic!` anywhere in the non-test half of the module
  (lines 1–212) — verified by grep, not just taken from the report.
- `rusqlite_migration` API used correctly: `M::up(...).comment(...)` in a
  `const` slice, `Migrations::from_slice`, `to_latest(&mut conn)`,
  `validate()`, `pending_migrations`, `usize::from(current_version(..))`.
  Bookkeeping is `PRAGMA user_version`; T3 asserts no `schema_migrations`
  table exists, which matches SPEC.md §2.2's "or equivalent".
- No SQL injection surface: the only SQL in non-test code is the static
  baseline DDL const. Every test insert is parameterized (`?1`/`?2`/`?3`);
  the two `format!`-built statements (`PRAGMA table_info({table})`,
  `SELECT COUNT(*) FROM {table}`) interpolate only `&'static str` literals
  from a test-local array. Not a risk.
- CHECK clauses are *exactly* SPEC.md §2.3's two and no more — no format
  globs, no non-empty check on `notes.body`, no FKs, no WAL/busy_timeout,
  no `DEFAULT` on `target_minutes`, no `down` migrations.
- Contract 8/10 respected: no `Punch`/`PunchKind`/`Note`/UTC-format helpers
  are defined here. `src/db.rs` is schema + migrations + error type only.
- `Cargo.toml`: `tempfile = "3.27.0"` and `chrono-tz = "0.10.4"` are in
  `[dev-dependencies]` only; no new runtime dependency was added
  (`anyhow` was already present on the base commit). `chrono-tz` is unused
  by this milestone but is explicitly sanctioned by PLAN.md's "Consolidated
  dev-dependencies" paragraph, which assigns both to Milestone 3.

## The two self-flagged items — both check out

- **`MLM_DB_PATH` + `apply_migrations` implemented despite the milestone
  plan's §7.3 sign-off gate**: the implementer's reading of PLAN.md is
  **correct**. PLAN.md's Milestone 3 *scope* names both explicitly
  ("An `MLM_DB_PATH` environment variable override on the real-path
  resolution…", "A separate `apply_migrations(conn: &mut Connection) ->
  Result<(), DbError>`…") and its *acceptance criteria* list both as
  pass/fail conditions ("`MLM_DB_PATH`, when set, is honored…",
  "`apply_migrations` succeeds against a fresh `Connection::open_in_memory()`").
  PLAN.md's own open-risks section also records the matter as **resolved** in
  favour of both. The milestone plan doc's §7.3 is simply the older text.
  Building them was right; not building them would have failed PLAN.md.
- **`DbError::Migrate.path: Option<PathBuf>`**: reasonable, not leaky. An
  in-memory connection genuinely has no path, so `Option` is the honest
  encoding rather than a workaround; the alternative (a sentinel path, or a
  second `MigrateInMemory` variant) would be worse. `connect_at` re-stamps
  `Some(path)` so file-backed failures still name the file, and the `None`
  arm has its own `Display` branch, both covered by T16. Fine as-is (though
  see finding 3 on *how* the re-stamp is written).

---

## Findings

1. **`src/db.rs:113-137` — `Display` folds the source into its own message, so
   `anyhow`'s `{:#}` prints it twice.** Verified empirically:
   `anyhow::Error::new(DbError::Open { .. })` renders as
   `could not open database /x/y.db: Query is not read-only` under `{}` and
   `could not open database /x/y.db: Query is not read-only: Query is not read-only`
   under `{:#}`. The milestone plan deliberately chose a self-contained
   one-liner, and that is defensible — but PLAN.md contract 7 says `main`
   prints "the final `anyhow::Error`'s `Display` (or `{:#}` for the full
   context chain, if that reads better)". Those two decisions are incompatible
   for this type. Nothing to change in Milestone 3; Milestone 7 must use `{}`,
   not `{:#}`, or accept stuttering messages. Worth recording here so it isn't
   rediscovered as "a bug in `DbError`". **Severity: minor.**

2. **`src/db.rs:64-67` — `week_targets.week_id TEXT PRIMARY KEY` permits NULL,
   and permits *many* NULLs.** SQLite's well-known rowid-table quirk: a
   non-`INTEGER` PRIMARY KEY column is not implicitly `NOT NULL`. Confirmed
   against the real built schema: two successive
   `INSERT INTO week_targets (week_id, target_minutes) VALUES (NULL, …)`
   both succeed, leaving 2 rows — which would also defeat Milestone 8's
   `ON CONFLICT(week_id)` upsert for those rows. This exactly matches SPEC.md
   §2.3's table, so it is spec-compliant, and app code will always supply a
   `WeekId`, so it is unreachable in practice. Still a schema-layer hole in a
   milestone whose whole job is "the schema enforces its own constraints";
   `week_id TEXT PRIMARY KEY NOT NULL` costs nothing and rejects nothing the
   spec considers valid. Related and lesser: `target_minutes INTEGER NOT NULL
   CHECK (target_minutes >= 0)` admits non-numeric TEXT — `('2026-07', 'abc')`
   is accepted and stored as TEXT, because INTEGER affinity cannot coerce it
   and SQLite compares TEXT as greater than any number. Numeric strings
   (`'-5'`) *are* coerced and correctly rejected, and typed Rust params make
   this unreachable too. **Severity: minor** (latent, spec-literal,
   unreachable from app code) — recommend the `NOT NULL` as a follow-up, and
   a T10-style test for `week_id` NULL, which the current T10 does not cover.

3. **`src/db.rs:197-203` — the Migrate path re-stamp is a `match` with an
   unreachable arm.** `apply_migrations` can only ever return
   `DbError::Migrate`, so `other => other` is dead by construction and will
   silently become a real branch if a future `apply_migrations` grows another
   variant. A private `fn migrate(conn: &mut Connection, path: Option<&Path>)`
   with `apply_migrations` and `connect_at` as its two thin callers expresses
   the same thing without the re-wrap. Cosmetic, no behavioural defect.
   **Severity: minor.**

4. **`src/db.rs:168-170` and `src/db.rs:763-775` — the `MLM_DB_PATH` *env
   plumbing* is untested; only the private pure helper is.** T18 exercises
   `resolve_db_path(Option<OsString>)`, which is the correct call given
   `std::env::set_var`'s edition-2024 unsafety — but nothing asserts that
   `default_db_path()` reads `DB_PATH_ENV`, and nothing asserts
   `DB_PATH_ENV == "MLM_DB_PATH"`. A typo in the const's value, or a
   `var_os(SOME_OTHER_CONST)`, passes the entire suite while silently breaking
   the AC and every downstream hermetic integration test. Cheap fixes: assert
   the const's literal value, and/or let Milestones 7+ cover the real read via
   `Command::env` on the compiled binary (which the report already plans).
   **Severity: minor.**

5. **`src/db.rs:773` — T18 asserts against `data_dir().expect("data dir")`,
   making an otherwise-pure test dependent on `ProjectDirs` resolving.** In a
   container/CI with no `HOME` (or no `%APPDATA%`), `data_dir()` returns
   `DataDirUnavailable` and the test panics on a condition it is not trying to
   test. Asserting only `empty == unset` and `unset.ends_with("mlm.db")` would
   keep the intent without the environmental coupling. **Severity: minor.**

6. **`src/main.rs:10` — `db::connect().expect("failed to open database")`
   remains.** `src/db.rs` itself never panics (verified), so the milestone's AC
   is met at the module boundary; but end-to-end the binary still panics rather
   than printing one stderr line and exiting nonzero, which is precisely
   SPEC.md §6.1/§6.3 and E6. The milestone plan §7.11 assigns this to Milestone
   7 and the report correctly declines to touch it — noted only so it is not
   mistaken for "AC 5 is fully satisfied at the product level". It is not, yet,
   and nothing in the repo currently tracks that besides prose.
   **Severity: minor (correctly out of scope).**

7. **`coverage-baseline.json` still reads `line_coverage_percent: 0`.**
   AGENTS.md's pre-commit gate ratchets this up when coverage improves; a
   milestone that added 18 real tests should have moved it. Most likely the
   hook was not installed in this worktree (`git config core.hooksPath
   .githooks` is opt-in per clone) or `cargo-llvm-cov` is absent, in which case
   the gate `SKIP`s by design. Not a code defect, but the baseline is now
   stale/toothless for the next milestone. **Severity: minor (process).**

8. **SPEC.md still does not document `MLM_DB_PATH`.** The report owns this as
   an outstanding one-line spec note that is out of this milestone's file
   scope; confirmed — the string appears only in `NOTES.md:281` and `PLAN.md`.
   A user-visible environment variable that is not in the source of truth will
   drift. Should land with Milestone 7 or 12 at the latest.
   **Severity: minor.**

9. **`src/db.rs:581-616` — T12's root guard is a silent `return`, and it uses a
   hand-rolled `extern "C" { fn geteuid() }`.** A vacuous pass under root looks
   identical to a real pass in CI output; the plan sanctioned the skip, but an
   `eprintln!("skipping: running as root")` would make it visible. The
   hand-rolled `geteuid` avoids a `libc` dev-dependency and is correct.
   **Severity: minor.**

## Things checked and found genuinely fine (no finding raised)

- Migration set structure: `const MIGRATION_SLICE: &[M<'static>]` with a frozen
  DDL const and an explicit append-only INVARIANT comment. Correct shape for
  future additions; `migrations()` allocates nothing.
- A database from a *future* schema version degrades safely: setting
  `user_version = 7` and calling `apply_migrations` returns
  `Err(Migrate { source: MigrationDefinition(DatabaseTooFarAhead) })`,
  displays as one actionable line, and leaves `user_version` untouched at 7 —
  no silent re-application, no data destruction.
- `DROP TABLE IF EXISTS entries` (DECISION-A): correctly scoped to pre-release
  dev machines, no-op on fresh DBs, runs inside the crate's own migration
  transaction, and documented in-file as never to be imitated post-release.
  Endorsed as the plan recommended.
- `DbError` is a well-behaved `std::error::Error`: one-line ASCII `Display`
  with no trailing newline and no `Error:` prefix, the offending path named
  where one exists, `source()` returning the wrapped error for all four
  wrapping variants and `None` for `DataDirUnavailable`; no blanket
  `From<rusqlite::Error>`, so an `Open` failure can never be misfiled as a
  `Query`. T16 asserts all of it.
- First-run vs. genuine failure: no `path.exists()` predicate anywhere, no
  TOCTOU branch, no error swallowed as "probably first run" — absence is
  handled by construction exactly as the plan's §3.3 specifies.
- `connect_at` handles a parent-less relative path (`parent()` is `Some("")`)
  via the `!is_empty()` guard rather than crashing or creating `""`.
- `#[allow(dead_code, reason = …)]` on `DbError::Query` is the right minimal
  concession to `-D warnings` for a variant reserved by contract 7; the report
  correctly asks Milestone 4 to remove it.
- Index `("date", at_utc, id)` (DECISION-B) is verified by `PRAGMA index_info`
  in column order, not merely by name.
- T15 pins `sqlite_sequence`'s existence, so no later test can assert an exact
  `sqlite_master` table count.

---

**Verdict: APPROVE WITH NITS** — every PLAN.md acceptance criterion is met by
tests I re-ran and mutation-checked myself, `src/db.rs` contributes zero clippy
findings and no panic path, both self-flagged deviations are correct readings of
PLAN.md, and all nine findings are minor follow-ups (the largest being the
NULL-permitting `week_id` PRIMARY KEY and the `{:#}` message stutter) with none
blocking merge.
