# Plan: overnight-visibility

> **Archived — historical planning record.** Not authoritative. Current spec: `docs/dev/SPEC.md`. Design/process history: `docs/dev/NOTES.md`.

Spec: `docs/dev/specs/2026-10-04-overnight-visibility.md` (locked). Spec review: `docs/dev/plans/reports/overnight-visibility-spec-review.md`. Decision log: NOTES.md entries 103-104.

**Altitude note**: scope of work only. No code, no pseudocode, no function, type or SQL names. Task plans choose the identifiers.

## Goal

A start left open from yesterday counts nothing after midnight, and today it shows only as the `[i]` hint on today's `status`. This changeset makes that overnight stint visible in two more places without changing any figure. Today's `status` lists it as a stint line ahead of today's own stints. `mlm week` marks yesterday's row with `[i]` and adds an `[i]` footer line, and on Mondays the current week's view names Sunday's stint in that footer slot. The glossary, SPEC and README are updated to match, and SPEC §1.2a and README's Possible issues section become "None currently."

## Architecture (prose)

### Current shape

- The `[i]` hint is built in `src/lookback.rs`. One ranged read feeds a 14-day scan, and a lone age-1 open start becomes the hint. Today's `status` drops the hint when a refused-join line already names yesterday.
- `src/status.rs` builds the stint list from the target date's own paired stints, sorted by start time. The overnight stint belongs to yesterday's bucket, so it is not in that list. `StintLine` is a small copyable struct, and its yesterday-age wording belongs to `status -1`, which must not change.
- `src/week_view.rs` has a pure renderer over `WeekView` and `WeekRow`, both built as struct literals in about a dozen tests. Rows are classified from the loader's buckets. The footer block holds the `[!]` footer, then the carry marker line.
- `src/render.rs` holds the `[!]` footer text. `src/cli.rs` holds the glossary that says `[i]` never marks a row.

### Target shape

1. **One trigger decision.** A single pure decision in `src/lookback.rs` answers "is there an overnight stint, and what is its start" from yesterday's and today's loaded buckets, `today` and `now`. It decides the refused-join suppression itself from those buckets, so no caller passes a flag. It encodes spec §3 once, and the hint's own entry condition calls it rather than re-deriving the same facts (plan review F3). `status` and `week` both call it, and a test pins that the hint and the new line appear together and disappear together.
2. **Today's `status` (decides F3).** The overnight line is a separate optional field on `StatusView`, not a member of the date-sorted stint list and not a new age variant of the existing stint line. Reasons:
   - The stint list is sorted by start, and a `22:00` start would sort after a `09:00` one.
   - Reusing the yesterday-age wording would change `status -1`.
   - A separate field keeps the list's "sorted ascending" doc comment true and keeps pairing and sorting untouched.

   The renderer prints it first inside the stint section, with the standard column layout. The section prints when either the field or the list is non-empty. `resolve` fills the field only for today's page, from the same buckets it already loaded, so no new read happens. The elapsed time is whole minutes from the start to `now`, through the standard formatter. Day total, EOD, week line and `has_open_stint` are not touched.
3. **`mlm week` (decides F5).** The pure renderer stays pure, and the view carries the facts:
   - Each row gains a plain "has overnight stint" flag, rendered after any `[!]` marker (`  [!] [i]` when both).
   - The view gains one optional field for the Monday case, holding the overnight stint's date. It is set only when the shown week contains today and the stint's date lies in the previous week.
   - `build_week_view` classifies yesterday from the loader's buckets with the shared decision. No new storage read is needed, because the loader already pads one day around the shown week and today's bucket is in range. The reviewer confirmed this (F5).
   - Row flag: set on the row whose date is yesterday, for whichever week is shown, including a past week by id. Future weeks and other past weeks get nothing.
   - The footer block prints, in order: `[!]` footer, `[i]` footer, carry marker line, blank line. The `[i]` slot holds the plain footer when any row shows `[i]`, or the Monday line when the optional field is set. The two never print together, because a marked row means yesterday is in the shown week and the Monday field means it is not. The blank line prints when any of the three lines printed.
   - The footer and Monday line depend only on the trigger, never on whether the row also shows `[!]` (answers F2). The shared-row case prints both footers, and the Monday line prints even when last week's Sunday row also carries `[!]`.
   - Footer text points at `mlm status` (F1), exactly as in spec §5. Footer strings live next to the existing `[!]` footer in `src/render.rs`.
