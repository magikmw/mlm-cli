# Review: orphan-stop-warning plan (rescoped)

Plan: `docs/dev/plans/orphan-stop-warning-plan.md` at `44f8537`.
Spec: `docs/dev/specs/2026-10-24-orphan-stop-warning.md` (locked).

## What checks out

- `punch()` already returns `(target_date, punch_id)` (`src/commands.rs:315-347`). `stop` discards it today (`src/commands.rs:39-42`).
- `start`'s step is generic over its writer (`src/commands.rs:49-60`, `72-87`). It swallows write errors with `let _ =` (`src/commands.rs:85`).
- `storage::punches_in_range` (`src/storage.rs:398`) plus `PunchBuckets::trio` (`src/history.rs:41-47`) give the same trio `status` uses (`src/status.rs:472-483`). So `anomalies::date_facts` (`src/anomalies.rs:436-444`) yields the same `orphaned_end_times` as `status`.
- Orphans come out ascending. `close_group` pushes them in sorted group order (`src/stint.rs:144`, `211-219`), and the splice removes only index 0 (`src/stint.rs:309-311`).
- The D+1 splice touches only `day.open` and `day.completed` (`src/stint.rs:315-327`). The plan's claim that D's orphans do not depend on D+1 is correct (plan:95).
- `e8_lines` is private (`src/anomalies.rs:272-286`). Contract 1's indentation (4 and 5 spaces, two before the parenthetical) matches it and SPEC §7.3 (`docs/dev/SPEC.md:1229-1233`). The weekday form matches `format_date_with_weekday` as `start` uses it (`src/date.rs:175`, `src/lookback.rs:111-127`).
- A malformed `at_utc` row can be seeded (`src/db.rs:44-48` has no CHECK on `at_utc`). It fails `punch_from_row` (`src/storage.rs:363-374`). The write path reads nothing (`src/storage.rs:334-357`), so the write still commits. A D-only read would leave the new end orphaned and warning, so a silent result really proves the error was suppressed.
- Breaking tests: no CLI test asserts `stop`'s stderr (`tests/start_hint_cli.rs:117,130,246`, `tests/overlap_cli.rs:150,160`, `tests/week_target_cli.rs:133-141`). `week_target_cli` sets `TZ=UTC` (`tests/week_target_cli.rs:15`), so plan:96's "before 09:00 UTC" is right. The `commands.rs` unit tests that call `stop()` will print to the real stderr, which is noise but breaks nothing.
- E27 is the next free id (`docs/dev/SPEC.md:1583` is E26).
- Every spec §3-§7 item and every Interaction bullet maps to a Task 1 or Task 2 criterion. The two tasks own disjoint files.

## Findings

### F1 — low — internal inconsistency / ownership: AGENTS.md goes stale with no owner
- Claim: "It adds no module, so the AGENTS.md layout list is unchanged" (plan:19). AGENTS.md is out of scope for Task 2 (plan:88). The coordinator keeps only NOTES.md and the archive banner (plan:90).
- Evidence: AGENTS.md:40-42 says of `src/anomalies.rs` that "`status`, `week` rows and the carry scan all call it". After Task 1, `stop` calls it too. The module doc says the same (`src/anomalies.rs:3-5`), and Task 1 owns that file.
- Fix: give Task 2 (or the coordinator) the AGENTS.md line. Have Task 1 update the module doc.

### F2 — low — docs task not fully parallel from the pinned text
- Claim: "the contract below is all they share" (plan:32). Task 2's spec scope is "§7, Interaction, §1.2" (plan:73).
- Evidence: Task 2's §7.4 criterion needs "the trigger ... and the best-effort rule" (plan:83). Those rules live in spec §3 (splice exemption, displaced and pre-existing orphans, `--date`) and spec §5. The contracts pin only the text, the order and silence. They do not pin the trigger rules.
- Fix: add spec §3-§5 to Task 2's Spec line.

### F3 — low — factual error: the superseded task plans no longer exist
- Claim: the two task plans "are not inputs. The next pass overwrites them" (plan:5).
- Evidence: `44f8537` deletes both files (`git show --stat HEAD`). `docs/dev/plans/` holds no task plan.
- Fix: say they were removed.

