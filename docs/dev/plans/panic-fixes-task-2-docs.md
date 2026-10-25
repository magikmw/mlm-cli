# Task plan: panic-fixes, Task 2 (docs)

> **Archived — historical planning record.** Not authoritative. Current spec: `docs/dev/SPEC.md`. Design/process history: `docs/dev/NOTES.md`.

Changeset plan: `docs/dev/plans/panic-fixes-plan.md` (Task 2, interface contracts 1-9).
Spec: `docs/dev/specs/2026-10-25-panic-fixes.md` (locked).
Branch `panic-fixes`, planned at tip `9c8108b`.

## Read for this plan

- `docs/dev/plans/panic-fixes-plan.md`, in full
- `docs/dev/specs/2026-10-25-panic-fixes.md`, in full
- `docs/dev/plans/reports/panic-fixes-plan-review.md`, in full
- `docs/dev/plans/reports/unwrap-audit-review.md` F1-F4 (what the oversized target breaks)
- `docs/dev/NOTES.md` entries 138-142 (lines 1912-1977)
- `docs/dev/SPEC.md`, in full (1854 lines)
- `README.md`, in full (670 lines)
- `AGENTS.md`, in full, Documentation section at lines 70-83
- `clippy.toml`, `.githooks/pre-commit`
- Code read to confirm the doc claims: `src/main.rs:1-100` (the `now` capture at 37-41, `error: ` prefix at 84), `src/cli.rs:265-289` (help block, DURATION line at 278), `src/time.rs:56-180` (DURATION parse and its two messages), `src/week_target.rs:1-90` (parse-then-write order), `src/commands.rs:375-392` (untyped `TIME` uses `now.time()`), `tests/week_target_cli.rs:250-270` (help golden)
- Sibling task plan `docs/dev/plans/panic-fixes-task-1-code.md` does not exist yet (Task 1 is planned in parallel). This plan quotes the changeset plan's contracts only.

## Scope

**Owns**: `docs/dev/SPEC.md`, `README.md`, `AGENTS.md` (checked, no edit expected).

**Must not touch**:

| file | owner |
|---|---|
| every file under `src/` | Task 1 |
| every file under `tests/` | Task 1 |
| `coverage-baseline.json` | Task 1 (the hook ratchets it) |
| `docs/dev/NOTES.md` | coordinator |
| `docs/dev/specs/2026-10-25-panic-fixes.md` (archive banner) | coordinator |
| `docs/dev/plans/**` | coordinator and planners |

## Dependencies

| dependency | status | what to check against today | swap at integration |
|---|---|---|---|
| Task 1 cap message (contract 2) | not landed | the changeset plan's contract 2 text, quoted below | re-check against Task 1's completion report, or `src/week_target.rs`'s message on the integration branch |
| Task 1 help line (contract 3) | not landed | contract 3 text | re-check against `tests/week_target_cli.rs`'s help golden on the integration branch |
| Task 1 write-failure line and exit codes (contracts 4, 5) | not landed | contracts 4 and 5 | re-check against `tests/panic_fixes_cli.rs` on the integration branch |
| Task 1 module list | not landed | contract "no new top-level module" | if Task 1's report adds a module, add its line to AGENTS.md's Layout list |

Docs import nothing, so no stub is needed. The pinned strings are the interface.

## Pinned strings (byte for byte)

| contract | text |
|---|---|
| 1 cap | `168h` (10080 minutes) accepted; `168h`, `167h60m`, `10080m` accepted; `168h1m`, `10081m` rejected |
| 2 cap error | `error: target cannot exceed 168h 00m (got 200h 00m)` |
| 2 examples | `168h1m` gives `(got 168h 01m)`; `99999999999h` gives `(got 99999999999h 00m)` |
| 3 help line | `  <DURATION>  Target to set: 20h, 33h30m or 45m, up to 168h (human format, never raw minutes)` |
| 4 broken pipe | nothing on stderr, exit `141`, every platform |
| 5 other failure | `error: cannot write output: <reason>`, exit `1` |
| 5 Linux example | `error: cannot write output: No space left on device (os error 28)` |

Every inline code span holding one of these stays on one source line. A span split across a line break renders the same in Markdown but breaks the `grep` checks below.

## Facts gathered

### SPEC.md

