# Plan: point-in-time-week

> **Archived — historical planning record.** Not authoritative. Current spec: `docs/dev/SPEC.md`. Design/process history: `docs/dev/NOTES.md`.

Spec: `docs/dev/specs/2026-10-03-point-in-time-week.md` (locked).

**Altitude note**: this document is scope of work only. No code, no
pseudocode, no function, type, or SQL names beyond what the spec itself
already names (`status::resolve`, `DbWeekData`, `owed`, `required`,
`gap`, `live`, `est. EOD`). If an edit here reads like an
implementation, that is a drafting error and the fix belongs one level
up, in prose.

**Split decision**: two tasks, not more. The code stays in one task
because the three code areas (shared accounting and headline, `status`,
`mlm week`) feed each other. `status` and `mlm week` both consume the
new live-inclusive accounting and the new headline, so a parallel split
would need a third task to own the shared helpers and would then run
serially behind it anyway. The docs task runs in parallel with the code
task.

## Goal

The current week's headline in `status` and `mlm week` shows the whole
week's remainder (`target − fulfillment`) phrased as due by end of
today, while the Day total line's pace hint shows what is actually due
today. After this changeset the current week has one point-in-time
figure. The week line shows `required − fulfillment`, where `required`
is the slice of the target due by end of today. Fulfillment, worked and
today's day total all count the running stint's live minutes. The Day
total line drops its pace clause and keeps `est. EOD`. `mlm week` for
the current week shows the same headline and the same live-inclusive
figures as `status`. Past weeks, stored data, pairing, and `est. EOD`'s
formula do not change. Fri–Sun `required` becomes the full target, which
moves `est. EOD` and `target already met` up to 4 minutes later for
targets that are not a multiple of 5 minutes.

## Architecture (prose)

Live minutes enter the week accounting before carry is walked. Today's
live figure is the whole minutes of the most recent open start dated
today, clamped at 0, and 0 when nothing is open today. It is computed
from today's punches whatever date `status` shows, so `status DATE` for
an earlier date in the current week classifies today separately from
`DATE`. The current week's worked minutes then equal its closed-stint
minutes plus live. Two places build that accounting today, the ledger
in `src/status.rs` and `DbWeekData` in `src/week_view.rs`. Both must add
live to the current week's worked minutes only, so a future week's
carry-in reflects the current week's live-inclusive fulfillment. Past
weeks never receive live. `owed` stays the stored quantity behind carry
and the closed-week `Total behind` / `Total ahead`. It is no longer
printed for the current week.

`required` has one definition, shared by `status` and `mlm week`:
`floor(target / 5) × weekday` for Mon–Thu, and the full target for
Fri–Sun, always by today's weekday. It is computed for every `status`
of the current week, not only when `DATE` is today. The week headline
logic in `src/render.rs` takes the gap and required inputs for the
current week and renders `X left to / over <required> by end of
<Weekday>` on Mon–Thu and `X left to / over <target> target` on
Fri–Sun. Closed weeks keep their current headline. The `status` week
line and the `mlm week` headline both go through that one headline
logic, so the two commands cannot drift.

On the `status` Day total line, today's total includes live and the
flag reads `(incl. ongoing)` for an open start today, including a start
typed later than now (live 0). The pace clause is removed everywhere,
including the Fri–Sun `required today` form. Earlier-date flags
`(+ ongoing)` and stale `(+ unclosed)` are unchanged. `est. EOD` keeps
its formula (open start plus required minus fulfillment without live)
and its `target already met` rule at `gap <= 0`, and the daily-target
data it needs stays available to the view even though the pace clause no
longer prints.

In `mlm week`, today's row total includes live, its marker reads
`(incl. ongoing)`, and `Worked:` and `Fulfillment:` show the
live-inclusive figures, so rows still sum to `Worked:`. `Carry-in:`,
`Target:`, past weeks, and the footer are unchanged.

