# Milestone 4 — Punch and note storage (implementation plan)

**Status**: design only, written against PLAN.md's pinned interface contracts.
Milestones 1 (TIME parsing) and 3 (schema) are being designed in parallel, so
everything this plan needs from them is stated as an **assumption** in §0 and
re-flagged in §8 for cross-checking once those plans land.

**Spec sections owned here**: §2.1 (UTC storage, per-instant local↔UTC
conversion), §2.3 (`punches`/`notes` column semantics, note trimming), §6.1
(empty/whitespace-only note rejection, checked pre-trim; DB failure as a hard
error). **Flows**: E5 (+ its negative case), F12 (storage half), F4/F5 support.

**Out of scope** (belongs to other milestones, do not build here): stint
pairing (M5), week accounting (M6), CLI wiring / exit codes / stderr messages
(M7), `week_targets` writes (M8), any rendering (M9–11).

---

## 0. Assumptions about upstream milestones

These are the things this milestone cannot confirm yet. Each is listed again in
§8 as a cross-check item.

### 0.1 From Milestone 1 (time parsing)

**A1.** `TIME` parsing produces a **`chrono::NaiveTime`** with `second == 0` and
`nanosecond == 0` (§4.1: "no seconds precision anywhere — everything is
minute-granular"). If M1 instead returns a bare `(u8, u8)` hour/minute pair or a
custom `struct HourMinute`, this milestone's public signatures change only in
that one parameter type; everything downstream of the local `NaiveDateTime` is
unaffected. **Milestone 4 does no TIME string parsing of its own** — it accepts
an already-validated wall-clock value. Out-of-range hours/minutes (E1) are
rejected by M1 before this layer is ever called; `insert_punch` therefore has no
range-validation responsibility and must not re-implement one.

**A2.** M1 owns the hard-error type or, per PLAN contract 7, a shared error enum
exists. This plan defines its own `StorageError` (§5) and states how it should
merge with M1/M2's if a single crate-wide error type is chosen instead.

### 0.2 From Milestone 3 (schema)

Exactly SPEC.md §2.3, applied by `rusqlite_migration` on `db::connect()`:

```sql
CREATE TABLE punches (
  id     INTEGER PRIMARY KEY AUTOINCREMENT,
  at_utc TEXT NOT NULL,
  date   TEXT NOT NULL,
  kind   TEXT NOT NULL CHECK (kind IN ('start','end'))
);
CREATE INDEX idx_punches_date    ON punches(date);
CREATE INDEX idx_punches_date_at ON punches(date, at_utc, id);

CREATE TABLE notes (
  id             INTEGER PRIMARY KEY AUTOINCREMENT,
  date           TEXT NOT NULL,
  body           TEXT NOT NULL,
  created_at_utc TEXT NOT NULL
);
CREATE INDEX idx_notes_date ON notes(date, created_at_utc, id);
```

**A3.** `kind` is stored as the lowercase literals `'start'` / `'end'` (the
CHECK constraint in §2.3 fixes this exactly).

**A4.** `date` is stored as `YYYY-MM-DD` text, `at_utc` / `created_at_utc` as
RFC 3339 text.

**A5.** Milestone 3 exposes something equivalent to
`db::connect() -> Result<rusqlite::Connection, _>` plus a test-friendly way to
open an **in-memory or temp-file** database with migrations applied (e.g.
`db::connect_at(path: &Path)` or `db::open_migrated(conn)`). Milestone 4's tests
**require** this — they must never touch the real
`directories::ProjectDirs` app-data path. If M3 does not provide it, M4's first
task is to add it (a thin seam, not a schema change).

**A6.** Milestone 4 does **not** define, alter, or migrate the schema. If a
column is missing, that is an M3 defect to report, not something to patch here.

### 0.3 Timezone source (resolves PLAN.md's open risk)

PLAN.md flags "local timezone source in tests" as unresolved and blocking F12.
**Resolution proposed here**: every function in this milestone that performs a
local↔UTC conversion is **generic over `chrono::TimeZone`** and takes the
timezone as an explicit parameter. There is no hidden `Local::now()` or
`Local` reference anywhere inside `storage.rs`.

- Production callers (Milestone 7) pass `&chrono::Local`.
- Tests pass a `chrono_tz::Tz` with a known, hardcoded transition date.

This requires adding **`chrono-tz`** as a `[dev-dependencies]` entry in
`Cargo.toml` (it is not currently a dependency). `chrono_tz::Tz` implements
`chrono::TimeZone`, so it drops straight into the generic parameter with no
production-code cost and no extra runtime dependency in the shipped binary.

Rejected alternative: setting the `TZ` environment variable in tests. `chrono`'s
`Local` caches/derives the offset through libc in a way that is not reliably
re-read per call, `std::env::set_var` is racy under `cargo test`'s default
thread-parallel harness, and it does not work on Windows (a stated target
platform in AGENTS.md). Do not use it.

---

## 1. Module placement

New file **`src/storage.rs`** (registered as `mod storage;` in `main.rs`),
holding the punch/note value types, insert functions, and read functions.

Rationale: `db.rs` stays "connection + schema + migrations" (Milestone 3's
territory); `storage.rs` is "typed reads and writes over that schema". Keeping
them separate means M3 and M4 touch disjoint files and merge cleanly from
parallel worktrees. If the eventual preference is one file, the split is a
`pub use` away — but plan for two.

---

## 2. Public types

### 2.1 `PunchKind`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

    /// Inverse of `as_str`; unknown text is a data-integrity error.
    pub fn from_str(s: &str) -> Result<Self, StorageError> { /* "start" | "end" */ }
}
```

`from_str` returning `Err(StorageError::CorruptRow { .. })` rather than
panicking matters because the CHECK constraint could in principle be bypassed
by an externally-edited DB file.

### 2.2 `Punch` — **PLAN.md interface contract 1, the hand-off to Milestone 5**

This is the single most important type in this milestone, because Milestone 5's
fixtures are being written against it right now in parallel. It is stated here
completely and exhaustively; no field is implicit.

```rust
/// One stored start/end event, read back.
///
/// Contract (PLAN.md interface contract 1): an ordered instant (UTC + local
/// calendar date), a start/end kind, and an insertion-order tiebreaker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Punch {
    /// `punches.id`. Surrogate key (NOTES.md decision 9), monotonically
    /// increasing with insertion (AUTOINCREMENT). This is the **insertion-order
    /// tiebreaker** used when two punches share an identical `at_utc`
    /// (§4.3 step 1, §4.3's zero-length-stint edge case, E14).
    pub id: i64,

    /// The instant, in UTC. Minute-granular: seconds and nanoseconds are
    /// always zero (§4.1). Sorting by this field is the primary sort key
    /// for §4.3's pairing algorithm.
    pub at_utc: chrono::DateTime<chrono::Utc>,

    /// The **local** calendar date this punch belongs to (§2.1/§2.3).
    /// NOT derivable from `at_utc` by string slicing — it is its own stored
    /// column, computed app-side at insert. This is the field that pairing,
    /// day totals and the `status`/`week` date scoping key off.
    pub date: chrono::NaiveDate,

    /// Whether this is a `start` or an `end` event (§1.3).
    pub kind: PunchKind,
}
```

**Explicit decisions baked into this shape, so M5 does not have to guess:**

1. It is the **full row**, not a lighter intermediate value (PLAN contract 1
   explicitly asks this question to be settled). Four fields is cheap, and M5's
   anomaly rendering (§7.3: `orphaned end at 18:00`) needs the instant, while
   M11's per-date rollup needs the date, so nothing here is dead weight.
2. `at_utc` is a **typed `DateTime<Utc>`**, not the raw RFC 3339 `String`.
   Parsing happens once at the storage boundary; M5 never parses strings. M5
   derives displayable local times by calling `at_utc.with_timezone(tz)` with
   the same injected timezone (§3.4) — per-instant, per §2.1.
3. `date` is a typed **`NaiveDate`**, not a `String`. It is redundant with
   `at_utc` *given* the timezone, and deliberately so: it is what was actually
   persisted, and carrying it avoids every consumer re-deriving it (and
   re-deriving it *differently*).
4. There is **no duration, no pairing state, no "is open" flag** on `Punch`.
   Those are M5's derived outputs, not storage's.
5. `Punch` derives `PartialEq`/`Eq`/`Clone`/`Debug` so M5 can build fixtures by
   hand and assert on vectors of them directly.
6. `PunchKind` is an **enum, not a string**, so M5's match is exhaustive.

**Fixture-construction note for Milestone 5**: a hand-built fixture just
literal-constructs `Punch { id, at_utc, date, kind }`. All fields are `pub`, so
no constructor function is required. To keep M5's fixtures terse, Milestone 4
additionally provides a test-only helper (behind `#[cfg(test)]` it would be
invisible to M5, so make it a plain `pub fn` in `storage.rs`):

