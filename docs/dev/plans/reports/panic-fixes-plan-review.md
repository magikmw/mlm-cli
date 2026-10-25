# Review: panic-fixes plan

Plan: `docs/dev/plans/panic-fixes-plan.md` at `02f5b17` (branch `panic-fixes`).
Spec: `docs/dev/specs/2026-10-25-panic-fixes.md` (locked). Also read NOTES.md entries 138-141, `docs/dev/plans/reports/panic-fixes-spec-review.md`, `docs/dev/plans/reports/unwrap-audit-review.md` F1, SPEC.md §1.2a, §2.1, §3.7, §4.1, §6-§6.3, README.md's `week target` section, and the source named below.

## What checks out

- The plan names all ten stdout sites, and there are no others in `src/`: `src/week_target.rs:79`, `src/status.rs:527`, `src/week_view.rs:329`, `src/export.rs:52`, `src/commands.rs:198,211,296,305,328,337`. No `print!` remains in `examples/` that ships.
- The plan names all stderr sites. The two that can panic are `src/main.rs:84` and `src/cli.rs:367`. The warnings already ignore failure (`src/commands.rs:94,112`). env_logger 0.11.11 discards its write result (`env_logger-0.11.11/src/logger.rs:667`). Clap's `err.print()` result is already dropped (`src/cli.rs:365`).
- File scopes are exclusive. Task 1 owns every source and test file that must change. Task 2 owns only `.md` files. The only golden that holds the help line is `tests/week_target_cli.rs:263`. `src/cli.rs` tests check `<DURATION>` only (`src/cli.rs:984`).
- Pinned strings match the spec byte for byte. Contract 2 matches spec:65. Contract 3 matches spec:114-115 once its line wrap is joined with one space, and it keeps the two-space layout of `src/cli.rs:278`. Contract 5 matches spec:82-85.
- Contract 2's examples are right. `99999999999h` parses to 5,999,999,999,940 minutes, which fits `i64` (`src/time.rs:92-96`). `9999999999999999999h` fails `digits()` and returns `InvalidFormat` (`src/time.rs:99-104`). `167h60m` gives 10080, because the minute part of `HhMMm` has no 59 cap (`src/time.rs:73,92-96`).
- Contract 8 matches the code. `is_duration_token` rejects only `InvalidFormat` (`src/cli.rs:446-451`), so `200h` still triggers the reminder.
- The storage claim is right. The untyped path passes `now.time()` (`src/commands.rs:384`) and storage maps `Ambiguous(earliest, _)` to `earliest` (`src/storage.rs:217-222`). `anomalies::local_midnight` already uses `.earliest()` (`src/anomalies.rs:380-384`). No other `now`-derived local rebuild can panic.
- The flush claim is right. `std::process::exit` runs `rt::cleanup`, which flushes stdout and drops the error (`src/main.rs:8`).
- Complexity ≤15 is reachable. The new branches are one cap comparison and a three-arm exit mapping.
- The coverage ratchet can go green on the Linux dev machine. The broken-pipe and `/dev/full` paths run in tests on Linux, and `process::exit(141)` runs atexit handlers, so the profile is still written.

## Findings

### F1. Medium. Plan drops the spec's §1.2a item and says the spec never asked for it

