# panic-fixes spec review

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

- Spec: `docs/dev/specs/2026-10-25-panic-fixes.md` (draft, `32b919d`)
- Tree: branch `panic-fixes`, source identical to `main` at `dd91952`
- Background read: `unwrap-audit-review.md` F1-F4, `export-notes-final-review.md` F1, NOTES 138-139, SPEC §2.1, §3.7, §4.1, §4.2, §6, §6.1, §6.3
- Nothing was run. Claims were checked by reading source, std (`~/.rustup/.../library/std`) and CI config.

## Checks that held

- The 168h cap does remove the F2/F4 paths for newly set targets, even with carry. A week's deficit is at most 10080 min. A carry walk from year 0000 (the earliest `--date` the parser takes, audit §5) to year 9999 (`mlm week 9999-52`) is about 5.2e5 weeks, so `|carry|` stays under about 5.3e9 min. Worked minutes are bounded by real elapsed time. That is far below the `DateTime + TimeDelta` limit (about 1.37e11 min from 2026, chrono `datetime/mod.rs:1529`) and the `TimeDelta::minutes` limit (1.54e14). `est_eod` (`src/status.rs:367`) and the `i64` sums at `src/week.rs:185-187`, `src/render.rs:66,94,112` and `src/status.rs:385` cannot overflow. Stored targets are a separate story (finding 2).
- Exit 141 is reachable on both platforms. Rust sets `SIGPIPE` to ignore on Unix (`std/src/sys/pal/unix/mod.rs:33-36`), so a closed reader gives `EPIPE`, which is `ErrorKind::BrokenPipe`. On Windows, std maps both `ERROR_BROKEN_PIPE` and `ERROR_NO_DATA` to `BrokenPipe` (`std/src/sys/io/error/windows.rs:23,29`). Every stdout site can return the error once it stops using `print!`/`println!` (`src/status.rs:527`, `src/week_view.rs:329`, `src/export.rs:52`, `src/week_target.rs:79`, `src/commands.rs:198,211,296,305,328,337`).
- "Nothing on stderr" is reachable. No stdout-writing command also writes to stderr on its normal path. `start`/`stop` warnings already use `let _ = writeln!` (`src/commands.rs:94,112`).

## Findings

### 1. High. Fixing `now` in `main.rs` does not make `start`/`stop` store the right instant in the second pass

- Category: factual error about the codebase, missing edge case
- Claim: spec:43-47 says "the earlier pass and the later pass are two different instants" and "Every command works in that hour as in any other". spec:83-85 tests that the two passes "store different instants".
- Evidence: an untyped `start`/`stop` does not store `now`. It takes the wall time, `None if target_date == today => now.time()` (`src/commands.rs:383-384`), and passes it to `storage::insert_punch_with_note` (`src/commands.rs:390-398`). That function re-resolves the wall time through `to_utc_and_date`, which maps `Ambiguous(earliest, _)` to `earliest` (`src/storage.rs:217-222`). So once `main.rs:37-41` stops panicking, an untyped `mlm start` in the second pass stores the first-pass instant, one hour before the real now. The audit's suggested fix (truncate in UTC, `unwrap-audit-review.md` F1) changes only `main.rs`, so an implementer who follows it ships this bug.
- Further effect: with only the main.rs fix, a first-pass `mlm start` at 02:30 followed by a second-pass `mlm stop` at 02:10 stores the stop at 00:10 UTC, before the 00:30 UTC start. Status then shows an orphaned end and an open start.
- Fix: §3 should say that an omitted `TIME` on today stores `now`'s instant directly and does not re-resolve the wall clock. It should name `src/commands.rs:383` and `storage::insert_punch_with_note` as sites that change. Notes already use the instant (`now.with_timezone(&Utc)`, `src/commands.rs:396`), so only punches need this.

### 2. Medium. "None of these panic" contradicts "no change to targets already stored"