```rust
/// Build a `Punch` from a local wall-clock date/time without touching a DB.
/// Same conversion path `insert_punch` uses, so fixtures cannot drift from
/// what real storage produces.
pub fn punch_from_local<Tz: chrono::TimeZone>(
    id: i64,
    kind: PunchKind,
    local_date: chrono::NaiveDate,
    local_time: chrono::NaiveTime,
    tz: &Tz,
) -> Result<Punch, StorageError>;
```

### 2.3 `Note`

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    /// `notes.id`, AUTOINCREMENT. Secondary insertion-order tiebreaker.
    pub id: i64,

    /// The local calendar date this note is attached to (§2.3: a note is
    /// attached to a day, not an instant — there is no `at_utc` here).
    pub date: chrono::NaiveDate,

    /// The note text, already trimmed of leading/trailing whitespace (§2.3).
    /// Never empty (§6.1 rejects that at insert). Interior whitespace and
    /// any project-name prefix are preserved verbatim; no length cap, no
    /// charset restriction.
    pub body: String,

    /// `notes.created_at_utc` — the primary insertion-order sort key for
    /// same-day notes (§2.3). Not displayed anywhere in MVP output.
    pub created_at_utc: chrono::DateTime<chrono::Utc>,
}
```

---

## 3. Public function signatures

All take `&rusqlite::Connection`. Because `rusqlite::Transaction` derefs to
`Connection`, these compose inside a transaction unchanged — which Milestone 7
needs for F5/E5 (punch + note in one invocation, all-or-nothing).

### 3.1 Insert a punch

```rust
/// Insert a start/end punch for a local wall-clock date+time.
///
/// `local_date` is the calendar date the user is punching against — always
/// *today* in MVP (§1.2, no backdating); the caller resolves it.
/// `local_time` is Milestone 1's parsed `TIME` (or "now"'s wall-clock time).
/// `tz` is the timezone to interpret them in — `&chrono::Local` in production.
///
/// Returns the new row's `id`.
pub fn insert_punch<Tz: chrono::TimeZone>(
    conn: &rusqlite::Connection,
    kind: PunchKind,
    local_date: chrono::NaiveDate,
    local_time: chrono::NaiveTime,
    tz: &Tz,
) -> Result<i64, StorageError>;
```

Why `local_date` + `local_time` as two parameters rather than one
`NaiveDateTime`: it mirrors exactly what the caller has (M1 gives a time; "today"
comes from the injected now), and it keeps the today-only rule (§1.2) visibly
the caller's decision rather than something buried in storage. An internal
`local_date.and_time(local_time)` is the first line of the body.

### 3.2 Insert a note

```rust
/// Insert a work-log note for a local calendar date.
///
/// `body` is raw user text; it is validated (§6.1) and trimmed (§2.3) here.
/// `now_utc` is the injected current instant used for `created_at_utc`
/// (PLAN contract 6 — never a hidden `Utc::now()`).
///
/// Returns the new row's `id`.
pub fn insert_note(
    conn: &rusqlite::Connection,
    local_date: chrono::NaiveDate,
    body: &str,
    now_utc: chrono::DateTime<chrono::Utc>,
) -> Result<i64, StorageError>;
```

No timezone parameter: `notes` has no `at_utc`, and `local_date` arrives already
resolved. `created_at_utc` is an opaque internal ordering key, never rendered,
so it needs no local counterpart.

### 3.3 Reads

```rust
/// All punches for a local calendar date, **sorted by instant, ties broken by
/// insertion order** (`at_utc ASC, id ASC`) — feeds Milestone 5's pairing
/// directly, already in §4.3 step 1's required order.
pub fn punches_for_date(
    conn: &rusqlite::Connection,
    date: chrono::NaiveDate,
) -> Result<Vec<Punch>, StorageError>;

