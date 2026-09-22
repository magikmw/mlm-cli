# Final review: changeset `boundary-context-cues`

**Scope reviewed**: `git diff 5b18ed1..boundary-context-cues` — the
whole changeset diff from `main`'s pre-changeset baseline to the branch
tip (`c28977d`, the merge of `worktree-agent-aadafdd8c6defb4ce`).

**Diffstat**: `coverage-baseline.json` (+1/-1), `src/render.rs` (+74/-19
incl. tests), `src/status.rs` (+532/-? incl. tests), `src/stint.rs`
(visibility only). No doc changes.

**Verdict**: **ship-with-followups** — 3 findings, none blocking.
Behavior is correct and matches every worked example in the spec,
verified by hand against a fresh scratch database (not by reading
alone).

**Read**: the locked spec (`docs/dev/specs/2026-09-20-boundary-context-cues.md`,
status line: locked, adversarially reviewed, amended once for §4's Day
total extension / NOTES decision 62), the changeset plan
(`docs/dev/plans/boundary-context-cues-plan.md`, collapsed to 1 task
after 4 rounds), the task plan
(`docs/dev/plans/boundary-context-cues-task-1-status-render.md`,
including its two named judgment calls), and SPEC.md §1.2a / §4.3.1 /
§7 / §7.1 / §7.2.

## 1. Gate checks

