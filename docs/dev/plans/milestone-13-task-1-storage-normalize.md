# Milestone 13, Task 1 — low-level implementation plan

`src/storage.rs`: normalize embedded newlines in note bodies.

**Parent plan:** `docs/dev/plans/milestone-13-note-body-normalization.md`
("Task 1: `storage.rs` — normalize embedded newlines in note bodies").
**Spec:** `docs/dev/specs/2026-09-13-delete-punches-notes.md` §2.
**File under change:** `src/storage.rs` only (implementation + its own
`#[cfg(test)] mod tests`).

This document is a plan only. No code in this document has been
applied to `src/storage.rs`.

---

## 1. Current state (as read, exact line numbers from the file as it
   stands today)

- `insert_note` is defined at `src/storage.rs:272-296`:

  ```
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
  ```

  The line we are changing is line 281: `let stored = body.trim();`.

- `StorageError` enum: `src/storage.rs:135-152`. `Display` impl:
  `src/storage.rs:154-170` (exhaustive `match` over 4 variants: `Db`,
  `CorruptRow`, `EmptyNote`, `NonexistentLocalTime`). `source()` impl:
  `src/storage.rs:172-181` (same 4-variant exhaustive `match`). **This
  task adds no new `StorageError` variant**, so neither exhaustive
  match needs a new arm — confirmed by the parent plan's "no new error
  variant" constraint and the fact that a silent-normalize design has
  no new failure case to represent.

- Test module: `#[cfg(test)] mod tests` starts at `src/storage.rs:439`.
  Section for note trim-and-store tests is "--- 7.3 Note trim-and-store
  -----------------------------------------" at `src/storage.rs:685`,
  containing tests `N1`-`N4` (`src/storage.rs:687-745`). The
  empty/whitespace-rejection section "--- 7.4 Empty / whitespace note
  rejection (E5) ---" follows immediately at `src/storage.rs:747`,
  containing `N5`-`N7` plus three unlabeled `insert_punch_with_note`
  tests (`src/storage.rs:749-835`).

- Naming/labeling convention observed: tests are grouped under a
  `// --- <section number/name> ---` comment banner matching the
  relevant spec section, each test additionally tagged with a short
  ordinal comment (`// N1`, `// N2`, ...) on the line directly above
  `#[test]`, and named as a `snake_case` sentence describing the
  behavior under test (e.g. `padded_note_is_trimmed`,
  `mixed_whitespace_kinds_are_trimmed`,
  `interior_whitespace_is_preserved`). New tests for this task continue
  the `N`-series numbering (next free number is `N13`, since `N1`-`N12`
  are already used: `N1`-`N7` in §7.3/§7.4, `N8`-`N12` in §7.5).

- Existing fixture helpers available in the test module (all defined
  before line 685, reusable as-is, no new fixture helper needed):
  `test_db()` (`src/storage.rs:447`), `d(y, m, day) -> NaiveDate`
  (`src/storage.rs:453`), `t(h, m) -> NaiveTime` (`src/storage.rs:457`),
  `utc(y, mo, day, h, mi, s) -> DateTime<Utc>` (`src/storage.rs:461`).

- Call-site audit (grep, confirmed): `insert_note` is called from
  exactly three places in the crate:
  1. `src/storage.rs:320` — inside `insert_punch_with_note` (the
     inline-note-on-`start`/`stop` path), itself called from
     `src/commands.rs:84` (`punch()`, shared by `start`/`stop` per
     `src/commands.rs:21-30`).
  2. `src/commands.rs:38` — inside `note()`, the standalone `note`
     command's handler.
  3. `src/status.rs:1061` — a test fixture inside `status.rs`'s own
     `#[cfg(test)] mod tests` (not a production call site).

  Because both production call sites (`commands.rs:38` and
  `commands.rs:84` via `storage.rs:320`) funnel through the single
  `insert_note` function body, normalizing inside `insert_note` covers
  the inline-note-on-`start`/`stop` path "for free" — no change needed
  in `commands.rs` or in `insert_punch_with_note`
  (`src/storage.rs:302-325`) itself. `insert_punch_with_note`'s own
  pre-transaction empty check at `src/storage.rs:311-315`
  (`body.trim().is_empty()`) is a fast-reject optimization ahead of
  opening a transaction, not a duplicate of the normalization logic —
  it stays exactly as-is; it only decides *whether* to proceed, never
  what gets stored.

---

## 2. New helper