/// All notes for a local calendar date, in insertion order
/// (`created_at_utc ASC, id ASC`) — §3.5's "that date's notes, in insertion
/// order".
pub fn notes_for_date(
    conn: &rusqlite::Connection,
    date: chrono::NaiveDate,
) -> Result<Vec<Note>, StorageError>;
```

Both return an **empty `Vec`**, never an error, for a date with no rows
(§6.2/E11/F4).

### 3.4 Optional convenience helper (recommended, flagged in §8)

Milestone 7's acceptance criterion — "a rejected note does not leave an orphaned
punch behind" (E5) — is cleanest if it is *impossible* to get wrong:

```rust
/// Insert a punch and, if `note_body` is `Some`, a note for the same local
/// date, atomically. Either both rows land or neither does (§6.1: a hard
/// error writes nothing).
pub fn insert_punch_with_note<Tz: chrono::TimeZone>(
    conn: &mut rusqlite::Connection,
    kind: PunchKind,
    local_date: chrono::NaiveDate,
    local_time: chrono::NaiveTime,
    tz: &Tz,
    note_body: Option<&str>,
    now_utc: chrono::DateTime<chrono::Utc>,
) -> Result<(i64, Option<i64>), StorageError>;
```

Implementation: `conn.transaction()?`, **validate the note body first**
(fail before any INSERT), then insert punch, then note, then `commit()`. The
early validation makes the empty-note rejection cost nothing even if the
transaction layer were later removed. `&mut Connection` is required by
`rusqlite::Connection::transaction`.

Whether this lives in M4 or M7 is a boundary judgement — §8 flags it. Building
it here is recommended, because it is pure storage and it lets M4's own tests
prove atomicity rather than deferring E5's most important half to M7.

### 3.5 Look-ahead reads deliberately *not* built here

Milestone 6 needs "the earliest week with any punch/note data" and per-date
totals across a range (§2.4). Those queries are not in Milestone 4's stated
scope and are **not** built here. §8 flags them so the M6 worktree knows to
either add them itself or request them as an M4 follow-up, rather than
discovering the gap at integration time.

---

## 4. UTC conversion mechanics (§2.1, DST-safety)

This is the part a naive implementation gets wrong, so it is specified step by
step.

### 4.1 Write path: local wall-clock → (`at_utc`, `date`)

Given `local_date`, `local_time`, and `tz`:

1. **Combine**: `let naive_local: NaiveDateTime = local_date.and_time(local_time);`
   Force minute granularity defensively — if `local_time` carries nonzero
   seconds/nanos, truncate to the minute (`with_second(0).with_nanosecond(0)`),
   per §4.1's "no seconds precision anywhere". This also guarantees `at_utc`'s
   serialized form always ends in `:00Z`, which §4.3's lexical sortability
   argument depends on.

2. **Resolve to an instant, per-instant**:
   `let resolved: chrono::LocalResult<DateTime<Tz>> = tz.from_local_datetime(&naive_local);`

   This is the **named chrono API that satisfies §2.1**: `TimeZone::from_local_datetime`
   consults the tz database for the offset in effect *at that specific local
   datetime*. It is the antithesis of grabbing `Local::now().offset()` once and
   reusing it, which is precisely what §2.1 and NOTES.md decision 28 forbid and
   what F12 is designed to catch.

   Handle all three `LocalResult` arms explicitly — never `.unwrap()`:
   - `Single(dt)` → use `dt`. The overwhelmingly common case.
   - `Ambiguous(earliest, _latest)` → **use `earliest`**. This is the
     fall-back repeated hour (e.g. `Europe/Warsaw` 2026-10-25 02:30 happens
     twice). Choosing the earlier instant is the conventional default and the
     only choice that keeps a user's punches monotonic with their wall clock as
     the hour repeats. Document the choice in a code comment citing this plan.
   - `None` → the **spring-forward gap**: that local time does not exist (e.g.
     `Europe/Warsaw` 2026-03-29 02:30). Return
     `Err(StorageError::NonexistentLocalTime { local: naive_local })`, a hard
     error, nothing written. See §8 — the spec does not cover this case, so
     this is a proposed resolution, not a confirmed rule.

3. **To UTC**: `let at_utc: DateTime<Utc> = resolved_dt.with_timezone(&Utc);`

4. **Derive the local calendar `date` column**:
   `let date_col: NaiveDate = at_utc.with_timezone(tz).date_naive();`

   **Deliberately round-tripped through the timezone rather than just reusing
   `local_date`.** §2.3 specifies the column as "computed app-side at insert
   from `at_utc` + local timezone", and this matches that literally. It also
   guarantees the invariant every reader depends on: *the stored `date` is
   always what the stored `at_utc` displays as locally*. In every non-gap case
   this equals the `local_date` argument, and a test asserts exactly that (P7).
   A debug assertion `debug_assert_eq!(date_col, local_date)` is reasonable
   belt-and-braces.

5. **Serialize** with one canonical formatter (§4.3) and INSERT.

### 4.2 Read path: row → `Punch`

- `at_utc`: parse the stored text with
  `DateTime::parse_from_rfc3339(s)?.with_timezone(&Utc)` (or
  `s.parse::<DateTime<Utc>>()`), mapping a parse failure to
  `StorageError::CorruptRow`. Never `unwrap`.
- `date`: `NaiveDate::parse_from_str(s, "%Y-%m-%d")`, same error handling.
- `kind`: `PunchKind::from_str`.

The read path does **no** timezone conversion at all — it returns UTC plus the
stored local date, and any local rendering is the display layer's job, done
per-instant there (§2.1, cross-cutting concern shared with M10).

### 4.3 Canonical serialization format

One private helper, used by every write:

```rust
const UTC_FMT: &str = "%Y-%m-%dT%H:%M:%SZ";
fn fmt_utc(dt: chrono::DateTime<chrono::Utc>) -> String { dt.format(UTC_FMT).to_string() }
```

Producing `2026-09-12T13:05:00Z`, exactly §2.3's documented example.

**Why this is load-bearing, not cosmetic**: §2.3 says `at_utc` is "sortable
lexically" and §5 below relies on an SQL `ORDER BY at_utc`. That only holds if
every row is fixed-width and uses the same offset spelling. `to_rfc3339()` would
emit `+00:00` instead of `Z`, and chrono's default `Display` can emit fractional
seconds — either would silently break lexical ordering the moment two formats
coexist in the table. **Never call `to_rfc3339()` or `to_string()` on a
`DateTime` on the write path.** A dedicated test (P9) asserts the exact stored
byte string.

`date` serializes as `date.format("%Y-%m-%d")`. `created_at_utc` uses the same
`fmt_utc`, and is truncated to the minute for format uniformity — see §6 for why
that makes the `id` tiebreaker mandatory rather than decorative.

---

## 5. Note validation and trimming (§6.1 / §2.3)

Order of operations, exactly as §6.1 words it ("checked *before* the trim in
§2.3 — a note that's nothing but whitespace has nothing left to trim to"):

```rust
// 1. Reject: is there anything at all once whitespace is discounted?
if body.trim().is_empty() {
    return Err(StorageError::EmptyNote);
}
// 2. Only now produce the stored form.
let stored = body.trim();
```

The rejection decision is made on the **raw input**, and the trim is only
performed to produce the value that gets persisted — the check is never applied
to an already-trimmed-and-possibly-empty string as if that were the user's
input. Functionally the predicate is the same; the ordering matters for the
error the user sees (an "empty note" hard error, §6.1) rather than an empty row
silently landing in the table.

Precise semantics to pin down for the TDD agent:

- **Trim function**: Rust's `str::trim()`, i.e. Unicode `White_Space`. This
  covers spaces, tabs, `\n`, `\r`, and also U+00A0 NBSP and U+3000. A
  NBSP-only note is therefore **rejected** — asserted by test N5. This is a
  deliberate choice (a paste-artifact NBSP is exactly as blank a log line as a
  space), stated so no one "fixes" it later.
- **Leading *and* trailing** only. Interior whitespace is preserved verbatim:
  `"did   a   thing"` stores with its interior runs intact, never collapsed.
- Newlines *inside* the body are preserved (no length cap, no charset
  restriction, §2.3). Rendering a multi-line note is M10's problem, not a
  storage-layer validation concern.
- A project-name prefix stays in the text (§2.3) — no parsing, no splitting.
- **Nothing is written on rejection.** `insert_note` returns before any
  `execute`; `insert_punch_with_note` validates before opening any INSERT and
  runs inside a transaction, so a late failure still rolls back.
- No other validation exists. No length cap, no control-character filtering, no
  deduplication.

---

## 6. Guaranteeing read order

### 6.1 Punches — **SQL `ORDER BY`, not a Rust sort**

```sql
SELECT id, at_utc, date, kind
FROM punches
WHERE date = ?1
ORDER BY at_utc ASC, id ASC;
```

Decision: **the read itself sorts**, so Milestone 5 receives data already in
§4.3 step 1's required order and never has to re-sort. (PLAN.md's acceptance
criterion allows either, as long as the contract is pinned — it is pinned here,
to the sorting read.)

Rationale for SQL over a Rust sort:
- The `(date, at_utc, id)` index (§0.2) makes this an index-ordered scan with no
  sort step at all.
- Ordering lives next to the query, so no caller can forget it.
- It is a single source of truth: M5 sorting again would be redundant work whose
  tie-break rule could drift from storage's.

**Correctness precondition**: `ORDER BY at_utc` on a `TEXT` column is a
**lexical** comparison. It equals chronological order only because §4.3's
canonical format is fixed-width, zero-padded, always UTC, and always `Z`-suffixed.
This is why §4.3's single-formatter rule is mandatory and separately tested.
(An alternative immune to format drift would be `ORDER BY datetime(at_utc), id`,
using SQLite's date functions — rejected because it defeats the index. The
format test P9 is the cheaper guard.)

`id ASC` is the insertion-order tiebreaker: AUTOINCREMENT guarantees
monotonically increasing ids, so `id` order *is* insertion order (and,
unlike plain `rowid`, AUTOINCREMENT guarantees no reuse after deletes — relevant
once editing lands post-MVP). This tiebreak is what makes E14 (a start and end
at the identical instant) deterministic.

Milestone 5 should still treat the sort as its own documented step-1
responsibility for fixture inputs it builds by hand — but against real storage
reads it is a no-op re-sort, and if M5 re-sorts it **must** use the identical
`(at_utc, id)` key, and must use a **stable** sort (`sort_by` / `sort_by_key`,
never `sort_unstable_*`) so it cannot perturb equal-key rows.

### 6.2 Notes — `ORDER BY created_at_utc ASC, id ASC`

```sql
SELECT id, date, body, created_at_utc
FROM notes
WHERE date = ?1
ORDER BY created_at_utc ASC, id ASC;
```

The `id` tiebreaker here is **not** optional. Because `created_at_utc` is stored
minute-truncated (§4.3), two notes added in the same minute — e.g. `mlm start
0900 "foo"` followed immediately by `mlm note "bar"`, or two notes in one
scripted invocation — carry **identical** `created_at_utc` values. Without
`, id ASC` their relative order would be unspecified, and PLAN.md's "notes for a
date are returned in insertion order" criterion would be flaky rather than
wrong-on-every-run. Test N6 covers exactly this by inserting two notes with the
same injected `now_utc`.

PLAN.md asks that ordering use "the insertion-order tiebreaker column, not just
`id` incidentally". Satisfied: `created_at_utc` is the primary key of the sort,
`id` only disambiguates ties within it. Test N7 proves `created_at_utc` is
genuinely the primary key by inserting rows whose id order and timestamp order
**disagree** (a note with an earlier injected `now_utc` inserted second) and
asserting the timestamp order wins.

---

## 7. Test plan

**Harness conventions** (apply to every test below):

- Unit tests in `src/storage.rs`'s `#[cfg(test)] mod tests`.
- Each test opens its **own** fresh database via Milestone 3's test seam (§0.2
  A5) — in-memory (`Connection::open_in_memory()` + migrations) preferred;
  a `tempfile`-backed path if any test needs a real file. Never the app-data
  directory.
