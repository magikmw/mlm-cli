# Task 2 plan: docs (SPEC.md, README.md, AGENTS.md)

> **Archived — historical planning record.** Not authoritative. Current spec: `docs/dev/SPEC.md`. Design/process history: `docs/dev/NOTES.md`.

Changeset: week-notes. Altitude: task plan. No file is edited by this plan.

## Read

- `docs/dev/plans/week-notes-plan.md` (changeset plan, all 101 lines, re-read after the known-issues drop: Task 2 leaves README's possible-issues list unchanged and has no §1.2a criterion)
- `docs/dev/specs/2026-10-24-week-notes.md` (spec, all, re-read after the drop: §1.2 Known issues is "None.")
- `docs/dev/plans/reports/week-notes-plan-review.md` (all, for U2, which became contract 2's glossary entry)
- `docs/dev/SPEC.md`: lines 1-380 (§1-§3.7, incl. §1.2a, §3.5 glossary sentence, §3.6), 590-679 (§6, §6.1), 675-720 (§7.1 Notes block), 977-1166 (§7.2), 1438-1655 (§8.1, §8.2 incl. E20)
- `README.md` (all 600 lines)
- `AGENTS.md` (all; "Layout" lines 24-59, "Documentation" lines 61-74)
- `docs/dev/README.md` (archive convention)
- `src/cli.rs:1-57` (the shared `GLOSSARY` constant) and `src/cli.rs:282-287` (`with_glossary` applies it to both `status` and `week`), read only
- `docs/dev/plans/orphan-stop-warning-task-2-docs.md` (house format for a docs task plan)
- Sibling task plan `docs/dev/plans/week-notes-task-1-code.md` (all; authoritative for shared text and signatures). Shared text below comes from it (step 1.1 help line, step 1.2 glossary entry and slot, step 2.1 `notes_in_range`, step 3.4 `NOTE_PREFIX`, step 4 `src/history.rs` doc) and from the changeset plan's contracts 2, 3, 6 and 8 and spec §3-§6.

## Scope

| Owned by this task | Files not to touch, and their owner |
|---|---|
| `docs/dev/SPEC.md`, `README.md`, `AGENTS.md` | `src/cli.rs` (flag, help line, glossary entry), `src/storage.rs`, `src/week_view.rs`, `src/history.rs` module doc, `tests/week_notes_cli.rs`, `tests/week_target_cli.rs` (the `mlm week -h` golden): Task 1 (code and tests) |
| | `docs/dev/NOTES.md`, the working spec's archive banner, the changeset plan and reports: coordinator |

`--help` text is Task 1's. This task quotes contract 2's flag help line. It names the glossary entry's term (`- <note>`) and slot, both pinned in Task 1 step 1.2, and does not quote the entry's body.

This task adds no known-issue text anywhere. SPEC §1.2a and README's "Possible issues" list stay as they are (spec §1.2 is "None.").

## Dependencies and handoffs

| Dependency | State | What this task uses | Swap at integration |
|---|---|---|---|
| Task 1 (code) | plan written, not implemented | Contracts 2, 3, 4, 6, 7, 8; the name `storage::notes_in_range` and its shape (inclusive `NaiveDate` range in, `Result<Vec<Note>, StorageError>` out) for A1. | None for the quoted blocks. Check 2.9 runs the binary at merge. |
| Task 1's glossary entry | pinned (Task 1 step 1.2) | Term `- <note>`, third of three entries before `[!], [i]`, in the one shared `GLOSSARY`. | None. S1 is fixed to that slot. |

- **H1 (resolved by Task 1).** `GLOSSARY` stays one constant that `with_glossary` attaches to both `status` and `week` (`src/cli.rs:282-287`), so the entry shows in `mlm status --help` too. Task 1 places it after `spans to next day, ...` and before `[!], [i]`, which stays last. SPEC §3.5 (lines 313-323) counts the entries before `[!], [i]`, so S1 updates the count to three and names the new term.

## Pinned literals (quote byte for byte)

Contract 2's flag help line:

```
Show each date's notes under its row
```

Contract 3's note line shape is six spaces, `- `, then the body. Contract 6's example is spec §5's block, reproduced here exactly:

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

The arithmetic holds. Rows sum to 48h 48m, fulfillment is 48h 48m - 03h 32m = 45h 16m, and a Saturday headline uses the full-target form, 45h 16m - 40h 00m = 05h 16m over.

## Facts gathered

### SPEC.md

| Line | Section | Current text (relevant part) |
|---|---|---|
| 64-66 | §1.2a | One bullet. Unchanged by this task. |
| 313-323 | §3.5 glossary sentence | "Two entries precede the `[!], [i]` one. ... The `[i]` entry, last, ends ..." |
| 325 | §3.6 heading | `` ### 3.6 `mlm week [WEEK_ID]` `` |
| 333-338 | §3.6 bullets | `[i]` bullet, then the trailing-block bullet |
| 340-342 | §3.6 | `WEEK_ID` forms paragraph |
| 640-645 | §6.1 | Read-failure bullet listing three reads |
| 692-694 | §7.1 | `Notes:` header and two-space bullets in `status` |
| 1157-1160 | §7.2 | Last bullet, "Every storage read `mlm week` makes is a hard error..." |
| 1509-1511 | §8.1 F13 | Highest F id. F14 is free. |
| 1601-1605 | §8.2 E20 | Names the `notes` table and punch range |

Recent §8 entries use ` - ` after the id. F14 follows that form.

### README.md

| Line | Section | Current text (relevant part) |
|---|---|---|
| 16-17 | Features | "`week` — a week's per-day totals plus carry-in/fulfillment against target" |
| 48-51 | Possible issues | One bullet. Unchanged by this task. |
| 409 | `mlm week` heading | `` ### `mlm week|w [WEEK_ID]` `` |
| 480-484 | `mlm week` | Ends "If `mlm week` cannot read the database, it prints an `error:` line and exits 1 instead of showing default figures." Next line is the `week target` heading at 486. |

README headings spell flags `[-d/--date DATE]` (lines 134, 183, 215), hence contract 8's `-n/--notes`.

### AGENTS.md

| Line | Bullet | Current text |
|---|---|---|
| 36-37 | `src/storage.rs` | "...punch/note insert+read, one ranged week-target-override read" |
| 46-50 | `src/history.rs` | "...for `status` and `mlm week`. Any read failure is an error. `stop`'s orphaned-end warning also uses its `PunchBuckets` view, over a three-date punch read of its own." |

## Edits

Each step gives the current text (the `old_string`) and the exact replacement. Line numbers are from the branch tip (`3bcff42`) and shift as earlier steps land, so match on text. Keep every other line as it is.

### SPEC.md

#### S1. §3.5 glossary sentence (lines 316-323), per H1

Task 1 step 1.2 puts the `- <note>` entry third, before `[!], [i]`, in the glossary both commands share.

Current:

```
Two entries precede the `[!], [i]` one. The first is `so far, started
yesterday`, for the overnight stint line (§7.1): a start still open
from yesterday, listed on today's page and not counted until stopped.
The second is `spans to next day, continues previous day's stint`, for
the §4.3.1 suffixes: a stint stopped after midnight is counted on the
day it started, `spans to next day` ends the start date's stint line
and `continues previous day's stint` ends the next date's header. The
`[i]` entry, last, ends ``It marks yesterday's row in `mlm week` until that start is stopped.``
```

Replacement:

```
Three entries precede the `[!], [i]` one. The first is `so far, started
yesterday`, for the overnight stint line (§7.1): a start still open
from yesterday, listed on today's page and not counted until stopped.
The second is `spans to next day, continues previous day's stint`, for
the §4.3.1 suffixes: a stint stopped after midnight is counted on the
day it started, `spans to next day` ends the start date's stint line
and `continues previous day's stint` ends the next date's header. The
third is `- <note>`, for one of a date's notes in insertion order.
`mlm status` lists them under `Notes:` (§7.1), and `mlm week --notes`
lists them under the date's row (§7.2). Both commands share the
glossary, so `mlm status --help` lists it too. The `[i]` entry, last,
ends ``It marks yesterday's row in `mlm week` until that start is stopped.``
```

The colon-led clauses in the unchanged lines are existing text, left alone.

#### S2. §3.6 heading (line 325)

Current:

```
### 3.6 `mlm week [WEEK_ID]`
```

Replacement:

```
### 3.6 `mlm week [WEEK_ID] [--notes|-n]`
```

#### S3. §3.6 gains a notes bullet (lines 333-338)

Current:

```
- yesterday's row ends with `[i]` while a start from yesterday is still
  open and unjoined, with an `[i]` footer line, and on a Monday the
  current week's view names Sunday's stint in that footer line (§7.2)
- a trailing block, in this order: carry-in, worked, fulfillment,
```

Replacement:

```
- yesterday's row ends with `[i]` while a start from yesterday is still
  open and unjoined, with an `[i]` footer line, and on a Monday the
  current week's view names Sunday's stint in that footer line (§7.2)
- with `--notes`/`-n`, each date's notes on the lines directly under
  its row (§7.2)
- a trailing block, in this order: carry-in, worked, fulfillment,
```

#### S4. §3.6 gains the flag rules (after line 342)

Anchor (unchanged, lines 340-342):

```
`WEEK_ID` accepts either a full id (`YYYY-WW`, e.g. `2026-07`) or a
bare week number (`WW`, e.g. `7`), which defaults the year to the
current one.
```

Insert after it, with one blank line before (the blank before `### 3.7` stays):

```
`--notes`/`-n` works with or without `WEEK_ID`, in either order (`mlm
week -n 43`, `mlm week 43 -n`). `mlm week --help` lists it as `Show
each date's notes under its row`. The flag belongs to the view only,
so `week target` (§3.7) does not take it. After `target`, `--notes`
and `-n` are ordinary tokens in target's own argument list. `mlm week
target 40h --notes` fails as an invalid `WEEK_ID` or `DURATION` with
exit `1`, and `mlm week target 43 40h --notes` is a usage error with
exit `2`. Neither writes a target.
```

The help line is split across two source lines here. Check 2.7 greps for it with the line break in mind.

#### S5. §6.1 read-failure bullet (lines 640-645)

Current:

```
- A storage read failure in `status` or `mlm week`, on any of the
  reads they make (the earliest data week, the target overrides, the
  punches behind the carry walk). The command prints `error:` and the
  storage error text to stderr, prints nothing to stdout, and exits `1`.
  `mlm week` never falls back to the default target or to "no earlier
  weeks".
```

Replacement:

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

#### S6. §7.2 gains the notes bullet (before line 1157)

Anchor (unchanged, lines 1157-1160, the section's last bullet):

```
- Every storage read `mlm week` makes is a hard error, as in `status`
  (§6.1). The storage error goes to stderr as `error: ...`, stdout stays
  empty, and the exit code is `1`. `mlm week` never falls back to the
  default 40h target or to "no earlier weeks".
```

Insert directly before it, with no blank line between the new bullet's last line and the anchor:

````
- Notes (`--notes`/`-n`, §3.6). Each of a date's notes prints on its
  own line directly under that date's row, before the next row, as six
  spaces, `- `, then the body as stored. No `Notes:` header and no blank
  line surround a row's notes. A date shows the notes `status DATE`
  lists, in the same insertion order (`created_at_utc ASC, id ASC`,
  §2.3). Only the layout differs, since `status` prints a `Notes:`
  header and two-space bullets (§7.1). A note belongs to its stored
  `date`, so stint pairing and the §4.3.1 splice never move it. Row
  markers stay at the end of the row, so a marked row reads marker
  first, notes below. Blank lines, footers, the carry-in marker line and
  the Carry-in block are unchanged. An illustrative current week, today
  Sat 2026-10-24, one marked row, `mlm week --notes`:

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

  A note-only date keeps its `00h 00m` row, with its notes under it.
  Notes never change a total, a marker, a footer, the headline or the
  Carry-in block. A week with no notes prints exactly what plain `mlm
  week` prints, and plain `mlm week` prints no note lines. Note bodies
  pass through unchanged, non-ASCII included, as in `status` (§2.3,
  §7). With the flag, `mlm week` makes one ranged note read over the
  shown week's seven dates, Monday to Sunday, and a failed read is a
  hard error like every other `week` read (next bullet, §6.1). Without
  the flag it makes no note read.
````

The inner fence sits at column 0 and the continuation text is indented two spaces, as the `[!]` footer and carry-in marker bullets above already do (lines 1076-1086, 1133-1149).

#### S7. F14 after F13 (after line 1511)

Anchor (unchanged, lines 1509-1511):

```
- **F13** - First run on a fresh database prints nothing on stderr,
  except that a first `mlm stop` warns about its orphaned end (§7.4,
  E27). With `--verbose` the migration line and debug lines appear (§3).
```

Insert directly after it, no blank line (the blank before `### 8.2` stays):

```
- **F14** - Week notes (§3.6, §7.2): `mlm note "fixed migration runner
  bug"`, then `mlm note "started punch pairing tests"` on one date, then
  `mlm week --notes` for its week prints both under that date's row, in
  that order, each as six spaces, `- ` and the body. Other rows stay one
  line, and every figure, marker and footer matches plain `mlm week`.
  `mlm week -n WEEK_ID` and `mlm week WEEK_ID --notes` print the same.
  Plain `mlm week` prints no note lines.
```

#### S8. E20 gains the note read (lines 1601-1605)

Current:

```
- **E20** - `mlm week` read failure (§6.1, §7.2): an unreadable
  `week_targets` table, `notes` table, or punch range makes `mlm week`
  and `mlm week WEEK_ID` print `error: ...` to stderr, nothing to
  stdout, and exit `1`. `status` does the same. Successful output is
  unchanged.
```

Replacement:

```
- **E20** - `mlm week` read failure (§6.1, §7.2): an unreadable
  `week_targets` table, `notes` table, or punch range makes `mlm week`
  and `mlm week WEEK_ID` print `error: ...` to stderr, nothing to
  stdout, and exit `1`. `status` does the same. Successful output is
  unchanged. `mlm week --notes` also reads the shown week's notes, so a
  renamed `notes.body` column fails it the same way. Plain `mlm week`
  on the same database still prints, because it makes no note read.
```

### README.md

#### R1. Features line (lines 16-17)

Current:

```
- `week` — a week's per-day totals plus carry-in/fulfillment against
  target
```

Replacement:

```
- `week` — a week's per-day totals plus carry-in/fulfillment against
  target, with each date's notes under its row on `-n/--notes`
```

#### R2. `mlm week` heading (line 409)

Current:

```
### `mlm week|w [WEEK_ID]`
```

Replacement:

```
### `mlm week|w [WEEK_ID] [-n/--notes]`
```

#### R3. `mlm week` gains the flag and the example (after line 484)

Anchor (unchanged, lines 480-484):

```
With one affected week the line reads `from 1 earlier week, 2026-38 (see
mlm week 2026-38)`. The line also prints on `status` (see above). It
stays until the old anomaly is fixed. If `mlm week` cannot read the
database, it prints an `error:` line and exits 1 instead of showing
default figures.
```

Insert after it, with one blank line before and after (the blank before `### mlm week|w target` stays):

````
Add `-n/--notes` to list each date's notes under its row. They are the
notes `status` shows for that date, in the same order, each printed as
six spaces, `- ` and the note. Rows without notes stay one line. Every
total, marker, footer and the Carry-in block print as they do without
the flag, and a week with no notes prints the same either way. The flag
goes before or after `WEEK_ID` (`mlm week -n 43`, `mlm week 43 -n`),
and `week target` does not take it. With today Sat 2026-10-24:

```sh
$ mlm week --notes
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

If `mlm week -n/--notes` cannot read the week's notes, it prints an
`error:` line and exits 1, and plain `mlm week` still works.
````

### AGENTS.md

#### A1. `src/storage.rs` bullet (lines 36-37)

Current:

```
- `src/storage.rs` — `Punch`/`PunchKind`/`Note`, punch/note insert+read,
  one ranged week-target-override read
```

Replacement:

```
- `src/storage.rs` — `Punch`/`PunchKind`/`Note`, punch/note insert+read,
  one ranged week-target-override read, and one ranged note read
  (`notes_in_range`, an inclusive date range in, a flat `Vec<Note>`
  out, ascending by date, insertion order within a date) that
  `week --notes` calls
```

#### A2. `src/history.rs` bullet (lines 46-50)

Current:

```
  ledger and the bucketed punches for `status` and `mlm week`. Any read
  failure is an error. `stop`'s orphaned-end warning also uses its
  `PunchBuckets` view, over a three-date punch read of its own.
```

Replacement:

```
  ledger and the bucketed punches for `status` and `mlm week`. Any read
  failure is an error. `stop`'s orphaned-end warning also uses its
  `PunchBuckets` view, over a three-date punch read of its own.
  `mlm week --notes` makes one ranged note read of its own, outside the
  loader.
```

## Test plan

Run every command from `/home/magikmw/projects/mlm-cli`. This task touches no Rust, so `cargo fmt`, `clippy` and coverage see no change, and no test reads doc text. Each subsection maps to one acceptance criterion of the changeset plan's Task 2 and cites the spec case it carries.

### 2.1 SPEC §3.6 heading and body (spec §3)

- `grep -n '^### 3.6 `mlm week \[WEEK_ID\] \[--notes|-n\]`$' docs/dev/SPEC.md` gives one hit.
- `grep -n "in either order (\`mlm" docs/dev/SPEC.md` gives one hit in §3.6.
- `grep -n "so \`week target\` (§3.7) does not take it" docs/dev/SPEC.md` gives one hit.
- Read §3.6. The notes bullet sits between the `[i]` bullet and the trailing-block bullet, and the flag paragraph names both exit codes (`1`, `2`) and "Neither writes a target".

### 2.2 SPEC §7.2 layout, rules and example (spec §4, §5; contracts 3, 4, 6)

- `grep -n "^- Notes (\`--notes\`/\`-n\`, §3.6)" docs/dev/SPEC.md` gives one hit, after the carry-in marker bullet and directly before `- Every storage read \`mlm week\` makes`.
- `grep -n "created_at_utc ASC, id ASC" docs/dev/SPEC.md` hits inside the new bullet.
- `grep -n "never move it\|Notes never change a total\|prints exactly what plain" docs/dev/SPEC.md` gives one hit each, in the new bullet.
- Block diff (contract 6). Expected: no output.

```sh
bash -c 'ex() { awk "/^Week 2026-43 /{p=1} p{print} p&&/^Target:/{exit}" "$1"; }; diff <(ex docs/dev/specs/2026-10-24-week-notes.md) <(ex docs/dev/SPEC.md)'
```

### 2.3 SPEC §6.1 and E20 add the note read (spec §6; spec §7 "`notes.body` column renamed")

- `grep -n "for \`mlm week --notes\` the shown" docs/dev/SPEC.md` gives one hit in §6.1.
- `grep -n "makes no note read" docs/dev/SPEC.md` gives three hits: §6.1, §7.2's notes bullet, E20.
- `grep -n "renamed \`notes.body\` column" docs/dev/SPEC.md` gives one hit, in E20.

### 2.4 SPEC §8.1 gains F14 (spec §7 happy-path cases)

- `grep -n "^- \*\*F14\*\* - Week notes" docs/dev/SPEC.md` gives one hit, directly after F13 and before `### 8.2`.

### 2.5 README `mlm week` section and example; possible issues unchanged (spec §8; contracts 6, 8)

- `grep -n '^### `mlm week|w \[WEEK_ID\] \[-n/--notes\]`$' README.md` gives one hit.
- `grep -c -- '--notes|-n' README.md` gives 0 (contract 8 spelling only).
- `grep -n '^\$ mlm week --notes$' README.md` gives one hit, after the carry-in paragraph and before `### \`mlm week|w target`.
- Block diff, as in 2.2, with `README.md` in place of `docs/dev/SPEC.md`. Expected: no output.
- `git diff -U0 README.md` shows no hunk between `### Possible issues` and `## Install`.

### 2.6 AGENTS.md layout bullets (spec "Interaction with the project spec")

- `grep -n "(\`notes_in_range\`, an inclusive date range in" AGENTS.md` gives one hit in the `src/storage.rs` bullet.
- After Task 1 merges, `grep -n "pub fn notes_in_range" src/storage.rs` gives one hit, so the name AGENTS.md cites exists.
- `grep -n "makes one ranged note read of its own, outside the" AGENTS.md` gives one hit in the `src/history.rs` bullet.

### 2.7 Quoted lines match contracts 2 and 3 byte for byte (spec §3, §5)

`-F` matches literal text, spaces included.

```sh
grep -cF '      - fixed migration runner bug' README.md docs/dev/SPEC.md
grep -cF '      - started punch pairing tests' README.md docs/dev/SPEC.md
grep -cF '      - code review for #41' README.md docs/dev/SPEC.md
grep -cF '  Wed 2026-10-21   07h 24m  [!]' README.md docs/dev/SPEC.md
```

Expected count is 1 per file for each line. The two-space `  - fixed migration runner bug` lines in SPEC §7.1 and README's `status` example do not contain six leading spaces, so they do not match.

The help line wraps in SPEC §3.6, so join lines before grepping:

```sh
tr '\n' ' ' < docs/dev/SPEC.md | grep -cF "lists it as \`Show each date's notes under its row\`"
```

Expected: 1. If Task 1 has merged, also confirm `mlm week --help` prints `Show each date's notes under its row` on the `-n, --notes` line.

### 2.8 SPEC §3.5 glossary sentence (spec §8 glossary entry; H1)

- `grep -n "^Three entries precede the \`\[!\], \[i\]\` one" docs/dev/SPEC.md` gives one hit, in §3.5.
- `grep -n "third is \`- <note>\`" docs/dev/SPEC.md` gives one hit, in §3.5.
- After Task 1 merges, `GLOSSARY` in `src/cli.rs` has `- <note>` third before `[!], [i]`, and `mlm status --help` shows it, as S1 states.

### 2.9 Binary check at merge (contract 6)

If Task 1 has merged before this task's review, seed a scratch DB with the example's notes on a past week and run `TZ=UTC MLM_DB_PATH=<scratch> cargo run -- week <WEEK_ID> --notes`. The note lines and their position under rows must match the pinned block's shape. The headline and totals will differ, since the example is a current-week illustration.

### 2.10 No known-issue text and no pointer out to working docs

- `git diff -U0 docs/dev/SPEC.md` shows no hunk between `### 1.2a` and `### 1.3`.
- `grep -n "docs/dev/specs\|docs/dev/plans\|week-notes" README.md docs/dev/SPEC.md AGENTS.md` gives no new hit.
- Every section reference in new text is a SPEC.md section (§2.3, §3.6, §3.7, §4.3.1, §6.1, §7, §7.1, §7.2, E20). None is §1.2a.

## Risks, ambiguities and disagreements

- **§3.5 sentence.** Its glossary sentence counts entries and calls `[i]` last, so the new entry falsifies it. The changeset plan's Task 2 criteria now cover it, and Task 1 step 1.2 pins the slot, so S1 has one replacement. Task 1's plan calls this step "S2 variant A". It means S1 here.
- **README Features line (R1).** Outside the plan's acceptance list, but the Features list summarizes `week` and would otherwise omit the flag. One line, no new mechanism. Drop R1 if the coordinator wants the AC list exact.
- **"Short example" versus "verbatim".** Task 2's acceptance criterion says README gets "a short example taken from contract 6", while contract 6 says README's example is spec §5's block verbatim. This plan follows the contract and quotes the full block, which also lets check 2.5 diff it.
- **Task 1 plan cross-checked, not yet implemented.** The help line, the six-space `- ` prefix (`NOTE_PREFIX`), the glossary term and slot, `notes_in_range` and its shape, and the `src/history.rs` doc sentence all match this plan. If the implementation deviates, checks 2.6-2.9 catch it at merge.
- **Stale bullet in Task 1's risks.** Task 1's "Spec §1.2 dropped" bullet says this plan still adds §1.2a and README known-issue bullets. It does not (see Scope). No action here.
