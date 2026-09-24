# Adversarial review — `current-week-ahead-wording-plan.md`

Reviewed against: `docs/dev/plans/current-week-ahead-wording-plan.md`,
`docs/dev/specs/2026-09-24-current-week-ahead-wording.md`,
`src/render.rs`, `src/status.rs`, `src/week_view.rs`, `README.md`,
`docs/dev/SPEC.md`, `docs/dev/NOTES.md`, `clippy.toml`,
`.githooks/pre-commit`, `tests/*.rs`, `CHANGELOG.md`.

## Findings

### 1. [Moderate] Internal inconsistency the plan's docs task doesn't fix — `docs/dev/SPEC.md:654-657`

**Claim (plan)**: Task 2's acceptance criteria for `docs/dev/SPEC.md`
lists exactly five edits: the §7.1 week-line bullet (line 676), the
§7.2 headline bullet (line 781, by reference), the §5 negative-`owed`
paragraph (line 503), and a new §1.2a known-issue bullet. No other
SPEC.md text is named as needing a change.

**Evidence**: `docs/dev/SPEC.md:654-657` (the day-total pace hint
bullet, same §7.1 subsection) currently reads: "On Friday, Saturday,
and Sunday ... this trailing phrase drops the weekday name entirely
and reads `required today` ... since the week line directly above
already names the same weekday and, on those three days, the same
figure." That sentence gives a *reason* for the pace hint's Fri/Sat/
Sun behavior: the week line above always names a weekday too, so
repeating it would be redundant. Once the ahead-of-target branch
ships, that premise becomes false on any ahead week — the week line
above it names no weekday at all. The plan's new known-issue bullet
(Task 2, week-line bullet edit) documents the asymmetry as an
accepted consequence, but nothing in the plan touches this earlier
sentence, so SPEC.md will ship with a stated justification that
contradicts the new behavior it's describing the flip side of. Same
subsection, same page, two bullets apart.

This is genuinely easy to miss because the plan's "Interaction with
the project spec" section (spec doc, bottom) and the plan's own
acceptance criteria both enumerate SPEC.md edits by line number and
this line isn't one of them.

### 2. [Low] Testing gap — carry-in-only ahead case isn't locked into Task 1's acceptance criteria

**Claim (spec)**: §3 of the design spec states as an explicit behavior
note: "A carry-in surplus can put a week ahead before any work is
logged. The ahead form applies the same way, since it keys only on
`owed`." This is called out as a named behavior worth stating.

**Evidence**: Task 1's acceptance criteria (plan,
"New `render.rs` coverage" bullet) asks for an ahead-form
`status_week_line` case with "the expanded carry-in parenthetical,"
but doesn't specify that case's `worked` value. An implementer
satisfying the letter of the criteria can pick any nonzero-carry-in
ahead example (e.g. the README's own worked example, which has
nonzero `worked`) and never exercise the specific "worked = 0,
ahead only because of carry-in" shape the spec's prose flags as
notable. Since the code path is a single sign check with no separate
branch for this case, the risk is more about the spec's called-out
behavior having zero test evidence than about a plausible bug.

### 3. Undeclared user-visible gap — none found beyond Finding 1

Checked specifically for a consequence of the wording decision that a
user would see and that the document doesn't record. The one real
candidate is Finding 1 above, but that's a document-internal
contradiction rather than a runtime consequence the doc fails to
mention — the actual runtime consequence (pace hint may show a
weekday while the week line doesn't) *is* declared, in the design
spec's §1.2 second bullet and in the SPEC.md §7.1 edit the plan
requires. I looked for other candidates and didn't find one:
- The `status`/`week` fulfillment/target parenthetical stays signed
  and unchanged on ahead weeks (explicit non-goal, verified still
  true against `render.rs:86-106` — `status_week_line`'s parenthetical
  construction doesn't touch `week_headline`'s branch).
- `week`'s per-day rows and Carry-in/Worked/Fulfillment/Target block
  are untouched (verified: `week_headline`'s return value is only
  consumed as the headline string, `week_view.rs:252`, nothing else
  downstream reads its content to decide row rendering).
- A future-dated `status DATE` within an ahead current week can't hit
  the §1.2a known-issue (wrong weekday named) because the ahead form
  never prints a weekday — declared explicitly in the plan's Task 1
  "Out of scope" list and in the design spec's non-goals.

## Verification notes (claims checked and confirmed accurate)

- File-ownership claim: `status_week_line` (`render.rs:76-114`) and
  `week_line` (`status.rs:300`) and `week_view.rs:252`'s
  `render::week_headline` call are the only three places wrapping
  `week_headline`/`status_week_line`; none needs a non-test code
  change. Confirmed via grep — no other caller exists in `src/`.
- Every `#[cfg(test)]` boundary check: `status.rs`'s test module
  starts at line 551, `week_view.rs`'s at line 281 — the "left by end
  of" hits the plan doesn't route through Task 1 (`status.rs:1173,
  1306, 1586`; `week_view.rs:330,367,1070`) are all inside those test
  modules and all carry positive-`owed` fixtures, so they're both
  correctly in-scope-as-test-code and correctly unaffected by this
  change.
- No `tests/*.rs`, `examples/`, or `CHANGELOG.md` file emits or
  asserts `left by end of` with a negative sign — confirmed via
  repo-wide grep. `CHANGELOG.md` isn't touched by the plan, which
  matches the project's established process (NOTES.md entry 72: the
  changelog is hand-curated at release time, not during
  implementation).
- Line citations all check out exactly: README.md 193/213/227/261/290
  and "Known issues" heading at 42; SPEC.md 503, 676, 781. Re-grepped
  independently rather than trusting the plan's numbers.
- Pipeline test buildability: `status.rs` has `resolve()` (line 426,
  takes injected `now: DateTime<Local>`) and `render()` (line 257),
  plus existing fixtures (`test_db()`, `now_thu_1800()`) and a
  precedent end-to-end test
  (`resolve_end_to_end_nonzero_carry_in_shows_worked_and_carry_in_breakdown`,
  line ~1949) to model from. `week_view.rs` has `build_week_view()`
  (line 233), `render_week()` (line 72), `new_test_db()`, `fixed_now()`
  (pinned to Thu 2026-02-12, matching the spec's reference "today"),
  and `seed_week()` — all reusable to construct a current-week,
  fulfillment-exceeds-target fixture without any new test
  infrastructure. Default week target (`DEFAULT_WEEK_TARGET_MINUTES =
  2400`, `week.rs:18`) means such a fixture doesn't even require a
  `set_week_target` call. Both pipeline tests the plan asks for are
  buildable with existing fixtures, as claimed.
- `clippy.toml`'s `cognitive-complexity-threshold = 15` confirmed;
  `.githooks/pre-commit` confirmed to gate on `cargo fmt --check`,
  the same clippy lint, and `coverage-baseline.json` comparison,
  degrading to SKIP when tooling (`jq`, `cargo-llvm-cov`) is missing —
  matches the plan's "Global constraints" section description.
- Interface contract's exact literal (`"{abs(owed)} ahead of
  target"`, weekday dropped entirely, fires only for `owed_minutes <
  0`) is consistent between the plan and the spec, and consistent
  with the existing `Closed` arm's `format_minutes(-owed_minutes)`
  pattern already at `render.rs:70` — no new formatting helper needed,
  supporting the plan's complexity-risk assessment.

## Verdict

ship-with-followups
