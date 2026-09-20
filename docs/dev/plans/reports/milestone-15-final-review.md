# Final review — milestone-15 "carry-inclusive required-by-day pace hint"

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.


Scope: `git diff 03b98b2..HEAD` (baseline v0.3.1 → merged HEAD), commits
`7d40cdc` (status: carry-inclusive required-by-day pace hint) and
`a1d94cf` (docs: spec milestone-15) under merge `790d985`. Full
`src/status.rs` read as merged (1201 lines), not just diff hunks.
`cargo test` (403+2+1+2+5 tests, all green) and
`cargo clippy --all-targets -- -D warnings` (clean, 0 warnings) run
directly, not taken on trust from prior reports.

## Findings

### 1. [Medium] README.md's `status` example still shows the pre-milestone-15 wording/numbers

- Claim: this changeset's entire point is that `status`'s pace-hint
  line changes from `"X left to <daily target> daily target"` to
  `"X left to <required> required by end of <weekday>"`, and SPEC.md's
  worked example was updated accordingly (SPEC.md:509, confirmed
  earlier adversarial review's finding #1 folded — now reads
  `est. EOD 20:45`, consistent with the stated `now + (required −
  fulfillment)` formula).
- Evidence: README.md:185 still reads exactly the old, pre-change
  form: `` Day total:     07h 25m (+ ongoing), 00h 35m left to 08h 00m daily target, est. EOD 23:39 ``.
  `git diff 03b98b2..HEAD -- README.md` is empty — this file was not
  touched at all by the changeset, despite carrying a near-duplicate
  of the exact SPEC.md §7.1 sample this changeset rewrote (same
  stint list shape, same "Day total ... left to ... daily target,
  est. EOD" phrase). README.md:196 also still narrates it as "The
  daily-target pace hint", the retired name.
- Impact: a user reading README (the more likely first stop than
  SPEC.md) sees a phrase (`"daily target"`) and format that no longer
  exist in the shipped binary's output — this is exactly the kind of
  doc drift the review brief asked to hunt for, and it's real, not
  hypothetical (verified `grep` shows no other README occurrence was
  updated either).
- Not a functional defect — `src/status.rs` and SPEC.md are correct
  and internally consistent with each other and with the code. This
  is documentation-only drift outside the diff's touched-files list
  (`git diff --stat` confirms README.md wasn't part of this
  changeset), which is exactly the kind of thing a narrower,
  diff-scoped pass wouldn't catch.

### 2. [Low] NOTES.md: new decisions 52-54 land in front of the pre-existing decision 51, worsening an already-odd ordering

- Claim (implicit in the file's own structure): NOTES.md decisions are
  meant to read in ascending numeric order.
- Evidence: decision 51 ("Multi-week-format stretch") already sat
  *after* the `## Open questions` heading in the pre-changeset baseline
  (`git show 03b98b2:docs/dev/NOTES.md` lines 313-320: `50.` → `##
  Open questions` heading → `51.`) — a pre-existing quirk, not
  introduced here. This changeset (`a1d94cf`) inserts the new `##
  More decisions (round 9...)` section with `52.`/`53.`/`54.`
  *before* that same `## Open questions` heading (NOTES.md:312-348),
  so the file now reads `50, 52, 53, 54, [Open questions heading],
  51` — the changeset had the opportunity to also relocate the
  orphaned `51.` next to `50.` while editing this exact seam, and
  didn't, leaving a numerically worse jumble than before (previously
  just one out-of-place entry after a heading; now three more
  numbers inserted in front of it).
- Cosmetic only — no ambiguity about which decision is which, and NOTES.md
  is dev-internal, not shipped. Flagging because the task asked for a
  genuinely broader pass and this is a real, verifiable file-ordering
  defect adjacent to the exact lines this changeset edited.

## Things checked and found clean (worth recording, not re-litigating)

- **Formula correctness**: `required_minutes = daily_target_minutes(acct.target) * weekday_number.min(5)`,
  `gap_minutes = required_minutes - acct.fulfillment` (src/status.rs:347-350)
  matches SPEC.md §2.4/§7.1 exactly. Hand-verified all three new F9b
  tests' expected numbers against `week::week_series`'s actual
  carry-chain arithmetic (src/week.rs:157-196, no clamping anywhere):
  - `resolve_f9b_large_carry_in_negative_pace_on_day_one`
    (src/status.rs:1116-1144): week 6 (target 0, worked 2000) →
    `carry_out` 2000 → week 7's `fulfillment` = 2000 + 0 = 2000 on
    Monday morning; `required` = 480×min(1,5) = 480; `gap` =
    480−2000 = −1520 = `-25h 20m`. Confirmed correct by independent
    recomputation, not just re-reading the test's own comments.
  - `resolve_f9b_saturday_pin_multiple_of_five_target`
    (src/status.rs:1146-1164): default target 2400, Saturday →
    `weekday_number.min(5)` = 5, `required` = 480×5 = 2400. Correct.
  - `resolve_f9b_sunday_pin_non_multiple_of_five_target_override`
    (src/status.rs:1166-1187): target override 2011 →
    `daily_target_minutes` = `2011.div_euclid(5)` = 402 → `required` =
    402×5 = 2010 (`33h 30m`), NOT 2011/2012. This is precisely the
    case the prior adversarial spec review (finding #2) flagged as
    missing and untested before this pass — confirmed now present
    and numerically correct, i.e. that finding is genuinely folded,
    not just claimed folded.
- **Test quality**: none of the new/changed assertions are
  formula-ambiguous — e.g. the Sunday-override test explicitly
  `assert_ne!`s against the naive (wrong) answer 2011 in addition to
  asserting the correct 2010, which specifically guards against the
  "silently round up to the override" bug the prior review worried
  about. The large-carry-in test's expected day-total (`0`) and
  pace-hint gap (`-1520`) could not both hold under the old
  (pre-milestone-15) formula (old formula would compare `480` against
  `day_total_minutes` = `0`, giving `+480`, not `-1520`), so this test
  would fail against the previous implementation — it's not
  coincidentally-passable.
- **Regression scope**: `git diff --stat 03b98b2..HEAD` for the code
  portion (`7d40cdc`) touches only `src/status.rs` and
  `coverage-baseline.json` (the latter a routine coverage-percentage
  bump, 98.866%→98.880%, consistent with 3 added tests). `src/week.rs`,
  `src/render.rs`, `src/main.rs`, `src/week_target.rs` are untouched —
  the week-level `owed`/`mlm week` output, `status_week_line`, and
  every other command are provably unaffected by this diff, not just
  unmentioned.
- **Merge cleanliness**: `790d985` is a clean union of its two parents
  (`a1d94cf` docs-only, `7d40cdc` code-only) touching disjoint file
  sets — `git diff 03b98b2..HEAD --stat` is exactly the sum of the two
  branches' own diffs with no overlapping hunks, so there is no
  merge-conflict-resolution artifact to review.
- **Cross-cutting concerns**: required-by-day is computed only inside
  `resolve()`'s `is_today` branch, downstream of `acct.fulfillment`,
  which is itself produced by the pre-existing, already-tested
  `build_ledger`/`week::week_accounting` path that already correctly
  folds in backdated punches (`storage::punches_for_date` reads by
  date, not entry order) and mid-week target overrides
  (`week_target::get_week_target` per visited week). Milestone-14's
  delete-note/delete-punch feature only removes rows from the same
  tables this path already reads generically; nothing in this diff
  assumes punches are never deleted. No new interaction risk found.
- **CHANGELOG.md**: no entry added by this changeset, but that's
  consistent with existing process — `CHANGELOG.md` is regenerated by
  `git-cliff` (`cliff.toml`) inside `scripts/prep_release.sh` from
  conventional commit messages at release time, not hand-edited per
  milestone (confirmed: `CHANGELOG.md` changes only in past `Release
  vX.Y.Z` commits, e.g. `03b98b2`, `2572b27`, never in a milestone
  commit like this one's `7d40cdc`/`a1d94cf`). Not a gap.

## Verdict

**ship-with-followups** — the implementation, tests, and SPEC.md text
are correct and the prior adversarial review's two findings are
genuinely folded (verified by hand, not by trusting the report). The
only defects found are documentation drift: README.md's `status`
example needs the same wording/number update SPEC.md already got
(finding 1), and NOTES.md's decision numbering could be tidied while
someone's next in that file (finding 2, cosmetic). Neither blocks
release; both are cheap one-file text fixes suitable for a fast
follow-up commit before or shortly after the next release cut.
