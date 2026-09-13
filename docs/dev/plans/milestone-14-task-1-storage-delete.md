# Milestone 14 — Task 1 low-level plan: `storage.rs` delete-by-id

**Scope:** `src/storage.rs` only. Implements `delete_punch`/`delete_note`
plus the new `StorageError::NotFound` variant and its tests. No other
file changes. This is a plan, not a diff — no code has been written or
edited against `src/storage.rs` as part of producing it.

Source documents read in full: `docs/dev/plans/milestone-14-delete-punches-notes.md`
(Task 1 section), `docs/dev/specs/2026-09-13-delete-punches-notes.md`
§5 (+ skim of the rest), and the current `src/storage.rs`.

---

## 1. Facts gathered (so the steps below are unambiguous)

- **`Punch` fields** (line 94-111): `id: i64`, `at_utc: DateTime<Utc>`,
  `date: NaiveDate`, `kind: PunchKind`.
- **`Note` fields** (line 114-130): `id: i64`, `date: NaiveDate`,
  `body: String`, `created_at_utc: DateTime<Utc>`.
- **`PunchKind`** (line 54-57): `Start`, `End` — unrelated to this
  task except that `punch_from_row`/`map_punch_row` already handle it.
- **Existing row-mapping helpers to reuse, not duplicate:**
  - `map_punch_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<(i64, String, String, String)>`
    (line 327-329) — column order `(id, at_utc, date, kind)`.
  - `punch_from_row(id: i64, at_utc: String, date: String, kind: String) -> Result<Punch, StorageError>`
    (line 331-343) — turns that tuple into a typed `Punch`.
  - **Notes have no equivalent named helpers today** —
    `notes_for_date` (line 394-418) inlines both the row-mapping
    closure and the tuple → `Note` construction. This task extracts
    two new private helpers, `map_note_row` and `note_from_row`,
    mirroring the punch pair exactly, and changes `notes_for_date` to
    use them instead of its inline code — this is required so
    `delete_note` can reuse the same mapping logic per the milestone
    plan's explicit instruction, and is a pure refactor (no behavior
    change, covered by `notes_for_date`'s own existing passing tests).
