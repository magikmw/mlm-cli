# live-eod spec review

Target: docs/dev/specs/2026-10-03-live-eod.md (branch live-eod, baseline main c9e980b, confirmed by `git log main`).

## Findings

### F1 (medium, factual/omission) Source of `live` is unnamed, and the §3.2 guarantee holds only for `ctx.day`
- Claim: spec.md:94-98 "Today's page only holds open starts dated today."
- Evidence, claim holds for `ctx.day`: `classify` pairs one date's punches (src/stint.rs:124-175) and `resolve` feeds it `punches_for_date(target_date)` (src/status.rs:585-590, 598). A yesterday-dated open start lives in `prev` and is never merged into today's `open`. `classify_at` only touches `prev` to delete an orphaned end from `day` (stint.rs:301-303), so no prev open leaks into `day.open`. Verified: a spliced overnight stint never appears as today's open.
- Gap 1: the spec never says which field `live` reads. `has_open_stint` reads `ctx.day.is_ongoing()` (status.rs:601). `ctx.day` is the correct source. `ctx.day_raw` (status.rs:344) is wrong. `classify_at` can remove today's open start into a completed stint when tomorrow has a first-punch orphan end (stint.rs:307-319), so `day_raw.open` could be non-empty while `has_open_stint` is false, giving `live > 0` with no EOD. The spec should say `ctx.day.open.last()`.
- Gap 2: that splice-next removal is reachable only when tomorrow-dated punches exist. The CLI refuses future writes (SPEC.md:567), but a timezone change or a seeded DB can produce it. The spec does not mention that `live` then correctly becomes 0.
- Wording error: spec.md:96-97 says "A forgotten one shows up only as a lookback `[!]` line." The lookback covers ages 2-14 only (status.rs:449-472). A yesterday-dated open start shows nothing on today's page (SPEC.md:75-80, §1.2a). An age-15+ start shows nothing either (SPEC.md:62-65).

### F2 (medium, internal inconsistency / factual) "Rounding may shift est. EOD by one minute" is wrong, and so is the equality claim for future starts
- Claim: spec.md:30-33 "`est. EOD` equals `open start + (required − fulfillment)`... Whole-minute rounding of `live` may shift it by one minute between reads."
- Evidence, rounding: punches are minute-aligned (storage.rs:100-101). `live = floor((now − start)/60)` (stint.rs:172). `eod = now + (G − live)` is `start + G + s` with seconds `s < 60`. `%H:%M` truncates (status.rs:211). The displayed value is exactly `start + G` on every read, with no drift. T2 (spec.md:125-126) asserts "same est. EOD", so the spec contradicts itself unless a test fixture uses non-zero seconds.
- Evidence, future starts: when the open start is later than `now`, `live` is clamped to 0 (stint.rs:168-173) and `est. EOD = now + gap`, not `start + gap`. A future-dated TIME is allowed (no chronology check, SPEC.md:282; future rejection is for `--date` only). The spec's equality holds only for `start ≤ now`.

### F3 (medium, undeclared user-visible gap) Overnight session gets no live credit after midnight
- Decision: spec.md:94-98 limits `live` to starts dated today.
- Visible consequence: a session started yesterday at 22:00 and still running at 01:00 has no open stint on today's page (stint.rs:124 grouping by stored date). `has_open_stint` is false, `est. EOD` is omitted (status.rs:627-637), and the pace hint ignores the 3h of live time. This is the same "frozen" behaviour the changeset fixes, but for overnight sessions.
- A session that crosses midnight also changes behaviour at 00:00. `est. EOD` (possibly `(tomorrow)`) disappears. `required` steps up a weekday while `live` drops to 0, so `gap` jumps by roughly `live + daily target`.
- The spec records neither effect. §1.2 lists only the day-total split. Candidate for known issues.

### F4 (medium, undeclared user-visible gap) Pace hint now diverges from the week line, most visibly Fri-Sun
- Decision: subtract `live` from `gap` (spec.md:65-67) while §1.1 keeps the week line and `owed` unchanged (spec.md:39-41).
- Evidence: on Fri/Sat/Sun `required == 5 × daily target` ≈ the week target (SPEC.md:226-242 and §2.4 pin). Before, the pace hint's `gap` roughly equalled the week line's `owed`. Now, with an open stint, the hint can read `00h 10m over 40h 00m required today` next to `Week: 00h 20m left by end of Friday`.
- The README sample at README.md:230-231 already shows the mismatch. Week line `04h 35m ahead of target` sits next to a hint that will read `09h 54m over` (see F6).
- §1.2 declares only the day-total vs hint split. It does not declare hint vs week line, nor that the week line stays frozen while the hint counts down.

### F5 (medium, docs scope) §4 docs list misses the places that go stale
- README.md:230 sample: `04h 35m over 40h 00m required today, target already met` with open stint `17:45-now (05h 19m, ongoing)` (README.md:235). Subtracting live 05h19m gives `09h 54m over 40h 00m required today`. The spec says only "the sample output matches" (spec.md:115-116). No arithmetic is given, and the new value widens the visible gap with the week line (F4).
- README.md:13-14 feature bullet ("live 'how much is left today' estimate") and README.md:74-82 known-issues bullet ("pace hint built from the week's running fulfillment, not from the day total") are not in the §4 list. The README.md:267-272 paragraph is covered only vaguely.
- SPEC.md §1.2a pace-hint vs day-total bullet (SPEC.md:74-78 region, "day total is today in isolation") is not in the §4 list. It becomes incomplete once the hint also includes live time.
- NOTES decisions 19 (NOTES.md:174-178, "`now + (daily target − day total)`") and 37 (NOTES.md:256-260, "estimated-EOD figure" only) are not updated. Decision 37 says live feeds only the stint line and EOD. The spec adds the pace hint, so decision 37 turns false.
- spec.md:19-21 says "§7.1's formula never did" feed live. SPEC.md:660-662 says explicitly "Fulfillment here still excludes the open stint's live minutes (§2.4)". That is a documented decision to reverse, not an omission. NOTES should record the reversal.

