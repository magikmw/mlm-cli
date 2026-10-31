# export-notes plan review

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

Reviewed: `docs/dev/plans/export-notes-plan.md` (untracked, branch `export-notes`, HEAD `d29091b`) against the locked spec `docs/dev/specs/2026-10-24-export-notes.md`.

Source checked: `src/cli.rs`, `src/date.rs`, `src/lib.rs`, `src/main.rs`, `src/storage.rs` (`notes_in_range`), `src/week_view.rs`, `tests/` (all files, with `week_target_cli.rs`, `week_notes_cli.rs`, `ascii_sweep.rs`, `orphan_stop_cli.rs` read closely), `.githooks/pre-commit`, `docs/dev/SPEC.md` §1.2, §3, §6, §7, §8, `README.md`, `AGENTS.md`, `docs/dev/NOTES.md` entries 125-133 and "Triaging review findings", and the spec review `docs/dev/plans/reports/export-notes-spec-review.md`.

Experiment: `/tmp/cd-scratch/epr`, clap `=4.6.6` (the locked version, `Cargo.toml:32`). It reproduces the planned shape: `Export(ExportArgs)` with a required `ExportWhat::Notes(NotesArgs)` subcommand, `NotesArgs` with three `Option<String>` flags in a `#[group(multiple = false)]`, each with `allow_hyphen_values = true` and the pinned `value_name`s, plus the global `-v` and a top-level `note` aliased `n`.

## Claims verified true

These hold, so they are not findings:

- `notes_in_range` orders `"date" ASC, created_at_utc ASC, id ASC` and collects into a `Vec` before returning (`src/storage.rs:483-506`), so a row error yields no partial output (plan:21, contract 5).
- `resolve_date` is the resolver without the future check (`src/date.rs:126`); `resolve_future_checked_date` adds it (`src/date.rs:161-167`) (plan:19).
- `parse_week_id` turns `-1` into a shape error, because `split_once('-')` gives an empty year part (`src/date.rs:373-379`), so contract 7's `-w -1` message is right.
- DATE's `YYYY-MM-DD` accepts any four digits, `0000` included (`src/date.rs:98-118`), while `WeekId::new` rejects years outside `1000..=9999` (`src/date.rs:237`) (plan:111). The comment at `src/date.rs:142-147` claims the opposite for DATE and is wrong, but the plan is right.
- The only exhaustive `match` on `Command` is `dispatch` (`src/main.rs:51-71`), which Task 1 owns. `src/cli.rs` tests use `other =>` arms (`src/cli.rs:430-431`, `538-546`, `685-686`). `examples/` names no `Command` or `ArgKind`.
- The only help goldens are `status -h` and `week -h` (`tests/week_target_cli.rs:203-245`). `mlm --help` is only checked for exit `0` and non-empty output (`tests/week_target_cli.rs:390-398`) (plan:88).
- `ex` does not collide with `e`, and the inner `n` does not collide with top-level `n` (experiment: `mlm ex n -w 43` parses as export).
- Clap gives exit `2` for two flags (`ArgumentConflict`), a repeated flag, a trailing flag with no value, and bare `mlm export` (experiment). Nothing here passes through `rename_args_context` (`src/cli.rs:353-371`) or `swapped_week_target` (`src/cli.rs:374-392`) in a way that changes the result.
- A doc comment on an `Args` struct does not leak into help, so the `WeekArgs` re-wrap (`src/cli.rs:226`) leaves `week --help` byte-identical (experiment, `STRUCTDOC` probe).

## Findings

### F1 — medium — internal inconsistency: contract 8 says a flag with no value exits `2`, but hyphen values make that true only at the end of argv

- Claim, plan:15: clap owns the usage errors, "a flag with no value" among them, with exit `2`. Plan:60 (contract 8) says the same, and plan:80 asks for a test of "each contract 8 case".
- Evidence (experiment): all three flags take hyphen values (plan:15, contract 6, spec:55-57), so a flag with no value swallows the next token whenever one follows.
  - `mlm ex n -w` gives `a value is required for '--week <WEEK_ID>'`, exit `2`. This matches the contract.
  - `mlm ex n -m -d` parses as `month = "-d"`, so the user gets `invalid MONTH "-d": expected YYYY-MM or MM` with exit `1`.
  - `mlm ex n -d -w 43` parses as `date = "-w"`, then fails on `43` with `unexpected argument '43' found`, exit `2`. The message points at the wrong token.
  - `mlm ex n -w -v` parses as `week = "-v"` and `verbose = false`.
- The plan states the no-value case as a clean exit-`2` rule, so Task 1's test author pins only the trailing case and Task 2 documents a rule that does not hold.
- Fix: narrow contract 8 to "a range flag as the last token with no value". Then either state that a following flag is taken as the value (and reaches the §6 messages), or list it as a known issue. `start`/`note -d` already behave this way (`src/cli.rs:124`, `144`), so the narrowed wording matches existing behavior.

### F2 — medium — testing gap: nothing forces a test that tells the calendar year apart from the ISO year for bare `M`

