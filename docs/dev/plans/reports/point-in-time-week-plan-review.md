# Plan review: point-in-time-week

Plan: `docs/dev/plans/point-in-time-week-plan.md`. Spec: `docs/dev/specs/2026-10-03-point-in-time-week.md` (locked). Reviewed against the working tree on branch `point-in-time-week`.

Spec coverage check: every spec requirement maps to a plan line. §2 live/required/figures map to plan:42-76 and contracts 1-7. §3.1-3.4 map to contracts 1-5 and 8. §4 SPEC/README sweep maps to Task 2 (plan:223-231). §5/§6 map to contract 9. §7 test list maps to acceptance bullets (plan:169-201), each one present. Both week-accounting builders are named (plan:44-47). File ownership is disjoint (plan:212-214, 272). No file the change must touch is unowned, except the two process files in F9. The findings below are the defects.

## Findings

### F1. medium, factual error about the codebase: Task 2 skips SPEC §8 and §4.2

- Claim: plan:228-229 lists the SPEC sections touched as "§1.2a, §2.4, §3.5, §3.6, §5, §7.1, and §7.2". The spec's §4 (spec:218-241) lists the same set.
- Evidence: SPEC.md carries stale pace-hint and old-headline text outside those sections.
  - `docs/dev/SPEC.md:1107-1118` (§8.1 F9): "est. EOD while the pace hint drops by the minutes elapsed". The rule in this changeset removes that hint.
  - `docs/dev/SPEC.md:1110-1124` (F9b): the whole flow tests the Day total "pace hint". It also says Sat/Sun `required` is `5 × floor(target/5)`, "a few minutes under the override itself". That is exactly what §2.2 reverses.
  - `docs/dev/SPEC.md:1126-1131` (F11 and F10): "deadline framing" and "Total still owed" wording.
  - `docs/dev/SPEC.md:1092,1097` (F7, F8): "owed math".
  - `docs/dev/SPEC.md:419` (§4.2): "ahead-of-target `owed`".
- Effect: the plan's regrep list (plan:232-234) would find these hits. But the section list and the "Files owned" statement frame Task 2 as touching only seven sections, and no acceptance bullet covers §8. F9b is a test-basis section and would contradict shipped behavior.
- Fix: add §8.1 (F7, F8, F9, F9b, F10, F11) and §4.2 to Task 2's section list and acceptance criteria. Fold the same fix back into the spec's §4.

### F2. medium, factual error: the complexity mitigation targets the wrong function

- Claim: plan:94-96 says "`status::resolve` is already busy, so Task 1 re-runs the full gate and extracts the live and required steps".
- Evidence: measured with `clippy::cognitive_complexity` at threshold 3.
  - `resolve`, `src/status.rs:618`: 7 of 15.
  - `build_ledger`, `src/status.rs:541`: 9 of 15. This is where live is added to the worked accounting (loop at `src/status.rs:567-572`), and it is the busiest function the change touches.
  - `render::week_headline`, `src/render.rs:60`: 4 of 15. It gains the gap/required branches.
  - `status::day_total_line`, `src/status.rs:211`: 7 of 15.
  - `week_view::render_week`, `src/week_view.rs:72`: 6 of 15.
  - `stint::classify`, `src/stint.rs:256`: exactly 15 of 15. Task 1 owns `src/stint.rs` (plan:212) for a comment edit. Any logic added there fails the gate.
- Effect: the plan names `resolve` as the hot spot and does not mention `build_ledger`, which has the least headroom among the functions Task 1 will edit. It also does not warn that `stint.rs:256` has zero headroom.
- Fix: name `build_ledger` as the function to keep under the limit and tell Task 1 not to add logic to `stint::classify`.

### F3. medium, factual error and testing gap: stale doc comments not covered

