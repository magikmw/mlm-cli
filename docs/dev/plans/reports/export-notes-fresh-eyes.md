# export notes — fresh-eyes run

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

Sandbox: /tmp/cd-sandbox/export-notes/ (artifact `mlm`, `README.md`). Today was Sat 2026-10-24 (week 2026-43).
All runs used `MLM_DB_PATH=/tmp/cd-sandbox/export-notes/test.db` unless noted (scratch DBs `big.db`, `typo.db`, `typo2.db`, `nodir/`, `nodir2/` also inside the sandbox).

## Scenarios — what you ran, what it printed

### 1. This month's notes for a monthly summary
Recorded notes with the documented forms: `mlm note --date 2026-10-01 ...` (x2), `mlm note -d 2026-10-07 ...`, `mlm n -d 2026-10-15 ...` (x2), `mlm start -d -3 09:00 "standup and triage"` + `mlm stop -d -3 17:00`, `mlm n -d -1 ...` (x2, one with a quote and double spaces), `mlm n "today: wrote report"`, `mlm n -d 2026-09-30 ...`. All silent, rc 0. `mlm n -d 2026-11-01 future?` -> `error: invalid DATE "2026-11-01": date is in the future`, rc 1.

`mlm export notes --month 2026-10` and `mlm ex n -m 10` both printed:
```
2026-10-01  kicked off Q4 planning
2026-10-01  set up CI cache
2026-10-07  wrote export spec
2026-10-15  pairing session with Ana
2026-10-15  fixed flaky test
2026-10-21  standup and triage
2026-10-23  reviewed PR #52
2026-10-23  it's got a 'quote' and  double  spaces
2026-10-24  today: wrote report
```
rc 0. Sep 30 note correctly excluded. Note text (quotes, double spaces) printed verbatim. The note attached to `start` shows up too. This does what I wanted.

Side check: a note containing a newline (`"$(printf 'line one\nline two')"`) came out as `line one line two` (it is flattened at storage time, `status` shows the same). A tab is kept as a literal tab. Unquoted multi-word note fine.

While here, `mlm status -3` showed `Week 2026-43: 152h 00m left ... carry-in -120h 00m`. After I later added a note on 2025-12-29 (for a boundary test), `mlm week -n 43` read `1712h 00m left to 40h 00m target`. See Finding 11.

### 2. Yesterday and one day last week
`mlm ex n -d -1` -> the two 2026-10-23 lines. `mlm ex n --date 2026-10-15` -> the two 2026-10-15 lines. `--date=-1` and `-d -9` also work. Both in one call, `mlm ex n -d -1 -d 2026-10-15` -> `error: the argument '--date <DATE>' cannot be used multiple times` + usage, rc 2. So two commands. `-d 0` / `-d today` -> `invalid DATE "0": expected YYYY-MM-DD or -N (N >= 1)`, rc 1. `-d 2026-02-30` -> `not a real calendar date`. `-d 2026-10-1` and `-d 20261001` rejected with the format message. `-d -99999999` -> `date offset is out of range`.

### 3. This week and last week, various namings
Worked: `-w 43`, `-w 2026-43`, `-w 42`, `-w 2026-42`, `-w42`, `--week=42`, `mlm export n`, `mlm ex notes`, `mlm ex n`; no flag at all gave this week (matches README). `-w 2026-4` accepted (zero pad optional). `-w 2026-53` accepted (2026 really has 53 ISO weeks), empty.
Rejected (rc 1, `expected YYYY-WW or WW`): `2026-W42`, `W42`, `-1`, `042`, `2026-042`. `-w 0` -> `week number must be 1 or greater`; `-w 54` -> `2026 has only 53 ISO weeks` (good message).
`mlm ex n 42` (positional, like `mlm week 42`) -> `error: unexpected argument '42' found` + `Usage: mlm export notes [OPTIONS]`, rc 2.
`mlm ex note -w 42` -> `unrecognized subcommand 'note'` with tip `'n', 'notes'`, rc 2. `mlm x n` -> tip suggests `export`. Bare `mlm ex` prints its help, rc 2.
Boundaries correct: `-w 1` includes 2025-12-29 and 2026-01-01; `-w 40` spans Sep 30 / Oct 1; `-m 1` and `-m 2025-12` split them correctly. Output agrees with `mlm week -n 43`.