- Category: factual error (about the spec), internal inconsistency
- Claim: plan:112 says "'Interaction' names no SPEC section for it" and leaves stored oversized targets out of Task 2. Contract 9 (plan:61) and Task 2's criteria (plan:94-102) do not mention §1.2a.
- Evidence: spec:120 says "§1.2a gains stored oversized targets as a possible issue." SPEC.md:61-69 is that section, and it does not yet carry the item.
- Consequence: Task 2 can pass every criterion and still ship a SPEC that leaves out a locked spec requirement. A user with an old oversized target gets a `status` panic that no live doc mentions.
- Fix: add a Task 2 criterion for SPEC §1.2a (stored oversized targets can still panic `status` and overflow totals until the week's target is set again). Add §1.2a to contract 9. Delete the risk at plan:112.

### F2. Low. The risk at plan:111 misreads the spec

- Category: factual error (about the spec)
- Claim: plan:111 says "The spec does not say whether `week 2026-41 target 200h` gets the reminder or the cap error."
- Evidence: spec:71-72 says "The `week WEEK_ID target` reminder (SPEC §3.7, exit `2`) still comes first."
- Consequence: none in the code, because contract 8 lands where the spec does. The risk text is wrong, and a task planner may treat it as open.
- Fix: replace the risk with a citation of spec:71-72.

### F3. Low. "MSRV stays 1.85" is not true of the crate today, and CI does not test it

- Category: factual error about the codebase
- Claim: plan:28 says "MSRV stays `1.85`. Test helpers must build on it". Plan:21 says "no MSRV change".
- Evidence: the crate already uses let chains, which are stable only from Rust 1.88. They appear at `src/db.rs:183-184`, `src/storage.rs:343-344` and `src/commands.rs:89-90`. So the crate does not build on 1.85 now. CI installs only `stable` (`.github/workflows/ci.yml`, "Install Rust" step), and no `rust-toolchain` file exists. "Builds on 1.85" cannot be checked as written.
- Mitigation: clippy's `incompatible_msrv` lint reads `rust-version` and fails on `std::io::pipe` under `-D warnings`. I tested this in a scratch crate with `rust-version = "1.85"` on rustc 1.99. The output was "current MSRV ... is `1.85.0` but this item is stable since `1.87.0`". So the plan's real gate already exists, but the lint checks only std APIs and not language features.
- Fix: write the constraint as "clean under clippy's `incompatible_msrv` (part of the existing clippy gate)". Record that the 1.85 declaration is already stale, as a coordinator follow-up outside this changeset.

### F4. Medium. The `mlm export notes` broken-pipe test exits `0` unless a note is seeded

- Category: testing gap
- Claim: plan:78 says a closed-read-end stdout gives exit `141` and empty stderr for `mlm week`, `mlm export notes` and `mlm status`.
- Evidence: `render_lines` of an empty list is `""` (`src/export.rs:36-41`, asserted at `src/export.rs:184`). `print!("")` or `write_all(b"")` makes no syscall, so no `EPIPE` occurs, and a flush of an empty buffer makes no syscall either. On a fresh database the default range has no notes, so the command exits `0`. SPEC.md §6.2 already says an empty range "prints nothing and exits `0`".
- Fix: the criterion should say the test first seeds a note in the default range (today's week or month, per §3.8). `mlm week` and `mlm status` always print, so they need no seeding.

### F5. Low. "Assert exact stdout" cannot hold for the clock-dependent cases

- Category: internal inconsistency, testing gap
- Claim: plan:69 says "CLI cases assert exact stdout, stderr and exit code."
- Evidence: plan:76 runs `status` with an open stint against the real clock. Live minutes and the `est. EOD` time depend on `now` (`src/status.rs:363-371`, rendered at `src/status.rs:219`). The repeated-hour cases (plan:71) run under a `TZ` rule computed from the current time, so their `status` output depends on the clock too. The binary has no injectable clock.
- Fix: list which cases assert exact stdout. Let the clock-dependent cases assert exit code, empty stderr, and the presence of an `est. EOD` line.

### F6. Low. The plan does not pin how the write error is recognised, and a DB failure could be misreported

- Category: error-handling gap
- Claim: plan:19 says the write error "has to survive as a typed I/O error, so `src/main.rs` can tell a broken pipe from other failures."
- Evidence: `DbError::CreateDataDir` carries a `std::io::Error` and exposes it through `source()` (`src/db.rs:93,143,186`). If `main` walks the anyhow chain (`chain()`, `root_cause()`) looking for `io::Error`, a data-dir failure would print `error: cannot write output: Not a directory (os error 20)`. The existing guard does not catch this, because `tests/e6_db_failure.rs:54-57` asserts only that stderr is non-empty.
- Fix: pin a dedicated wrapper (for example a write-error type around `io::Error`). Or pin a top-level downcast with no chain walk. Add a criterion that the e6 path still prints its own `could not create ...` text.

### F7. Low. SPEC §6.3's sentence "A runtime error ... exits `1`" goes false, and Task 2 does not name it

- Category: internal inconsistency (docs task)
- Claim: contract 9 (plan:61) and Task 2 (plan:98) file the broken pipe in §6.3 "as a runtime error" with exit `141`. They require only that §6.1's heading and the §6 intro stay true.
- Evidence: SPEC.md:756-757 says "A runtime error (a bad value, a storage failure) exits `1`." Exit `141` is a runtime error that does not exit `1`.
- Fix: add to Task 2 that §6.3's runtime-error sentence names the `141` exception.

### F8. Low. The repeated-hour tests cover only the second pass, and the `TZ` rule has date edge cases beyond midnight

- Category: testing gap, missing edge case
- Claim: plan:70-71 tests a second-pass instant only. Plan:115 names only the midnight boundary for the computed `TZ` rule.
- Evidence: the audit says the panic fires in both passes (`unwrap-audit-review.md` F1, "both passes of 02:00-02:59"). No criterion checks the first pass. A northern-style POSIX rule also needs its DST start before its end in the same year. Near 1 January that leaves little room. The `Jn` form also skips 29 February.
- Fix: run the unit test at a first-pass instant as well (it is cheap, and the zone is fixed). Tell the task plan to build the rule with `Mm.w.d` or zero-based `n` dates, and to handle the start-of-year case or skip it as it skips midnight.

## Undeclared user-visible gaps

Every hit here is a candidate for spec §1.2 (which currently reads "None") or SPEC §1.2a. Choosing among them is the coordinator's and user's call.

### U1. An untyped `start` in the second pass shows an open stint one hour too long

- Choice: storage is unchanged, so a second-pass untyped punch stores the first-pass instant (plan:11, spec:53-57).
- Evidence: the punch lands one hour before the real now (`src/commands.rs:384`, `src/storage.rs:221`). `status` measures the live stint against the real `now_utc`, and `est. EOD` adds the gap to the stored start (`src/status.rs:367`).
- What the user sees: right after `mlm start` at 02:10 in the second pass, `status` shows the stint at about 1h running, the day total one hour high, and `est. EOD` one hour early. Neither the plan nor the spec writes this down. Spec §1.2 says "None".

### U2. A stint that crosses the two passes still goes backwards when both punches are untyped

- Choice: same as U1.
- Evidence: the spec review describes the case (`panic-fixes-spec-review.md:21` and item 17). `mlm start` at 02:30 in the first pass is stored at 00:30 UTC. `mlm stop` at 02:10 in the second pass is stored at 00:10 UTC. NOTES 140 says the storage choice "avoids the backwards-stint and orphan cases F17-F19". That holds only for the typed-versus-untyped mismatch. With two untyped punches across the passes it still happens.
- What the user sees: an orphaned end and an open start in `status`, and a stop warning (`src/commands.rs:43-52`). Not recorded as a consequence.

### U3. The swapped-target reminder suggests a command that the cap then rejects

- Choice: the reminder comes first and checks shape only (contract 8, spec:71-72).
- Evidence: the reminder prints `error: put WEEK_ID after target: mlm week target 2026-41 200h` (`src/cli.rs:428-431`). Running that command gives contract 2's cap error.
- What the user sees: two errors in a row for one mistake, and the first one tells them to run a command that fails. Plan:75 tests that the reminder fires, but nothing records that its advice now fails for over-cap values.

### U4. `delete punch|note N` with failed output loses the recreate line, and a retry deletes a different entry

- Choice: committed writes stay, and only the output is lost (contract 6, spec:90-92).
- Evidence: the recreate line is the only record of the deleted row (`src/commands.rs:305-308,337-340`). Ids are positions in a fresh listing (`src/commands.rs:247-252`), so after one delete, `N` names the next entry.
- What the user sees: under `/dev/full`, `error: cannot write output: ...` with exit `1` and no sign that the delete happened. Under a closed pipe there is no message at all, only exit `141`. A user who reads either one as "it failed" and runs it again deletes a second entry. In both cases the recreate line is gone. Contract 6 records "keeps the change" but not this.

### U5. `--help` and `--version` into a closed pipe or a full disk exit `0` silently

- Choice: clap's help and version handling stays unchanged (contract 8, spec §1.1).
- Evidence: `src/cli.rs:363-366` drops the `err.print()` result and returns clap's exit code `0`.
- What the user sees: `mlm --help > /dev/full` exits `0` with nothing, and `mlm --help | <closed>` exits `0`. Every other command exits `1` or `141` (spec §5 says "Any command whose stdout closes early"). The plan does not record that help and version are the exception.

### U6. Empty-output commands exit `0` into a closed pipe, so the exit code depends on the data

- Choice: a broken pipe is detected only when a write fails (plan:52).
- Evidence: `mlm export notes` over an empty range writes nothing (`src/export.rs:36-41`). See F4.
- What the user sees: under `set -o pipefail`, `mlm export notes --month X | <reader that closed>` exits `141` in a month with notes and `0` in one without. Not recorded.

### U7. A closed stdout descriptor counts as success, not as "any other write failure"

- Choice: write failures go through std's `Stdout` (plan:19).
- Evidence: std's raw stdout turns `EBADF` into a successful write (`handle_ebadf` in std's `io/stdio.rs`). So `mlm week >&-` "succeeds". Spec:80 says "Any other stdout write failure: one line on stderr, exit `1`."
- What the user sees: `mlm week >&-` exits `0` with no message, and `mlm delete punch 1 >&-` deletes and exits `0` with no recreate line. The plan pins `/dev/full` only and does not record the `EBADF` exception.

### U8. Exit `141` is a normal exit, not a death by signal

- Choice: `141` is returned as an exit code (contract 4, spec:76-79).
- Evidence: spec:77-79 justifies it as "what a shell reports for a process killed by `SIGPIPE`". Rust ignores `SIGPIPE` (spec review, "What checks out"), so `mlm` returns 141 through `exit`.
- What the user sees: shells report `$?` = 141 either way. Callers that inspect how the process ended do not. Python's `subprocess` shows `returncode == 141` where a real `SIGPIPE` death shows `-13`, and `WIFSIGNALED` is false. Plan:116 records only the Windows side.

## Verdict

ship-with-followups

F1 and F4 need a plan edit before the worktrees open. F1 leaves out a locked spec requirement. Under F4 a test written exactly as the plan states would fail. Each fix is one line. The other findings are wording or task-plan guidance. U1-U8 go to the coordinator for the known-issues decision.
