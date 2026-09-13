# Milestone 13 — Note-body newline normalization (implementation plan)

**Goal:** Extend the existing note-body trim-before-storage step so
that any embedded `\r`/`\n` in a note body is collapsed to a single
space, everywhere a note is written (`note` command and the inline
note on `start`/`stop`, both of which already funnel through the same
storage insert path).

**This is a forward-only guarantee, not unconditional.** There is no
backfill/migration (spec §2's explicit caveat) — this is a pure
insert-path change. Once this ships, "a note body can never contain
`\n`" holds for anything *written from that point forward*; any note
row already stored before this lands can still contain a literal `\n`
and stays that way until it's deleted/recreated or otherwise
rewritten. Milestone 14 owns its own defensive fallback for that
pre-existing-data case (its recreate-echo checks for `\n` at echo time
and falls back to a plain description) — this plan does not need to
add a migration, and must not describe its outcome as an absolute
guarantee over the whole `notes` table.

**Architecture:** One new private helper in `src/storage.rs`, called
from the existing note-insert function (`insert_note`), which changes
what it stores in place of its current plain `body.trim()`. No new
public API, no new error variant, no schema change — a pure
text-normalization change to an existing write path. The inline-note
path on `start`/`stop` needs no separate change of its own: it already
delegates note storage to the same insert function.

**Precisely which existing trim/emptiness checks this task touches vs.
must not touch** (see the real current code, `src/storage.rs`):
`insert_note` itself has *two* separate sites, not one — an
emptiness check (`if body.trim().is_empty() { return
Err(StorageError::EmptyNote); }`) and, immediately after it, the
stored-value trim (`let stored = body.trim();`). This task changes
only the stored-value trim (`stored` gains the new newline-collapsing
step, composed with the existing trim). The emptiness check must not
change at all — it keeps firing on the untouched `body`, exactly as
today. Separately, and in a different function entirely,
`insert_punch_with_note` (the inline-note-on-punch path) has its own
independent whitespace-emptiness check (`body.trim().is_empty()`) run
*before* it calls `insert_note` — this third site is also
out-of-scope and must not change; it's what makes the "inline note is
normalized the same way" acceptance criterion below true without any
edit to `insert_punch_with_note` itself, since it simply delegates
storage to the now-updated `insert_note`.

**Tech Stack:** Rust, std only — no new dependency for a one-function
fix.

**Spec:** `docs/dev/specs/2026-09-13-delete-punches-notes.md` §2
(source of truth — read in full before starting). Also update
`docs/dev/SPEC.md` §2.3 and `docs/dev/NOTES.md`'s matching decision
per that section's explicit instruction (§2 calls out both docs by
name — SPEC.md going stale is not the only risk, NOTES.md's decision
log makes the same unqualified claim independently).

**Ships as its own commit**, independent of Milestone 14 (the `delete`
feature) — it touches none of Milestone 14's files. Land it first;
Milestone 14 depends on its invariant ("a note body can never contain
`\n`") already holding.

## Global Constraints

- Silent normalize, not reject: an embedded `\r`/`\n` is replaced,
  never a hard error.
- Collapse a *maximal run* of `\r`/`\n` to exactly one space, not one
  space per character — a `\r\n` pair must not become two spaces.
- **Tie-break rule for a run interrupted by plain whitespace** (spec
  §2, matched exactly): only a maximal run of `\r`/`\n` characters
  collapses to one space; ordinary whitespace (a plain space or tab)
  sitting between two such runs is left exactly as typed, never merged
  into either run's replacement space. E.g. `"a\n\n \nb"` (a two-`\n`
  run, then one literal space, then a one-`\n` run) becomes `"a   b"`
  — three spaces: one from the first run, the untouched literal space,
  one from the second run — not `"a b"`.
- **Unicode line separators (U+2028/U+2029) are out of scope.** This
  normalization targets ASCII `\r`/`\n` only, consistent with this
  project's existing plain-ASCII conventions elsewhere (SPEC §7). An
  embedded U+2028/U+2029 passes through completely untouched — a
  deliberate boundary, not a gap to close later.
- No new dependency (no `regex` crate) for this.
- Only `\r`/`\n` are targeted — regular internal whitespace (spaces,
  tabs) is untouched.
- Leading/trailing whitespace handling (including leading/trailing
  `\r`/`\n`) is unchanged — the existing trim step still owns that;
  this only touches *embedded* runs.
- The existing empty/whitespace-only rejection must keep firing exactly
  as before — a body that's only newlines/spaces is still empty after
  trim, checked before normalization touches anything.
- **Applies to newly-written notes only — no backfill.** See the
  Goal/Architecture caveat above; this constraint list is scoped to
  the insert path, not to rows already in the table.