- Category: internal inconsistency
- Claim: spec:20 says "After this changeset none of these panic." spec:26-27 says "No change to targets already stored. The cap applies when a target is set." spec:35 says there are no known issues.
- Evidence: a stored `target_minutes` is read back unchecked (`src/week.rs:145-149`) and still feeds `src/status.rs:367` and `src/week.rs:185-187`. A database that already holds `99999999999h` still panics in `status` with an open stint, and still wraps silently in release `week` totals.
- Consequence the user sees: the panic and garbage totals stay until the user overwrites that week's target. No message tells them to.
- Fix: narrow §1's claim to newly set targets and list the stored-target case in §1.2. Another option is a read-side clamp or error, but that is a new mechanism and the user's call.

### 3. Medium. The write-failure error does not fit §6.1's "reject, no write"

- Category: internal inconsistency (with SPEC)
- Claim: spec:108 says "§6.1 gains ... the write-failure error". spec:78-79 says a write command keeps its committed change.
- Evidence: SPEC §6.1 is headed "Hard errors (reject, no write)" (`docs/dev/SPEC.md:671`). The §6 intro defines hard errors as "nonzero exit, nothing written" (`docs/dev/SPEC.md:662-664`). A stdout write failure after `week target` or `delete punch N` commits is a nonzero exit with a write made.
- Fix: put the write failure in its own §6 subsection, or carve it out of §6.1's heading and intro.

### 4. Medium. Help, version and the swapped-target message are not covered

- Category: missing edge case, error-handling gap
- Claim: spec:17-18 says "Any command whose stdout closes early". spec:70 says "Any other stdout write failure: one line on stderr, exit 1".
- Evidence: help and version print through `let _ = err.print()` (`src/cli.rs:363-366`), which drops every error. So `mlm --help | head -1` exits 0, and `mlm --help >/dev/full` exits 0 with nothing on stderr. Neither gives the 141 or the `error:` line the spec promises. The swapped-target message uses `eprintln!` (`src/cli.rs:367`), which panics when stderr fails. That conflicts with spec:76 ("A failed write to stderr is ignored"), and spec §5 does not name the site.
- Fix: say whether clap's help and version output is in scope. If it is not, record the exit 0 for help as a known issue. Name `src/cli.rs:367` as a stderr site that changes.

### 5. Medium. The repeated-hour test has no stated mechanism and cannot run on Windows CI

- Category: testing gap
- Claim: spec:83-85 says a command in the repeated hour exits 0 and stores the correct instant, and the two passes store different instants.
- Evidence:
  - The code has no clock injection. `main.rs:37` reads `Local::now()`, and no `MLM_NOW`-style override exists (grep of `src/` and `tests/` finds none).
  - A process-level test has to build a POSIX `TZ` rule whose fall-back falls around the real current time, as the audit did (`unwrap-audit-review.md` F1 proof). A fixed `M10.5.0` rule only works on one day a year. A `Jn`/day-of-year rule built from today's date works, but the time has to be computed carefully near midnight.
  - Proving the second pass needs a second `TZ` rule in which now sits in the later pass. One run cannot cover both passes.
  - Comparing against the test's own `Utc::now()` races the minute boundary.
  - chrono's Windows `Local` ignores `TZ`, and CI runs `cargo test` on `windows-latest` (`.github/workflows/ci.yml:22-23,48-49`). The test needs `#[cfg(unix)]`.
- Fix: name the mechanism. One option is a pure, `TimeZone`-generic helper that truncates now and builds the punch instant, unit-tested with `chrono_tz::Europe::Warsaw` (already a dev-dependency, `Cargo.toml` `[dev-dependencies]`) at a fixed second-pass instant. Add a `cfg(unix)` binary test with a computed `TZ` rule. The unit test must cover the `commands::punch` path from finding 1, not only `main.rs`.

### 6. Low. The closed-stdout and `/dev/full` tests need platform gates and a deterministic pipe

