# Plan: status-wording-fixes

Spec: `docs/dev/specs/2026-09-23-status-wording-fixes.md` (locked).

## Goal

Today, three `status`/`week` output strings mislead or repeat
themselves, and the docs don't cover the shapes users actually see.
After this changeset: a closed week that finished short of target
reads `"Total behind: Xh Ym"`, the direct pair of the existing `"Total
ahead"` phrasing, instead of the debt-toned `"Total still owed"`. An
`est. EOD HH:MM` estimate that lands on a later calendar day than
today carries a `(tomorrow)` suffix, so the clock time shown can't be
mistaken for landing today. On Friday, Saturday, and Sunday — the days
`required_minutes` is capped at the week's full target — the
day-total pace hint drops its redundant `required by end of <weekday>`
phrasing (which on those three days names the same day and figure as
the week line directly above it) in favor of `required today`. And
README gains closed-period worked examples for both `status` and
`week`, matching the shapes SPEC.md's own examples already show, so a
user hitting a closed date/week for the first time isn't surprised by
unfamiliar output. No accounting figures change anywhere — every fix
is presentation-only.

## Architecture (prose)

Two tasks, no new modules, no new files.

- **`src/render.rs`**: the `WeekFraming::Closed` branch of
  `week_headline` that currently formats `"Total still owed: ..."`
  changes its literal to `"Total behind: ..."`, mirroring the
  already-correct `"Total ahead: ..."` sibling branch. No signature or
  logic change.
- **`src/status.rs`**: three changes, all presentation-only over
  figures already computed correctly.
  - `EodState::At(NaiveTime)` gains a second field, a bool recording
    whether the estimate's calendar date is later than `today`.
    Compare the already-computed full `NaiveDateTime`'s `.date_naive()`
    against `today` at the existing construction site inside
    `resolve()`; carry the result through. `day_total_line`'s
    rendering of this variant grows a `(tomorrow)` branch.
  - `day_total_line` is a pure `&StatusView -> String` function — it
    never sees `today` or `weekday_number`, so the `weekday_number.min(5)
    == 5` fact can't be recomputed inside it. `DailyTargetHint` (which
    already carries `required_minutes`/`gap_minutes`) gains a third
    field, `day_reaches_week_cap: bool`, computed once in `resolve()`
    alongside the other two and threaded through unchanged — the same
    shape as Fix B's bool. `day_total_line`'s daily-target clause
    renders `required today` in place of `required by end of {weekday}`
    when that field is true (Friday, Saturday, and Sunday alike),
    unchanged otherwise.
  - Every existing test's `EodState::At(t)` construction becomes
    two-argument; new test fixtures cover a Friday and a weekend day
    for the day-total wording, alongside the existing Monday-through-
    Thursday case. One existing test already pins a Sunday and asserts
    the old wording via `render()` — it breaks under this fix and gets
    updated, not just supplemented.
  - The rename from `render.rs` also updates literal-string test
    assertions in `src/status.rs`, `src/week_view.rs`, and
    `src/commands.rs` — no production code in those last two files
    changes, only `#[cfg(test)]` expected strings.
- **`README.md`** and **`docs/dev/SPEC.md`**: doc-only. README gains
  one new worked example each for a closed `status` date and a closed
  `week`, output verbatim, placed next to the existing current-period
  examples. SPEC.md needs no new example (its §7.1/§7.2 closed-period
  examples already exist) — only its existing `Total still owed`
  occurrences, and the `est. EOD`/day-total prose describing the two
  `status.rs` fixes, get updated to match Task 1's actual output.
  Both files' "Known issues" sections drop the four bullets this
  changeset closes.

## Global constraints

- Every task's changes must pass the full verification set from
  `AGENTS.md`'s "Verifying changes" section before being considered
  done: `cargo fmt`, `cargo build`, `cargo test`,
  `cargo clippy --all-targets -- -D warnings`. Task 1 additionally
  needs the two `cargo run` smoke-check lines from that section to
  still behave sanely, since it touches `status`'s render path
  directly.
