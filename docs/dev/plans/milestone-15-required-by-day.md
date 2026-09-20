# Milestone 15 — carry-inclusive required-by-day pace hint

> **Archived — historical planning record.** Not authoritative. Current spec: `docs/dev/SPEC.md`. Design/process history: `docs/dev/NOTES.md`.


## Goal

`status`'s daily pace hint (the "X left to `<required>` required by end of
`<weekday>`" line and its paired `est. EOD` projection) currently compares
today's day-total against a carry-free `daily target` (`week target ÷ 5`).
Per NOTES.md decision 52 (reversing decision 17) and SPEC.md §2.4/§5/§7.1
as folded by decision 53, it must instead compare the week's
**fulfillment** (`carry_in + worked`, the same figure the week-level
`owed` already uses) against **required-by-day**
(`daily target × min(today's ISO weekday number, 5)`, Mon=1…Fri=5,
Sat/Sun pinned at 5), and `est. EOD` must be re-derived from that same
new gap (`required − fulfillment`) instead of the old day-total gap.
This is a formula and wording correction to one existing pace hint plus
its projection — no new architecture, no new data.

## Architecture

Everything needed already exists and is reused as-is:

- `week::daily_target_minutes(week_target_minutes)` (src/week.rs:225) —
  purely `target ÷ 5` floored — is kept exactly as it is today. It stops
  being the thing compared against day total and becomes purely the
  per-weekday increment for required-by-day, per its own doc comment's
  stated purpose.
- `week::week_accounting(...)`'s `acct.fulfillment` (src/week.rs:69-72,
  180-189) is already computed in `status::resolve` (src/status.rs:333,
  `acct` from `week::week_accounting`) right next to where
  `daily_target_minutes` is currently used (src/status.rs:339). No new
  plumbing, no new query, no new field on `WeekAccounting` — just read
  the value that's already sitting there instead of `day_total_minutes`.
