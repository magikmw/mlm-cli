# Task plan: relative-ids, Task 2 (docs)

Changeset plan: `docs/dev/plans/relative-ids-plan.md`, "Task 2: docs".
Spec: `docs/dev/specs/2026-10-25-relative-ids.md` (locked).
Branch: `relative-ids` at `13e0f80`.

## Read for this plan

- `docs/dev/plans/relative-ids-plan.md` in full (contracts 1-9, Task 1 and Task 2 sections, risks)
- `docs/dev/specs/2026-10-25-relative-ids.md` in full
- `docs/dev/plans/reports/relative-ids-spec-review.md` in full (F4's stale-line list)
- `docs/dev/plans/reports/relative-ids-plan-review.md` in full
- `docs/dev/NOTES.md` entries 149-154 (lines 2046-2111)
- `docs/dev/SPEC.md`: lines 1-500 (§1 through §4.1), 700-849 (§6), 1655-1963 (§7.5, §8 in full), plus a grep of every `WEEK_ID`/`MONTH`/`-N` mention
- `README.md` in full (691 lines)
- `AGENTS.md` in full (176 lines)
- `src/date.rs` in full (1416 lines)
- `src/cli.rs` lines 1-520 in full (definitions, help, classifier, swap detector, `split`), and the test-function list for 520-1086
- `src/time.rs:164-176` (the `invalid duration` text)
- `.githooks/pre-commit` in full
- `docs/dev/plans/export-notes-task-2-docs.md` lines 1-80 (shape reference)
- Sibling task plans: none exist. `docs/dev/plans/relative-ids-task-1-code.md` (or any `relative-ids-task-1*`) was absent when this plan was written. Every handoff below goes through the changeset plan's contracts.

## Scope

Owns exclusively:

- `docs/dev/SPEC.md`
- `README.md`
- `AGENTS.md` (checked, no edit, see §3)

Must not touch:

| File | Owner |
|---|---|
| `src/date.rs`, `src/cli.rs`, `src/export.rs`, `src/week_target.rs`, `src/week_view.rs`, `src/main.rs`, `src/lib.rs`, every other `src/*` | Task 1 |
| `tests/week_target_cli.rs`, `tests/export_notes_cli.rs`, `tests/relative_ids_cli.rs` (new), every other `tests/*` | Task 1 |
| `coverage-baseline.json` | Task 1 (the hook ratchets it) |
| `docs/dev/NOTES.md`, archive banners on the spec and plans | coordinator |
| this plan, the changeset plan, the spec | coordinator |

Handoffs, all against the changeset plan's contracts. No Task 1 work is planned here.

- Every message quoted below is contract 2, 3 or 4 verbatim. Task 1 prints them.
- Every help line quoted below is contract 5 verbatim. Task 1 ships them in `--help`.
- The `[WEEK_ID]` line quoted in §3.7 as a rendered help row assumes Task 1 keeps the current hand-written padding, two spaces, `[WEEK_ID]`, three spaces (`src/cli.rs:278`). Contract 5 pins only the text after the name. See Risks R2.
- Exit codes come from contract 6. Flag order comes from contract 7. The `week target` shapes come from contract 8.
- The month floor example (`-m -24313` is `0000-01` from February 2026, `-24314` is out of range) comes from Task 1's acceptance criteria in the changeset plan.

## Facts gathered

### Code facts behind the docs

- The current `WEEK_ID` and `MONTH` shape texts are `expected YYYY-WW or WW` and `expected YYYY-MM or MM` (`src/date.rs:79-80`). Contract 2 replaces both.
- The `WEEK_ID` out-of-range arm prints `week number must be 1 or greater` (`src/date.rs:83`). Contract 3's offset text is new and separate. SPEC quotes neither the week-0 text nor the `has only N ISO weeks` text today, and this plan adds neither.
- `DATE`'s `-N` resolver is the grammar model: `-` plus ASCII digits, a `u64` overflow is out of range, `-0` is a shape error (`src/date.rs:132-162`). `DATE`'s shape text is `expected YYYY-MM-DD or -N (N >= 1)` (`src/date.rs:78`).
- `WeekId` holds ISO years `1000..=9999` (`src/date.rs:215-216,231-232,243`). `Month` holds years `0..=9999` (`src/date.rs:401-402,412`). These are the two floors, `1000-01` and `0000-01`.
- `status`'s `DATE` positional uses `allow_negative_numbers` (`src/cli.rs:103`). `mlm week`'s positional gets the same setting (contract 6).
- Current help strings: `mlm week` `[WEEK_ID]` (`src/cli.rs:241-242`), `week target` hand-written Arguments block (`src/cli.rs:277-279`), `export notes` `-w`/`-m` (`src/cli.rs:324,328-329`). All four change to contract 5.
- The swapped reminder is `error: put WEEK_ID after target: mlm week target {week} {duration}`, echoing tokens as typed (`src/cli.rs:434-436`). Its week check calls `parse_week_id` (`src/cli.rs:443-445`), so `-1` qualifies once Task 1 lands.
- With one token, `week target` reads it as `DURATION` today (`src/cli.rs:460-462`).
- The lone-token fallback message is `invalid duration "<token>": expected a form like 20h, 33h30m, or 45m` (`src/time.rs:168-170`), printed with the `error: ` prefix, exit `1`.
- `u64::MAX` is `18446744073709551615` (20 digits), so `-99999999999999999999` (20 nines) overflows the digit parse. The plan's Risks section uses this exact token.

### Calendar facts used in examples

All from the changeset plan's contract 1, from `src/date.rs` tests, or from SPEC itself.

| Fact | Source |
|---|---|
| 2026-01-05 is in week 2026-02; `-2` is `2025-52` | contract 1; `src/date.rs:803` (T71) |
| 2027-01-04 is in week 2027-01; `-1` is `2026-53`, `-2` is `2026-52` | contract 1; `src/date.rs:808-809` (T77, T78) |
| 2025 has 52 ISO weeks, 2026 has 53 | `src/date.rs:633,642` (T30, T35) |
| In January 2026, `-m -1` is `2025-12` | contract 1 |
| Week 2026-43 is 2026-10-19 to 2026-10-25, so 2026-42 starts 2026-10-12 | SPEC §7.5 example (SPEC:1670-1674); README:510 |
| Week 2026-37 is 2026-09-07 to 2026-09-13, 2026-35 is 2026-08-24 to 2026-08-30, so 2026-36 is 2026-08-31 to 2026-09-06 | README:432, README:592 |
| From February 2026, `-m -24313` is `0000-01` (2026 x 12 + 1) | changeset plan Task 1 AC; plan review "What checks out" |

### Doc locations that go stale (spec review F4, extended)

| File:line | Today | Becomes |
|---|---|---|
| SPEC:49-52 | `+N`/`-N` relative week notation is a non-goal | `+N`-only non-goal |
| SPEC:363-365 | `WEEK_ID` takes `YYYY-WW` or `WW` | adds `-N` |
| SPEC:367-374 | `--notes` order examples | adds `mlm week -1 -n` |
| SPEC:378-379 | `WEEK_ID` "same accepted forms as §3.6" | still true, gains "`-N` included" |
| SPEC:422-428 | reminder echo, normalized echo, help lead-in | gains `-1` examples and the `[WEEK_ID]` help row |
| SPEC:434 | end of §3.7 | lone-`WEEK_ID` paragraph follows |
| SPEC:448-456 | `-w`/`-m` forms | add `-N` |
| SPEC:464-468 | "`-w -1` and `-m -1` reach their own ... errors" | false, rewritten |
| SPEC:485-486 | old `-w`/`-m` help lines | contract 5 |
| SPEC:736-742 | `WEEK_ID` bullet, no message quoted | contracts 2-4 for weeks |
| SPEC:743-753 | `MONTH` bullet, old shape message | contracts 2-4 for months |
| SPEC:1771 | end of F15 | F16, F17 follow |
| SPEC:1806-1808 | E10 zero-token only | adds the lone-`WEEK_ID` case and its boundary |
| SPEC:1889-1892 | E24 | gains the `-1` reminder |
| SPEC:1924-1931 | E29 says `-m -1`, `-w -1` fail; old shape text | rewritten |
| SPEC:1962 | end of E34 | E35-E38 follow |
| README:42-43 | `+N`/`-N` Planned bullet | `+N`-only |
| README:425-428 | `mlm week` forms | adds `-N` with example |
| README:504-506 | `-n` order examples | adds `mlm week -1 -n` |
| README:539-540 | `week target` forms | "`-N` included" |
| README:566 | after the echo note | `-N` example and lone-token rule |
| README:582-584 | swapped reminder block | gains the `-1` case |
| README:618-621 | `-w`/`-m` forms | add `-N` |
| README:638-640 | redirect example | `-m -1` example follows |

`grep -n 'expected YYYY-WW or WW\|expected YYYY-MM or MM' docs/dev/SPEC.md README.md` finds exactly SPEC:746 and SPEC:1926, both replaced below. README quotes neither.

`grep -n 'E3[5-9]\|F1[6-9]' docs/dev/SPEC.md` is empty, so F16, F17 and E35-E38 are free ids.

`grep -rn 'specs/\|plans/\|relative-ids' README.md docs/dev/SPEC.md` is empty today and must stay empty.

## Editing rules for the implementer

- Copy the new text below exactly. Do not reword it.
- Keep every quoted message, help line and command example on one source line, unwrapped, even past 72 columns. SPEC already does this (SPEC:431, SPEC:484). Wrap other prose at 72 columns like its neighbours. The blocks below are already wrapped, and no new backtick span breaks across a line, so every quoted command can be found with one `grep -F`.
- Where a block below contains its own fenced code block, this plan quotes it inside a four-backtick fence. The file gets ordinary three-backtick fences.
- New text is plain ASCII apart from `§` and the em dash `—` where the neighbouring README/SPEC lines already use it.
- No pointer to `docs/dev/specs/`, `docs/dev/plans/` or the changeset name in either file.
- Line numbers refer to the files at `13e0f80`. Apply the edits bottom-up within each file, so earlier edits do not shift later anchors. The sections below are listed top-down for reading. Each edit also names an anchor string so it can be found after a shift.
- A "replace lines A-B" edit replaces those lines in full, and the new text supplies every line, the last included. A blank line before or after the range stays.

## 1. `docs/dev/SPEC.md`

Apply in this order: 1.13, 1.12, 1.11, 1.10, 1.9, 1.8, 1.7, 1.6, 1.5, 1.4, 1.3, 1.2, 1.1.

### 1.1 §1.2 non-goals (lines 49-52)

Anchor: ``- `+N`/`-N` relative week notation for `week`'s week-id argument. (This``.

Replace lines 49-52 with:

```
- `+N` relative notation for future weeks and months. `WEEK_ID` and
  `MONTH` take `-N` for past ones (§3.6, §3.8). A future week takes
  `YYYY-WW` or `WW`, and a future month takes `YYYY-MM` or `MM`.
```

### 1.2 §3.6 `WEEK_ID` forms and the parser rule (lines 363-365)

Anchor: `` `WEEK_ID` accepts either a full id (`YYYY-WW`, e.g. `2026-07`) or a ``.

Replace lines 363-365 with these two paragraphs (one blank line between them):

```
`WEEK_ID` accepts a full id (`YYYY-WW`, e.g. `2026-07`), a bare week
number (`WW`, e.g. `7`), which defaults the year to the current one,
or `-N` for N ISO weeks before the current one (`-1` = last week). The
current week is the ISO week of today's local date. `-N` has the
grammar of `DATE`'s `-N` (§3.4a). It is `-` followed by one or more
ASCII digits, N is at least `1`, and leading zeros are tolerated
(`-01` = `-1`, `-07` is seven weeks ago), so `-0` is rejected. `-N`
crosses year boundaries and week 53. On 2026-01-05 (week 2026-02),
`-2` is `2025-52`. On 2027-01-04 (week 2027-01), `-1` is `2026-53` and
`-2` is `2026-52`. The earliest week `-N` reaches is `1000-01` (§6.1).
There is no `+N` (§1.2), so a future week takes `YYYY-WW` or `WW`.
`mlm week --help` lists `WEEK_ID` as this:
``Week to show: YYYY-WW (e.g. 2026-07), a bare week number (e.g. 7), or `-N` for N weeks before this one (e.g. `-1` = last week). Defaults to the current week``.

`mlm week` takes `-1` as `WEEK_ID`, as `mlm status -1` takes it as
`DATE` (§3.5). The argument parser passes each token it reads as a
negative number (`-1`, `-1.5`, `-1e3`) on to the `WEEK_ID` parser.
Only `-` plus digits resolves, so `mlm week -1.5` fails with the
`WEEK_ID` shape error and exit `1` (§6.1). Any other token starting
with `-` (`-1x`, `-x`, `--bogus`) stays a usage error from the
argument parser, exit `2` (§6.3), as `mlm status -1x` is.
`week target` (§3.7) and `export notes` (§3.8) pass every value
starting with `-` to their parsers, so `-1x` there gets the shape
error and exit `1`.
```

The help line is the contract 5 `mlm week` text inside a double-backtick span, on its own source line, with the sentence's period after the closing backticks.

### 1.3 §3.6 `--notes` order (lines 367-374)

Anchor: `` `--notes`/`-n` works with or without `WEEK_ID`, in either order (`mlm ``.

Replace lines 367-374 with:

```
`--notes`/`-n` works with or without `WEEK_ID`, in either order (`mlm
week -n 43`, `mlm week 43 -n`). A `-N` works the same, so
`mlm week -1 -n` and `mlm week -n -1` both show last week with its
notes. `mlm week --help` lists the flag as `Show each date's notes
under its row`. The flag belongs to the view only, so `week target`
(§3.7) does not take it. After `target`, `--notes` and `-n` are
ordinary tokens in target's own argument list. `mlm week target 40h
--notes` fails as an invalid `WEEK_ID` or `DURATION` with exit `1`,
and `mlm week target 43 40h --notes` is a usage error with exit `2`.
Neither writes a target.
```

The first line and the two existing split spans (`mlm week target 40h --notes`, `Show each date's notes under its row`) keep their current breaks. Only the new `-N` sentence follows the one-line-span rule. "it" becomes "the flag" because the new sentence sits between the old one and its pronoun.

### 1.4 §3.7 intro (lines 378-382)

Anchor: `` Set an absolute target override for a week. `WEEK_ID` (same accepted ``.

Replace lines 378-382 with:

```
Set an absolute target override for a week. `WEEK_ID` (same accepted
forms as §3.6, `-N` included) defaults to the current week when
omitted. `DURATION` uses the human duration format (§4.2), not raw
minutes — e.g. `20h`, `33h30m` — so setting a target has the same
friction as everything else here: no mental-math input.
```

Only ``, `-N` included`` is new. The rest is the old text re-wrapped.

### 1.5 §3.7 reminder echo, resolved echo, `[WEEK_ID]` help row (lines 422-428)

Anchor: ``The line echoes the week id and duration as typed, always spells `mlm``.

Replace lines 422-428 with:

````
The line echoes the week id and duration as typed, always spells `mlm
week target` even when the user typed the `w` alias, and prints no
`Usage:` block. So `mlm week -1 target 35h` prints this:

```
error: put WEEK_ID after target: mlm week target -1 35h
```

The success echo above uses the normalized week id, so
`mlm week target 2026-7 33h30m` prints `Week 2026-07 target: 33h 30m`.
A `-N` echoes the week it resolved to, so with today in week 2026-43
`mlm week target -1 35h` prints `Week 2026-42 target: 35h 00m` and
leaves week 2026-43's target as it was. `mlm week target --help` shows
the usage `mlm week target [OPTIONS] [WEEK_ID] <DURATION>` and an
Arguments section naming both arguments. Its `WEEK_ID` line names the
`-N` form:

```
  [WEEK_ID]   Week to set: YYYY-WW (e.g. 2026-07), a bare week number (e.g. 7), or `-N` for N weeks before this one (e.g. `-1` = last week). Defaults to the current week
```

Its `DURATION` line names the cap:
````

Lines 429-432 (blank line, then the existing `<DURATION>` code block) follow unchanged. The first two lines keep the old `mlm`/`week target` break.

### 1.6 §3.7 lone `WEEK_ID` (after line 434)

Anchor: `` No parser error for this subcommand mentions `<ARGS>`. ``

Insert after line 434, with one blank line before the new text (line 435 is the blank line before `### 3.8`, which stays):

````

With one token, `mlm week target` reads it as `DURATION`, unless the
token parses as a `WEEK_ID` (§3.6). Then `DURATION` is missing.
`mlm week target -1`, `mlm week target 7` and
`mlm week target 2026-07` print the same usage error as bare
`mlm week target`, byte for byte, and exit `2` (§6.3). Stdout stays
empty and no target is written. The `w` alias, `-v` before or after
`target`, and a `--` before the token give the same error. It is a
parse failure, so like the reminder it comes before every check. A
bare number is a `WEEK_ID`, so `mlm week target 40`, missing its `h`,
gets this error too.

Only a token that parses as a `WEEK_ID` gives the usage error. Any
other lone token keeps the `DURATION` error and exit `1`, including
`-0`, an out-of-range `-N` such as `-99999999999999999999`, `0`, and
`53` in a 52-week year. `mlm week target -0` prints this:

```
error: invalid duration "-0": expected a form like 20h, 33h30m, or 45m
```

`mlm week target 35h` still sets this week's target.
````

### 1.7 §3.8 range bullets and the `-N` paragraph (lines 448-456)

Anchor: `` - `--week`/`-w WEEK_ID`: Monday to Sunday of one ISO week, in the ``.

Replace lines 448-456 with:

```
- `--week`/`-w WEEK_ID`: Monday to Sunday of one ISO week, in the
  `WEEK_ID` forms of §3.6, with a bare `WW` in the current ISO year
  and `-N` counted back from the current ISO week
- `--month`/`-m MONTH`: the first to the last calendar day of one
  month. `MONTH` is `YYYY-MM` (e.g. `2026-03`), `YYYY-M` (`2026-3`, the
  same month), a bare `M`/`MM` (e.g. `3`) in the current calendar
  year, or `-N` for N calendar months before the current one. The
  calendar year is not the ISO year, so on 2025-12-30 `-m 1` is
  `2025-01`. The year is exactly four digits, and any value from
  `0000` to `9999` is accepted. February runs to the 28th or the 29th
  by the calendar.

`-N` has the grammar of §3.6 for both flags, so `-0` is rejected and
leading zeros are tolerated. The current week and month come from
today's local date, and `-N` crosses year boundaries. On 2026-01-05
(week 2026-02), `-w -2` is `2025-52`, and in January 2026 `-m -1` is
`2025-12`, December 1 to 31. The earliest month `-m -N` reaches is
`0000-01` (§6.1).
```

Line 457 (blank) and line 458 (`At most one range flag is given. ...`) follow unchanged.

### 1.8 §3.8 hyphen values (lines 464-468)

Anchor: `` All three flags accept values that start with `-`. So `-d -1` is ``.

Replace lines 464-468 with:

```
All three flags accept values that start with `-`. So `-d -1` is
yesterday, `-w -1` is last week and `-m -1` is last month. Any other
value starting with `-` reaches the flag's own parser and fails with
exit `1` (§6.1), never with a parser error, so `-w -1x` and `-m -1x`
get the `WEEK_ID` and `MONTH` shape errors. A flag followed by another
flag takes that flag as its value, so `-m -d` fails as a bad `MONTH`.
```

### 1.9 §3.8 help lines (lines 485-486)

Anchor: `` - `-w, --week <WEEK_ID>`: `Week to export: ``.

Replace lines 485-486 with:

```
- `-w, --week <WEEK_ID>`: ``Week to export: YYYY-WW (e.g. 2026-07), a bare week number (e.g. 7), or `-N` for N weeks before this one (e.g. `-1` = last week)``
- `-m, --month <MONTH>`: ``Month to export: YYYY-MM (e.g. 2026-03), a bare month number (e.g. 3), or `-N` for N months before this one (e.g. `-1` = last month)``
```

Line 484's `-d` line already uses the double-backtick form, so these match it.

### 1.10 §6.1 `WEEK_ID` and `MONTH` bullets (lines 736-753)

Anchor: `` - Malformed `WEEK_ID` (§3.6): not a bare week number or `YYYY-WW` ``.

Replace lines 736-753 (both bullets, through `  any four-digit year is accepted.`) with:

```
- Malformed `WEEK_ID` (§3.6): not a bare week number, a `YYYY-WW`
  shape or a `-N`, or a week number that isn't a valid ISO week for
  its year (most years have 52, some have 53 — `2027-53` is invalid if
  2027 only has 52; this is a calendar-validity check, not a flat
  `1..=53` range test). An unpadded number is accepted and normalized
  (`2026-7` parses the same as `2026-07`), consistent with how
  `TIME`/`DURATION` input is lenient about padding elsewhere. A bad
  shape, `-0` included, prints this:

  `invalid WEEK_ID "<input>": expected YYYY-WW, WW or -N (N >= 1)`

  A `-N` that would land before week `1000-01`, the earliest week the
  absolute form accepts, prints this. So does a `-N` whose digit run is
  too large for a `u64`, as for `DATE`:

  `invalid WEEK_ID "<input>": week offset is out of range`

  A `+` followed by one or more digits (`+1`, `+0`) gets the shape
  message plus a hint, since `-N` reaches only past weeks. `+1` prints
  this:

  `invalid WEEK_ID "+1": expected YYYY-WW, WW or -N (N >= 1); a future week takes YYYY-WW or WW`

  Any other value starting with `+` (`+1x`, `+`) gets the plain shape
  message. `<input>` is the value as typed.
- Malformed `MONTH` (§3.8). A value that is not `YYYY-MM`, `YYYY-M`, a
  bare `M`/`MM` or a `-N`, `-0` included, prints this:

  `invalid MONTH "<input>": expected YYYY-MM, MM or -N (N >= 1)`

  A month number of `0` or above `12` prints this:

  `invalid MONTH "<input>": month must be 1-12`

  A `-N` that would land before `0000-01`, the earliest month the
  absolute form accepts, prints this. So does a `-N` whose digit run is
  too large for a `u64`:

  `invalid MONTH "<input>": month offset is out of range`

  A `+` followed by one or more digits (`+1`, `+0`) gets the shape
  message plus a hint, since `-N` reaches only past months. `+1` prints
  this:

  `invalid MONTH "+1": expected YYYY-MM, MM or -N (N >= 1); a future month takes YYYY-MM or MM`

  Any other value starting with `+` (`+1x`, `+`) gets the plain shape
  message. `<input>` is the value as typed. The year is exactly four
  digits, and any four-digit year is accepted.
```

The `DATE` bullet at lines 724-726 stays as it is. `DATE` gets no `+` hint (spec §1.1).

### 1.11 §8.1 happy paths (after line 1771)

Anchor: `` same as the long form, and a non-ASCII body passes through unchanged. `` (end of F15).

Insert after line 1771, before the blank line 1772:

```
- **F16** - Relative weeks (§3.6, §3.7): on 2026-01-05 (week 2026-02),
  `mlm week -1` shows week 2026-01, `mlm week -01` the same, and
  `mlm week -2` week 2025-52. On 2027-01-04 (week 2027-01), `-1` shows
  2026-53 and `-2` shows 2026-52. `mlm week -1 -n` and `mlm week -n -1`
  both show last week with its notes. `mlm week target -1 35h` writes
  last week's target and echoes last week's id, so on 2026-01-05 it
  prints `Week 2026-01 target: 35h 00m`. A target already set for the
  current week stays.
- **F17** - Relative export ranges (§3.8): `mlm ex n -w -1` prints last
  week's notes, Monday to Sunday, and `mlm ex n -m -1` last month's.
  Neither prints a note from outside its range. In January 2026 `-m -1`
  covers 2025-12-01 to 2025-12-31.
```

### 1.12 §8.2 E10, E24, E29 rewrites

Do these three bottom-up too (E29, then E24, then E10).

E29, lines 1924-1931. Anchor: `` - **E29** - `mlm export notes` value errors (§6.1): `-m 2026-003`, ``. Replace lines 1924-1931 with:

```
- **E29** - `mlm export notes` value errors (§6.1): `-m 2026-003`,
  `-m 26-03`, `-m 2026-` and `-m -0` fail with the shape message, e.g.
  `-m abc` prints `error: invalid MONTH "abc": expected YYYY-MM, MM or -N (N >= 1)`.
  `-m 0`, `-m 13` and `-m 2026-13` fail with the range message, e.g.
  `-m 13` prints `error: invalid MONTH "13": month must be 1-12`. A bad
  `DATE` and a bad `WEEK_ID`, `-w -0` included, print their §6.1
  messages, and `-m -d` fails as a bad `MONTH`. Each exits `1` with
  nothing on stdout. `-w -1` and `-m -1` are ranges (F17).
```

E24, lines 1889-1892. Anchor: `` - **E24** - Argument order (§3.7, §6.1): `mlm week 2026-41 target 20h` ``. Replace lines 1889-1892 with:

```
- **E24** - Argument order (§3.7, §6.1): `mlm week 2026-41 target 20h`
  prints `error: put WEEK_ID after target: mlm week target 2026-41 20h`,
  no `Usage:` block, exit `2`. `mlm week -1 target 35h` prints
  `error: put WEEK_ID after target: mlm week target -1 35h`, exit `2`.
  `mlm week 7 foo` keeps the parser's own error.
```

E10, lines 1806-1808. Anchor: `` - **E10** — `week target` missing the `DURATION` argument entirely → ``. Replace lines 1806-1808 with:

```
- **E10** — `week target` missing the `DURATION` argument → parser
  missing-argument error whose usage names `[WEEK_ID] <DURATION>` and
  never `<ARGS>`, exit `2`. Bare `mlm week target` gives it. So does one
  token that parses as a `WEEK_ID` (`-1`, `7`, `2026-07`), with stderr
  byte-identical to the bare case, nothing on stdout and no target
  written (§3.7). `mlm w target -1`, `mlm -v week target -1`,
  `mlm week target -v 7` and `mlm week target -- -1` give it too. Any
  other lone token, such as `-0`, `-99999999999999999999`, `0` or `53`
  in a 52-week year, keeps `invalid duration` with exit `1`.
  `mlm week target 35h` still sets this week's target.
```

### 1.13 §8.2 new error flows (after line 1962)

Anchor: `` `week WEEK_ID target` shape still exits `2`. `` (last line of E34).

Insert after line 1962, keeping the trailing blank line 1963 as the file's last line:

```
- **E35** - Relative-id shape errors (§3.6, §6.1): `mlm week -0`,
  `mlm week -1.5`, `mlm week target -1x 35h` and `mlm ex n -w -1x`
  print the `WEEK_ID` shape error, e.g. `mlm week -0` prints
  `error: invalid WEEK_ID "-0": expected YYYY-WW, WW or -N (N >= 1)`.
  `mlm ex n -m -0` and `mlm ex n -m -1x` print the `MONTH` one, e.g.
  `error: invalid MONTH "-1x": expected YYYY-MM, MM or -N (N >= 1)`.
  Each exits `1`, prints nothing to stdout and writes nothing.
- **E36** - Offset out of range (§6.1): `mlm week -99999999999999999999`
  prints
  `error: invalid WEEK_ID "-99999999999999999999": week offset is out of range`.
  From a fixed today, the last `-N` that reaches week `1000-01`
  resolves to it, and the next one prints the week offset error. In
  February 2026, `mlm ex n -m -24313` is `0000-01` and exits `0`, and
  `-m -24314` prints
  `error: invalid MONTH "-24314": month offset is out of range`. A
  25-digit `-m` run prints the month offset error too. Each error exits
  `1` with nothing on stdout. No `-N` panics or resolves to a week
  before `1000-01`.
- **E37** - Future hint (§6.1): `mlm week +1`, `mlm week target +1 35h`
  and `mlm ex n -w +1` print
  `error: invalid WEEK_ID "+1": expected YYYY-WW, WW or -N (N >= 1); a future week takes YYYY-WW or WW`,
  and `mlm ex n -m +1` prints
  `error: invalid MONTH "+1": expected YYYY-MM, MM or -N (N >= 1); a future month takes YYYY-MM or MM`.
  `+0` gets the hint too, and `+1x` and `+` get the plain shape
  message. `mlm status +1` keeps
  `error: invalid DATE "+1": expected YYYY-MM-DD or -N (N >= 1)`, with
  no hint. Each exits `1`.
- **E38** - `mlm week` hyphen tokens (§3.6, §6.3): `mlm week -1x`,
  `mlm week -x` and `mlm week --bogus` are usage errors from the
  argument parser, exit `2`, with nothing on stdout. `mlm week -1.5`
  reaches the `WEEK_ID` parser instead and fails as in E35.
```

### 1.14 SPEC sections checked and left alone

- §3.4a, §3.5, §6.1's `DATE` bullet: `DATE` is unchanged (spec §1.1).
- §6.3 (lines 809-844): "a missing argument" already covers the lone-`WEEK_ID` usage error, and "an unknown flag" covers `mlm week -1x`. No edit.
- §6.1 line 789-790 (the swapped shape): still true with `-1`.
- §7.2: `mlm week -1`'s header shows the resolved id, as for any `WEEK_ID`. No edit.
- §8.2 E3 (lines 1778-1781): its inputs stay malformed. E35 carries the `-N` shapes.
- §8.2 E28: `-d -1` is still yesterday. F17 carries `-w -1`/`-m -1`.

## 2. `README.md`

Apply in this order: 2.8, 2.7, 2.6, 2.5, 2.4, 2.3, 2.2, 2.1.

### 2.1 Planned bullet (lines 42-43)

Anchor: ``- `+N`/`-N` relative week notation for `week`'s week-id argument``.

Replace lines 42-43 with:

```
- `+N` relative notation for future weeks and months — a future week
  takes `YYYY-WW` or `WW`, and a future month `YYYY-MM` or `MM` (`-N`
  for past ones works, see `week` and `export notes` below)
```

### 2.2 `mlm week` forms (lines 425-428)

Anchor: `Show a week's per-day totals plus its carry-in/worked/fulfillment/`.

Replace lines 425-428 with:

```
Show a week's per-day totals plus its carry-in/worked/fulfillment/
target summary. `WEEK_ID` accepts a full id (`YYYY-WW`, e.g. `2026-37`),
a bare week number for the current year (e.g. `37`), or `-N` for N
weeks before the current one (e.g. `-1` = last week), and defaults to
the current week. With today in week 2026-37, `mlm week -1` shows week
2026-36 (2026-08-31 - 2026-09-06). `-N` counts back across years and
53-week years, so in week 2026-02 `-2` is `2025-52`. `-0` is not
accepted, and there is no `+N`, so a future week takes its id. As with
`status`, a token like `-1x` is a usage error with exit code 2.
```

### 2.3 `-n` order examples (lines 504-506)

Anchor: `the flag, and a week with no notes prints the same either way. The flag`.

Replace lines 504-506 with:

```
the flag, and a week with no notes prints the same either way. The flag
goes before or after `WEEK_ID` (`mlm week -n 43`, `mlm week 43 -n`,
`mlm week -1 -n`), and `week target` does not take it. With today Sat
2026-10-24:
```

Line 507 is blank and line 508 opens the existing `sh` block. Check that line 506's old text (``and `week target` does not take it. With today Sat 2026-10-24:``) is fully replaced and not duplicated.

### 2.4 `week target` forms (lines 538-542)

Anchor: `` when no override exists). `WEEK_ID` accepts the same forms as `week` ``.

Replace lines 538-542 with:

```
Set an absolute target override for a week (default target is 40h
when no override exists). `WEEK_ID` accepts the same forms as `week`,
`-N` included, and defaults to the current week; `DURATION` uses the
human format (`20h`, `33h30m`, `45m`), never raw minutes, and is always
the last token. On success it prints one line with the stored target:
```

Only ``, `-N` included,`` is new. The rest is the old text re-wrapped.

### 2.5 `week target` `-N` example and lone-token rule (after line 566)

Anchor: `` The duration echoes in the standard format, so `90m` prints as `01h 30m`. ``

Insert after line 566, with one blank line before the new text (line 567 is the blank line before the 168h paragraph, which stays):

````

`-N` sets a past week's target, and the line names the week it
resolved to. With today in week 2026-37:

```sh
$ mlm week target -1 38h
Week 2026-36 target: 38h 00m
```

`DURATION` is always required, so a lone week id counts as a missing
duration. `mlm week target -1`, `mlm week target 7` and
`mlm week target 2026-07` print the same usage error as bare
`mlm week target` and exit with code 2. A bare number counts as a week
id, so `mlm week target 40` fails the same way. Write `40h`.
````

(The outer four-backtick fence is only this plan's quoting. README gets the inner three-backtick `sh` block.)

### 2.6 Swapped reminder with `-1` (after line 582)

Anchor: the closing fence on line 582, after `error: put WEEK_ID after target: mlm week target 2026-41 20h`.

Insert after line 582, before the blank line 583:

````

The reminder echoes what you typed, so `mlm week -1 target 35h` prints
`error: put WEEK_ID after target: mlm week target -1 35h`.
````

Line 583 (blank) and line 584 (`` `mlm week target --help` shows the arguments as `[WEEK_ID] <DURATION>`. ``) follow unchanged.

### 2.7 `export notes` forms (lines 618-621)

Anchor: `` - `-w`/`--week WEEK_ID` — one week, Monday to Sunday, in the same forms ``.

Replace lines 618-621 with:

```
- `-w`/`--week WEEK_ID` — one week, Monday to Sunday, in the same forms
  as `week` (`2026-43`, a bare `43` for the current year, or `-1` for
  last week)
- `-m`/`--month MONTH` — one calendar month, `YYYY-MM` (e.g. `2026-03`),
  a bare month number for the current year (e.g. `3`), or `-N` for N
  months before the current one (e.g. `-1` = last month, so in January
  2026 `-m -1` is `2025-12`)
```

### 2.8 `export notes` `-N` example (after line 640)

Anchor: `$ mlm export notes --month 2026-10 > october.txt` and its closing fence on line 640.

Insert after line 640, before the blank line 641:

````

`-w -1` and `-m -1` always mean the week or month before the current
one, so one command makes a weekly or monthly report:

```sh
$ mlm ex n -m -1 > last-month.txt
```
````

### 2.9 README sections checked and left alone

- Features list (lines 8-28): no argument forms named.
- Possible issues (lines 45-57): spec §1.2 is "None", and NOTES.md entries 153-154 file no known issue.
- `status` and `delete` sections: `DATE` is unchanged.

## 3. `AGENTS.md`

Checked in full. No edit.

- Line 36-37 says `src/date.rs` holds "DATE/WEEK_ID/MONTH parsing, `WeekId`, and a month's first and last day". Still true, since the parsers gain a form in place.
- Line 32 (`src/cli.rs`) stays true.
- "Current state" (lines 15-26) lists commands, not argument forms, and names no relative-id stretch idea.
- No new module (changeset plan "Architecture"), so the Layout list and `src/lib.rs` note need nothing.
- `tests/relative_ids_cli.rs` is a test file. AGENTS.md lists no test files.

If Task 1's report names a new module or source file, recheck lines 30-82 before merge. That would be a coordinator call, since the changeset plan says none is added.

## 4. Check plan

Run from the Task 2 worktree root after the edits. Every command is read-only. Each subsection maps to one acceptance criterion in the changeset plan's Task 2 section, in order.

### 4.1 AC 1: §1.2 `+N`-only non-goal

- ``grep -nF '`+N`/`-N` relative' docs/dev/SPEC.md`` is empty.
- ``grep -nF -e '- `+N` relative notation for future weeks and months' docs/dev/SPEC.md`` gives one line in §1.2 (about line 49).
- The bullet names `YYYY-WW`/`WW` and `YYYY-MM`/`MM` as the future forms.

### 4.2 AC 2: §3.6 and §3.7 describe `-N`, the exit-`2` rule, `-1 -n`, the `week target` shapes and the `-1` reminder

- ``grep -nF 'or `-N` for N ISO weeks before the current one' docs/dev/SPEC.md`` matches in §3.6, and the paragraph names 2026-01-05, 2027-01-04 and ``` `1000-01` ```.
- ``grep -nF '(`-1x`, `-x`, `--bogus`) stays a usage error' docs/dev/SPEC.md`` matches in §3.6, and the next line holds ``exit `2` (§6.3)``.
- `grep -cF '`mlm week -1 -n` and `mlm week -n -1`' docs/dev/SPEC.md` is `2` (§3.6, F16).
- `grep -cF 'error: put WEEK_ID after target: mlm week target -1 35h' docs/dev/SPEC.md` is `2` (§3.7 block, E24).
- `grep -cF 'Week 2026-42 target: 35h 00m' docs/dev/SPEC.md` is `1`.
- §3.7 contains the lone-token paragraph naming `-1`, `7` and `2026-07`.

### 4.3 AC 3: §3.8 `-N` for `--week`/`--month`, year crossing, rewritten hyphen sentence, contract 5 help lines

- `grep -n 'reach their own' docs/dev/SPEC.md` is empty.
- `grep -nF '`-w -1` is last week and `-m -1` is last month' docs/dev/SPEC.md` matches inside §3.8 (the phrase sits on one source line in 1.8's text).
- ``grep -nF '`-w -2` is `2025-52`' docs/dev/SPEC.md`` matches in §3.8, and the next line starts with ``` `2025-12` ```.
- Each of these returns exactly `1`:
  - ``grep -cF '`-w, --week <WEEK_ID>`: ``Week to export: YYYY-WW (e.g. 2026-07), a bare week number (e.g. 7), or `-N` for N weeks before this one (e.g. `-1` = last week)``' docs/dev/SPEC.md``
  - ``grep -cF '`-m, --month <MONTH>`: ``Month to export: YYYY-MM (e.g. 2026-03), a bare month number (e.g. 3), or `-N` for N months before this one (e.g. `-1` = last month)``' docs/dev/SPEC.md``
- The old lines are gone. `grep -cF 'or a bare week number (e.g. 7)`' docs/dev/SPEC.md` and `grep -cF 'or a bare month number (e.g. 3)`' docs/dev/SPEC.md` are `0`.

### 4.4 AC 4: §6.1 messages, floors, `u64` rule

Each of these `grep -cF` counts in `docs/dev/SPEC.md` must match:

| String | Count | Where |
|---|---|---|
| `invalid WEEK_ID "<input>": expected YYYY-WW, WW or -N (N >= 1)` | 1 | §6.1 |
| `invalid WEEK_ID "<input>": week offset is out of range` | 1 | §6.1 |
| `invalid WEEK_ID "+1": expected YYYY-WW, WW or -N (N >= 1); a future week takes YYYY-WW or WW` | 2 | §6.1, E37 |
| `invalid MONTH "<input>": expected YYYY-MM, MM or -N (N >= 1)` | 1 | §6.1 |
| `invalid MONTH "<input>": month offset is out of range` | 1 | §6.1 |
| `invalid MONTH "+1": expected YYYY-MM, MM or -N (N >= 1); a future month takes YYYY-MM or MM` | 2 | §6.1, E37 |
| `expected YYYY-WW or WW` | 0 | gone |
| `expected YYYY-MM or MM` | 0 | gone |

- §6.1 names `1000-01` and `0000-01` as floors, and ``grep -cF 'too large for a `u64`' docs/dev/SPEC.md`` is `2`.
- `grep -nF 'expected YYYY-MM-DD or -N (N >= 1)' docs/dev/SPEC.md` finds only E37's `mlm status +1` line, which has no `; a future` suffix. SPEC:724-726 (`DATE`) is untouched.

### 4.5 AC 5: §8.1 happy paths, §8.2 error flows, E29 and E10 rewrites

- `grep -n '^- \*\*F1[67]\*\*' docs/dev/SPEC.md` gives two lines, both inside §8.1 (before `### 8.2`).
- `grep -n '^- \*\*E3[5-8]\*\*' docs/dev/SPEC.md` gives four lines, after E34.
- F16 names `mlm week -1` and `mlm week target -1 35h`. F17 names `mlm ex n -w -1` and `mlm ex n -m -1`.
- E35 covers `-0`. E36 covers offset out of range. E37 covers `+1`. E38 covers `mlm week -1x`. E10 covers the lone `week target -1`.
- `grep -nF '`-m -1` fail' docs/dev/SPEC.md` and `grep -nF '`-w -1` included' docs/dev/SPEC.md` are empty.

### 4.6 AC 6: the lone-token boundary

- §3.7's second new paragraph and E10 both say only a token that parses as a `WEEK_ID` gives the usage error, and both name `-0` and an out-of-range `-N` as keeping `invalid duration` with exit `1`.
- `grep -cF 'error: invalid duration "-0": expected a form like 20h, 33h30m, or 45m' docs/dev/SPEC.md` is `1`. The text matches `src/time.rs:168-170` with the `error: ` prefix.

### 4.7 AC 7: §3.6 `mlm week -1.5`

- ``grep -nF 'so `mlm week -1.5` fails with the' docs/dev/SPEC.md`` matches in §3.6, and the next line holds ``shape error and exit `1` (§6.1)``.
- `grep -cF '`mlm week -1.5`' docs/dev/SPEC.md` is `3` (§3.6, E35, E38).

### 4.8 AC 8: README

- ``grep -nF '`+N`/`-N`' README.md`` is empty, and the Planned list has the `+N`-only bullet.
- The `mlm week` section contains ``or `-N` for N`` and the `mlm week -1` example naming week 2026-36.
- `week target` section contains `$ mlm week target -1 38h` and `Week 2026-36 target: 38h 00m`.
- The `export notes` section contains ``or `-1` for`` (the `-w` bullet), ``or `-N` for N`` (the `-m` bullet) and `$ mlm ex n -m -1 > last-month.txt`.

### 4.9 AC 9: AGENTS.md

- `git diff --stat -- AGENTS.md` is empty.
- §3's checks hold against the merged Task 1 file list. With no new `src/*.rs` file, nothing is stale. `git diff --name-only <base>..<task-1-branch> -- src/` lists only modified files if Task 1 is merged first. The coordinator runs this at merge.

### 4.10 AC 10: byte-for-byte contracts

- Every contract 2-5 string in 4.3 and 4.4 passes its `grep -cF`.
- Contract 5 `mlm week` and `week target` lines:
  - ``grep -cF 'Week to show: YYYY-WW (e.g. 2026-07), a bare week number (e.g. 7), or `-N` for N weeks before this one (e.g. `-1` = last week). Defaults to the current week' docs/dev/SPEC.md`` is `1`.
  - ``grep -cF '  [WEEK_ID]   Week to set: YYYY-WW (e.g. 2026-07), a bare week number (e.g. 7), or `-N` for N weeks before this one (e.g. `-1` = last week). Defaults to the current week' docs/dev/SPEC.md`` is `1`.
- Contract 8 echo shape `Week <id> target: 35h 00m` appears with a concrete id in §3.7 and F16.
- Exit codes: every new flow says `1` for contracts 2-4, `2` for `mlm week -1x`/`-x`/`--bogus`, the reminder and the lone `WEEK_ID`.
- Once Task 1 is on the integration branch, a spot check against the binary confirms the quoted text. Build into a private dir (`CARGO_TARGET_DIR=/tmp/cd-scratch/relids-task2/target cargo build`), then with `MLM_DB_PATH` on a scratch db and `TZ=UTC`, compare stderr of `mlm week -0`, `mlm week +1`, `mlm ex n -m +1`, `mlm ex n -m -24314` (only meaningful in February 2026; otherwise skip), `mlm week target -0` and `mlm week target -1` with the SPEC text. `mlm week target -1` and `mlm week target` must give identical stderr. `mlm week target --help` must contain 4.10's `[WEEK_ID]` row.

### 4.11 Self-containedness and style (AGENTS.md "Documentation")

- `grep -rn 'specs/\|plans/\|relative-ids' README.md docs/dev/SPEC.md` is empty.
- `git diff -U0 -- docs/dev/SPEC.md README.md | grep '^+' | grep -nP '[^\x00-\x7F]'` shows only lines whose non-ASCII characters are `§` and `—`, as the neighbouring lines use.
- `git diff --stat` lists only `docs/dev/SPEC.md` and `README.md`.

### 4.12 Commit gate

- The pre-commit hook (`.githooks/pre-commit`) runs `cargo fmt --check`, clippy cognitive complexity and `cargo llvm-cov` on every commit, docs-only ones included (`.githooks/pre-commit:35-148`). With no source change in this task, all three pass, and coverage reports "unchanged".
- The commit carries no `Co-Authored-By` or `Claude-Session` line.

## 5. Risks, ambiguities, disagreements

### R1. No sibling task plan exists yet

Task 1's plan was absent when this was written. Every quoted string comes from contracts 2-5 and 8, so drift can only enter if Task 1's code departs from a contract. 4.10's binary spot check catches that at merge.

### R2. The rendered `[WEEK_ID]` help row in §3.7 pins padding the contract does not

Contract 5 gives the text after `[WEEK_ID]`. SPEC §3.7 quotes the `<DURATION>` row as rendered (SPEC:431), so 1.5 quotes the `[WEEK_ID]` row the same way, with `src/cli.rs:278`'s three spaces after the name. If Task 1 changes that padding, the SPEC row drifts. The fix is a one-line SPEC edit at merge. The other three help lines use the backtick form of SPEC:484, which pins no padding.

### R3. The spec's "and not a valid `DURATION`" clause is left out of SPEC.md

Spec §3 says a lone token that is a valid `WEEK_ID` "and not a valid `DURATION`" becomes the usage error. No `WEEK_ID` is ever a valid `DURATION`, since a duration needs an `h` or `m` suffix (changeset plan Risks; plan review "What checks out"). So SPEC.md states the rule as "parses as a `WEEK_ID`". This changes no behavior and avoids a clause that has no effect.

### R4. `mlm week target 40` is now a usage error, stated in docs

NOTES.md entry 154 records this (plan review U2) as following the user's own lone-token rule, not filed as a known issue. SPEC §3.7 and README state the behavior plainly, since a user who drops the `h` hits it. Neither file lists it under possible issues.

### R5. The week floor has no numeric example

SPEC E36 states the week floor in words ("the last `-N` that reaches week `1000-01`"). It gives no N, because neither the spec nor the changeset plan pins one and this plan could not compute one with a read-only toolset. Task 1 derives it from an injected today. The month floor does get numbers, which the changeset plan pins.

### R6. `mlm week -1x`'s clap text is not quoted

Clap prints `unexpected argument '-1' found` and a tip to use `-- -1`, which now runs last week (plan review U1, not filed). SPEC and README say only "usage error, exit 2", as they do for `mlm status -1x` today. Quoting clap's text would pin wording Task 1 does not own.

### R7. Contract 9 understates §6.1 and §8

Contract 9 says §6.1's `WEEK_ID` bullet gains contracts 2 and 3. Task 2's AC 4 adds contract 4 for both kinds, and this plan follows the AC. AC 5 asks for E29 and E10 rewrites, and this plan also edits E24 (the `-1` reminder) and §3.6's `--notes` paragraph. Both serve contract 8 and AC 2. They add no new mechanism.

### R8. Clock-relative examples in SPEC and README

SPEC and README examples fix "today" in words (2026-01-05, week 2026-37, week 2026-43), as SPEC §3.8 already does for 2025-12-30. README's week 2026-36 dates are derived from README's own 2026-35 and 2026-37 examples, not from a calendar tool.

### Disagreement with the spec or changeset plan

None of substance. R3 and R7 are wording choices within the AC.