**Name:** `normalize_newlines`
**Location:** `src/storage.rs`, placed directly above `insert_note`
(i.e. immediately after `to_utc_and_date`/`punch_from_local`'s block
ends at line 243, and before the doc comment block that currently
starts at line 266 for `insert_note`) — grouped with `insert_note`
since it exists solely to serve that one call site, mirroring how
`fmt_utc`/`parse_utc`/`parse_date` sit directly above the functions
that use them (lines 27-45).

**Signature:**
```
fn normalize_newlines(s: &str) -> String
```
- Private (`fn`, no `pub`), free function, not a method — consistent
  with `fmt_utc`, `parse_utc`, `parse_date`, `to_utc_and_date` all
  being free private functions in this file rather than inherent
  methods on a type.
- Takes `&str`, returns an owned `String` — it cannot return a
  sub-slice or a `Cow` cheaply in the general case, since a run of
  multiple `\r`/`\n` collapses to a single space, which is a genuine
  rewrite of the byte content whenever a run has length > 1. Returning
  `String` unconditionally keeps the signature simple and matches
  `fmt_utc`'s existing `-> String` precedent; no need for a `Cow`
  micro-optimization in a CLI note-insert path that runs once per
  command invocation.
- No `Result` — this function cannot fail; it is a pure text
  transform, consistent with the parent plan's "silent normalize, not
  reject" constraint (no new failure mode to report).

**Doc comment** (placed directly above the `fn`, matching this file's
convention of a doc comment on every non-trivial function): explain
that it collapses every maximal run of `\r`/`\n` to exactly one ASCII
space, leaves every other character (including other whitespace:
space, tab) untouched, and is applied to an already-trimmed body so it
only ever needs to handle *embedded* runs, not leading/trailing ones
(those no longer exist by the time this runs — see §4 below for why
this matters for the algorithm, though the algorithm behaves correctly
either way).

---

## 3. Exact algorithm (prose/pseudocode, not final Rust)

Single forward pass over the input's `char`s, building an output
`String`:

1. Create `out = String::with_capacity(s.len())` (upper bound on final
   length since the transform never grows the string — a run of N
   `\r`/`\n` chars becomes 1 space, i.e. the output is never longer
   than the input).
2. Iterate over `s.chars()` using a peekable/manual-index char
   iterator (needed because the algorithm must look ahead to consume
   a whole run before deciding to push a single space).
3. On each character:
   - If the character is **not** `\r` and **not** `\n`: push it to
     `out` unchanged, advance by one character.
   - If the character **is** `\r` or `\n`: this is the start of a run.
     Continue consuming characters from the iterator, without pushing
     anything, for as long as the next character is also `\r` or `\n`
     (this is what correctly handles a `\r\n` pair, or `\n\n\n`, or any
     mixed run like `\r\n\r\n`, as exactly one run rather than one unit
     per character). Once the run ends (next character is neither, or
     the input ends), push exactly one `' '` (ASCII space) to `out`,
     then resume normal iteration from the first non-`\r`/`\n`
     character.
4. Return `out` once the iterator is exhausted.

Net effect: every maximal run of one-or-more `\r`/`\n` characters,
wherever it appears in the string (including at the very start or
end, though that case never actually reaches this function in
practice — see §4), is replaced by exactly one space; every other
character (letters, digits, punctuation, spaces, tabs, unicode) passes
through untouched, in its original relative position and count.

Two Global-Constraints edge cases fall out of this design with no
extra logic, worth naming explicitly: (a) a plain space/tab sitting
between two `\r`/`\n` runs is "not `\r`/`\n`" per step 3, so it is
pushed unchanged and ends the first run immediately — it is never
absorbed into either run's replacement space, which is exactly the
spec's interrupted-run tie-break rule (see the `"a\n\n \nb" ->
"a   b"` acceptance test in §5); (b) U+2028/U+2029 are likewise "not
`\r`/`\n`" per step 3 and always pass through untouched, satisfying
the Global Constraints' out-of-scope note for Unicode line separators
without the algorithm needing to special-case them.

Implementation note for whoever writes the real Rust: a
`chars().peekable()` loop, or equivalently `s.split_inclusive(...)`,
or a manual `while let Some(c) = chars.next()` with an inner `while
matches!(chars.peek(), Some('\r' | '\n')) { chars.next(); }` all
realize this algorithm; the plan intentionally leaves the exact
iterator-combinator choice to the implementer since it has no
behavioral consequence, only style — as long as it is a single pass
(no `regex`, no repeated `.replace()` chains that could double-count
overlapping runs).

---

## 4. Exact one-line change to `insert_note`, and where normalization
   goes in the function body

**Current line 281** (inside `insert_note`, `src/storage.rs:272-296`):
```
    let stored = body.trim();