- Category: testing gap
- Claim: spec:90-93.
- Evidence:
  - With `Stdio::piped()`, dropping `child.stdout` after `spawn` races the child's first write. Output that fits the 64 KiB pipe buffer can be written before the drop, which gives exit 0.
  - A deterministic setup closes the read end before spawning. `std::io::pipe` does that, but it is stable only since Rust 1.87, and `Cargo.toml` declares `rust-version = "1.85"`. CI builds on stable, so CI passes while the MSRV claim breaks.
  - "Closed" as in `>&-` gives exit 0, because std treats `EBADF` as success (`std/src/sys/stdio/unix.rs:92-93`). The test must mean a pipe with no reader.
  - `/dev/full` exists only on Linux, and CI also runs macOS (`ci.yml:24-27`). The test needs `#[cfg(target_os = "linux")]`. Windows and macOS then have no test of the "other failure" path.
- Fix: in §6, say "pipe whose read end is closed before spawn", note the platform gates, and either raise `rust-version` or use a 1.85-compatible pipe.

### 7. Low. The "write command keeps its write" test is vacuous for most write commands

- Category: testing gap
- Claim: spec:94 says "a write command with stdout closed keeps its write".
- Evidence: `start`, `stop` and `note` never write to stdout (`docs/dev/SPEC.md:379-380`, and `src/commands.rs:26-58` prints only to stderr). With one of them, the test passes with no fix. Only `week target` (`src/week_target.rs:79`) and `delete note|punch N` (`src/commands.rs:305,337`) write after committing. The test also does not state the expected exit code (141).
- Fix: name `week target` and `delete punch N`, and expect exit 141 plus the committed change.

### 8. Low. An `i64`-overflowing DURATION gets the format error, not the cap error

- Category: missing edge case
- Claim: spec:61-62 says a malformed value keeps its message, and the cap message covers values that are too large.
- Evidence: `checked_minutes` returns `None` on overflow, which becomes `InvalidFormat` (`src/time.rs:91-104`). `mlm week target 9999999999999999999h` therefore prints `invalid duration "...": expected a form like 20h, 33h30m, or 45m` (`src/time.rs:168-171`), while `99999999999h` prints the cap message. Both values are too large, but only one gets the cap message.
- Fix: state this case in §4, or treat overflow as over the cap.

### 9. Low. "168h, every hour in a week" is wrong for DST weeks

- Category: factual error
- Claim: spec:53 says "`168h` (every hour in a week)".
- Evidence: the fall-back week has 169 real hours and the spring-forward week has 167, and this spec is partly about the fall-back hour. The cap value is fine. Only the stated reason is wrong.
- Fix: say "the hours in a standard week", or drop the parenthetical.

### 10. Low. The "No user-set target cap" non-goal reads as contradicting §4

- Category: internal inconsistency (wording)
- Claim: spec:28 says "No user-set target cap."
- Evidence: §4 adds a target cap. The non-goal means there is no user-configurable cap value, but on a first read it says there is no cap at all.
- Fix: write "The cap is fixed. No setting changes it."

### 11. Low. "Lines written before the failure stay written" can leave a partial line

- Category: missing edge case
- Claim: spec:75.
- Evidence: `status`, `week` and `export notes` each print the whole page in one call (`src/status.rs:527`, `src/week_view.rs:329`, `src/export.rs:52`). Under `> file` on a full disk, a short write can end mid-line, and the partial line stays in the file.
- Fix: say "output written before the failure stays written, possibly ending mid-line".

### Undeclared user-visible gaps

Each item below is a deliberate, correct choice whose visible effect the spec does not record. §1.2 says "None" (spec:35). The work does have visible consequences, because it changes exit codes, an error path and stored instants. Each item is a candidate for §1.2.

#### 12. `| head` now fails `pipefail` scripts, and the exit code depends on timing

