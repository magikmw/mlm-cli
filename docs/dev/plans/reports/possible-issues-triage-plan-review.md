# Plan review: possible-issues-triage

Reviewed `docs/dev/plans/possible-issues-triage-plan.md` against the locked spec `docs/dev/specs/2026-10-03-possible-issues-triage.md` and the source on branch `possible-issues-triage`. Line references are to the plan unless a path is given.

## Findings

### F1. High. E10 and "no `<ARGS>` in any clap error" has no working mechanism (plan:49, 267; T5)

- Claim: keep the single variadic positional, override the usage line, replace the Arguments entry, and `<ARGS>` appears nowhere in the help or in the missing-argument (E10) error.
- Evidence: I built the same shape in `/tmp/cd-scratch/clapt` with clap 4.6.6 (the version in `Cargo.toml:32`), copying `src/cli.rs:201-211`, `override_usage = "mlm week target [WEEK_ID] <DURATION>"` and `value_name = "ARGS"`.
  - `mlm week target` still prints `the following required arguments were not provided:\n  <ARGS>...`. Only the `Usage:` line changes.
  - `--help` still shows `Arguments:\n  <ARGS>...  doc`.
  - Switching to `value_names = ["WEEK_ID","DURATION"]` gives `<WEEK_ID> [DURATION]...` in both places, so that is no fix either.
- Consequence: the AC as written (plan:49, 267) cannot pass with "override usage + replace Arguments entry". The plan never names how the entry is replaced. Clap has no replace hook, so the likely route is `hide = true` plus a hand-written `after_help` Arguments section. Hiding does not change the required-arguments list in E10.
- Fix: pick a mechanism now (for example hidden positional plus custom `help_template`, and an explicit decision on the E10 list line), or narrow the AC to the usage line and Arguments section as spec §4.3 words it. The spec says "every clap error ... name the arguments in both the usage line and the Arguments section", which does not mention the required-arguments list.

### F2. Medium. T1 says "the one test" pins a future `status DATE`; there are two (plan:139)

- Claim: only one test, the one that pins a future `status DATE` succeeding, changes in T1.
- Evidence: `src/status.rs:1706-1716` (`resolve_future_absolute_date_still_succeeds_and_renders_empty`) and `src/status.rs:3288-3295` (`pit_future_week_carry_in_includes_live`, which calls `resolve(Some("2026-02-16"), now_thu_1800(), ..)` when today is 2026-02-12). Once §4.5 lands in `resolve` the second test errors.
- Consequence: T1's "expectations unchanged except two items" rule (plan:139, 315) is violated by a second test, and the live-inclusive future-carry check on the `status` side loses its only pin. `src/week_view.rs:1421` covers the `mlm week` side only.
- Fix: name both tests. Say the second is rewritten against the `mlm week` path or a past date, and that its assertion is kept in some form.

### F3. Medium. Loader reads target overrides once per walked week, which contradicts "far-future completes quickly" (plan:40, 142)

- Claim: the loader makes one ranged punch read, "the target-override reads are made per walked week as today", and a year-9000 week id "completes quickly".
- Evidence: `src/status.rs:534-538` loops `date::week_range(start_week, through)` and calls `get_week_target` for every week. Year 9000 is about 370,000 weeks, so that is about 370,000 SQLite queries. `week_series` (`src/week.rs:162-205`) also builds a 370,000-entry `Vec`. T1 may not add a ranged override read, because `src/storage.rs` is unowned and `src/week_target.rs` belongs to T5 (plan:39, 159, 282).
- Consequence: the AC has no stated bound and no measured baseline. T1 either misses it or needs a file it does not own.
- Fix: either make the AC "one punch read" only, or give T1 the override read (a single `SELECT` of all `week_targets` rows) and say which file owns that function.

### F4. Medium. Plan text calls the clap failure an "argument-conflict error"; the discriminator is the token shape, and help/version need handling (plan:45, 269)

- Evidence:
  - The scratch build confirms `week 2026-41 target 20h` returns `ErrorKind::ArgumentConflict` (matches `clap_builder-4.6.6/src/parser/parser.rs:536-545`).
  - The same kind is returned for `week 7 foo`, which prints `the subcommand 'foo' cannot be used with '[WEEK_ID]'`.
  - The kind therefore says nothing about the shape, and the pure-function token match carries all the weight. That is feasible, but the plan should say so.
- Gap: `Cli::try_parse` also returns `DisplayHelp` and `DisplayVersion` as `Err`. The plan does not say the fallback must call `err.exit()` (exit 0 for help). The AC "every other clap error keeps clap's text and exit 2" is false for those two kinds.
- Gap: `mlm w 2026-41 target 20h` echoes `mlm week target ...` (the alias is not echoed). That is a visible choice, see U9.

### F5. Medium. T6 writes SPEC §7.3/§7.4/§8 hint text before the window edge cases are decided (plan:87, 314)

