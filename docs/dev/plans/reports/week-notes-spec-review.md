# week-notes spec review

- Spec under review: `docs/dev/specs/2026-10-24-week-notes.md` (untracked draft)
- Source baseline: branch `week-notes` = `main` = `3bcff42`
- Clap probes: replica of `WeekArgs` + `WeekTargetArgs` with a `notes: bool` (`-n`/`--notes`) flag, clap 4.6.6, built in `/tmp/cd-scratch/week-notes-spec`

## Findings

### F1. `mlm week target ... --notes` is not a clap usage error (High, factual error)

- Claim: spec:39-40 says "`mlm week target ... --notes` is a usage error from clap. `WeekArgs` already sets `args_conflicts_with_subcommands`." spec:108 turns that into a test.
- Evidence: `args_conflicts_with_subcommands` (src/cli.rs:227) only governs parent args that come *before* the subcommand. After `target`, tokens belong to `WeekTargetArgs.args`, which is `num_args = 1..=2` with `allow_hyphen_values = true` (src/cli.rs:268-275). That positional swallows `--notes`/`-n` as a value. `week_target::apply` then parses the first token as WEEK_ID (src/week_target.rs:61-68), so the result is a runtime error, exit `1`.
- Replica results:

| argv | parse result | real outcome |
|---|---|---|
| `week target 40h --notes` | `args = ["40h","--notes"]` | `invalid WEEK_ID "40h"`, exit 1 |
| `week target --notes 40h` | `args = ["--notes","40h"]` | invalid WEEK_ID, exit 1 |
| `week target 40h -n` | `args = ["40h","-n"]` | invalid WEEK_ID, exit 1 |
| `week target -n` | `args = ["-n"]` | DURATION parse error, exit 1 |
| `week target 43 40h --notes` | clap "unexpected value '--notes'" | usage error, exit 2 |

- Only the three-token form matches the claim. Nothing is ever written, because no form yields a valid WEEK_ID plus DURATION.
- Fix: restate §3 to describe the actual outcome (runtime WEEK_ID/DURATION error, exit 1, nothing written) and rewrite the spec:108 test to match. Rejecting it as a usage error would need new parsing logic, which is a scope decision for the user.

### F2. The §5 example is labelled "past week" but is the current week (Medium, internal inconsistency)

- Claim: spec:61 says "Illustrative past week ... `mlm week 43 --notes`".
- Evidence: the example's headline at spec:66 is `05h 16m over 40h 00m target`, the current-week Friday-to-Sunday form. SPEC.md §7.2 (SPEC:1030-1041) says a closed week always gets `Total behind`/`Total ahead`. The span 2026-10-19..25 also contains today (2026-10-24), and Sun 2026-10-25 is in the future.
- The arithmetic is consistent (rows sum to 48h 48m; 48h 48m - 3h 32m = 45h 16m).
- Impact: spec:121 folds this example into SPEC §7.2, so the wrong label would land in the live spec.
- Fix: call it the current week (a Saturday with no open stint), or switch the headline to `Total ahead: 05h 16m` and pick a week that has closed.

### F3. The notes-read failure test passes before the feature exists (Medium, testing gap)

- Claim: spec:94-97 adds a new ranged note read whose failure is a hard error. spec:110 tests it by dropping the `notes` table.
- Evidence: plain `mlm week` already fails when `notes` is dropped. `history::load_history` calls `storage::earliest_data_date` (src/history.rs:89), which unions `punches` and `notes` (src/storage.rs:503-509). An existing test proves it: `build_week_view_errors_when_notes_is_unreadable` (src/week_view.rs:1072-1077). A `--notes` run against a dropped table fails at the old read and never reaches the new one.
- spec:98 ("Without `--notes`, `mlm week` makes no new read") has no test at all.
- Fix: break only the new read. For example, `ALTER TABLE notes RENAME COLUMN body TO b` leaves `earliest_data_date` working, because it reads only `date`. Then assert that plain `mlm week` succeeds and `mlm week --notes` exits `1` with empty stdout. That covers both spec:94-97 and spec:98.

### F4. The project-spec section misses SPEC passages that enumerate `week` reads (Low, internal inconsistency)

- Claim: the "Interaction with the project spec" section (spec:118-122) lists only §3.6, §7.2 and §8.1.
- Evidence: two other passages list exactly which reads `week` makes, and both go stale:
  - SPEC §6.1 (SPEC:638-643) names "the earliest data week, the target overrides, the punches behind the carry walk".
  - E20 (SPEC:1601-1605) names the `week_targets` table, the `notes` table and the punch range.