### F6 (low, testing gap) Tests do not cover the spec's own central guarantees
- No test that a yesterday-dated open start is absent from `live` and `has_open_stint` on today's page (§3.2's headline claim).
- No test for the clamp case (future-dated open start, stint.rs:168-173).
- No test for splice-next consuming today's open start.
- No Fri/Sat/Sun `required today` test with live.
- T6 does not say how to assert "later one". `open` is ascending by start (stint.rs:163-165 comment), so the choice is `.last()`. Pin it with starts of different ages.
- The "update existing §7.1 sample assertions" item (spec.md:136) targets hand-built `StatusView` fixtures (status.rs:1102-1115, 1391-1445). Those never run `resolve`, so updating them proves nothing about the new formula. T1-T4 must be resolve-level, as spec.md:122 says, and the spec should say the hand-built ones are cosmetic only.
- T5 "byte-identical to baseline" (spec.md:132) names no baseline artefact. Existing resolve-level tests such as status.rs:1994-2008 and 2097-2113 read `gap_minutes` and should be named explicitly.
- T2 needs a `now` with non-zero seconds to check the F2 claim at all.

### F7 (low, design gap) `DailyTargetHint.gap_minutes` semantics change is not stated
- Evidence: `gap_minutes` is both the hint number (status.rs:188-191, 620-625) and the EOD input (status.rs:628-629). The spec does not say whether the field becomes post-live or a second field is added.
- Existing tests read it as a public field: status.rs:2005 (`<= 0`), status.rs:2105 (`> 360`), and the hand-built fixtures. A rename or redefinition silently changes what they assert. The struct doc comment (status.rs:66-71) also needs updating.

### F8 (low, undeclared user-visible gap) Multiple open starts: hint counts only the latest, stint list shows each
- Decision: spec.md:100-104, `live` = most recent open start's `minutes_so_far`.
- Visible consequence: the stint list shows every open stint against `now` with its own duration (status.rs:475-500, test status.rs:1315-1345 shows 9h/8h/7h). The pace hint credits only the smallest. A reader summing the list will not reproduce it.
- Fixing the anomaly (stop or delete the extra start) changes `live` discontinuously, so `est. EOD` jumps. The `[!]` line does not mention the pace hint.
- The spec says only that `est. EOD` "errs late". The same applies to the pace hint's "left to" / "over".

### F9 (low, undeclared user-visible gap) Crossing the target while working now flips wording mid-stint
- Before: `est. EOD` and the hint stayed frozen until `stop`. After: at the minute `gap` reaches 0, `est. EOD HH:MM` is replaced by `target already met` (status.rs:628-635, T3/T4) and the hint switches to `over`, with `X` then growing every minute. The spec describes the thresholds (spec.md:71-76) but not that `status` output now changes continuously, minute by minute, and diverges between two runs with no punches in between.
- Anything diffing or scripting `status` output, or the stretch idea "shell prompt integration" (AGENTS.md), sees a time-varying figure. Not recorded in §1.2.

### F10 (low, undeclared user-visible gap) `stop` no longer makes the hint jump, but a backdated `stop` or `start` does
- Before: `stop` moved the stint into fulfillment, so the hint dropped by the whole stint at once. After: the drop is continuous (live floor matches stint minutes), so the post-`stop` value equals the pre-`stop` value. The visible discontinuity moves to `stop HH:MM` with an earlier time (hint rises by the removed minutes) and to `start HH:MM` with an earlier time (hint drops by the extra live minutes). The spec records neither.

### F11 (low, error-handling) Future-dated open start today is unhandled in the spec
- `start 23:00` at 18:00 is legal. `live` clamps to 0, `has_open_stint` is true, and `est. EOD = now + gap` is printed, though nothing is running. Not mentioned in §3.2 or the test plan. See F2.

### F12 (low, internal inconsistency) Non-goals vs docs section on wording
- spec.md:44-45 "No change to the pace hint's or est. EOD's wording." But spec.md:115 asks README to say the hint "counts the running stint" and F9 above shows `target already met` now appears earlier. This is consistent for strings and not for behaviour, so it should be phrased "no string changes". Minor.

## Checks that came out clean
- Worked example arithmetic (spec.md:80-90): 165 − 15 = 150 = 02h 30m, EOD 18:00 + 150 min = 20:30. At 18:30, live 45 gives 120 = 02h 00m and EOD 20:30. Correct.
- Thresholds (spec.md:71-76) match code: `gap >= 0` is `left to` (status.rs:188), `gap > 0` gives EOD and else `TargetAlreadyMet` (status.rs:628-635), `(tomorrow)` from date compare (status.rs:631).
- Past-date `status` shows neither figure (status.rs:611-613, 639). No change needed.
- `week_view` and `commands` do not consume `DailyTargetHint` or `EodState` (grep: only status.rs). No cross-module fallout.
- No scope creep found.

## Verdict
ship-with-followups

Fix F1 (name `ctx.day.open.last()`), F2 (drop or correct the rounding and equality claims) and F5 (README arithmetic, NOTES 19/37, §1.2a) before implementation. Record F3, F4, F8, F9 and F10 in §1.2 if accepted.
