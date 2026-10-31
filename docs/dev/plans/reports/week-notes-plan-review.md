# week-notes plan review

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

- Plan: `docs/dev/plans/week-notes-plan.md` (untracked)
- Locked spec: `docs/dev/specs/2026-10-24-week-notes.md`
- Source baseline: branch `week-notes` = `3bcff42`
- Prior review: `docs/dev/plans/reports/week-notes-spec-review.md` (its findings are not repeated here)

## Findings

### F1. Task 1 may need two files it does not own (Medium, file ownership / ambiguous contract)

- Claim: plan:72 gives Task 1 `src/cli.rs`, `src/storage.rs`, `src/week_view.rs`, `src/history.rs` and the new test file, and says it reads `src/status.rs` "without editing" it. Plan:17 says "the week view module makes the note read itself" but does not say whether the read goes in `run` or in `build_week_view`.
- Evidence: `build_week_view(conn, week, now)` (src/week_view.rs:234-238) is called from tests in two other files:
  - src/status.rs:3091, 3282, 3296, 3384, 3385
  - src/lookback.rs:1011 (a file the plan does not mention at all)
- If Task 1 adds a `notes: bool` parameter to `build_week_view`, which is the obvious place to attach notes to rows, six call sites outside its ownership stop compiling.
- Fix: pin it. Either `build_week_view` keeps its signature, and the note read and row attachment happen in a separate step called from `run` (src/week_view.rs:277-287). Or Task 1 is granted test-only edits to `src/status.rs` and `src/lookback.rs`.

### F2. The insertion-order acceptance case contradicts the sort key (Medium, internal inconsistency)

- Claim: plan:55 says "Two notes on one date, inserted out of `created_at_utc` order, print in insertion order under that row."
- Evidence: contract 4 (plan:43) and the existing read (src/storage.rs:463-468) sort by `created_at_utc ASC, id ASC`. The project calls that order "insertion order". If two rows are inserted with ids ascending and `created_at_utc` descending, they print in `created_at_utc` order, which is the reverse of row-insertion order.
- The sentence can be read two ways:
  - Expected output follows `created_at_utc`. That matches contract 4.
  - Expected output follows id order. An implementer who reads it this way can write `ORDER BY "date", id` in the new ranged read, and the test will pass while contract 4 is broken.
- Fix: write the case as "two notes whose ids and `created_at_utc` disagree print in `created_at_utc` order", and give the expected lines.

### F3. "Exact stdout" cannot hold for the clock-dependent cases (Medium, testing gap)

- Claim: plan:54 says every CLI case asserts exact stdout unless it says otherwise. The cases at plan:56 (`[i]` row, `[!] [i]` row) and plan:60 (`(incl. ongoing)` row) do not say otherwise.
- Evidence:
  - The binary uses the real clock (src/main.rs dispatches with `now`). No test hook injects time. `grep MLM_NOW|faketime tests/` finds nothing.
  - `(incl. ongoing)` minutes are live minutes (src/week_view.rs:213-215), so they change from one minute to the next.
  - `[i]` marks yesterday's row, but on a Monday it moves to the previous week and becomes the Monday footer (src/week_view.rs:158-165, 265-274). The existing CLI test branches on the weekday for this reason (tests/overnight_visibility_cli.rs:163-183).
  - The current week's headline depends on the real weekday (Mon-Thu "by end of" vs Fri-Sun "target").
- Fix: test the layout of marked and ongoing rows in `src/week_view.rs` unit tests over a hand-built `WeekView`, since `render_week` is pure (src/week_view.rs:84-91). At CLI level, assert only the row line and the note lines after it, which is how `tests/overnight_visibility_cli.rs` already works.

### F4. "Existing tests pass unchanged" cannot hold literally (Low, internal inconsistency)

- Claim: plan:68 says existing week, status, target and ASCII sweep tests pass unchanged. Plan:28 repeats this.
- Evidence: test P6 destructures `WeekArgs` with no `..` (src/cli.rs:470-473). Adding the flag field makes it fail to compile. If the notes go on `WeekView`, the struct literals in the fixtures need the new field too (src/week_view.rs:337, 364, 469, 492). If the notes go on `WeekRow`, the `row()` helper needs it (src/week_view.rs:311-320).
- All of these files are Task 1's, so Task 1 can make the edits. But the criterion as written forbids them.
- Fix: reword it as "existing assertions and goldens unchanged; fixtures may gain the new field".

### F5. `week_view.rs` docs go stale, and the plan lists only `history.rs` (Low, missing doc target)

