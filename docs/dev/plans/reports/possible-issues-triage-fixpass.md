# Fix pass: possible-issues-triage

- Branch: `pit-fixpass`, created from `possible-issues-triage` at `5211698`. The dispatched worktree was not based on that tip (it sat on an unrelated branch), so I made the task branch from the tip rather than rebasing.
- Gates: `cargo fmt`, `cargo build`, `cargo test` (726 unit plus integration, 0 failed), `cargo clippy --all-targets -- -D warnings` all clean.
- Smoke runs used `MLM_DB_PATH=/tmp/fp-s1.db`. They cover the swapped-order message, the future `status` error (exit 1), and `week target --help`. The `week 'foo$(id)'` token cannot go through the shell wrapper here, so an integration test covers it.

## Per finding

### F1 yesterday hint `or:` line
- Change: `yesterday_hint_lines` in `src/lookback.rs` prints `or: mlm stop` only when `facts.next.is_none()` and today has no punches. SPEC §7.3, the §3.1-equivalent bullet, the E22 entry and the README text now say so.
- Failed first: `yesterday_hint_lines_drop_or_line_when_a_later_punch_follows_the_start` and `page_drops_or_line_when_yesterday_has_a_later_closed_stint` (the review fixture, 09:00 open plus 14:00-15:00). Both got the three-line hint. The plain case (`page_shows_three_line_hint_when_today_is_empty`) still passes.

### F2 argument-order interception
- Change: `swapped_week_target` now takes `today` and fires only when the week token parses with `date::parse_week_id` and the duration token is not `DurationParseError::InvalidFormat`. A negative duration such as `-5h` still counts as a duration because it is well-shaped. `classify` passes `chrono::Local::now().date_naive()`, which only matters for a bare week 53.
- Failed first: `swapped_needs_a_valid_week_id_and_duration` failed to compile against the old one-argument signature (red was a compile error). The behavioral tests then passed once the new signature existed, so I did not see a behavioral red. The old code accepted any non-flag token, so the new cases would have returned the custom message. Integration cases `foo$(id)` and `2026-99` were added to `other_week_errors_keep_clap_text`, and unit test `invalid_week_token_gets_clap_error_not_the_custom_message` was added.

### F3 `week target --help` blank lines
- Change: template `{before-help}\n\n{all-args}` became `{before-help}{all-args}` (`{all-args}` already emits the separating blank line). `week_target_help_text` now asserts the full output.
- Failed first: `week_target_help_text` with the pinned full text got three blank lines before `Options:`.

### F4 tautological history test
- Change: replaced with `accounting_gives_concrete_carry_worked_and_fulfillment_across_weeks`. It uses three weeks with overrides on 06 and 08 and asserts target, carry-in, worked, fulfillment, owed and carry-out for each.
- Failed first: none. It replaces a test that could not fail and the code under test is correct, so it passes on first run. It asserts concrete figures.

### Hook
- Change: `.githooks/pre-commit` captures `cargo llvm-cov` stderr to a temp file. A nonzero exit sets `STATUS=1` and prints the status, a clear message and the last 30 lines of stderr. A missing `cargo-llvm-cov` or `jq` still prints SKIP. AGENTS.md's pre-commit section now states this.
- Verified by hand: I added a test with `assert_eq!(1, 2)` and ran `bash .githooks/pre-commit`. It printed `FAIL: 'cargo llvm-cov' exited with status 101` plus the stderr tail and exited 1. I then removed the test.

### FE3 glossary
- Change: `GLOSSARY` in `src/cli.rs` gains `Total behind, Total ahead`, `overlapping stints` and `orphaned end` entries, placed before `[!], [i]` so the glossary still ends with "It never marks a row."
- Failed first: `long_help_carries_glossary` with three new terms (the `\n  overlapping stints\n` form matches the entry heading only, since the `[!]` entry already mentions both phrases in passing). The tests have no full golden for long help, only the term list, so that list was updated.

### FE9 future `status DATE` error
- Change: `status::resolve` maps a `Cause::Future` error to `... date is in the future; to view a future week, run mlm week WEEK_ID`. Only `status` gets the pointer. `start`/`stop`/`note --date` keep the old message. Exit stays 1. SPEC §3.5 and §6.1 and README quote it.
- Failed first: `status::tests::resolve_future_date_is_rejected` (updated expected text). I also updated `tests/status_cli.rs::status_future_date_is_rejected`.

### FE10b README internal codes
- Change: README line 289 (in the status section, not under Possible issues) said `E7/E8/stale-start`. It now says `[!]` line for its unclosed start or orphaned end. The Possible issues section itself had no codes. No other internal codes remain in README (grepped E/I/T/P/F numbers, `§`, "decision N").
- Test: none, docs only.

### FE4 old unclosed stint (spec §4.6)
- Change: `stint_line` in `src/status.rs` prints `09:00-?` when the end is open and the age is `TwoOrMoreDaysBack`. `(unclosed)` and the 11-wide padding stay. SPEC stint bullet updated. README had no example of the old form.
- Failed first: `open_stint_two_or_more_days_back_drops_duration_and_shows_unclosed` (now expects `  09:00-?      (unclosed)` and no `-now`).

## Notes, not fixed

- The long-help glossary is pinned only by a term list, not a full golden. A full golden could be added.
- SPEC §7.3 line about overlapping stints says "An open stint prints as `09:00-now`". That is a general statement and remains true for today and yesterday, but it does not mention the `-?` form.
- `swapped_week_target` now reads the system clock through `classify`. Only a bare `53` depends on it.
