# Consistency report: open-punch-visibility task plans

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

Scope: cross-document drift between the changeset plan, the locked
spec, the six task plans, and the current source they describe. Not a
review of any single document's own correctness.

## Findings

### Finding 1 (medium) — Task 5 says Task 2 replaces `Anomalies` outright; Task 2 (and NOTES.md) say it's additive

- **Task 5**: `docs/dev/plans/open-punch-visibility-task-5-week-view.md:104`
  states, in its Dependencies table: `render::DateAnomalies` "Replaces
  the old `render::Anomalies` outright (Task 2 plan: 'replaced, not
  extended in place' — the old two-field struct no longer exists)."
  The same claim repeats at lines 123-125 ("the pre-Task-2 two-field
  type; Task 2 replaces it with `DateAnomalies`") and at lines 303-305
  ("`Anomalies` no longer exists (Task 2 replaced it outright with
  `DateAnomalies`)").
- **Task 2**: `docs/dev/plans/open-punch-visibility-task-2-render.md:79-109`
  ("Additive, not replacing") states the opposite explicitly: "Task 2
  instead adds `DateAnomalies`... *beside* the existing `Anomalies`
  struct... Nothing existing is deleted, renamed, or changed in
  behavior... `render::Anomalies` (both fields, both methods) stays
  exactly as it is today... Deleting the old `Anomalies` type is
  explicitly **Task 4's job**."
- **NOTES.md** (`docs/dev/NOTES.md`, entry 79) corroborates Task 2:
  "Task 2 is additive and keeps the old type working; Tasks 3 and 5
  switch to the new type; Task 4 deletes the old type once both have
  merged."

Task 5's own code changes are compatible either way (it stops
importing `Anomalies` and switches to `DateAnomalies`), so this does
not break the build. But Task 5's stated model of what Task 2 did is
factually wrong as written, and a reader of Task 5 alone would believe
`Anomalies` is already gone by the time Task 5 runs — which contradicts
Task 4's own plan (`open-punch-visibility-task-4-start-warning.md:303-332`),
which spends a whole section walking through deleting `Anomalies`
*because* it still exists after Task 2 and Task 3/5 land. Task 5's plan
should be corrected to match the additive model before use as an
implementation reference, even though its actual code diff needs no
change.

### Finding 2 (low) — Task 4 still flags a Task 3 naming mismatch that Task 3 has already resolved

- **Task 4**: `docs/dev/plans/open-punch-visibility-task-4-start-warning.md:142-152`
  states: "Task 3's plan re-exports `pub use crate::render::{LookbackFinding,
  LookbackFindingKind};` and calls a free function
  `render::lookback_stderr_line(&finding)`... This plan is written
  against Task 2's actual names... Task 3's plan has the same
  inconsistency to resolve on its own side." Repeated in the Risks
  section at lines 501-510 ("flagged, not fixed here... may already
  correct this by the time it merges").
- **Task 3**: `docs/dev/plans/open-punch-visibility-task-3-status.md:1-69`
  (the "Re-alignment note (phase 9)" at the top of the file) already
  corrects this: line 181 is `pub use crate::render::{LookbackFinding,
  LookbackKind};` (not `LookbackFindingKind`), and lines 213-216 call
  `finding.stderr_line()` as a method (not a free function), matching
  Task 2 exactly. The mismatch Task 4 describes no longer exists in
  Task 3's current text.

This is a stale caveat, not a live contradiction — Task 4's own design
already codes against the correct (Task 2-matching) names, so nothing
in Task 4 is actually wrong. But Task 4's text asserts a live
disagreement with Task 3 that Task 3 has since fixed, which would send
an implementer double-checking a non-issue, or worse, "fixing" Task
3's plan to match Task 4's stale description of it.

### Everything else checked and found consistent

- **`lookback` signature**: Task 3 (`open-punch-visibility-task-3-status.md:199-204`)
  pins `pub fn lookback(conn: &rusqlite::Connection, today: chrono::NaiveDate, now_utc: chrono::DateTime<chrono::Utc>) -> anyhow::Result<Vec<render::LookbackFinding>>`.
  Task 4 (`open-punch-visibility-task-4-start-warning.md:156, 248`)
  calls it with exactly that signature, including the `now_utc`
  parameter both plans agree is required for `classify_at`. Consistent.
- **`stint::StaleOpenStart` vs `render::StaleOpenStart`**: Task 1
  (`open-punch-visibility-task-1-stint-predicates.md:111-114`) pins
  `pub(crate) struct StaleOpenStart { pub at: DateTime<Utc>, pub age_days: i64 }`
  in `stint.rs`. Task 2 (`open-punch-visibility-task-2-render.md:208-216`)
  pins a distinctly-shaped `pub struct StaleOpenStart { pub time: NaiveTime, pub age_days: i64 }`
  in `render.rs`, and explicitly calls out the collision
  (`open-punch-visibility-task-2-render.md:111-145`, "Name collision
  with Task 1"). Task 3 (`open-punch-visibility-task-3-status.md:226-229, 311-315`)
  and Task 5 (`open-punch-visibility-task-5-week-view.md:102, 105, 160-163`)
  both convert one into the other correctly and both explicitly warn
  against importing either type unqualified. Consistent, deliberately
  and legibly handled everywhere it's used.
- **`is_refused_join` name and signature**: pinned once by Task 1
  (`open-punch-visibility-task-1-stint-predicates.md:129-133`,
  `pub(crate) fn is_refused_join(earlier: &DayStints, later: &DayStints, later_punches: &[Punch]) -> bool`)
  and referenced identically by Task 3
  (`open-punch-visibility-task-3-status.md:227, 316-319`) and Task 5
  (`open-punch-visibility-task-5-week-view.md:103, 165-176, 201-208`).
  No plan still calls it `refused_join`.
- **File ownership**: no two task plans claim the same file for
  overlapping work. `render.rs` is owned by Task 2 (additive) then
  sequentially by Task 4 (deletion-only, after Task 3 and Task 5
  merge) — both plans state this the same way
  (`open-punch-visibility-task-2-render.md:66-70`,
  `open-punch-visibility-task-4-start-warning.md:73-78, 303-361`).
  `tests/status_cli.rs` moved from Task 2 to Task 3 in both plans
  consistently (`open-punch-visibility-task-2-render.md:44-47, 553-582`
  and `open-punch-visibility-task-3-status.md:123-159`).
  `storage.rs`/`main.rs` are claimed by no task, and every task plan
  that mentions them agrees no task needs them.
- **Batch/dependency ordering**: the changeset plan's diagram
  (`open-punch-visibility-plan.md`, "Ordering and parallelization")
  puts Task 1 and Task 2 in parallel first, then Task 3 and Task 5 in
  parallel (both depending on 1 and 2), then Task 4 (depends on Task 3
  only), then Task 6. Every task plan's own "Scope"/"Dependencies"
  section agrees with this graph and, since Task 2 is additive, the
  build stays green at every merge point in between (explicitly argued
  in `open-punch-visibility-task-2-render.md:612-616` and reflected in
  Task 4's sequenced, non-blocking cleanup step). No task plan places
  itself before something it calls.
- **Contract B's refused-join predicate rule**: "true when the earlier
  date has at least one open start, the later date has at least one
  orphaned end, and the existing splice gate rejects the pair" appears
  verbatim in the changeset plan and is implemented exactly that way
  in Task 1's code sketch (`open-punch-visibility-task-1-stint-predicates.md:208-217`),
  and both Task 3 and Task 5 call it against raw (`classify()`, not
  `classify_at`-resolved) classifications as required
  (`open-punch-visibility-task-3-status.md:249-283, 326-329`;
  `open-punch-visibility-task-5-week-view.md:80-89, 143-146`).
- **Output strings against spec §3.4-§3.6**: Task 2's Published
  interface (`open-punch-visibility-task-2-render.md:287-365`) quotes
  every line shape (E7, stale, E8, refused join, both lookback block
  forms, both lookback stderr forms, the week footer) and each matches
  the spec's `2026-09-26-open-punch-visibility.md` §3.4/§3.5/§3.6
  blocks byte for byte, including the double-space-vs-single-space
  distinction between block and stderr forms and the exact 4-space/
  5-space `fix:`/`or:` indentation. Task 3, Task 4, Task 5, and Task 6
  all quote the same strings via method calls or literal reproduction
  rather than re-deriving new wording, so no task plan introduces a
  paraphrased or divergent variant.
- **Current-source citations spot-checked**: `splice_candidate`'s
  location/visibility (`src/stint.rs:334`, `pub(crate)`),
  `DayStints.open`'s "ascending by start instant" doc comment
  (`src/stint.rs:61-62`), `render::Anomalies`'s current location
  (`src/render.rs:123-168`), `status.rs::to_anomalies`'s current body
  (`src/status.rs:319-328`), the age-boundary logic at
  `src/status.rs:436-441`, and `tests/status_cli.rs:82` (current E7
  text, `"[!] 2 open stints for this date (unmatched starts)"`) all
  match what the task plans cite. No task plan states a fact about
  current source that current source contradicts.

## Verdict

ship-with-followups — both findings are fixable one-paragraph plan
edits (Task 5's Dependencies-table claim about `Anomalies`, Task 4's
stale caveat about Task 3), neither requires new design work, and
neither would change the actual code an implementer would write from
these plans, since the substantive signatures all already agree.