| Gate | Result |
|---|---|
| `cargo build` | clean |
| `cargo test` | 443 + 2 + 1 + 2 + 5 = 453 tests, 0 failures |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo clippy -- -W clippy::cognitive_complexity` | no warning (the task plan's flagged `day_total_line` risk did not materialize) |
| `cargo fmt --check` | clean |
| Coverage baseline | bumped 98.9556 -> 99.0111 (improve-only, per `.githooks/pre-commit`) |

## 2. Manual verification against the spec's worked examples

All runs used `MLM_DB_PATH=<scratchpad>/*.db` against fresh scratch
databases; the real app-data path was never touched. System date during
review: 2026-09-20 (Sunday, week 2026-38), so week ids and weekday
names differ from the spec's illustrative `2026-02-*` / `2026-07`
examples — every other character was compared literally.

### §2 cross-midnight span suffix — PASS

Seeded `start -d -3 23:30`, `stop -d -2 00:45`; `status -3`:

```
  23:30-00:45  (01h 15m, spans to next day)
```

Byte-identical to spec §2's example, including the fixed-width
`HH:MM-HH:MM` field. An ordinary same-date stint on the same view
renders `  09:00-10:00  (01h 00m)` — unchanged.

### §3 receiving-date header suffix — PASS (both cases)

Same fixture, `status -2`:

```
Fri 2026-09-18  (00:45 continues previous day's stint)

Day total:     00h 00m
Week 2026-38:  38h 45m left by end of Sunday (fulfillment 01h 15m / target 40h 00m)
```

This *is* the "only punch of the day" edge case the spec §6 / task plan
call out: the stint-list section is entirely omitted (SPEC.md §7.1) yet
the header suffix, Day total and week line all render, and the spliced
75 minutes appear in the week line. Two spaces before the paren,
matching spec §3.

Positive case with further punches on the receiving date (added
`09:00-10:00` to that date): suffix still present, exactly once, and
the new stint renders normally — confirming the suffix keys off the
date's *chronologically first* punch, not "the date has no stints".

Negative case: the donor date's own header (`Thu 2026-09-17`) carries
no suffix; an ordinary non-boundary date carries none.

`week 2026-38` was also run: its per-date row for the receiving date
shows a plain `00h 00m` with no marker — the accepted status/week
asymmetry (spec §3/§8), unchanged, as intended.

### §4 three-way open-stint age caption — PASS (all three, Day total included)

Three separate scratch DBs, one open `start` each:

```
today      Day total:     00h 00m (+ ongoing), ...     |   17:45-now    (02h 41m, ongoing)
yesterday  Day total:     00h 00m (+ ongoing)          |   23:10-now    (21h 16m, ongoing - elapsed since now, not a running total)
3 days ago Day total:     00h 00m (+ unclosed)         |   23:10-now    (unclosed)
```

Today's line is byte-identical to pre-change output. Yesterday keeps
the live figure and gains the caption verbatim per spec §4. The
2+-days-back line drops the duration figure entirely (no digits
anywhere in the parenthetical) and Day total's suffix flips to
`(+ unclosed)` — the §4 amendment's Day-total-matching requirement,
satisfied in both directions (`(+ ongoing)` for today/yesterday,
`(+ unclosed)` only for 2+).

Mixed case also checked (2+-days-back date with one completed stint and
one open): `Day total:     01h 30m (+ unclosed)` above
`  09:00-10:30  (01h 30m)` / `  14:00-now    (unclosed)` — the day
total still counts only completed minutes (§7.1), and the two lines do
not contradict each other.

### §5 carry-in parenthetical — PASS (all three worked examples)

Constructed real ledgers (prior-week target override + punches) so the
figures travel the full `build_ledger -> week_accounting -> week_line
-> status_week_line` chain:

- Deficit (prior week target 3h, worked 50m; this week 31h25m):
  `Week 2026-38:  10h 45m left by end of Sunday (fulfillment 29h 15m = worked 31h 25m + carry-in -02h 10m / target 40h 00m)`
  — spec §5's first example, character-for-character after the week id.
- Surplus (prior target 0h, worked 50m; this week 39h10m):
  `... (fulfillment 40h 00m = worked 39h 10m + carry-in 00h 50m / target 40h 00m)`
  — no `+` printed on the positive carry-in, per §4.2.
- Fulfillment goes negative (prior target 5h, worked 0; this week 3h):
  `Week 2026-38:  42h 00m left by end of Sunday (fulfillment -02h 00m = worked 03h 00m + carry-in -05h 00m / target 40h 00m)`
  — the `42h 00m left` headline matches the spec's own example too.
- `carry_in == 0`: unchanged one-term form
  (`(fulfillment 01h 15m / target 40h 00m)`).
- Closed (past) week: `Week 2026-37:  Total still owed: 05h 00m` — no
  parenthetical, no `carry-in` token, matching §5's closing paragraph
  and pinned by the new `t38`.

## 3. Specifically requested verifications

### `splice_candidate` visibility change is the only `stint.rs` edit — CONFIRMED

The entire `src/stint.rs` hunk is:

```rust
-fn splice_candidate(earlier_open_count: usize, later: &DayStints, later_punches: &[Punch]) -> bool {
+pub(crate) fn splice_candidate(
+    earlier_open_count: usize,
+    later: &DayStints,
+    later_punches: &[Punch],
+) -> bool {
```

Body, signature, parameter order, doc comment and both in-module call
sites untouched; the multi-line reflow is `rustfmt`'s width rule, not a
semantic change. No other hunk in that file.

### The two fresh `classify()` calls do not reuse the spliced `day` — CONFIRMED

`src/status.rs`, inside `resolve()`, immediately after the pre-existing
`let day = stint::classify_at(...)`:

```rust
let receiving_end_time = {
    let prev_classified = stint::classify(&prev_punches, now_utc);
    let fresh_target_classified = stint::classify(&punches, now_utc);
    stint::splice_candidate(
        prev_classified.open.len(),
        &fresh_target_classified,
        &punches,
    )
    .then(|| {
        fresh_target_classified.orphaned_ends[0]
            .punch
            .at_utc
            .with_timezone(&Local)
            .time()
    })
};
```

`day` appears nowhere in this block. Both operands are independent
`classify()` results over slices already in scope (no new data fetch).
The `[0]` index is reachable only inside `.then(|| ...)`, whose closure
runs only when `splice_candidate` already established
`orphaned_ends.len() == 1` — `then` (lazy) not `then_some` (eager), so
no panic path. This is exactly what the two rounds of plan rework
demanded, and the positive-case test
(`resolve_midnight_splice_later_date_shows_no_orphan_anomaly`) plus my
manual run prove it actually fires rather than silently never firing.

Cross-checked for faithfulness to the real splice decision: inside
`classify_at`, the `(prev, day)` gate is
`splice_candidate(prev.open.len(), &day, punches)` where
`prev = classify(prev_punches, now)` — the same value `resolve()`
recomputes — and the only mutation on the prev side anywhere is
`day.orphaned_ends.remove(0)`, which never touches `open`. So
`resolve()`'s gate cannot diverge from `classify_at`'s, including when
the previous date itself received a splice from *its* predecessor.

### Plain ASCII per SPEC.md §7 — CONFIRMED

Every added line in `src/**` was scanned for bytes outside
`0x20..=0x7E`: the only hits are `§` and `—` inside `//` / `///`
comments (matching the file's pre-existing convention). No added string
literal, and no rendered output, contains a non-ASCII byte. SPEC.md
§7's rule is explicitly about rendered layout characters (restated at
SPEC.md line 181), so comments are out of its scope. The five
`status.rs` literals are pinned by the new
`t13b_new_boundary_context_cue_literals_are_plain_ascii` (byte-range
assertion per variant) and `carry-in` by the extended
`t33_every_produced_string_is_ascii`. All manual CLI output above is
ASCII.

## 4. Compliance with the plans and the spec

- Spec §§2-6: all implemented as written; §6's testing plan is covered
  point for point (span present/absent/open-never, header suffix
  positive + two negatives + only-punch-of-day, three age buckets with
  Day total matching, carry-in zero/deficit/surplus/negative-
  fulfillment, plus regression).
- SPEC.md §1.2a bullets 1/2/6/7 are the four behaviors now fixed;
  bullets 3/4/5 are untouched in code, as scoped.
- SPEC.md §4.3.1 / §7 / §7.1 rendering contracts: the new suffixes all
  occupy the existing duration parenthetical or append to the existing
  header line — no new lines, no new bracket markers, `LABEL_WIDTH`
  alignment preserved, `render()`'s no-doubled/trailing-blank-line
  invariant still asserted and passing.
- No accounting math changed: `week.rs`/`week_view.rs`/`stint.rs` logic
  untouched; `worked` is *derived* in `status_week_line` as
  `fulfillment_minutes - carry_in_minutes` (not passed), so it cannot
  drift from `WeekAccounting.worked`.
- **The implementer's "no deviations" claim holds** on substance. Two
  immaterial departures from the letter of the task plan: the ASCII
  coverage was added as a new sibling test `t13b_...` rather than by
  editing `t13_...` in place, and the §5 render assertions landed as
  new `t34`-`t38` rather than only extending existing tests. Both are
  additive and stronger than the plan's minimum; neither changes an
  existing expected string. Every existing golden test passes with only
  the mechanical new-field additions the plan predicted.

## 5. Security

Nothing in this diff builds a shell-replayable, SQL, or otherwise
injectable string. The three new user-data-adjacent strings are:

- the header suffix, whose only interpolation is
  `NaiveTime::format("%H:%M")` over a punch instant — two digits, a
  colon, two digits, from the DB's `at_utc`, never from free text;
- the stint-line suffixes and `(unclosed)`, which are constant literals
  plus `format_minutes` over an `i64`;
- the week-line parenthetical, four `format_minutes` calls over `i64`s.

Note bodies (the one genuinely free-text field) still pass through the
pre-existing, unchanged path, and `t13`'s non-ASCII-note pass-through
assertion still holds. No new `std::process`, no new SQL construction,
no format string built from input. `punches_for_date` continues to be
called with `NaiveDate` values through the existing parameterized
queries. `splice_candidate` widening to `pub(crate)` grants no
cross-crate access and the function is a pure predicate. No security
finding.

## 6. Findings

### F1 — spec §7's SPEC.md/NOTES.md updates never landed; SPEC.md now contradicts shipped behavior (medium, follow-up)

The changeset spec's §7 lists five required doc edits. None are on the
branch (`git diff --stat 5b18ed1..boundary-context-cues -- docs` is
empty). Concretely, today:

- SPEC.md §1.2a (lines 58-93) still lists all four bullets this
  changeset fixed, including "has no visual cue that it spans two
  calendar days", "shows **zero trace of it**", the "443h 06m, ongoing"
  no-caption bullet, and the carry-in bullet.
- SPEC.md §4.3.1 (line 457) still asserts "a spliced stint carries no
  visual marker distinguishing it from an ordinary same-date stint
  (deliberate — see §1.2a)". That sentence is now **false**: §2's span
  suffix and §3's header suffix are exactly such markers.
- SPEC.md §7.1's worked example and prose document neither the header
  suffix, nor the expanded carry-in parenthetical, nor Day total's
  `(+ unclosed)` variant. §7.2 notes neither the backdated caption nor
  the accepted status/week asymmetry.
- NOTES.md has no decision entry for this changeset (decision 62 is
  cited by the spec's own status line as the amendment's source, but
  nothing new was written).

The task plan deliberately scoped these out ("tracked separately from
this code task"), and Task 1 was the changeset's *only* task — so
nothing on the branch does them. This is the one place where the
changeset is genuinely incomplete rather than merely improvable. Not a
code-correctness blocker; it does leave the repo's own spec stating the
opposite of what the binary prints, which is exactly the kind of drift
`docs: fix drift...` (6929889) was cleaning up two commits earlier.
Recommend a doc-only follow-up commit before release.

### F2 — `stint_line()` computes a suffix it throws away and re-matches the same tuple twice (low, code quality)

`src/status.rs` `stint_line()`:

```rust
let suffix = match (line.end, line.open_stint_age) {
    ...
    (StintEnd::Now, Some(OpenStintAge::TwoOrMoreDaysBack)) => String::new(),
};
if matches!(
    (line.end, line.open_stint_age),
    (StintEnd::Now, Some(OpenStintAge::TwoOrMoreDaysBack))
) {
    format!("  {:<11}  (unclosed)", range)
} else { ... }
```

The `TwoOrMoreDaysBack` arm's value is unreachable-by-construction dead
data: the `if` below discards `suffix` in precisely that case, and the
same two-field tuple is matched twice in a row. A single `match`
returning the finished line (the `TwoOrMoreDaysBack` arm returning
`format!("  {:<11}  (unclosed)", range)` directly) is shorter, removes
the dead arm and the duplicated scrutinee, and keeps the cognitive
complexity budget it was written to respect. Behavior is correct as
shipped; this is a readability cleanup only.

Related and accepted, not a finding: the `(StintEnd::Now, None)`
fallback arm is dead by construction (`build_stint_lines` always
populates `Some(..)` for opens) and is documented as a deliberate
judgment call in the task plan's "Risks, ambiguities and
disagreements". I agree with keeping it — it degrades to today's
`, ongoing` rather than panicking — but it should survive any F2
refactor.

### F3 — the only end-to-end carry-in assertion is substring-shaped, so a transposed `acct` field would pass (low, test strength)

`resolve_end_to_end_nonzero_carry_in_shows_worked_and_carry_in_breakdown`
asserts only:

```rust
assert!(out.contains("worked") && out.contains("+ carry-in"), ...);
```

The exact figures are pinned only at the `render::status_week_line`
unit level (`t35`-`t37`), where the carry-in value is supplied by the
test itself. So the one test covering the real
`build_ledger -> week_accounting -> week_line -> status_week_line`
chain would still pass if `week_line()` handed
`status_week_line` the wrong `WeekAccounting` field (e.g. `acct.worked`
instead of `acct.carry_in`) — the template's literals would be
unchanged and only the numbers would be wrong.
`week_line_matches_status_week_line_field_for_field` does not close the
gap either: it passes `acct.carry_in` on both sides, so a transposition
inside `week_line` mirrored in the test's own call would go unnoticed
(and that fixture's `carry_in` is 0 anyway). Cheap fix: assert the full
expected parenthetical string for that fixture (its values are
deterministic: target 8h / worked 10h in week 2026-06 gives
`carry-in 02h 00m`). I verified the real values by hand (§2 above, all
three spec examples reproduced end-to-end through `resolve()`), so this
is a regression-protection gap, not a live defect.

## 7. Notes that are not findings

- **§2's gate is inequality, not ordering.** `spans_to_next_day:
  s.start.date != s.end.date` prints "spans to next day" for any
  date mismatch, where spec §2 says "end falls on a *later* calendar
  date". Unreachable today: `classify` pairs within one date and
  `classify_at` only ever splices onto the immediately-following date,
  so a backwards or multi-day pairing cannot be constructed. Worth a
  comment at most.
- **The task plan's flagged future-date extrapolation is inert.**
  `target_date > today` falls into `TwoOrMoreDaysBack` via the `else`
  arm. I tried to reach it: `mlm start -d 2026-09-25 09:00` is rejected
  at write time (`error: invalid DATE "2026-09-25": date is in the
  future`), so a future-dated open stint cannot exist in the database.
  The ambiguity the task plan asked a reviewer to rule on is therefore
  closed by an existing input guard, not left open. Current behavior
  needs no change.
- Assertion strength elsewhere is good: the load-bearing tests pin
  whole literal lines (`"  23:10-now    (10h 35m, ongoing - elapsed
  since now, not a running total)"`, `"  09:00-now    (unclosed)"`,
  `"Day total:     00h 00m (+ unclosed)"`, the full
  `receiving_date_suffix` string and the composed header line built
  from `date::format_date_with_weekday` rather than a hand-typed
  weekday), include explicit negative assertions
  (`!out.contains("(+ ongoing)")`, `!out.contains("spans to next
  day")`), use a deliberately implausible `duration_minutes: 4321` plus
  a per-line digit-leak check for the `(unclosed)` case, and assert the
  `StintLine`-level and `StatusView`-level `open_stint_age` agree across
  all three buckets plus a dedicated exactly-two-days-back boundary
  test. The `spans_to_next_day`-for-opens gap is covered twice, once at
  `stint_line()` level and once at `build_stint_lines()` level, which is
  the right split.

## 8. Verdict

**ship-with-followups.** The code is correct, well tested, ASCII-clean,
clippy/fmt-clean, matches every worked example in the locked spec when
run by hand, and honors both explicitly-contested design points (the
`stint.rs` edit is visibility-only; the two `classify()` calls are
fresh). Merge it. Then land F1 as a doc-only commit before release —
SPEC.md currently asserts the opposite of what ships — and fold F2/F3
in opportunistically.