```
This binds `stored: &str` — a borrow of `body` with leading/trailing
whitespace removed — which is then passed by reference into
`conn.execute`'s parameter tuple at line 291.

**Becomes:**
```
    let stored = normalize_newlines(body.trim());
```
This changes `stored`'s type from `&str` to `String` (the return type
of `normalize_newlines`). The parameter tuple at
`conn.execute("INSERT INTO notes ...", (..., stored, ...))` (line
287-294) does not need to change: `rusqlite`'s parameter binding
accepts owned `String` via its `ToSql` impl exactly as it already
accepts `&str`, so no further edit is needed at the call site below
this line — only the one `let` line changes.

**Ordering, and why it must go exactly here:**

- Line 278-280 (the empty-check, `if body.trim().is_empty() { return
  Err(StorageError::EmptyNote); }`) **must stay first, unchanged, and
  must keep testing `body.trim()`, not a normalized value.** Reasoning
  tied to the actual code: the check runs on `body.trim()` — i.e. on
  whitespace-trimmed but not yet newline-collapsed text. A body that
  is *only* newlines/spaces (e.g. `"\n\n"`) has `body.trim()` produce
  `""`, which is already caught as empty *before* `normalize_newlines`
  ever runs — this is exactly the parent plan's "the existing
  empty/whitespace-only rejection must keep firing exactly as before
  ... checked before normalization touches anything" constraint,
  satisfied by construction: the empty-check line (278-280) precedes
  the trim/normalize line (281) in program order today, and nothing in
  this change reorders that. Do not move the empty-check below the
  new `let stored = ...` line, and do not change the empty-check's own
  condition to run on a normalized value — it must keep checking
  `body.trim()` exactly as today.
- The trim (`body.trim()`) must run **before** `normalize_newlines`,
  not after, for two independent reasons visible in the algorithm
  itself: (a) trimming first removes leading/trailing `\r`/`\n` runs
  entirely (no space introduced at the edges — the parent plan's
  explicit acceptance criterion), whereas normalizing first and
  trimming spaces after would leave a stray space at each edge where a
  leading/trailing newline run used to be; (b) `str::trim()`'s
  Unicode-whitespace definition already includes `\r`/`\n`, so
  trimming first is what guarantees `normalize_newlines` only ever
  sees *embedded* runs, matching its doc comment's stated contract.
  Concretely: `normalize_newlines(body.trim())` (trim, then
  normalize) — not `normalize_newlines(body).trim()` (which would
  produce the wrong edge behavior) and not two separate `let`
  bindings — a single expression composing the two, matching the
  file's existing terse style (e.g. line 281 today is already a single
  expression).
- `now_utc`'s normalization block (`src/storage.rs:282-286`) and the
  `conn.execute` call (`287-294`) are unaffected and keep their current
  relative position, after the `stored` binding.

Net diff shape for `insert_note`'s body: exactly one line changes
(line 281); no other line in the function moves or is rewritten.

---

## 5. Tests to write first (TDD) — exact list, named per this file's
   `N`-series convention, placed in the existing "--- 7.3 Note
   trim-and-store ---" section (after `N4`, i.e. after
   `src/storage.rs:745`, before the "--- 7.4 ---" banner at line 747),
   continuing the ordinal-comment convention

All new tests use the existing `test_db()`, `d()`, `utc()` fixtures;
none need a new helper.

1. **`// N13` `embedded_newline_collapses_to_single_space`** — seed
   `insert_note` with a body containing one interior `\n` (e.g.
   `"line one\nline two"`), no leading/trailing whitespace. Assert the
   read-back `notes[0].body == "line one line two"` (the exact
   spec-cited example from
   `docs/dev/specs/2026-09-13-delete-punches-notes.md` §2).

2. **`// N14` `embedded_carriage_return_newline_pair_collapses_to_one_space`**
   — seed with a body containing one interior `"\r\n"` pair (e.g.
   `"line one\r\nline two"`). Assert `notes[0].body == "line one line
   two"` — specifically assert there is exactly one space between
   "one" and "two" (e.g. via exact string equality, not a substring
   check), proving the pair became one space, not two.

3. **`// N15` `lone_embedded_carriage_return_collapses_to_single_space`**
   — seed with a body containing one interior `\r` with **no**
   accompanying `\n` (e.g. `"line one\rline two"`). Assert
   `notes[0].body == "line one line two"` exactly — a dedicated test
   distinct from the `\r\n`-pair case (N14), per the parent plan's
   explicit "a lone embedded `\r` (no accompanying `\n`)" acceptance
   criterion; the mixed-run test (N17 below) alone does not prove this
   case since it never exercises a bare `\r` in isolation.