4. **Untouched.** `has_any`, the `[!]` footer condition, the carry marker, pairing, the splice, the `[i]` hint text and conditions, accounting, and every figure.

## Global constraints

- Pre-commit gate: `cargo fmt` clean, cognitive complexity at most 15 per function (`clippy.toml`), coverage ratchet, and the hook now fails on test failures. Never use `--no-verify`.
- TDD: write the failing test first for each acceptance criterion.
- Output the spec does not change stays byte-identical. Existing golden and CLI tests must pass unedited, except these, which assert the old `[i]` rule and are rewritten to the new one: the lookback test near `src/lookback.rs:959`, `tests/week_target_cli.rs` near line 184 (`ends_with("It never marks a row.")`), and `tests/start_hint_cli.rs` `hint_does_not_mark_week_row_or_footer`. Struct-literal fixtures of `WeekRow`/`WeekView` (about five sites) and `StatusView` (`src/status.rs` near 555, 995, 1125, 1218, 1236, 1270) gain the new fields with their default values only.
- Spec §8's "`[!]` precedence on a shared row" is stale; §5's `  [!] [i]` rule governs.
- New literals pass the existing ASCII sweep (`tests/ascii_sweep.rs`).
- SPEC.md and README.md are self-contained. They never point to `docs/dev/plans` or `docs/dev/specs`.
- Tests that touch a database use a temp `MLM_DB_PATH` or an in-memory DB, never the real app-data path.
- No new storage reads on the `week` path. `status` gains no read beyond what it already does.
- Docs and comments avoid narrating discarded options.

## Ordering and parallelization

Two tasks. They own disjoint files and share only the pinned text below, so they can run in parallel worktrees. The coordinator merges either first. The docs task has no code dependency.

Three or more tasks are not needed. The shared trigger decision lives inside the code task, since only code consumes it.

## Interface contracts to pin before worktrees open

Both tasks must treat these as fixed.