- Claim: plan:78-83 and 199-200 cover only the three comments spec §4 names (`WeekData`, `completed_minutes`, row `minutes`). The plan's acceptance is "Doc comments in `src/week.rs`, `src/stint.rs`, and `src/week_view.rs` state the new rule".
- Evidence: further comments in owned files state the old rule.
  - `src/stint.rs:29-31`: `OpenStint::minutes_so_far` says "SPEC.md §2.4 forbids it from entering any day/week/carry total".
  - `src/week.rs:67`: `WeekAccounting::worked` says "Completed-stint minutes for this week".
  - `src/week.rs:213`: `daily_target_minutes` is called "The `status`-only daily pace hint".
  - `src/status.rs:91-112`: the `DailyTargetHint` doc describes the Day total pace clause and `required_minutes - fulfillment - live`.
  - `src/status.rs:168`: `day_total_minutes` is "Completed-stints-only total".
  - `src/status.rs:173-176`: the `weekday_name` comment says it is used only by the pace-hint line.
  - `src/status.rs:597-600`: the `est_eod` comment.
  - `src/render.rs:2`: the module doc says "week-owed headline".
  - `src/render.rs:56-60`: the `week_headline` doc describes an `owed_minutes` parameter.
  - `src/render.rs:78`: the sample line still reads "left by end of Thursday".
  - `src/week_view.rs:49`: the `is_ongoing` doc says "`(ongoing)` marker".
  - `src/week_view.rs:56-57`: says `owed` is absent because "the headline already states it".
- Effect: the AGENTS.md "no stale docs" posture is not met, and the acceptance only checks three comments.
- Fix: widen the Task 1 sentence from "the three doc comments named in spec §4" to "every doc comment in the owned files that states the old rule", and widen the acceptance bullet the same way.

### F4. medium, testing gap: existing pins are not only strings

- Claim: plan:98-100 says existing pins "of the old strings" in `src/status.rs`, `src/week_view.rs`, `src/render.rs` are "updated to the new output, not deleted".
- Evidence: many pins are numeric or semantic, and some invert under the new rule.
  - `src/status.rs:2107,2125-2135` (`resolve_f9b_sunday_pin_non_multiple_of_five_target_override`): asserts `required_minutes == 2010` for a 2011 target and `"33h 30m required today"`. Under §2.2 the value is 2011 and the clause is gone.
  - `src/status.rs:2082-2090`: asserts the Day total pace clause.
  - `src/status.rs:2910-2975` (`live_eod_*` tests): assert that "pace hint counts down" while est. EOD stays stable. The hint no longer exists.
  - `src/status.rs:1265-1300` (`fix_c_*`): pin "required today".
  - `src/status.rs:1346-1354` (t8): pins the absence of a pace hint on another date.
  - `src/week_view.rs:808-821` (b3): "completed-stints only" row totals.
  - `src/week_view.rs:982-995` (b8): "rows sum to worked" must now include live.
- Effect: "update the string" understates the work. These tests need rewriting, and some become different tests.
- Fix: say in Task 1 that old-rule tests are rewritten to the new rule, not only re-stringed.

### F5. low, internal inconsistency: wrong acceptance wording on the `(+ ongoing)` flag

- Claim: plan:171-174 says that for `status DATE` on an earlier date with a live stint today, "the Day total for `DATE` has no live and keeps its `(+ ongoing)` flag".
- Evidence: spec §7 (spec:316-317) says only that the Day total for `DATE` does not count live. The `(+ ongoing)` flag appears only when `DATE` itself has an open stint, which in practice means yesterday (`src/status.rs:219-221`). An earlier date such as Monday has no open stint, so no flag.
- Effect: a test written literally from this bullet would fail, or would be forced onto a yesterday-only fixture.
- Fix: say "has no live and no `(incl. ongoing)` flag, and keeps `(+ ongoing)` only if `DATE` has its own open start".

### F6. low, error-handling gap: `DbWeekData` and live

- Claim: plan:44-47 says `DbWeekData` must add live to the current week. Plan:207-208 leaves storage read-error handling alone.
- Evidence: `DbWeekData` holds only `conn` and `now_utc` (`src/week_view.rs:262-265`), so it has no local date. Today's date comes from `now.date_naive()` (`src/week_view.rs:318`), and the struct must grow a field or take `today`. Its reads use `.unwrap_or_default()` (`src/week_view.rs:277-278`). A failed read of today's punches silently yields live 0, so `mlm week` shows live-less figures with no signal.
- Effect: the non-goal (spec:51-52) says the read error handling is unchanged. It was previously harmless to carry figures. Now a read failure also drops the live-inclusive current-week figures while the headline and rows come from separate reads. The two can disagree.
- Fix: record the struct change as in scope, and note that rows and `worked_minutes` use separate reads of today's punches, so a failure can desync them.

### F7. low, missing edge case: live classification is unpinned

