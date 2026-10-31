# panic-fixes — fresh-eyes user run

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

Sandbox: /tmp/cd-sandbox/panic-fixes/ (`mlm` + `README.md`). `MLM_DB_PATH` pointed at
scratch DBs inside the sandbox (t.db, dst*.db, e*.db, day.db). Today = Sun 2026-10-25,
system zone CET (the DST change day itself in Europe/Warsaw).

## Scenarios — what you ran, what it printed

### 1. Odd week targets (`mlm week target X`, then `mlm week` headline)

| input | printed | exit | stored after |
|---|---|---|---|
| 40h | `Week 2026-43 target: 40h 00m` | 0 | 40h |
| 168h | `Week 2026-43 target: 168h 00m` | 0 | 168h |
| 1680h | `error: target cannot exceed 168h 00m (got 1680h 00m)` | 1 | unchanged (168h) |
| 168h1m | `error: ... (got 168h 01m)` | 1 | unchanged |
| 169h, 99999999h, 9223372036854775807m | same error, the huge one shown as `153722867280912930h 07m` | 1 | unchanged |
| 99999999999999999999h | `error: invalid duration "...": expected a form like 20h, 33h30m, or 45m` | 1 | unchanged |
| 0h, 0m, 0h0m, -0h | `target: 00h 00m`; headline `00h 00m left to 00h 00m target` | 0 | 0h |
| 10080m, 167h60m | `target: 168h 00m` | 0 | 168h |
| 00040h, 40h0m | `target: 40h 00m` | 0 | 40h |
| -5h | `error: duration cannot be negative (got -300 minutes)` | 1 | unchanged |
| 40, 1680, 1.5h, 40H, "", " 40h", 1h30, 30m1h, 40m40m, 7d | `error: invalid duration "...": expected a form like ...` | 1 | unchanged |
| `week target 2026-44 1680h` | limit error | 1 | unchanged |
| `week target 44 50h` | `Week 2026-44 target: 50h 00m` | 0 | ok |
| `week target` (nothing) | clap "required arguments were not provided: <DURATION>" + usage | 2 | - |
| `week target 2026-43` (forgot duration) | `error: invalid duration "2026-43": ...` | 1 | - |
| `week target 2026-99 40h` | `error: invalid WEEK_ID "2026-99": 2026 has only 53 ISO weeks` | 1 | - |

The 1680h typo is caught cleanly and the old target is kept, as the README says.

### 2. Pipes (bash, with and without `set -o pipefail`)

- `mlm week | head -1`, `mlm export notes | head -1`, `mlm status | head -1`, `mlm week -n | head -1`:
  first line printed, PIPESTATUS `0 0`, pipefail rc 0 (output is small, head reads it all first).
- `mlm status | true`, `week | true`, `export notes | true`, `week -n | true`, `delete note -d -1 | true`,
  `week target 41h | true`: nothing on stderr, PIPESTATUS `141 0`, pipefail rc 141. Repeated 8x, always 141,
  same as `seq 1 5 | true` and `/bin/echo hi | true` on this box. Target change went through anyway (41h stored).
- `week target 2000h | true`: exit 1 with the limit error (the validation error wins, fine).
- `mlm week --help | true`, `mlm status --help | true`, `mlm --version | true`: exit **0**, not 141.

### 3. Redirects

- `> /dev/full` for week, status, export notes, week -n, week target 45h, delete note --date -1 (listing):
  each printed `error: cannot write output: No space left on device (os error 28)`, exit 1.
- `week target 45h > /dev/full`: exit 1 with that error, but the target **was** set to 45h (`mlm week` showed 45h).
- `delete note 2 --date -1 > /dev/full`: exit 1, same error. The note **was** deleted, and the
  `deleted. to recreate: ...` line went nowhere. The note text is gone unless you remember it.
- `week --help > /dev/full`, `--version > /dev/full`: silent, exit 0.
- `start 08:00 --date -1 > /dev/full`: exit 0 (prints nothing, so fine).
- `week target 40h > /dev/full 2>&1`: exit 1, nothing visible at all, target set.
- Normal file: `week target 42h > out.txt` -> file holds `Week 2026-43 target: 42h 00m`, exit 0;
  `week > w.txt`, `export notes > n.txt` normal; `week target 1680h > bad.txt` -> error on terminal, empty file, exit 1.

