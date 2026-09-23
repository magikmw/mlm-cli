# Status/week wording fixes — design spec

**Baseline**: written against `v0.3.5`.
**Status**: adversarially reviewed twice: spec review 2026-09-23
(`docs/dev/plans/reports/status-wording-fixes-spec-review.md`,
needs-rework, 5 findings, folded — NOTES.md entry 66) and changeset
plan review 2026-09-23
(`docs/dev/plans/reports/status-wording-fixes-plan-review.md`,
needs-rework, 5 findings, folded back into this spec — NOTES.md entry
67). Locked.

## 1. Summary

Four of SPEC.md §1.2a's known issues (filed in NOTES.md entry 61, all
pre-existing, none caused by the `boundary-context-cues` changeset)
get fixed here:

- **A** — the closed-week/closed-date headline (`render.rs`'s
  `week_headline`, `WeekFraming::Closed` branch) reads `"Total still
  owed: Xh Ym"` for a week that finished short of target. Its sibling
  branch for a week that finished *over* target already reads `"Total
  ahead: Xh Ym"` (§1.2a bullet, README "Known issues"). The debt-toned
  phrasing sits next to SPEC's otherwise neutral vocabulary
  (`fulfillment`, `carry-in`, `shortfall`, `surplus`). Renamed to
  `"Total behind: Xh Ym"` — the direct pair of `"Total ahead"`.
- **B** — `status.rs`'s `est. EOD HH:MM` line (§7.1) drops the date:
  `(now + gap_minutes).time()` discards whatever day the estimate
  actually lands on. When the gap is large enough to cross midnight,
  the clock time shown can belong to tomorrow with nothing in the
  string saying so. Gains a `(tomorrow)` suffix when that happens,
  following the same disclosure-suffix pattern the
  `boundary-context-cues` changeset already used for the cross-midnight
  stint span cue and the open-stint-age captions.
- **C** — once `today`'s weekday reaches the work week's last day,
  `status.rs`'s day-total pace hint (`"X left to Y required by end of
  <weekday>"`) and `render.rs`'s week line (`"X left by end of
  <weekday>"`) land on the identical weekday name and, since
  `required_minutes` is capped at the week's full target from that
  point on (§2.4/§3 below), the identical figure too — different
  sentences saying close to the same thing directly above each other.
  This is **not** a single calendar day: under the default 5-day work
  week, `required_minutes` plateaus at the full week target on
  Friday, Saturday, *and* Sunday alike, so the redundancy applies to
  all three (§3 corrects the original triage note's "one day"
  framing). The day-total clause's trailing `required by end of
  <weekday>` becomes `required today` on each of those days, dropping
  the repeated weekday token without dropping the weekday-vs-target
  meaning. **Constraint**: the label column stays fixed-width — see
  §3 below for what that requires.
- **D** — no behavior change; a documentation gap. README's `status`/
  `week` output examples only ever show a *current*-period week — a
  user who queries a closed date/week for the first time there hits
  the different sentence shape from **A** unexplained. SPEC.md already
  carries closed-period worked examples (§7.1 lines 707-716, §7.2
  lines 744-766); those just need their string updated for Fix A, not
  a new example (§5 below).

## 1.1 Non-goals

- Bullets 1-4 of §1.2a (the hidden-state and undisclosed-rule known
  issues triaged separately, still open) — untouched by this
  changeset.
- No new `EodState`/`OpenStintAge`-style enum variant for **B**; a
  bool alongside the existing `NaiveTime` is enough (§2).
- No change to how `gap_minutes`, `required_minutes`, or `owed_minutes`
  are computed anywhere — every fix here is presentation-only, over
  figures the codebase already derives correctly.

## 1.2 Known issues

None newly accepted by this changeset — **D** closes a known issue by
documenting it rather than leaving a gap.

## 2. Fix B — `est. EOD` tomorrow marker

`status.rs`'s `EodState::At(NaiveTime)` (line 49) gains a second field:

```rust
At(NaiveTime, bool) // bool: true when the estimate falls on a later
                     // calendar date than `today`
