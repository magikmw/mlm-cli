# week --notes: fresh-eyes user run

Sandbox: /tmp/cd-sandbox/week-notes/ (`mlm 0.4.0`, README.md). Today Sat 2026-10-24, current week 2026-43.
I used `MLM_DB_PATH` pointing at a scratch DB inside the sandbox and deleted it afterwards. My first attempt put the DB in a path
that did not last between shell calls, so the data looked like it had vanished. That was my setup, not the tool, and I
redid the run.

## Scenarios — what you ran, what it printed

### 1. Backfilled days, then the week with notes
Ran `start/stop --date 2026-10-20 ... "note"`, `start/stop/note --date -3`, two stints plus several `note --date -2`,
and `start --date -1 10:00 "deep work"` plus `stop`. All writes were silent, as the README says.
`mlm week --notes` printed the normal table. Each dated row had its notes underneath, indented six spaces with `- `.
Notes attached to start/stop and standalone notes looked the same. Mon (no data) was a single line. Totals, Carry-in
block and headline looked the same as plain `mlm week`. This was usable for a weekly report.

### 2. One day with many notes, one very long
Thu 2026-10-22 had 6 notes. One was about 330 characters. The tool prints it as one physical line and does not wrap it.
Piped through `fold -w 80` to mimic an 80-column terminal:
```
      - reviewed PR #52 for the scheduler
      - This is a very long note that I wrote because I wanted to capture everyt
hing about the meeting with the infrastructure team regarding the migration of o
ur database cluster to the new region, including risks, timelines, the rollback 
plan, and the fact that nobody has tested the failover script since last spring 
which worries me quite a lot honestly
      - lunch with team
```
The continuation lines start at column 0, to the left of the day rows (column 2). The six notes themselves were fine.

### 3. Last week, typed several ways
`mlm week 42 -n`, `mlm week -n 42`, `mlm week 2026-42 --notes`, `mlm week --notes 2026-42`, `mlm w 2026-42 -n` all gave
identical output (checked with diff). Headline `Total behind: 32h 00m`. Notes appeared under Tue/Wed/Thu/Fri.
Other tries:
- `mlm week w42 -n` -> `error: invalid WEEK_ID "w42": expected YYYY-WW or WW`, exit 1. Fine.
- `mlm week 43-n` -> same invalid WEEK_ID error, exit 1. Fine.
- `mlm week 42 -n -n` -> clap `the argument '--notes' cannot be used multiple times`, exit 2. Fine.
- `mlm week -n=42` -> `unexpected argument '-=' found`, exit 2. A bit cryptic but understandable.
- `mlm week -n -1` (hoping for "last week") -> `unexpected argument '-1' found  tip: to pass '-1' as a value, use '-- -1'`, exit 2.
  Following the tip, `mlm week -- -1` -> `error: invalid WEEK_ID "-1": expected YYYY-WW or WW`, exit 1.

### 4. Note on a day without work
`mlm note --date 2026-10-15 "last week thu note, no work"` and `mlm note "today: no work, just a note"`.
In `week -n`, these show as `Thu 2026-10-15   00h 00m` with the note underneath, and the same for Sat 2026-10-24.
`status` for those dates shows `Day total: 00h 00m` and a `Notes:` block. This was clear and matched the README.
Note texts I also tried:
- an embedded newline (`printf 'line one\nline two'`) -> printed as `- line one line two` in both `week -n` and `status`.
- `"   leading spaces and<TAB>tab"` -> leading spaces were removed but the tab was printed as a raw tab.
- `"  - looks like a nested bullet"` -> `- - looks like a nested bullet`.

### 5. Help for the week and day views vs README
`mlm week --help` lists `-n, --notes  Show each date's notes under its row`. Its glossary has a `- <note>` entry: "One of a
date's notes, in the order they were added. `mlm status` lists them under `Notes:`, and `mlm week --notes` under the
date's row. Notes change no total, marker or footer." That answered my question about the indented lines.
`mlm status --help` has the same glossary, word for word, including the `- <note>` entry. `mlm week -h` shows the
short form without a glossary and points to `--help`. `mlm help week` is the same as `--help`.
These match the README's `week --notes` section: six spaces plus `- `, the same order as status, rows without notes stay
one line, the flag works before or after WEEK_ID, and `week target` does not take it. I found no contradiction here.