- Choice: exit 141 on a broken pipe rather than 0 (spec:66-69, NOTES 139).
- Effect: `set -o pipefail; mlm export notes --month 10 | head -1` aborts the script. The same pipe on `mlm status | head -1` usually exits 0, because the page fits the pipe buffer and is written before `head` closes. So one command gives 0 or 141 depending on output size and scheduling. Under the current code these cases panic with 101, which is also nonzero, but the spec presents 141 as matching other tools without saying when it appears.

#### 13. `delete punch N` with a closed stdout commits, loses the recreate line, and exits nonzero

- Choice: the committed write stays, and only the output is lost (spec:78-79).
- Effect: the recreate line (`src/commands.rs:337-340`) is the only record of what was deleted, and it is lost. The nonzero exit invites a retry. Ids are positional and re-listed on every run (`src/commands.rs:314-336`), so a retried `delete punch 1` deletes the next punch. A `week target` retry is harmless, because it is idempotent.

#### 14. Windows reports 141 with no SIGPIPE meaning behind it

- Choice: 141 on every platform (spec:67-69).
- Effect: PowerShell `$LASTEXITCODE` and `cmd` `%ERRORLEVEL%` show 141. The stated reason (spec:68-69, "what a shell reports for a process killed by SIGPIPE") does not apply on Windows.

#### 15. A 141 exit is a normal exit, not a signal death

- Choice: exit with code 141 (spec:67).
- Effect: bash's `$?` is 141 either way. Callers that inspect the wait status do see a difference. Python `subprocess` reports `returncode == 141` rather than `-13`, and `WIFSIGNALED` is false.

#### 16. `mlm week >&-` exits 0 silently

- Choice: only a broken pipe gives 141, and only other write errors give exit 1 (spec:66-74).
- Effect: std treats `EBADF` on stdout as success (`std/src/sys/stdio/unix.rs:92-93`), so a closed fd 1 gives no output, no error and exit 0. A user who reads §5 expects 141 or exit 1.

#### 17. A stint that crosses the two passes shows its times backwards

- Choice: an untyped `now` keeps its real instant in the second pass (spec:43-46).
- Effect: start at 02:30 in the first pass and stop at 02:10 in the second, and `status` shows a stint `02:30-02:10` lasting 40m. Rendering converts each instant to local time with no offset (`src/status.rs:340-354`). The `delete punch` list shows the same backwards pair (`src/commands.rs:195-204`), with nothing to tell the two passes apart.

#### 18. Typed and untyped times disagree by an hour in the second pass

- Choice: an untyped `now` takes the real instant, while a typed `TIME` still resolves to the earlier pass (spec:47-49, `src/storage.rs:221`).
- Effect: at 02:10 in the second pass, `mlm stop` and `mlm stop 02:10` store instants an hour apart. After an untyped second-pass `start`, a typed `mlm stop 02:20` lands before the start, which leaves an orphaned end and an open start.

#### 19. The `delete punch` recreate line no longer round-trips for a second-pass punch

- Choice: as in item 18.
- Effect: `punch_recreate_line` prints `HH:MM` only (`src/commands.rs:232-243`). Running `mlm start 02:10` from it stores the first-pass instant (`src/storage.rs:221`), an hour off the deleted punch. Before this change no second-pass punch could exist, so this is new.

#### 20. One week's target cannot absorb a large surplus carry-in

- Choice: a hard cap at 168h (spec:53-54).
- Effect: a user with, say, 200h of surplus carry-in who wants to zero it with one large target cannot. They have to spread it over several weeks.

#### 21. The cap message and the overflow message disagree

- Choice: the cap check runs after the format checks (spec:61-62).
- Effect: this is the visible side of finding 8. Of two "too big" values, one gets the cap message and the other gets "expected a form like 20h". Record it here, or fix it under finding 8.

## Verdict

needs-rework

Finding 1 means §3's promised behavior and its own §6 test cannot be met by the fix the background material points to, and the spec does not name the site that has to change. Findings 2-4 make the spec contradict itself or the project spec. The rest are cheap to fold into the spec or record in §1.2.
