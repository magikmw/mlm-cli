# Plan: possible-issues-triage

Spec: `docs/dev/specs/2026-10-03-possible-issues-triage.md` (locked). Spec review: `docs/dev/plans/reports/possible-issues-triage-spec-review.md`. Decision log: NOTES.md entries 96-98. Plan review: `docs/dev/plans/reports/possible-issues-triage-plan-review.md` (folded, entry 98).

**Altitude note**: scope of work only. No code, no pseudocode, no function, type or SQL names. File paths, including paths of new files, are named because ownership depends on them. Task plans choose the identifiers.

## Goal

SPEC §1.2a lists 34 possible issues. This changeset fixes the ones that hide a mistake or skew a figure silently, applies cheap wording and logging polish, and moves accepted behavior out of §1.2a into the spec sections that own it. The fixes are the §3.1 start-still-open-from-yesterday hint, the §3.2 overlapping-stints anomaly with a clean-pair rule for stop hints, the §3.3 carry-in marker, `mlm week` read failures becoming hard errors, and the `week target` echo. The polish is the `Warn` default log level, the `--help` glossary, the `week target` argument-order message, `done for today`, and rejecting a future `status DATE`. Pairing, the splice, carry arithmetic, the week-line wording, the 2-day stale threshold and stored data do not change.

## Architecture (prose)

### Current shape

Per-date anomaly facts are built in two places that duplicate each other. `src/status.rs` builds them for one date from a four-way classification (post-splice day, plus raw previous, day and next). `src/week_view.rs` builds them for each of seven rows. Both feed `DateAnomalies` in `src/render.rs`, which also renders the anomaly lines. The lookback scan (ages 2-14, one ranged read) lives in `src/status.rs`, its finding types live in `src/render.rs`, and `src/commands.rs` consumes both for `mlm start`'s stderr warnings. `status` accounts weeks through a ledger prefetched day by day. `mlm week` accounts weeks through an adapter that answers the week-accounting traits from storage and swallows read errors (`src/week_view.rs`). The week-accounting traits in `src/week.rs` are infallible, and the pure accounting tests rely on that.

### Target shape

Every task after the foundation edits a different file. The foundation task creates the seams.

1. **One per-date anomaly module.** It builds the full fact set for a date from raw punch buckets (previous, day, next), today and now, and it renders the date's anomaly lines. `status` rows, `mlm week` rows and the carry scan all call it, so the row `[!]`, the block and the marker cannot disagree (§3.3: "same per-date anomaly predicate"). It is the one home for the overlap anomaly (§3.2.1) and the clean-pair hints on stale-start and E7 lines (§3.2.2). Nothing outside it constructs the fact type field by field.
2. **One lookback module.** It owns today's lookback scan, the finding types and the `mlm start` stderr forms. It will also own the §3.1 hint. Today's page gets its lookback and hint lines from one entry point that returns finished lines. `status::lookback` stays reachable under its old path so `src/commands.rs` compiles untouched until its own task.
3. **One history loader** (decision F6, below). It makes one ranged punch read from the earliest data date to the shown week's end (padded by one day each side) and builds the week ledger from it. It returns the ledger and the bucketed punches, so the carry scan reuses the same data and makes no second read. Both `status` and `mlm week` use it.
4. **A carry-scan module**, stubbed by the foundation task to report "no marker" and filled in by its own task. The marker's data shape, line text and both renderings (`status` line, `mlm week` `[!]` on the Carry-in figure plus the line after the footer) are built by the foundation task from hand-built inputs. The scan task only produces the data.
5. **Clean-pair helper and text** (§3.2.2). A pure helper finds the first punch on a date after a given start. Shared phrases render the `(a time after S)` and `(a time between S and N)` parentheticals, with and without the E7 `closes the S start` tail. The foundation task builds and unit-tests them. The overlap task (stale and E7 lines) and the hint task (lookback lines, hint, start warnings) both consume them, so those two tasks never wait on each other.

`src/status.rs` and `src/week_view.rs` end up as thin wiring over these modules. `src/week.rs` stays untouched in the normal case, because the loader feeds the existing infallible ledger type.

### Decisions the spec left to the plan

**F6, `mlm week` read-error mechanism.** Prefetch a ledger through the shared loader. Do not make the week-accounting traits fallible.

- Reasons:
  - The traits and `week_series`/`week_accounting` and all their pure tests stay as they are.
  - One ranged read from the earliest data date to the shown week replaces `mlm week`'s per-week reads and `status`'s per-day reads, so cost falls instead of rising. The spec review (F6) feared the opposite because it assumed `status`'s per-day pattern would be copied.
  - The carry marker needs a scan of every earlier date with punches anyway. The loader gives it that data for free.
