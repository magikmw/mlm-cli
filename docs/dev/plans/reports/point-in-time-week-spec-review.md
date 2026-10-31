# point-in-time-week spec review

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

Spec: `docs/dev/specs/2026-10-03-point-in-time-week.md` (called "spec" below).
Checked against `src/status.rs`, `src/week_view.rs`, `src/render.rs`, `src/week.rs`,
`src/stint.rs`, `docs/dev/SPEC.md`, `docs/dev/NOTES.md`, `README.md`.

Worked example (§3.4) arithmetic verified and correct: live 17:25 to 18:05 = 40m; today closed
4h00 + 3h25 = 7h25; day total 8h05; Mon-Wed rows 4h30+4h40+4h30 = 13h40; worked 21h45;
fulfillment 28h40; required 4 x 8h = 32h; gap 3h20; est. EOD 17:25 + (32h - 28h00) = 21:25 =
18:05 + 3h20; 2026-10-01 is a Thursday in ISO week 2026-40 (span 09-28..10-04); Friday sample
34h15 + 6h55 = 41h10, over target by 1h10. Label column alignment matches `LABEL_WIDTH` (15).

## Findings

### F1. High. Factual error: "carry-out is only computed for closed weeks"
- Claim: spec:38-40 (§1.1), "Carry-out is only computed for closed weeks ... so no carried
  figure changes."
- Evidence: `week_series` (src/week.rs:157-200) computes `carry_out` for every week in the walk,
  including the current one, and walks through any requested week. `status DATE` or
  `mlm week WEEK_ID` for a future week (SPEC.md:~96 "future week", src/status.rs:541-586
  `build_ledger` through `target_week`) takes its carry-in from the current week's
  `fulfillment - target`. The `WeekData` contract (src/week.rs:20-28) forbids live minutes in
  `worked_minutes`, and the spec never says where live gets added.
- Consequence: if live enters through the ledger, a future week's Carry-in changes. If it is
  added after `week_accounting`, the future week's Carry-in omits live while the current week's
  Fulfillment minus Target includes it. Either way the "no carry change" claim is unsupported.

### F2. High. Missing mechanism: live source for `status DATE` where DATE is not today
- Claim: spec:112-113 (§3.1) "The week line prints for the current week whether or not DATE is
  today. It always describes now." Spec:110 says F and W are live-inclusive.
- Evidence: src/status.rs:652-680 computes `live` only inside `if is_today`, from
  `ctx.day.open` (the classified day of `target_date`). With `DATE` = an earlier day this week,
  `ctx.day` is that day and holds no "today" open stint. `required`, weekday and `gap` are also
  computed only for is_today (src/status.rs:666-679), while the week line is built earlier
  (src/status.rs:649) from `acct.owed` with no live or required input.
- The spec must state that today's punches (plus the prev/next neighbours the splice needs) are
  classified separately from `DATE`'s, and that `required` is computed for every DATE. It states
  neither. `render::week_headline` / `status_week_line` (src/render.rs:60-107) take only `owed`
  and have no input for required, live, or weekday-name mode. The signature change is unspecified.

### F3. Medium. Internal inconsistency: `est. EOD` "unchanged value" vs Fri-Sun `required = target`
- Claim: spec:46 (§1.1) and table row spec:33 "No change to est. EOD's value"; spec:127-129.
- Evidence: spec:63-68 (§2.2) changes `required` on Fri-Sun. `est_eod` takes
  `base_gap = required - fulfillment` (src/status.rs:671, 321-331) and the `gap <= 0` threshold.
  For a target not divisible by 5 min, est. EOD moves up to 4 minutes later on Fri-Sun, and a
  day that read `target already met` can stop reading it.
- Also spec:81-83 (§2.3) says `gap` "equals the old post-live pace-hint gap ... moves unchanged".
  That is false on Fri-Sun for such targets.

### F4. Medium. Factual error: "SPEC §1.2 possible issues"
- Claim: spec:203-204 (§4) and spec:208 title. They call the list "SPEC §1.2 possible issues".
- Evidence: the list is `### 1.2a Possible issues to revisit` (SPEC.md:58). SPEC.md:36 is `### 1.2
  Non-goals`.
- The §5 entry names also do not map one to one. "Day total mixes ..." and "Pace hint counts
  live while day total does not" are two sentences of one bullet (SPEC.md:~105-112). "Pace hint
  counts down while owed stays frozen ..." and "mlm week stays frozen while status counts live"
  are one bullet (SPEC.md:~114-121). Deleting bullets as written would be wrong. Only the
  resolved sentences should go.

