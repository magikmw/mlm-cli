# live-eod fix pass

- **Scope**: final-review F1, F2 and fresh-eyes F1, F2, F6, F7, F8, F9, F11, plus the "Known issues" rename.
- **Base**: worktree started on an older commit (`f76ad3b`); rebased onto `live-eod` at `d435603`.
- **Commits**: `e2955f2` (code), one docs commit after it.

## Per finding

| Finding | What changed | Test that failed first | Not done |
|---|---|---|---|
| Review F1 | SPEC §7.1 pace-hint bullet, `est. EOD` bullet and §8.1 F9 reflowed to the ~72-column wrap | none (formatting; no test can catch it) | |
| Review F2 | SPEC §2.4 `required` bullet now says "the same figure the week-level `owed` uses" before "less the running stint's live minutes" | none (wording) | |
| Fresh-eyes F1 | `EodState::At(NaiveTime, EodDay)` replaces the bool. `EodDay` is `Today`, `Tomorrow`, `Weekday(date)`, `Dated(date)`, picked by `EodDay::classify`. Suffix uses `date::format_date_with_weekday` (weekday abbreviation is its first token). SPEC §7.1 `est. EOD` bullet now describes the table. SPEC §1.2a day-suffix bullet reworded. README has no `(tomorrow)` text, so its `status` section gained the new rule. | `eod_day_suffix_one_day_is_tomorrow_and_two_days_is_weekday` (got `00:00 (tomorrow)`, wanted `00:00 (Sat)`), `eod_day_suffix_six_days_is_weekday_and_seven_days_adds_date` (got `23:59 (tomorrow)`, wanted `23:59 (Wed)`) | |
| Fresh-eyes F7 | README possible issues: two-open-starts count note, and `fix:`/`or:` narrowness note. SPEC §1.2a: new `fix:`/`or:` bullet (the count note was already there). `fix:` text unchanged. | doc only | |
| Fresh-eyes F11 | README `status` section says `est. EOD` can land days out with the suffix, and points to Possible issues for why figures on one screen disagree while a stint runs | doc only | |
| Rename | README heading and both "see ..." cross-references (lines 284, 372); SPEC §1.2a heading and its body sentence ("a possible issue works but has a rough edge"). NOTES.md, specs and plans untouched. | doc only | |
| Fresh-eyes F2, F6, F8, F9 | One possible-issue entry each in SPEC §1.2a and README | doc only | |
| Fresh-eyes F3, F4, F5, F10 | Left alone as instructed | | |

Tests added (all in `src/status.rs`): same-day (no suffix), 1 day (`(tomorrow)`), 2 days (`(Sat)`, the 1 to 2 boundary), 6 days (`(Wed)`), 7 days (`(Thu 2026-02-19)`, the 6 to 7 boundary). Each runs through `resolve` and `render` from punches with a set week target. Existing tests moved to the new `EodDay` values. The tomorrow resolve test now uses `matches!` instead of a match with a panic arm, which also removed an uncovered line.

## Verification

- `cargo fmt`, `cargo build`, `cargo test` (514 lib tests plus integration, 0 failed), `cargo clippy --all-targets -- -D warnings`: all clean.
- Pre-commit hook on the code commit: format OK, complexity OK, coverage 99.1465% to 99.1506%, baseline ratcheted. The first attempt failed the coverage ratchet (99.1465% to 99.1394%) because of an extra uncovered panic arm; fixed as above, no `--no-verify`.
- Hand run, built binary, `MLM_DB_PATH` under the scratchpad, `start 9:00` on Saturday 2026-10-03 with the default 40h target:

```
Day total:     00h 00m (+ ongoing), 31h 24m left to 40h 00m required today, est. EOD 01:00 (Mon)
```

## Notes for the coordinator (not fixed)

- Fresh-eyes F2 says "Saturday", but `required` plateaus at the full target from Friday on. The new entries say "From Friday on".
- The F9 possible issue says the migration line goes to stderr (checked by running the binary). The fresh-eyes report did not say which stream.
- `date::weekday_abbrev` is private. The suffix takes the first token of `format_date_with_weekday` instead of widening `date.rs`. Making it `pub` would be tidier if wanted.
- SPEC §3.2's wording on future start times and the new F6 possible issue overlap with the existing "start typed for later today" bullet. Both kept, as asked.