### 4. File and pipe
`mlm ex n -m 10 > october.txt` rc 0; `cat -A` shows clean `$` line ends, no header, 9 lines. `| grep -c PR` -> 1. `| cut -c13-` strips the date prefix cleanly (fixed 12-char prefix). `| head -1` on a 400-note DB (`big.db`): one line, `PIPESTATUS[0]` 0, no broken-pipe panic. stderr empty on success.

### 5. Typing mistakes
- `-m 13`, `-m 0`, `-m 2026-13`, `-m 2026-00` -> `invalid MONTH "...": month must be 1-12`, rc 1. Clear.
- `-m 26-10`, `2026/10`, `10-2026`, `Oct`, `october`, `2026-10-01`, `" 10"`, `010` -> `expected YYYY-MM or MM`, rc 1. `-m 2026-1` and `-m 9`/`-m 2026-9` accepted.
- `-w 42 -m 10` -> `error: the argument '--week <WEEK_ID>' cannot be used with '--month <MONTH>'` then `Usage: mlm export notes --week <WEEK_ID>`, rc 2. `-d -1 -w 43` same shape.
- `-m` / `-w` / `-d` at end of line -> `a value is required for '--month <MONTH>' but none was supplied`, rc 2. Clear.
- `-m -w 42` -> `error: unexpected argument '42' found`, rc 2. `-d -w 42` and `-w -m 10` same. `-m -w` -> `invalid MONTH "-w"`; `-d --month` -> `invalid DATE "--month"`. The value-less flag swallows the next flag as its value.

### 6. Empty and future periods
`-m 2026-08`, `-d 2026-10-02`, `-d 2026-12-25`, `-w 50`, `-m 12`, `-m 2099-01`, `-d 9999-12-31`, `-m 0001-01`, `-w 2025-01`: no output at all, rc 0. With `-v`: only migration/db-path debug lines, nothing about the result count.
Pointing `MLM_DB_PATH` at a typo (`typo.db`) or a non-existent directory (`nodir/x.db`): export prints nothing, rc 0, and silently creates the file (and the directory).

## Findings — each with the scenario, what you expected, what you got, why it would confuse someone

1. **A forgotten value swallows the next flag, and the error blames the wrong token** (Sc 5). Expected: `mlm ex n -m -w 42` to say `--month` needs a value. Got: `error: unexpected argument '42' found` / `Usage: mlm export notes [OPTIONS]`, rc 2 (`-d -w 42`, `-w -m 10` alike). `-m -w` gives `invalid MONTH "-w"`. The user is told `42` is wrong, when `42` is the one correct thing on the line; nothing says `-m` lacked its value. Presumably a side effect of letting values start with `-` for `-N` dates, but it reaches `--week`/`--month`, which never take negative values.

2. **Empty result is indistinguishable from "wrong database" or "wrong period"** (Sc 6, 4). Expected: some signal, at least on stderr or under `-v`, that 0 notes matched (stdout can stay empty for scripts). Got: nothing, rc 0, even with `-v`. With a typo'd `MLM_DB_PATH` (or an unset one on a new machine), export silently creates a fresh empty DB, even a missing directory, and prints nothing, rc 0. A person writing a monthly report who gets a blank file can't tell "no notes", "I typed the wrong month" and "this is the wrong DB" apart. The README documents "prints nothing and exits 0" but gives no way to tell these cases apart.

3. **No way to say "last week" without looking up its number** (Sc 3). Expected: since `-d -1` means yesterday, `-w -1` might mean last week. Got: `invalid WEEK_ID "-1": expected YYYY-WW or WW`. The README lists `+N/-N` week notation as planned only for `week`'s argument; the export section says nothing, and the error doesn't say relative weeks aren't supported or point to `mlm week` to find the current id. A user has to run `mlm week`, read `Week 2026-43`, and subtract.