- Constraints:
  - The loader visits only dates that have punches, never every calendar date, so a far-future week id does not walk millions of empty days. The ledger's idle-week walk already lives in `week_series` and is untouched.
  - Only the earliest data week matters to the walk (the ledger uses the minimum data week), so one earliest-date read, which already counts notes, is enough. No notes range read.
  - Target overrides come from one ranged read over the walked weeks, added to `src/storage.rs` by T1. Per-week override reads would make a far-future week id cost hundreds of thousands of queries. Any failure propagates.
  - The loader keeps two behaviors of today's `status` ledger: the earliest data week (punches or notes) anchors the walk, and live credit marks the current week as a data week.
- Message and exit: the error flows through the normal error path. Stderr gets `error: <storage error text>`, stdout stays empty, exit is `1`. The two commands may word the storage text differently, and that is accepted. Exit `1` follows `src/main.rs` ("one nonzero exit code for every hard error"). SPEC §6.3 only says "nonzero", and no distinct storage-error code is introduced (answers spec-review F4).

**F7, `week WEEK_ID target DURATION` message and help.**

- Message: clap rejects this shape before any of our code runs. Parse with the fallible clap entry point. When clap fails with any error and the raw arguments match exactly `week`, one week-id token, `target`, one duration token (the global verbose flag allowed anywhere, the `w` alias allowed), print the one-line custom message and exit `2`. The detection is a pure function over the argument tokens, so it is unit-testable without the binary.
  - The message is exactly the line in spec §4.3, echoing the user's tokens, and prints no clap `Usage:` block.
  - The discriminator is the token shape, never the clap error kind, because the same kind also covers unrelated junk such as `week 7 foo`.
  - Help and version requests pass through clap's own exit path unchanged (exit `0`).
  - Every other clap error keeps clap's own text and exit `2` (`mlm week 7 foo`, `week 2026-41 target` with no duration, `week 2026-41 target 20h extra`). Only the exact four-token shape is intercepted for this message.
  - Reasons: this leaves the clap definition and its other error texts alone, and it avoids reshaping `WeekArgs`, which would change errors for unrelated junk.
- Help and usage: keep the single variadic positional. A required positional cannot follow an optional one in clap, and its comment says so. A clap 4.6 scratch build (plan review F1) showed that a usage override fixes only the usage line, so three parts are needed:
  - override the usage line to `mlm week target [OPTIONS] [WEEK_ID] <DURATION>`;
  - hide the positional from the generated Arguments section and write an Arguments section by hand that names `[WEEK_ID]` and `<DURATION>`;
  - in the same fallible-parse step, rewrite the context of clap's own errors for `week target` (missing argument E10, too many values) so they name the two arguments; clap's text and usage line otherwise stay (T5 task plan, finding 1).
  - Acceptance: the substring `<ARGS>` appears nowhere in `mlm week target --help` or in any error printed for that subcommand.

**F8, where anomalies are collected for the carry walk.** The one per-date anomaly module is the single place. The carry scan:

- iterates only dates that have punches, from the earliest data date through the day before the shown week starts, using the loader's bucketed punches;
- classifies each date against its previous and next buckets exactly as a `mlm week` row does, and asks the shared predicate (`has_any`, including overlap) whether the date would carry `[!]`;
- records the earliest week containing such a date, and the count of distinct weeks;
- ignores note-only dates (notes cannot be anomalies, and a notes-only first week still anchors the carry walk as today).

The per-date fact set grows by the overlap pairs and the clean-pair next-punch time. That is data the module computes itself, so `has_any`, the block and the row agree by construction.

## Global constraints

- Every task passes the AGENTS.md verification set before it counts as done. That is `cargo fmt`, `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings`, and the two `MLM_DB_PATH` smoke runs of `start` and `status`.
- The pre-commit hook gates every commit. Cognitive complexity per function stays at or under 15 (`clippy.toml`).
  - The new logic (overlap detection, hint composition, marker scan, argument interception) is written as small helpers from the start, not extracted after a failure.
  - `stint::classify` is already at the limit and stays untouched.
  - The fact-building function grows the most. T1 merges the two builders into one and splits it in its task plan. T2 grows it again with overlap facts and must split again rather than push it over the limit.
