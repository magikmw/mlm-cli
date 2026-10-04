# Final review: possible-issues-triage

- Baseline: `be85617` (`main`)
- Changeset: `possible-issues-triage` at `d50fa2d` (clone tip verified)
- Clone: `/tmp/cd-scratch/possible-issues-triage-final`
- Spec: `docs/dev/specs/2026-10-03-possible-issues-triage.md` §3, §4, §6; SPEC.md §1.2a, §3.5, §3.7, §4.3, §6, §7.1-§7.4
- Ship verdict: **ship-with-followups**

## Verification set (run in the clone)

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo build` | ok |
| `cargo test` | all pass (722 unit + 82 integration across 10 binaries, 0 failed) |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo clippy -- -W clippy::cognitive_complexity` (threshold 15, after `touch`) | no warnings |
| Smoke runs | every run used a fresh `MLM_DB_PATH=/tmp/smokeN.db` |

## Hand runs against the worked examples

All of these matched the spec text byte for byte, except where a finding says otherwise.

- §3.1 yesterday hint, today empty. The `[i]` line, `fix:` and `or:` all print.
- §3.1 after `mlm start` today. The stderr warning has two lines and no `or:`, and the page drops `or:`.
- §3.1 round trip. Following `or: mlm stop` joins (`01:16 continues previous day's stint`, `18:02-01:16 (07h 14m, spans to next day)`), and the hint clears.
- §3.2.2 window. Yesterday has `09:00` open plus `14:00-15:00`, so the hint reads `(a time between 09:00 and 14:00)`. Following it at `14:00` (the inclusive edge) pairs cleanly: `09:00-14:00` and `14:00-15:00`, with no anomaly.
- §3.2.1 overlap. The nested `09-12`/`10-11` example prints the spec line and the `delete punch` fix. Touching `13-14`/`14-15` is not flagged, and zero-length `16:00-16:00` is not flagged. The `mlm week` row gets `[!]` and the footer prints.
- §3.3 carry marker. An E8 in week 37 and an E7 in week 39 give today's `status` the plural line naming `2026-37`. `status` for a 2026-09-23 date shows the singular form. `week 2026-38` and the current week mark `Carry-in: ... [!]`. The earliest week shows no marker. A future week (`2026-41`) shows the marker.
- §3.5 echo. `week target 20h` prints `Week 2026-40 target: 20h 00m`, and `90m` prints `01h 30m`.
- §4.1. A fresh DB with no `-v` prints nothing on stderr. `-v` shows the DEBUG migration lines.
- §4.2. `status -h` shows `(see more with '--help')`, and `--help` ends with the glossary.
- §4.3. `w 2026-41 target 20h` prints the spec message with exit 2. Missing and extra values now name `<DURATION>` and `[WEEK_ID] <DURATION>` instead of `<ARGS>`.
- §4.5. `status 2026-10-05` (tomorrow) prints `error: invalid DATE "2026-10-05": date is in the future` with exit 1.

## Mutation tests (all reverted, clone clean at `d50fa2d`)

Every mutation was caught by at least one test.

| Mutation | Tests failed |
|---|---|
| Overlap `<` changed to `<=` (touching counts as overlap) | 5 |
| `has_any` ignores overlaps | 5 |
| `next_punch_after` counts a same-instant punch | 23 |
| Refused-join suppression of the hint removed | 2 |
| `or:` condition inverted / always printed | 7 / 4 |
| Live credit `<=` changed to `<` | 17 |
| Past-date open stint joins the overlap check | 3 |
| Zero-length filter removed | 1 |
| Carry scan predicate narrowed to E8 only | 11 |
| Carry scan includes the shown week | 8 |
| Carry scan passes `today = date` (stale lost) | 2 |
| Overlap pair sort without `Reverse(end)` | 1 |
| Spliced span not clipped at midnight | 1 |
| `status` future check removed | 1 |
| Swapped-order exit code 1 | 1 |
| Default log level `Info` | 3 |
| Each of the three `load_history` reads made infallible | 2 / 2 / 1 |
| Stale and lookback hints ignore `N` | 3 / 5 |

The carry-scan agreement with `mlm week` rows holds by construction. Both paths call `anomalies::date_facts`/`build_facts` on `PunchBuckets::trio`, and `tests/carry_marker_cli.rs::marker_weeks_each_have_a_marked_row` checks the agreement end to end. The history loader matches the old `status::build_ledger` and `DbWeekData` semantics. Both read the earliest date, classify each punch date with its neighbours, and credit live minutes only when the walk reaches the current week. Only the earliest data week mattered to `WeekLedger`, so the loader drops the per-week note reads without changing any figure.

## Findings

