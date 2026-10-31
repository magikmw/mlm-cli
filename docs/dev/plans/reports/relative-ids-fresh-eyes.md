# relative-ids — fresh-eyes user run

Sandbox: /tmp/cd-sandbox/relative-ids/ (`mlm` + `README.md`). DB: `MLM_DB_PATH=/tmp/cd-sandbox/relative-ids/test.db`
(plus a second scratch DB `t2.db` for one check). Today on the machine: **Sat 2026-10-31, ISO week 2026-44**.
I had no way to fake the date (`faketime` isn't installed), so "last Monday" was played on a Saturday. The
relative forms still meant the same thing (`-1` = 2026-43, `-2` = 2026-42, `-m -1` = 2026-09).

Seeded data: 09:00-17:00 on Mon-Fri of week 42; 09:00-16:00 plus a note on Mon-Thu of week 43 (the short week);
09:00-17:00 plus a note on Mon-Fri of week 44; today 09:00-12:00 with note "today"; and two note-only
entries in September (2026-09-15, 2026-09-30).

## Scenarios — what you ran, what it printed

### 1. Look back at last week and the week before, then last week with notes
- `mlm week -1` printed `Week 2026-43 (2026-10-19 - 2026-10-25)` with the `Total behind:` headline, the 4x07h rows, then `Carry-in: -160h 00m`. Exit 0.
- `mlm week -2` printed `Week 2026-42 (2026-10-12 - 2026-10-18)` correctly. Exit 0.
- `mlm week -1 -n`, `mlm week -n -1` and `mlm week --notes -2` all worked, and the notes showed indented under their rows as the README describes.
- The headline figures were huge (`Total behind: 172h 00m`, carry-in -160h). The cause is the September note-only entries, covered in finding 1.
- My own slip, `mlm week w -1`, printed `error: unexpected argument '-1' found  tip: to pass '-1' as a value, use '-- -1'` and exited 2.

### 2. Set last week's target to 32h
- Before: `mlm week` read `169h 00m left to 40h 00m target`, Carry-in -172h. `status` read `Week 2026-44:  169h 00m left to 40h 00m target (fulfillment -129h 00m = worked 43h 00m + carry-in -172h 00m / target 40h 00m)`.
- `mlm week target -1 32h` printed `Week 2026-43 target: 32h 00m`. Exit 0. Naming the resolved week was reassuring.
- `mlm week -1` then showed `Target: 32h 00m` and `Total behind: 164h 00m`.
- After: `mlm week` read `161h 00m left to 40h 00m target`, Carry-in -164h. `status` showed the same numbers. Both moved by 8h, as expected.

### 3. Notes for the monthly and weekly reports, with slips
Things that worked:
- `mlm export notes -m -1`, `--month -1`, `-m 9`, `-m 09`, `-m 2026-09`, `-m=-1`, `--month=-1` and `-m-1` all printed the two September notes. Exit 0.
- `mlm export notes -w -1`, `--week -1` and `-w-1` all printed the four week-43 notes. Exit 0.
- `-m -01` and `mlm week -01` were accepted as `-1`.

Slips:
| typed | printed | exit |
|---|---|---|
| `mlm week -0` | `error: invalid WEEK_ID "-0": expected YYYY-WW, WW or -N (N >= 1)` | 1 |
| `mlm week -00` | same | 1 |
| `mlm week +1` / `+0` / `+3` | `error: invalid WEEK_ID "+1": expected YYYY-WW, WW or -N (N >= 1); a future week takes YYYY-WW or WW` | 1 |
| `mlm week -1x` / `-1w` | `error: unexpected argument '-1' found` / `tip: to pass '-1' as a value, use '-- -1'` | 2 |
| `mlm week -- -1x` | `error: invalid WEEK_ID "-1x": expected ...` | 1 |
| `mlm week x1` | `invalid WEEK_ID "x1"` | 1 |
| `mlm week --1` | clap `unexpected argument '--1'` | 2 |
| `mlm week - 1` | `error: the subcommand '1' cannot be used with '[WEEK_ID]'` | 2 |
| `mlm week 0` | `week number must be 1 or greater` | 1 |
| `mlm week 54` | `2026 has only 53 ISO weeks` | 1 |
| `mlm week -999` | Week 2007-36, `Total behind: 40h 00m` | 0 |
| `mlm week target -1 32` | `error: invalid duration "32": expected a form like 20h, 33h30m, or 45m` | 1 |
| `mlm week target -1 32H` | same, for `"32H"` | 1 |
| `mlm week target -1 32 h` | clap `unexpected value 'h' for '[WEEK_ID] <DURATION>'` | 2 |
| `mlm week target -1` | missing `<DURATION>` usage error | 2 |
| `mlm week target -0 32h` | `invalid WEEK_ID "-0"` | 1 |
| `mlm week target +1 32h` | `invalid WEEK_ID "+1" ... a future week takes YYYY-WW or WW` | 1 |
| `mlm week target -1x 32h` | `error: invalid WEEK_ID "-1x": expected YYYY-WW, WW or -N (N >= 1)` | **1** |
| `mlm week target 1 32h` (dropped the minus) | `Week 2026-01 target: 32h 00m`, a silent change to week 1 | 0 |
| `mlm week target -1 0h` | `Week 2026-43 target: 00h 00m` (accepted) | 0 |
| `mlm week -1 target 32h` | `error: put WEEK_ID after target: mlm week target -1 32h` (good) | 2 |
| `mlm export notes -m -0` | `invalid MONTH "-0": expected YYYY-MM, MM or -N (N >= 1)` | 1 |
| `mlm export notes -m +1` | `... a future month takes YYYY-MM or MM` | 1 |
| `mlm export notes -m -1x` / `-m 1x` / `-m -1m` | `invalid MONTH "..."` | **1** |
| `mlm export notes -m 0` / `-m 13` | `month must be 1-12` | 1 |
| `mlm export notes -w -1x` | `invalid WEEK_ID "-1x"` | **1** |
| `mlm export notes -d -0` | `invalid DATE "-0": expected YYYY-MM-DD or -N (N >= 1)` | 1 |
| `mlm export notes -d +1` | `invalid DATE "+1": expected YYYY-MM-DD or -N (N >= 1)`, with no future-date hint | 1 |
| `mlm export notes -w -1 -m -1` | `cannot be used with`, and the usage line reads `mlm export notes --week <WEEK_ID>` | 2 |
| `mlm export notes -1` | `unexpected argument '-1'` | 2 |
| `mlm status -1x` | clap `unexpected argument '-1' found` + tip | 2 |
| `mlm status -0` / `+1` | `invalid DATE` | 1 |

After testing, I put the targets back: week 2026-01 and 2026-45 to 40h, week 2026-43 to 32h.

### 4. A few weeks ahead
- `mlm week +1` and `+3` are rejected. The message says to use `YYYY-WW or WW`, but not which id next week is.
- `mlm week 45`, `47`, `2026-47`, `53`, `2026-53` and `2027-01` all worked. A not-yet-started week shows `Total behind:` with a growing figure (201h, 281h, ... 561h), because empty future weeks keep piling 40h onto the deficit.
- `mlm week target 45 32h` worked: `Week 2026-45 target: 32h 00m`.

### 5. Help for each command used
- `mlm week -h/--help`, `mlm week target --help` and `mlm export notes --help` all list the `-N` form with the `-1` example, and say a bare number means a week or month number.
- None of them says that a bare number means the current year, or that `-0` and `+N` are refused. None says how to reach a future week.
- `mlm export notes --help` doesn't say the three range flags are mutually exclusive.
- `mlm status --help` documents only `-N` days.

## Findings

1. **One stray note five weeks back creates a -160h carry-in (seen in scenarios 1 and 2).** I expected last week to read about `-12h behind` (28h worked against 40h). It read `Total behind: 172h 00m` with `Carry-in: -160h 00m`. A clean second DB confirmed the cause. With week 42 alone, carry-in was 00h 00m. After adding a single `mlm note -d 2026-09-15 "x"`, carry-in became -160h: every week from that note's week onward counts as a 40h shortfall. The README explains carry-in but never says from which week it starts counting, or that a note with no punches anchors it. Someone who backfills one old note for a report suddenly sees their balance drop by hundreds of hours, with no `[!]` and no hint why. This is probably not new to relative-ids, but a relative lookback is exactly where you first notice it.

2. **`-1x` (and `-1w`, `-1 -1`, `2026-43 -1`) gives a baffling clap error naming `-1` (scenario 3).** For `mlm week -1x` I expected `invalid WEEK_ID "-1x"`. I got `error: unexpected argument '-1' found` with the tip `use '-- -1'`. The README and help both say `-1` is valid, so being told `-1` is unexpected is contradictory. The tip suggests a different command from the one I typed, and following it (`mlm week -- -1`) quietly shows last week, whether or not that's what I meant. `mlm status -1x` does the same. The README only promises "a usage error with exit code 2", which is technically true, but the message misleads.

3. **The same slip gets different exit codes and messages depending on where it's typed (scenario 3).** `mlm week -1x` and `mlm status -1x` exit 2 with the clap message. `mlm week target -1x 32h`, `mlm export notes -w -1x`, `-m -1x` and `mlm week -- -1x` exit 1 with a clear `invalid WEEK_ID/MONTH "-1x"`. `-0`, `+1` and `x1` exit 1 everywhere. The README says "-1x is a usage error with exit code 2" for `week` and implies the same for `week target` ("same forms"), so the `week target` and `export` behaviour contradicts the page. A script checking for 2 would miss these.

4. **`+N` is refused, but you're left to work out the week number yourself (scenario 4).** The whole point of `-N` was not having to work out week numbers. Looking ahead, `mlm week +1` fails with `a future week takes YYYY-WW or WW` and doesn't name the week I meant (e.g. "next week is 2026-45"). The error already knows the current week, so naming the target id would give me a next step. The README lists `+N` under Planned, which helps, but `mlm week --help` never mentions it. Near New Year the bare `WW` route is a trap too: in late December, `mlm week 2` means week 2 of *this* year, already past, not the upcoming one.

5. **Dropping the minus silently changes a different week's target (scenario 3).** `mlm week target 1 32h` (meant `-1`) printed `Week 2026-01 target: 32h 00m` and exited 0. The line does name the week, which saved me. But there's no undo or "clear override" command, so the only fix is to guess the old value and set it again (I set 40h). The README never says how to remove an override. This is worse for weeks between data and today, because it changes carry-in for every later week.

6. **A `-0` error gives no next step (scenario 3).** `mlm week -0` says `expected ... -N (N >= 1)`. A person typing `-0` almost certainly means "this week", so a hint like "omit WEEK_ID for the current week" would help. The same applies to `-m -0` and `-d -0`. `-00` gets the same message. Meanwhile `-01` is accepted as `-1`, which is fine but undocumented.

7. **Duration without a unit gets no specific suggestion (scenario 3).** `mlm week target -1 32` printed `invalid duration "32": expected a form like 20h, 33h30m, or 45m`. That's a fine message, but it could say `did you mean 32h?`, as the README does for `40`. `32H` (capital) is also refused, though many people type it that way. `32 h` with a space gives a clap message about `'[WEEK_ID] <DURATION>'` that doesn't tell you the space is the problem. Both errors exit 1 while the other usage errors exit 2.

8. **`export notes -d +1` has no future hint, unlike week and month (scenario 3).** The week and month errors add `a future week/month takes ...`, but the date error doesn't, even though `export notes -d` explicitly allows future dates. The three flags are siblings on one command, and the inconsistency is visible.

9. **Help never says what a bare number means or what's refused (scenario 5).** `mlm week --help`, `week target --help` and `export notes --help` say "a bare week number (e.g. 7)" but not "of the current year". None mentions that `-0`/`+N` are refused or how to view a future week. The README covers these, but the help is what you check mid-task. `export notes --help` also doesn't say `-d`/`-w`/`-m` are mutually exclusive, and the conflict error's usage line (`Usage: mlm export notes --week <WEEK_ID>`) reads as if `--week` were required.

10. **A future week's headline reads like a debt that's already due (scenario 4).** `mlm week 45` (next week, not started) shows `Total behind: 201h 00m`, and `2027-01` shows 561h. The glossary covers it ("not-yet-started week"), but projecting zero work into every future week makes the look-ahead figure meaningless, and each week further out looks 40h worse. I'd expect a future week to show its carry-in and target, not a growing "behind".

11. **The `mlm week w -1` slip gives a misleading message (scenario 1).** Typing the alias twice treats `w` as a WEEK_ID and then complains `unexpected argument '-1'` with the `-- -1` tip, instead of `invalid WEEK_ID "w"`. Small, but it's the same confusing clap path as finding 2.

12. **A 0h target is accepted with no comment (scenario 3, minor).** `mlm week target -1 0h` printed `Week 2026-43 target: 00h 00m`. That's probably legitimate (a holiday week), but it's an easy slip, and since the page documents the 168h upper bound I wondered about the lower one. It isn't documented either way.

13. **I couldn't play "last Monday" (environment note, minor).** The tool has no way to set "today" (no env var or flag), so the Monday-specific behaviour from the README (the `[i]` footer for last week's Sunday) couldn't be checked from a user's seat. This isn't a defect for a user, just a coverage gap.

## Verdict

findings(13)