- Coverage must not drop below `coverage-baseline.json`. Every branch added gets a test written first. Moved tests are moved, never deleted. Pins of old strings that invert under a new rule are rewritten to the spec's new behavior.
- `coverage-baseline.json` is coordinator-owned. If the hook rewrites it in a task worktree, the task reverts that change before its final commit, and the coordinator re-ratchets after the last merge. Merge conflicts on it resolve to the higher value.
- TDD per spec §7. Tests come first against the exact output lines in the spec, and fail for the right reason.
- Output that the spec does not change stays byte-identical. This includes past-week output, stint lines, notes, the headline, `est. EOD`, carry arithmetic and the schema. Each task's tests include a byte-identical check for the scenario it touches without intending to change.
- `[i]` and `[!]` are the only markers. The §3.1 hint never marks a `mlm week` row and never counts toward the week footer.
- No pointers from SPEC, README or `--help` to the spec file or to `docs/dev/plans/` (AGENTS.md Documentation rule).
- NOTES.md and the spec file are coordinator-owned. No task edits them.
- A task that finds it needs a file outside its list stops and reports so the coordinator can re-scope. No two tasks touch a file.
- One exception, for T2, T3 and T4 only. When the task's own spec change inverts an existing test assertion in a T1-owned file (`src/status.rs`, `src/week_view.rs`, `src/render.rs`, the T1 test files), the task may re-pin that assertion's expected value and nothing else. It lists every such re-pin in its completion report. The coordinator resolves any overlap between two tasks' re-pins at merge.
- Integration tests under `tests/` follow the existing pattern: drive the compiled binary with `MLM_DB_PATH` and an isolated temp database, seed punches by raw insert, pin `TZ` where the clock matters.

## Ordering and parallelization

```
T1 foundation ──┬─> T2 overlap + clean-pair lines ──┐
                ├─> T3 yesterday hint + start warning│──> merge, coordinator re-ratchet
                └─> T4 carry-scan ───────────────────┘
T5 CLI surface (independent) ─────────────────────────┘
T6 docs (independent) ────────────────────────────────┘
```

- **T5 and T6 start immediately**, in parallel with T1. They own files disjoint from every other task, and they quote the literals pinned below. The coordinator checks T6's quoted strings and T5's glossary against the built binary at merge.
- **T1 is the critical path.** T2, T3 and T4 open their worktrees from the merged T1.
- **T2, T3 and T4 run in parallel.** Their owned files are disjoint (the per-date anomaly module, the lookback module with `src/commands.rs`, and the carry-scan module).
- **T4 has one soft dependency on T2.** T4's acceptance test that an overlap-only earlier week raises the marker needs T2's overlap anomaly. T4 builds and tests everything else against T1, and adds that one test after rebasing onto merged T2. Merge order is T2, then T3 and T4 in either order. After the last of T2-T4 merges, the coordinator's merge gate checks end to end that today's page prints the §3.1 hint and then the §3.3 marker line in that order.
- **Critical path**: T1 -> T2 -> T4's final test -> merge. T3 and T5 and T6 are off the critical path.
- T1 is the largest task. Its task plan may split it into sequential commits (loader and read errors, then anomaly/lookback module extraction, then polish), but it stays one worktree because every step edits `src/status.rs` and `src/week_view.rs`.

## Interface contracts to pin before opening worktrees

All in plain terms. The spec's sample blocks are the byte-exact reference.

1. **File ownership map.** As in the task sections. The new files are `src/anomalies.rs`, `src/lookback.rs`, `src/history.rs` and `src/carry_scan.rs`. T1 declares all four in `src/lib.rs`, so no other task edits that file.
2. **Per-date anomaly module.**
   - Input is raw punch buckets for previous, day and next, plus today and now.
   - Output is a fact value, with `has_any`, the ordered detail lines, and the row marker.
   - The fixed block order is E7, stale, E8, overlap, refused join with previous, refused join with next.
   - The fact type has a way to build a default value, so adding a field never breaks another file's tests. Only this module constructs it field by field.
3. **Lookback module entry points.**
   - Today's page: input is the connection, today, now, and a flag saying whether today's own page already shows a refused-join line naming yesterday. Output is the finished lookback and hint lines in the spec §3.1 order, which is lookback lines oldest first, then the hint.
   - `mlm start`: structured entries for the stderr warnings. T1 publishes the finding types; T3 may wrap them in its own entry type in `src/lookback.rs` to carry the clean-pair `N` and whether today has punches, because T1's frozen `render.rs` test `t33b` builds the finding types as struct literals. The just-written-punch exemption and best-effort rule of SPEC §7.4 stay unchanged.
   - After T1 the behavior equals today's. The flag is accepted and ignored until T3.
4. **Status page composition.** For any date, `status` prints the date's anomaly block, then (today only) the lookback and hint lines, then the carry marker line. This is the §3.1 and §3.3 placement. T1 builds the composition.
5. **History loader.**
   - Input is the connection, the shown week, today and now. The loader derives today's live minutes itself.
   - Output is the ledger (worked per week with live added to the current week only, data weeks, target overrides) plus the bucketed punches covering earliest data date through the shown week.
   - The earliest data week (punches or notes) anchors the walk, and live credit marks the current week as a data week, both as today.
   - Any storage failure is an error. It never degrades to defaults.