- Today's ISO weekday number (Mon=1…Sun=7) is available directly from
  chrono's `Weekday::number_from_monday()`, which is the same convention
  the rest of the codebase already relies on (e.g. `week.rs`'s
  Monday-start week machinery, `date.rs`'s weekday-name mapping). No
  existing named helper wraps it, but none is needed — it's a one-line
  call at the point of use.
- `date::format_weekday_full` (src/date.rs:179-190) — already used by
  the week line ("left by end of `<weekday>`") — is reused verbatim for
  the day line's new "required by end of `<weekday>`" wording, so both
  lines name the weekday the same way.

What changes, all within `src/status.rs`:

- `resolve()`'s daily-target/EOD block (src/status.rs:338-355): compute
  `required = daily_target_minutes(acct.target) * min(today_iso_weekday, 5)`
  instead of using `daily_target_minutes` directly as the comparison
  figure, and compute `gap = required - acct.fulfillment` instead of
  `daily_target_minutes - day_total_minutes`. `est. EOD` derives from
  this new `gap` exactly as it does today (`now + gap` when positive,
  `target already met` otherwise) — its own logic is unchanged, only
  the gap it's fed changes.
- The `DailyTargetHint` type (src/status.rs:53-58) and the rendering it
  feeds (`day_total_line`, src/status.rs:115-138): the hint needs to
  carry the `required` figure and (for rendering) today's weekday name,
  and the rendered phrase changes from "`<gap>` left to `<target>` daily
  target" to "`<gap>` left to `<required>` required by end of
  `<weekday>`" to match SPEC.md §7.1's sample line. This is a rename/
  reshape of an existing small struct and its one rendering call site,
  not a new component.
- Existing tests that assert the old wording/numbers (notably
  `t6a_f9_open_stint_with_gap_shows_est_eod`, `t6b_f9_...`,
  `t7_f10_golden_second_spec_example`, `t11_golden_full_first_spec_example`,
  src/status.rs:587-928) get their expected strings and fixture numbers
  corrected to the new formula, and new cases covering F9b's carry-in
  and Sat/Sun sub-cases are added alongside them.

Nothing in `week.rs`, `week_target.rs`, `render.rs` (the week-level
`status_week_line`), or the CLI/storage layers needs to change — the
review already confirmed multi-week carry, zero-target weeks, and
mid-week target overrides are handled by pre-existing, already-tested
machinery this change doesn't touch.

## Global constraints (every task must hold to)

- Must not change the week-level `owed`/`week` command output or
  `render::status_week_line`'s behavior or text (src/render.rs) — the
  week line's "left by end of `<weekday>`" framing and its
  fulfillment/target parenthetical are untouched.
- Must not change the "Day total" figure itself
  (`day_total_minutes`/`has_open_stint`, completed-stints-only,
  §2.4) — only the pace-hint text and gap/EOD math that sit next to it
  on the same line.
- Must preserve the existing signed-duration formatting convention
  (§4.2, `format_minutes`) for the gap figure — a negative gap renders
  the same signed way as any other summary value; no new formatting
  path.
- Must implement the F9b edge cases from the decision-53 fold:
  - Sat/Sun `required` is `5 × daily_target_minutes(target)` (i.e.
    `5 × floor(target/5)`), **not** re-derived from the raw week
    target — so it can legitimately sit a few minutes under the full
    target when the target isn't a multiple of 5.
  - A large carry-in can already put `required − fulfillment` at or
    below zero on day 1 even when day total is 0/small; the pace hint
    (and `est. EOD`) must read off `fulfillment`, independently of the
    unaffected "Day total" figure on the same line.
- The `est. EOD` sample in SPEC.md §7.1 is `20:45` (fixed by decision
  53, was previously the stale `18:35`) — any golden test mirroring
  that example must assert `20:45`, not the old number.

## Ordering / parallelization

One task. Everything in scope — the formula change, the struct/rendering
rename, and all affected/new tests — lives in one file (`src/status.rs`)
and is small enough that splitting it would only create an artificial
seam (e.g. "logic" vs "tests" landing in the same PR anyway, or two
tasks fighting over the same struct definition and the same render
function). No dependency graph or interface-pinning step is needed
beyond what's already fixed by the spec itself (the formula, the
wording, and the sample numbers above).

## Task 1 — carry-inclusive required-by-day in `status`

**Owns exclusively:** `src/status.rs` (the only file this changeset
touches; no other task exists to collide with it).

**Spec citations:** SPEC.md §2.4 ("Daily target"/"Required-by-day"
bullets, lines ~197-213), §5's explanatory note (lines ~425-433), §7.1
(sample output ~line 503, pace-hint bullet ~530-546, est.-EOD bullet
~547-552), §7.1 test cases F9 (line 727), F9b (lines 730-734), F10
(741), F11 (745). NOTES.md decisions 52 and 53.

**Acceptance criteria** (concrete enough to write tests from):

1. *F9 / golden first example (§7.1), decision-53-corrected numbers.*
   Week: target 2400 (`40h 00m`), carry_in −130 (`−02h 10m`), worked
   1885 (`31h 25m`) ⇒ fulfillment 1755 (`29h 15m`). Today = Thursday
   (ISO weekday 4). `daily_target_minutes(2400) = 480`. `required =
   480 × min(4,5) = 1920` (`32h 00m`). `gap = 1920 − 1755 = 165`
   (`02h 45m`). Open stint ends `17:45-now` (`00h 15m`) ⇒ `now =
   18:00`. `est. EOD = 18:00 + 02h45m = 20:45`. Rendered day line:
   `Day total:     07h 25m (+ ongoing), 02h 45m left to 32h 00m required by end of Thursday, est. EOD 20:45`.
   This replaces the stale `18:35`/`00h 35m left to 08h 00m daily
   target` currently asserted by `t11_golden_full_first_spec_example`.