- No change to how `gap_minutes`, `required_minutes`, or
  `owed_minutes` are computed, anywhere, by either task — every fix is
  a string change over already-correct figures.
- No new `EodState`-style enum variant for the `(tomorrow)` marker — a
  bool alongside the existing `NaiveTime` is the full extent of the
  type change (spec §1.1).
- Date comparison for the `(tomorrow)` marker must use `.date_naive()`,
  never the deprecated `.date()` — the latter returns `Date<Local>`,
  won't type-compare against `today: NaiveDate`, and its deprecation
  lint fails the repo's `-D warnings` clippy gate outright.
- `day_reaches_week_cap` reuses `status.rs`'s existing
  `weekday_number.min(5)` result rather than introducing a second bare
  `5` literal for the work-week length, and is threaded as a new
  `DailyTargetHint` field rather than re-derived inside `day_total_line`
  (which has no access to `today`/`weekday_number` to re-derive it
  from).
- No padding added around `required today` to match the length of the
  phrase it replaces — resolved with the user in the spec; the line's
  visible length already varies day to day by weekday-name length, and
  this is one more instance of that, not a new kind of change.

## Ordering and parallelization

No hard file dependency — Task 1 (`src/render.rs`, `src/status.rs`)
and Task 2 (`README.md`, `docs/dev/SPEC.md`) touch disjoint files, and
the spec's §6 explicitly notes no interface contract between them.
But there is a soft ordering constraint worth respecting: Task 2's
closed-period README examples and its SPEC.md wording updates need to
quote Task 1's exact output strings correctly, so Task 2 should read
Task 1's task plan and completion report before writing its own docs,
rather than working from the spec's prose description alone. In
practice this means starting Task 1 first and opening Task 2's
worktree once Task 1's completion report exists, even though nothing
stops the two worktrees from existing concurrently.

## Interface contract to pin before opening worktrees

Both tasks must agree on these exact string shapes up front, since
Task 2 quotes them verbatim in docs and worked examples:

- The closed-week/closed-date deficit headline is exactly
  `"Total behind: {formatted minutes}"` — same `format_minutes`
  convention as the existing `"Total ahead: {formatted minutes}"`
  sibling, no other wording change to that line.
- The `est. EOD` line's tomorrow marker is exactly the literal suffix
  `" (tomorrow)"` appended after the existing `HH:MM` time, never a
  weekday name or a calendar date. It appears only when the estimate's
  date is later than `today`'s date, in the same variable-suffix slot
  `boundary-context-cues` already used for `(+ ongoing)` / `(+
  unclosed)` / `, spans to next day`.
- The day-total clause's capped-week wording is exactly
  `"required today"`, replacing the entire trailing
  `"required by end of {weekday}"` phrase (not appended alongside it).
  It fires when `weekday_number.min(5) == 5` — Friday, Saturday, *and*
  Sunday under the default 5-day work week, not a single day.

## Task 1 — code: `render.rs` + `status.rs`

**Spec citations**: §2 (Fix B), §3 (Fix C), §4 (Fix A).

**Files owned exclusively**: `src/render.rs`, `src/status.rs`,
`src/week_view.rs`, `src/commands.rs` (the latter two only for
literal-string test-assertion updates from Fix A's rename — no
production-code change in either).

**Acceptance criteria**:

- Fix A: `week_headline`'s `WeekFraming::Closed if owed_minutes > 0`
  branch renders `"Total behind: {}"` instead of
  `"Total still owed: {}"`. The `owed_minutes <= 0` sibling branch is
  byte-identical to today. Every test in `render.rs`, `status.rs`,
  `week_view.rs`, and `commands.rs` asserting the old literal is
  updated to the new one (re-run `grep -rn "Total still owed" src/`
  rather than trusting any fixed line-number list, since Task 2's own
  grep against SPEC.md will need the same discipline).