### 4. DST, TZ=Europe/Warsaw, 2026-10-25 (02:00-03:00 repeats)

Typed (no --date), fresh DB:
```
mlm start 01:30 "before change"; mlm stop 02:30 "in repeated hour"; mlm start 02:45; mlm stop 03:15
  01:30-02:30  (01h 00m)
  02:45-03:15  (01h 30m)
Day total: 02h 30m   (week row Sun 02h 30m)
```
With `--date 2026-10-25`, fresh DB, start 02:50 then stop 02:10 (meaning: started first pass, stopped second pass):
```
warning: orphaned end on Sun 2026-10-25 at 02:10 (no matching start)
    fix: mlm start HH:MM --date 2026-10-25  (a time before 02:10)
status: Day total: 16h 58m (incl. ongoing), est. EOD 17:50 (tomorrow)
  [!] orphaned end at 02:10 ...
  02:50-now    (16h 58m, ongoing)
week row: Sun 2026-10-25   16h 58m (incl. ongoing)  [!]
```
Fresh DB, `start -d .. 02:00; stop -d .. 03:00; start -d .. 2; stop -d .. 0230`:
```
[!] overlapping stints 02:00-02:00 and 02:00-02:30 (the overlap counts twice)
  02:00-02:30  (00h 30m)
  02:00-02:00  (01h 00m)
delete punch: 1 start 02:00 / 2 start 02:00 / 3 end 02:30 / 4 end 02:00
```
Boundary probe, `start -d 2026-10-25 01:00` then stop at:
```
02:59 -> 01:00-02:59 (01h 59m)
03:00 -> 01:00-02:00 (02h 00m)   listed as "end 02:00"
0300  -> 01:00-02:00 (02h 00m)
03    -> 01:00-02:00 (02h 00m)
03:01 -> 01:00-03:01 (03h 01m)
```
Deleting that "end 02:00" printed `deleted. to recreate: mlm stop 02:00`; running it gave `01:00-02:00 (01h 00m)`.
Viewing the first DB with `TZ=UTC`: `00:45-02:15 (01h 30m)` listed above `23:30-00:30 (01h 00m)`, all on 2026-10-25.

### 5. Ordinary day (system zone)

`start 09:00 "standup"`, `note "reviewed PR #12"`, `stop 12:30`, `start 13:15`, all silent, exit 0.
`status`: `Day total: 09h 03m (incl. ongoing), est. EOD 01:45 (Tue)`, stints, notes. `stop` -> `13:15-18:48`.
`week`, `week -n` (notes under Sun row), `export notes` (`2026-10-25  standup` ...) all as documented.
`export notes -d -1` empty, exit 0. `delete punch 4` -> `deleted. to recreate: mlm stop 18:48` (no --date);
`delete note 1 -d 2026-10-25` -> `deleted. to recreate: mlm note 'reviewed PR #12'` (no --date).
Yesterday's open start (t.db) showed the `[i]` hint in status and `[i]` footer in week, as documented.

## Findings

1. **Typed 03:00 on the clock-change day turns into 02:00 (scenario 4).** Expected: `stop 03:00` after
   `start 01:00` shows `01:00-03:00` and 3h (or at least 2h with the 03:00 label kept). Got:
   `01:00-02:00 (02h 00m)`, and `delete punch` lists it as `end 02:00`. 02:59 gives 1h59m, 03:00 gives 2h,
   03:01 gives 3h01m. It also happens for `0300` and `03`. The time I typed is not the time shown, and the
   duration jumps by an hour between two minutes. Anyone checking their day would think the punch was recorded wrong.

2. **Recreate command after deleting that punch doesn't recreate it (scenario 4).** Deleting the `end 02:00`
   printed `mlm stop 02:00`. Running it stored a different instant: the stint went from 2h to 1h. The README
   presents the recreate line as the undo for a delete. Here the undo silently changes the data.

3. **No way to say "the second 02:xx", and the README says nothing about clock changes (scenario 4).**
   Expected some word on how a repeated hour is read. Got: always the first pass, with no note anywhere.
   `02:45-03:15 (01h 30m)` looks like an arithmetic mistake, since the label says 30 minutes. A start at 02:50
   with a stop at 02:10 (second pass) is flagged as an orphaned end, and the suggested fix ("a time before 02:10")
   can't describe what really happened. Someone who worked through the night has no next step besides fudging times.