```

At the construction site (`status.rs:472`, inside `resolve()`), the
full `NaiveDateTime` is already computed as `now + ChronoDuration::minutes(gap_minutes)`
before `.time()` discards the date. Compare its **`.date_naive()`**
against `today` (the same `today` already in scope at that point, per
`week_headline`'s existing convention of using the injected "now"
date, never the displayed date) and carry the result into the new
bool. **Not** `.date()` — that method is deprecated since chrono
0.4.23, returns `Date<Local>` rather than `NaiveDate`, and won't
type-compare against `today: NaiveDate` (`status.rs:409`) at all; this
repo's CI runs `clippy --all-targets -- -D warnings`
(`.github/workflows/ci.yml:50`), which rejects the deprecation lint
outright even if a cast made it compile.

`day_total_line` (`status.rs:185`) renders:

```
Some(EodState::At(t, false)) => ", est. EOD {t}"
Some(EodState::At(t, true))  => ", est. EOD {t} (tomorrow)"
```

The word is always `(tomorrow)`, never a weekday name or a date — the
gap this fixes is the day boundary, not calendar precision, and this
matches the suffix vocabulary already in use for the open-stint
captions (`(+ ongoing)`, `(+ unclosed)`) and the stint-line span cue
(`, spans to next day`).

**Test plan note**: every existing single-argument `EodState::At(t)`
construction becomes two-argument once this lands — mechanical, but
Task 1 updates all of them, not just the ones its own new test cases
touch. The known sites are `status.rs:658` (`t1`), `950` (`t6a`),
`1158` (`t11`), `1256` (`t12`), `1320` (`t13`) — five, not the six an
earlier draft listed (`base_view` sets `eod: None` and has no such
construction — corrected from plan review, finding 4). Task 1 re-runs
`grep -n "EodState::At(" src/status.rs` rather than trusting this
list, in case it's drifted.

**Edge case**: if `gap_minutes` is large enough to cross *two*
midnights (more than 24h still owed today), the marker still just
reads `(tomorrow)` — technically imprecise past one day out, but
`daily_target`'s `required_minutes` is capped at 5 days' worth of
target and single-day gaps this large are already an extreme case; a
precise date is out of scope for a suffix-style cue and would break
the established pattern of the other three suffixes.

## 3. Fix C — day-total wording once the week target is capped, fixed-width constraint

`status.rs`'s `day_total_line` (line 170-183): the daily-target
clause's trailing phrase changes based on whether `required_minutes`
has reached its cap for the week:

```
!day_reaches_week_cap  → ", {mag} {word} {req} required by end of {weekday}"   (unchanged)
day_reaches_week_cap   → ", {mag} {word} {req} required today"
```

This is a work-week-length check, not an ISO-week check — ISO week
(§1.3) only identifies a week by (year, week-number) and fixes
Monday as its start; it says nothing about how many of those days
count as work days. The 5-day work week is a separate constant that
already exists in the code twice, ungrouped: `week.rs:226`'s
`daily_target_minutes` (`week_target_minutes.div_euclid(5)`) and
`status.rs:464`'s `required_minutes` calculation
(`weekday_number.min(5)`). Fix C reuses `status.rs:464`'s existing
`weekday_number.min(5)` result directly — `day_reaches_week_cap =
weekday_number.min(5) == 5` — rather than writing a second bare `5`
literal. If a future changeset ever makes the work week's length
configurable, both existing sites and this one change together; this
fix doesn't widen that surface, just reads the same cap a third time.

**Data flow (corrected from plan review, finding 1)**: `day_total_line`
(`status.rs:157-190`) is a pure `&StatusView -> String` function — it
never sees `today` or `weekday_number` at all, only
`view.daily_target: Option<DailyTargetHint>` (`required_minutes`,
`gap_minutes`) and `view.weekday_name: Option<String>`. Neither field
encodes `day_reaches_week_cap`, and matching on `weekday_name`'s
string value (`"Friday"`/`"Saturday"`/`"Sunday"`) would reintroduce
the "second bare constant" this section just ruled out, as three
weekday-name literals instead of a `5`. So `DailyTargetHint`
(`status.rs:57-64`) gains a third field, `day_reaches_week_cap: bool`,
computed once at `resolve()` alongside `required_minutes` and
`gap_minutes` (`status.rs:463-469`) and threaded through unchanged —
the same shape Fix B already uses for its own bool (§2). This mirrors
Fix B closely enough that Task 1 should implement them the same way:
compute the fact where `today`/`weekday_number` are in scope, carry it
as a field, let the purely-rendering function match on it.

**Correction from spec review**: `weekday_number.min(5) == 5` is true
for Friday (5), Saturday (6), **and** Sunday (7) under
`number_from_monday()`'s Monday=1..Sunday=7 numbering — not "the one
day," which an earlier draft of this section wrongly assumed
throughout. That's not a bug to route around: `required_minutes`
genuinely is the same capped full-week-target figure on all three
days (there's no Sat/Sun gate on the `is_today` branch that computes
`daily_target` — `status.rs:461` — so a `status` query on a weekend
already renders this hint today), so the day-total/week-line
redundancy this fix addresses genuinely spans all three days, and
`day_reaches_week_cap` (deliberately not named `is_last_workday`,
which would misdescribe Sat/Sun) is the correct, complete condition.
The variable name and every description in this section reflect that;
none of it implies exactly one day.

**Test plan addition**: most of `status.rs`'s daily-target/EOD tests
pin `today` to Thursday 2026-02-12 (`status.rs:549-551`), unaffected
by this fix. One existing test already hits the capped range and
**will break**, not just lack coverage (corrected from plan review,
finding 2): `resolve_f9b_sunday_pin_non_multiple_of_five_target_override`
(`status.rs:1759-1780`) pins a Sunday and asserts, via `render()`,
`"33h 30m required by end of Sunday"` — under this fix that becomes
`"33h 30m required today"`, so Task 1 updates that assertion as part
of this fix, not as incidental fallout. A companion Saturday-pinned
test at `status.rs:1739-1757` doesn't call `render()` so it won't
break, but Task 1 should still add explicit coverage: a Friday case
and one weekend case, each asserting `required today` and the *same*
`required_minutes` figure as the adjacent weekday case, plus a
Monday-through-Thursday case confirming `required by end of {weekday}`
stays unchanged.

**The user's layout constraint**: the day-total line must not visibly
"jump" — shrink or shift — on the days this wording differs.
`required today` (13 chars) is shorter than `required by end of
Thursday` (25 chars) or `required by end of Tuesday` (27 chars,
longest weekday). Since `day_total_line` builds one line with no
column after this clause (it's the line's last segment before
`est. EOD`, which itself is conditional on `has_open_stint`), a
shorter clause here does not disturb any later fixed-width column —
there isn't one downstream of it. So "jump" here can only mean the
line's own visible length changing day to day, which happens anyway
today (weekday names are different lengths: `Monday` vs `Wednesday`).
**Resolved with the user**: no padding. Weekday names already vary in
rendered length day to day (`Monday` vs `Wednesday`), so `required
today` being shorter than the longest weekday form is one more length
variation among the ones the line already has, not a new kind of
change. `required today` renders exactly as written, no trailing
padding.

## 4. Fix A — "Total behind" rename

`render.rs:68`, `WeekFraming::Closed if owed_minutes > 0` branch:

```rust
format!("Total behind: {}", format_minutes(owed_minutes))
```

was:

```rust
format!("Total still owed: {}", format_minutes(owed_minutes))
```

Mirrors the existing `WeekFraming::Closed` else-branch at line 70,
`format!("Total ahead: {}", format_minutes(-owed_minutes))`, unchanged.

Every test asserting the literal string `"Total still owed"` updates
to `"Total behind"` — `render.rs`, `status.rs`, `week_view.rs`,
`commands.rs` all currently do (found via
`grep -rn "Total still owed" src/`). No test asserts the *old*
`"Total behind"` currently, so no ambiguity there.

The docs side of this rename is not confined to §1.2a and the two
worked examples — `grep -n "Total still owed" docs/dev/SPEC.md`
returns 8 hits: the two in §1.2a (deleted as part of §6/interaction
section below), the two worked examples (§5), and four more in
normative prose describing the wording rule itself, at lines 647, 672,
679, and 776 (each some variant of "the plain `Total still owed`/
`Total ahead` form"). All eight update to `Total behind` — Task 2
re-runs that grep rather than trusting this enumeration, in case line
numbers have drifted by the time it starts.

## 5. Fix D — closed-period worked example

**Correction from spec review**: SPEC.md already has both closed-period
worked examples — a `status` one at §7.1 lines 707-716 (explicitly
introduced as "a past date, different week") and a `week` one at §7.2
lines 744-766 (introduced as "a past or future week has no 'today' to
frame a deadline against"). SPEC.md needs no *new* example; it needs
its two existing ones' `Total still owed` strings updated by Fix A
(§4), same as everywhere else. Adding a second, redundant closed-period
example to SPEC.md alongside those would be scope creep, not a fix.

README.md is the actual gap: its `status` examples (lines ~195, ~215)
and `week` examples (lines ~233, ~262) all use the current week
(`2026-37`) — no closed-period example exists there at all. README
gains one worked example each for a closed (past) `status` date and a
closed `week`, output verbatim, right next to the existing
current-period examples. The example should show the (post-fix-A)
`"Total behind: Xh Ym"` line and the absence of the fulfillment/target
parenthetical, matching what `render.rs`'s `WeekFraming::Closed` branch
actually produces (per `render.rs:67-70`, `status_week_line`'s
closed-branch tests around line 276-336) — SPEC.md's own §7.1/§7.2
closed examples are the reference source for what README's should
show, since they already render the real output shape.

**Existing README example goes stale under this changeset (plan
review, finding 3) — not a new example, a correction to a live one.**
`README.md:196-198` already ships a Saturday `status` example whose
output line reads `..., 04h 35m over 40h 00m required by end of
Saturday, est. EOD target already met`. Saturday is exactly Fix C's
trigger condition (`weekday_number.min(5) == 5`), so once Fix C ships
the real output for that scenario is `required today`, not `required
by end of Saturday` — this is the only `required by end of`
occurrence in all of `README.md` (`grep -n "required by end of"
README.md`), so nothing else would catch the drift. Task 2 updates
this existing example's output line to match, as part of Fix C, not
Fix D. Separately, that same line concatenates `est. EOD target
already met`, a phrase the renderer cannot produce — `day_total_line`
(`status.rs:184-188`) emits either `, est. EOD HH:MM` *or* `, target
already met`, never both. This predates this changeset and isn't
caused by any of its four fixes, but Task 2 is already editing this
exact line for Fix C, so it corrects this too rather than
re-publishing a line that was already wrong before this changeset
touched it — pick whichever of the two EOD phrases matches a
plausible re-derivation of the example's own numbers (has an open
stint with a nonzero gap → `est. EOD HH:MM`; gap already closed →
`target already met`), never both.

## 6. Task split

- **Task 1 (code)**: `src/render.rs`, `src/status.rs` — fixes A, B, C
  and their in-file `#[cfg(test)]` assertions. Also fixes the same
  literal-string assertions in `src/week_view.rs` and
  `src/commands.rs` (A's rename only touches those two as test-string
  updates, no production-code change there).