4. **`// N16` `reversed_newline_carriage_return_pair_collapses_to_one_space`**
   — seed with a body containing one interior `"\n\r"` pair, the
   reverse ordering of N14 (e.g. `"line one\n\rline two"`). Assert
   `notes[0].body == "line one line two"` exactly — proves the run
   detection is order-agnostic, not hardcoded to the `\r`-then-`\n`
   ordering, per the parent plan's explicit "the reversed pairing, an
   embedded `\n\r`" acceptance criterion.

5. **`// N17` `run_of_several_newlines_and_carriage_returns_collapses_to_one_space`**
   — seed with a body containing a longer mixed run, e.g. `"line
   one\n\r\n\n\rline two"` (several consecutive `\n`/`\r` characters in
   any mixed order). Assert `notes[0].body == "line one line two"` —
   again exact equality, proving the *entire* run becomes exactly one
   space regardless of run length or character mix.

6. **`// N18` `newline_run_interrupted_by_plain_whitespace_keeps_both_runs_and_the_literal_space`**
   — the Global Constraints tie-break case (spec §2, matched exactly).
   Seed with the exact spec-cited shape `"a\n\n \nb"` (a two-`\n` run,
   then one literal space, then a one-`\n` run). Assert
   `notes[0].body == "a   b"` (three spaces: one from the first run,
   the untouched literal space passed through unchanged, one from the
   second run) — and explicitly assert it is **not** `"a b"`, i.e. the
   literal space between the two runs must never be merged into either
   run's replacement space or otherwise collapsed. This needs its own
   test because none of N13-N17 exercise two runs separated by
   non-newline whitespace; per §3's algorithm walkthrough this already
   falls out of the single-forward-pass design without any algorithm
   change, but the parent plan calls this out as needing its own
   explicit test rather than being inferred from the plain-run cases.

7. **`// N19` `unicode_line_separators_pass_through_untouched`** — seed
   with a body containing an embedded U+2028 (LINE SEPARATOR) and,
   separately or in the same body, U+2029 (PARAGRAPH SEPARATOR), e.g.
   `"line one\u{2028}line two\u{2029}line three"`. Assert
   `notes[0].body` is byte-for-byte identical to the trimmed input
   (i.e. `"line one\u{2028}line two\u{2029}line three"`, unchanged) —
   a positive assertion that both characters survive untouched, not
   collapsed to a space and not otherwise altered, per the Global
   Constraints' explicit U+2028/U+2029-out-of-scope note. No algorithm
   change is needed for this to pass (§3's run-detection only ever
   matches ASCII `\r`/`\n`), but the parent plan requires this as its
   own explicit positive test, not merely the absence of a test.

8. **`// N20` `leading_and_trailing_newlines_are_trimmed_not_spaced`**
   — seed with a body like `"\n\nline one\n\n"` (leading and trailing
   newline runs, no embedded ones). Assert `notes[0].body == "line
   one"` exactly — no leading or trailing space, proving the
   pre-existing trim step still fully removes edge runs rather than
   normalization turning them into edge spaces. (This directly guards
   the ordering decision in §4.)

9. **`// N21` `regular_internal_whitespace_untouched_by_normalization`**
   — seed with a body containing multiple interior spaces and a tab,
   no newlines, e.g. `"did   a\tthing"`. Assert `notes[0].body ==
   "did   a\tthing"` byte-for-byte — proving only `\r`/`\n` are
   targeted, not general whitespace. (This is a variant/companion of
   existing `N3` `interior_whitespace_is_preserved`, added instead of
   modifying `N3`, since `N3` already exists and passes today and
   should keep passing unmodified — new behavior gets its own test.)

10. **`// N22` `whitespace_and_newline_only_body_still_rejected_as_empty`**
    — seed `insert_note` with a body that is only newlines/spaces mixed
    together, e.g. `"\n \r\n \n"`. Assert the call returns
    `Err(StorageError::EmptyNote)` (same assertion style as existing
    `N5`) and that a follow-up `notes_for_date` read for that date is
    empty — proving the empty-check still fires and nothing is ever
    written, exactly as `N5` already proves for pure-whitespace bodies
    without newlines. This can either extend `N5`'s existing `cases`
    array (`src/storage.rs:752`) with a couple of newline-bearing
    strings, or stand as its own test — extending `N5`'s array is
    preferred since it is the exact same assertion shape over a list of
    inputs and avoids duplicating the loop; if reusing `N5`, no new
    `N22` comment/test is needed and this item becomes "add
    `"\n \r\n \n"` (and similar) to `N5`'s `cases` array at
    `src/storage.rs:752`" instead of a new test function. State
    explicitly in the implementation which choice was made, since the
    acceptance criteria in the parent plan phrase this as its own
    bullet.

