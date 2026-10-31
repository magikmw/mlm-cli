# Final review: week-notes

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

Tip `a1c315b` (clone HEAD matched). Baseline `3bcff42`.

## Verification

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo build` | ok |
| `cargo test` | all pass (lib 781, every integration binary green) |
| `cargo clippy --all-targets -- -D warnings` | clean |

## Hand runs (fresh DB, today Sat 2026-10-24)

- `week --notes` prints three notes under Sat in insertion order, six-space `- ` prefix. Note bodies holding `$(rm -rf x)`, backticks, quotes, `;` and non-ASCII print verbatim. Nothing is interpreted, and no output is replayable shell.
- Plain `week` prints no note lines. `-n` and `--notes` work before and after `WEEK_ID`, and bare.
- `week target 40h --notes` gives `error: invalid WEEK_ID "40h"`, exit 1. `week target 43 40h --notes` is a clap usage error, exit 2. Both match spec §3.
- `week --help` lists `-n, --notes` with the description on the next line, and the glossary has the `- <note>` entry.
- With `notes.body` renamed, `week -n` prints `error: database error: no such column: body ...` on stderr, empty stdout, exit 1. Plain `week` exits 0.
- Mutations: reversing the note order within a date fails `notes_in_range_orders_by_date_then_created_at_then_id`. Shrinking the read range to one day fails `run_with_notes_attaches_through_storage` and two other week_view tests. The assertions are real.

## Spec compliance

- SPEC §3.6, §6.1, §7.2, §8.1 (F14), E20, README `mlm week` section, AGENTS.md and the `history.rs` module doc all match the spec's section 8 and "Interaction" list.
- Both worked-example outputs (README, SPEC §7.2) are byte-identical to spec §5.
- `notes_in_range` is a single ranged query. The storage layer binds parameters, so there is no injection surface.

## Findings

1. **Low, docs: spec text vs help layout.** Spec §3 says `--help` lists the flag as "Show each date's notes under its row". The long help puts that text on its own line, and `-h` keeps it on the flag line. This is clap's standard layout. The task 1 report already records it and the test asserts the real layout. Suggested fix: none needed. Optionally mention it in SPEC §3.6.
2. **Low, security note (pre-existing behavior, not a regression).** Note bodies with raw control characters, such as ESC sequences, reach the terminal unfiltered. `status` does the same, and the spec says to pass bodies through "as stored". `week --notes` widens the exposure slightly, since it prints a week of notes at once. Suggested fix: a follow-up decision on sanitizing control characters for both commands, not for this changeset.
3. **Nit, docs.** The `cli.rs` doc comment for `WeekArgs` runs past 80 columns. It is harmless and rustfmt leaves comments alone.

Test strength is good. Byte-exact goldens, strip-note-lines equivalence against the plain render, a 16-mask footer matrix, a renamed-column hard-error test at the storage, view and CLI layers, and an adjacent-week exclusion test. The only uncovered new line is the `?` after `query_map`, which is the same gap `punches_in_range` has.

## Verdict

ship-with-followups. Nothing blocks the merge. Findings 1 and 2 are optional follow-ups.