- Fix B: `EodState::At` becomes two-argument, `(NaiveTime, bool)`; the
  bool is true exactly when `(now + gap_minutes).date_naive() !=
  today`. `day_total_line` renders `, est. EOD {t}` when false and
  `, est. EOD {t} (tomorrow)` when true. Every existing
  `EodState::At(t)` construction across `status.rs`'s test module is
  updated to two-argument form — the known sites are `t1` (line 658),
  `t6a` (950), `t11` (1158), `t12` (1256), `t13` (1320); `base_view`
  sets `eod: None` and has no such construction. Re-run `grep -n
  "EodState::At(" src/status.rs` rather than trusting this list. A gap
  large enough to cross two midnights still renders only `(tomorrow)`,
  not a more precise marker — this is an accepted edge case, not a bug
  to route around.
- Fix C: `DailyTargetHint` gains `day_reaches_week_cap: bool`,
  computed in `resolve()` as `weekday_number.min(5) == 5` and threaded
  through unchanged. `day_total_line`'s daily-target clause renders
  `required by end of {weekday}` when that field is false and
  `required today` when true, with no other wording change to the
  clause and no padding on the shorter form. Coverage: a Friday case
  and one weekend case (Saturday or Sunday) each assert `required
  today` and the *same* `required_minutes` figure as the adjacent
  weekday case; a Monday-through-Thursday case (the existing Thursday
  fixture, `today` pinned at 2026-02-12) still renders `required by
  end of {weekday}` unchanged. One existing test,
  `resolve_f9b_sunday_pin_non_multiple_of_five_target_override`
  (`status.rs:1759-1780`), currently asserts `"33h 30m required by end
  of Sunday"` via `render()` — this breaks under this fix and updates
  to `"33h 30m required today"` as part of Fix C's own change, not as
  incidental fallout discovered later.
- Regression: every existing test not touched by the three fixes above
  passes unchanged.
- Full verification set from `AGENTS.md` passes: `cargo fmt`,
  `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D
  warnings`.

**Out of scope**:

- Any change to how `gap_minutes`, `required_minutes`, or
  `owed_minutes` are computed.
- A new `EodState` enum variant, or any date (as opposed to a bare
  `(tomorrow)` marker) in the EOD line.
- Any padding or alignment logic for the day-total clause.
- SPEC.md's bullets 1-4 of §1.2a (the hidden-state/undisclosed-rule
  issues) and README's matching "Known issues" bullets — those
  updates belong to Task 2, not Task 1, even though Task 1's own fixes
  are what make the corresponding bullets droppable.
- Any doc file (`README.md`, `docs/dev/SPEC.md`) — Task 2's scope
  exclusively.

## Task 2 — docs: `README.md` + `docs/dev/SPEC.md`

