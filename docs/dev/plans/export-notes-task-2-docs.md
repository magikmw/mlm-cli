# Task plan: export-notes, Task 2 (docs)

Changeset plan: `docs/dev/plans/export-notes-plan.md`, "Task 2: docs".
Spec: `docs/dev/specs/2026-10-24-export-notes.md` (locked).
Branch: `export-notes` at `e80fa86`.

## Read for this plan

- `docs/dev/plans/export-notes-plan.md` in full (contracts 1-9, Task 2 section, risks)
- `docs/dev/specs/2026-10-24-export-notes.md` in full
- `docs/dev/SPEC.md`: headings; §1.2 and §1.2a (lines 36-66); §3 through §3.7 (lines 220-391); §6, §6.1, §6.2, §6.3 (lines 605-684); §7.2 (lines 994-1223); §7.4 (lines 1402-1499); §8 (lines 1501-1727)
- `README.md` in full (636 lines)
- `AGENTS.md` in full, including "Documentation" (lines 66-79)
- `docs/dev/README.md` in full (archive convention)
- Sibling task plans: none exist yet. `docs/dev/plans/export-notes-task-1-code.md` was absent when this plan was written. Nothing here depends on it.

## Scope

Owns exclusively:

- `docs/dev/SPEC.md`
- `README.md`
- `AGENTS.md`

Must not touch:

| File | Owner |
|---|---|
| `src/cli.rs`, `src/date.rs`, `src/lib.rs`, `src/main.rs`, `src/export.rs` (new) | Task 1 |
| `tests/export_notes_cli.rs` (new), every other test file | Task 1 |
| `coverage-baseline.json` | Task 1 (hook ratchets it) |
| `docs/dev/NOTES.md` | coordinator |
| `docs/dev/specs/2026-10-24-export-notes.md` (archive banner) | coordinator |
| this plan and the changeset plan | coordinator |

Handoffs against the changeset plan's contracts, with no Task 1 work planned here:

- Every help line quoted below comes from contracts 2 and 3. Task 1 ships the same strings in `--help`.
- The MONTH messages come from contract 7. Task 1 prints them.
- The example block comes from spec §5 via contract 9.
- `src/export.rs` is the module name pinned by the changeset plan ("Architecture"). AGENTS.md names it, and Task 1 creates it.

Spec §1.2 (known issues) is "None." No edit below adds known-issue or possible-issue text, and SPEC §1.2a and README "Possible issues" stay untouched.

## Editing rules for the implementer

- Copy the new prose below exactly. Do not reword it.
- Keep every quoted help line, error message and example-block line on one source line, unwrapped, even past 72 columns (SPEC line 327 does the same). Wrap the surrounding prose at 72 columns like its neighbours.
- New prose is plain ASCII apart from `§`, as the neighbouring sections are.
- No pointer to `docs/dev/specs/`, `docs/dev/plans/` or the changeset name in any of the three files.
- Line numbers refer to the files at `e80fa86`. Apply the edits bottom-up within a file, or locate each by its quoted anchor text, so earlier edits do not shift later anchors.

## 1. `docs/dev/SPEC.md`

New section numbers are §3.8 (surface) and §7.5 (layout). Neither number exists today (`grep -n '§3\.8\|§7\.5' docs/dev/SPEC.md` is empty), so no existing reference shifts.

### 1.1 §1.2 non-goals

Anchor, lines 49-53:

```
- `+N`/`-N` relative week notation for `week`'s week-id argument. (This
  is distinct from `status`'s `DATE` argument and `start`/`stop`/
  `note`'s `--date` flag, both of which already accept a `-N` shorthand
  — §3.4a, §3.5.)

```

Insert after line 52 (`  — §3.4a, §3.5.)`), before the blank line 53:

```
- Other `mlm export` formats, such as JSON or CSV, and an `--output`
  file. `mlm export notes` prints plain text to stdout, and the user
  redirects it (§3.8).
```

### 1.2 §3.8, new subsection

Anchor, lines 389-393:

```
`mlm week target --help` shows the usage `mlm week target [OPTIONS]
[WEEK_ID] <DURATION>` and an Arguments section naming both arguments. No
parser error for this subcommand mentions `<ARGS>`.

## 4. Time/duration semantics
```