- Claim: plan:21 and plan:52 name the `src/history.rs` module doc as the one source doc to update.
- Evidence: two passages in `src/week_view.rs` say every read goes through the loader:
  - module doc, src/week_view.rs:17-18: "Accounting comes from `history::load_history` ... A failed storage read is a hard error."
  - `build_week_view` doc, src/week_view.rs:231-233: "Every storage read goes through `history::load_history`."
- The new note read is made outside the loader (plan:17), so the second passage becomes false.
- Fix: add both passages to Task 1's doc edits.

### F6. The ranged read's shape is called a contract but is not pinned (Low, ambiguous contract)

- Claim: plan:19 says the shape "is a contract here, not a private detail of `week`", because `export-notes` reuses it. Contract 5 (plan:44) pins the range and the order but not the return type.
- Evidence: the existing reads return a flat `Vec` (src/storage.rs:398-424 punches, 465-478 notes). A grouped map is just as plausible for attaching notes to rows. The next changeset inherits whichever one Task 1 picks.
- Fix: pin `(conn, from: NaiveDate, to: NaiveDate) -> Result<Vec<Note>, StorageError>`, inclusive, ordered `"date" ASC, created_at_utc ASC, id ASC`. Or say the shape is left open.

### F7. The ASCII constraint has no check, and the sweep risk bullet is moot (Low, testing gap)

- Claim: plan:30 says all non-body output stays ASCII and cites `tests/ascii_sweep.rs`. Plan:100 says the sweep's body allowance "must cover week output too" if it gains `--notes`.
- Evidence:
  - The sweep never runs `week --notes` (tests/ascii_sweep.rs:91-96). Task 1 does not own that file (plan:72), so the six-space-and-dash scaffolding has no ASCII check.
  - The allowance already covers every command's output. It strips the body from the combined text with `replace` (tests/ascii_sweep.rs:106), so the risk the bullet describes cannot happen.
- Fix: add a non-ASCII-body case to `tests/week_notes_cli.rs` that strips the body and asserts the rest is ASCII. That keeps the check inside Task 1's files. Drop or correct the risk bullet.

### F8. README synopsis spelling is not pinned (Low, ambiguous contract)

- Claim: plan:81 pins SPEC §3.6's heading to `[--notes|-n]`. Plan:86 says only that README "gains the flag".
- Evidence: README headings use the `[-d/--date DATE]` style (README.md:134, 183, 215). The current week heading is `mlm week|w [WEEK_ID]` (README.md:409). The spec's form `[--notes|-n]` (spec:45) follows neither, so the two docs can end up spelling the synopsis differently.
- Fix: pin the README heading as `### \`mlm week|w [WEEK_ID] [-n/--notes]\``, or say the docs task picks the README house style.

### U1. One unreadable note row hides the whole week with `--notes` (Low, undeclared user-visible gap)

- Decision: the note read is all-or-nothing and parses every row (plan:29, contract 7; `note_from_row` parses `date` and `created_at_utc`, src/storage.rs:448-460).
- Visible consequence: a single note in the shown week with a malformed `created_at_utc` makes `mlm week --notes` print only `error: ...`, with no totals, headline or rows. Plain `mlm week` on the same database still works, because the earliest-date read only takes `MIN(date)` (src/storage.rs:503-509). The user loses the week's figures because of one note's metadata, which the output never shows.
- Spec §1.2 does not record this. `status DATE` fails the same way for that one date (src/status.rs:470), but `week --notes` widens the failure to seven dates and to every figure on the screen.

### U2. The `week` long-help glossary says nothing about the note lines (Low, undeclared user-visible gap)

- Decision: only the flag's one help line is pinned (contract 2, plan:41).
- Visible consequence: `mlm week --help` appends the glossary (src/cli.rs:280-286). The glossary explains every other piece of week output, including `[!]`, `[i]` and `(incl. ongoing)` (src/cli.rs:9-55). It does not explain the indented `- ` lines, and nothing there says they are the same notes `status` lists. A user who reads the glossary finds no entry for the one new element.
- Not recorded in spec §1.2 or §8.

## Verdict

ship-with-followups

F1 and F3 should be settled before the worktrees open. F1 can stop Task 1 from compiling without touching files it does not own. F3 makes three acceptance cases impossible to write as stated. F2 should be reworded, so the order test cannot pass against the wrong sort key. F4-F8 are wording and doc-target fixes. U1 and U2 are candidates for spec §1.2, and deciding on them is the user's call.
