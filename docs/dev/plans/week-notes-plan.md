# Plan: week-notes

Spec: `docs/dev/specs/2026-10-24-week-notes.md` (locked). Reviews: `docs/dev/plans/reports/week-notes-spec-review.md` (folded, NOTES.md entry 126), `docs/dev/plans/reports/week-notes-plan-review.md` (folded, NOTES.md entry 127). Decision log: NOTES.md entries 125, 126 and the "Triaging review findings" process section.

**Altitude**: scope of work only. Task plans choose identifiers. The one new file name below is pinned for ownership.

**Read for this plan**: the spec in full; NOTES.md entries 125-126 and "Triaging review findings"; `src/cli.rs` (the week args and the target subcommand's argument list); `src/week_view.rs` (view build, render, `run`); `src/history.rs` (module doc, loader); `src/storage.rs` (per-date note read, earliest-date read); `tests/week_read_errors.rs`; AGENTS.md's layout list; SPEC.md §3.6, §6.1 and E20.

## Goal

`mlm week` gains an opt-in `--notes`/`-n` flag (spec §3). With it, each date's notes print under that date's row, one note per line, as six spaces, `- ` and the body, in the same insertion order `status DATE` uses (spec §4, §5). Rows without notes stay one line, and a week with no notes prints exactly what plain `mlm week` prints. Notes never change a total, a marker, a footer, the headline or the Carry-in block. Plain `mlm week`, `week target`, `status` and every write command stay as they are (spec §1.1). With the flag, `week` makes one new ranged note read over the shown week's seven dates, and a failed read is a hard error (spec §6).

## Architecture

The week arguments in `src/cli.rs` today hold an optional target subcommand and an optional `WEEK_ID` positional. The flag joins them as a view-level boolean. It is not global, so `week target` gains nothing, and target's own argument list, which accepts hyphen values, keeps swallowing `--notes` as one of its tokens (spec §3).

`src/week_view.rs` builds a view struct from the shared history loader and renders it to a string. The loader reads the earliest data date, a padded punch range and the target overrides. It feeds `status` too, so the note read stays out of it. The week view module makes the note read itself, after the loader's reads, only when the flag is set. The read happens in the command entry point, not in the existing view builder, whose signature stays unchanged. That builder has test callers in `src/status.rs` and `src/lookback.rs`, which Task 1 does not own. It attaches each date's notes to that date's row in the view. The renderer then prints a row's notes directly after the row line, so the row's marker still ends the row. Rows, footers, the carry-in marker line and the Carry-in block come from the same code as before. A view without notes renders byte-identical to today's output.

`src/storage.rs` already has a per-date note read ordered by `created_at_utc ASC, id ASC`. It gains a ranged sibling. That function takes an inclusive date range and returns a flat list of the existing note type, each carrying its own date, ascending by date and in insertion order within each date. `export-notes` will reuse it (spec §6), so its shape is a contract here, not a private detail of `week`.

The flag adds no module, no schema change and no runtime dependency. The `src/history.rs` module doc says the loader feeds `status` and `mlm week`. It gains a sentence saying `week --notes` makes one note read of its own outside the loader (spec "Interaction with the project spec"). The two loader passages in the `src/week_view.rs` docs (module doc, and the doc on the view build) gain the same fact.

## Global constraints

- Verification set: `cargo fmt`, `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- The pre-commit hook also enforces cognitive complexity of 15 or less per function (`clippy.toml`) and a coverage ratchet against `coverage-baseline.json`. The read-failure path and the ranged storage read each need a test, or coverage drops.
- No new runtime dependency and no schema change.
- Without the flag, `mlm week` makes no new read, and its output stays byte-identical for every existing test. Existing assertions keep their meaning. Mechanical updates to struct patterns and fixtures in owned files are allowed, for example the week-args pattern test in `src/cli.rs`.
- A failed note read is an error, never an empty note list (spec §6, SPEC §7.2).
- Note bodies pass through unchanged, non-ASCII included (spec §5). All other output stays ASCII (`tests/ascii_sweep.rs`).

## Ordering and parallelization

The two tasks run in parallel in separate worktrees. They share no file, and the contracts below are all they share. There is nothing to sequence.

A finer split buys nothing. The storage read, the flag and the row layout are each small, and the CLI tests exercise all three together. Splitting them would put several tasks on `src/week_view.rs` or on one test file.

## Interface contracts (pinned before worktrees open)

1. **Flag.** Long `--notes`, short `-n`, on the week view only. It works with or without `WEEK_ID`, in either order.
2. **Help line.** `mlm week --help` lists the flag as `Show each date's notes under its row` (spec §3). The help's output glossary, which explains row markers, gains one entry for the indented `- ` note lines, in the glossary's existing format.
3. **Note line.** Six spaces, `- `, then the body as stored. One line per note, directly under its date's row and before the next row. No header line and no blank line around a row's notes (spec §5).
4. **Order.** Within a date, `created_at_utc ASC, id ASC`, the same order `status DATE` prints. A note belongs to its stored `date`, so stint pairing and the cross-midnight splice never move it (spec §4).
5. **Ranged note read.** One inclusive date range in, a flat list of the existing note type out (each note carries its date), ascending by date, insertion order within a date. With `--notes`, `week` calls it once over the shown week's Monday-Sunday. Without the flag it is never called.
6. **Doc example.** SPEC §7.2's and README's example is spec §5's block verbatim: Week 2026-43, today Sat 2026-10-24, notes on Mon and Wed, `[!]` on Wed.
7. **Errors.** A failed note read prints `error: ...` to stderr, nothing to stdout, and exits `1`, the same as E20's other reads.
8. **README spelling.** README writes the flag `-n/--notes`, matching its `-d/--date` style. SPEC §3.6 writes `[--notes|-n]` as the spec pins.

## Task 1: code and tests

**Spec**: §3 surface, §4 semantics, §5 layout, §6 errors, §7 testing; non-goals §1.1.

The week arguments gain the flag with contract 2's help text. Storage gains the ranged note read of contract 5, with unit tests for range edges, cross-date order and in-date insertion order. The week view reads notes only when the flag is set, attaches them to rows and renders contract 3's lines. The `src/history.rs` module doc gains the sentence from the architecture section. The CLI tests go in a new test file, driven through the binary with `MLM_DB_PATH` and `TZ=UTC`, seeding raw rows the way `tests/week_read_errors.rs` does, and naming a past `WEEK_ID` wherever the case does not need today.

Acceptance criteria. Each CLI case asserts exact stdout unless it says otherwise. Cases whose output depends on the real clock or weekday (`[i]`, `[!] [i]`, `(incl. ongoing)`, the Monday `[i]` line) are unit tests over the renderer with a fixed view, not CLI cases.
- Two notes on one date, where the later `id` has the earlier `created_at_utc`, print in `created_at_utc` order under that row, so an `id`-only sort fails the test. A note on another date prints under its own row.
- A `[!]` row, an `[i]` row and a row with both keep the marker at the row's end, with notes on the following lines.
- A note-only date shows its `00h 00m` row with its notes under it.
- A week with no notes prints byte-identical output with and without `--notes`.
- Plain `mlm week` with notes stored prints no note lines.
- An `(incl. ongoing)` row is followed by its notes.
- The carry-in marker line and the `[i]` footer are unchanged with `--notes`.
- `-n` and `--notes` both work, before and after `WEEK_ID`.
- `mlm week target 40h --notes` exits nonzero, and no target row is written.
- Notes dated the Sunday before and the Monday after the shown week never appear.
- With the `notes.body` column renamed, `mlm week --notes` prints `error: ...`, empty stdout and exits `1`. Plain `mlm week` on the same database succeeds. Renaming `body`, not dropping the table, keeps the earliest-date read (which reads `notes.date`) working.
- A non-ASCII body passes through unchanged, and all other `week --notes` output is ASCII.
- `mlm week --help` contains contract 2's line and the glossary entry.
- Existing week, status, target and ASCII sweep tests pass, with only the mechanical updates the global constraints allow.

**Out of scope**: wrapping or truncating bodies. Any change to `status`, `week target`, write commands, pairing or the splice. Moving notes into the history loader. Export. Any `.md` file.

**Owns exclusively**: `src/cli.rs`, `src/storage.rs`, `src/week_view.rs`, `src/history.rs` (module doc only), the new `tests/week_notes_cli.rs`, and `tests/week_target_cli.rs` (the `mlm week -h` golden only, since the new flag line changes it). It reads `src/render.rs`, `src/status.rs` and `src/main.rs` without editing them. Any change to another file needs a coordinator decision first.

## Task 2: docs

**Spec**: §3 surface, §4 semantics, §5 layout and example, §6 errors, §8 user-visible surface, "Interaction with the project spec".

This task folds the behavior into the live docs so SPEC.md and README.md stand alone, with no pointer out to the working spec.

Acceptance criteria:
- SPEC §3.6's heading and body gain `[--notes|-n]`, with the either-order rule and the note that `week target` does not take it.
- SPEC §7.2 gains the note layout (contracts 3 and 4), the rules that notes change no figure, marker or footer and that a no-note week matches plain `week`, and spec §5's example verbatim (contract 6).
- SPEC §6.1's read-failure bullet and E20 add the `--notes` note read, and say plain `week` makes no such read.
- SPEC §8.1 gains a happy-path flow for `mlm week --notes`.
- SPEC §3.5's sentence describing the shared `--help` glossary stays true with the new entry, whose text and position Task 1's plan pins.
- README.md's `mlm week` section gains the flag (contract 8 spelling), a Features line, and contract 6's example in full. Its possible-issues list is unchanged.
- AGENTS.md's `src/storage.rs` bullet names the ranged note read, and its `src/history.rs` bullet says `week --notes` makes one note read outside the loader.
- Every quoted note line and help line matches contracts 2 and 3 byte for byte.

**Out of scope**: `--help` text and the `src/history.rs` module doc (Task 1). NOTES.md. Archiving the working spec. Any source or test file.

**Owns exclusively**: `docs/dev/SPEC.md`, `README.md`, `AGENTS.md`. NOTES.md and the spec archive banner stay with the coordinator.

## Risks and ambiguities

- **Who owns the `src/history.rs` doc.** The spec lists it among doc targets, but it is a source file under the fmt and clippy gate. Task 1 owns it, so the docs task touches no `.rs` file. The loader's code does not change.
- **Where the read lives.** The spec pins one ranged read and no read without the flag, but not where it is made. Keeping it out of the shared loader means `status` cannot pick up a cost it does not use. The architecture section pins this.
- **E20 already names the `notes` table.** The earliest-date read already queries `notes.date`, so a dropped `notes` table breaks plain `week` today. That is why spec §7's error test renames `notes.body`, which fails only the new read. Task 1's error case must use the rename.
- **ASCII sweep.** Task 1 leaves `tests/ascii_sweep.rs` alone. The ASCII check for `week --notes` lives in the new test file.
