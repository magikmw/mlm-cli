# possible-issues-triage: cross-document consistency review

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

Scope. Changeset plan (`docs/dev/plans/possible-issues-triage-plan.md`, "plan"), spec (`docs/dev/specs/2026-10-03-possible-issues-triage.md`, "spec"), task plans T1-T6 (`docs/dev/plans/possible-issues-triage-task-N-*.md`), current source at `cccfb8f` plus working-tree edits, NOTES.md entries 96-98. Looked for drift between documents only. Nothing was built or run.

## Findings

### F1. T2's overlap label for a spliced stint disagrees with how the stint list renders the same stint (medium)

- T2 clamps the start-side part of a spliced stint to the next local midnight and prints it as `00:00`: `docs/dev/plans/possible-issues-triage-task-2-overlap.md:127-132` (`closed_span`), `:175-182` (`label`), ambiguity A1 at `:433` ("for example `22:00-00:00`"). The receiving date gets `00:00-HH:MM` (`:134-142`).
- Existing stint lines print the real end of a spliced stint. `src/status.rs:213-231` formats `start-end` from the stint's own end, `:463` sets `spans_to_next_day`, and the test at `src/status.rs:1755-1762` pins `23:30-00:45` with `, spans to next day`. The receiving date lists no stint and shows only `(00:45 continues previous day's stint)` (`src/status.rs:1781-1790`).
- Effect. On the same page the stint list says `22:00-00:30` and the overlap line says `22:00-00:00`. On the receiving date the overlap line names a `00:00-00:30` stint that the page never lists. Spec §3.2.1 (`spec:117-118`) fixes the comparison range but gives no label, and no other task plan says how a spliced stint is labelled.
- Resolution. T2 plan: keep the clamped range for the overlap test, but label a spliced stint with its real start and end times (the `22:00-00:30` form the stint list uses). Then the A1 pin and `label` change. If `00:00` is kept, record the label rule in T6's §7.3 overlap block.

### F2. T6's merge-time literal grep cannot match what T1-T3 tests contain (medium)

- T6 section 1.7 expects `grep -rnF '<literal>' src tests` to hit for: `[i] start on Fri 2026-10-02 at 18:02 still open (yesterday; not counted until stopped)`, `    fix: mlm stop HH:MM --date 2026-10-02  (a time after 18:02)`, `[!] 2 open stints on Fri 2026-10-02 (yesterday)`, `warning: start on Fri 2026-10-02 at 18:02 ...`, and the overlap line "and its `fix:` line": `docs/dev/plans/possible-issues-triage-task-6-docs.md:957-970`.
- T3's unit fixtures use `Wed 2026-02-11` (`task-3-yesterday-hint.md:405-409`, `:436`, `:455-456`). Its integration tests compute the date from the clock (`:395`, `:414`). T2's overlap `fix:` line is asserted with `--date 2026-09-01` (`task-2-overlap.md:367`), while the spec and T6 print `--date D` (`spec:124`).
- Effect. Several of the 1.7 greps return nothing even when code and docs agree, so the check either fails spuriously or gets waved through.
- Resolution. T6 plan 1.7: say date-bearing literals are compared by shape with the date substituted, as it already does for the windowed examples (`task-6-docs.md:972`). Alternatively T3 adds one unit test that uses the spec's own fixture (2026-10-02 is a Friday) so the literal exists in a test.

### F3. Contract 3 says `mlm start` gets "structured findings". T3 returns a new wrapper type and gives a reason T1 never states (low)

- Plan contract 3, `plan:113`: "`mlm start`: structured findings for the stderr warnings".
- T3 returns `Vec<LookbackEntry>` (a `LookbackFinding` plus `LineFacts`) and keeps `LookbackFinding::detail_lines()` / `stderr_line()` as default-facts forms used only by tests: `task-3-yesterday-hint.md:74-96`, risk R1 at `:490`. T3's stated cause is that T1 keeps `t33b` in `render.rs` with `LookbackFinding` struct literals (`:64`, `:76`).
- T1 does say `t33b` stays (`task-1-foundation.md:308`, `:382`). It does not say this freezes the shape of `LookbackFinding` and `LookbackKind` for T3. Location is consistent: the types live in `src/lookback.rs` (`task-1-foundation.md:139-156`) and `render.rs` re-exports them (`:179`). So suspect (a) is not a location conflict, only a contract-wording and awareness gap.
- Resolution. Plan contract 3: reword to "findings plus per-line facts", and add one line to T1's ownership notes that `t33b` fixes the `LookbackFinding`/`LookbackKind` literal shape. Alternative: let T1 move the lookback half of `t33b` into `src/lookback.rs`, which T3 owns, so T3 can add fields.

### F4. Contract 5 lists live minutes as a loader input. T1 removed it (low)

- Plan contract 5, `plan:117`: input includes "today's live minutes where the page needs them".
- T1 derives live minutes inside the loader and exposes `History.live_minutes`: `task-1-foundation.md:209-216`, `:233` ("Deviation from contract 5"), risk 2 at `:779`.
- T1 declares the deviation and no other task uses the missing input, so nothing breaks. The pinned contract is still stale.
- Resolution. Plan contract 5: change the input list to match T1's published `load_history(conn, shown, today, now_utc)`.

### F5. The end-to-end order "hint, then carry marker" has no owner (low)

- T3 expects the coordinator to add a merged-branch assertion that a marker line follows the hint: `task-3-yesterday-hint.md:450`.
- The plan's merge description lists only "merge, coordinator re-ratchet" and the soft T2-to-T4 test: `plan:86-98`. T1 pins the order only through hand-built `compose_anomaly_lines` tests (`task-1-foundation.md:753-757`). T4's end-to-end test checks the marker after the date's `fix:` lines, not after the hint (`task-4-carry-scan.md:254`).
- Effect. Hint and marker are built by T3 and T4 in parallel, and only the coordinator's undocumented step ever checks them together.
- Resolution. Plan "Ordering and parallelization": add the check to the merge gate, or give it to T4's post-merge step (T4 already rebases for the overlap test).

### F6. No task owns the re-pin of existing tests that T2 or T3 break (low)

- T2 and T3 say a red test in a T1-owned file must be reported "so the owning task (T1 ...) can re-pin it": `task-2-overlap.md:292`, `:424`; `task-3-yesterday-hint.md:399`, `:504`. T2 also lists T1's files as "frozen at merge" (`task-2-overlap.md:31`).
- The plan opens T2-T4 from the merged T1 and says "No two tasks touch a file" and "stops and reports" (`plan:81`, `plan:95`). T1's worktree is gone by then (MEMORY: worktrees removed after merge), so no task can act on the report.
- Resolution. Plan global constraints: state who fixes a pin broken by a later task, for example the coordinator grants a named file to the reporting task. T2 and T3 then replace "owning task".

### F7. Plan T5 paragraph still says short `-h` is unchanged (low)

- Plan T5 paragraph: "leaving short `-h` unchanged" at `plan:267`.
- Plan T5 acceptance (`plan:273`), spec §4.2 (`spec:234`) and T5's task plan (`task-5-cli-surface.md:85`, `:456-470`) all allow clap's `(see more with '--help')` suffix on the help line. T6 agrees with the suffix (`task-6-docs.md:217-220`), so suspect (e) is clean.
- Resolution. Plan T5 paragraph: reword to match line 273.

### F8. T6 leaves SPEC text that describes a future-week `status DATE` page (low)

- Spec §4.5 makes `status DATE` with a future date a hard error: `spec:257-262`. The plan assigns `status DATE` behavior to §7.1 (`plan:301-302`).
- `docs/dev/SPEC.md:821-823` and `:830` say a past or future week gets the plain `Total behind`/`Total ahead` form when "`DATE`'s week isn't the current one". T6's §7.1 edits (S10a-S10d, `task-6-docs.md:321-419`) append text near these lines but never edit them, and S11e puts the future-week statement in §7.2 only (`:472-478`).
- Effect. After the changeset, §7.1 still reads as if `status` can show a future week.
- Resolution. T6 plan: add an S10 sub-edit that limits the "past or future week" wording in §7.1 to past weeks, with `mlm week WEEK_ID` named for the future case.

## Checked and found consistent

- Suspect (b). T1 moves `lookback_excludes_age_one_open_start` and `today_page_lines_equals_flattened_findings` into `src/lookback.rs` (`task-1-foundation.md:443-445`, `:768`). T3 owns that file and rewrites both (`task-3-yesterday-hint.md:247-248`, `:325`) with the same test names. T3 uses T1's age 4 and 10 fixture, and the original age 1 and 4 fixture matches `src/status.rs:2724-2750`.
- Suspect (d). T6's quoted echo line, argument-order message, usage line (`[WEEK_ID] <DURATION>`), marker layout, footer constant and `done for today` all match T5 (`task-5-cli-surface.md:151`, `:390-399`, `:232`) and T1 (`task-1-foundation.md:544-556`). The only mismatch is F2's grep fixtures.
- Suspect (f). T4 calls `anomalies::date_facts(&trio, date, today, now_utc).has_any()` (`task-4-carry-scan.md:121`), matching T1's signature (`task-1-foundation.md:125-130`). T2 adds `overlaps` and `latest_open_next` inside `build_facts` and `has_any`, so T4 needs no change and its overlap tests wait for T2 as the plan says.
- Suspect (g). Test-file ownership: T1 `status_cli`, `e6_db_failure`, `ascii_sweep`, `week_read_errors`; T2 `overlap_cli`; T3 `start_hint_cli`; T4 `carry_marker_cli`; T5 `week_target_cli`; T6 none. No file is claimed twice. The existing `tests/` directory holds only T1's three files. `examples/seed_test_data.rs` references none of the moved items.
- Dependency graph. Every call T2, T3 and T4 make resolves to a T1-published seam. T3 and T2 do not call each other. The T2-to-T4 soft dependency is declared on both sides. T5 and T6 share no file with any other task.

## Verdict

ship-with-followups

F1 and F2 should be fixed before the task plans are dispatched, because F1 changes T2's pinned output and F2 makes T6's merge check unreliable. F3-F8 are plan wording and ownership edits that do not block any task.