- Base SPEC.md §2.3's "no length cap or charset restriction" line on
  `notes.body` needs a carve-out noting embedded `\r`/`\n` are
  normalized away, not preserved. `docs/dev/NOTES.md`'s decision log
  (decision 35, "Note content policy": "no length cap, no charset
  restriction") makes the same unqualified claim and needs the
  identical carve-out — updating only SPEC.md leaves it stale.

## Parallelization

```
Batch A (fully independent, start immediately):
  Task 1 (src/storage.rs — normalization)
  Task 2 (docs/dev/SPEC.md §2.3 + docs/dev/NOTES.md wording)
```

Disjoint files, no shared interface — safe to hand to two parallel
workers. Task 2 is docs-only but still gets its own worktree and review
pass per this project's isolation convention.

## Worktree & Review Protocol (applies to every task below)

- Before starting a task, open an isolated workspace (a native worktree
  tool if available, otherwise a plain `git worktree` per
  `superpowers:using-git-worktrees`). Never work directly on the branch
  this plan was dispatched from.
- A task is test-first: its acceptance-criteria cases are written as
  failing tests before any implementation change, then implementation
  follows until they pass — no task skips straight to implementation.
- After a task's own tests, the full suite, and clippy are all green,
  run `superpowers:finishing-a-development-branch` — verify once more,
  then go through its merge/PR/keep menu. Never merge silently. A task
  only counts as landed once its branch is actually merged back, not
  merely once its own tests pass locally.
- Task 1 and Task 2 can merge in either order — no conflict between
  them.

---

### Task 1: `storage.rs` — normalize embedded newlines in note bodies

**Files:** `src/storage.rs` only (the `insert_note` function's
stored-value trim, per the Architecture section's precise scoping
above). `storage.rs` does not have a separate test module for notes —
the whole file shares one `#[cfg(test)] mod tests`, and note tests are
just a comment-delimited section within it (e.g. the existing `// ---
7.3 Note trim-and-store ---` block); new tests for this task are added
to that same shared module, in that section, not a new module.

**Interfaces:** No signature change to the existing note-insert
function — every current caller keeps working unmodified. Nothing new
exposed to other tasks; Milestone 14 depends on the *behavior*
(bodies never contain `\n`), not on any new function from this task.

**Acceptance criteria** (tests written first, covering each):
- A body with a single embedded `\n` stores with that newline replaced
  by one space.
- A body with an embedded `\r\n` pair stores with exactly one space,
  not two.
- A body with a lone embedded `\r` (no accompanying `\n`) stores with
  exactly one space in its place.
- A body with the reversed pairing, an embedded `\n\r`, stores with
  exactly one space, same as `\r\n`.
- A body with a run of several consecutive newlines/carriage returns
  stores with exactly one space for the whole run.
- **A run interrupted by plain whitespace** — a body shaped like
  `"a\n\n \nb"` (newline-run, one literal space, newline) — stores as
  `"a   b"` (three spaces: one per run, plus the untouched literal
  space), not `"a b"`. This is the Global Constraints tie-break rule,
  matching spec §2 exactly, and needs its own explicit test rather
  than being inferred from the plain-run cases above.
- **Unicode line separators (U+2028, U+2029) are explicitly out of
  scope and must NOT be touched**: a body containing an embedded
  U+2028 or U+2029 stores with that character preserved byte-for-byte,
  not collapsed to a space and not otherwise altered — this is a
  positive assertion that they pass through untouched, not merely the
  absence of a test.
- Leading/trailing newlines are still trimmed away entirely (no
  space introduced at the edges) — the pre-existing trim step's
  behavior is unchanged.
- Regular internal whitespace (multiple spaces, tabs) is left exactly
  as typed — only `\r`/`\n` are targeted.
- A body that's only whitespace/newlines is still rejected as empty,
  exactly as today, nothing written.
- An inline note on `start`/`stop` (not just the standalone `note`
  command) is normalized the same way — confirms the fix isn't
  accidentally local to one call site.
- Full existing `storage.rs` test suite still passes (no regressions,
  in particular the pre-existing empty-note and full-text-preservation
  tests).
- Clippy clean.

---

### Task 2: `docs/dev/SPEC.md` §2.3 and `docs/dev/NOTES.md` — document the newline carve-out

**Files:** `docs/dev/SPEC.md` (the `notes.body` row in the §2.3 table)
**and** `docs/dev/NOTES.md` (decision 35, "Note content policy," whose
current text reads: "no length cap, no charset restriction — §7's
plain-ASCII commitment is about rendered layout characters, not what a
user can type into a note body."). Spec §2 calls out both documents by
name for this carve-out, not just SPEC.md — NOTES.md's decision log
makes the same unqualified "no charset restriction" claim
independently and goes stale the moment this lands if only SPEC.md is
touched.

**Interfaces:** None — documentation only, no code dependency on
Task 1, nothing produced for other tasks.

**Acceptance criteria:**
- The `notes.body` row's description in SPEC.md §2.3 reflects that
  embedded `\r`/`\n` are silently normalized to a single space, not
  preserved verbatim, while everything else about the "no length
  cap/charset restriction" claim stays accurate.
- NOTES.md's decision 35 gets the same carve-out added to its "no
  charset restriction" text, so the two documents no longer disagree
  once this ships — worded to note that embedded `\r`/`\n` are
  normalized away rather than left as evidence of an unrestricted
  charset.
- Neither update claims or implies this applies to notes already
  stored before this change ships — the carve-out is about the
  charset/content-policy claim going forward, and must not be worded
  as a retroactive guarantee over existing rows (see the
  Goal/Architecture no-backfill caveat).
- The updated wording doesn't contradict the existing empty/whitespace
  rejection rule described elsewhere in SPEC.md.