- Claim, plan:74: "A bare `WW` uses the current ISO year. A bare `M` and `MM` use the current calendar year." Plan:67 lets clock-dependent cases either compute dates from the real clock or use unit tests with a fixed today.
- Evidence: the two years differ on only a few days around New Year. A CLI test on the real clock passes whether the resolver uses `today.year()` or `today.iso_week().year()`, so a parser copied from `parse_week_id` (`src/date.rs:380`, `None => (today.iso_week().year(), s)`) would pass. That copy is exactly what "mirroring the WEEK_ID shape" (plan:23) invites. WEEK_ID already has the fixed-today test (`src/date.rs:701-707`, `bare_week_defaults_to_todays_iso_year_not_calendar_year`). MONTH would have no counterpart.
- The spec review raised this case (F7, the 2025-12-29 example). NOTES entry 133 says F7's tests were folded, but neither spec §7 nor the plan carries it.
- Fix: require a `date.rs` unit test with a fixed today where the ISO and calendar years differ, for example `today = 2027-01-01` (ISO 2026-W53), where `parse_month("1", today)` must give 2027-01.

### F3 — low — missing edge case: `DateWeekError`'s `Display` has a catch-all that prints week wording for any new `OutOfRange`

- Claim, plan:23: the error type gains a MONTH kind, so the new messages print through the shared `invalid NAME "input": ...` form.
- Evidence: `src/date.rs:76` is `(_, Cause::OutOfRange) => write!(f, "week number must be 1 or greater")`. If Task 1 reuses `Cause::OutOfRange` for month `0`/`13`, the compiler stays quiet and the user sees `invalid MONTH "13": week number must be 1 or greater`. `(Month, Shape)` will fail to compile, but `(Month, OutOfRange)` will not. The `Cause::OutOfRange` doc (`src/date.rs:31-33`) also names only the week and year cases.
- Contract 7's exact-message unit tests would catch the wrong text, so the risk is small. The plan names neither the trap nor whether MONTH gets its own `Cause`.
- Fix: one line in Task 1, "add an explicit `(Month, OutOfRange)` arm ahead of the catch-all, or a new `Cause`".

### F4 — low — ambiguous contract: the MONTH year domain is deferred to Task 1's plan while Task 2 documents it in parallel

- Claim, plan:111: "Task 1's plan pins this" (whether a four-digit year below 1000 is accepted).
- Evidence: contract 7 (plan:55-59) already allows only two MONTH messages, so the only reading that keeps contract 7 is to accept the year. But Task 2 writes SPEC §3 and §6.1 in its own worktree (plan:97-98) and reads the contracts, not Task 1's plan. Contract 6 says only "year exactly four digits" (plan:54), so Task 2 cannot tell whether `-m 0999-05` is an empty success or an error. `-w 0999-05` is an error (`src/date.rs:237`).
- Fix: move the decision into contract 6: "any four digits, `0000`-`9999`, is accepted; a month with no notes prints nothing".

### F5 — low — doc targets missing from Task 2, though NOTES 133 says the spec review's F8 targets were folded

- Claim, NOTES.md entry 133: "F7/F8 tests and doc targets added." Plan:96-103 lists Task 2's targets.
- Evidence, by stale or missing text:
  - `AGENTS.md:16-17`, the "Current state" list, names `start`, `stop`, `note`, `delete`, `status`, `week` and `week target` as all the commands. It will be missing `export notes`. Spec review F8 named this line.
  - The `src/storage.rs` entry in `AGENTS.md:36-40` says `notes_in_range` is the read "that `week --notes` calls". It will be missing export.
  - SPEC §6.2 "Not errors" (`docs/dev/SPEC.md:666-676`) lists only `status`/`week` empty targets. An empty or future export range belongs there too. Spec review F8 named it.
  - SPEC §1.2 (`docs/dev/SPEC.md:36-56`) gets none of spec §1.1's non-goals (JSON/CSV, `--output`, `--from`/`--to`). Its `-N` week non-goal (`docs/dev/SPEC.md:50-53`) names only `week`'s argument, but after this change `-w -1`/`-m -1` are also rejected by design. Spec review F8 named §1.2.
- The spec's "Interaction" list (spec:145-153) omits these too, so the plan is faithful to the spec. The gap is between the fold record and the spec.
- Fix: add the four targets to Task 2's acceptance criteria, or record in NOTES that they were dropped.

### F6 — low — testing ambiguity: "compute the expected dates from the local date" conflicts with the `TZ=UTC` child process

- Claim, plan:67: CLI tests run the binary with `TZ=UTC`. Clock-dependent cases "compute the expected dates in the test from the local date".
- Evidence: the test process does not run under `TZ=UTC`, so `chrono::Local::now()` in the test gives the developer's zone while the binary uses UTC. In a UTC+1/+2 zone the two disagree for one to two hours after local midnight, every day, and at week and month edges that changes the expected range. Existing CLI tests use `Utc::now()` for this reason (`tests/orphan_stop_cli.rs:4`, `32-33`; `tests/overnight_visibility_cli.rs:37-38`; `tests/carry_marker_cli.rs:324`).
- Fix: say "from `Utc::now()`, matching the child's `TZ=UTC`".