4. **The overlap message and stint order on the DST day are hard to read (scenario 4).**
   `[!] overlapping stints 02:00-02:00 and 02:00-02:30`: a "02:00-02:00" stint that lasts 01h 00m, and the list
   shows `02:00-02:30` above `02:00-02:00`. Nothing on the page tells you which 02:00 is which.

5. **A write failure hides that the change happened, and for delete the recreate line is lost (scenario 3).**
   `delete note 2 --date -1 > /dev/full`: exit 1, `error: cannot write output: ...`. The note is deleted, and the
   only copy of its recreate command went to the full device. `week target 45h > /dev/full` is the same: exit 1,
   target set. The README does say the change stays, but the error line alone reads like "it failed". A script
   that retries or reports failure on exit 1 would be wrong. For delete, putting the recreate line on stderr when
   stdout fails, or saying "deleted; ..." in the error, would leave a next step.

6. **`--help`/`--version` ignore write failures and closed pipes (scenarios 2-3).** `mlm week --help > /dev/full`
   and `mlm --version > /dev/full` exit 0 silently. `mlm week --help | true` exits 0 while `mlm week | true` exits 141.
   The README says output "pipes and redirects like any other command's" and that any other write failure prints
   `error: cannot write output` and exits 1. Help output doesn't follow that.

7. **The negative-duration error shows raw minutes (scenario 1).** `-5h` gives `duration cannot be negative (got -300 minutes)`.
   The limit error says `(got 1680h 00m)`, and the README says durations are "never raw minutes". It should read `-05h 00m`.

8. **Forgetting the duration gives a misleading error (scenario 1).** `mlm week target 2026-43` says
   `invalid duration "2026-43"` (exit 1). Plain `mlm week target` says `<DURATION>` is missing (exit 2). In the
   first case I gave a week id and left out the duration. Being told my week id is a bad duration is confusing,
   and the two cases get different exit codes.

9. **Recreate commands leave out `--date` for today (scenario 5).** `delete punch 4` printed
   `mlm stop 18:48`, and even `delete note 1 -d 2026-10-25` printed `mlm note 'reviewed PR #12'`. The README example
   (`mlm delete punch 2` -> `mlm stop 13:00 --date 2026-09-10`) includes the date. If I paste the line tomorrow,
   the entry lands on the wrong day. The page and the tool disagree.

10. **Odd durations get through without comment (scenario 1).** `167h60m` is stored as 168h, and `00040h` is
    accepted. `-0h` is accepted as 0h, while `-5h` is rejected as negative. `0h` is a legal target, and the headline
    then reads `00h 00m left to 00h 00m target`, which is odd wording for "nothing due". None of these is wrong exactly,
    but `-0h` passing the negative check, and minutes over 59 being folded in, are surprising. The README doesn't say
    whether 0h is allowed or what happens with 60m and up.

11. **Changing TZ re-renders stored days strangely (scenario 4).** The same DB viewed with `TZ=UTC` lists
    `00:45-02:15` above `23:30-00:30`, all under 2026-10-25, although 23:30 UTC is the 24th. This is a niche case
    (travel, a laptop zone change), but the order is plainly wrong, and the README never says which zone times are shown in.

12. **The ongoing label on a past date isn't the documented one (scenario 3/5).** Yesterday's page showed
    `08:00-now    (35h 47m, ongoing - duration as of right now, not a running total)`. The README documents only the
    `(+ ongoing)` day-total flag and today's `so far, started yesterday, not counted until stopped` line. The extra
    wording is fine, but I couldn't find it on the page, and "not a running total" next to a number that grows every
    minute is a bit of a riddle.

13. **est. EOD on a Sunday points into next week (scenario 5).** On Sunday with 30h 57m left, status says
    `est. EOD 01:45 (Tue)`. The week ends tonight, so the target can't be met. A reasonable person would rather see
    that this week will close short than a finish time in a week that hasn't started. This follows the documented
    rule, so it's a nitpick about the rule.

Not findings (as documented and sensible): the 1680h typo is rejected and the old value kept; `| true` exits 141
quietly with nothing on stderr, matching coreutils; `| head -1` exits 0; normal-file redirects are clean and the
error goes to the terminal, not the file; the week id range errors are clear.

## Verdict

findings(13)