11. **`// N23` `inline_note_on_punch_is_normalized_same_as_standalone_note`**
    — exercises `insert_punch_with_note` (not `insert_note` directly),
    confirming the fix is not accidentally local to one call site, per
    the parent plan's explicit acceptance criterion. Seed via
    `insert_punch_with_note(&mut conn, PunchKind::Start, d(...), t(...),
    &TZ_UTC, Some("line one\nline two"), utc(...))` (same call shape as
    the existing `insert_punch_with_note_commits_both_rows_together`
    test at `src/storage.rs:796-815`). Assert the returned `note_id` is
    `Some`, and that a follow-up `notes_for_date` read shows
    `notes[0].body == "line one line two"` — same normalized result as
    test 1, proving both call sites share the one fixed code path.

No existing test needs to change except the optional `N5` array
extension in item 10. All of `N1`-`N12` (existing) must keep passing
unmodified — they are the regression guard the parent plan calls out
("Full existing `storage.rs` test suite still passes ... in particular
the pre-existing empty-note and full-text-preservation tests").

---

## 6. Step-by-step sequencing

1. In `src/storage.rs`, insert the ten-to-eleven new test functions from
   §5 (depending on the N22/`N5`-array choice) into the existing
   `mod tests` block, in the "--- 7.3 Note trim-and-store ---" section
   right after `N4`
   (`already_clean_note_round_trips_byte_identical`, ending at line
   745) and before the "--- 7.4 ---" banner (line 747). Run `cargo
   test storage::tests` (or the narrower filter in §7) and confirm the
   new tests **fail** (compile is fine since they only call existing
   `insert_note`/`insert_punch_with_note`/`notes_for_date`, but the
   assertions fail because normalization doesn't exist yet) — this is
   the TDD red step.
2. Add the `normalize_newlines` private helper (§2) directly above
   `insert_note`, between line 243 (end of `punch_from_local`) and line
   266 (start of `insert_note`'s doc comment).
3. Change line 281 of `insert_note` from `let stored = body.trim();` to
   `let stored = normalize_newlines(body.trim());` (§4). No other line
   in `insert_note` changes.
4. Re-run the full `storage` test module. All new tests from §5 should
   now pass (TDD green), and all pre-existing tests (`N1`-`N12`, `P*`,
   `D*`, `X*`, the `earliest_data_date`/`punches_in_range` tests) must
   still pass unmodified.
5. Run clippy (§7) and address any lint it raises in the new code
   (expected clean given the algorithm has no obvious clippy trap, but
   verify rather than assume).
6. Do not touch `commands.rs`, `status.rs`, or any other file — this
   task's diff is scoped to `src/storage.rs` only, per the parent
   plan's "Files: `src/storage.rs` only" constraint. (Milestone
   13 Task 2 — `docs/dev/SPEC.md` §2.3 wording — is a separate task in
   the parent plan, not part of this document's scope.)

---

## 7. Verification

- Targeted test filter (fast inner loop while iterating on the helper):
  ```
  cargo test --lib storage::tests
  ```
  or, to run only the new/changed area precisely:
  ```
  cargo test --lib normalize
  ```
  (matches by substring against the new test names, e.g.
  `embedded_newline_collapses_to_single_space`, plus catches
  `normalize_newlines` if it were ever unit-tested directly — it isn't
  here, since it's exercised only through `insert_note`/
  `insert_punch_with_note`, consistent with this file's existing
  convention of testing private helpers like `to_utc_and_date`/
  `fmt_utc`/`parse_utc` only indirectly through the public functions
  that call them).
- Full suite (must stay green — this is the actual regression gate the
  parent plan requires):
  ```
  cargo test
  ```
  Expected: every existing test in `src/storage.rs` (and every other
  module's tests, since `insert_note`/`insert_punch_with_note` are also
  exercised from `status.rs`'s and `commands.rs`'s own test modules)
  passes, plus all new tests from §5 pass.
- Lint:
  ```
  cargo clippy --all-targets --all-features -- -D warnings
  ```
  Expected: clean, no new warnings introduced by `normalize_newlines`
  or the one-line change to `insert_note`.
- No `StorageError` variant added, so no need to grep for missed
  exhaustive-match arms — confirmed in §1 that both `Display`'s and
  `source()`'s matches stay untouched.