- **Task 2 (docs)**: `README.md`, `docs/dev/SPEC.md` — fix D (new
  closed-period examples in README only; SPEC.md's two existing ones
  get their string updated, not duplicated), correcting README's
  existing Saturday `status` example for Fix C plus its unrelated
  `est. EOD`/`target already met` concatenation bug (§5), the full
  8-hit `Total still owed` → `Total behind` rename across SPEC.md
  (§4), the `est. EOD (tomorrow)` and `required today` cases
  documented in SPEC.md §7.1, and updating SPEC.md §1.2a and README's
  "Known issues" to drop the four bullets this changeset closes
  (matching the convention NOTES.md entry 61 used for its own fixed
  bullets).

No interface contract between the two tasks — Task 2 depends on
knowing Task 1's exact output strings (for D's worked example and for
confirming the `[!]`-adjacent known-issue bullets are worded
accurately), so it reads Task 1's task plan and completion report
before writing its own example, but touches no file Task 1 owns.

## Interaction with the project spec

- SPEC.md §7.1 (`est. EOD` line) — gains the `(tomorrow)` suffix case.
- SPEC.md §7.1 (day-total pace hint) — gains the `required today` case
  for Fri/Sat/Sun (weekday_number.min(5) == 5), not a single weekday.
- SPEC.md, all 8 occurrences of `"Total still owed"` (§1.2a ×2, §7.1/
  §7.2 worked examples ×2, normative prose at lines 647/672/679/776)
  → `"Total behind"`.
- README.md gains new closed-period `status`/`week` worked examples
  (SPEC.md needs none — its two already exist, just get the string
  update above).
- SPEC.md §1.2a — drops the four bullets this changeset fixes (kept:
  the four in NOTES.md's other triage group — splice-gate disclosure,
  anomaly remedy guidance, lone-unclosed-start ambiguity, multi-day-old
  forgotten stop).
- README.md "Known issues" — same four bullets dropped, matching
  SPEC.md.