- Claim: plan:42-44 says live comes "from today's punches whatever date `status` shows".
- Evidence: `status` uses two classifications. `ctx.day` is splice-aware via `classify_at` (`src/status.rs:371-381`) and the ledger uses `classify_at` with neighbors (`src/status.rs:567-568`). The anomaly facts use raw `classify` (`src/status.rs:378-380`). For `status DATE` the plan needs a separate classification of today, but it does not say it must use `classify_at` with yesterday's and tomorrow's punches.
- Effect: a raw `classify` of today differs from the spliced one when today's first punch is an orphaned `end` that splices yesterday's open start. `mlm week` rows (`src/week_view.rs:131-175`) use `classify_at`. If the implementer chooses raw, `status DATE` and `status` would disagree on live.
- Fix: pin live to the splice-aware classification (`classify_at`, as the rows and ledger do) and add a test on the splice boundary.

### F8. low, scope creep: acceptance additions beyond spec §7

- Evidence: these bullets are not in the spec's §7 list (spec:309-328).
  - Two open starts credit only the most recent, and a yesterday or 2+ day old start earns no live (plan:197-198).
  - `Fulfillment` equals `Carry-in` plus `Worked` (plan:190-191).
  - `est. EOD` value and threshold match the pre-change rule on Mon-Thu (plan:187-188).
  - "No pace clause anywhere" as an explicit test (plan:186).
  - The two smoke runs and `--help` check (plan:91).
- Effect: all are derived from spec §2/§3 and are low risk, but the instruction was that the plan adds nothing the spec does not ask for.
- Fix: either mark them as spec-derived (§2.1, §2.3, §3.2) or drop them.

### F9. low, file ownership: two files nobody owns

- Claim: plan:110 makes NOTES.md and the spec file coordinator-owned and says no task edits them. Spec §4 (spec:247-248) says NOTES.md gets a decision entry superseding entry 37.
- Evidence: the plan never schedules that entry. `coverage-baseline.json` is rewritten by the pre-commit hook when coverage improves (AGENTS.md "Coverage regression"). New tests in Task 1 will likely ratchet it. It is not listed under either task's owned files.
- Effect: the NOTES entry is easy to forget at merge. The baseline file changes in Task 1's worktree and is not declared.
- Fix: add a coordinator step "write NOTES decision superseding entry 37" and list `coverage-baseline.json` as Task 1's auto-updated file.

## Undeclared user-visible gaps

These choices are deliberate and correct. Their visible consequence is not in spec §6 or the plan. Each is a candidate for the spec's known-issues list.

### U1. The headline form switches at Thursday to Friday midnight

On Thursday night the headline reads `X left to 32h 00m by end of Thursday`. After midnight it reads `Y left to 40h 00m target`, with no weekday. The same `gap` jumps by eight hours of required time at once, and the phrase "by end of" disappears. Spec §3.1 documents each form but not the jump. §6 mentions only the Friday whole-week reading in the old pace-hint entry that is reworded.

### U2. After `stop`, the Day total line carries no due-today figure

Before: with no open stint, the Day total line still showed `X left to <required> required by end of <weekday>` (plan:63-66 removes it everywhere). After: the due-today amount appears only on the week line, and `est. EOD` appears only while a stint is open. A user who closes the stint and glances at the Day total line sees no pace information on that line. §6 declares `est. EOD` location, not this loss.

### U3. A storage read failure in `mlm week` now also hides live

Follows F6. With the pre-existing "failed read means no data" rule (spec:51-52), the current week's `Worked`, `Fulfillment` and headline silently lose live minutes while rows may still show them. No message appears.

### U4. The week line moves while the Day total stays still on earlier dates (partly declared)

Declared at spec:285-287 for `status DATE`. Not declared: the same page shows `Day total` for an earlier date that does not move at all, while `mlm week` for the same current week shows a row total that moves if that earlier date is today. The `(incl. ongoing)` and `(+ ongoing)` entry covers the flags but not the "moves vs frozen" difference between `status DATE` and `mlm week`.

## Verdict

ship-with-followups

The plan covers every spec requirement and its ownership is disjoint. F1 to F4 are medium and should be fixed in the plan before dispatch, because Task 2 would leave SPEC §8 contradicting shipped behavior, Task 1 would aim the complexity work at the wrong function, and the stale-comment and test-rewrite scope is understated. F5 to F9 and U1 to U4 can be folded in as edits or recorded as known issues.