6. **Carry marker data and text.**
   - Data: number of earlier weeks with an anomaly (at least 1) and the oldest such week's id.
   - Line text, byte for byte from spec §3.3:
     - `[!] carry-in includes anomalies from 2 earlier weeks, oldest 2026-38 (see mlm week 2026-38)`
     - with one week: `[!] carry-in includes anomalies from 1 earlier week, 2026-38 (see mlm week 2026-38)`
   - `mlm week` renders the Carry-in figure as `Carry-in:      -80h 00m  [!]`. The marker line sits in the footer slot, which is above the Carry-in block: day rows, blank line, the anomaly footer if any row is marked, the marker line directly under it (no blank between), then one blank line, then the Carry-in block. With no marked row, the marker line takes the footer's place. Nothing is printed when the data is absent.
   - T1 renders it from hand-built data. T4's scan produces the data.
7. **Clean-pair rule.**
   - Target: one open start S. N is the first punch on S's date strictly later than S (any punch kind). In practice N is a start, because an end after S with no start between would have closed S.
   - The window includes N's own minute. A stop at the same instant as a start N pairs with S, because a same-instant end first pops starts carried in from earlier groups (SPEC §4.3 step 2). So the window always holds a legal minute, even when N is one minute after S. Round-trip tests cover a stop at N's minute.
   - Phrases:
     - no N: `(a time after S)`
     - with N: `(a time between S and N)`
     - E7 variants add ` closes the S start`
   - Punches at the same minute as S do not count as "after S".
   - The helper and phrases live in `src/render.rs`. T1 writes and unit-tests them, and their signatures are frozen when T1 merges. T2 and T3 call them and do not edit `src/render.rs`. T2 applies them to the stale-start and E7 lines. T3 applies them to the lookback lines, the §3.1 hint and `mlm start`'s warnings.
8. **Literals.** Spec §3.1 (the `[i]` hint, the `or:` line, the `(yesterday)` forms, the `mlm start` warning), §3.2.1 (the overlap line and its `fix:`), §3.3, §3.5 (`Week 2026-41 target: 20h 00m`), §4.3 (the argument-order message), §4.4 (`done for today`). The `--help` glossary wording is T5's, checked against SPEC §1.3, §2.4, §7.1 and §7.2. It must call carry-in a balance where a shortfall is negative and a surplus positive, because surplus does carry (spec-review F2).
9. **Exit codes.** Every runtime error exits `1` through the existing error path. Only clap-owned usage errors, and the intercepted argument-order message, exit `2`. SPEC §6.3 stays "nonzero".

## T1 — Foundation, read errors and one-line polish

**Spec citation**: §3.4, §4.4, §4.5, and the seams that §3.1-§3.3 and §3.2.2 need (architecture items 1-5 above).

**Paragraph**: Do the extraction and plumbing that lets the later tasks work on separate files, with no visible change except the three spec items below. Move the anomaly facts and their rendering into one per-date anomaly module used by `status` and by `mlm week` rows. Move the lookback scan and its finding types into one lookback module and re-export the old paths. Build the shared history loader, and move `status` and `mlm week` onto it, which makes `mlm week` read failures hard errors. Build the carry marker's data shape, line text and both renderings from hand-built data, with a stub scan that reports no marker. Build the clean-pair helper and phrases. Change `Day total` wording to `done for today` and make `status DATE` reject a future date. Declare all new modules in `src/lib.rs`, so later tasks never edit it.

**Acceptance criteria**:

- Every existing `status` and `mlm week` test passes with its expectations unchanged, except pins of `target already met` on the Day total line (now `done for today`) and the two tests that run `status` on a future date (`src/status.rs` near lines 1706 and 3288, the second being the future-week live carry-in test). The first becomes the §4.5 rejection test. The second keeps its carry-in assertion by checking `mlm week` for that future week instead. Moved tests keep their assertions.
- `status` and `mlm week` anomaly blocks, row markers, footer and lookback lines are byte-identical to today for every existing fixture. This includes the stale, E7, E8 and refused-join orderings.
- §3.4. Make each of these fail in turn on an otherwise valid database: the `week_targets` table and the `notes` table (which breaks the earliest-data read). The ranged punch read cannot be broken alone from the CLI, because the earliest-data read also reads `punches`; cover it with a loader-level unit test that makes only that read fail, or report why that is not feasible. Each CLI case makes `mlm week` and `mlm week WEEK_ID` exit `1`, print nothing on stdout, and print a stderr line starting with `error:`. Successful `mlm week` output is byte-identical to before for current, past and future weeks, including the live-inclusive current week and a future week's carry-in.
- A far-future week id (for example year 9000) completes quickly on a small database. The loader makes one ranged punch read, not one per day.
- `status` and `mlm week` produce equal ledgers (carry-in, worked, fulfillment, target) for the same week on the same database.
- §4.4. `done for today` replaces `target already met` on the Day total line, with the same trigger (gap at or below zero with an open stint today). `est. EOD` wording is unchanged. Update the literal in `src/status.rs`.
- §4.5. `status DATE` with DATE after today exits `1` with the same message shape as `start`/`stop`/`note --date` in the future. `status` for today, a past date, and `-N` shorthand is unchanged. `mlm week` for a future week still prints.
- Carry marker renderings. Given hand-built data of 2 weeks and of 1 week, `status` prints the exact lines of contract 6, after the date block and any lookback lines. `mlm week` prints the `[!]` on the Carry-in figure and the line after the footer. With no data, neither prints and output is byte-identical to today.
- Clean-pair helper. Cover no later punch, a later start, a later end, a later punch at the same minute as S, S at 23:59, and punches on other dates being ignored. Phrases match contract 7 for the plain and E7 forms.
- A unit test pins that the lookback entry point's output equals today's lookback lines when the refused-join flag is true or false.
- The full verification set, the complexity limit (every new function at or under 15) and the coverage ratchet pass.

**Out of scope**:

- Overlap detection, clean-pair application to any existing line, the §3.1 hint and the carry scan. All stay stubbed or unchanged.
- Any change to `src/commands.rs` beyond import paths for the moved lookback types (`render::LookbackFinding`, `render::LookbackKind` and `status::lookback`). T3 owns the file after T1 merges.
- Storage schema, and any `src/storage.rs` change beyond the ranged target-override read. `src/week.rs` changes beyond a doc-comment wording touch about read errors.
- README, SPEC, NOTES.
- `--help`, `main.rs`, `week_target.rs`.

**Files owned exclusively**: `src/status.rs`, `src/week_view.rs`, `src/render.rs`, `src/stint.rs`, `src/lib.rs`, `src/week.rs`, `src/storage.rs`, `src/date.rs` (the comments that say `status` accepts future dates), and `src/commands.rs` import lines only (handed to T3 at merge), plus the new files `src/anomalies.rs`, `src/lookback.rs`, `src/history.rs` and the stub `src/carry_scan.rs` (until T4 takes it over). Also the existing integration tests `tests/status_cli.rs`, `tests/e6_db_failure.rs`, `tests/ascii_sweep.rs`, and the new `tests/week_read_errors.rs`.

## T2 — Overlap anomaly and clean-pair stop hints in anomaly lines

**Spec citation**: §3.2.1, §3.2.2 (stale-start line and E7 line), §7 (§3.2 and §3.2.2 tests).

**Paragraph**: Teach the per-date anomaly module to detect overlapping stints and to render the overlap line. Pair every closed stint and today's most recent open stint taken up to now, using the part of a spliced stint that falls on each date. Emit one line per overlapping pair in the specified order, include it in `has_any`, and place it after orphaned ends and before a refused join. Apply the clean-pair rule to the stale-start and E7 hints, using the helpers from T1. The `status` page, the `mlm week` row `[!]` and the footer count all follow from the shared module with no edit to `src/status.rs` or `src/week_view.rs`.

**Acceptance criteria** (all against the exact lines in spec §3.2.1):

- Overlap detection:
  - nested pair: flagged
  - identical ranges: flagged, one pair
  - nest sharing one endpoint, at start and at end: flagged
  - stints that only touch at an endpoint: not flagged
  - a zero-length stint: not flagged
  - today's open stint containing a closed one: flagged, with the open stint shown as `09:00-now`
  - an open stint on a past date: not flagged
  - a today open stint that starts after now (live zero): not flagged
  - a spliced cross-midnight stint taking part on each date with the part that falls on that date
  - deep nests: one line per overlapping pair (3 levels give 3 lines, 4 give 6), sorted by first start then second start