Insert after line 391 (`parser error for this subcommand mentions `<ARGS>`.`) a blank line and then this block, so one blank line separates it from `## 4.`:

````
### 3.8 `mlm export notes [--date DATE | --week WEEK_ID | --month MONTH]`

Print the notes of one date, one ISO week or one calendar month to
stdout as plain text, one line per note (§7.5), for pasting into a
report or piping into another tool. `export` has the alias `ex` and
`notes` the alias `n`, so `mlm ex n -w 43` works. `export` requires a
subcommand, and `notes` is the only one.

- no range flag: the current ISO week, Monday to Sunday
- `--date`/`-d DATE`: one date, in the `DATE` forms of §3.4a
  (`YYYY-MM-DD`, or `-N` for N days before today), with no future-date
  check
- `--week`/`-w WEEK_ID`: Monday to Sunday of one ISO week, in the
  `WEEK_ID` forms of §3.6, with a bare `WW` in the current ISO year
- `--month`/`-m MONTH`: the first to the last calendar day of one
  month. `MONTH` is `YYYY-MM` (e.g. `2026-03`), `YYYY-M` (`2026-3`, the
  same month) or a bare `M`/`MM` (e.g. `3`) in the current calendar
  year. The calendar year is not the ISO year, so on 2025-12-30 `-m 1`
  is `2025-01`. The year is exactly four digits, and any value from
  `0000` to `9999` is accepted. February runs to the 28th or the 29th
  by the calendar.

At most one range flag is given. The range covers every date from its
first to its last day, inclusive. A note belongs to its stored `date`,
so punches play no part. The command makes one storage read, the
ranged note read over the range, and touches no punch, total or
target.

All three flags accept values that start with `-`. So `-d -1` is
yesterday, and `-w -1` and `-m -1` reach their own `WEEK_ID` and
`MONTH` errors with exit `1` (§6.1) instead of a parser error. A flag
followed by another flag takes that flag as its value, so `-m -d` fails
as a bad `MONTH`.

A future date, week or month is not an error. It has no notes, so the
command prints nothing and exits `0` (§6.2).

These are usage errors from the argument parser, with exit `2` (§6.3):

- bare `mlm export`, whose output lists `notes`
- two or more range flags
- one range flag given twice
- a range flag with no value as the last token

`mlm --help` lists `export` as `Print stored data as plain text`, and
`mlm export --help` lists `notes`. `mlm export notes --help` shows:

- command: `Print notes for a date, week or month, one per line (defaults to the current week)`
- `-d, --date <DATE>`: ``Date to export: YYYY-MM-DD, or `-N` for N days before today (e.g. `-1` = yesterday)``
- `-w, --week <WEEK_ID>`: `Week to export: YYYY-WW (e.g. 2026-07) or a bare week number (e.g. 7)`
- `-m, --month <MONTH>`: `Month to export: YYYY-MM (e.g. 2026-03) or a bare month number (e.g. 3)`
````

The heading text matches spec §3's surface line. The four help strings are contract 3 verbatim, and the `export` description is contract 2 verbatim.

### 1.3 §6.1, future-date rule

Anchor, lines 623-629:

```
- `--date`/`-d` on `start`/`stop`/`note` resolving to a date later than
  today (§3.4a), or `status`'s `DATE` argument (§3.5) resolving to one.
  The first three never write into the future, and `status` is no
  preview of a future date. `status DATE` uses the same message shape and
  exit code as the `--date` case, and adds `; to view a future week, run
  mlm week WEEK_ID` to it. `mlm week WEEK_ID` for a future week is
  not an error (§7.2).
```

Replace line 629 (`  not an error (§7.2).`) with:

```
  not an error (§7.2). `mlm export notes` checks no range against today,
  so its `--date`, `--week` or `--month` may name a future range
  (§3.8, §6.2).
```

### 1.4 §6.1, MONTH errors

Anchor, lines 630-637:

```
- Malformed `WEEK_ID` (§3.6): not a bare week number or `YYYY-WW`
  ...
  `TIME`/`DURATION` input is lenient about padding elsewhere.
- Malformed `DURATION` (§3.7/§4.2): doesn't match the input grammar,
```

Insert after line 636 (`  `TIME`/`DURATION` input is lenient about padding elsewhere.`), before the `DURATION` bullet:

```
- Malformed `MONTH` (§3.8). A value that is not `YYYY-MM`, `YYYY-M` or
  a bare `M`/`MM` prints this:

  `invalid MONTH "<input>": expected YYYY-MM or MM`

  A month number of `0` or above `12` prints this:

  `invalid MONTH "<input>": month must be 1-12`

  `<input>` is the value as typed. The year is exactly four digits, and
  any four-digit year is accepted.
```

The two messages are contract 7 verbatim.

### 1.5 §6.1, read-failure rule

Anchor, lines 655-662:

```
- A storage read failure in `status` or `mlm week`, on any of the
  reads they make (the earliest data week, the target overrides, the
  punches behind the carry walk, and for `mlm week --notes` the shown
  week's notes). The command prints `error:` and the storage error text
  to stderr, prints nothing to stdout, and exits `1`. `mlm week` never
  falls back to the default target or to "no earlier weeks", and
  `--notes` never falls back to an empty note list. Plain `mlm week`
  makes no note read.
```

Replace lines 655-662 with:

```
- A storage read failure in `status`, `mlm week` or `mlm export notes`,
  on any of the reads they make (the earliest data week, the target
  overrides, the punches behind the carry walk, for `mlm week --notes`
  the shown week's notes, and for `mlm export notes` its one ranged
  note read). The command prints `error:` and the storage error text to
  stderr, prints nothing to stdout, and exits `1`. `mlm week` never
  falls back to the default target or to "no earlier weeks", and
  neither `--notes` nor `mlm export notes` falls back to an empty note
  list. Plain `mlm week` makes no note read.
```

### 1.6 §6.2 not errors

Anchor, lines 675-678:

```
- A week with no `week_targets` row: valid, uses the default target
  (40h) rather than erroring for "unset".

### 6.3 Exit codes
```

Insert after line 676 (`  (40h) rather than erroring for "unset".`), before the blank line:

```
- `mlm export notes` over a range with no notes: prints nothing and
  exits `0` (§7.5).
- `mlm export notes` for a future date, week or month: not checked
  against today (§3.8). It has no notes, so it prints nothing and exits
  `0`.
```

### 1.7 §7.4, output summary sentence

Anchor, lines 1406-1408:

```
Unix-conventional, and exit `0` (§6.3) is the only signal. A hard error
(§6.1) still prints its message to stderr as usual. `status` and `week`
produce stdout output (§7.1, §7.2). `week target` prints one line
```

Replace the sentence `` `status` and `week` produce stdout output (§7.1, §7.2).`` with:

```
`status`, `week` and `export notes` produce stdout output (§7.1, §7.2,
§7.5).
```

Re-wrap the rest of that paragraph to 72 columns. No other word in it changes.

### 1.8 §7.5, new subsection

Anchor, lines 1494-1501:

```
This check is best-effort too. If any read after the committed write
...
is unchanged.

## 8. User flows (test basis)
```

Insert after line 1499 (`is unchanged.`) a blank line and then this block, so one blank line separates it from `## 8.`:

````
### 7.5 `mlm export notes`

One line per note: the date as `YYYY-MM-DD`, two spaces, then the body
as stored. No header, no blank line, no summary. Lines run ascending by
date, and within a date in insertion order (`created_at_utc ASC, id
ASC`, §2.3), the order `status` (§7.1) and `mlm week --notes` (§7.2)
use. A week with notes on three of its dates:

```
$ mlm export notes --week 2026-43
2026-10-19  fixed migration runner bug
2026-10-19  started punch pairing tests
2026-10-21  code review for #41
2026-10-24  refactored the parser
```

- An empty range prints nothing and exits `0`.
- Note bodies pass through unchanged, non-ASCII included, as in
  `status` (§2.3, §7). Every other byte is ASCII.
````

The first sentence has a colon before a clause. It is kept because it introduces the line's three parts. If the implementer's style check objects, use this instead: `Each note prints on one line as the date in `YYYY-MM-DD`, two spaces, then the body as stored.` The code block is spec §5 verbatim (contract 9), `$` line included.

### 1.9 §8.1 happy path F15

Anchor, lines 1580-1583:

```
  `mlm week -n WEEK_ID` and `mlm week WEEK_ID --notes` print the same.
  Plain `mlm week` prints no note lines.

### 8.2 Error / edge paths (§6 direct mapping)
```

Insert after line 1581 (`  Plain `mlm week` prints no note lines.`), before the blank line:

```
- **F15** - Export notes (§3.8, §7.5): two notes on 2026-10-19 in that
  entry order, one on 2026-10-21 and one on 2026-10-24, then `mlm
  export notes --week 2026-43` prints the four lines of §7.5's example.
  `--date 2026-10-19` prints that date's two lines, and `--month
  2026-10` prints all four. Notes dated the day before a range's first
  day or the day after its last day never appear. No range flag prints
  the current week's notes and none from the weeks either side. Two
  notes on one date whose later `id` has the earlier `created_at_utc`
  print in `created_at_utc` order. `mlm ex n -w 2026-43` prints the
  same as the long form, and a non-ASCII body passes through unchanged.
```

### 1.10 §8.2 error flows E28-E31

Anchor, lines 1724-1727:

```
  17:00) and an orphan already on the date. A clean pair and an end that
  splices with the previous date's open start stay silent. A read
  failure after the write prints nothing, and the punch is stored.

```

Insert after line 1726 (`  failure after the write prints nothing, and the punch is stored.`):

```
- **E28** - `mlm export notes` ranges (§3.8, §6.2): `--month 2024-02`
  covers the 1st to the 29th, `--month 2026-02` the 1st to the 28th,
  and a December runs to the 31st. `--month 2026-3` equals `--month
  2026-03`. A bare `WW` uses the current ISO year and a bare `M`/`MM`
  the current calendar year, so on 2025-12-30 `-m 1` is `2025-01`.
  `-d -1` is yesterday. A range with no notes, and a future date, week
  or month, print nothing and exit `0`.
- **E29** - `mlm export notes` value errors (§6.1): `-m 2026-003`,
  `-m 26-03`, `-m 2026-` and `-m -1` fail with the shape message, e.g.
  `-m abc` prints `error: invalid MONTH "abc": expected YYYY-MM or MM`.
  `-m 0`, `-m 13` and `-m 2026-13` fail with the range message, e.g.
  `-m 13` prints `error: invalid MONTH "13": month must be 1-12`. A bad
  `DATE` and a bad `WEEK_ID`, `-w -1` included, print their existing
  messages, and `-m -d` fails as a bad `MONTH`. Each exits `1` with
  nothing on stdout.
- **E30** - `mlm export notes` usage errors (§3.8, §6.3): bare `mlm
  export` (its output lists `notes`), two range flags such as `-d
  2026-10-19 -w 2026-43`, one flag given twice such as `-m 3 -m 4`, and
  a range flag with no value as the last token each exit `2` from the
  argument parser.
- **E31** - `mlm export notes` read failure (§6.1): with the
  `notes.body` column renamed, it prints `error: ...` to stderr,
  nothing to stdout, and exits `1`.
```

Line 1727 stays the file's trailing blank line.

## 2. `README.md`

### 2.1 Features line

Anchor, lines 18-20:

```
- `week target` — override a week's target hours and print the stored
  target back; any shortfall or surplus carries into the next week
- `delete note`/`delete punch` — list a date's entries and delete one
```

Insert after line 19 (`  target back; any shortfall or surplus carries into the next week`):

```
- `export notes` — print a date's, a week's or a month's notes as plain
  text, one per line, ready for a report or another tool
```

### 2.2 Usage intro, output sentence

Anchor, lines 124-125:

```
`week target` prints one line confirming the stored target. `status`
and `week` are the commands that produce output.
```

Replace lines 124-125 with:

```
`week target` prints one line confirming the stored target. `status`,
`week` and `export notes` are the commands that produce output.
```

### 2.3 Usage intro, alias sentence

Anchor, line 131:

```
Every command has a 1 character alias, except `delete` (`del`).
```

Replace line 131 with:

```
Every command has a 1 character alias, except `delete` (`del`) and `export` (`ex`).
```

Line 131 is already unwrapped, like line 132, so the longer line keeps that style.

### 2.4 New `mlm export notes` section

Anchor, lines 584-587:

```
Target:        40h 00m
```