### F7 — low — file ownership: the pre-commit hook stages `coverage-baseline.json` in Task 1's worktree, and Task 1 does not own it

- Claim, plan:88: Task 1 owns six files, and "any change to another file needs a coordinator decision first".
- Evidence: when coverage rises, `.githooks/pre-commit:138` runs `git add "$BASELINE_FILE"`, so Task 1's commit carries a file outside its list. That happened in `week-notes` (`git log -- coverage-baseline.json` shows `54079d4`). `docs/dev/plans/possible-issues-triage-plan.md:77` pinned a rule for this. This plan does not.
- Only Task 1 changes coverage, so the two tasks cannot conflict. The gap is that Task 1 hits its own "needs a coordinator decision" rule on its first commit.
- Fix: list `coverage-baseline.json` as hook-managed in Task 1, or reuse the triage plan's revert-and-re-ratchet rule.

## Undeclared user-visible gaps

The plan has user-visible consequences, a whole new command among them, so this category is not empty. Each item below is a deliberate choice whose visible result neither the plan nor the spec records (spec §1.2 is `None.`). Items the spec review already raised (U1-U10, F3, F5) are left out.

### U1 — `--help`, `-h` and `-v` after a range flag become its value

- Source: hyphen values on all three flags (plan:15, contract 6).
- Experiment: `mlm ex n -w --help` parses as `week = "--help"`, so the user sees `error: invalid WEEK_ID "--help": expected YYYY-WW or WW`, exit `1`, instead of help. `mlm ex n -d -h` gives `invalid DATE "-h"`. `mlm ex n -w -v` drops the verbose flag and errors on `"-v"`.
- `start -d --help` already does this. But export has three such flags, and asking for help partway through typing is a normal move. This is the visible side of F1.

### U2 — bare `mlm export` prints the whole subcommand help to stderr, exit `2`

- Source: plan:113 leaves the shape open, and plan:60 says only "its output lists `notes`".
- Experiment: clap's default for a required subcommand prints the full help block (about line, usage, `Commands:` with `notes ... [alias: n]` and `help`, then `Options:`) to stderr with exit `2`. It prints no `error:` line. A script that checks stderr for `error:` sees none.

### U3 — the MONTH shape message names fewer forms than the parser accepts

- Source: contract 7 (plan:57), spec:107.
- `invalid MONTH "...": expected YYYY-MM or MM` omits `M` and `YYYY-M`, which are accepted (plan:74). The `--month` help in the same command (plan:51) shows the example `3`, which the message never names. `WEEK_ID`'s message has the same shape (`src/date.rs:73`), but users have seen that one before, and this one is new.

### U4 — the export help lines drop the backticks the other DATE help lines use

- Source: contract 3 (plan:49).
- `-d` help reads `or -N for N days before today (e.g. -1 = yesterday)`. `status` (`src/cli.rs:100-101`) and `start`/`stop`/`note -d` (`src/cli.rs:120-121`, `141-142`) render `` `-N` `` and `` `-1` ``, backticks included (golden at `tests/week_target_cli.rs:208`). Two commands next to each other in `mlm help` describe the same grammar in two styles.

### U5 — where `export` sits in `mlm --help` is not pinned

- Source: contract 2 (plan:46) pins only that `mlm --help` lists `export`.
- The derive order of `Command` variants (`src/cli.rs:75-104`) sets the order of `mlm --help`. Whether `export` comes after `status` or between `week` and `status` is visible on every `mlm --help`, and the plan leaves it to whoever edits the enum.

### U6 — export lines carry no time and no listing position

- Source: contract 4 (plan:52).
- Within a date the lines run in insertion order (contract 5), but no time is shown, so a note backdated later sits below an earlier-written one with no visible reason. Export also gives no route to `mlm delete note ID`, which needs the per-listing position from `mlm delete note -d DATE` (`src/cli.rs:88-91`). A user who spots a bad line in an export has to re-list that date to delete it.

## Split and ownership

- Two tasks is right. Code and tests share `src/cli.rs` and one test file, and splitting them would collide on both (plan:41). Docs have no source file in common with code. The split matches `week-notes` (NOTES entries 127, 130).
- Apart from F7, no file is claimed twice or left without an owner. Task 1 needs no existing test file. Adding the variant to `Command` and `ArgKind` breaks only matches that live in `src/main.rs` and `src/date.rs`, which Task 1 owns.
- Plan:7 lists `tests/ascii_sweep.rs` as reading, while the ASCII check goes in the new test file (plan:82). That matches the `week-notes` precedent (NOTES 127 F7), so it is not a finding.
- No scope creep beyond the declared `WeekArgs` re-wrap (spec "Housekeeping"), and the experiment shows the re-wrap does not change help output.

## Verdict

ship-with-followups

F1 and F2 should be folded before worktrees open. F1 is a contract that Task 1's tests and Task 2's docs would both pin wrong, and F2 is a test that would let a plausible wrong implementation through. F3-F7 are one-line clarifications. U1-U6 are candidates for spec §1.2, and the user decides which to keep.