- Timezone is always an explicit `chrono_tz::Tz` parameter; `chrono::Local` is
  never referenced in a test.
- "Now" is always an explicit `DateTime<Utc>` literal; `Utc::now()` is never
  called in a test.
- `TZ_WARSAW` (`chrono_tz::Europe::Warsaw`, UTC+1 winter / UTC+2 summer, EU DST
  rules) is the default test timezone. `chrono_tz::UTC` is used where an offset
  of zero makes an assertion clearer, and `America/New_York` for one
  negative-offset case.

### 7.1 Punch round-trip

- **P1 — start punch round-trip, UTC+1 (winter).** `insert_punch(Start,
  2026-01-15, 09:05, Warsaw)`; read `punches_for_date(2026-01-15)`. Expect one
  `Punch` with `kind == Start`, `at_utc == 2026-01-15T08:05:00Z`,
  `date == 2026-01-15`, `id` matching the returned insert id.
- **P2 — end punch round-trip, UTC+2 (summer).** `insert_punch(End, 2026-07-15,
  17:30, Warsaw)` → `at_utc == 2026-07-15T15:30:00Z`, `date == 2026-07-15`,
  `kind == End`. Asserts the offset genuinely varies by date, not by a constant.
- **P3 — `HH`-form time (minute defaults to zero).** Insert with
  `NaiveTime::from_hms(9,0,0)` → `at_utc` minute is `00`. (Guards against any
  accidental seconds/minutes mangling; M1 owns the parse itself.)