### `mlm delete|del note|n [ID] [-d/--date DATE]`
```

Insert after line 585 (the closing ```` ``` ```` of the past-week `mlm week 2026-35` example) a blank line and then this block, so one blank line separates it from the `delete note` heading:

````
### `mlm export|ex notes|n [-d/--date DATE | -w/--week WEEK_ID | -m/--month MONTH]`

Print a date's, a week's or a month's notes as plain text, one per
line, ready to paste into a report or pipe into another tool. With no
flag it prints the current week. Give at most one range:

- `-d`/`--date DATE` — one date, `YYYY-MM-DD` or `-N` for N days
  before today (e.g. `-1` = yesterday). A future date is allowed.
- `-w`/`--week WEEK_ID` — one week, Monday to Sunday, in the same forms
  as `week` (`2026-43`, or a bare `43` for the current year)
- `-m`/`--month MONTH` — one calendar month, `YYYY-MM` (e.g. `2026-03`)
  or a bare month number for the current year (e.g. `3`)

Each line is the note's date, two spaces, then the note. Lines run by
date, and within a date in the order you entered them:

```sh
$ mlm export notes --week 2026-43
2026-10-19  fixed migration runner bug
2026-10-19  started punch pairing tests
2026-10-21  code review for #41
2026-10-24  refactored the parser
```

A range with no notes prints nothing and exits 0. So does a future
date, week or month. There is no header or summary line, so the output
redirects straight into a file:

```sh
$ mlm export notes --month 2026-10 > october.txt
```
````

The heading follows the `delete|del note|n` convention (alias after `|`, short/long flag as `-d/--date`). The example lines are spec §5 verbatim (contract 9). README fences command examples as `sh`, as every README example with a `$` line does.

## 3. `AGENTS.md`

### 3.1 Current state

Anchor, lines 16-17:

```
All commands (`start`, `stop`, `note`, `delete`, `status`, `week`,
`week target`) are implemented, tested, and match `docs/dev/SPEC.md`.
```

Replace lines 16-17 with:

```
All commands (`start`, `stop`, `note`, `delete`, `status`, `week`,
`week target`, `export notes`) are implemented, tested, and match
`docs/dev/SPEC.md`.
```

### 3.2 `src/date.rs` entry

Anchor, line 35:

```
- `src/date.rs` — DATE/WEEK_ID parsing, `WeekId`
```

Replace line 35 with:

```
- `src/date.rs` — DATE/WEEK_ID/MONTH parsing, `WeekId`, and a month's
  first and last day
```

### 3.3 `src/storage.rs` entry

Anchor, lines 36-40:

```
- `src/storage.rs` — `Punch`/`PunchKind`/`Note`, punch/note insert+read,
  one ranged week-target-override read, and one ranged note read
  (`notes_in_range`, an inclusive date range in, a flat `Vec<Note>`
  out, ascending by date, insertion order within a date) that
  `week --notes` calls
```

Replace line 40 (`  `week --notes` calls`) with:

```
  `week --notes` and `export notes` call
```

### 3.4 `src/export.rs` entry

Anchor, lines 61-63:

```
- `src/commands.rs`, `src/status.rs`, `src/week_target.rs`,
  `src/week_view.rs` — per-command wiring over the above
- `src/main.rs` — CLI entry point: parses args, dispatches, exit code
```

Insert after line 62 (`  `src/week_view.rs` — per-command wiring over the above`):

```
- `src/export.rs` — `mlm export notes`, which resolves the range, makes
  one ranged note read and prints one line per note
```

## 4. Test plan

Run every check from `/home/magikmw/projects/mlm-cli` after the edits. "Added lines" means `git diff e80fa86 -- <file> | grep '^+'`. Each subsection maps to one changeset-plan acceptance criterion.

### 4.1 SPEC §3 subsection (spec §3, contracts 1, 3, 6, 8)

```sh
grep -n '^### 3.8 `mlm export notes \[--date DATE | --week WEEK_ID | --month MONTH\]`$' docs/dev/SPEC.md
grep -cF 'mlm ex n -w 43' docs/dev/SPEC.md            # >= 1
grep -nF 'any value from' docs/dev/SPEC.md             # four-digit year rule
grep -nF 'on 2025-12-30 `-m 1`' docs/dev/SPEC.md       # calendar year, twice (3.8, E28)
grep -nF 'accept values that start with `-`' docs/dev/SPEC.md
grep -nF 'A future date, week or month is not an error.' docs/dev/SPEC.md
grep -nF 'bare `mlm export`, whose output lists `notes`' docs/dev/SPEC.md
```

