# Review: overnight-visibility plan

Plan: `docs/dev/plans/overnight-visibility-plan.md`. Spec (source of truth): `docs/dev/specs/2026-10-04-overnight-visibility.md`. Spec review: `docs/dev/plans/reports/overnight-visibility-spec-review.md`.

## Findings

### F1 (high) Docs task owns NOTES.md

- Claim: plan:113 "Owns exclusively: `docs/dev/SPEC.md`, `README.md`, `docs/dev/NOTES.md`", plus the criterion at plan:109 that the task writes the NOTES decision entry.
- Evidence: the dispatch rule says NOTES.md is coordinator-owned and must not be a task file. NOTES.md:16 and entries 103-104 (NOTES.md:1437, :1451) are the coordinator's decision log. The plan also says at plan:109 "the coordinator's close-out may add the merge entry", so two writers would touch one file.
- Fix: drop NOTES.md from plan:113 and plan:109. Move the F3/F5 decision entry to the coordinator.

### F2 (medium) Docs task is not really parallel, and the glossary text is not pinned

- Claim: plan:51 "The docs task has no code dependency" and "can run in parallel". Contract 6 (plan:64) says the code task owns the exact glossary text and the docs task "reads it from the code task's branch before its final pass".
- Evidence: that is a dependency, and plan:117 half-admits it. The text is also not pinned as a literal. Contract 6 gives two sentences, but the current glossary (`src/cli.rs:44-48`) is one paragraph that ends "It never marks a row.", and `tests/week_target_cli.rs:184` asserts `ends_with("It never marks a row.")`. The new paragraph's final sentence and its wording against the `[!]` sentence are open.
- Fix: write the full replacement `[i]` glossary text into the plan as a contract, and have the code task copy it verbatim. Then both tasks are truly independent.

### F3 (medium) The shared trigger takes the refused-join flag from the caller

- Claim: plan:22 the pure decision takes "the loaded buckets, `today`, `now` and whether a refused join names yesterday". It "encodes spec §3 once".
- Evidence: the two callers would compute that flag from different facts. Status uses today's `facts.refused_join_with_previous` (`src/status.rs:451-455`). Week would use yesterday's `refused_join_with_next` (`src/anomalies.rs:350-355`). Both call `stint::is_refused_join` (`src/anomalies.rs:470`), which is `pub(crate)` (`src/stint.rs:394`), so the decision can call it itself. A caller-supplied flag moves the §3 refused-join clause back into two call sites, which is the duplication the shared decision exists to remove. The plan also does not say the hint's own entry (`src/lookback.rs:215-249`, built from the separate read at `:200`) must call the decision. Today there are two §3 implementations (hint, and the new decision). The plan only asks that they "agree", and tests it for status only (F4).
- Fix: the decision derives the refused join from the raw yesterday/today buckets, with no flag parameter. Say the hint's age-1 entry condition is routed through it, or that a test fails on divergence.

### F4 (medium) Week-side trigger agreement has no acceptance criterion

- Claim: spec §3 "Both changes apply exactly when today's `status` prints the `[i]` hint". Plan risk (plan:118) names the guard as "the hint-and-line-together test".
- Evidence: plan:81 pins that test for `status` only. The week criteria (plan:82-85) cover marked row, both markers, Monday, other weeks. They never list the negative triggers for `week`: joined to today, refused join, two open starts yesterday, start two days old. Those are the cases where status and week could diverge, and plan:118 names that divergence as the main risk.
- Fix: add a criterion that the week row mark and footer are absent in those four cases, or one test asserting `status` shows the hint iff `week` marks the row across the case table.

### F5 (medium) Existing tests the change invalidates are only partly named

- Claim: plan:43 "Existing golden and CLI tests must pass unedited, except the tests the spec names for rewrite". Plan:73 names the lookback test.
- Evidence:
  - `tests/week_target_cli.rs:184` asserts the glossary ends "It never marks a row.". It needs editing. The plan covers it only through "any existing test file whose golden the glossary changes" (plan:93), and the "unedited" rule at plan:43 conflicts with it.
  - `tests/start_hint_cli.rs` `hint_does_not_mark_week_row_or_footer` (around line 194) is named for the old rule. It still passes only because it asserts `[!]` and "needs a look" are absent. It is a stale test that asserts the opposite intent, and it is not listed for rewrite or rename.
  - `src/lookback.rs:959-968` is named in the spec and plan, correctly.
- Fix: list those two files as expected edits in plan:43 and plan:93.

### F6 (low) Fixture-literal impact is understated and misattributed

- Claim: plan:18 `WeekView`/`WeekRow` "built as struct literals in about a dozen tests".
- Evidence: literal sites are `src/week_view.rs:262, 286, 312, 416, 438` (five), plus `build_rows_from` at `:174` and `build_week_view` at `:211`. The plan omits the other struct that gains a field. `StatusView` is built as a literal at `src/status.rs:466, 555, 995, 1125, 1218, 1236, 1270`, and each needs the new overnight field.
- Fix: say that both views' test fixtures need touching, and that `base_view()` (`src/status.rs:554`) is the place to default the new field.