### 6. `--notes` added to `week target`
Every variant was rejected and the target stayed unchanged (`mlm week` still showed a 40h target afterwards). The errors:
- `mlm week target 36h -n` -> `error: invalid WEEK_ID "36h": expected YYYY-WW or WW`, exit 1
- `mlm week target -n 36h` -> `error: invalid WEEK_ID "-n": expected YYYY-WW or WW`, exit 1
- `mlm week -n target 36h` -> `error: the subcommand '36h' cannot be used with: --notes [WEEK_ID]`, exit 2
- `mlm week --notes target 43 36h` -> `error: the subcommand '43' cannot be used with: --notes [WEEK_ID]`, exit 2
- `mlm week target 43 36h --notes` -> `error: unexpected value '--notes' for '[WEEK_ID] <DURATION>' found; no more were expected`, exit 2
- `mlm week target 42 -n 36h` -> `error: unexpected value '36h' for '[WEEK_ID] <DURATION>' found; no more were expected`, exit 2
`mlm week target --help` does not list `-n`, which matches the README.

## Findings — each with the scenario, what you expected, what you got, why it would confuse someone

1. **A long note breaks the indented layout (scenario 2).** Expected: a long note either wraps at the note's indent or
   is clearly kept as one line. Got: one unwrapped physical line, so the terminal's soft wrap puts the continuation at
   column 0. That is further left than the day rows. On a busy day it is hard to see where the note ends and whether
   the next lines are notes or rows. The README's description ("six spaces, `- ` and the note") does not mention long
   notes. This is also the output I would copy into a report, and the copy shows the same messy wrapping.

2. **A line break inside a note is silently turned into a space (scenario 4).** Expected: the line break kept, or a
   warning when the note was saved. Got: `line one line two` in both `week -n` and `status`. Neither the README nor the
   help says this happens. A user who pastes a multi-line note will see it joined and won't know whether the line break
   was stored and only hidden, or lost.

3. **Inconsistent whitespace handling (scenario 4).** Leading spaces are removed without notice, but a tab inside the
   note is printed raw, which can push the text out of line with the other notes. This is minor and undocumented.

4. **`week target` reads `-n` as a positional value (scenario 6).** Expected: something like "`--notes` is not accepted
   by `week target`", which is what the README says. Got: `invalid WEEK_ID "36h"` for `week target 36h -n`, which
   blames my correct duration, and `invalid WEEK_ID "-n"` for `week target -n 36h`. Neither message mentions the flag
   as the problem, so I would start doubting how I wrote the duration or the week id.

5. **`week -n target ...` names the wrong subcommand (scenario 6).** Got: `the subcommand '36h' cannot be used with:
   --notes [WEEK_ID]` (and `'43'` when a week id is given). `36h` and `43` are not subcommands. The subcommand is
   `target`. It also lists `[WEEK_ID]`, which I did not type before `target`. This is confusing, though at least
   `--notes` is named.

6. **`week target 42 -n 36h` blames the duration (scenario 6).** Got: `unexpected value '36h' ... no more were expected`.
   The extra item is `-n`, not `36h`. The trailing variant (`... 36h --notes`) calls `--notes` a "value", which is
   wrong but at least points at the right token.

7. **Exit codes are inconsistent for similar mistakes (scenarios 3, 6).** An invalid WEEK_ID (including the `-n` cases
   above) exits 1. Clap usage errors exit 2. The README's WEEK_ID-before-`target` error exits 2. A script cannot use
   the exit code to tell a usage error from a runtime failure (the README says a DB read error also exits 1).

8. **The `-1` tip leads to a dead end (scenario 3).** `mlm week -n -1` suggests `use '-- -1'`, and doing that gives
   `invalid WEEK_ID "-1"`. The README lists relative weeks as planned, but the tool's own tip suggests they work, and
   neither error suggests a working alternative (e.g. "use the week number, 42"). `status` accepts `-1`, so trying it
   on `week` is a natural guess.

9. **The notes don't show which stint or time they belong to (scenario 1).** Notes given with `start 09:00 "planning"`
   and standalone `note` calls look the same under the row. There are no times. For a weekly report this is mostly
   fine, but I briefly wondered whether the punch notes would show at all, since the README's `--notes` section only
   says "the notes `status` shows". This is a small point and the behavior matches `status`.

10. **A headline with more left than the target (scenario 1/4, not specific to --notes).** With last week's -32h
    carry-in, the current week's headline reads `42h 45m left to 40h 00m target`. "42h left to a 40h target" looks
    wrong until you read the carry-in glossary. It appears the same with or without `-n`. I'm noting it because it
    gave me pause while reading the notes view.

Positives: the flag works in both positions, the output stays the same apart from the indented notes, a week without
notes would look the same either way, a note on an empty day is clearly shown, and the help glossary explains the
indented lines and agrees with the README.

## Verdict — clear | findings(N) | blocked

findings(10)