**Spec citations**: §4 (Fix A's docs footprint), §5 (Fix D), §6.

**Files owned exclusively**: `README.md`, `docs/dev/SPEC.md`.

**Acceptance criteria**:

- SPEC.md: all 8 occurrences of `Total still owed` update to
  `Total behind` — the two in §1.2a (deleted along with the rest of
  that bullet, see below), the two worked examples at §7.1
  (lines ~707-716) and §7.2 (lines ~744-766), and the four normative-
  prose occurrences (originally at lines 647, 672, 679, 776 — re-run
  `grep -n "Total still owed" docs/dev/SPEC.md` rather than trusting
  those line numbers, since Task 1 may land first and the file may
  have drifted, though Task 1 does not itself touch SPEC.md).
- SPEC.md §7.1 gains documentation of the `est. EOD ... (tomorrow)`
  case and the `required today` case (firing on Friday, Saturday, and
  Sunday — `weekday_number.min(5) == 5` — not a single weekday),
  matching Task 1's actual rendered output.
- SPEC.md's two existing closed-period worked examples (§7.1, §7.2)
  get their `Total still owed` string updated by the rename above; no
  new example is added to SPEC.md — a second closed-period example
  there would be redundant with what already exists.
- README.md gains one new worked example each for a closed `status`
  date and a closed `week`, placed next to the existing current-period
  `status` examples (currently ~lines 195, 215) and `week` examples
  (currently ~lines 233, 262). Both new examples show verbatim output:
  the `"Total behind: Xh Ym"` line and the absence of the
  fulfillment/target parenthetical, matching what the
  `WeekFraming::Closed` branch actually produces. SPEC.md's own
  closed-period examples are the reference source for the exact shape
  to reproduce, since they already render real output.
- README.md's existing Saturday `status` example (`README.md:196-198`)
  goes stale under Fix C — its output line reads `required by end of
  Saturday`, and Saturday is exactly Fix C's trigger condition, so the
  real post-fix output is `required today`. This is the only
  `required by end of` occurrence in the whole file, so nothing else
  would catch it; Task 2 corrects this line as part of Fix C's docs
  footprint. Separately, that same line currently concatenates `est.
  EOD target already met` — a phrase the renderer cannot produce
  (`day_total_line` emits one or the other, never both). This predates
  the changeset and isn't caused by any of its four fixes, but Task 2
  is already editing this line for the reason above, so it corrects
  this too: pick whichever single EOD phrase is consistent with the
  example's own numbers, never both concatenated.
- SPEC.md §1.2a and README's "Known issues" section both drop the four
  bullets this changeset closes (the debt-toned headline, the
  datestamp-free EOD estimate, the redundant day-total/week-line
  wording, and the missing closed-period README example), leaving the
  other four bullets from NOTES.md's separate triage group (splice-
  gate disclosure, anomaly remedy guidance, lone-unclosed-start
  ambiguity, multi-day-old forgotten stop) untouched, in both files.
- Every string quoted in either doc file matches Task 1's actual
  output byte for byte — verify against Task 1's completion report or
  the built binary, not against the spec's prose paraphrase of it.

**Out of scope**:

- Any new closed-period example in SPEC.md (only README gets a new
  example; SPEC.md's existing two get a string update).
- Any of `src/render.rs`, `src/status.rs`, `src/week_view.rs`,
  `src/commands.rs` — Task 1's scope exclusively.
- The four §1.2a/"Known issues" bullets this changeset does not close.

## Cross-cutting concerns

- Both tasks touch the same four-fix surface described in one spec;
  neither should re-derive wording independently. Task 2's strings
  must be copied from Task 1's actual output, not reconstructed from
  the spec's own prose, since the two could drift if Task 2 works from
  memory of the spec rather than from Task 1's report.
- The `grep -rn "Total still owed"` re-run (Task 1 over `src/`, Task 2
  over `docs/dev/SPEC.md`) is each task's own responsibility — neither
  should trust the spec's line-number citations, which are a snapshot,
  not a guarantee.
- Plain-ASCII output convention (SPEC.md §7, already established by
  prior changesets): `(tomorrow)`, `required today`, and `Total
  behind` are all already-ASCII literals, so no new ASCII-golden-test
  coverage is required beyond what Task 1's own regression tests
  already exercise.

## Open risks / ambiguities

- Task 1's exact line numbers for the `EodState::At` construction
  sites will shift once the bool field is added; the five known test
  names (`t1`, `t6a`, `t11`, `t12`, `t13`) are a floor, not a ceiling —
  Task 1 should search rather than rely on that enumeration being
  exhaustive.
- Task 2 depends on Task 1's completion report existing and being
  accurate. If Task 1's report doesn't quote the exact rendered
  strings (as opposed to describing them), Task 2 may need to read
  Task 1's actual test assertions or run the binary itself rather than
  trusting the report's prose.
- No hard mechanism enforces the soft ordering constraint (Task 2
  should start after Task 1's report exists) beyond coordinator
  discipline — nothing in the repo blocks Task 2's worktree from being
  opened early, so this depends on the coordinator sequencing the
  dispatch rather than any file-level gate.
