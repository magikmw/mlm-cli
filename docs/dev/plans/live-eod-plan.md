# Plan: live-eod

Spec: `docs/dev/specs/2026-10-03-live-eod.md` (locked).

**Altitude note**: this document is scope of work only. No code, no
pseudocode, no function/type/SQL names beyond what the spec itself
already names (`status::resolve`, `ctx.day`, `ctx.day.open`,
`DailyTargetHint.gap_minutes`, `StatusView`). If an edit here reads
like an implementation, that is a drafting error and the fix belongs
one level up, in prose.

## Goal

`status` computes its pace hint and `est. EOD` from
`gap = required − fulfillment`, and fulfillment counts completed
stints only. While a stint is open, its live minutes never reach `gap`,
so the pace hint stays frozen and `est. EOD` slides later every minute
and cannot be reached until `stop`. After this changeset, today's
`status` subtracts the running stint's live minutes from `gap` and
projects `est. EOD` from the open stint's start, so the hint counts
down while you work and `est. EOD` holds still until a punch is added
or removed. This reverses SPEC §7.1's explicit exclusion of live
minutes for these two figures and brings it in line with §2.4. Nothing
else changes (the day total, week line, `owed`, carry-in, `required`,
and every printed string stay as they are).

## Architecture (prose)

One production change point, in `src/status.rs`. `status::resolve`
already derives the daily-target hint and the est. EOD state from a
single `gap`. It gains one input, `live`, taken from the most recent
open start in `ctx.day` (not `ctx.day_raw`), clamped at 0 and equal to
0 when no stint is open. Two derived figures follow from it.

- `DailyTargetHint.gap_minutes` holds the post-live gap. The renderer's
  existing `left to` / `over` split and the existing `gap > 0` /
  `gap <= 0` est. EOD split keep their thresholds, so no rendering
  branch changes. Only which branch fires at a given moment changes.
- The est. EOD instant becomes the open start plus
  `required − fulfillment`, instead of now plus the gap. For a start at
  or before now the two are equal, which is what makes the printed
  value stable. For a start later than now (spec §3.3) it projects from
  the start's own time. The `(tomorrow)` suffix rule is unchanged.

The doc comment on the est. EOD state at `src/status.rs:54` currently
says `now + gap`. It is part of Task 1's production edit.

Docs describe the new behavior, and the SPEC/README worked examples
that show the old figures are updated. NOTES.md and the spec file are
coordinator-owned.

## Global constraints

- Every task passes the verification set from `AGENTS.md` before it
  counts as done: `cargo fmt`, `cargo build`, `cargo test`,
  `cargo clippy --all-targets -- -D warnings`.
- The pre-commit hook also gates the commit. Cognitive complexity per
  function stays at or under the `clippy.toml` threshold (15), and
  overall line coverage via `cargo llvm-cov` must not fall below
  `coverage-baseline.json`. `resolve` is already a busy function, so
  Task 1 must re-run the full gate, not just the four cargo commands,
  and extract the live-minutes step if the threshold is approached.
- No change to the day total or its `(+ ongoing)` flag, the week line,
  `owed`, carry-in, `required`, the weekday-cap wording, pairing
  (§4.3), the splice (§4.3.1), past-date `status`, or `mlm week`.
  Live minutes never enter a stored or carried figure.
- No string changes. Every pace-hint and est. EOD literal is
  byte-identical to today.
- No credit for an open start dated before today, and no summing of
  multiple open starts (spec §3.2).
- No pointers from SPEC, README, or `--help` to the spec file
  (AGENTS.md Documentation rule). Docs are self-contained.
- NOTES.md and the spec file are coordinator-owned. No task edits them.

## Ordering and parallelization

Two tasks, run in parallel. Task 1 (code and tests) and Task 2 (docs)
own disjoint files, so both worktrees open at once. Task 2 quotes the
literals pinned below and never waits on Task 1's output. The
coordinator checks the quoted strings against the built binary at
merge. A decomposition into three or more tasks is not warranted, since
the production change sits in one file and the docs are prose edits.

## Interface contracts to pin before opening worktrees

All strings below already exist today. Only the figures and the moment
they print change.

1. **Pace hint.** `X left to <required>` when `gap >= 0`, and
   `X over <required>` when `gap < 0`, with `X = |gap|` and
   `gap = required − fulfillment − live`. Exactly zero reads
   `00h 00m left to`.
2. **Est. EOD, open stint, `gap > 0`.** `est. EOD HH:MM`, where the
   time is the open start plus `required − fulfillment`, with the
   ` (tomorrow)` suffix when it lands after today's midnight.
3. **Est. EOD, open stint, `gap <= 0`.** `target already met`.
4. **No open stint.** `live = 0`, pace hint equals baseline, and
   `est. EOD` is omitted.
5. **Which stint.** The most recent open start dated today
   (`ctx.day.open.last()`). `live` is whole minutes from that start to
   now, clamped at 0. Start later than now gives `live = 0` and an
   `est. EOD` projected from the start's own time. Earlier-dated open
   starts give `live = 0` and no `est. EOD`.
