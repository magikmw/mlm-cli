# live-eod Task 2 report (docs)

Worktree started at f76ad3b (behind). Rebased onto `live-eod` tip f4b139a (fast-forward) before editing.

## Arithmetic

Thursday sample (SPEC 7.1), required 32h 00m = 1920 min.
- Completed 07h 25m = 445 min. Open stint 17:45-now.
- Fulfillment = 29h 15m = 1755 min. required - fulfillment = 1920 - 1755 = 165 min = 02h 45m.
- At 18:00: live = 15. gap = 165 - 15 = 150 min = 02h 30m. Line reads `02h 30m left to`. Matches the pinned line.
- est. EOD = 17:45 + 165 min = 17:45 + 02h 45m = 20:30. Matches.
- At 18:30: live = 45. gap = 165 - 45 = 120 min = 02h 00m left. est. EOD still 17:45 + 02h 45m = 20:30.

README Saturday sample, required 40h 00m = 2400 min.
- Fulfillment 44h 35m = 2675 min. Old gap = 2400 - 2675 = -275 min = 04h 35m over (matches old text).
- Open stint 05h 19m = 319 min (live). New gap = -275 - 319 = -594 min.
- 594 min = 9 h 54 m (9 x 60 = 540, 594 - 540 = 54). Reads `09h 54m over`. gap <= 0 so `target already met` holds.
- Week line unchanged: 04h 35m ahead (2675 - 2400 = 275).

## Re-grep (SPEC.md, README.md)

Terms: `EOD`, `live`, `ongoing`, `pace hint`, `left to`, `now + `, `excludes`.
- `now + `: one hit, SPEC 7.1 est. EOD bullet, "equals `now + gap`" for a start at or before now with post-live gap. Correct, not the old formula.
- `excludes` / `keep going from right now` / `04h 35m over` / `20:45`: no hits.
- `live` hits: day-total "not folded in live" (kept per spec), 1.3 "computed live" (open stint duration, unrelated), and the new live-minutes text.
- `ongoing` hits: stint-line and `(+ ongoing)` flag text, unchanged by design.
- Remaining `pace hint` / `est. EOD` passages (2.4, 3.5, 5 owed-vs-required paragraph, 7.1 week-line bullet, F10, README intro) describe no formula and need no change.
- No reference to docs/dev/specs/, docs/dev/plans/ or NOTES.md added to either file.

## Edits

SPEC: 1.2a (nine new bullets covering every spec 1.2 item), 2.4 (open stint feeds own line, pace hint, est. EOD; both display-only; required-by-day compared against fulfillment less live), 3.5, 7.1 (sample, pace-hint bullet with `live` term and most-recent-start rule, est. EOD bullet rewritten), F9 stability, F9b over-by-live.
README: feature bullet, two known-issue bullets plus extension of the day-total bullet, Saturday sample, pace-hint paragraph.

## Deviations

- Spec 1.2 wording "used to mean" / "never flips" history phrasing was recast in present tense (AGENTS.md: no narration of old behavior).
- Also edited SPEC 2.4's required-by-day sentence, 3.5 bullet, and the 7.1 "fulfillment under required" parenthetical, because they described the old formula (review fold F1).
- No code changes; cargo gates run only to confirm unaffected.