Doc comments that state the old rule (worked excludes live, the day
total, row minutes) move to the new rule in the same edit. The
`WeekData` contract in `src/week.rs` says implementations report
completed stints only. That rule stays true of storage and carry for
past weeks, and the comment is reworded to say live enters only for the
current week.

## Global constraints

- Every task passes the verification set from `AGENTS.md` before it
  counts as done: `cargo fmt`, `cargo build`, `cargo test`,
  `cargo clippy --all-targets -- -D warnings`, plus the two
  `MLM_DB_PATH` smoke runs of `start` and `status`.
- The pre-commit hook also gates the commit. Cognitive complexity per
  function stays at or under the `clippy.toml` threshold (15), and
  overall line coverage via `cargo llvm-cov` must not fall below
  `coverage-baseline.json`. `status::resolve` is already busy, so
  Task 1 re-runs the full gate and extracts the live and required steps
  rather than growing it.
- TDD. Tests are written first, seen failing for the right reason, then
  the code makes them pass. Existing pins of the old strings in
  `src/status.rs`, `src/week_view.rs`, and `src/render.rs` are updated
  to the new output, not deleted.
- No change to past-week output, stored data, pairing (§4.3), the
  splice (§4.3.1), stint lines, anomaly markers, the header, notes, the
  `week` footer, `est. EOD`'s formula and day-suffix rule, or storage
  read-error handling in `mlm week`. Nothing live-inclusive is written
  to the database.
- No credit for an open start dated before today, no credit for a start
  two or more days old, and no summing of multiple open starts.
- No pointers from SPEC, README, or `--help` to the spec file
  (AGENTS.md Documentation rule). Docs are self-contained.
- NOTES.md and the spec file are coordinator-owned. No task edits them.

## Ordering and parallelization

Task 1 (code and tests) and Task 2 (docs) own disjoint files and run in
parallel in separate worktrees. Task 2 quotes the literals pinned below
and never waits on Task 1's output. The coordinator checks the quoted
strings against the built binary at merge.

## Interface contracts to pin before opening worktrees

All contracts below are in plain terms. The spec's §3.4 samples are the
byte-exact reference for both tasks.

1. **Current-week headline, Mon–Thu.** `X left to <required> by end of
   <Weekday>` when `gap >= 0`, and `X over <required> by end of
   <Weekday>` when `gap < 0`, with `X = |gap|` and `gap = required −
   fulfillment`. Exactly zero reads `00h 00m left to …`.
2. **Current-week headline, Fri–Sun.** `X left to <target> target` and
   `X over <target> target`.
3. **Status week line.** `Week <id>:  <headline> (<parenthetical>)`,
   with the parenthetical shape unchanged and its figures
   live-inclusive. It prints for the current week whether or not `DATE`
   is today.
4. **Day total line.** `(incl. ongoing)` for an open start today
   (live may be 0). `(+ ongoing)` and `(+ unclosed)` unchanged on
   earlier dates. No pace clause.
5. **`mlm week`.** Headline equal to contract 1/2, on its own line.
   Today's row marker `(incl. ongoing)`. `Worked:` and `Fulfillment:`
   live-inclusive.
6. **`required`.** Mon–Thu `floor(target / 5) × weekday`, Fri–Sun the
   target, by today's weekday.
7. **Live.** As defined in Architecture, taken from today's punches,
   added to the current week only, before carry is walked.
8. **Reference samples.** The spec's §3.4 Thursday `status` and
   `mlm week` blocks and the Friday week line, byte for byte.