6. **Stability.** The printed `est. EOD` is the same on every read
   while the stint runs. Only a punch change moves it.
7. **Reference lines** (Thursday, completed `07h 25m`, open stint
   `17:45-now`, `required − fulfillment = 02h 45m`).
   - At 18:00, the SPEC §7.1 sample line reads exactly
     `Day total:     07h 25m (+ ongoing), 02h 30m left to 32h 00m required by end of Thursday, est. EOD 20:30`.
   - At 18:30 the same day reads `02h 00m left`, still `est. EOD 20:30`.
8. **README sample** (Saturday 2026-09-12, open stint
   `17:45-now (05h 19m, ongoing)`). The hint moves from
   `04h 35m over 40h 00m required today` to
   `09h 54m over 40h 00m required today`. The `target already met`
   clause and the week line (`04h 35m ahead of target (fulfillment
   44h 35m / target 40h 00m)`) are unchanged.
9. **Known-issue wording** that Task 2 must carry into SPEC §1.2a and
   README (the substance of spec §1.2, in the docs' own words).
   - Pace hint counts the running stint, the day total beside it does
     not.
   - Pace hint counts down while the week line's `owed` stays frozen
     until `stop`, so on Fri–Sun the two can disagree in direction
     (e.g. `00h 10m over 40h 00m required today` beside
     `Week …: 00h 20m left by end of Friday`).
   - Two `status` runs with no punch between them print different
     pace-hint figures, and the hint can flip from `left to` to `over`
     with `est. EOD` becoming `target already met` mid-stint.
   - Jumps move to backdated punches. A backdated `stop HH:MM` raises
     the hint by the minutes it cuts off, and a backdated `start HH:MM`
     lowers it by the minutes it adds.
   - Two or more open starts today credit only the most recent one.
   - An overnight stint still open after midnight gets no live credit
     and no `est. EOD`, and the hint jumps at 00:00.

## Task 1 — code and tests: `src/status.rs`

**Spec citation**: §2 (Terms), §3 (Behavior), §5 (Test plan).

**Paragraph**: Make `resolve` subtract the live minutes of the most
recent open start in `ctx.day` from the gap, store the post-live value
in `DailyTargetHint.gap_minutes`, and compute the est. EOD instant from
the open start. Update the est. EOD doc comment. Add the behavior tests
T1 to T10 through `status::resolve` with a fixed `now` and a real
connection. Update the hand-built `StatusView` fixtures (the §7.1
sample near `status.rs:1102` and `status.rs:1391`) to the 18:00 "after"
line in contract item 7, and re-check the existing `resolve`-level
`gap_minutes` assertions (near `status.rs:2005` and `status.rs:2105`)
against the post-live meaning. Those assertions are the likeliest to
need adjusting, since they encode `now + gap`.

**Acceptance criteria**:

- T1: open stint with `gap > 0` after live. The pace hint and
  `est. EOD` both use the reduced gap.
- T2: same punches, two `now`s 30 minutes apart, both with non-zero
  seconds. `est. EOD` identical, pace hint 30 minutes lower.
- T3: live tips the gap from positive to negative. The hint flips to
  `over` and est. EOD reads `target already met`.
- T4: live makes the gap exactly 0. The hint reads
  `00h 00m left to` and est. EOD reads `target already met`.
- T5: no open stint. The hint and `gap_minutes` equal
  `required − fulfillment`.
- T6: two open starts today at different times. `live` equals the later
  start's minutes.
- T7: the `(tomorrow)` suffix still fires when est. EOD crosses
  midnight.
- T8: yesterday-dated open start with no punches today. `live = 0`, no
  `est. EOD`, hint equals baseline.
- T9: open start later than now. Hint equals baseline, est. EOD equals
  that start plus `required − fulfillment`.
- T10: Friday with an open stint. `required today` wording holds and
  the gap includes live.
- Day total, week line, and every non-status command render
  byte-identical to before for the same inputs.
- Full verification set passes, plus the hook's complexity (≤ 15) and
  coverage-ratchet gates.

**Out of scope**:

- Any doc file (`README.md`, `docs/dev/SPEC.md`, NOTES, the spec).
- Changes to the day total, week line, `owed`, carry-in, `required`,
  pairing, splice, or `mlm week`.
- Any change to rendered strings.
- A fix for the known issues in spec §1.2.

**Files owned exclusively**: `src/status.rs` (production and test
code). The task adds tests only inside that file's existing test module
and touches no other source file. If implementation finds that
`gap_minutes` is consumed outside `src/status.rs`, the task stops and
reports it so the coordinator can re-scope.

## Task 2 — docs: `docs/dev/SPEC.md` and `README.md`

**Spec citation**: §4 (Docs), minus the NOTES line.

**Paragraph**: Rewrite the SPEC and README passages that describe the
pace hint and est. EOD so they state the new behavior, using the
literals in the interface contract. SPEC §2.4 names the pace hint
alongside est. EOD as the only places live minutes enter, still
display-only. SPEC §7.1 gets the new sample line, the `live` term in
the pace-hint bullet, and a replacement for the est. EOD bullet's
"Fulfillment here still excludes the open stint's live minutes" with
the open-start rule, the most-recent-start rule, and the
start-later-than-now rule. The day-total paragraph keeps its "not
folded in live" wording. SPEC §1.2a, §8.1 F9/F9b, and the README
feature bullet, known-issues bullet, sample, and pace-hint paragraph
get the edits listed below. Each doc stands alone with no pointer to
the spec file.

**Acceptance criteria**:

- SPEC §2.4 names the pace hint and est. EOD as the only places live
  minutes enter, and says both remain display-only.
- SPEC §7.1 sample line equals the 18:00 line in contract item 7. The
  pace-hint bullet states `gap = required − fulfillment − live`. The
  est. EOD bullet states the open-start rule, the most-recent-start
  rule for multiple open starts, and the later-than-now rule. The
  "Fulfillment here still excludes…" sentence is gone.
- SPEC §1.2a: the pace-hint vs day-total bullet gains the first four
  items of contract item 9, the yesterday's-open-session bullet gains
  the overnight item, and a two-open-starts item exists.
- SPEC §8.1 F9/F9b gain the stability property (same `est. EOD` at two
  different `now`s) and the over-by-live case.
- README feature bullet (near lines 13-14) says the "how much is left
  today" estimate counts the running stint. The pace-hint known-issues
  bullet (near 74-82) gains the week-line divergence. The sample (near
  230-235) shows `09h 54m over` per contract item 8. The pace-hint
  paragraph (near 267-272) says the hint and est. EOD count the running
  stint.
- Line numbers are approximate. Re-locate each passage by content
  before editing, and re-grep both files for `est. EOD`,
  `excludes the open`, and `04h 35m over` before finishing.
- Every quoted string matches the pinned literals byte for byte. The
  merge gate checks them against the built binary.
- No edit to NOTES.md or the spec file, and no reference to either from
  the two doc files.

**Out of scope**:

- Any change to `src/`, `tests/`, or `examples/`.
- NOTES.md (the coordinator records that this reverses §7.1's
  exclusion) and the spec file.
- New worked examples beyond updating the existing ones named above.
- Documentation of the week line, `mlm week`, or the day total beyond
  the divergence notes listed.

**Files owned exclusively**: `docs/dev/SPEC.md`, `README.md`.

## Review folds (phase 7)

From `docs/dev/plans/reports/live-eod-plan-review.md`. These override
anything above that disagrees.

- **Task 2, stale SPEC prose (F1).** Rewrite every `now + gap` /
  "keep going from right now" / "excludes the open stint's live
  minutes" passage in SPEC.md, including §7.1's `est. EOD` bullet
  (SPEC.md:692-699), the §2.4/§3 mentions (SPEC.md:243-249, 322-324)
  and the pace-hint bullet (SPEC.md:655). Re-grep SPEC.md and README.md
  for `EOD`, `live`, `ongoing`, `pace hint` and `left to` before
  finishing.
- **Task 1, stale comments (F2).** Update the `gap_minutes` doc comment
  (status.rs:66-71), the `EodState` doc (status.rs:50-56) and the test
  comments at status.rs:2093 and 2106 to the post-live meaning.
- **Task 1, existing assertions (F3).** The resolve-level assertions at
  status.rs:2006 and 2106 still pass post-live. Keep them, and re-check
  only that their comments are accurate.
- **Task 1, fixtures (F4).** Every hand-built fixture carrying the §7.1
  sample figures (`165` gap / `20:45`) moves to the "after" values
  (`150` / `20:30`): status.rs:1115, 1439, 1496, 1562 and any others a
  grep for `20:45` finds.
- **Contract item 9 vs Task 2 (F5).** README gets the week-line
  divergence and the `est. EOD` meaning change. SPEC §1.2a gets every
  spec §1.2 item.
- **Pinned literals (F6).** Task 1 adds one resolve-level test that
  rebuilds the §7.1 18:00 Thursday sample from punches and asserts the
  full "after" day-total line, so the pinned string is checked by a
  test, not the real-clock binary. The README Saturday figure is
  checked by arithmetic in Task 2's report.
- **Task 1, more tests (F7).** T6 also asserts `est. EOD` from the
  later start. Add a splice case where tomorrow's first punch is an
  orphan end that consumes today's open start, and assert `live = 0`
  with no `est. EOD`. Add a T9 variant with `required − fulfillment ≤ 0`
  (`target already met`). Backdated-punch jumps and the overnight case
  are known issues, not tested.
- **Known issues (U1-U5).** Folded into spec §1.2 (`est. EOD` meaning,
  early `(tomorrow)`, future start, magnitude gap, `mlm week` frozen).
  Task 2 copies them per the spec's §4.
