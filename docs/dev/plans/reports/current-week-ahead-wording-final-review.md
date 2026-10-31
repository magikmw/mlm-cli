# Final review: current-week-ahead-wording

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

**Scope**: full diff `v0.3.6..current-week-ahead-wording`. Spec:
`docs/dev/specs/2026-09-24-current-week-ahead-wording.md`. Plan:
`docs/dev/plans/current-week-ahead-wording-plan.md`.

## Verification performed

- `cargo fmt --check` — clean.
- `cargo build --release` — clean.
- `cargo test` — 454 + 2 + 1 + 2 + 5 tests, all pass.
- `cargo clippy --all-targets -- -D warnings` — clean.
- Manual CLI runs against a scratch DB
  (`/tmp/claude-.../scratchpad/mlm-review.db`), never the real
  app-data DB, covering all three `owed` regimes on both `status` and
  `week`:
  - `owed > 0` (target 40h, 3h worked): `status` → `Week 2026-39:
    37h 00m left by end of Friday (fulfillment 03h 00m / target
    40h 00m)`; `week` → bare `37h 00m left by end of Friday`.
  - `owed < 0` (target shrunk to 2h via `mlm week target 2h`, 3h
    worked): `status` → `Week 2026-39:  01h 00m ahead of target
    (fulfillment 03h 00m / target 02h 00m)`; `week` → bare `01h 00m
    ahead of target`. Matches the spec's worked example exactly
    (§3), scaled to the test fixture's numbers.
  - `owed == 0` boundary (target set to exactly 3h): both `status`
    and `week` still read `00h 00m left by end of Friday` — confirms
    the zero case stays on the "left" side per §1.2/§4 and the
    existing `render.rs:269`-family test, not folded into the new
    ahead branch.
  - `status <date>` for a non-today date inside the same ahead week
    (`mlm status 2026-09-23`) still renders the ahead form with no
    weekday, confirming the framing keys off *today's* week context,
    not the queried date (§7.1's existing F11 rule, untouched).
  - Ran `cargo run --example seed_test_data` for a richer randomized
    dataset and inspected closed-week headlines (`Total behind: …`)
    for wording consistency with the new current-week `ahead of
    target` form — both use the word "ahead"/"behind" consistently,
    per the spec's stated design goal of keeping "ahead" shared
    across the Current → Closed transition.

All manual output matches the spec's worked examples in §3 and the
README's updated examples byte for byte.

## Correctness

`src/render.rs`'s `week_headline` now splits the `WeekFraming::Current`
arm on sign via a match guard:

```rust
WeekFraming::Current if owed_minutes >= 0 => format!(
    "{} left by end of {}",
    format_minutes(owed_minutes),
    format_weekday_full(today)
),
WeekFraming::Current => {
    format!("{} ahead of target", format_minutes(-owed_minutes))
}
```

- `owed_minutes >= 0` (positive and exact zero) keeps the old form —
  correct per spec §1.2/§4.
- `owed_minutes < 0` negates to get the magnitude and drops the
  weekday — correct, and consistent with the existing `Closed` arm's
  identical `format_minutes(-owed_minutes)` idiom two lines below, so
  no new formatting convention was introduced.
- `status_week_line` needed no change (confirmed: it only wraps
  `week_headline`'s return value in the label + parenthetical), so
  both call sites (`status`'s week line, `week`'s bare headline) pick
  up the new wording for free, exactly as the plan predicted.
- No changes anywhere to how `owed`, `fulfillment`, or `carry_in` are
  computed — confirmed by diff: only `render.rs`'s formatting branch
  and test modules changed, `week.rs`/`stint.rs`/accounting untouched.

## Compliance with changeset spec and project SPEC.md

- §1.2a (project SPEC): gained the zero-boundary known issue exactly
  as specced, worded to note both framings predate this changeset.
- §5: negative-`owed` paragraph gained the one added sentence
  pointing at the headline's rendered form, referencing §7.1.
- §7.1: week-line bullet gained the `owed < 0` form and the
  ahead-week pace-hint/week-line divergence note. The plan-review's
  finding 1 (the pace-hint bullet's own justification for dropping the
  weekday on Fri-Sun assumed the week line above always names a
  weekday, false once ahead) was folded — the reworded bullet now
  holds in both cases.
- §7.2: refers back to §7.1 rather than re-stating, as specced.
- Changeset spec's own non-goals (§1.1) are respected: no change to
  `owed`/`fulfillment`/`carry_in` computation, the closed-week
  headline, the `status` week line's parenthetical, the day-total
  pace hint, or `est. EOD` — confirmed both by diff and by manual runs
  (parenthetical, pace hint, and closed-week forms all render
  unchanged in the transcripts above).

## Test strength

- `render.rs`'s new/rewritten unit tests (`t11`, `t11b`, `t11c`,
  `t11d`) all use `assert_eq!` against the full exact string, not
  `contains` — strong, would catch any wording drift including
  whitespace or punctuation changes.
- The two new pipeline tests are the ones worth scrutinizing for
  whether they'd actually fail on a regression:
  - `status.rs::resolve_current_week_ahead_of_target_renders_ahead_of_target`
    asserts `view.week_line.contains("ahead of target (fulfillment ")`
    on both the pre-render view and the rendered `out` string. Tying
    "ahead of target" to "(fulfillment " in one substring means a
    regression to the old `left by end of` form, or a regression that
    dropped the parenthetical, both fail this assertion. It would
    *not* catch a regression that kept "ahead of target (fulfillment "
    but reintroduced a weekday elsewhere in the string, but that's a
    narrow miss given the interface contract explicitly says the
    weekday is dropped entirely, not appended.
  - `week_view.rs::c10b_current_week_ahead_of_target_end_to_end` adds
    a second, sharper assertion: `!view.headline.contains("Thursday")`
    alongside `contains("ahead of target")`. That directly catches a
    regression that kept the weekday around. Good — this is the
    stronger of the two pipeline tests specifically because it checks
    a negative as well as a positive.
  - Both pipeline tests are exactly what spec §4 asked for (one in
    each module, driving the real `resolve()`/`build_week_view()` →
    `render()`/`render_week()` path, not just the pure function), so
    they cover the path a user actually hits, not just `render.rs`'s
    unit-level guarantees.
- Re-grepped `src/` and `tests/` and all doc files for `left by end
  of` / `ahead of target` (per spec's explicit re-grep instruction).
  No stale live occurrence remains outside of (a) unaffected
  positive-`owed` examples correctly left as-is, and (b) historical
  plan/spec files under `docs/dev/plans/` and `docs/dev/specs/`, which
  AGENTS.md's archiving convention permits to stay as-is.

## Docs prose accuracy

README's three occurrences of the old `-04h 35m left by end of
Saturday` (lines 193, 213, 261 pre-change) are now `04h 35m ahead of
target`, matching what the built binary actually emits for that
`owed`/`fulfillment`/`target` combination — verified structurally
against the manually-run ahead case above, which produces the same
`{X} ahead of target` shape. Line 290's unrelated positive-`owed`
example is untouched, correctly. SPEC.md's §1.2a/§5/§7.1/§7.2 edits
read accurately against the actual match-arm behavior; no doc claim
overstates or understates what the code does.

## Decision log / process hygiene

NOTES.md entries 73 and 74 record the spec lock and plan-review fold
respectively, both citing the exact findings folded and where. Both
prior review reports (`current-week-ahead-wording-spec-review.md`,
`current-week-ahead-wording-plan-review.md`) conclude
ship-with-followups with all findings folded into the spec/plan before
implementation — nothing outstanding carried into code.

## Code quality / comment style

The new match arm has no comment, which is fine — the branch is a
two-line, self-evident sign split right next to the doc comment above
the function that already states the "today's weekday, never DATE's"
contract. The one added inline comment (`t11d`'s "No work logged at
all; a carry-in surplus alone puts the week ahead of target.") is
present-state-oriented, states the scenario the test pins rather than
narrating alternatives considered, and matches the terse style of
surrounding tests. No historicizing, no leftover dead-path comments.

## Security

No new user input path. The change is a pure presentation branch over
an already-validated, already-computed `i64` (`owed_minutes`); nothing
here builds a shell-replayable, SQL, or otherwise injectable string
from user input. Not applicable.

## Findings

None. All three prior review passes (spec review, plan review, and
this final review) surfaced no unresolved issues; the two review
findings that did surface (pace-hint bullet's stale justification,
carry-in-surplus test fixture pinning) were folded before this branch
was cut, and are already reflected in the diff and NOTES entries 73/74
above.

## Verdict

ship