### F5. Medium. Missing: stale SPEC/README/NOTES text the spec does not list
- SPEC.md:94-97 (§1.2a zero-boundary entry: "00h 00m left ... matches the pace hint's gap == 0
  rule" and the current-vs-closed zero flip). It is neither removed nor reworded in spec §5.
  The "ahead of target" form it contrasts with disappears.
- SPEC.md:588 (§5 closing note "renders this case as `X ahead of target`") and SPEC.md:744-747,
  778-781 (§7.1 "ahead form drops the weekday", "day-total pace hint can still name a weekday
  though the week line names none"). §4 does not name these.
- SPEC.md:~700 (§7.1 rationale: ongoing time "isn't folded into [day total] live (avoids the
  total silently changing mid-read)"). The spec reverses a stated design reason and does not say
  so or give a new reason.
- NOTES.md:256-260 decision 37 ("live minutes never enter that math ... only appear on the
  stint's own display line and est. EOD"). The spec contradicts it for displayed fulfillment and
  worked. §4 has no NOTES item to supersede it.
- Code doc comments pinning the old rule: src/week.rs:20-28, src/stint.rs:108-109
  (`completed_minutes` "the day total"), src/week_view.rs:49-50 (`minutes` "Completed-stint
  minutes only (§2.4, NOTES 37/38)"). The spec names no code-doc updates.

### F6. Medium. Testing gap: no test plan, many existing pins break
- The spec has no testing section. Existing tests pinning the old text include
  src/status.rs:873, 900, 1161, 1184-1221, 1265-1300, 1485, 2090, 2134-2135, 2955-3024,
  3136-3150; src/week_view.rs:410, 452, 484-495, 593, 1264-1288; src/render.rs t1, t4, t5,
  t11 and t11b-d (headline `ahead of target`, `left by end of`).
- Needed new coverage that nothing assigns: `status DATE` earlier-in-week with live (F2),
  Fri-Sun required = target with a non-multiple-of-5 target, a zero-target week
  (`00h 00m left to 00h 00m by end of Monday`), gap exactly 0 and +-1, future week Carry-in with
  a running stint (F1), `mlm week` rows-sum-to-worked with live (week_view b8), `(incl. ongoing)`
  with a future-dated start, and Yesterday-aged open start marker (F8).

### F7. Low. Error-handling / edge gap: zero-target and negative edge not specified
- Spec:95-105 gives no rendering for target 0 weeks (legal, week.rs:~140, SPEC §6.2). With
  `required = 0` the Mon-Thu line reads `X over 00h 00m by end of Monday`, which is odd, and
  nothing says it is accepted. Also no statement on a stale or future open start crossing
  into the live arithmetic beyond §2.1.
- `DbWeekData` swallows storage errors into "no data" (src/week_view.rs:274-300 / 305-310 use
  `unwrap_or_default`, `.ok()`). The week line now adds a headline that depends on that data
  plus live, so a read failure yields a confident wrong "left to" figure. Pre-existing, but the
  spec raises its stakes and does not mention it.

### F8. Low. Internal inconsistency in §1 motivation
- spec:13-15 says the two deadlines are "8h apart". Old week line `12h 00m` vs pace hint
  `04h 00m` closed-only is 8h. With live (spec's own §3.4 data) the pace hint is 03h 20m, so
  12h 00m vs 3h 20m is 8h 40m. The 8h is `target - required` (40h - 32h), not the displayed gap
  difference.

### F9. Low. Scope creep: Fri-Sun `required = target`
- spec:63-68 / NOTES.md:1163 entry 90 ("Coordinator call"). The user asked for a point-in-time
  week line. Changing `required` alters est. EOD and `target already met` (F3) and only matters
  for overrides not divisible by 5 min (default 40h is unaffected). It is a behavior change
  outside the stated problem, adopted to make a label read right.

## Undeclared user-visible gaps (candidates for the spec's known-issues section)

Each is a deliberate choice whose visible consequence is not recorded in §5/§6.

- U1. The "ahead of target" form disappears for the current week. Mon-Thu with the week target
  still unmet now reads `X over 32h 00m by end of Thursday`, which sounds like overtime against
  a week whose target is far off. The old rule said "ahead" only when the whole target was met.
  Not in §1 table, §4 or §6.
- U2. `status` Day total now changes minute by minute (it reverses SPEC §7.1's stated
  "avoids silently changing mid-read"). §6 records the change only for `mlm week`.
- U3. Two markers now coexist. Today shows `(incl. ongoing)` (counted). A page for yesterday
  with an open stint keeps `(+ ongoing)` (src/status.rs:219-220, not counted), and `(+ unclosed)`
  stays for 2+ days. Same word family, opposite meaning, no note. (spec:117-123 lists them but
  not the consequence.)
- U4. `(incl. ongoing)` prints with live = 0 for a future-dated start (spec:119-120), claiming
  inclusion of nothing. The mistyped start produces no visible hint except the stint line's
  `00h 00m`.
- U5. The week line and est. EOD are no longer adjacent. Day total keeps `est. EOD 21:25` but the
  `left to 32h 00m` figure that explains it moved a line down. At gap == 0 the week line says
  `00h 00m left to ...` while the Day total says `target already met`, and on Mon-Thu
  `target already met` now means "today's slice", not the target the week line names.
- U6. The day total is today's hours while Worked/Fulfillment are week figures; once the
  Day total includes live and the week line carries the old pace figure, a weekday midnight
  crossing drops live from all of them at once. A stint open past midnight on any day (open start
  dated yesterday, no live credit per src/status.rs:670-673 `ctx.day` for today and
  week_view.rs:172 `date == today`) removes its minutes from today's Day total, Worked,
  Fulfillment and the headline until `stop`. §6 item 3 covers only the Sunday case.
- U7. Future-week Carry-in excludes live while the current week's Fulfillment includes it (F1),
  so `Fulfillment - Target` on this week's page differs from next week's `Carry-in`.
- U8. The whole-week remainder is not printed for the current week (declared at spec:235-236).
  Also the `Target:` line in `mlm week` and the headline now name different numbers on Mon-Thu
  (`Target: 40h 00m` vs `left to 32h 00m`) with only the headline word `required` gone: the
  headline never says the 32h is a prorated slice, only `by end of Thursday`.
- U9. README Saturday sample (README.md:261-262, 369, 412) and the `Total behind` future-week
  rows change wording; README samples listed in §4, but the loss of the `ahead of target` phrase
  in prose is not.
- U10. Fri-Sun, a non-multiple-of-5 target changes est. EOD and `target already met` by up to
  4 minutes (F3). Visible only with an override, undeclared.

## Verdict

needs-rework

Reason: F1 and F2 mean the stated invariants (no carry change, week line live-inclusive for any
DATE) have no specified mechanism and one is wrong against `week_series`. F3 contradicts a
non-goal. F5/F6 leave the doc and test sweep undefined. The arithmetic in §3.4 is sound.