| section | lines | current state |
|---|---|---|
| §1.2a Possible issues | 61-69 | one bullet (stop before start) |
| §2.1 Time zone, DST cases | 114-123 | two bullets, gap and typed fall-back; nothing about `now` |
| §3.7 `week target` | 362-394 | DURATION paragraph 364-368, echo 370-376, reminder 381-391, help 392-394; no cap; help not quoted line by line |
| §4.1 Parsing and conversion | 450-457 | typed `TIME` only; nothing about how `now` is taken |
| §6 intro | 662-669 | "hard errors ... (nonzero exit, nothing written, message on stderr)"; stays true, no edit |
| §6.1 DURATION bullet | 705-708 | grammar and sign only |
| §6.3 Exit codes | 752-758 | "A runtime error (a bad value, a storage failure) exits `1`." |
| §7 intro | 762-764 | says output is "plain `println!` output"; Task 1 removes the `println!` stdout sites |
| §7.4 `stop` warnings | 1571-1572 | "A failed write to stderr is ignored and never changes the exit code." Stays true, no edit |
| §8.2 last flow | 1851-1853 | E31; the next free ids are E32-E34 |

Current wording of each passage that changes is quoted in the "before" block of its step.

### README.md

| section | lines | current state |
|---|---|---|
| Possible issues | 45-53 | mirrors SPEC §1.2a's one bullet |
| Usage, output paragraph | 120-131 | says what prints where; nothing about pipes |
| `week target` | 524-563 | no cap; line 554 echo sentence and line 555 `WEEK_ID` sentence share a paragraph |

### AGENTS.md

- Layout says `src/main.rs` is "CLI entry point: parses args, dispatches, exit code" (line 67) and `src/export.rs` "prints one line per note" (65-66). Both stay true after Task 1.
- No line names `println!`, the `now` capture, or the DURATION grammar.
- Documentation section (70-83) forbids pointing SPEC.md or README.md at `docs/dev/plans/` or `docs/dev/specs/` for behavior. Today neither file holds such a pointer (`grep 'plans/\|specs/'` hits only AGENTS.md:76-77, the rule itself).
- Result: no AGENTS.md edit, unless Task 1 adds a module (dependency table).

### Code facts behind the doc text

- `now` today is `Local::now().with_second(0).unwrap().with_nanosecond(0).unwrap()` (`src/main.rs:37-41`), captured once and passed to every command (`src/main.rs:43`, `dispatch`).
- An untyped `TIME` on today's date uses `now.time()` (`src/commands.rs:384`), then storage re-resolves that wall time and takes the earlier pass (plan review, `src/storage.rs:217-222`).
- `-5h` prints `duration cannot be negative (got 300 minutes)`; `10x` and `9999999999999999999h` print `invalid duration "...": expected a form like 20h, 33h30m, or 45m` (`src/time.rs:165-175`, `digits` at 99-104). The docs say "keep their own messages" and quote neither, so Task 1 is free to leave them as they are.
- `main` prefixes every error with `error: ` (`src/main.rs:84`).
- The help DURATION line lives at `src/cli.rs:278` and the golden at `tests/week_target_cli.rs:263`.

## Edits

Apply each as an exact-string replacement. Edits are independent, so their order does not matter. All new text is ASCII. In the blocks below, `~~~~` fences belong to this plan; the inner fences are part of the doc text.

### S1. SPEC §1.2a, new bullet (criterion 5)

Before (SPEC.md:67-69):

~~~~
- Entering a stop before its start warns once. `mlm stop 17:00` then
  `mlm start 09:00` is legal on any date (§3.2), but the first command
  prints an orphan warning (§7.4). The second command clears it.
~~~~

After:

~~~~
- Entering a stop before its start warns once. `mlm stop 17:00` then
  `mlm start 09:00` is legal on any date (§3.2), but the first command
  prints an orphan warning (§7.4). The second command clears it.
- A week target above `168h` stored before the cap existed (§3.7)
  stays stored, and reads never clamp it. In that week, and in later
  weeks through carry-in, `status` with a stint open today can panic on
  the `est. EOD` line, and the week totals can overflow into wrong
  figures. Setting that week's target again, at `168h` or less,
  replaces it.
~~~~

### S2. SPEC §2.1, DST cases (criterion 1)

Before (SPEC.md:114-123):

~~~~
Two DST edge cases this raises, both resolved explicitly rather than
left to whatever the conversion library happens to do by default:

- **Spring-forward gap**: a typed local `TIME` that doesn't correspond
  to any real local instant on today's date (the hour skipped when
  clocks jump forward) is a hard error (§6.1) — nothing written, same
  treatment as the `24:00` boundary case.
- **Fall-back ambiguity**: a typed local `TIME` that occurs *twice*
  (the repeated hour when clocks fall back) resolves to the **earlier**
  of the two real instants.
~~~~

After:

~~~~
Three DST edge cases this raises, all resolved explicitly rather than
left to whatever the conversion library happens to do by default:

- **Spring-forward gap**: a typed local `TIME` that doesn't correspond
  to any real local instant on today's date (the hour skipped when
  clocks jump forward) is a hard error (§6.1) — nothing written, same
  treatment as the `24:00` boundary case.
- **Fall-back ambiguity**: a local time that occurs *twice* (the
  repeated hour when clocks fall back) resolves to the **earlier** of
  the two real instants. This holds for a typed `TIME` and for the
  current time that a `start` or `stop` without `TIME` stores (§4.1).
  So a punch made in the second pass, typed or not, stores the
  first-pass instant, earlier than the real clock by the length of the
  shift (one hour in most zones).
- **Current time in the repeated hour**: taking the current time never
  fails. `now` is the current instant truncated to the whole minute
  (§4.1), in the repeated hour as in any other, so every command works
  during it.
~~~~

### S3. SPEC §3.7, cap paragraph (criterion 2)

Insert after the DURATION paragraph. Before (SPEC.md:364-372):

~~~~
Set an absolute target override for a week. `WEEK_ID` (same accepted
forms as §3.6) defaults to the current week when omitted. `DURATION`
uses the human duration format (§4.2), not raw minutes — e.g. `20h`,
`33h30m` — so setting a target has the same friction as everything
else here: no mental-math input.

On success it prints one line to stdout, with the week id it wrote to
~~~~

After:

~~~~
Set an absolute target override for a week. `WEEK_ID` (same accepted
forms as §3.6) defaults to the current week when omitted. `DURATION`
uses the human duration format (§4.2), not raw minutes — e.g. `20h`,
`33h30m` — so setting a target has the same friction as everything
else here: no mental-math input.

`DURATION` is at most `168h` (10080 minutes, the hours in a week).
`168h`, `167h60m` and `10080m` are accepted, and `168h1m` and `10081m`
are not. A larger value fails with one line on stderr and exit `1`,
stdout stays empty, and no target is written:

```
error: target cannot exceed 168h 00m (got 200h 00m)
```

The `got` value is the parsed duration in the standard format (§4.2),
so `168h1m` prints `(got 168h 01m)` and `99999999999h` prints
`(got 99999999999h 00m)`. A week that already has a target keeps it.
The checks run in this order: `WEEK_ID`, then the `DURATION` grammar
and sign (§6.1), then the cap. So a malformed value (`10x`), a negative
one (`-5h`) and one too large to parse at all (`9999999999999999999h`)
keep their own messages. The `WEEK_ID`-before-`target` reminder below
comes before every check, so `mlm week 2026-41 target 200h` prints the
reminder and exits `2`. A target stored above `168h` before the cap
existed stays as it is (§1.2a).

On success it prints one line to stdout, with the week id it wrote to
~~~~

### S4. SPEC §3.7, help line (criterion 2)

Before (SPEC.md:392-394):

~~~~
`mlm week target --help` shows the usage `mlm week target [OPTIONS]
[WEEK_ID] <DURATION>` and an Arguments section naming both arguments. No
parser error for this subcommand mentions `<ARGS>`.
~~~~

After:

~~~~
`mlm week target --help` shows the usage `mlm week target [OPTIONS]
[WEEK_ID] <DURATION>` and an Arguments section naming both arguments.
Its `DURATION` line names the cap:

```
  <DURATION>  Target to set: 20h, 33h30m or 45m, up to 168h (human format, never raw minutes)
```

No parser error for this subcommand mentions `<ARGS>`.
~~~~

The code-block line is contract 3 verbatim, two leading spaces included.

### S5. SPEC §4.1, how `now` is taken (criterion 1)

Before (SPEC.md:452-457):

~~~~
A `TIME` argument (§3.1) is parsed as a local wall-clock hour/minute,
combined with the target date (local *today* by default) to form a
local datetime, then converted to UTC for storage per-instant (§2.1).
There is no seconds precision anywhere —
everything is minute-granular, per NOTES.md's "keep time in minutes"
call.
~~~~

