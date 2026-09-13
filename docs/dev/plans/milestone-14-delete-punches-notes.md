# Milestone 14 — Delete punches/notes (implementation plan)

**Goal:** Add `mlm delete note|punch [ID] [--date DATE]` — list mode
(no `ID`, numbers that date's entries `1..N`) and delete mode (`ID`
given, deletes by ephemeral position and echoes a ready-to-run recreate
command) — with no edit command and no confirmation prompt.

**Architecture:** Two new `storage.rs` functions do the actual row
removal, returning the deleted row. `cli.rs` gains a `Delete` command
with a `note|punch` subcommand, mirroring the existing `week` command's
shape. `commands.rs` owns the interesting logic: re-fetching and
re-numbering that date's rows on every invocation (never caching a
previous listing), validating the ephemeral `ID` against that fresh
count, and — on success — building the quoted, `--date`-prefixed
recreate line, using the same per-instant local-time conversion pattern
`status` already uses so a DST boundary can't skew the echoed time.
`main.rs` gains one new dispatch arm.

**Tech Stack:** Rust, `clap`, `rusqlite`, `chrono`.

**Spec:** `docs/dev/specs/2026-09-13-delete-punches-notes.md` §3–§9
(source of truth — read in full before starting). §2's prerequisite is
**Milestone 13**, which must already be merged — this plan assumes "a
note body written from this point forward can never contain `\n`"
already holds for newly-written notes (§5 of the spec adds a defensive
fallback for pre-existing rows that predate Milestone 13 and can still
carry a literal `\n` — see Task 3). Also skim `docs/dev/SPEC.md` §7.4
(the "only `status`/`week` produce stdout output" claim this milestone
makes false), `docs/dev/NOTES.md`'s item 35 (the matching "no charset
restriction" claim Milestone 13 already qualified, which Task 5 now
also cross-checks), and the backdated-punches plan's ordering-footgun
note (`docs/dev/plans/milestone-12-backdated-punches.md`) — the
recreate-echo for a note must respect that same footgun in the
*printed* string, even though it doesn't affect parsing `delete`'s own
args.

## Global Constraints

- Depends on Milestone 13 having landed first — don't start this
  milestone's branch from a point before Milestone 13's commit. Before
  branching, **verify this concretely rather than assuming it** (see
  Worktree & Review Protocol below for the exact check).
- CLI surface: `mlm delete note [ID] [--date DATE]`, `mlm delete punch
  [ID] [--date DATE]`. Aliases: `delete` → `del`, `note` → `n`, `punch`
  → `p`.
- No ranges, no bulk delete, no `--all`, no confirmation prompt, no
  edit command — all explicitly out of scope.
- `--date` uses the same future-checked resolver `start`/`stop`/`note`
  already use in `commands.rs`, `date::resolve_future_checked_date` —
  not the permissive `resolve_date` (that one is used in `status.rs`,
  not `commands.rs`). A malformed or future `--date` is therefore a
  hard error on `delete` too, exactly like those three commands —
  nothing listed or deleted, nonzero exit. (An earlier draft of the
  spec picked the permissive resolver on the theory that a future date
  "just has nothing to delete" anyway; that reasoning is superseded —
  reusing the resolver already established in this exact file beats
  introducing a second one for no real behavioral payoff, since nothing
  can insert data into the future in the first place.)
- `ID` is a 1-based, unsigned-integer-shaped positional. A negative,
  non-numeric, or too-large value is rejected by clap itself, before
  application code runs. `0` and anything past the current count are
  application-level hard errors, nothing deleted. Both paths are real
  and both need their own coverage — they are not the same code path.
- `ID` is always resolved against a freshly re-run listing query for
  the resolved date on every invocation — no caching of a previous
  listing between separate `delete` invocations.
- The backdated-punches ordering footgun (date flag must precede free
  note text) does not apply to parsing `delete`'s own arguments (there's
  no free-text positional on this command to fight with) — but it does
  still apply to the *printed* recreate string for a note, since that
  string is fed back into `note`, which does have the footgun.
- New storage functions to delete a punch/note by internal id, each
  returning the deleted row. A delete affecting zero rows is a new,
  dedicated error case rather than a panic or silent success — existing
  error-enum conventions in this codebase require a new variant to come
  with matching arms wherever that enum is matched exhaustively.