### F7 (low) Spec §8 still says "`[!]` precedence on a shared row"

- Claim: plan:35 and plan:119 follow spec §5 (`[!] [i]`, both footers).
- Evidence: spec:148 lists the test as "`[!]` precedence on a shared row", which contradicts spec §5 (spec:70-71). The spec review's F2 asked for the choice to be made. The plan chooses correctly, but does not tell the coordinator that spec §8 is stale text, and the code task, reading §8, could write a precedence test.
- Fix: add a line to the code task that §8's "precedence" means the §5 both-markers rule.

### F8 (low) Monday-line facts come from a date outside the rows

- Claim: plan:33 "`build_week_view` classifies yesterday from the loader's buckets with the shared decision."
- Evidence: this is feasible with no new read. `load_history` pads one day past each end (`src/history.rs:108-117`), and the walk starts at the earliest data week (`:101-106`), so Sunday's previous, own and next buckets are present on Monday. `build_rows_from` (`src/week_view.rs:162-183`) only builds facts for the seven row dates, so the Monday path needs its own trio and facts for a date outside `week.dates()`. The plan does not say so. It also needs `today` and `now`, and `build_week_view` has both (`src/week_view.rs:194-200`), so that part is fine. Fix: state that the Monday path builds its own trio.

### F9 (low) New literals have no ASCII check

- Evidence: `tests/ascii_sweep.rs` and `src/render.rs:585-648` (t33b) sweep the existing literals, and plan:41 does not add the new footers and stint line to them. The glossary has `glossary_is_ascii` (`tests/week_target_cli.rs`).
- Fix: add one criterion that the new strings are ASCII and covered by the sweep.

## Probe answers

- Every spec requirement §3-§8 has a criterion except the week-side negative triggers (F4). §6's SPEC "move" is split correctly between docs criteria 1-6. Spec §4 "Yesterday's own page keeps its line" lands as the `status -1` byte-identical criterion.
- File ownership: disjoint and complete for code, except F1 (NOTES.md) and the unnamed test edits (F5). `tests/start_hint_cli.rs`, `tests/week_target_cli.rs` and a new `tests/overnight_visibility_cli.rs` all sit under the "tests/" grant at plan:93. `history.rs` and `anomalies.rs` are not needed: `build_facts`, `classify_context` and `PunchBuckets::trio` are `pub`, and `is_refused_join` is `pub(crate)`.
- Trigger feasibility: yes. `status::resolve` already holds `history.buckets` (`src/status.rs:424-430`), and `week_view::build_week_view` holds the same type (`:205`). `classify_context` and `lookback` both use `stint::classify_at`, so open counts match (`src/anomalies.rs:317-319`, `src/lookback.rs:229-230`). Refused-join is symmetric, but see F3.
- `today` and `now` in the week view: yes (`src/week_view.rs:194-200`).
- Pinned literals versus spec: the stint line (spacing checked against `src/status.rs` `"  {:<11}  ("`), row marker, week footer, Monday footer and footer order are identical to spec §4-§5. The `format_date_with_weekday` form is right (`src/date.rs:175`).
- Pre-commit "fails on test failures" claim is true (`.githooks/pre-commit:109`).

## Undeclared user-visible gaps

Choices the plan makes whose visible consequence the plan and spec do not record. Candidates for known issues.

- G1. The overnight line prints first, so today's stint section is no longer ascending by clock time (`22:00-now` above `09:00-10:00`). Plan:28 and spec §4 say "first" but not that the list reads out of order.
- G2. With today's own open stint present, two `-now` lines print with different parentheticals (`(Xm, ongoing)` against `(..., started yesterday, not counted until stopped)`). Day total's `(incl. ongoing)` refers only to the second. Spec §7 covers the no-punches case, not this one.
- G3. The two week footers point to two different pages. `[!]` says `mlm status DATE` (`src/render.rs:134`) and `[i]` says `mlm status`. A shared row prints both lines with different advice.
- G4. A past week viewed by WEEK_ID changes with today's data. The Sunday row's `[i]` appears or disappears depending on whether Monday's first punch joins the start, so one historical `mlm week` command gives different output over the day.
- G5. With the separate-field design, `has_open_stint` stays false when only the overnight start exists, so Day total has no `(incl. ongoing)` or `(+ ongoing)` while a `-now` line prints (plan:28). The spec records the no-`est. EOD` effect but not this.
- G6. On a Monday the current-week view prints the `[i]` footer with no marked row, and last week's view marks the row. The Monday line says "(last week)" but the past-week-by-id view never prints it (plan:121). This is recorded in the plan, but not as a user-visible consequence in the spec.
- G7. Both-markers rows (`[!] [i]`) print two footers, so a legitimate overnight session on an anomalous day makes `week` output two lines longer.
- G8. The hint wording and the stint line wording stay different for the same stint, and `status -1` keeps older wording. Recorded as U1/U2 in the spec review and accepted at plan:120.

## Verdict

ship-with-followups

Fix F1 and F2 before worktrees open (both are ownership and contract edits). Fold F3-F5 into the plan or task plans. F6-F9 are task-plan details.