2. *F9 three EOD states.* Open stint with positive gap ⇒
   `EodState::At(HH:MM)` from `now + gap` (using the new gap). Open
   stint with `gap <= 0` ⇒ `target already met`. No open stint ⇒ EOD
   segment omitted entirely — this branch's logic (src/status.rs
   ~345-354) is unchanged, only the `gap_minutes` value feeding it
   changes.

3. *F9b large carry-in, negative pace on day 1.* Example fixture:
   target 2400 (`daily_target = 480`), today = Monday (ISO weekday
   1), carry_in = +2000, worked = 0, day_total_minutes = 0.
   `required = 480 × 1 = 480`. `fulfillment = 2000 + 0 = 2000`.
   `gap = 480 − 2000 = −1520`, so the pace hint reads negative /
   `target already met` (per which of gap-display vs EOD-state is
   being asserted) while `Day total: 00h 00m` on the same line is
   unaffected and unchanged by this fixture.

4. *F9b Sat/Sun pin, multiple-of-5 target.* Today = Saturday or
   Sunday (ISO weekday 6 or 7) ⇒ `min(weekday,5) = 5` ⇒ `required =
   5 × daily_target_minutes(target)`. With target 2400, `required =
   5 × 480 = 2400`, matching the full week target exactly (this case
   only, because 2400 is a multiple of 5).

5. *F9b Sat/Sun pin, non-multiple-of-5 target override.* Target
   overridden to `33h 31m` = 2011 minutes. `daily_target_minutes(2011)
   = floor(2011/5) = 402`. Sat/Sun `required = 5 × 402 = 2010` =
   `33h 30m` — one minute under the override itself (2011), not
   silently rounded up to match it. Test must assert this exact
   under-by-a-few-minutes value, not equality with the raw target.

6. *F10 (unchanged behavior, re-verify after the rename).* `status
   DATE` for a past date in a different, closed week: no daily-target/
   est.-EOD lines at all (`daily_target`/`eod` stay `None` because
   `is_today` is false) — logic untouched, but any fixture asserting
   the *other* week's numbers must still use the corrected formula if
   it happens to also exercise today's line.

7. *F11 (unchanged behavior).* `status DATE` for a different day
   within the current, still-open week: daily-target/EOD lines stay
   absent (`DATE != today`), the week line still keys its deadline
   framing to today's actual weekday — this is `render`/week-line
   territory, not touched by this task; just confirm the existing
   F11 test still passes unmodified.

8. Rendered wording changes from `"<gap> left to <target> daily
   target"` to `"<gap> left to <required> required by end of
   <weekday>"`, using `date::format_weekday_full` for the weekday
   word, everywhere the old phrase appeared (`day_total_line` and
   every test asserting that literal string).

**Out of scope for this task:**

- Any change to `week::daily_target_minutes`'s own formula (`target ÷
  5` floored) — it is reused unchanged as the per-weekday increment.
- Any change to `week::week_accounting`, `WeekAccounting`'s fields, or
  how `fulfillment`/`owed`/`carry_in`/`carry_out` are computed
  (src/week.rs) — only *consumed*, not modified.
- Any change to `render::status_week_line` or the week line's own
  text/behavior (src/render.rs) — that line's wording and framing are
  unaffected by this changeset.
- Multi-week carry chaining, zero-target weeks, idle gap weeks
  (F8/F8b), mid-week target overrides taking effect (F7/F7b) — all
  pre-existing, already-tested, and out of scope per the spec review's
  own finding that this change needs no new plumbing there.
- Introducing a new named "ISO weekday number" helper/type — a direct
  `chrono::Weekday::number_from_monday()` call at the point of use is
  sufficient; no cross-cutting abstraction is warranted for one call
  site.

VERDICT: green
FILE: docs/dev/plans/milestone-15-required-by-day.md
1 task — the whole change (formula, struct/rendering rename, and all affected/new tests) lives in one file, src/status.rs, with no disjoint-file seam to justify splitting it.