- Line text and `fix:` text match the spec byte for byte. Block order is E7, stale, E8, overlap, refused join.
- A date with only an overlap marks its `mlm week` row `[!]`, prints the footer once, and `status DATE` shows the overlap block. The day and week totals are unchanged (the overlap still counts twice).
- Clean-pair rule on the stale-start line: no later punch gives `(a time after S)`. A later punch gives `(a time between S and N)`.
- Clean-pair rule on E7: the `closes the S start` tail is kept and the window form is used when a punch follows S.
- Follow-the-hint round trip, a regression guard for spec-review F1. For each of the stale-start and E7 fixtures that has a later punch, adding the stop at a time inside the printed window produces no overlap anomaly. A stop at N's own minute is one of the asserted times (contract 7).
- Output for dates with no overlap and no later punch after the targeted start is byte-identical to before.
- Full verification set, complexity at or under 15, coverage ratchet.

**Out of scope**:

- The §3.1 hint, lookback lines, `mlm start` warnings, and the carry marker. These are T3 and T4.
- Any change to pairing, the splice, or day and week totals.
- Fixing the double count. The overlap only flags it.
- `src/status.rs`, `src/week_view.rs`, `src/render.rs`.

**Files owned exclusively**: `src/anomalies.rs`, `tests/overlap_cli.rs` (new).

## T3 — Yesterday hint, lookback window and `mlm start` warning

**Spec citation**: §3.1, §3.2.2 (lookback lines, hint, `mlm start` warnings), §7 (§3.1 tests).

**Paragraph**: In the lookback module, extend today's scan to read yesterday and today, so the hint can tell whether a `stop` typed today already joins yesterday's start. Today's page prints the `[i]` hint for a lone open start from yesterday that stays unjoined, with the `or:` line only when today has no punches. Yesterday with two or more open starts prints the E7 lookback line with `(yesterday)`. Apply the clean-pair rule to all lookback lines. `mlm start`'s stderr gains the matching yesterday warning, with no `or:` line. Update `src/commands.rs` to the new finding shapes.

**Acceptance criteria** (exact lines from spec §3.1):

- Today's `status`, one open start yesterday, no punches today: the three-line `[i]` hint with the `or:` line.
- Today has a punch that does not join (a start): the `fix:` line only, no `or:` line.
- Today's first punch is a `stop` that joins (splice): no hint, and no age-1 false positive after the user runs the hint's own `fix:` command.
- Today's page shows a refused-join line naming yesterday: no hint.
- Two or more open starts yesterday: `[!] 2 open stints on Fri 2026-10-02 (yesterday)` followed by `fix: see mlm status 2026-10-02`. Lookback ages become 1-14 for this case. Age 2-14 lines are otherwise unchanged except for the clean-pair window.
- The hint is `[i]`: it does not mark a `mlm week` row, does not count toward the footer, and does not appear on `status DATE` for any date other than today.
- Placement: after today's anomaly block and lookback lines, before the carry marker line.
- `mlm start` warns with the two spec lines after a successful write when a single unjoined open start is dated yesterday. Yesterday with two or more open starts warns in the existing one-line form with `(yesterday)`. The just-written-date exemption and the best-effort read-failure rule are unchanged.
- Clean-pair rule on every lookback line, hint and warning: no later punch gives `(a time after S)`, a later punch gives `(a time between S and N)`. Follow-the-hint round trip (spec-review F1 guard): a stop at a time inside the window creates no overlap anomaly.
- The 15-day-old single start (age 15) is still absent from the lookback, as today.
- Full verification set, complexity at or under 15, coverage ratchet.

**Out of scope**:

- The overlap anomaly and the stale-start and E7 lines (T2).
- The carry marker, and any carry marker line in `mlm start`'s stderr (a spec non-goal).
- A change to the 2-day stale threshold, or live credit for a stint open across midnight.
- `src/status.rs`, `src/render.rs`, `src/anomalies.rs`.

**Files owned exclusively**: `src/lookback.rs`, `src/commands.rs`, `tests/start_hint_cli.rs` (new).

## T4 — Carry-in anomaly scan

**Spec citation**: §3.3, §7 (§3.3 tests).

**Paragraph**: Fill in the carry scan that T1 stubbed. Over the loader's bucketed punches, check every date with punches from the earliest data date through the day before the shown week. Use the shared per-date anomaly predicate with the same previous, day and next classification a `mlm week` row uses. Report the count of distinct earlier weeks with at least one anomalous date and the oldest such week. T1's renderings print the result in both commands, so this task changes no output code.

**Acceptance criteria**:

- An anomaly in a week older than 14 days raises the marker on `status` and on `mlm week`.
- No anomaly in earlier weeks gives no marker. A week with no earlier data gives no marker. A notes-only earlier week gives no marker.
- Several anomalous weeks give the plural form with the oldest week id. One anomalous week gives the singular form, naming that week.
- Each anomaly kind (E7, stale, E8, refused join, and overlap once T2 is merged) in an earlier week raises the marker. A date that is only an overlap counts, which matches the `mlm week` row `[!]`.
- The marker uses the same predicate as the row marker. A fixture checks that for every week the scan counts, `mlm week WEEK_ID` shows at least one `[!]` row.
- The marker does not depend on the shown week. It prints on `status` for any date and on `mlm week` for any week, including future weeks and the current week. It does not include the shown week's own anomalies.
- A week id before all data prints no marker.
- `status` and `mlm week` print the same line for the same shown week, and the marker line is absent when the shown week is the earliest.
- The scan makes no storage reads of its own.
- Output with no earlier anomaly is byte-identical to before.
- Full verification set, complexity at or under 15, coverage ratchet.

**Out of scope**:

- Changing rendering, line text or placement (T1 pinned them).
- A lookback window limit, a marker in `mlm start`, and any carry arithmetic change.
- Skipping or suppressing the marker once the carry nets to zero.

**Files owned exclusively**: `src/carry_scan.rs`, `tests/carry_marker_cli.rs` (new).

## T5 — CLI surface: log level, glossary, argument order, `week target` echo

**Spec citation**: §3.5, §4.1, §4.2, §4.3, §7 (§4.3 tests).

**Paragraph**: Make `Warn` the default log level and keep `--verbose` at `Debug`. Add the term glossary to `status --help` and `week --help`. Short `-h` keeps its text apart from clap's `(see more with '--help')` suffix. Make `week target` print its echo line. Rework `week target`'s help and usage to name `[WEEK_ID] <DURATION>`, and intercept the `week WEEK_ID target DURATION` shape with the decision F7 mechanism. This task is independent of the other tasks and starts at once.

**Acceptance criteria**:

- §4.1. A fresh database run without `--verbose` prints nothing on stderr (the migration line no longer shows), for example a first `mlm start`. With `--verbose` the migration line and debug lines appear.
- §3.5. A successful `mlm week target 20h` prints exactly `Week <current week id> target: 20h 00m` on stdout. `mlm week target 2026-41 20h` prints `Week 2026-41 target: 20h 00m`, and a zero or non-round duration uses the standard formatter. Failures (bad duration, bad week) still print nothing on stdout. `start`, `stop` and `note` stay silent.
- §4.2. `mlm status --help` and `mlm week --help` end with a glossary that defines worked, carry-in, fulfillment, the week line's `left to` / `over` figure with `by end of <weekday>` (Mon-Thu) against `<target> target` (Fri-Sun), `est. EOD` and `done for today`, `(incl. ongoing)` and `(+ ongoing)`, and `[!]` and `[i]`, in one or two lines each. Carry-in is a signed balance (shortfall negative, surplus positive). `-h` output is byte-identical to before except clap's `(see more with '--help')` suffix on the help line; no replacement help flag. A test checks each term is present in both long helps and that `done for today` appears, and that `target already met` does not.
- §4.3 help. `mlm week target --help` shows usage `[WEEK_ID] <DURATION>` and an Arguments section that names both, and `<ARGS>` appears nowhere in the help or in the missing-argument (E10) error.
- §4.3 interception:
  - `mlm week 2026-41 target 20h` prints exactly `error: put WEEK_ID after target: mlm week target 2026-41 20h` on stderr (no `Usage:` block), nothing on stdout, and exits `2`.
  - The week-id token and duration token are echoed as typed. The `w` alias and an interleaved `--verbose` also match.
  - These shapes keep clap's own error text and exit `2`: `mlm week 7 foo`, `mlm week 2026-41 target` with no duration, `mlm week 2026-41 target 20h extra`.
  - `mlm week target 2026-41 20h` (correct order) and `mlm week 2026-41` (view) are unaffected.
  - The detection is a pure function with unit tests that need no database.
- Existing `src/cli.rs` parse tests still pass. The hyphen-duration case (`week target -5h` reaches the duration parser) is unaffected.
- Full verification set, complexity at or under 15, coverage ratchet.

**Out of scope**:

- The glossary wording of anything that is not in the spec's §4.2 list.
- Any change to the clap definitions of other commands, to `status`'s argument handling, or to `src/status.rs`.
- Reshaping `WeekArgs` or `WeekTargetArgs` into separate week and duration positionals.
- Distinct exit codes for storage errors.

**Files owned exclusively**: `src/cli.rs`, `src/main.rs`, `src/week_target.rs`, and the new `tests/week_target_cli.rs`.

## T6 — Docs: SPEC.md and README.md

**Spec citation**: §5.1, §5.2, §5.3, §5.4, §6.

**Paragraph**: Rewrite the SPEC and README passages named in the spec so the docs state the new behavior and carry no pointer to the spec file or any working doc. Quote the literals pinned above. Re-locate each passage by content, not line number. Fold the accepted behavior of the removed Tier 3 items into the sections that own it, in plain statements without "issue" framing. Runs in parallel with every code task.