4. **`mlm ex n 42` rejected, unlike `mlm week 42`** (Sc 3). Expected: a positional week id to work, or an error that suggests `--week 42`. Got: generic `unexpected argument '42' found`. Users who know `week 42` will try this first, and the message gives no next step.

5. **`export note` (singular) rejected** (Sc 3). Elsewhere the command is `mlm note`/`delete note`, but under export it is `notes`. `mlm ex note` fails (rc 2). The clap tip (`'n', 'notes'`) does rescue it. Nitpick: `delete note` is singular and `export notes` plural, an inconsistency that will trip people.

6. **Conflict error prints a misleading usage line** (Sc 5). `-w 42 -m 10` -> `Usage: mlm export notes --week <WEEK_ID>`, which looks like the tool is telling you the "right" form uses `--week`, when you might have meant the month. It never says plainly "give only one of --date, --week, --month" (the README's "at most one range"). Readable, but not a direct next step.

7. **Two separate dates need two calls; nothing says so** (Sc 2). My manager's request (yesterday + one day last week) needs `mlm ex n -d -1; mlm ex n -d 2026-10-15`. Repeating `-d` gives `cannot be used multiple times`. Fine once you know, but neither the README nor `--help` says it's one range per call, and the error doesn't suggest running it twice. Nitpick.

8. **Format leniency is inconsistent, and messages contradict what is accepted** (Sc 2, 3, 5). `-m 2026-1`, `-m 9` and `-w 2026-4` are accepted (no zero-pad needed), but `-d 2026-10-1` is rejected. The month error says `expected YYYY-MM or MM` while `M` and `YYYY-M` work. `-m 2026-10-01` (a date given to `--month`) gets the generic format message rather than a hint to use `--date`. The standard ISO week form `2026-W42` is rejected with no hint that you should drop the `W`. Small, but anyone pasting ISO-style ids will hit it.

9. **Exit codes differ by kind of mistake, undocumented** (Sc 5). Bad values (`-m 13`, `-d today`, `-w W42`) exit 1; usage mistakes (conflicting flags, missing value, positional arg, two `-d`) exit 2. The README only mentions exit 2 for `week target`'s argument-order case. A script author can't tell which to expect. Nitpick.

10. **Note text normalisation is undocumented** (Sc 1, 4). A note with an embedded newline is silently stored/exported as one line with a space, which protects "one per line" but isn't mentioned anywhere. A tab survives as a raw tab, so a note can contain the kind of separator a user might `cut -f` on. The README says nothing about either. Nitpick.

11. **Backfilling old notes silently inflates this week's deficit** (Sc 1, side effect). Following scenario 1, I recorded notes on past days with no punches. Status for this week then read `152h 00m left ... carry-in -120h 00m` (from a single 2026-09-30 note). One more note on 2025-12-29 made `mlm week 43` read `1712h 00m left to 40h 00m target`. A note with no worked time seems to set the start of tracked history, and every empty week since then counts as a 40h shortfall. The README explains carry-in, but never says that a note (not a punch) starts the history. Someone jotting old notes into the tool for an export would be alarmed by their week page afterwards. Not an export bug as such, but the export workflow leads straight into it.

12. **`export notes --help` is thin compared to the README** (Sc 3, 6). It doesn't describe the output line format (`DATE  note`), that a future range is allowed but empty, that only one range flag is allowed, or that week ids follow `mlm week`. A user who relies on `--help` (as I did first) learns the line format only by running it. Its week example `2026-07`/`7` is fine. Nitpick.

Things that worked well and gave no pause: month/week/date boundaries (including the ISO week 1 crossing the year and week 53), ordering, the redirect/pipe behaviour (no header, clean line ends, no SIGPIPE panic), `-w 54`'s "2026 has only 53 ISO weeks", `-d 2026-02-30`'s "not a real calendar date", and the missing-value message when the flag is last on the line.

## Verdict — findings(12)
