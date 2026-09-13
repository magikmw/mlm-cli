# Milestone 4 review — Punch and note storage

Independent adversarial review. Verified against SPEC.md §2.1/§2.3/§6.1,
PLAN.md's pinned interface contracts (8, 10), the merged real APIs in
`src/db.rs`/`src/time.rs`/`src/date.rs`, and `src/stint.rs`'s Punch/PunchKind
stand-in. `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D
warnings`, and `cargo fmt --check` were all run directly in this worktree
(not taken on the implementer's word).

## Verified clean

- `cargo build`: clean. `cargo test`: **169 passed, 0 failed** (41 in
  `storage::tests`), matching the report. `cargo clippy --all-targets -- -D
  warnings`: clean. `cargo fmt --check`: clean.
- SQL is fully parameterized everywhere (`?1`/`?2`/... placeholders); no
  string-interpolated user data reaches a query. No SQL injection surface.
- No `.unwrap()`/`.expect()` on a fallible/user-input path. The only
  `.expect()` calls in non-test code (`storage.rs:201,203,284,286`) are on
  `NaiveTime::with_second(0)`/`with_nanosecond(0)`, which are infallible for
  any already-valid `NaiveTime`/`DateTime` — correctly reasoned, not a risk.
- Per-instant DST conversion is correct and genuinely exercised: D1-D6 use
  real Europe/Warsaw 2026 transition dates (spring-forward March 29,
  fall-back October 25), assert distinct offsets either side, assert the
  spring-forward gap is a hard error with nothing written, and assert
  fall-back ambiguity resolves to the earliest instant. D2 in particular
  (`01:30`→`00:30Z` and `03:30`→`01:30Z` on the transition date itself, 60
  real minutes apart) is a genuine per-instant test that a cached-offset
  implementation would fail. The report's correction of the plan's D1
  arithmetic (47 hours, not 46) is correct — verified by direct
  subtraction of the two asserted instants.
- Note validation order is correct: `body.trim().is_empty()` is checked
  before trimming, `EmptyNote` returned before any `execute`, NBSP-only
  input rejected (N5), padded-but-nonempty accepted and stored trimmed
  (N1/N6), interior whitespace preserved (N3), nothing written on
  rejection including via `insert_punch_with_note` (N7 — validated before
  the transaction even opens).
- Notes are read back `ORDER BY created_at_utc ASC, id ASC`, and N9/N10
  genuinely prove `id` is only a tiebreak, not the primary key
  (N10 inserts an earlier `now_utc` second and asserts it still sorts
  first) — matches SPEC.md §2.3's final wording on `created_at_utc`
  granularity and the `id` tiebreak.
- `earliest_data_date()` and `punches_in_range()` (PLAN.md contract 10)
  exist with sane signatures, return `None`/empty `Vec` correctly for an
  empty database, compute the correct cross-table minimum, and the range
  read is `BETWEEN ?1 AND ?2` (correctly inclusive on both ends, per its
  own doc comment and test coverage).
- `Punch`/`Note` field types and the split between `insert_punch`'s
  `local_date`+`local_time` parameters vs. `insert_note`'s injected
  `now_utc` match the plan and PLAN.md contract 8/6 exactly. No hidden
  `Local::` or `Utc::now()` reference anywhere in the module.
- `Punch`/`PunchKind` field-for-field shape (names, types, order) matches
  `src/stint.rs`'s stand-in exactly, so the M5 integration swap
  (`use crate::storage::{Punch, PunchKind};` replacing the stand-in block)
  is a pure import change, as the report claims.

## Findings

1. **`punches_for_date`/`punches_in_range` do not produce the tiebreak order
   PLAN.md contract 10 and SPEC.md §4.3 step 1 require, and the code's own
   doc comments and a test assert the wrong order as correct.**
   `src/storage.rs:348-359` and `:364-387` — both queries are
   `ORDER BY at_utc ASC, id ASC` (no `kind` in the tiebreak). SPEC.md §4.3
   step 1 (`SPEC.md:323-328`) is explicit: "ties at an identical instant are
   broken first by kind (`start` before `end`), then by `id`." PLAN.md's
   Milestone 4 acceptance criteria for contract 10 (`PLAN.md:378-384`) state
   punches "are returned in a stable, deterministic order suitable for
   feeding directly into Milestone 5's sort step (sorted by instant, ties
   broken by **kind then insertion order**... either the read itself sorts
   this way, **or the milestone documents that the caller must**." Milestone
   4 does neither: the read sorts by `id` alone at tied instants, and the
   doc comment on `punches_for_date` (`storage.rs:345-347`) asserts the
   opposite — "ties broken by insertion order (`at_utc ASC, id ASC`) —
   **already in §4.3 step 1's required order**" — which is false for a
   tied `(start, end)` pair entered end-first. Test P12
   (`storage.rs:648-672`) locks this in: it explicitly asserts that
   inserting `End` before `Start` at an identical instant reads back as
   `[End, Start]`, i.e. pure insertion order — the reverse of what SPEC's
   own worked E14 case requires at the sort step.
   In practice this is masked because `src/stint.rs::classify()`
   explicitly does not trust caller order and re-sorts with the correct
   `(at_utc, kind, id)` key itself (`stint.rs:137-138,154`) — so Milestone
   5's actual pairing is unaffected today. But the milestone-4 plan itself
   silently overrode PLAN.md's pinned contract without flagging it (its
   §6.1 argues for pure id-order and never reconciles this against
   PLAN.md's explicit kind-based wording), the report never surfaces the
   discrepancy despite the report explicitly discussing tiebreak order
   elsewhere, and any future consumer of `punches_for_date`/
   `punches_in_range` that trusts the doc comment (e.g., a rendering
   milestone that lists raw punches for a date without re-sorting) will
   silently get the wrong same-instant order. The doc comments should
   either be fixed to admit the caller must re-sort by kind, or the SQL
   should add `kind` to the `ORDER BY` (trivial: `CASE kind WHEN 'start'
   THEN 0 ELSE 1 END` or sorting by `kind DESC` since `'end' > 'start'`
   lexically works out — needs a moment's care) and P12's assertion of
   `[End, Start]` needs to flip.
   Severity: **significant**.

2. **`PunchKind`'s derive list is not byte-for-byte identical to
   `stint.rs`'s stand-in, contradicting the report's explicit claim.**
   `storage.rs:53` derives `Hash` in addition to
   `Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord`;
   `stint.rs:25` does not derive `Hash`. The report (`milestone-4-report.md:84-88`)
   states the shapes are "byte-for-byte identical... same field names,
   order, and derives" — this is factually wrong for `PunchKind` (true for
   `Punch`/`Note`). Functionally harmless — an added derive is backward
   compatible with every existing use of the stand-in, so the M5
   integration swap is still a pure import change — but the report's
   claim should not be taken at face value here.
   Severity: **minor**.

## Not a defect (checked, found correct)

- The `date` column is genuinely computed by round-tripping through the
  timezone (`at_utc.with_timezone(tz).date_naive()`), not by reusing
  `local_date` verbatim or string-slicing `at_utc` — P4/P6 prove this
  (local date differs from UTC date at the midnight boundary, and reads
  filter by the stored local date, not the UTC date).
- `at_utc`/`date`/`created_at_utc` are written through the single
  canonical `fmt_utc`/`%Y-%m-%d` formatters only; P9 asserts the exact raw
  byte string (`"2026-01-15T08:05:00Z"`, no `+00:00`, no fractional
  seconds), which is what makes the lexical `ORDER BY at_utc` valid.
- `chrono-tz` is correctly a `[dev-dependencies]`-only entry
  (`Cargo.toml:18-20`), not shipped in the production binary, matching the
  plan's §0.3 intent (already true before M4 landed, per the report).
- `db.rs`'s real test seam (`Connection::open_in_memory()` +
  `db::apply_migrations`) is used correctly and exclusively by every
  storage test; no test touches `db::connect()`/`ProjectDirs`.
- `insert_punch_with_note`'s transaction is atomic and validates before
  opening the transaction, confirmed by N7 and by the two positive-path
  tests immediately following it.

## Verdict

**APPROVE WITH NITS** — build/test/lint all verified green, the DST and
note-validation logic is correct and well-tested, and the Punch/PunchKind/
Note handoff to Milestone 5 is a clean, working integration point. Finding
1 (the punch-read tiebreak diverging from PLAN.md contract 10 and SPEC.md
§4.3, with a misleading doc comment and a test that encodes the wrong
order) does not currently break anything because Milestone 5 defensively
re-sorts and never trusts the read's order — but it should be fixed (SQL
`ORDER BY` extended to include `kind`, or the doc comments and P12
corrected to honestly say "caller must re-sort by kind") before any future
milestone reads these functions and trusts the stated contract at face
value.