- Recreate-echo (a deliberate, narrow exception to the existing
  "write commands print nothing" convention):
  - Includes `--date` only when the deleted entry's date isn't today.
  - For a note, `--date` must precede the body text in the printed
    string (the footgun above).
  - The note body is wrapped as a single, safely-quoted shell token —
    never printed as bare whitespace-separated words (a body word that
    matches a registered global flag, or contains a literal `--`, would
    otherwise be silently misparsed on replay). Single-quoted, not
    double-quoted (double quotes still let a shell interpolate
    `$(...)`/backticks inside them).
  - A punch's time is derived per-row from its own stored instant
    converted to local time — never from "now"'s offset reused, which
    would echo the wrong time for a backdated punch across a DST
    boundary.
- Exit `0` on success (delete) and on an empty list (not an error).
  Nonzero exit, nothing written, on every hard-error path.
- Deleting a date's only/earliest data can shift which date the
  week/carry accounting anchors from, retroactively changing later
  weeks' figures on the next `status`/`week` view — an accepted
  consequence (mirrors the backdated-punches spec's own accepted
  retroactive-figures behavior), not guarded against, but must have its
  own explicit test.
- Landing this feature makes base SPEC.md §7.4's current stdout claim
  false — update it as part of this milestone, not a follow-up.

## Parallelization

```
Batch A (fully independent, start immediately):
  Task 1 (src/storage.rs: delete_punch/delete_note + new error case)
  Task 2 (src/cli.rs: Delete CLI surface)

Batch B (needs Task 1 + Task 2; single task — both land in one file):
  Task 3 (src/commands.rs: quoting helper + delete_note/delete_punch handlers)

Batch C (needs Task 3; two independent files):
  Task 4 (src/main.rs: dispatch wiring)
  Task 5 (docs/dev/SPEC.md §7.4 + docs/dev/NOTES.md + README update)

Batch D (needs Task 4 — the binary must run mlm delete end to end):
  Task 6 (e2e smoke test)
```

Tasks 1 and 2 touch disjoint files with no shared interface — safe to
hand to two parallel workers at once. Task 3 is the integration point
for both and must wait for both; it stays a single task since every
sub-piece (quoting, list mode, delete mode, recreate-line assembly)
lives in the same file and splitting it further would just create
merge conflicts between parallel workers. Tasks 4 and 5 only *consume*
Task 3's finished handlers and touch disjoint files — safe to
parallelize once Task 3 lands. Task 6 needs the full binary wired
(Task 4) to exercise a real `mlm delete` invocation end to end.

## Worktree & Review Protocol (applies to every task below)

- Before starting a task, open an isolated workspace (a native worktree
  tool if available, otherwise a plain `git worktree` per
  `superpowers:using-git-worktrees`). Branch from the tip that already
  has Milestone 13 merged.
- **Before branching, concretely verify Milestone 13 has actually
  landed on the branch point — don't just assume `main`/the parent
  branch already has it.** E.g.: grep `src/storage.rs`'s note-insert
  path for the `\r`/`\n`-collapsing normalization helper Milestone 13
  added (the note-body normalization spec, §2), and/or confirm
  Milestone 13's own tests (the `"line one\nline two"` →
  `"line one line two"` case, §2 of the spec) exist in the tree and
  pass at that commit. If either check comes back negative, stop and
  land/rebase onto Milestone 13 first — this milestone's Task 3
  defensive fallback (legacy pre-normalization data, see Task 3) is
  the only place this plan tolerates pre-existing `\n` in a note body;
  everywhere else assumes it structurally can't happen for anything
  written after that point.
- Every task is test-first: its acceptance-criteria cases are written
  as failing tests before implementation, then implementation follows
  until they pass.
- After a task's own tests, the full suite, and clippy are all green,
  run `superpowers:finishing-a-development-branch` — verify once more,
  then its merge/PR/keep menu. Never merge silently. A task counts as
  landed (unblocking dependents) only once its branch is actually
  merged, not merely once its own tests pass locally. "Clippy green"
  means both of this project's two clippy gates, run separately (see
  Task 3's verification step for the exact commands) — CI's plain
  `cargo clippy --all-targets -- -D warnings` and the pre-commit hook's
  additional cognitive-complexity pass; passing one doesn't imply the
  other.