- **P4 — midnight boundary, local date ≠ UTC date.** `insert_punch(Start,
  2026-07-15, 00:30, Warsaw)` → `at_utc == 2026-07-14T22:30:00Z` but
  `date == 2026-07-15`. **This is the test that proves `date` is a stored
  app-computed column, not a slice of `at_utc`** (§2.1/§2.3). A string-slicing
  implementation fails here.
- **P5 — same, negative offset.** `insert_punch(Start, 2026-07-15, 23:30,
  America/New_York)` → `at_utc == 2026-07-16T03:30:00Z`, `date == 2026-07-15`.
  The mirror-image failure mode of P4.
- **P6 — read filters by local date.** Insert P4's punch plus one at
  2026-07-14 23:00 local; `punches_for_date(2026-07-15)` returns exactly one
  row (the 00:30 one), `punches_for_date(2026-07-14)` exactly the other — even
  though both `at_utc` values fall on 2026-07-14 UTC.
- **P7 — stored `date` equals the requested local date** for a plain non-DST
  case (asserts §4.1 step 4's round-trip derivation agrees with the argument).
- **P8 — empty date returns an empty vec, not an error** (§6.2 / E11 / F4).
- **P9 — stored `at_utc` byte format.** Read the raw column with a direct
  `SELECT at_utc` and assert the string is exactly `"2026-01-15T08:05:00Z"`
  — no `+00:00`, no fractional seconds. Guards §4.3/§6.1's lexical-sort
  precondition. Same assertion for `date` == `"2026-01-15"`.
- **P10 — kind round-trips exactly.** Assert the raw stored `kind` text is
  `"start"` / `"end"` (lowercase, matching §2.3's CHECK).

### 7.2 Punches come back in sorted order

- **P11 — sorted by instant regardless of insertion order.** Insert, in this
  order, F3's exact scenario: `Start 09:00`, `Start 14:00`, `End 18:00`,
  `End 13:00`, all on one date. Read back; assert the `kind`/time sequence is
  `Start 09:00, End 13:00, Start 14:00, End 18:00`. (Storage's half of F3 — M5
  owns the pairing, M4 owns handing it the right order.)
- **P12 — identical instants tie-break by insertion order.** Insert `Start
  12:00` then `End 12:00` on the same date (E14's data). Read back; assert
  `[Start, End]` in that order and that `punches[0].id < punches[1].id`. Then
  repeat with the insertion order reversed (`End` first, then `Start`) and
  assert the read-back order is `[End, Start]` — i.e. insertion order, not a
  kind-based or alphabetical preference, decides the tie.
- **P13 — order holds across a DST boundary within one date.** See D2.

### 7.3 Note trim-and-store

- **N1 — padded note is trimmed.** `insert_note(2026-01-15, "  did a thing  ")`
  → read back `body == "did a thing"`. (E5's negative case, the acceptance
  criterion's "padded but not empty is accepted".)
- **N2 — mixed whitespace kinds are trimmed.** `"\t\n  fixed the bug \r\n"` →
  `"fixed the bug"`.
- **N3 — interior whitespace preserved.** `"  did   a   thing  "` →
  `"did   a   thing"` (three-space runs intact, not collapsed).
- **N4 — an already-clean note is stored unchanged**, including one with a
  project prefix: `"mlm: fixed migration runner bug"` round-trips byte-identical
  (§2.3's "project-name prefix stays in the text").

### 7.4 Empty / whitespace note rejection (E5)

- **N5 — rejection cases.** Each of `""`, `" "`, `"   "`, `"\t"`, `"\n"`,
  `"\t \n \r "`, and `"\u{00A0}"` (NBSP-only) passed to `insert_note` returns
  `Err(StorageError::EmptyNote)`, **and** a follow-up `notes_for_date` for that
  date returns an empty vec — i.e. nothing was written (§6.1's "reject, no
  write"). Table-driven over the list.
- **N6 — non-empty boundary.** `"."` and `" x "` are accepted (single
  non-whitespace character survives), storing `"."` and `"x"`.
- **N7 — atomicity via `insert_punch_with_note`** (if §3.4 is built here):
  calling it with a valid time and `Some("   ")` returns
  `Err(EmptyNote)` and leaves **both** tables empty for that date — no orphaned
  punch. This is the precise storage-level guarantee Milestone 7's E5 criterion
  depends on.

### 7.5 Notes in insertion order

- **N8 — plain insertion order.** Three notes inserted with strictly increasing
  `now_utc` values; read back in that order.
- **N9 — same-minute ties fall back to id.** Two notes inserted with the
  **identical** `now_utc`; read back in insertion order. (Guards §6.2's
  mandatory `, id ASC`.)
- **N10 — `created_at_utc` is the primary sort key, not `id`.** Insert note A
  with `now_utc = 12:05Z`, then note B with `now_utc = 12:01Z` (so `id` order
  and timestamp order disagree). Read back; assert B precedes A. A
  `ORDER BY id` — only implementation fails this.
- **N11 — notes are scoped to their date.** Notes on two dates; each date's read
  returns only its own. Empty date → empty vec (F4/E11 support).
- **N12 — notes and punches are independent.** A date with notes and no punches
  reads back notes plus an empty punch vec, and vice versa (F4).

### 7.6 DST-transition round-trip (F12, storage half)

Fixed reference transitions in `Europe/Warsaw` (EU rules: last Sunday of March /
October):
- **Spring forward**: 2026-03-29, local 02:00 → 03:00. Offset UTC+1 → UTC+2.
- **Fall back**: 2026-10-25, local 03:00 → 02:00. Offset UTC+2 → UTC+1.

- **D1 — different dates either side of spring-forward (F12's literal shape).**
  Insert `Start 2026-03-28 12:00` and `Start 2026-03-30 12:00`, both Warsaw.
  Assert `at_utc` values are `2026-03-28T11:00:00Z` (UTC+1) and
  `2026-03-30T10:00:00Z` (UTC+2) respectively, and each `date` equals its own
  local date. **A cached-single-offset implementation produces the same offset
  for both and fails.** Assert the two differ by 46 hours, not 48, as an
  explicit second check.
- **D2 — both sides of the transition on the transition date itself.** Insert
  `Start 2026-03-29 01:30` (still UTC+1 → `2026-03-29T00:30:00Z`) and
  `End 2026-03-29 03:30` (already UTC+2 → `2026-03-29T01:30:00Z`). Both carry
  `date == 2026-03-29`; `punches_for_date` returns them in that order; the
  elapsed UTC gap is 60 minutes despite a two-hour wall-clock gap. This is the
  sharpest per-instant test — a naive implementation using the *date's* offset
  (rather than the *instant's*) fails it even though it looks per-date-correct.
- **D3 — fall-back ambiguity resolves to the earlier instant.** Insert
  `Start 2026-10-25 02:30` (Warsaw, ambiguous — occurs at both
  `00:30Z` and `01:30Z`). Assert `at_utc == 2026-10-25T00:30:00Z` (the earliest
  arm of `LocalResult::Ambiguous`), `date == 2026-10-25`, and that the call
  succeeded rather than erroring. Pins §4.1 step 2's documented choice.
- **D4 — fall-back, both sides across different dates.** `2026-10-24 12:00` →
  `10:00Z` (UTC+2) and `2026-10-26 12:00` → `11:00Z` (UTC+1). Mirror of D1.
- **D5 — spring-forward gap is a hard error.** Insert
  `Start 2026-03-29 02:30` (Warsaw — this local time does not exist). Assert
  `Err(StorageError::NonexistentLocalTime { .. })` and that nothing was written.
  **Marked as testing a proposed rule, not a spec'd one** — see §8, item 3; if
  the resolution changes, this is the one test that changes with it.
- **D6 — a zero-offset timezone is unaffected.** Same inserts under
  `chrono_tz::UTC` produce `at_utc` equal to the local datetime and matching
  dates — a sanity control proving the tests aren't passing by coincidence of
  offset arithmetic.
- **D7 — notes are DST-agnostic.** A note inserted on the transition date with
  an arbitrary injected `now_utc` stores the `local_date` it was handed, with no
  conversion involved. Guards against someone "helpfully" adding timezone logic
  to `insert_note`.

### 7.7 Error surface

- **X1 — DB error propagates, not panics.** Against a connection whose
  `punches` table has been dropped (or a closed/unmigrated connection), both
  inserts and both reads return `Err(StorageError::Db(_))` rather than
  panicking or unwrapping (feeds E6; full command-level wiring is M7/M10's).
- **X2 — corrupt-row tolerance.** Hand-insert a row with
  `at_utc = 'not-a-timestamp'` via raw SQL (bypassing the typed insert) and
  assert `punches_for_date` returns `Err(StorageError::CorruptRow { .. })`
  rather than panicking.
- **X3 — the schema CHECK still holds under typed inserts.** Sanity: a raw
  `INSERT` with `kind = 'START'` (wrong case) is rejected by SQLite. Belongs to
  M3, asserted once here as a guard that `PunchKind::as_str` emits the exact
  accepted literal.

---

## 8. Ambiguities, risks, and disagreements to cross-check

Ordered most-consequential first. Items 1–3 need a decision before the TDD agent
finishes; 4–9 are flags for the cross-milestone review.

1. **The `Punch` shape is asserted here, not agreed** (PLAN contract 1;
   affects Milestone 5, being designed in parallel *right now*). §2.2 commits to
   `{ id: i64, at_utc: DateTime<Utc>, date: NaiveDate, kind: PunchKind }` — the
   full row, typed, with `PunchKind` as an enum and `date` carried alongside
   `at_utc` rather than re-derived. The four specific decision points most
   likely to have been guessed differently by M5's author:
   (a) full row vs. a lighter `(DateTime<Utc>, PunchKind, i64)` — PLAN
   explicitly flags this as needing pinning;
   (b) typed `DateTime<Utc>` vs. raw RFC 3339 `String`;
   (c) `date` present vs. absent (M5 filters by date upstream, so it *could* be
   dropped — but M11's per-day rollup wants it);
   (d) whether `Punch` is defined in `storage.rs` and imported by M5, or defined
   in M5's module and constructed by storage (this plan says **storage owns the
   type**, M5 imports it — the inverse would make M5 the dependency root, which
   contradicts PLAN's wave assignment). **Reconcile these four before either
   worktree writes code.**

2. **Milestone 3 must provide a test seam for opening a non-app-data,
   migrated database** (§0.2 A5). Every test in §7 depends on it. `db::connect()`
   as currently scaffolded hardcodes `ProjectDirs` and, worse, `expect()`s on
   both the directory creation and the ProjectDirs lookup — which also
   contradicts §6.1's "DB open failure is a *hard error*, the command aborts"
   (a panic is not a nonzero-exit-with-stderr-message). Flagging both to M3.

3. **The spring-forward gap is not covered by the spec.** §2.1/§6.1 say nothing
   about a user typing a local time that does not exist (`mlm start 0230` on a
   spring-forward Sunday). Three options: (a) hard error — **this plan's
   proposal**, consistent with §6.1's treatment of other impossible time input
   like `24:00` (NOTES.md decision 30); (b) shift forward to the first existing
   instant, silently; (c) shift forward and warn. Option (a) is the only one
   that never silently records a time the user didn't ask for, but it *is* a new
   hard-error class not enumerated in §6.1, so it warrants a SPEC.md addition.
   **Rare but genuinely reachable in MVP**, since `start`/`stop` always target
   today and today can be a transition day. Test D5 encodes the proposal.

4. **Fall-back ambiguity resolution is likewise unspec'd.** §4.1 step 2 picks
   the *earliest* arm. Defensible and conventional, but a real choice with no
   spec backing — worth one line in SPEC.md §2.1. Note the user-visible
   consequence: during the repeated hour, punches at `02:30` and a later real
   `02:45` both land in the first pass, so a genuinely later second-pass punch
   can sort *before* an earlier first-pass one. Accepted; not fixable without
   asking the user which pass they mean, which is out of MVP scope. Related to
   §1.2's cross-midnight limitation in spirit.

5. **`created_at_utc` precision.** This plan truncates it to the minute for
   format uniformity with `at_utc` (§4.1's "no seconds precision anywhere"),
   which makes the `, id ASC` tiebreak load-bearing (§6.2, test N9). The
   alternative — full sub-second precision, since the column is internal and
   never displayed — would make ties essentially impossible but breaks format
   uniformity and the lexical-sort argument. Either works; **the choice must be
   consistent with whatever M3's migration documents**, and the `, id` tiebreak
   should be kept regardless as cheap insurance.

6. **`insert_punch_with_note` ownership** (§3.4). Placed in M4 here so
   atomicity is storage-tested (N7) rather than deferred to M7's CLI tests. If
   M7's plan also defines a combined write path, one of the two must yield —
   otherwise E5's "a rejected note does not leave an orphaned punch" gets two
   implementations. Recommend M4 owns the transaction, M7 owns the argument
   parsing and exit code.

7. **Milestone 6's query needs are not met by this milestone** (§3.5). M6 needs
   the earliest date with any punch *or* note data, and per-date punch sets
   across a whole week or a multi-week range. `punches_for_date` called in a
   loop technically suffices (and at personal-use scale is fine — §2.4 already
   accepts an O(weeks) walk), but it is N+1 queries and nobody has agreed who
   adds the range read. Suggested additions, as a small M4 follow-up rather than
   silent scope creep: `earliest_data_date(conn) -> Result<Option<NaiveDate>>`
   (a `MIN(date)` union across both tables) and `punches_in_range(conn, from,
   to) -> Result<Vec<Punch>>` (same ordering contract, plus `date ASC`).
   **Raise with the M6 worktree before either builds its own.**

8. **`chrono-tz` is a new dev-dependency** (§0.3) not currently in `Cargo.toml`.
   Dev-only, so the shipped binary is unaffected, but it is a `Cargo.toml` edit
   that will touch the same lines as any other milestone adding a dep —
   a likely trivial merge conflict worth expecting. Also note `tempfile` may be
   wanted if any test needs a file-backed DB rather than in-memory.

9. **Minor spec observation, no action needed**: §2.3's `notes` table has no
   `kind`/`source` column, so a note attached to a `start` punch (F5) is
   indistinguishable from a standalone `note` (§3.4) once stored. That is
   consistent with §3.5's flat "that date's notes, in insertion order"
   rendering, and appears intentional — recording it so a reviewer doesn't read
   it as a missing column.

---

## 9. Definition of done

- `src/storage.rs` exists with §2's types and §3's functions, all public items
  documented with the spec section they implement.
- No `Local::` reference, no `Utc::now()`, no `unwrap()`/`expect()` on a
  fallible path anywhere in the module.
- Every test in §7 passes; `cargo test` is green; `cargo clippy` is clean.
- `date`, `at_utc`, `created_at_utc`, and `kind` are written through the single
  canonical formatter / `as_str` — verified by P9 and P10 asserting raw column
  bytes, not just round-tripped values.
- The `Punch` type in §2.2 has been diffed against Milestone 5's landed plan and
  any mismatch from §8 item 1 resolved, not papered over with an adapter.