- Claim: T6 runs in parallel with every code task and quotes pinned literals.
- Evidence: the plan itself says the one-minute window, S at 23:59 with a later same-minute punch, and the fallback sentence are left to "T1's task plan" and "T2 and T3 then assert... where a legal minute exists" (plan:314). SPEC §7.3, §7.4, E16, E18 and E19 must show exactly these hint forms (plan:295-298).
- Consequence: T6 either guesses the edge-case wording or leaves it out, and the coordinator finds a drift at merge.
- Fix: sequence the edge-case decision (a coordinator ruling before T6 opens), or mark the §8 flows and §7.3/§7.4 hint-time text as a T6 second pass after T2 and T3 merge.

### F6. Medium. Location of the clean-pair phrases is unpinned, so T2 and T3 may both need to edit it (plan:25, 127)

- Evidence: T1 "builds the helper and phrases" but the plan names no file. T2 owns only `src/anomalies.rs` and T3 only `src/lookback.rs` and `src/commands.rs`. If the phrases live in `src/anomalies.rs`, T3 depends on a file T2 edits. If they live in `src/render.rs` (T1-owned), neither T2 nor T3 may change them.
- Consequence: the "T2 and T3 never wait on each other" claim (plan:25) holds only if the phrases are frozen by T1.
- Fix: state the file and say the phrases are frozen after T1 (a T2/T3 need to change them is a stop-and-report).

### F7. Low. Old `render::` paths used by `src/commands.rs` are not named among the re-exports (plan:22, 135, 154)

- Evidence: `src/commands.rs:73, 78, 1757-1844` use `render::LookbackFinding` and `render::LookbackKind`, including struct-literal construction. The plan names the `status::lookback` re-export (plan:22) and "re-export the old paths" (plan:135), but T1 also moves the finding types out of `src/render.rs` (plan:135) and may not touch `src/commands.rs` (plan:154).
- Fix: list `render::LookbackFinding` and `render::LookbackKind` explicitly as paths that must keep compiling.

### F8. Low. §3.4 test coverage cannot isolate the carry-walk read (plan:141)

- Evidence: `src/storage.rs:501-515` `earliest_data_date` unions `punches` and `notes`, so dropping `punches` fails the earliest read first. The "punches table" case never reaches the ranged punch read. The spec lists three reads (override, earliest, carry walk), and the plan proves two.
- Fix: add a case that passes the earliest read and fails the ranged read (for example a punch row with an unparseable `kind` or `at_utc`, since `punch_from_row` errors on it at `src/storage.rs:411-415`).

### F9. Low. Stale doc comment and unowned docs (plan:159, 310)

- `src/date.rs:123-125, 159-160` say "`status` calls this directly" and "keeps accepting future dates". `src/date.rs` is unowned, and §4.5 makes the comment false.
- `AGENTS.md` "Layout" lists modules and says `status.rs`/`week_view.rs` are per-command wiring; after T1 it is missing four modules (`anomalies`, `lookback`, `history`, `carry_scan`). The Documentation rule in `AGENTS.md` makes SPEC/NOTES self-contained, but nothing in the plan owns `AGENTS.md`.
- Fix: give the comment to T1 (a one-line touch) and `AGENTS.md` to the coordinator's close-out.

### F10. Low. `status` onto the shared loader is scope beyond the spec, and the plan is candid about it (plan:315)

- Spec §3.4 only requires `mlm week` to hard-error and leaves the mechanism to the plan. `status` already propagates errors (`src/status.rs:524-563`). Moving `status` is justified only by sharing data with the scan.
- Safety: equivalence holds. `WeekLedger.earliest_data_week` is the minimum of `data_weeks` (`src/week.rs:126-129`), so one earliest-date read plus the live credit (`status.rs:565`, which also marks the current week as a data week through `with_worked`) is enough. The loader must keep `with_data_week(earliest)` for a notes-only first week, and must keep the live credit's data-week side effect.
- The risk is the 3,000 lines of inline `status` tests (F2 shows they are not unchanged). Alternative: keep `status::build_ledger` and only add the scan's read. The plan's choice is defensible, but T1 is the largest task and the first fallback to split off the loader is already noted (plan:320).

### F11. Low. Complexity gate

- Nothing in T1 is obviously over 15 if the plan's own split holds. The risk is `anomaly_facts_for_date` (`src/status.rs`) and `row_anomalies` (`src/week_view.rs:186-241`), which T1 merges into one builder that T2 then grows. T2 adds overlap pairs and the next-punch time to that same function. The plan says "split in the task plan" (plan:66) but does not give T2 the same instruction. Add it to T2's constraints.

## Spec-to-task coverage