- Batch B's task cannot start until both of Batch A's tasks are merged.
  Batch C's two tasks can start in parallel as soon as Task 3 merges.
  Task 6 waits for both of Batch C's tasks.

---

### Task 1: `storage.rs` — delete-by-id for punches and notes

**Files:** `src/storage.rs` only (the error enum and its `Display`/
`source` implementations, plus two new functions and their tests).

**Interfaces:**
- Consumes: existing punch/note row types and error enum, already in
  this file.
- Produces: a delete-by-internal-id function for punches and one for
  notes, each returning the deleted row on success and the new
  not-found error case when the id doesn't exist. Consumed directly by
  Task 3.

**Acceptance criteria** (tests written first, covering each):
- Deleting an existing punch removes it and returns the correct row;
  same for a note.
- Deleting a nonexistent id returns the new not-found error rather than
  panicking, for both punches and notes.
- Deleting one row never touches other rows for the same date or a
  different date.
- The new error case's display text names the id it failed on, and has
  no underlying source error.
- Full existing `storage.rs` test suite still passes; clippy clean.

---

### Task 2: `cli.rs` — `delete` CLI surface

**Files:** `src/cli.rs` only (the top-level command enum, new
subcommand types, and their tests).

**Interfaces:**
- Consumes: nothing from Task 1 — this is clap-level plumbing only.
- Produces: the `delete` command with its `note`/`punch` subcommands
  and their shared `id`/`--date` arguments, plus the `del`/`n`/`p`
  aliases. Consumed directly by Task 3 and Task 4.

**Acceptance criteria** (tests written first, covering each):
- `mlm delete note` and `mlm delete punch` with no id both parse as
  list mode (id absent).