**Acceptance criteria**:

- SPEC §1.2a keeps only the counting half of I14. It is reworded to say a stint open across midnight gets no live credit, its minutes leave today's figures until `stop`, the week line jumps at 00:00, and the `[i]` hint points the user to `stop`. Items I1-I5, I23 (wording), I28, I31-I34 and the Tier 3 items listed in spec §5.1 are gone from §1.2a. Entries that shared a bullet with a removed item stay intact.
- Tier 3 behavior from spec §5.2 is stated once in its owner: the week line and Day total in §7.1, live minutes and carry in §2.4, `est. EOD` in §7.1, E7 in §7.3, `status DATE` in §7.1, the future week in §7.2. If a section already says it, nothing is added. I1's dropping out of today's lookback after 14 days is stated in §7.1.
- Spec §5.3's table is applied. Each changed section carries the new text:
  - the hint, lookback ages 1-14, the overlap anomaly, the `(a time between S and N)` hint times (every `a time after <start>` example in §7.3 and §7.4), the carry marker, read errors as exit `1` in §6 and §7.2, the `week target` echo, and the log level wherever `--verbose` is described;
  - `week WEEK_ID target DURATION` in §3.7 and §6.1, `done for today` in §7.1, and the future `status DATE` rule in §3.5 and §6.1 (the line saying `status` has no future restriction is replaced);
  - §3.7 and §7.4 drop `week target` from the silent commands;
  - flows E16, E18, E19 and any other §8 flow that touches hint times or `status DATE`.
- SPEC §6.3 stays "zero or nonzero" and states that runtime errors exit `1`. Usage errors from the argument parser exit `2`.
- SPEC records the visible consequences of spec §6 that survive. These are the persistent carry marker, the double count that the overlap line flags but does not fix, `done for today` next to `(incl. ongoing)`, and the `00h 00m left to ...` figure beside `done for today` at gap zero.
- README: the Possible issues section keeps only the I14 counting item. Usage and output examples follow §3 and §4, including the `week target` output and the "silent on success" claim (rewritten so that it is true: `start`, `stop` and `note` are silent and `week target` prints one line). The README/SPEC wording drift (validation report F8) is gone with the list. Every README sample of `target already met` on the Day total line becomes `done for today`. The phrase has no other location.
- AGENTS.md Layout lists the four new modules (`src/anomalies.rs`, `src/lookback.rs`, `src/history.rs`, `src/carry_scan.rs`) and the storage note.
- Self-contained check: a grep of SPEC.md and README.md for `docs/dev/plans`, `docs/dev/specs` and `possible-issues-triage` finds nothing.

**Out of scope**:

- Anything under `src/`, `tests/` or `examples/`, NOTES.md, the spec file.
- New worked examples beyond updating existing ones.
- The `--help` glossary text (T5 owns it).

**Files owned exclusively**: `docs/dev/SPEC.md`, `README.md`, `AGENTS.md`.

## Risks and spec points left open

- **Clean-pair window edge cases.** Settled in contract 7. The window includes N's minute, so it always holds a legal minute. Punches at the same minute as S are not "after S".
- **Moving `status` onto the shared loader** changes working code that was not broken. It is justified by the single source of truth for the marker and by the read-error fix. The existing 3,000 lines of `status` tests are the guard, so T1 must not edit their expectations beyond the two items named.
- **Persistent noise and retroactive effects** (spec §6) hold. A user with old data sees new `[!]` rows and the marker on first run. The docs task records this.
- **Marker scan cost.** The scan classifies every punch date of history on every `status` and `mlm week`, in memory after one read. This is accepted at personal-database sizes. Reviewers should look at the T1 and T4 timing on a seeded multi-year database.
- **Spec §3.1 and `status DATE`.** The hint is today's-page only, as the spec's "today's `status`" says. `status -1` and other dates never print it. Lookback and hint need no data when `DATE` is not today.
- **Glossary against the F2 correction.** The review caught a false carry claim in an earlier draft. The locked spec is right, and T5's wording must follow SPEC §1.3 and §2.4.
- **T1 size.** If T1's review shows it too large for one pass, the first fallback is to split off the loader and read-errors work as a first sequential commit inside the same worktree, not as a separate parallel task.
- **Spec §6.3 and exit codes.** The spec cites exit-code sections that only say "nonzero". The plan treats `1` as the code for runtime errors and `2` for parser-owned usage errors, which matches `src/main.rs` today and needs no SPEC §6.3 rewrite beyond the statement above.
