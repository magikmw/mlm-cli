# point-in-time-week fixpass

Closes the listed findings from `point-in-time-week-final-review.md` and `point-in-time-week-fresh-eyes.md`.

## Base

The worktree started at `f76ad3b`, which is not a descendant of the `point-in-time-week` tip. It was a strict ancestor with no local work, so it was fast-forwarded to `693811c` (`git merge --ff-only point-in-time-week`). No rebase was needed.

The two review reports are untracked in the main checkout. They were read from there and are not part of this commit.

## Per finding

### R1 est. EOD formula (doc)

- SPEC.md §7.1 "Estimated EOD" bullet now reads `open start + (required − fulfillment without the running stint's live minutes)`, equivalently `now + gap`.
- README `status` prose says the same in words and ties it to the `left to` figure.
- `EodState::At` doc comment in `src/status.rs` states the same formula.
- Failing test first: none. The code was already right. Only the prose was wrong.

### R2 cleanup (no behavior change)

- New `week::required_and_gap(acct, today) -> (required, gap)`. `render::current_week_headline` and `status::today_eod` (was `today_hint`) both call it, so the rule lives once.
- Removed `DailyTargetHint` and `StatusView::daily_target`. Nothing rendered them.
- Removed the `_use_datelike` shim and the unused `Datelike` import in `status.rs`.
- Tests: the two tests for the new helper were written first and failed to compile (`required_and_gap` not found). The existing suite was green before the change (550 lib tests).
- Tests that read `view.daily_target` now assert through `view.week_line` (rendered text) and `view.eod`. Render-only fixtures that set an unread `daily_target` lost that line. Every assertion on a figure was kept, rewritten to the equivalent week-line text.
- The existing suite stays green after the change: 552 lib tests (550 plus the two new ones).
- Renamed one test, `t8_f11_..._has_no_daily_target_but_deadline_week_line`, to `..._has_no_eod_but_deadline_week_line`, since the field is gone.

### R3 SPEC §4.2 negative display (doc)

- The sentence now lists a surplus `carry_out` and a negative `Carry-in:` or `fulfillment` as the signed cases. The closed-week `owed` example is gone, since it renders as a positive figure under `Total ahead:`.
- Failing test first: none. Doc only.

### F3 est. EOD assumes no break (doc)

- README `status` prose adds "It assumes you keep working without a break."
- Doc only.

### F13, F8, F9, F11 (possible issues, doc)

- Added one entry each to SPEC.md §1.2a and README's Possible issues list, placed before the first-run stderr entry.
- F13 uses the review's own numbers (`50h 33m left to 40h 00m target`, carry-in `-28h 00m`, 17h 27m worked).
- No behavior change.

## Not touched

- F2, F1, F4, F5, F6, F7, F10, F12, F14, as instructed.
- NOTES.md and the locked spec file.

## Notes (not fixed)

- NOTES.md entries (lines 379, 741, 1089, 1104, 1231) name `DailyTargetHint`, which no longer exists. They are historical decision records, and NOTES.md was out of scope.
- The README "silent on success" statement (line 227) and the new F8 possible issue now sit in the same file. They agree, since F8 records the cost of the silence.
- The SPEC §1.2a "first run logs to stderr" entry and README's Usage section still disagree in tone (fresh-eyes F10, declared).

## Gates

- `cargo fmt`: clean.
- `cargo build`: clean.
- `cargo test`: 552 lib tests plus integration tests, all pass.
- `cargo clippy --all-targets -- -D warnings`: clean.
- Commit goes through the pre-commit hook without `--no-verify`.