9. **Known-issue wording** (the substance of spec §5 and §6, in the
   docs' own words) that Task 2 carries into SPEC §1.2a and README.

## Task 1 — code and tests: everything under `src/`

**Spec citation**: §2 (definitions), §3 (output), §4 (code doc
comments), §7 (testing).

**Paragraph**: Make the current week's accounting live-inclusive in both
the `status` ledger and `DbWeekData`, with live taken from today's
punches independent of `DATE`. Give `required` its single shared
definition with the Fri–Sun rule. Change the current-week headline and
week-line logic in `src/render.rs` to the point-in-time form, and route
the `mlm week` headline through it. Make `status` print `(incl.
ongoing)`, include live in today's total, and drop the pace clause while
keeping `est. EOD`. Make `mlm week` rows, `Worked:`, and `Fulfillment:`
live-inclusive with the new marker. Update the three doc comments named
in spec §4. Start with failing tests, then change the code. Keep
`status::resolve` under the complexity threshold by moving steps out of
it.

**Acceptance criteria** (tests write from these):

- `status` today, Mon–Thu and Fri–Sun: week line reads `left to` and
  `over` forms of contracts 1 and 2, with `gap` exactly 0, +1, and −1
  minute.
- `status DATE` for an earlier date in the current week with a live
  stint today: the week line counts live and uses today's weekday and
  `required`, and the Day total for `DATE` has no live and keeps its
  `(+ ongoing)` flag.
- Fri–Sun `required` equals the target for a target that is not a
  multiple of 5 minutes (for example 2011). `est. EOD` and `target
  already met` move accordingly.
- Zero-target week: `00h 00m left to 00h 00m by end of Monday` with
  nothing worked, and `02h 00m over 00h 00m by end of Monday` after two
  hours.
- Day total: `(incl. ongoing)` with live included, `(incl. ongoing)`
  with a future-dated start and live 0, `(+ ongoing)` on yesterday's
  page, and `(+ unclosed)` for a stale start, all unchanged where the
  spec says unchanged.
- No pace clause anywhere in `status` output, including Fri–Sun.
- `est. EOD` value and `target already met` threshold match the
  pre-change rule on Mon–Thu, and the §3.4 sample reads `est. EOD 21:25`.
- `mlm week` current week: headline text equals the `status` headline,
  today's row includes live with `(incl. ongoing)`, and the rows sum to
  `Worked:`. `Fulfillment:` equals `Carry-in:` plus `Worked:`.
- A future week shown by `status DATE` or `mlm week WEEK_ID` takes its
  `Carry-in` from the current week's live-inclusive fulfillment.
- Past week output is byte-identical to before in both commands.
- The §3.4 worked example (both blocks and the Friday line) is pinned at
  the resolve and render level with a fixed clock.
- Two open starts today credit only the most recent. An open start from
  yesterday and one two or more days old earn no live.
- Doc comments in `src/week.rs`, `src/stint.rs`, and `src/week_view.rs`
  state the new rule.
- Full verification set, complexity (≤ 15), and coverage ratchet pass.

**Out of scope**:

- `README.md`, `docs/dev/SPEC.md`, NOTES.md, and the spec file.
- Past-week behavior, carry or `owed` semantics, stored data, pairing,
  splice, and `est. EOD`'s formula or day-suffix rule.
- Storage read-error handling in `mlm week`.
- Any new flag, command, or `--short` style output.
- Fixes for the possible issues the spec lists as left open (§6).

**Files owned exclusively**: `src/status.rs`, `src/week_view.rs`,
`src/render.rs`, `src/week.rs`, `src/stint.rs` (production and test
code in each). If a grep for the old strings finds pins in `tests/` or
`examples/`, Task 1 updates them too and reports it. If implementation
needs a file outside this list, it stops and reports so the coordinator
can re-scope.

## Task 2 — docs: `docs/dev/SPEC.md` and `README.md`

**Spec citation**: §4 (SPEC.md and README.md lines), §5, §6.

**Paragraph**: Rewrite every SPEC and README passage that describes the
old week figure, the pace hint, `owed` on the current week, `ahead of
target`, the Day total flag, or the `mlm week` current-week output so
the docs state the new behavior. Use the samples and literals in the
interface contracts. SPEC sections touched are §1.2a, §2.4, §3.5, §3.6,
§5, §7.1, and §7.2. README gets its `status` and `week` samples
(including the Saturday sample), the prose on the pace hint, `owed`,
`ahead of target`, and `(+ ongoing)`, and its possible-issues list.
Each doc stands alone with no pointer to the spec file. Re-locate each
passage by content, not line number, and re-grep both files for `pace`,
`owed`, `ahead of target`, `left by end`, `ongoing`, `est. EOD`,
`required today`, and `live` before finishing.

**Acceptance criteria**:

- SPEC §1.2a no longer carries the five sentences the spec's §5 lists,
  and the surviving pace-hint entries are reworded to name the week
  line. Entries that share a bullet with removed sentences stay intact.
  The zero-boundary entry reads as the spec's §4 describes.
- SPEC §1.2a and README possible-issues lists carry every item of the
  spec's §6.
- SPEC §2.4 states the Fri–Sun `required` rule, that fulfillment and
  worked count live for the current week, and that the figure is the
  week-line figure.
- SPEC §3.5 and §3.6 describe the week figure as the point-in-time gap
  and no longer treat the pace hint as a separate item.
- SPEC §5 says `owed` drives carry and closed-week totals and the
  current week shows `gap`, and the `X ahead of target` closing note is
  rewritten.
- SPEC §7.1 has the new week-line and Day total rules and the §3.4
  sample. The pace-clause bullet, Fri–Sun `required today` wording,
  `ahead of target` form, and the notes contrasting the day line with
  the week line are gone. The rationale for keeping live out of the day
  total is replaced by the point-in-time rule.
- SPEC §7.2 has the new current-week sample, the headline rule, and the
  `(incl. ongoing)` marker.
- README samples match the interface contracts byte for byte, with the
  Saturday sample's figures checked by arithmetic and noted in the
  task report.
- No edit to NOTES.md or the spec file, and no reference to either from
  the two doc files.

**Out of scope**:

- Any change under `src/`, `tests/`, or `examples/`.
- NOTES.md and the spec file.
- New worked examples beyond updating the existing ones.
- Documenting behavior outside this changeset.

**Files owned exclusively**: `docs/dev/SPEC.md`, `README.md`.

## Review folds

From `docs/dev/plans/reports/point-in-time-week-plan-review.md`
(ship-with-followups, 13 items). These override anything above that
they contradict. The report holds the full line references.

- **F1 (Task 2)**: also sweep SPEC §8.1 (the F7, F8, F9, F9b, F10 and
  F11 entries) and §4.2 for the old rule. F9b's Sat/Sun
  `5 × floor(target/5)` becomes the spec's Fri–Sun `required = target`.
- **F2 (Task 1)**: the complexity risk is not `status::resolve`. Watch
  `build_ledger` in `src/status.rs`, where live enters the ledger, and
  leave `stint::classify` (already at 15) untouched or extract from it
  if it must change.
- **F3 (Task 1)**: update every doc comment that states the old rule,
  not only the three spec §4 names. The report lists them in
  `src/stint.rs`, `src/week.rs`, `src/status.rs`, `src/render.rs` and
  `src/week_view.rs`.
- **F4 (Task 1)**: some existing tests invert under the new rule
  rather than only changing strings (the 2010/2011 `required` test,
  the `live_eod_*` and `fix_c_*` tests in `src/status.rs`, and
  `week_view` b3 and b8). Rewrite each to assert the spec's new
  behavior. Never delete one to get green.
- **F5**: the `(+ ongoing)` acceptance bullet applies only when `DATE`
  itself has a non-stale open start, in practice yesterday.
- **F6 (Task 1)**: the `mlm week` data source needs today's local date
  to find live. Rows and `Worked:` must come from the same
  live-inclusive figures, so the rows still sum to `Worked:`. Read-error
  handling stays as it is (spec §1.1).
- **F7 (Task 1)**: classify today's punches for `live` the same way
  `status` for today does, splice-aware, so `status DATE` and `status`
  print the same week line.
- **F8**: the extra acceptance bullets stay. They follow from the spec.
- **F9**: the NOTES.md entry superseding entry 37 is coordinator work.
  `coverage-baseline.json` is ratcheted by the pre-commit hook in
  whichever task commits code, which is Task 1.
- **U1–U4**: added to spec §6 as possible issues; no behavior change.