- Related stale descriptions:
  - src/history.rs:1-7 and AGENTS.md ("Layout") describe the loader as exactly three reads.
  - AGENTS.md describes `storage.rs` as having "one ranged week-target-override read". No ranged note read exists today; storage has only `notes_for_date` (src/storage.rs:465-468).
- The spec also does not say whether the new read lives in `history::load_history` or in `week_view`, or that it needs a new storage function.
- Fix: add §6.1 and E20 to the interaction list, and add a NOTES/AGENTS touch if the new storage function lands.

### F5. Testing list misses layout combinations the spec asserts (Low, testing gap)

- spec:103 covers `[!]`, `[i]` and both, but no case covers today's row with `(incl. ongoing)` (src/week_view.rs:113-115) followed by notes.
- spec:58-59 says footers, the carry-in marker line and the Carry-in block are unchanged. No test runs `--notes` with the `[i]` footer, the Monday overnight line (`overnight_last_week`) or a `carry_marker` present.
- spec:41-42 fixes the `--help` text, but no test checks the help line.
- Fix: add one golden case with ongoing plus notes, one with carry marker plus `[i]` footer plus notes, and a help-text assertion.

### U1. `-n` before `target` loses the swapped-argument hint (Low, undeclared user-visible gap)

- Decision: `-n` is a parent flag on `WeekArgs`, so it sits in the token stream before a possible subcommand.
- Evidence: `swapped_week_target` (src/cli.rs:366-380) matches exactly four tokens, `week WEEK_ID target DURATION`, and filters out only `-v`/`--verbose`. In the replica, `mlm week 43 -n target 40h` gives clap's generic `error: the subcommand 'target' cannot be used with: ...` (exit 2) instead of `error: put WEEK_ID after target: ...`.
- Not recorded in spec §1.2 (spec:25-27 says "None").

### U2. `mlm week -n target` and `mlm week -n target 40h` produce odd errors (Low, undeclared user-visible gap)

- Decision: under `args_conflicts_with_subcommands` (src/cli.rs:227), once `-n` is seen, `target` is no longer recognized as a subcommand.
- Replica results:
  - `mlm week -n target` parses as `week_id = "target"`, which becomes a runtime `invalid WEEK_ID "target"` error with exit 1. Plain `mlm week target` is instead a missing-DURATION usage error with exit 2.
  - `mlm week -n target 40h` gives `the subcommand '40h' cannot be used with: ...` (exit 2), which names `40h` as a subcommand.
- Not recorded.

### U3. Note layout differs from `status` (Low, undeclared user-visible gap)

- Decision: spec:46-47 says a date's notes are "exactly `status DATE`'s Notes list". spec:55-56 then prints them with six spaces and `- `, and no header.
- Evidence: `status` prints a `Notes:` header and then `  - body` with two spaces (src/status.rs:309-312). The content matches but the presentation differs, and users of both commands will see it.
- Not recorded as a deliberate difference.

### U4. Long or unusual bodies break the indented column (Low, undeclared user-visible gap)

- Decision: no wrapping or truncation (spec:19-20, declared).
- The visible consequence is not declared. A body longer than the terminal width wraps to column 0, under the weekday column, so a long note can be mistaken for row text. Embedded U+2028/U+2029 survive storage (src/storage.rs:956-957) and render as a line break in some terminals, giving an unindented line.
- `status` has the same behavior, but there the note sits under a `Notes:` header. Here it sits inside the row table.

### U5. Notes push the footers and totals away from the rows (Low, undeclared user-visible gap)

- Decision: notes print inline under each row (spec:55-57), and the `[!]`/`[i]` footer slot stays after the seventh row (src/week_view.rs:85-92).
- Visible consequence: in a week with many notes, the `[!] needs a look` and `[i] open stint on <row>` footers sit several screen lines below the marked row. The output also stops having its fixed 16/18-line shape (src/week_view.rs:597-640), and any user script that relies on that shape should keep using plain `mlm week`.
- Not recorded.

### U6. `n` means "note" elsewhere but is a WEEK_ID inside `week` (Low, undeclared user-visible gap)

- Decision: the short flag is `-n`.
- Evidence: `n` is the visible alias of top-level `note` (src/cli.rs:81) and of `delete note` (src/cli.rs:177). NOTES entry 125 plans `n` for `export notes` too. Inside `week`, a bare `n` lands in the `WEEK_ID` positional, so `mlm w n` gives `invalid WEEK_ID "n"`, exit 1.
- Not recorded.

## Verdict

ship-with-followups

F1 and F2 should be corrected in the spec before planning, because F1 makes the spec:108 test impossible to write as stated, and F2 would put a mislabelled example into SPEC §7.2. F3 should change the spec's test list. U1-U6 are candidates for spec §1.2. Which of them to accept is the user's call.
