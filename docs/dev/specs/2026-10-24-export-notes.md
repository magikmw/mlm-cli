# Export notes — design spec

**Baseline**: written against `main` at `b25bef7` (`v0.4.0` + `orphan-stop-warning` + `week-notes`, unreleased).
**Status**: locked 2026-10-24 after one review
(`docs/dev/plans/reports/export-notes-spec-review.md`, folded in NOTES.md
entry 133).
**Input**: brainstorm with the user. Decisions: NOTES.md entries 125 and 133.

## 1. Summary

Notes are readable only through `status` (one date) and `week --notes`
(one week, mixed with totals). Neither gives plain text to paste into
a report or pipe into another tool.

After this changeset, `mlm export notes` prints a day's, a week's or a
month's notes to stdout, one note per line, prefixed by its date. The
`export` verb leaves room for other data later.

## 1.1 Non-goals

- No JSON, CSV or other format. JSON waits for a future `export all`.
- No `--output` file. The user redirects stdout.
- No export of punches, totals or targets.
- No arbitrary `--from`/`--to` range.
- No change to `status`, `week` or any write command.

## 1.2 Known issues

None.

## 2. Prerequisites

None. `week-notes` already added the ranged note read this command
uses.

## 3. Surface

`mlm export notes [--date DATE | --week WEEK_ID | --month MONTH]`

- Aliases: `mlm ex` for `export`, `n` for `notes`, so `mlm ex n -w 43`
  works.
- `export` requires a subcommand. Bare `mlm export` is a clap usage
  error with exit `2`, listing `notes`.
- At most one of the three range flags. Two or more are a clap usage
  error with exit `2`.
- No range flag means the current week.
- `--date`/`-d DATE`: one date. Accepts the same forms as
  `start`/`stop`/`note` (SPEC §3.4a), `YYYY-MM-DD` and `-N` (N days
  before today), but without their future-date check.
- `--week`/`-w WEEK_ID`: one ISO week, Mon-Sun. Accepts `YYYY-WW` and
  a bare `WW` in the current ISO year, as `mlm week` does (SPEC §3.6).
- `--month`/`-m MONTH`: one calendar month, first to last day. Accepts
  `YYYY-MM`, `YYYY-M` and a bare `M`/`MM` in the current calendar year,
  the same pattern as `WEEK_ID`. The year is exactly four digits, any
  value.
- All three flags accept values starting with `-`, so `-d -1` works
  and `-w -1` or `-m -1` reach the §6 messages instead of a clap
  error.
- A future date, week or month is not an error. It has no notes, so it
  prints nothing.

`mlm export notes --help` text:

- command: `Print notes for a date, week or month, one per line
  (defaults to the current week)`
- `-d, --date <DATE>`: ``Date to export: YYYY-MM-DD, or `-N` for N days
  before today (e.g. `-1` = yesterday)``
- `-w, --week <WEEK_ID>`: `Week to export: YYYY-WW (e.g. 2026-07) or a
  bare week number (e.g. 7)`
- `-m, --month <MONTH>`: `Month to export: YYYY-MM (e.g. 2026-03) or a
  bare month number (e.g. 3)`

## 4. Semantics

- The range covers every calendar date from its first to its last day,
  inclusive.
- Order is ascending by date, then insertion order within a date
  (`created_at_utc ASC, id ASC`), the order `status` and
  `week --notes` use.
- A note belongs to its stored `date`. Punches play no part.
- One read: the ranged note read over the resolved range.

## 5. Output layout

One line per note, as the date in `YYYY-MM-DD`, two spaces, then the
body as stored. No header, no blank lines, no summary.

```
$ mlm export notes --week 2026-43
2026-10-19  fixed migration runner bug
2026-10-19  started punch pairing tests
2026-10-21  code review for #41
2026-10-24  refactored the parser
```

- An empty range prints nothing and exits `0`.
- Bodies pass through unchanged, non-ASCII included (SPEC §7).

## 6. Errors

All exit `1` with `error: ...` on stderr and empty stdout, unless
marked clap.

| Input | Message |
|---|---|
| bad `DATE` | existing `invalid DATE "..."` messages |
| bad `WEEK_ID` | existing `invalid WEEK_ID "..."` messages |
| `MONTH` not `YYYY-MM`/`M`/`MM` | `invalid MONTH "...": expected YYYY-MM or MM` |
| month number 0 or above 12 | `invalid MONTH "...": month must be 1-12` |
| a range flag given twice, or with no value as the last token | clap usage error, exit `2` |
| a range flag followed by another flag | that flag's own parse error, exit `1` (`-m -d` is a bad `MONTH`) |
| failed note read | storage error, as other reads (SPEC §6.1) |
| two range flags | clap usage error, exit `2` |
| bare `mlm export` | clap usage error, exit `2` |

## 7. Testing

- day, week and month ranges, each with notes inside and just outside
  both edges
- default range is the current week
- order across dates, and within a date where `id` and
  `created_at_utc` disagree
- `-d -1` resolves to yesterday; `-w -1` and `-m -1` give the exit-1
  messages
- bare `WW` and bare `M`/`MM` use the current year; `YYYY-M` is
  accepted
- month edges: February in a leap and a non-leap year, December
- empty range → empty stdout, exit `0`
- future date, week and month → empty stdout, exit `0`
- each error row in §6, with exact stderr and exit code
- aliases `ex` and `n`
- non-ASCII body passes through; all other output is ASCII
- `--help` lines

## 8. User-visible surface

- `README.md`: new `mlm export notes` section with the example, and a
  Features line.
- `mlm --help` lists `export`. `mlm export --help` lists `notes`.
  `mlm export notes --help` per §3.

## Housekeeping

- Re-wrap the `WeekArgs` doc comment in `src/cli.rs` to 80 columns
  (`week-notes` final review F3).

## Interaction with the project spec

- §3 gains a new `mlm export notes` subsection.
- §1.2 gains other export formats and `--output` as non-goals.
- §6.2 gains the empty and future ranges as not errors.
- §6.1 gains the `MONTH` errors and the export read, and its
  future-date rule names export as not checking.
- §7 gains the export layout.
- §8 gains happy-path and error flows.
- AGENTS.md's layout list gains the new command module, and its
  `src/date.rs` entry gains `MONTH` parsing.