| Spec | Task | Note |
|---|---|---|
| §3.1 hint, ages 1-14, `mlm start` warning | T3 (seams in T1) | Covered. |
| §3.2.1 overlap | T2 | Covered. |
| §3.2.2 clean pair | helper T1, stale/E7 T2, lookback/hint/start T3 | Split by design; see F6. |
| §3.3 marker | rendering T1, data T4 | Split by design. T4's overlap test needs T2 (declared). |
| §3.4 read errors | T1 | F8. |
| §3.5 echo | T5 | Covered. |
| §4.1-§4.3 | T5 | F1, F4. |
| §4.4, §4.5 | T1 | F2. |
| §5, §6 | T6 | F5. |
| §7 tests | per task | Covered. |

File ownership is exclusive as listed. Checked: `src/lib.rs` (T1 only), Cargo files (no task needs them), `coverage-baseline.json` (coordinator), `tests/` (T1 owns three existing files; T2, T3, T4, T5 each own one new file). The three new modules are created by T1 and handed to T2, T3 and T4, and each handoff names one owner. The gaps are in F6, F7 and F9.

Dependency graph is right. T5 and T6 are independent of T1 by file. T2, T3 and T4 depend on T1. T4's only T2 link is the declared overlap test.

## Undeclared user-visible gaps

Each item is a deliberate choice whose visible consequence is not in spec §6 or the plan's risk list. Candidates for the known-issues section.

- U1. The `[i]` hint is today-only. A user running `mlm status -1` or `status DATE` for yesterday sees the open start only as a stint line, with no hint. Plan:318 notes it, spec §6 does not.
- U2. `mlm start` warns about yesterday's open start on every start while an overnight session is open, but spec §6 mentions only `status` ("every `status` until it is stopped"). A user who runs `mlm start` for a second stint repeatedly sees the warning each time.
- U3. At age 1 the output is an `[i]` hint with a fix line; at age 2 it becomes a `[!]` stale anomaly and marks the `mlm week` row and footer. The mark appears one day after a quiet hint, with no note in the spec.
- U4. The clean-pair window can print an unsatisfiable instruction: `(a time between 14:00 and 14:01)` when N is one minute after S has no legal minute strictly between them (plan:314 states this and punts it to a possible spec change). It is a user-visible choice and belongs in known issues.
- U5. A punch at the same minute as S is ignored for N (plan:127), so `(a time after S)` can print while a same-minute punch exists. Not mentioned in the spec.
- U6. The carry marker counts E8 (orphaned end) weeks and says carry-in "includes anomalies", although an orphaned end contributes zero minutes. The line can name a week whose figures are correct. Spec §6 mentions only the parenthetical placement.
- U7. The marker excludes the shown week and anything later. Viewing an older week hides anomalies in later weeks, even though a later week's carry-in includes the older week's. Viewing the anomalous week itself shows `[!]` rows but no marker.
- U8. `status` can no longer preview a future date, so the `status` week line never shows a future week's `Total behind` (only `mlm week` does). Spec §6 says this in general terms; the loss of the future carry-in view on `status` is the same fact and is not stated.
- U9. The argument-order message echoes `mlm week target ...` even when the user typed the `w` alias, and it prints no `Usage:` block or `--help` hint. Other clap errors keep both.
- U10. The `week target` echo uses the standard formatter, so `mlm week target 7 90m` echoes `01h 30m`, not the user's token. Spec §3.5 says "standard formatter" but not that a script comparing input to output will differ. The echo also runs under `--verbose`, and a shell script reading stdout from `week target` (previously silent) now sees a line (declared in spec §6).
- U11. Storage-error text for `mlm week` and `status` can differ (plan:41 accepts it). A user comparing the two sees different `error:` wording for the same fault.
- U12. Default log level `Warn` hides any future `info!` line too, not only the migration line. Today only the migration emits one (`grep` shows no `log::info!` in `src/`), so the visible effect is limited to a first run, which spec §6 already states.
- U13. T1 moves `status` to one ranged read. On a large database the `status` page gets faster, and the first run after upgrade changes timing. No output change. (Timing only; no further note needed unless the user cares.)
- U14. `done for today` replaces `target already met` only on the Day total line. `src/status.rs:94-95, 206` shows the same enum state feeds `est. EOD`'s slot. The README samples that show the old words next to `est. EOD` are in the same line, so there is no second location. The plan's wording in T6 ("not where they show `est. EOD`'s own state", plan:301) is therefore confusing, because both are the same text on the same line. Clarify before T6 starts.
- U15. Past weeks with old anomalies gain new `[!]` rows, footers and the marker on first run (spec §6, declared). The plan's one extra consequence is that the overlap check reclassifies every historical date on every `status` and `mlm week`; no cost bound is stated beyond "personal-database sizes" (plan:317).

## Verdict

needs-rework

F1 blocks T5 as written (the acceptance criterion cannot be met with the stated mechanism). F2, F3, F5 and F6 should be settled before worktrees open. F4 and F7-F11 can be folded into task plans.
