# Week notes — design spec

> **Archived — historical changeset spec.** Superseded by `docs/dev/SPEC.md`, which folds in the decisions made here.

**Baseline**: written against `main` at `3bcff42` (`v0.4.0` + `orphan-stop-warning`, unreleased).
**Status**: locked 2026-10-24 after one review
(`docs/dev/plans/reports/week-notes-spec-review.md`, folded in NOTES.md
entry 126).
**Input**: brainstorm with the user. Decisions: NOTES.md entries 125 and 126.

## 1. Summary

`mlm week` shows one row per date and no notes. Reading a week's work
log means running `mlm status` seven times.

After this changeset, `mlm week --notes` (short `-n`) prints each
date's notes under that date's row. Rows without notes stay one line.
Plain `mlm week` is unchanged.

## 1.1 Non-goals

- No change to plain `mlm week` output, `status`, or any write command.
- No wrapping or truncation of long note bodies. `status` prints them
  as stored, and so does this.
- No `--notes` on `mlm week target`.
- Exporting notes is the next changeset (`export-notes`, NOTES.md
  entry 125), not this one.

## 1.2 Known issues

None.

## 2. Prerequisites

None.

## 3. Surface

`mlm week [WEEK_ID] [--notes|-n]`

- The flag works with or without `WEEK_ID`, in either order
  (`mlm week -n 43`, `mlm week 43 -n`).
- The flag belongs to the view only. `week target` gains nothing.
  After `target`, `--notes`/`-n` is a token in target's own argument
  list (`allow_hyphen_values`, `cli.rs`), so `mlm week target 40h
  --notes` fails as an invalid `WEEK_ID`/`DURATION` with exit `1`, and
  `mlm week target 43 40h --notes` is a clap usage error with exit `2`.
  Neither writes a target.
- `mlm week --help` lists the flag as "Show each date's notes under
  its row".

## 4. Semantics

- A date shows the same notes as `status DATE`, in the same insertion
  order (`created_at_utc ASC, id ASC`). The layout differs. `status`
  prints a `Notes:` header and two-space bullets, and `week` prints six
  -space bullets under the row with no header.
- A note belongs to its stored `date`. Stint pairing and the
  cross-midnight splice never move a note.
- Notes never change a total, a marker, a footer, the headline or the
  Carry-in block.

## 5. Output layout

Each note prints on its own line directly under its date's row, before
the next row, as six spaces, `- `, then the body. Row markers (`[!]`,
`[i]`) stay at the end of the row, so a marked row reads marker first,
notes below. Blank lines, footers, the carry-in marker line and the
Carry-in block are unchanged.

Illustrative current week, today Sat 2026-10-24, one marked row,
`mlm week --notes`:

```
Week 2026-43 (2026-10-19 - 2026-10-25)

05h 16m over 40h 00m target

  Mon 2026-10-19   08h 27m
      - fixed migration runner bug
      - started punch pairing tests
  Tue 2026-10-20   07h 12m
  Wed 2026-10-21   07h 24m  [!]
      - code review for #41
  Thu 2026-10-22   08h 52m
  Fri 2026-10-23   08h 40m
  Sat 2026-10-24   08h 13m
  Sun 2026-10-25   00h 00m

[!] needs a look: run mlm status DATE for details and fixes

Carry-in:      -03h 32m
Worked:        48h 48m
Fulfillment:   45h 16m
Target:        40h 00m
```

- A note-only date keeps its `00h 00m` row, with its notes under it.
- A week with no notes prints exactly what plain `mlm week` prints.
- Note bodies pass through unchanged, non-ASCII included, as in
  `status` (SPEC §7).

## 6. Errors

- With `--notes`, `mlm week` makes one ranged note read over the
  week's seven dates. It is a new storage function returning notes for
  a date range in insertion order per date. `export-notes` reuses it. A failed read is a hard error, as every other
  `week` read is (SPEC §7.2): `error: ...` on stderr, empty stdout,
  exit `1`.
- Without `--notes`, `mlm week` makes no new read.

## 7. Testing

- notes under their rows, insertion order, two notes on one date
- marked row (`[!]`, `[i]`, both) keeps its marker, notes below
- note-only date shows `00h 00m` row plus notes
- week with no notes → byte-identical to plain `mlm week`
- plain `mlm week` with notes stored → no note lines
- `(incl. ongoing)` row followed by notes
- carry-in marker line and `[i]` footer unchanged with `--notes`
- `-n` and `--notes`, before and after `WEEK_ID`
- `mlm week target 40h --notes` exits nonzero and writes no target
- notes from adjacent weeks never appear
- `notes.body` column renamed: `--notes` → hard error, exit `1`;
  plain `mlm week` → succeeds (no new read without the flag)
- non-ASCII body passes through
- `mlm week --help` lists the flag

## 8. User-visible surface

- `README.md`: `mlm week` section gains the flag and a short example.
- `mlm week --help`: the new flag line, and a glossary entry for the
  indented note lines.

## Interaction with the project spec

- §3.6 gains the flag.
- §7.2 gains the notes layout and the example in §5 here.
- §6.1 and E20 list `week`'s reads and gain the `--notes` read.
- §8.1 gains a happy-path flow for `week --notes`.
- Internal docs naming `week`'s reads go stale: the `src/history.rs`
  module doc and AGENTS.md's layout list.