### F1 (medium): following the yesterday hint's `or:` line can create an overlap

- Location: `src/lookback.rs:182` (`yesterday_hint_lines`); SPEC §7.3 and changeset spec §3.1 (the `or:` rule)
- Repro: yesterday has `start 09:00`, `start 14:00`, `stop 15:00`, and today has no punches yet. The page prints `or: mlm stop (still working: ends it now, across midnight)`. Running `mlm stop` joins `09:00` across midnight, giving `09:00-01:16`, which nests over `14:00-15:00`. Yesterday then shows `[!] overlapping stints 09:00-01:16 and 14:00-15:00` and its week row gets `[!]`.
- Why it matters: the clean-pair rule (§3.2.2) protects the `fix:` time but not the `or:` line. In this shape the `09:00` start is almost certainly a forgotten stop, not "still working". The overlap anomaly catches the result, so nothing goes silent, but the hint led the user into the anomaly.
- Fix: print the `or:` line only when `facts.next.is_none()` and today has no punches. Update §3.1/§7.3 to say so, and add a test where yesterday has a later stint and the hint has no `or:`.

### F2 (low, security/UX): the argument-order message echoes any token into a copyable command

- Location: `src/cli.rs:344` (`swapped_week_target`), `src/cli.rs:364` (`is_week_token`)
- Repro: `mlm week 'foo$(id)' target 20h` prints `error: put WEEK_ID after target: mlm week target foo$(id) 20h` with exit 2.
- Why it matters: the line is a command the user copies, and the week token is echoed unvalidated. That token can carry shell metacharacters or terminal escape sequences. The input is the user's own, so the risk is low. The bigger cost is the wrong diagnosis: `mlm week foo target 20h` is told to swap order when the real error is an invalid week id.
- Fix: intercept only when the week token parses with `date::parse_week_id` (or the bare-number form) and the duration token parses with `time::parse_duration`. Otherwise fall through to clap's error. Add a test with a non-week token.

### F3 (low): `mlm week target --help` has three blank lines before `Options:`

- Location: `src/cli.rs:243` (`help_template`)
- Repro: `mlm week target --help | cat -A` shows three empty lines between the hand-written Arguments section and `Options:`. The hidden positional leaves `{all-args}` with an empty leading section, and the template adds `\n\n` on top of that.
- Fix: tighten the template (for example `{before-help}\n{all-args}`, or put the Arguments text in `after_help`) and pin the full `--help` output in `tests/week_target_cli.rs::week_target_help_text`, which today checks only substrings.

### F4 (low, test strength): one history test cannot fail

- Location: `src/history.rs:478` (`history_accounting_is_the_week_module_walk_over_the_ledger`)
- Why it matters: `History::accounting` is defined as exactly the call the test compares it against, so the assertion is a tautology. Equivalence with the old ledger is covered elsewhere (status/week golden tests, `two_weeks_of_data_give_worked_per_week`, padding and live tests), but this test adds nothing.
- Fix: assert concrete `carry_in`/`worked`/`fulfillment` figures for a multi-week fixture with an override, or delete the test.

### F5 (low, docs/process): working docs are not archived, and README has a missing period

- Location: `docs/dev/specs/2026-10-03-possible-issues-triage.md`, `docs/dev/plans/possible-issues-triage-*.md`; `README.md:54`
- What: SPEC.md and NOTES.md already hold the settled behavior. The changeset's spec and plans carry no `Archived` banner yet, while earlier specs (for example `2026-09-19-boundary-stint-pairing.md`) do (`docs/dev/README.md` convention). README's single Possible issues bullet ends without a period ("points to `stop`").
- Fix: banner the working spec, plan and task plans in the fix or close pass, and add the period.

## Areas reviewed with no finding

- Security of `fix:`/`or:` lines. Every interpolated value is a program-formatted date, time, count or week id, and no note text or other user string reaches them.
- `week_targets_in_range` binds its parameters, and its zero-padded `YYYY-WW` text order is chronological. A corrupt key in range is now a hard error, which matches §3.4's intent.
- Code quality. `anomalies`, `lookback`, `history` and `carry_scan` are small and pure where they claim to be. Facts are built only in `anomalies::build_facts`, and every function is under the complexity threshold of 15.
- SPEC.md §1.2a keeps only the I14 counting half. §7.1-§7.4 carry the hint, overlap, marker, echo and read-error text, and those match the binary. No live doc links into `plans/` or `specs/` for behavior.

## Verdict

**ship-with-followups.** The §3 fixes and §4 polish work as specified, and the tests behind them are strong (every mutation was caught). F1 is the only behavior problem, and the overlap anomaly makes it visible rather than silent. F1 should go into the scheduled fix pass along with the pre-commit hook fix.