After:

~~~~
A `TIME` argument (§3.1) is parsed as a local wall-clock hour/minute,
combined with the target date (local *today* by default) to form a
local datetime, then converted to UTC for storage per-instant (§2.1).
There is no seconds precision anywhere —
everything is minute-granular, per NOTES.md's "keep time in minutes"
call.

`now` is the current instant truncated to the whole minute, taken once
per command. The truncation drops seconds from the instant itself and
never rebuilds a local wall time, so taking the current time never
fails, the repeated autumn hour included (§2.1). A `start` or `stop`
without `TIME` takes `now`'s local hour and minute on today's date and
converts them the same way as a typed `TIME`. In the second pass of the
repeated hour that local time resolves to the earlier instant (§2.1),
so the punch stores the first-pass instant, as a typed `TIME` does.
~~~~

### S6. SPEC §6.1, DURATION bullet (criterion 3)

Before (SPEC.md:705-708):

~~~~
- Malformed `DURATION` (§3.7/§4.2): doesn't match the input grammar,
  or parses to a negative number of minutes. Zero is legal (a
  deliberate week off, see §2.3's `week_targets` note) — only
  negative is rejected.
~~~~

After:

~~~~
- Malformed `DURATION` (§3.7/§4.2): doesn't match the input grammar,
  or parses to a negative number of minutes. Zero is legal (a
  deliberate week off, see §2.3's `week_targets` note) — only
  negative is rejected. A value too large to parse at all
  (`9999999999999999999h`) fails the grammar check. A well-formed
  `DURATION` above `168h` fails the cap (§3.7), checked after the
  grammar and the sign:

  `error: target cannot exceed 168h 00m (got 200h 00m)`

  `got` is the parsed value in the standard format (§4.2). `168h`
  itself is accepted.
~~~~

### S7. SPEC §6.3, exit codes and write failures (criterion 4)

Before (SPEC.md:752-758):

~~~~
### 6.3 Exit codes

`0` on success (including a status view that reports anomalies, since
the anomaly is data to look at and not a tool failure). Nonzero on any
§6.1 hard error. A runtime error (a bad value, a storage failure) exits
`1`. A usage error from the argument parser (an unknown flag, a missing
argument, the `week WEEK_ID target DURATION` shape) exits `2`.
~~~~

After:

~~~~
### 6.3 Exit codes

`0` on success (including a status view that reports anomalies, since
the anomaly is data to look at and not a tool failure). Nonzero on any
§6.1 hard error. A runtime error (a bad value, a storage failure, a
failed output write) exits `1`, except a broken pipe on stdout, which
exits `141` (below). A usage error from the argument parser (an unknown
flag, a missing argument, the `week WEEK_ID target DURATION` shape)
exits `2`.

A failed write to stdout is a runtime error, not a §6.1 rejection,
because a write command may already have committed its change:

- Stdout closed by its reader (a broken pipe): the command stops
  writing, prints nothing to stderr, and exits `141`, on every
  platform. `141` is what a shell reports for a process killed by
  `SIGPIPE`, so `set -o pipefail` scripts see the same code as from
  other tools.
- Any other stdout write failure: one line on stderr, exit `1`:

  `error: cannot write output: <reason>`

  `<reason>` is the I/O error's standard text, `(os error N)` suffix
  included. On Linux, stdout on `/dev/full` prints:

  `error: cannot write output: No space left on device (os error 28)`

- Lines written before the failure stay written.
- A write command that already committed its change keeps it. Only
  the output is lost. `week target` keeps the stored target, and
  `delete note|punch N` keeps the deletion.

A failed write to stderr is ignored at every stderr site: the `start`
and `stop` warnings (§7.4), `error:` lines, and the
`week WEEK_ID target` reminder (§3.7). The exit code is the one the
command would have returned.
~~~~

Notes on S7:

- The §6 intro (662-669) still describes hard errors and anomalies. The write failure is filed under §6.3 as a runtime error, so the intro's "nothing written, message on stderr" never covers it.
- §6.1's heading ("reject, no write") gains nothing about write failures.
- The bullet colon before the error line introduces a quoted line, the house style in §6.1's MONTH bullet.

### S8. SPEC §7 intro, `println!` wording (stale after Task 1)

Before (SPEC.md:762-764):

~~~~
Plain ASCII throughout (no box-drawing/unicode dashes) — matches the
portability goal behind the ratatui pick (README) even though this is
plain `println!` output, not a TUI screen.
~~~~

After:

~~~~
Plain ASCII throughout (no box-drawing/unicode dashes) — matches the
portability goal behind the ratatui pick (README) even though this is
plain line output, not a TUI screen.
~~~~

### S9. SPEC §8.2, new flows E32-E34 (criterion 6)

Append after E31 (SPEC.md:1851-1853, the file's last lines). Before:

~~~~
- **E31** - `mlm export notes` read failure (§6.1): with the
  `notes.body` column renamed, it prints `error: ...` to stderr,
  nothing to stdout, and exits `1`.
~~~~

After:

~~~~
- **E31** - `mlm export notes` read failure (§6.1): with the
  `notes.body` column renamed, it prints `error: ...` to stderr,
  nothing to stdout, and exits `1`.
- **E32** - Week target cap (§3.7, §6.1): `mlm week target 168h` and
  `mlm week target 10080m` print the echo line with `168h 00m`.
  `168h1m`, `200h` and `99999999999h` print the cap error with
  `(got 168h 01m)`, `(got 200h 00m)` and `(got 99999999999h 00m)`,
  exit `1`, print nothing to stdout, and write no target. A week that
  already had a target keeps it. `-5h`, `10x` and
  `9999999999999999999h` keep their own messages.
  `mlm week 2026-41 target 200h` prints the reminder of E24 and exits
  `2`. `status` with a stint open today and a `168h` target exits `0`
  and shows its `est. EOD` line.
- **E33** - Stdout closed by its reader (§6.3): with stdout a pipe
  whose reader has already closed, `mlm week`, `mlm status` and
  `mlm export notes` over a range with at least one note print nothing
  to stderr and exit `141`. `mlm week target 40h` and `mlm delete punch 1`
  under the same pipe exit `141`, and the target is stored and the
  punch is gone.
- **E34** - Other write failures (§6.3): `mlm week` with stdout on
  `/dev/full` (Linux) prints this on stderr and exits `1`:

  `error: cannot write output: No space left on device (os error 28)`

  With stderr a closed pipe, a malformed `TIME` still exits `1` and the
  `week WEEK_ID target` shape still exits `2`.
~~~~

E32 quotes contract 2's message by its `got` parts only, because the full line already sits in §3.7 and §6.1. The three `(got ...)` spans each stay on one line.

### S10. README Possible issues, new bullet (mirrors S1)

Before (README.md:50-53):

~~~~
- Entering a stop before its start warns once. `mlm stop 17:00` then
  `mlm start 09:00` is fine on any date, but the first command prints
  an orphaned-end warning (see `mlm stop` below). The second command
  clears it.
~~~~

After:

~~~~
- Entering a stop before its start warns once. `mlm stop 17:00` then
  `mlm start 09:00` is fine on any date, but the first command prints
  an orphaned-end warning (see `mlm stop` below). The second command
  clears it.
- A week target above 168h, set before `week target` had a limit, stays
  stored. With a stint open, `status` can crash on it, and week totals
  can come out wrong, in that week and later ones through carry-in.
  Setting that week's target again, at 168h or less, fixes it.
~~~~

### S11. README Usage, piping paragraph (criterion 7)

Insert after the output paragraph. Before (README.md:129-133):

~~~~
correctly. Add `-v`/`--verbose` to any command to see debug log lines
on stderr, including the one-time migration line.

Every command has a 1 character alias, except `delete` (`del`) and `export` (`ex`).
~~~~

After:

~~~~
correctly. Add `-v`/`--verbose` to any command to see debug log lines
on stderr, including the one-time migration line.

Output pipes and redirects like any other command's. If the reader
closes the pipe before `mlm` finishes writing, `mlm` stops quietly and
exits 141, the code a shell reports for a process killed by a closed
pipe, so `set -o pipefail` scripts see what they see from other tools.
Any other write failure, such as a full disk under `> file`, prints one
`error: cannot write output: <reason>` line and exits 1. A change the
command already made, such as a new week target or a deletion, stays.

Every command has a 1 character alias, except `delete` (`del`) and `export` (`ex`).
~~~~

### S12. README `week target`, cap and message (criterion 7)

Before (README.md:554-561):

~~~~
The duration echoes in the standard format, so `90m` prints as `01h 30m`.
`WEEK_ID` goes after `target`. Typing it before `target` fails with a
reminder and exit code 2:

```sh
$ mlm week 2026-41 target 20h
error: put WEEK_ID after target: mlm week target 2026-41 20h
```
~~~~

After:

~~~~
The duration echoes in the standard format, so `90m` prints as `01h 30m`.

The target is at most 168h, the hours in a week. A larger value fails
with exit code 1 and leaves the week's target as it was:

```sh
$ mlm week target 200h
error: target cannot exceed 168h 00m (got 200h 00m)
```

`WEEK_ID` goes after `target`. Typing it before `target` fails with a
reminder and exit code 2, even when the duration is over the limit:

```sh
$ mlm week 2026-41 target 20h
error: put WEEK_ID after target: mlm week target 2026-41 20h
```
~~~~

README line 563 (`mlm week target --help` shows the arguments as `[WEEK_ID] <DURATION>`) stays as is. It quotes no DURATION help text, so contract 3 does not bind it.

### S13. AGENTS.md (criterion 8)

No edit. Re-check at integration: if Task 1's completion report names a new top-level module, add one Layout line for it in the existing style and record the change in the commit message.

## Commit

One commit on the Task 2 worktree branch, touching only `docs/dev/SPEC.md` and `README.md`. Message:

```
docs: fold panic-fixes into SPEC and README
```

The pre-commit hook runs fmt, clippy and `cargo llvm-cov` even for a docs-only commit. No source changes, so coverage reads "unchanged" and the gate passes.

## Check plan

Run from the Task 2 worktree root after all edits.

### C1. §2.1 and §4.1 describe `now` (criterion 1; spec §3)

```sh
grep -n 'taking the current time never' docs/dev/SPEC.md        # 2 hits: §2.1, §4.1
grep -n 'truncated to the whole minute' docs/dev/SPEC.md        # 2 hits: §2.1, §4.1
grep -n 'first-pass instant' docs/dev/SPEC.md                   # 2 hits: §2.1, §4.1
```

Read §2.1 and §4.1 once in full. Both say an untyped `start`/`stop` stores the first-pass instant, as a typed `TIME` does.

### C2. §3.7 cap, order, help line (criterion 2; spec §4, §7; contracts 1-3)

```sh
grep -nF 'error: target cannot exceed 168h 00m (got 200h 00m)' docs/dev/SPEC.md
grep -nF '`168h`, `167h60m` and `10080m` are accepted' docs/dev/SPEC.md
grep -nF '(got 168h 01m)' docs/dev/SPEC.md
grep -nF '(got 99999999999h 00m)' docs/dev/SPEC.md
grep -nxF '  <DURATION>  Target to set: 20h, 33h30m or 45m, up to 168h (human format, never raw minutes)' docs/dev/SPEC.md
grep -nF 'then the `DURATION` grammar' docs/dev/SPEC.md
```

The `-x` match proves the help line is whole and verbatim. The order sentence names `WEEK_ID`, grammar and sign, cap, with the reminder first.

### C3. §6.1 cap error (criterion 3; spec §4)

```sh
sed -n '/^- Malformed `DURATION`/,/^- Malformed `TIME` boundary/p' docs/dev/SPEC.md | grep -F 'error: target cannot exceed 168h 00m (got 200h 00m)'
```

One hit, inside the DURATION bullet.

### C4. §6.3 write failures (criterion 4; spec §5; contracts 4-7)

```sh
sed -n '/^### 6.3/,/^## 7\./p' docs/dev/SPEC.md > /tmp/cd-scratch/panic-fixes-task-2/63.txt
grep -c '141' /tmp/cd-scratch/panic-fixes-task-2/63.txt                         # 3 or more
grep -F 'error: cannot write output: <reason>' /tmp/cd-scratch/panic-fixes-task-2/63.txt
grep -F 'error: cannot write output: No space left on device (os error 28)' /tmp/cd-scratch/panic-fixes-task-2/63.txt
grep -F 'except a broken pipe on stdout' /tmp/cd-scratch/panic-fixes-task-2/63.txt
grep -F 'A failed write to stderr is ignored at every stderr site' /tmp/cd-scratch/panic-fixes-task-2/63.txt
grep -c '141' docs/dev/SPEC.md       # §6.3 and E33 only; never inside §6.1
sed -n '/^### 6.1/,/^### 6.2/p' docs/dev/SPEC.md | grep -c 'cannot write output\|141'   # 0
```

(Create `/tmp/cd-scratch/panic-fixes-task-2/` first.) Then read §6 intro and §6.1's heading. Both stay true, because the write failure appears only in §6.3.

### C5. §1.2a stored oversized targets (criterion 5; spec "Interaction", §1.1)

```sh
sed -n '/^### 1.2a/,/^### 1.3/p' docs/dev/SPEC.md | grep -F 'above `168h`'
```

One hit. The bullet names the `est. EOD` panic, the totals overflow, and the fix (set the target again).

### C6. §8.2 flows (criterion 6; spec §6 cases)

```sh
grep -n '^- \*\*E3[234]\*\*' docs/dev/SPEC.md     # 3 hits, after E31
```

E32 covers spec §6's cap cases (`168h` accepted, `168h1m` and `200h` rejected, `-5h` and `10x` unchanged, the `168h` `status` case). E33 covers the closed-pipe cases and the `week target 40h` keeps-target case. E34 covers `/dev/full`.

### C7. README cap and piping (criterion 7; spec §7)

```sh
grep -nF 'error: target cannot exceed 168h 00m (got 200h 00m)' README.md
grep -n 'at most 168h' README.md
grep -nF 'exits 141' README.md
grep -nF 'error: cannot write output: <reason>' README.md
```

One hit each.

### C8. AGENTS.md unchanged (criterion 8)

```sh
git diff --stat -- AGENTS.md     # empty
```

### C9. Contract strings byte for byte (criterion 9)

Run every `grep -F`/`-x` above with the pinned strings copied from the "Pinned strings" table. Also:

```sh
git diff -U0 -- docs/dev/SPEC.md README.md | grep '^+' | grep -nP '[^\x00-\x7F]'   # no hits: new text is ASCII
grep -n 'plans/\|specs/' docs/dev/SPEC.md README.md                             # no hits
git diff --name-only     # docs/dev/SPEC.md and README.md only
```

The `§` signs in new SPEC text are an exception to the ASCII grep, because SPEC.md already uses them in every cross-reference. Filter them with `grep -nP '[^\x00-\x7F§]'` instead.

### C10. Integration re-check (after Task 1 merges)

```sh
git show <integration-branch>:tests/week_target_cli.rs | grep -F 'up to 168h'
git show <integration-branch>:src/week_target.rs | grep -F 'target cannot exceed'
git show <integration-branch>:tests/panic_fixes_cli.rs | grep -F 'cannot write output'
```

Each string must match the doc text. A mismatch goes back to the coordinator. The docs follow the contracts, not a deviating implementation.

## Risks and ambiguities

- **README Possible issues bullet (S10).** The changeset plan's criteria name only §1.2a. README's "Possible issues" section mirrors §1.2a today, so this plan adds the same item there. The coordinator can drop S10 without touching any other step.
- **SPEC §7 intro (S8).** No criterion names it. Task 1 removes the `println!` stdout sites, so "plain `println!` output" goes stale. S8 rewords two words.
- **U2 not documented.** NOTES 142 records that a stint whose untyped start and stop fall in different passes of the repeated hour can store an end before its start. S2 and S5 state the first-pass rule, which implies it, but no doc says so outright. Adding a §1.2a bullet is a coordinator call. This plan adds none.
- **U1 stated as a fact.** S2 says a second-pass punch lands earlier than the real clock by the shift. That is the direct meaning of "stores the first-pass instant" and adds no new behavior.
- **No repeated-hour flow in §8.** Spec "Interaction" asks for §8.2 flows for the cap and write failures only. The repeated-hour test lives in Task 1, and §8 gains no flow for it.
- **Configurable cap.** Spec §1.1 names it a non-goal. SPEC §1.2 could list it as a deferred feature, but neither the plan nor the spec's "Interaction" asks for it. Left out.
- **Pre-existing stale line.** README.md:61 says "1.85+". The crate already needs 1.88 (plan review F3). That is outside this changeset and left alone.
- **Task 1 deviation.** If Task 1 ships a different string, the docs still follow the contracts and C10 flags the mismatch. The fix belongs to whichever side drifted, decided by the coordinator.