### F4 — low — scope creep: a criterion pins a non-feature
- Claim: "`stop` never prints `start`'s lookback lines" (plan:65).
- Evidence: NOTES entry 118 dropped F5, "tests pinning non-features" (`docs/dev/NOTES.md:1694`). The exact-stderr assertion on every case (plan:55) already rules those lines out.
- Fix: drop the bullet.

### F5 — low — testing gap: three cases are underspecified
- Two orphans (plan:61): the case doesn't require the orphans to be written in non-ascending order. If they are written in order, the test can't catch a missing sort.
- Pre-existing orphan (plan:60): the times aren't pinned. A seeded orphan between the new start and stop (for example 12:00 with `start 09:00`, `stop 17:00`) pairs with the start. The block then names 17:00, so the case no longer tests the old orphan. Pin the orphan before the start.
- ASCII (plan:28): the constraint cites `tests/ascii_sweep.rs`, which only covers `status` and `week` (`tests/ascii_sweep.rs:50-113`). Exact-stderr asserts cover the new text in practice, but the citation overstates what the sweep checks.

### F6 — low — error-handling gap: a stderr write failure is not pinned
- Claim: contract 3 covers silence and read failure only (plan:45).
- Evidence: `start` ignores write errors (`src/commands.rs:85`). The plan says the writer is injected "the same way" (plan:53) but never says a failed write must not change the exit code.
- Fix: one clause in contract 3. A failed stderr write is ignored and the exit code stays `0`.

### U1 — medium — undeclared user-visible gap: a forgotten stop from yesterday stays silent, while one from two days back warns
- Decision: an end that splices with yesterday's open start draws no warning (spec §3, spec:54-55; plan:58). The long-splice warning was cut (NOTES 117).
- Consequence: say yesterday's 09:00 start is still open and today's first punch is `mlm stop 17:00`. The splice credits a 32h stint, and `stop` prints nothing. The same mistake with a start two days back prints a warning (FE5). One day of difference flips the output between a warning and silent credit, and the silent case is the one that adds hours. Neither the spec's known issues (spec:35-39) nor the plan records this.
- Candidate known issue.

### U2 — low — undeclared user-visible gap: a backdated `stop` can orphan an end on the next date without a word
- Decision: `stop` checks only D (spec:32; plan:67, "Checking D+1").
- Consequence: D has one open start, and D+1's first punch is an end spliced to it. `mlm stop 23:00 --date D` closes D's start. D+1's end becomes an orphan, and `stop` prints nothing. The non-goal states the scope, but the spec never says that `stop` can create an orphan it does not report.

### U3 — low — undeclared user-visible gap: the refused-join context is missing from `stop`'s warning
- Decision: no refused-join lines from `stop` (plan:67; NOTES 117).
- Consequence: when D-1 has two open starts, D's first end is orphaned. `stop` prints only the E8 block, whose `fix:` says "start before T on D". `status` prints the same block plus "not joined to ... across midnight" (`src/anomalies.rs:306-313`). Following `stop`'s hint alone leaves D-1's E7 in place. Nothing records that `stop`'s block is a subset of what `status` shows for this shape.

### U4 — low — undeclared user-visible gap: the FE5 warning names neither the stale start nor its date
- Decision: no lookback lines after the warning (plan:19, plan:65).
- Consequence: in the doc example (spec:69-75), the `fix:` line suggests `mlm start ... --date 2026-10-05`. The real problem is the start open on 2026-10-03. Spec:77 points to `status`, but the plan's FE5 doc example (contract 4) does not say the warning omits the stale start. NOTES 118 already dropped this as U1, so the coordinator can drop it again. It is listed because the visible difference is still undocumented.

### U5 — low — undeclared user-visible gap: the D+1 read adds a silent-suppression case
- Decision: read D-1 through D+1 (plan:15, plan:95).
- Consequence: a malformed row on D+1 suppresses a warning that D's data alone would produce, and D+1 never changes D's orphans (`src/stint.rs:315-327`). The plan records this in Risks, but the spec (§5) and the planned SPEC §7.4 text don't. A user with a corrupt neighbour row loses the warning with no sign of why. Reading D-1 through D would give the same result for valid data and avoid this. Either record the choice in §7.4 or narrow the read.

No other user-visible consequence was found undeclared. The repeat on every `stop` (spec:51-53), the weekday header (spec:63-64), first-run stderr (plan:79, 85) and the stop-before-start warning (spec:37-39) are all declared.

## Verdict

ship-with-followups
