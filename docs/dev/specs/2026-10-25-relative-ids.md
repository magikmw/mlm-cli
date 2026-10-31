# Relative week and month ids — design spec

**Baseline**: written against `main` at `77fb19f` (`v0.4.0` + `orphan-stop-warning` + `week-notes` + `export-notes` + `panic-fixes`, unreleased).
**Status**: draft, spec review folded
(`docs/dev/plans/reports/relative-ids-spec-review.md`, NOTES.md entry 152).
**Input**: NOTES.md entry 137, and the user's scope call on
2026-10-25 to include `week target`.

## 1. Summary

`DATE` takes `-N` for N days before today, but `WEEK_ID` and `MONTH`
take only absolute forms. Looking at last week means working out its
number first.

After this changeset, every `WEEK_ID` takes `-N` for N weeks before
the current ISO week, and `MONTH` takes `-N` for N calendar months
before the current month. `mlm week -1`, `mlm week target -1 35h`,
`mlm ex n -w -1` and `mlm ex n -m -1` all work.

## 1.1 Non-goals

- No `+N` for future weeks or months. An absolute id still reaches
  them.
- No change to `DATE`.
- No relative forms anywhere else.

## 1.2 Known issues

None.

## 2. Prerequisites

None.

## 3. Surface

`-N` is `-` followed by one or more ASCII digits, N >= 1, leading
zeros tolerated (`-01` = `-1`), the same grammar `DATE` uses.

| Argument | Where | `-N` means |
|---|---|---|
| `WEEK_ID` | `mlm week [WEEK_ID]` | N ISO weeks before the current one |
| `WEEK_ID` | `mlm week target [WEEK_ID] DURATION` | same |
| `WEEK_ID` | `mlm export notes --week` | same |
| `MONTH` | `mlm export notes --month` | N calendar months before the current one |

- The current week and month come from today's local date.
- `-N` crosses year boundaries. On 2026-01-05 (week 2026-02), `-2` is
  week 2025-52. In January 2026, `-m -1` is December 2025.
- `mlm week -1` takes `-1` as `WEEK_ID`, as `mlm status -1` takes it
  as `DATE`. `-n`/`--notes` stays a flag, so `mlm week -1 -n` and
  `mlm week -n -1` both work.
- `mlm week` passes only `-` plus digits through to the `WEEK_ID`
  parser, as `mlm status` does for `DATE`. Any other token starting
  with `-` (`-1x`, `-x`, `--bogus`) stays a parser usage error, exit
  `2` (SPEC §6.3), as `mlm status -1x` is today.
- `week target` and `export notes` already pass every value starting
  with `-` to their parsers, so `-1x` there gets the §4 shape error,
  exit `1`.
- `mlm week target -1 35h` sets last week's target. The swapped shape
  `mlm week -1 target 35h` keeps the reminder (SPEC §3.7), echoing
  `-1` as typed.

Help text:

- `mlm week`, `[WEEK_ID]`: ``Week to show: YYYY-WW (e.g. 2026-07), a
  bare week number (e.g. 7), or `-N` for N weeks before this one (e.g.
  `-1` = last week). Defaults to the current week``
- `mlm week target`, `[WEEK_ID]`: ``Week to set: YYYY-WW (e.g.
  2026-07), a bare week number (e.g. 7), or `-N` for N weeks before
  this one (e.g. `-1` = last week). Defaults to the current week``
- `mlm export notes`, `-w, --week <WEEK_ID>`: ``Week to export:
  YYYY-WW (e.g. 2026-07), a bare week number (e.g. 7), or `-N` for N
  weeks before this one (e.g. `-1` = last week)``
- `mlm export notes`, `-m, --month <MONTH>`: ``Month to export:
  YYYY-MM (e.g. 2026-03), a bare month number (e.g. 3), or `-N` for N
  months before this one (e.g. `-1` = last month)``

## 4. Errors

All exit `1`, `error: ...` on stderr, nothing written. The `mlm week`
tokens §3 leaves to the parser keep exit `2`.

| Input | Message |
|---|---|
| bad `WEEK_ID` shape, `-0` included | `invalid WEEK_ID "<input>": expected YYYY-WW, WW or -N (N >= 1)` |
| bad `MONTH` shape, `-0` included | `invalid MONTH "<input>": expected YYYY-MM, MM or -N (N >= 1)` |
| `WEEK_ID` `-N` too large to resolve | `invalid WEEK_ID "<input>": week offset is out of range` |
| `MONTH` `-N` too large to resolve | `invalid MONTH "<input>": month offset is out of range` |

- `-N` is out of range when it resolves to an id the absolute form
  rejects. For `WEEK_ID` that is any ISO year below `1000`, so
  `1000-01` is the earliest week `-N` reaches. For `MONTH` that is any
  year below `0000`, so `0000-01` is the earliest month.
- A digit run too large for a `u64` is out of range, as for `DATE`.
- The two shape messages replace `expected YYYY-WW or WW` and
  `expected YYYY-MM or MM` everywhere those appear. Every other
  `WEEK_ID` and `MONTH` message is unchanged.

## 5. Testing

- `-1`, `-2`, `-01`, and an N crossing a year boundary, for each row of
  the §3 table, against a fixed today
- week 53 years: `-N` landing on week 53 and stepping over it
- `-0` gives the shape errors; `-1x` gives them for `week target`,
  `-w` and `-m`; `mlm week -1x` and `mlm week -x` exit `2`
- offset boundaries, mirroring `DATE`'s: the last N that reaches
  `1000-01` / `0000-01`, the first N past it, and a digit run that
  overflows `u64`
- existing tests that assert `-N` is rejected become acceptance tests
  or move to inputs that stay malformed: `src/date.rs` T57 (`-5`), T59
  (`-07`, now seven weeks ago), T65 (`-1`), the MONTH bad-shape list
  and `-1` message test, `src/export.rs`'s `-w -1` test. Every
  assertion of the old shape messages takes the new text
- `mlm week -1 -n`, `mlm week -n -1`
- `mlm week target -1 35h` writes last week's target and echoes its
  normalized id
- `mlm week -1 target 35h` prints the reminder, exit `2`
- `--help` lines per §3

## 6. User-visible surface

- `README.md`: the `mlm week`, `week target` and `export notes`
  sections gain the `-N` form with an example. The non-goals bullet
  on `+N`/`-N` relative week notation (README.md:42-43) becomes a
  `+N`-only bullet.
- `--help` per §3.

## Interaction with the project spec

- §1.2 drops the relative week non-goal and names `+N` as one.
- §3.6, §3.7 and §3.8 gain the `-N` form.
- §3.8's sentence saying `-w -1` and `-m -1` reach their own errors
  is rewritten, and its quoted `-w`/`-m` help lines take the §3 text.
- §6.1's `WEEK_ID` bullet gains both `WEEK_ID` messages. Its `MONTH`
  bullet's shape message is replaced, and the offset message added.
- §8 gains happy-path and error flows.
