# Milestone 4 report — Punch and note storage

## What was implemented

`src/storage.rs` (new), registered via `mod storage;` in `src/main.rs`.
No other files touched.

- `PunchKind` (Copy, Ord, `Start` < `End`), `Punch` (Copy), `Note` — the
  sole-owned types per PLAN.md contract 8, field-for-field matching
  `stint.rs`'s existing local stand-in.
- `StorageError` (Db, CorruptRow, EmptyNote, NonexistentLocalTime) with
  `Display`/`std::error::Error`/`From<rusqlite::Error>`.
- Write path: `insert_punch`, `insert_note`, `insert_punch_with_note`
  (transactional, §3.4's atomicity helper), `punch_from_local` (test
  fixture helper for Milestone 5).
- Read path: `punches_for_date`, `punches_in_range`, `notes_for_date`,
  `earliest_data_date` (PLAN.md contract 10, for Milestones 6/11).
- Canonical UTC formatter/parser (`%Y-%m-%dT%H:%M:%SZ`), per-instant
  `TimeZone::from_local_datetime` conversion handling all three
  `LocalResult` arms (spring-forward gap → hard error, fall-back
  ambiguity → earliest instant).

## Test / build / lint state

- `cargo build`: clean, no warnings.
- `cargo test`: **169 passed, 0 failed** (41 in `storage::tests`, plus
  all pre-existing M1/M2/M3/M5/M6 tests unaffected).
- `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo fmt`: applied (reformatted one multi-line helper it didn't like).

Full test list implemented per the plan's §7: P1–P13, N1–N12, D1–D7,
X1–X3, plus `earliest_data_date`/`punches_in_range` coverage the plan's
§7 didn't enumerate line-by-line but the acceptance criteria required.

## Deviations from the plan

1. **`db::connect_at`/`apply_migrations` used as the actual M3 test seam**
   (plan's §0.2 A5 guessed at this; it landed almost exactly as guessed).
   Tests use `Connection::open_in_memory()` + `db::apply_migrations`,
   never `db::connect()`/`ProjectDirs`.
2. **D1's arithmetic**: the plan states the UTC gap between
   `2026-03-28T11:00:00Z` and `2026-03-30T10:00:00Z` is "46 hours, not
   48". Direct subtraction of those two instants is **47 hours** (48
   calendar hours minus the 1-hour DST offset change), not 46. Fixed
   the test to assert 47 with a comment noting the plan's arithmetic
   slip; the underlying storage behavior (each date converts through
   its own offset) needed no change — this was purely a wrong expected
   number in the plan, not a design bug.
3. **`PunchKind::from_str`** carries a documented
   `#[allow(clippy::should_implement_trait)]`, since the plan pins this
   exact public name/shape as an inherent method rather than
   `impl std::str::FromStr`.
4. `chrono-tz` was already a dev-dependency (added when Milestone 3
   landed, per PLAN.md's consolidated-dev-deps decision) — no
   `Cargo.toml` edit was needed, contrary to the plan's §0.3/§8 item 8
   assumption that M4 would be first to add it.

No other deviations; `db.rs`/`time.rs`/`stint.rs`/`date.rs`/`week.rs`/
`cli.rs` were not touched.

## Final shape of `Punch`/`PunchKind`/`Note`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PunchKind { Start, End }  // Start < End

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Punch {
    pub id: i64,
    pub at_utc: chrono::DateTime<chrono::Utc>,
    pub date: chrono::NaiveDate,
    pub kind: PunchKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub id: i64,
    pub date: chrono::NaiveDate,
    pub body: String,
    pub created_at_utc: chrono::DateTime<chrono::Utc>,
}
```

This is **byte-for-byte identical** to `src/stint.rs`'s current local
stand-in of `Punch`/`PunchKind` (same field names, order, and derives).
The Milestone 5 integration swap is exactly what its own comment says:
delete the stand-in block in `stint.rs` and add
`use crate::storage::{Punch, PunchKind};` — no adapter needed.

## Open questions / risks for the reviewer

- **D1 arithmetic fix (above)** — worth a quick second look in case I've
  mismodeled the intended dates; I'm confident in the direct subtraction
  but flagging since it contradicts the plan's own stated number.
- **Spring-forward-gap hard error and fall-back-earliest-instant
  resolution** are both *proposed* rules (plan §8 items 3–4), not
  currently written into SPEC.md itself. I implemented them as
  specified by the plan and PLAN.md's Milestone 4 acceptance criteria
  (which do state these rules), but SPEC.md §2.1 itself already
  documents both cases explicitly and matches — no SPEC.md gap found
  after re-reading; the plan's "not spec'd" framing was written before
  SPEC.md was amended to include §2.1's two DST bullets, so this is
  stale in the plan, not a real open question.
- `insert_punch_with_note` was built here (per plan §3.4/§8 item 6,
  "recommend M4 owns the transaction, M7 owns argument parsing/exit
  codes") — flagging so Milestone 7's implementer doesn't duplicate it.
- No production code path yet calls any of this module's functions
  (`main.rs` still has its M7-owned TODO stubs) — expected at this
  stage; `#![allow(dead_code)]` mirrors `stint.rs`'s same situation.