- **`StorageError`** (line 135-152), current variants: `Db(rusqlite::Error)`,
  `CorruptRow { column: &'static str, value: String }`, `EmptyNote`,
  `NonexistentLocalTime { local: NaiveDateTime }`.
  - `Display` impl: line 154-170, one match arm per variant.
  - `source()` impl: line 172-181, one match arm per variant (a
    3-variant `|`-combined arm returning `None`, plus `Db`'s `Some`).
  - `grep -rn "StorageError" src/` confirms **no other file** matches
    over this enum exhaustively — `commands.rs` only uses
    `err.downcast_ref::<StorageError>()` against a specific variant
    (`EmptyNote`), which needs no update for a new variant. So exactly
    two match blocks need a new arm, both inside `src/storage.rs`.
- **Test fixtures already in `mod tests` to reuse, not reinvent**
  (line 439-484): `test_db() -> Connection`, `d(y, m, day) -> NaiveDate`,
  `t(h, m) -> NaiveTime`, `utc(y, mo, day, h, mi, s) -> DateTime<Utc>`,
  plus raw-column probes `raw_kind`/`raw_at_utc`/`raw_date` (only
  useful for punches; no note equivalent needed for this task's tests).
- **RETURNING support** — `Cargo.toml`: `rusqlite = { version = "0.40.2",
  features = ["bundled"] }`. The `bundled` feature compiles SQLite from
  the vendored `libsqlite3-sys` amalgamation; checked directly
  (`libsqlite3-sys-0.38.2/sqlite3/sqlite3.h`): `SQLITE_VERSION
  "3.53.2"`. `DELETE ... RETURNING` has been supported since SQLite
  3.35.0 (2021-03-12), so 3.53.2 supports it unconditionally, and
  since the connection always uses the bundled library there is no
  version-skew risk from a system SQLite. `rusqlite` itself needs no
  special support for `RETURNING` — it's plain SQL text executed
  through `query_row`, identical in shape to every other query in this
  file.

## 2. Design decision: `DELETE ... RETURNING`, not `SELECT` + `DELETE`

Chosen over a two-statement `SELECT` then `DELETE`:

- **One round trip, no transaction needed.** A `SELECT`-then-`DELETE`
  pair would need a transaction to be safe against the row
  disappearing between the two statements; `RETURNING` makes the
  delete-and-fetch atomic in a single statement, so no `conn.transaction()`
  wrapping is needed in either new function.
- **Confirmed supported** by the bundled SQLite version (§1) — no
  fallback needed.
- **Symmetric with existing style.** Every other read in this file
  (`punches_for_date`, `notes_for_date`, `punches_in_range`) is a
  single `stmt.prepare` + `query_map`/`query_row` — a single
  `query_row` call over a `DELETE ... RETURNING` statement fits that
  same shape, whereas `SELECT` + separate `DELETE ... WHERE id = ?`
  would be more code for no behavioral gain (the spec itself says
  "implementation's choice, no behavior difference").
- **Row-not-found detection is free**: `Connection::query_row` returns
  `Err(rusqlite::Error::QueryReturnedNoRows)` when the statement
  produces zero rows — exactly the zero-affected-rows case both new
  functions must turn into `StorageError::NotFound`, with no separate
  `execute` + `changes()` check required.

## 3. New error variant

Add to the `StorageError` enum (`src/storage.rs`, immediately after the
`NonexistentLocalTime` variant, i.e. after line 152, before the closing
`}` of the enum at line 152/153):

```rust
    /// §5 of the delete-punches-notes spec: a delete was attempted
    /// against an internal id that doesn't exist (already deleted, or
    /// external DB tampering — SPEC.md §2.2 puts concurrent access out
    /// of scope, so this is defensive, not a concurrency feature).
    /// Nothing was written.
    NotFound { id: i64 },
```

**Two exhaustive-match sites to update, both in this file:**

1. `impl fmt::Display for StorageError` (line 154-170) — add, after the
   `NonexistentLocalTime` arm (after line 167, before the closing `}`
   of the match):

   ```rust
   StorageError::NotFound { id } => write!(f, "no row found with id {id}"),
   ```

2. `impl std::error::Error for StorageError` / `source()` (line 172-181)
   — extend the existing `None`-returning combined arm to also list
   `NotFound`:

   ```rust
   StorageError::CorruptRow { .. }
   | StorageError::EmptyNote
   | StorageError::NonexistentLocalTime { .. }
   | StorageError::NotFound { .. } => None,
   ```

No other exhaustive match over `StorageError` exists anywhere in the
crate (confirmed by `grep -rn "StorageError" src/`) — `commands.rs`'s
uses are all `downcast_ref` equality checks against `EmptyNote`, not
matches that need to be exhaustive.

## 4. New helpers (note row-mapping extraction)

Placed immediately before `notes_for_date`, i.e. inserted after
`punches_in_range` ends (after line 389) and before the
`notes_for_date` doc comment (line 391) — directly mirroring where
`map_punch_row`/`punch_from_row` sit relative to `punches_for_date`:

```rust
fn map_note_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<(i64, String, String, String)> {
    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
}

fn note_from_row(id: i64, date: String, body: String, created_at_utc: String) -> Result<Note, StorageError> {
    Ok(Note {
        id,
        date: parse_date(&date, "date")?,
        body,
        created_at_utc: parse_utc(&created_at_utc, "created_at_utc")?,
    })
}
```

Column order fixed as `(id, date, body, created_at_utc)` — matches
`notes_for_date`'s existing `SELECT id, "date", body, created_at_utc`
column list, so the same tuple shape works for both the plain read and
the new `RETURNING` delete.

**Refactor `notes_for_date` (line 394-418)** to call these instead of
its inline closure/construction:

```rust
pub fn notes_for_date(conn: &Connection, date: NaiveDate) -> Result<Vec<Note>, StorageError> {
    let mut stmt = conn.prepare(
        "SELECT id, \"date\", body, created_at_utc FROM notes \
         WHERE \"date\" = ?1 ORDER BY created_at_utc ASC, id ASC",
    )?;
    let rows = stmt.query_map((date.format("%Y-%m-%d").to_string(),), map_note_row)?;
    let mut out = Vec::new();
    for row in rows {
        let (id, date, body, created_at_utc) = row?;
        out.push(note_from_row(id, date, body, created_at_utc)?);
    }
    Ok(out)
}
```

This is a behavior-preserving refactor — `notes_for_date`'s existing
tests (N8-N12, D7, the two `notes_and_punches_are_independent` /
`earliest_data_date_finds_minimum_across_punches_and_notes` etc. tests
that call it) must still pass unchanged and serve as its regression
coverage; no new test is needed solely for this extraction.

## 5. New functions

### 5.1 `delete_punch`

**Placement:** immediately after `punches_in_range` (after line 389),
directly before the `map_note_row`/`note_from_row` helpers added in
§4 (so all punch-related functions stay grouped before the note
section starts, matching the file's existing punch-section /
note-section grouping at line 391's `--- notes` conceptual boundary).

```rust
/// Delete the punch with the given internal `id` and return the row
/// as it was immediately before deletion (for the delete command's
/// recreate-echo). `id` is the internal DB surrogate key
/// (`Punch::id`), never the ephemeral per-listing position computed
/// by the `delete` command — that resolution happens in the caller.
/// `StorageError::NotFound` if no row with that id exists; nothing is
/// written on that path. Deleting one row never touches any other row
/// (enforced by the `WHERE id = ?1` filter, verified by test).
pub fn delete_punch(conn: &Connection, id: i64) -> Result<Punch, StorageError> {
    let result = conn.query_row(
        "DELETE FROM punches WHERE id = ?1 RETURNING id, at_utc, \"date\", kind",
        (id,),
        map_punch_row,
    );
    match result {
        Ok((id, at_utc, date, kind)) => punch_from_row(id, at_utc, date, kind),
        Err(rusqlite::Error::QueryReturnedNoRows) => Err(StorageError::NotFound { id }),
        Err(source) => Err(StorageError::Db(source)),
    }
}
```

### 5.2 `delete_note`

**Placement:** immediately after `notes_for_date` (after line 418),
before the `earliest_data_date` doc comment (line 420) — mirroring
`delete_punch` sitting right after its own table's read functions.

```rust
/// Delete the note with the given internal `id` and return the row as
/// it was immediately before deletion (for the delete command's
/// recreate-echo). `id` is the internal DB surrogate key (`Note::id`),
/// never the ephemeral per-listing position — that resolution happens
/// in the caller. `StorageError::NotFound` if no row with that id
/// exists; nothing is written on that path. Deleting one row never
/// touches any other row (enforced by the `WHERE id = ?1` filter,
/// verified by test).
pub fn delete_note(conn: &Connection, id: i64) -> Result<Note, StorageError> {
    let result = conn.query_row(
        "DELETE FROM notes WHERE id = ?1 RETURNING id, \"date\", body, created_at_utc",
        (id,),
        map_note_row,
    );
    match result {
        Ok((id, date, body, created_at_utc)) => note_from_row(id, date, body, created_at_utc),
        Err(rusqlite::Error::QueryReturnedNoRows) => Err(StorageError::NotFound { id }),
        Err(source) => Err(StorageError::Db(source)),
    }
}
```

Note the `RETURNING` column order is written out explicitly as
`id, "date", body, created_at_utc` to match `map_note_row`'s tuple
order exactly (same as the `SELECT` column list `notes_for_date`
already uses) — `"date"` needs the same quoting as every other
reference to that column in this file, since it's a SQL keyword.

## 6. Step-by-step sequencing (TDD — tests before implementation)

1. Add the `NotFound { id: i64 }` variant to `StorageError` (§3) and
   its two match arms. The crate will not compile yet until step 2's
   helpers and step 3/4's functions exist and satisfy every call site
   — do this step and step 2 together in one compile-checkpoint if
   preferred, since neither alone compiles cleanly against tests that
   reference the new variant.
2. Add `map_note_row`/`note_from_row` (§4) and refactor
   `notes_for_date` to use them. Run the full existing test suite
   (`cargo test --lib storage::`) to confirm this refactor alone is
   behavior-preserving before writing any new test.
3. Write the new tests listed in §7 below (they will fail to compile
   until step 4, since `delete_punch`/`delete_note` don't exist yet —
   that's expected/required for TDD: red before green).
4. Implement `delete_punch` (§5.1) and `delete_note` (§5.2).
5. Run `cargo test --lib storage::` — all new tests plus the full
   existing suite green.
6. Run `cargo clippy --all-targets --all-features -- -D warnings` (or
   this project's pinned clippy invocation if `clippy.toml` specifies
   stricter lints — check `clippy.toml` at the repo root) — clean.
7. Hand off: Task 3 (`commands.rs`) consumes `delete_punch`/
   `delete_note` and the `NotFound` variant directly.

## 7. Tests to write first (TDD), named per this file's existing convention

All new tests live in the existing `#[cfg(test)] mod tests` block
(after the current last test, `punches_in_range_empty_span_returns_empty_vec`,
line 1164-1170), grouped under a new `--- 7.8 delete-by-id` comment
header mirroring the file's existing `--- 7.N <name> ---` section
convention. Each uses `test_db()`/`d()`/`t()`/`utc()` from §1, never
hand-rolled equivalents.

- `fn delete_punch_removes_row_and_returns_it()` — insert one punch via
  `insert_punch`, capture its `id`, call `delete_punch`, assert the
  returned `Punch`'s fields (`id`, `at_utc`, `date`, `kind`) match what
  was inserted, then assert `punches_for_date` for that date is empty.
- `fn delete_note_removes_row_and_returns_it()` — same shape for
  `insert_note`/`delete_note`: assert returned `Note`'s `id`, `date`,
  `body`, `created_at_utc` match, then `notes_for_date` for that date
  is empty.
- `fn delete_punch_nonexistent_id_returns_not_found()` — on a fresh
  `test_db()` with no rows, call `delete_punch(&conn, 999)`, assert
  `matches!(err, Err(StorageError::NotFound { id: 999 }))`.
- `fn delete_note_nonexistent_id_returns_not_found()` — same for
  `delete_note(&conn, 999)`.
- `fn delete_punch_leaves_other_punches_on_same_date_untouched()` —
  insert two punches on the same `d(2026, 1, 15)` (distinct `at_utc`,
  e.g. `t(9,0)` start and `t(17,0)` end), delete only the first one's
  `id`, assert `punches_for_date` for that date now returns exactly
  the second one (by `id`).
- `fn delete_punch_leaves_punches_on_other_dates_untouched()` — insert
  one punch on `d(2026, 1, 15)` and one on `d(2026, 1, 16)`, delete the
  first, assert `punches_for_date` for `2026-01-16` is unaffected
  (len 1, same `id`) and for `2026-01-15` is now empty.
- `fn delete_note_leaves_other_notes_on_same_date_untouched()` — mirror
  of the punch same-date test, two notes on one date, delete one,
  assert the other remains via `notes_for_date`.
- `fn delete_note_leaves_notes_on_other_dates_untouched()` — mirror of
  the punch cross-date test for notes.
- `fn not_found_error_names_the_id_and_has_no_source()` — single test
  covering both punch and note paths (since the error type/behavior is
  shared): trigger `StorageError::NotFound { id: 42 }` (e.g. directly
  via `delete_punch(&conn, 42)` on an empty db), assert
  `err.to_string()` contains `"42"`, and assert
  `std::error::Error::source(&err).is_none()`.

Existing tests (`P1`-`P13`, `N1`-`N12`, `D1`-`D7`, `X1`-`X3`, the
`earliest_data_date`/`punches_in_range` block) are not modified except
as an incidental consequence of the `notes_for_date` refactor in §4,
which must not change their observable behavior or require editing
their assertions.

## 8. Verification

- Targeted: `cargo test --lib storage::tests`
- Full crate (required before calling the task done, per the
  milestone plan's "full existing storage.rs test suite still
  passes"): `cargo test`
- Lint: `cargo clippy --all-targets --all-features -- -D warnings`
  (check `clippy.toml` at the repo root first in case the project
  pins additional lint args or a stricter cognitive-complexity
  threshold to fold into this exact invocation).