Each prints at least one line inside §3.8. Help-line exactness is 4.8.

### 4.2 SPEC §6.1 (spec §6, contract 7)

```sh
grep -nF 'invalid MONTH "<input>": expected YYYY-MM or MM' docs/dev/SPEC.md
grep -nF 'invalid MONTH "<input>": month must be 1-12' docs/dev/SPEC.md
grep -nF 'for `mlm export notes` its one ranged' docs/dev/SPEC.md
grep -nF '`mlm export notes` checks no range against today,' docs/dev/SPEC.md
```

Each match lies between the `### 6.1` and `### 6.2` headings (compare with `grep -n '^### 6\.[12]' docs/dev/SPEC.md`).

### 4.3 SPEC §7 layout (spec §5, contracts 4, 5, 9)

```sh
grep -n '^### 7.5 `mlm export notes`$' docs/dev/SPEC.md
grep -nF '`created_at_utc ASC, id' docs/dev/SPEC.md     # hits in both 7.2 and 7.5
tr -s '\n ' ' ' < docs/dev/SPEC.md | grep -cF '`status`, `week` and `export notes` produce stdout output (§7.1, §7.2, §7.5).'
```

The second check joins lines first, because 1.7 re-wraps that paragraph. It prints `1`.

The example block is checked byte for byte in 4.8.

### 4.4 SPEC §8 flows (spec §7 testing list, spec "Interaction with the project spec")

```sh
grep -nE '^- \*\*(F15|E28|E29|E30|E31)\*\* - ' docs/dev/SPEC.md   # 5 lines
```

F15 must fall between `### 8.1` and `### 8.2`, and E28-E31 after `### 8.2`. Read the five bullets against spec §7's list. Each spec §7 item maps to F15 or one of E28-E31, apart from `--help` lines, which §3.8 pins.

### 4.5 README section and Features line (spec §8)

```sh
grep -n '^### `mlm export|ex notes|n ' README.md
grep -n '^- `export notes` — ' README.md
grep -nF 'except `delete` (`del`) and `export` (`ex`).' README.md
grep -nF '`week` and `export notes` are the commands that produce output.' README.md
```

The README example block is checked in 4.8.

### 4.6 AGENTS.md (changeset plan Task 2 criterion 6)

```sh
grep -nF '`week target`, `export notes`) are implemented' AGENTS.md
grep -nF -- '- `src/date.rs` — DATE/WEEK_ID/MONTH parsing' AGENTS.md
grep -nF '`week --notes` and `export notes` call' AGENTS.md
grep -nF -- '- `src/export.rs` — `mlm export notes`' AGENTS.md
```

### 4.7 SPEC §6.2 and §1.2 (spec §1.1, §3, §5, "Interaction with the project spec")

```sh
grep -nF '`mlm export notes` over a range with no notes' docs/dev/SPEC.md
grep -nF '`mlm export notes` for a future date, week or month' docs/dev/SPEC.md
grep -nF 'Other `mlm export` formats, such as JSON or CSV' docs/dev/SPEC.md
```

The first two lie between `### 6.2` and `### 6.3`, the third between `### 1.2 ` and `### 1.2a`.

### 4.8 Byte-for-byte quotes (contracts 2, 3, 4, 7, 9)

Each help line must appear whole on one line of SPEC.md:

```sh
f=docs/dev/SPEC.md
grep -cF 'Print stored data as plain text' $f
grep -cF '`Print notes for a date, week or month, one per line (defaults to the current week)`' $f
grep -cF '``Date to export: YYYY-MM-DD, or `-N` for N days before today (e.g. `-1` = yesterday)``' $f
grep -cF '`Week to export: YYYY-WW (e.g. 2026-07) or a bare week number (e.g. 7)`' $f
grep -cF '`Month to export: YYYY-MM (e.g. 2026-03) or a bare month number (e.g. 3)`' $f
```

Each prints `1`.

The example block must match spec §5 in both docs:

```sh
mkdir -p /tmp/cd-scratch/export-notes-task-2
sed -n '89,93p' docs/dev/specs/2026-10-24-export-notes.md > /tmp/cd-scratch/export-notes-task-2/ex.txt
for f in docs/dev/SPEC.md README.md; do
  grep -A4 -xF '$ mlm export notes --week 2026-43' "$f" | diff - /tmp/cd-scratch/export-notes-task-2/ex.txt && echo "$f ok"
done
```

Both print `ok`. The block's lines are separated by two spaces exactly. A tab or one space fails the diff.

### 4.9 Self-contained docs and no known-issue text (AGENTS.md "Documentation", spec §1.2)

```sh
git diff e80fa86 -- docs/dev/SPEC.md README.md AGENTS.md | grep '^+' | grep -nE 'docs/dev/(specs|plans)|export-notes|2026-10-24-export'
git diff e80fa86 -- docs/dev/SPEC.md README.md AGENTS.md | grep '^+' | grep -niE 'known issue|possible issue'
diff <(git show e80fa86:docs/dev/SPEC.md | sed -n '/^### 1.2a/,/^### 1.3/p') <(sed -n '/^### 1.2a/,/^### 1.3/p' docs/dev/SPEC.md)
diff <(git show e80fa86:README.md | sed -n '/^### Possible issues/,/^## Install/p') <(sed -n '/^### Possible issues/,/^## Install/p' README.md)
```

All four print nothing. The two `diff`s prove SPEC §1.2a and README "Possible issues" are unchanged. Run them in bash, since the login shell is fish.

### 4.10 ASCII and file boundaries

```sh
git diff e80fa86 -- docs/dev/SPEC.md | grep '^+' | LC_ALL=C.UTF-8 grep -nP '[^\x00-\x7F§]'
git diff --name-only e80fa86
```

The first prints nothing. README and AGENTS.md use `—` in their list entries already, so their new `—` bullets are expected and not checked. The second lists only `AGENTS.md`, `README.md` and `docs/dev/SPEC.md` (plus this plan if the coordinator commits it on the same branch).

### 4.11 Repo gate

`cargo fmt --check`, `cargo build`, `cargo test` and `cargo clippy --all-targets -- -D warnings` stay green because no source file changes. The pre-commit hook runs them anyway.

## 5. Risks, ambiguities, disagreements

- **Colon in §7.5's first sentence.** "One line per note: the date ..." reads as a three-part list, so it fits the project's list exception. An alternative sentence is given in 1.8 in case review disagrees.
- **Help strings past 72 columns.** SPEC wraps prose at 72, but a wrapped help line would break the byte-for-byte greps in 4.8. The plan keeps them on one line, as SPEC line 327 already does for a help string.
- **`export` one-line description.** The changeset plan's risk list says Task 2 need not quote it, but contract 2 now pins `Print stored data as plain text`. §3.8 quotes it, in line with §3.6, which quotes `--notes`'s help line. If Task 1 ships a different string, the contract wins and §3.8 changes with it.
- **README alias sentence.** Line 131 says every command but `delete` has a one-character alias. `export` (`ex`) breaks that, so 2.3 updates it. The changeset plan does not list this edit, but leaving the sentence would make README wrong.
- **README and SPEC §7.4 "commands that produce output" sentences.** Both name only `status` and `week`. They are updated for the same reason, and the changeset plan does not list them either.
- **README year wording.** README's `week` section says "the current year" for a bare week number. The new README section keeps that plain wording for both `-w` and `-m`. SPEC §3.8 spells out ISO year versus calendar year. README skips the 2025-12-30 edge case for brevity.
- **Example of `mlm ex n -w 43`.** SPEC §3.8 quotes contract 1's `mlm ex n -w 43`. F15 uses `-w 2026-43` so the flow does not depend on the current year.
- **AGENTS.md drift.** The AGENTS.md text loaded with the dispatch shows a shorter `src/storage.rs` entry than the file on disk at `e80fa86` (lines 36-40 name `notes_in_range`). The anchors here follow the file on disk.
- **`src/history.rs` entry.** It mentions `mlm week --notes`'s note read outside the loader. `export notes` never uses the loader, so the entry is left alone.
- **No disagreement** with the spec or the changeset plan's Task 2 scope.