1. **Today's stint line.** Exactly `  HH:MM-now` padded to the 11-column range layout, then `  (DURATION so far, started yesterday, not counted until stopped)`. Example from spec §4: `  22:00-now    (15h 30m so far, started yesterday, not counted until stopped)`. It is the first line of the stint section.
2. **Week row marker.** `  [i]` after the duration in the same column as `  [!]`, and `  [!] [i]` when both apply.
3. **Week footer.** `[i] open stint: run mlm status to stop it`.
4. **Monday footer.** `[i] open stint on Sun 2026-10-04 (last week): run mlm status to stop it`, weekday and date being the stint's, in the existing weekday-date display form.
5. **Footer order.** `[!]` footer, `[i]` footer or Monday line, carry marker line, blank line.
6. **Glossary wording.** The `[i]` glossary entry's last sentence changes from `It never marks a row.` to exactly `It marks yesterday's row in mlm week until that start is stopped.` (backticks around `mlm week` as the surrounding glossary uses them). Nothing else in the glossary changes. Pinned here so both tasks quote the same text and run in parallel.
7. **Moved behavior statements** for SPEC §7.1/§7.2 (from spec §6 and review F7): no live credit and no `est. EOD` for the stint, minutes return to yesterday on `stop`, the week line jumps at 00:00 because `required` steps up and the stint is not counted, and past Sunday midnight the closed week's `Worked` leaves the stint out until `stop`. Also that the stint is listed and marked as above, and that the line and marks disappear when the start turns two days old and the `[!]` stale handling takes over.

## Task 1: code

**Spec**: §3, §4, §5, §6 (glossary, test rewrite), §8.

Build the shared trigger decision, the separate optional overnight line on the status view and its rendering, the row flag, Monday field, footers and ordering on the week view, and the glossary change. Rewrite the existing lookback test that asserts the hint never marks yesterday's week row so it asserts the new rule. Add the §8 tests at unit level (renderers with hand-built views, byte-exact) and end to end (temp-DB CLI runs for the status page and week table).

**Acceptance criteria**

- Today's `status` with one open start yesterday, no punches today: the stint line appears under `Day total: 00h 00m` with no `est. EOD`.
- Same with punches today: line appears before today's own stints, and Day total, Worked, Fulfillment, week line and `est. EOD` are identical to the output without the line.
- Today also has its own open stint: both `-now` lines print, overnight first, and only the own stint counts.
- Start joined to today (splice) shows no line. Refused join naming yesterday shows no line. Two open starts yesterday show no line. Start two days old shows no line and the `[!]` stale lookback and row marker appear instead.
- Duration is correct and formatted by the standard formatter, including over 24h.
- `status DATE` for any date other than today is byte-identical to before, including `status -1`.
- The hint and the stint line appear and disappear together across all cases above, in one end-to-end `status` test.
- `mlm week` for the week containing yesterday, current or by WEEK_ID: yesterday's row ends with `  [i]`, the `[i]` footer prints once, and no other row is marked.
- Week row mark follows the same trigger: no `[i]` when the start is joined, when a refused join names yesterday, when yesterday has two or more open starts, or when the start is two days old.
- The Monday footer needs Sunday's facts even though Sunday is outside the current week's seven rows; the loader's buckets already hold that date.
- Row with both an anomaly and the overnight stint: `  [!] [i]`, both footers print, in order `[!]`, `[i]`, carry marker, blank line.
- Monday: current week's view has no marked row and prints the Monday footer naming Sunday. Last week's view marks the Sunday row and prints the plain footer. The Monday line still prints when Sunday's row also has `[!]`. Other past and future weeks show nothing.
- `has_any`, the `[!]` footer condition, the carry marker and carry accounting ignore `[i]`. A row with only `[i]` prints no `[!]` footer.
- The blank line before Carry-in appears exactly when any footer or carry marker line printed.
- `status --help` and `week --help` glossary golden updated to the new `[i]` wording.
- With no overnight stint, `status` and `week` output is byte-identical to before.
- Cognitive complexity of every touched function stays at 15 or below, so split render helpers rather than growing existing ones.

**Out of scope**: any figure change or live credit; the `[i]` hint text or conditions; pairing and the splice; changes to `status -1` wording; new storage reads; SPEC.md, README.md and NOTES.md edits.

**Owns exclusively**: `src/lookback.rs`, `src/status.rs`, `src/week_view.rs`, `src/render.rs`, `src/cli.rs`, and new or changed files under `tests/` (a new `tests/overnight_visibility_cli.rs` plus edits to any existing test file whose golden the glossary changes). `src/history.rs` and `src/anomalies.rs` are read only. If the task finds it must edit either, it reports back rather than editing.

## Task 2: docs

**Spec**: §6, and review F4, F6, F7.

Update SPEC.md and README.md to the new behavior. The task plan must relocate each passage by content, not by line number.

**Acceptance criteria**

- SPEC §1.2a keeps its heading, drops its last item and reads "None currently." Every reference to §1.2a elsewhere in SPEC stays true.
- SPEC §7.1 and §7.2 state the moved behavior of contract 7 as plain statements, and describe the new stint line, row marker, footers and Monday line with the exact texts of contracts 1-4.
- Every SPEC passage that says the `[i]` hint never marks a `week` row, or that `week` has no yesterday hint, is rewritten. This covers §3.5 (status contents), §7.1 (the hint is not an anomaly), §7.2 (`week` anomaly output list gains a fourth output), §7.3 (the `[i]` quiet marker), the §4.3.1/§7.1 yesterday hint bullet, and edge case E22.
- SPEC states that `[i]` never feeds `[!]`, the footer, the carry marker or `has_any`.
- README's Possible issues section keeps its heading and reads "None currently." README output examples and any `[i]` description follow spec §4 and §5.
- A grep of SPEC.md and README.md for `docs/dev/plans` and `docs/dev/specs` finds no reader-facing pointers added by this task.

**Out of scope**: any code or test change; the changeset spec and review files; archiving the working docs (coordinator close-out).

**Owns exclusively**: `docs/dev/SPEC.md`, `README.md`. NOTES.md stays coordinator-owned.

## Risks and notes

- **Glossary text.** Pinned in contract 6, so the tasks are independent. The coordinator checks the docs against the built binary at merge.
- **Trigger agreement.** Status builds the trigger from today's facts, and week builds it from yesterday's row facts. The review found them equal because refused-join detection is symmetric. The shared decision plus the hint-and-line-together test is the guard. A divergence found in testing goes back to the coordinator, not into a quiet fix.
- **Spec F2 reading.** The spec's section 5 says `[!] [i]` shows both, which resolves the review's F2. The plan follows the spec.
- **Duplicate wording** (U1, U2) is accepted by the spec, so the task does not harmonize the hint and status -1 wording.
- **Monday footer in a past-week view by id.** The spec limits it to the week containing today, so a past-week view never prints the Monday line. That view marks the Sunday row instead.
