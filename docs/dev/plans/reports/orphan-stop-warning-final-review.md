# orphan-stop-warning — final review

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

- **Baseline**: `v0.4.0` (`4c451d7`)
- **Branch**: `orphan-stop-warning`, tip `f687365` (clone HEAD verified)
- **Clone**: `/tmp/cd-scratch/orphan-stop-warning-final`
- **Spec**: `docs/dev/specs/2026-10-24-orphan-stop-warning.md`, NOTES.md entries 111, 117-121, "Triaging review findings"
- **Verdict**: ship-with-followups

## 1. Verification set

All run in the clone.

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo build` | ok |
| `cargo test` | all suites pass (761 lib, 10 in `orphan_stop_cli`, rest unchanged) |
| `cargo clippy --all-targets -- -D warnings` | clean |

## 2. Hand runs of the built binary

`TZ=UTC`, fresh scratch `MLM_DB_PATH` per case, today Sat 2026-10-24.

| Case | Commands | Result |
|---|---|---|
| FE5 | `start 22:00 --date -2`, bare `stop` | one block for `Sat 2026-10-24 at 15:12`; `status` shows the same E8 lines plus the 2-days-ago unclosed start |
| Spec example verbatim | `start 22:00 --date 2026-10-03`, `stop 14:16 --date 2026-10-05` | byte-for-byte the spec's block |
| Clean pair | `start 09:00`, `stop 17:00` | silent |
| Splice | `start 22:00 --date -1`, `stop 01:00` | silent |
| Displaced end | `start 09:00`, `stop 17:00`, `stop 12:00` | block for 17:00; `status` shows 09:00-12:00 and the 17:00 orphan |
| Pre-existing orphan | `stop 08:00`, `start 09:00`, `stop 17:00` | both `stop`s print the 08:00 block |
| Two orphans | `stop 16:00`, `stop 10:00` | 10:00 then 16:00 blocks, no blank line |
| Backdated | `stop 18:00 --date -5 'some note; rm -rf /'` | block names Mon 2026-10-19; note text never echoed |
| Read failure on D-1 | garbage `at_utc` row on 2026-10-23, `stop 17:00` | nothing printed, exit 0, end row stored; `status` then errors on the corrupt row, as expected |
| Malformed row on D+1 | garbage on 2026-10-20, `stop 17:00 --date 2026-10-19` | nothing printed (accepted in entry 119 U5) |
| Malformed row outside D±1 | garbage on 2026-10-10 | warning prints normally |
| Hard errors | `stop 17:00 "   "`, `stop 25:00`, `stop --date -1` | error line, exit 1, no warning |
| Non-UTC splice | `TZ=Europe/Warsaw`, `start 23:00 --date -1`, `stop 00:30` | silent; `stop 00:45 --date -3` warns |

Stdout was empty and exit 0 in every success case.

## 3. Correctness

- `stop` classifies `D` through `anomalies::date_facts(&buckets.trio(D), ...)`, the same call `status` makes (`src/status.rs:472`). Parity with the status page holds by construction, so the E8 set cannot drift.
- One ranged read over `[D-1, D+1]` makes a neighbor read failure impossible to mistake for an empty date. Any error suppresses the whole warning (spec §5).
- The warning reads only after `punch` returns `Ok`, so a hard error never reaches it.
- The `D+1` read is not needed for orphaned ends, since `D`'s ends pair only with `D` or `D-1` starts. A mutation that drops it survives every test. Entry 119 U5 already accepted the cost (a malformed `D+1` row suppresses the warning), and the shared trio keeps the code identical to `status`, so this is not a finding.

## 4. Security

- Every warning line is built from a `NaiveDate` and a `NaiveTime`. No user string (note text, raw `--date` text) reaches the output. A shell-metachar note and a `$(id)` note were both stored and never echoed.
- The hint commands are fixed templates with `HH:MM` placeholders, so nothing in them can be replayed with injected content.
- The new code adds no SQL. It reuses `storage::punches_in_range`.

## 5. Test strength (mutation results)

Each mutation was applied with `sed`/`perl`, the suite run, then `git checkout -- src`.

| Mutation | Killed by |
|---|---|
| Read `D` only (no neighbors) | `stop_is_silent_when_the_end_splices_with_yesterday` |
| Splice off (empty `prev` bucket) | same unit test |
| Read error treated as a fixed orphan | `stop_warning_read_failure_prints_nothing` |
| Classify today instead of the written date | 3 CLI tests (`past_date_…`, `spec_example_…`, `stop_hint_lines_…`) |
| Reverse block order | `orphan_warning_lines_one_block_per_time_in_order` |
| `writeln!(..).unwrap()` instead of ignoring | `stop_warning_ignores_a_failing_writer` |
| Warning call removed | 7 CLI tests |
| Warning sent to stdout | 7 CLI tests |
| Hint wording drift in `anomalies.rs` | `t_e8_gains_fix_lines_text_unchanged`, `stop_warns_for_an_orphan_two_days_after_an_open_start` |
| Drop the `D+1` read | survives (equivalent, see §3) |

The CLI tests assert exact stderr, empty stdout and exit 0. `block()` restates the hint text, so a drift in the shared helper fails the file. `stop_hint_lines_match_status_page` checks parity against a real `status` page. `stop_warning_ignores_a_failing_writer` asserts nothing after the failing write, but it does fail on a panic, which is the property it guards.

## 6. Findings

### F1 — SPEC §4.3 lead-in says `stop`'s warning is the only insert-time signal (low)

- **Location**: `docs/dev/SPEC.md:461-463`
- **Problem**: "The one insert-time signal is `stop`'s stderr warning…" is false. `start`'s lookback warning (§7.4) also reports §4.3 anomalies (stale open starts, E7) at insert time. The colon that introduces the edge-case list also hangs off "The punch is stored either way" now, so the list reads as a list of ways the punch is stored.
- **Fix**: "`start` and `stop` print stderr warnings at insert time (§7.4), and the punch is stored either way. The edge cases are:"

### F2 — README paragraph left with one unwrapped line (cosmetic)

- **Location**: `README.md:124`
- **Problem**: "`week target` prints one line confirming the stored target. `status` and `week` are the commands that produce" runs past 120 columns. The rest of the file wraps near 72.
- **Fix**: re-wrap the paragraph.

### F3 — No binary-level test runs a bare `mlm stop` (low)

- **Location**: `tests/orphan_stop_cli.rs:84-90` (`fe5_stop_today_after_a_start_two_days_back`)
- **Problem**: FE5's literal trigger is a bare `mlm stop`, and the spec and README examples show it. Every CLI test passes an explicit TIME. The no-TIME branch (`now.time()`) shares the write path, and the hand run passes, but no test pins that a bare `stop` warns.
- **Fix**: add a CLI case that runs `stop` with no TIME after `start 22:00 --date -2` and asserts exit 0, empty stdout, three stderr lines, a header prefix `warning: orphaned end on <today with weekday> at `, and hint lines that match `status`'s page (reuse `stop_hint_lines_match_status_page`'s approach).

## 7. Notes, not findings

- SPEC §7.4's lead says the write commands are silent and "exit `0` is the only signal" right after "apart from the stderr warnings". That tension predates this changeset (`start`'s warning) and the "apart from" scopes it.
- The changeset spec, plan and task plans carry no `Archived` banner yet. Earlier changesets add the banners at close-out (`da7e92a`), so this belongs to the close-out step.
- `README.md`, `AGENTS.md` (`anomalies.rs` and `history.rs` bullets) and SPEC §1.2a, §3, §3.3, §3.7, §6, §6.2, §7.4, F13, E8 and E27 match the shipped behavior, apart from F1.
- Code quality is good. The hint lines live in one helper (`anomalies::orphan_hint_lines`) used by both `status` and `stop`. The new functions are small and below the complexity threshold.

## 8. Verdict

**ship-with-followups**. The behavior matches the spec in every worked example, the tests that matter kill every non-equivalent mutation, and nothing user-supplied reaches the output. F1-F3 are a doc sentence, a line wrap and one missing test.