- `id` and `--date` parse identically regardless of which comes first
  on the command line (confirming the ordering footgun genuinely does
  not apply to this command's own arguments).
- The `del`, `n`, and `p` aliases each parse identically to their full
  names.
- An id of `0` is accepted by clap itself (rejecting it is an
  application-level rule, not a parsing rule) — confirmed as a parsing
  case, not asserted against application behavior here.
- A negative id, a non-numeric id, and an id past the type's maximum
  are all rejected at the parsing level, before reaching application
  code.
- Full existing `cli.rs` test suite still passes; clippy clean.

---

### Task 3: `commands.rs` — list/delete handlers and the recreate-echo

**Depends on:** Task 1, Task 2.

**Files:** `src/commands.rs` only (new handlers, a shared quoting
helper, and their tests).

**Interfaces:**
- Consumes: Task 1's delete-by-id functions and existing
  fetch-by-date functions; the same future-checked date resolver,
  `date::resolve_future_checked_date`, that `start`/`stop`/`note`
  already use in this file (verified: `commands.rs` only imports and
  calls `resolve_future_checked_date` — the permissive `resolve_date`
  lives in, and is only used by, `status.rs`, not here) — no new
  resolver, no new import, this is established precedent in this exact
  file; Task 2's argument type.
- Produces: a list/delete handler for notes and one for punches,
  matching this file's existing handler shape. Consumed directly by
  Task 4.

**Acceptance criteria** (tests written first, covering each):
- A malformed or future `--date` on `delete note`/`delete punch` is a
  hard error via `resolve_future_checked_date`, exactly like
  `start`/`stop`/`note` — nothing listed or deleted, nonzero exit. (Not
  "nothing to delete": a future date is rejected before any listing or
  deletion is attempted, the same shape as the existing
  `start`/`stop`/`note` future-date tests.)
- A note body containing a token that matches a registered global flag,
  and a body containing an embedded quote character, both — once
  quoted by the new helper and fed back through the actual CLI parser —
  come back out as the exact original body. This is the concrete
  regression test for the flag-collision/`--`-swallowing issue found in
  review, not a formatting/appearance check. **The flag-lookalike case
  must use a *leading* flag-lookalike token** (e.g. a body of
  `"--verbose logging bug"`), not one appearing mid-sentence (e.g.
  `"fixed --verbose logging bug"`). This is not a stylistic choice: the
  underlying bug is position-dependent — clap's `trailing_var_arg`
  only re-checks the *first* token of the captured free-text region
  against registered flags; once an ordinary word has already been
  captured, later tokens in the same region are no longer re-checked.
  A mid-sentence flag-lookalike body would pass this test identically
  whether or not the quoting fix is actually implemented, so it proves
  nothing — a future implementer must not "simplify" this back to a
  mid-sentence example.
- A note body containing a literal `\n` (simulating a pre-Milestone-13
  row, inserted directly via storage rather than through the
  now-normalizing `note` command) produces the plain-description
  fallback on delete (`deleted note (<date>): <first line of body>...`)
  rather than a broken or multi-line "command" — the defensive fallback
  the spec's §5 adds for pre-existing data that predates Milestone 13's
  normalization and was never backfilled.
- List mode on an empty date prints the "nothing to delete" message and
  writes nothing to storage.
- List mode never mutates storage even when entries exist, **and** on a
  non-empty date produces correctly numbered `1..N` output whose
  ordering and punch/note formatting matches the underlying storage
  order exactly (not just "no mutation" — the numbering and formatting
  themselves are asserted).
- A basic `--date` omitted-vs-included test for the recreate line,
  independent of the DST-specific case below: deleting a same-day
  entry omits `--date` from the recreate line; deleting an entry for a
  non-today date (a plain absolute date, no DST involved) includes
  `--date` with the resolved absolute date.
- A dedicated test that, for a note's recreate line, `--date` precedes
  the body text in the *printed string specifically* — checked as an
  ordering assertion on the output string, not the quoting round-trip
  test above (a different concern: this is the exact regression the
  backdated-punches ordering footgun exists to guard against, and
  needs its own explicit coverage rather than being assumed covered by
  the quoting test).
- A valid id in delete mode removes exactly the right row (verified via
  a follow-up read), for both notes and punches.
- An id of `0`, and an id past the current count, are both rejected
  with nothing deleted, for a date with existing entries.
- Deleting from one date never touches a different date's rows or
  numbering, including when both dates happen to have the same count.
- A deleted backdated punch, on the far side of a DST boundary from
  today, echoes the correct local time in its recreate line — not one
  shifted by reusing today's offset.
- Deleting the only remaining entry on the currently-earliest tracked
  date is followed by a subsequent status/week view reflecting the new
  earliest date and changed carry/owed figures.
- Full existing `commands.rs` test suite still passes. Both of this
  project's clippy gates pass, run and verified separately: CI's plain
  `cargo clippy --all-targets -- -D warnings`, and the pre-commit
  hook's additional cognitive-complexity pass (`.githooks/pre-commit`),
  which is `cargo clippy --all-targets --quiet --message-format=json --
  -W clippy::cognitive_complexity`, checked against the threshold in
  `clippy.toml` (`cognitive-complexity-threshold`) — keep the new
  handlers small by delegating to helper functions rather than raising
  that threshold.

---

### Task 4: `main.rs` — dispatch wiring

**Depends on:** Task 2, Task 3.

**Files:** `src/main.rs` only (the command dispatch match).

**Interfaces:**
- Consumes: Task 2's command/subcommand types, Task 3's handlers.
- Produces: nothing new for other tasks — final wiring point.

**Acceptance criteria:**
- `mlm delete note`/`mlm delete punch`, with and without an id, and
  with `--date`, all route to the correct handler when run as the real
  compiled binary.
- Full test suite passes; clippy clean.
- A manual smoke check (add a punch, list it, delete it by id, confirm
  it's gone from status) behaves as expected before moving on.

---

### Task 5: `docs/dev/SPEC.md` §7.4 + `docs/dev/NOTES.md` + `README.md`
— documentation

**Depends on:** Task 2 and Task 3. It needs Task 2's finalized CLI
shape (the `delete note`/`delete punch` syntax and `del`/`n`/`p`
aliases README documents come from `cli.rs`, Task 2 — not Task 3) and
Task 3's finalized output strings (the recreate-echo and list-mode
text README and SPEC.md quote verbatim). Task 3 transitively implies
Task 2 is merged (Task 3 depends on both), but the rationale for this
task's own doc content should name both sources accurately rather than
attributing the CLI syntax to Task 3.

**Files:** `docs/dev/SPEC.md`, `docs/dev/NOTES.md`, and `README.md`
only.

**Interfaces:** None — documentation only.

**Acceptance criteria:**
- SPEC.md §7.4's current claim that only `status`/`week` produce stdout
  output is corrected to also name `delete`'s two stdout cases, as a
  narrow, deliberate exception, cross-referencing the delete spec.
- README's command reference includes `delete note`/`delete punch`
  (with their aliases) at the same level of detail as the existing
  commands, with a short usage example.
- README explicitly documents the stale-id caveat: `ID` is resolved
  against a freshly re-run listing on every invocation, so a listing
  taken earlier in the same sitting can go stale if entries are
  added/removed in between — worst case is deleting the wrong row at
  that position, never a nonexistent one. The spec (§5, §8) calls this
  out as "a documentation/onboarding note... belongs in README", not
  something coded around — this is a required README addition, not
  optional.
- `docs/dev/NOTES.md`'s item 35 ("no length cap, no charset
  restriction — §7's plain-ASCII commitment is about rendered layout
  characters, not what a user can type into a note body") carries the
  same embedded-`\r`/`\n`-normalization carve-out SPEC.md §2.3 gets
  from Milestone 13, per the spec's own §8 Docs bullet, which names
  both docs explicitly. If Milestone 13's own doc task already updated
  NOTES.md, this is a no-op confirmation; if it didn't, this task is
  the backstop that catches it before the decision log goes stale.
- The exact strings quoted in all three docs match the actual command
  output byte-for-byte, checked against the merged Task 3 branch (not
  this plan's wording, in case wording changed during that task's own
  review).

---

### Task 6: End-to-end smoke test

**Depends on:** Task 4.

**Files:** `.github/workflows/ci.yml` only — **extend the existing
e2e smoke-test step** (`e2e smoke test (seed + status + week)`, the
`set -euo pipefail` shell block that already seeds a real db via
`cargo run --example seed_test_data` and drives the compiled
`$mlm_bin` directly, including its existing backdated-punches section
against a separate `$backdate_db`). Do **not** add a new Rust
integration test file (e.g. a `tests/status_cli.rs`-style file) —
that's a different, also-real convention in this repo, but the spec
(§8) explicitly says "existing script", and this repo already has one
purpose-built for exactly this kind of full-binary, cross-platform
smoke coverage; a second, parallel mechanism would duplicate it for no
reason.

**Interfaces:** Consumes the compiled binary as a black box (built
earlier in the same CI job, `target/${{ matrix.target }}/debug/mlm`);
produces nothing consumed by other tasks — the final leaf in the
dependency graph.

**Acceptance criteria** — add a new section to the e2e step's shell
script, in the same style as its existing sections (its own
`$RUNNER_TEMP`-scoped db, `echo "--- ... ---"` section banners,
`tee`+`grep -q`/`grep -qE` assertions against captured `.out`/`.err`
files, and the same `if … ; then echo … exit 1; fi` pattern the
existing future-date-rejected check uses):
- A new, separate db (`delete_db="${RUNNER_TEMP}/mlm-ci-smoke-delete.db"`,
  mirroring how the existing backdate section uses its own
  `backdate_db`) seeded with a punch and a note via `start`/`stop`/
  `note`, then: bare `delete note`/`delete punch` (list mode) shows the
  seeded entries numbered; delete by listed id; confirm the deleted
  entry is gone from `status`; the printed recreate line, run for real
  as an actual shell command against the same db, restores the
  original entry (confirmed via a follow-up `status`).
- An empty date's list mode (a date with no punches/notes) prints
  "nothing to delete" and exits `0`.
- An out-of-range id (e.g. `2` against a date with only one entry) is
  rejected: nonzero exit, and the target entry still present in a
  follow-up `status` afterward — same `if ... ; then ... exit 1; fi`
  pattern the script's existing future-date check uses.
- A note seeded with a **leading** flag-lookalike word (e.g.
  `"--verbose logging bug"`, matching Task 3's leading-position
  requirement), once deleted, produces a recreate line that — run for
  real through the actual shell (not just string-inspected) — restores
  the exact original body, including the flag-lookalike word, verified
  via a follow-up `status` grep.
- A future `--date` on `delete note`/`delete punch` is rejected with a
  nonzero exit, mirroring the script's existing
  `start -d 2099-01-01` future-date check.
- Full test suite passes; clippy clean (both gates, per Task 3's
  verification step). This is the milestone's overall completion
  signal.
