# Plan: current-week-ahead-wording

> **Archived — historical planning record.** Not authoritative. Current spec: `docs/dev/SPEC.md`. Design/process history: `docs/dev/NOTES.md`.

Spec: `docs/dev/specs/2026-09-24-current-week-ahead-wording.md` (locked).

**Altitude note**: this document is scope of work only. No code, no
pseudocode, no function/type/SQL names beyond what the spec itself
already names (`week_headline`, `WeekFraming::Current`, `owed`,
`status_week_line`). If an edit here reads like an implementation,
that's a drafting error — the fix belongs one level up, in prose.

## Goal

The current-week headline in `status` and `week` always prints a
signed `owed` figure next to a weekday deadline, so once a week's
fulfillment passes its target the line reads a double negative like
`-04h 35m left by end of Saturday`, and the weekday deadline is
meaningless once the target is already met. Every sibling figure in
this output already switches wording on sign instead of printing a
raw negative (the day-total pace hint's `left to`/`over` split, the
closed-week headline's `Total behind`/`Total ahead` split) — the
current-week headline is the last one still doing it the old way.
After this changeset, a current week whose `owed` has gone negative
reads `{abs(owed)} ahead of target` instead, dropping the now-
meaningless weekday, while the `owed >= 0` form is untouched. No
accounting figure changes anywhere; this is presentation only.

## Architecture (prose)

One rendering function grows a second branch, and both call sites
that already flow through it pick up the new wording for free.

- **`src/render.rs`**: `week_headline`'s `WeekFraming::Current` arm,
  currently a single unconditional format string, splits on the sign
  of `owed_minutes`. `owed_minutes >= 0` keeps today's exact wording.
  `owed_minutes < 0` renders the new ahead form and drops the weekday
  entirely. `status_week_line` needs no change of its own — it
  already just wraps whatever `week_headline` returns, so `status`'s
  week line and `week`'s bare headline both pick up the new wording
  through this one call site each.
- **`README.md`** and **`docs/dev/SPEC.md`**: doc-only, describing the
  new branch and updating the worked examples that currently show the
  old signed form.

## Global constraints

- Every task's changes must pass the full verification set from
  `AGENTS.md`'s "Verifying changes" section before being considered
  done: `cargo fmt`, `cargo build`, `cargo test`, `cargo clippy
  --all-targets -- -D warnings`.
- The pre-commit hook in `.githooks` also gates the commit: cognitive
  complexity per function must stay under `clippy.toml`'s threshold
  (15), and overall line coverage (via `cargo llvm-cov`) must not drop
  below `coverage-baseline.json`. The code task's new branch is a
  simple sign check with no nested control flow, so it shouldn't
  approach the complexity threshold, but the task should re-run the
  full gate (not just the four cargo commands above) before treating
  its work as done, since the hook is what actually blocks the commit.
- No change to how `owed`, `fulfillment`, or `carry_in` are computed,
  anywhere, by either task — every fix is a string change over an
  already-correct figure.
- No change to the closed-week headline (`Total behind`/`Total
  ahead`), the `status` week line's fulfillment/target parenthetical,
  the day-total pace hint, or `est. EOD` — all four are explicitly out
  of scope per the spec's non-goals (§1.1) and must render byte-
  identical to today.
- The zero case (`owed_minutes == 0`) stays on the `left by end of
  <weekday>` side of the split, unchanged — it is not folded into the
  new ahead branch.

## Ordering and parallelization

Collapsed form: two tasks, no dependency graph, run in parallel. Task
1 (code) and Task 2 (docs) touch fully disjoint files, so both
worktrees can open at once. Task 2 must still hold its own strings
against the exact literal Task 1 emits (see the interface contract
below) rather than re-deriving them from the spec's prose, since the
spec's own prose and the shipped literal could in principle drift
apart during implementation.

## Interface contract to pin before opening worktrees

Both tasks must agree on this exact string shape up front, since Task
2 quotes it verbatim in docs and worked examples:

- The current-week ahead form is exactly `"{abs(owed)} ahead of
  target"`, where `{abs(owed)}` uses the same `format_minutes`
  convention already used everywhere else in this output (never a
  raw signed number, never a leading `-`). It replaces the entire
  `"{owed} left by end of {weekday}"` phrase — the weekday is dropped,
  not appended alongside the new wording.
- This form fires whenever `owed_minutes < 0`, and only then. The
  `owed_minutes == 0` case keeps today's `"00h 00m left by end of
  {weekday}"` wording unchanged. The `owed_minutes > 0` case is
  likewise unchanged.
- Both call sites inherit the same literal unmodified: `status`'s week
  line wraps it in the existing label and parenthetical exactly as it
  does today, and `week`'s headline prints it bare.

## Task 1 — code: `src/render.rs` + test modules of `src/status.rs` and `src/week_view.rs`

**Spec citation**: §3 (Behavior) and §4 (Tests).

**Files owned exclusively**: `src/render.rs`; the `#[cfg(test)]`
modules only of `src/status.rs` and `src/week_view.rs`. No non-test
code in either of those last two files changes — confirmed against
the current tree: `status_week_line` in `render.rs` already just
wraps `week_headline`'s return value, so neither `status.rs` nor
`week_view.rs` needs a production-code change to pick up the new
wording.

**Acceptance criteria**:

- `week_headline`'s `WeekFraming::Current` arm renders the unchanged
  `"{owed} left by end of {weekday}"` form for `owed_minutes >= 0`
  (covers both the positive and exact-zero case) and the new
  `"{abs(owed)} ahead of target"` form for `owed_minutes < 0`, per the
  interface contract above. The `WeekFraming::Closed` arm is byte-
  identical to today.
- `render.rs`'s existing test `t11_current_week_already_ahead_passes_
  sign_through_verbatim` (currently at line 341, pinning the old
  `-02h 30m left by end of Thursday`) is rewritten to expect `02h 30m
  ahead of target` and renamed to reflect the new behavior instead of
  the old sign-passthrough it no longer does.
- New `render.rs` coverage: a `status_week_line` case for the ahead
  form with the short (zero-carry-in) parenthetical, one with the
  expanded carry-in parenthetical where worked is `00h 00m` and a
  carry-in surplus alone puts the week ahead (spec §3), and a
  boundary case at
  `owed_minutes == -1` asserting `"00h 01m ahead of target"`. The
  existing zero-case test already covers `owed_minutes == 0` staying
  on the `left` side and needs no change beyond what the rename above
  already touches.
- One new pipeline test in `status.rs`'s test module driving
  `resolve()` then `render()` for a current week whose fulfillment
  exceeds target, asserting the rendered week line reads `… ahead of
  target (fulfillment …`.
- One new pipeline test in `week_view.rs`'s test module driving
  `build_week_view()` then `render_week()` for the same shape,
  asserting the rendered headline reads `… ahead of target`.
- Before considering the task done, re-grep `src/` and `tests/` for
  `left by end of` and for negative-left literals, rather than
  trusting this list of known sites — the spec explicitly calls this
  out as its own check, not just this plan's.
- Full verification set passes (`cargo fmt`, `cargo build`, `cargo
  test`, `cargo clippy --all-targets -- -D warnings`), and the
  pre-commit hook's complexity and coverage gates pass.

**Out of scope**:

- Any change to how `owed`, `fulfillment`, or `carry_in` are computed.
- Any change to the `WeekFraming::Closed` arm, the `status` week
  line's fulfillment/target parenthetical, the day-total pace hint, or
  `est. EOD`.
- Any doc file (`README.md`, `docs/dev/SPEC.md`) — Task 2's scope
  exclusively.
- The §1.2 known-issue about future `DATE`s naming today's weekday —
  unaffected, since the ahead form simply doesn't surface it (no
  weekday printed on that path).

## Task 2 — docs: `README.md` + `docs/dev/SPEC.md`

**Spec citation**: §5 (Docs).

**Files owned exclusively**: `README.md`, `docs/dev/SPEC.md`.

**Acceptance criteria**:

- `README.md`: the three occurrences of `-04h 35m left by end of
  Saturday`, confirmed still at lines 193, 213, and 261, become `04h
  35m ahead of target`. Line 290's `00h 25m left by end of Saturday`
  (a positive-`owed` example) is confirmed unaffected and stays as-is.
  Re-grep `left by end of` in `README.md` before finishing rather than
  trusting this line list, since Task 1 may land first and nothing
  blocks other unrelated edits from shifting these lines.
- `docs/dev/SPEC.md` §7.1's week-line bullet (confirmed still at line
  676, the `"<owed>" left by end of "<weekday>"` sentence) gains the
  `owed < 0` ahead form alongside the existing `left by end of` form,
  stating the sign rule and the reason the weekday drops. It also
  states that on an ahead week the day-total pace hint above may name
  a weekday (`required by end of Thursday`) while the week line below
  it names none, and that this is intended, per the spec's §1.2
  second bullet.
- `docs/dev/SPEC.md` §7.1's day-total pace-hint bullet (around lines
  654-657) justifies `required today` on Friday-Sunday by saying the
  week line above already names the same weekday. That premise is
  false on an ahead week. Reword it so it holds in both cases: the
  week line names that weekday while the week is still owed, and
  names none once it is ahead.
- `docs/dev/SPEC.md` §7.2's headline bullet (confirmed still at line
  781, "Headline is the same 'owed' figure...") refers back to §7.1
  for the same sign split rather than re-stating it.
- `docs/dev/SPEC.md` §5's negative-`owed` paragraph (confirmed still
  at line 503, "already-ahead-of-pace") gains a sentence stating how
  the headline renders that case (`X ahead of target`).
- `docs/dev/SPEC.md` §1.2a gains the zero-boundary known issue: a
  current week exactly at target reads `00h 00m left by end of
  <weekday>`, but the same week once closed reads `Total ahead: 00h
  00m` — both predate this changeset and are unchanged by it.
- `README.md`'s "Known issues" section (currently at line 42) gains
  the same zero-boundary bullet, worded to match.
- Every string quoted in either doc file matches Task 1's actual
  output byte for byte. Task 1 runs in parallel, so quote the strings
  pinned in this plan's interface contract section, not the spec's
  prose paraphrase. The merge gate checks them against the built
  binary.

**Out of scope**:

- Any change to `src/render.rs`, `src/status.rs`, or `src/week_view.rs`
  — Task 1's scope exclusively.
- Any change to the `Total behind`/`Total ahead` closed-week wording,
  the fulfillment/target parenthetical, the day-total pace hint, or
  `est. EOD` documentation — none of those change under this spec.
- Any new worked example beyond updating the existing ones named
  above — this changeset doesn't need a fresh example, only a wording
  fix to ones that already exist.
</content>
