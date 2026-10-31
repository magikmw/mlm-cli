# Plan review: orphan-stop-warning

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

Reviewed: `docs/dev/plans/orphan-stop-warning-plan.md` at `116c92c`, against the locked spec `docs/dev/specs/2026-10-24-orphan-stop-warning.md`, NOTES.md 111-114, and the source on branch `orphan-stop-warning`.

The plan's "Current shape" claims check out against source:

- `stop` never prints (`src/commands.rs:39-42`)
- `start` swallows lookback read failures (`src/commands.rs:56-58`)
- E8 and refused-join renderers are private and hardcode `[!]` and `(a time before T)` (`src/anomalies.rs:272-313`)
- the yesterday `or:` keys off two facts (`src/lookback.rs:183`)
- today's page filters through `overnight_stint` (`src/lookback.rs:299-311`)
- a splice lands in the earlier date's `completed` (`src/stint.rs:315-326`)
- punches are minute-granular (`src/storage.rs:102`, `:207-213`)

The spec §4.5 examples trace correctly through `classify`/`classify_at`/`is_refused_join`. The findings below are what is left.

## Findings

### F1. "D+1 is not read" contradicts itself and the single-definition rule. Medium, internal inconsistency.

- Claim, `plan:29`: "`D-1` through `D+2`, or `D-1` through `D+1` when `D` is today, so `D+1` is never read then." A range of `D-1` through `D+1` reads `D+1`.
- `plan:80` then requires "When `D` is today, `D+1` is not read."
- `plan:42` requires today's `status` to stay "the single definition of what `stop` prints".
- Evidence: today's `status` does read tomorrow. `history::read_range` extends through the shown week's end plus one day (`src/history.rs:115-119`). `live_for` classifies today "with its neighbours so a splice with tomorrow's orphaned end applies" (`src/history.rs:140-153`). `status` builds today's facts from `history.buckets.trio(today)` (`src/status.rs:470-471`), so its `next` bucket is tomorrow's real punches.
- Consequence: if `stop` skips the read, `D`'s facts use an empty `next` bucket, and they can differ from `status` whenever tomorrow holds a punch (clock or TZ change). The task agent cannot satisfy `:29`, `:42` and `:80` together.
- Fix: say "when `D` is today, `D+1`'s checks are skipped". Then decide in the plan whether `D`'s own trio reads tomorrow, matching `status`, or uses an empty `next`. If the empty `next` wins, record it as a deliberate divergence. Make the `:80` acceptance test the chosen behaviour.

### F2. Task 2 misses two sentences that state the yesterday `or:` condition. Medium, missing coverage.

- Spec §6 (`spec:256`) says the `or:` line rule in "SPEC §7.1, §7.3" gains the 12h condition.
- Task 2 updates the §7.1 condition (`plan:108`). Its §7.3 bullet (`plan:109`) covers only the E8 rules and the stop-hint consumer list.
- Evidence 1: SPEC §7.3 restates the condition on its own at `docs/dev/SPEC.md:1302-1303`: "The `or:` line prints only when no punch follows the open start on yesterday's date and today has no punches yet." Nothing in the plan updates it.
- Evidence 2: README restates it at `README.md:267-268`. The README bullet (`plan:112`) covers only the "joins the start" sentence (`README.md:268-269`) and the 15h 30m example. The condition sentence becomes false.
- Related: the README and SPEC `18:02` `[i]` examples (`README.md:262-264`, `docs/dev/SPEC.md:1296-1300`) still show the `or:` line. They hold only before 06:02 the next day, and neither states its clock.
- Fix: add both sentences to Task 2's acceptance list. Then either state a clock for the `18:02` examples or note that they assume a start under 12h old.

### F3. Docs that promise a silent first run are not on Task 2's list. Medium, missing coverage. Also a candidate known issue.

- Evidence 1: SPEC flow F13 (`docs/dev/SPEC.md:1460-1461`) says "First run on a fresh database prints nothing on stderr."
- Evidence 2: README says "That holds on the first run too, when the database is created" (`README.md:115-117`).
- Evidence 3: SPEC §3 says "a run that creates and migrates a new database prints nothing" (`docs/dev/SPEC.md:225-227`).
- On a fresh database, `mlm stop` as the first command writes an orphaned end, so under spec §3.1 it now prints the orphan block on stderr.
- Neither spec §9, the spec's interaction section (`spec:316-336`) nor Task 2 (`plan:104-114`) lists F13 or the §3 sentence. The README sentence sits in the paragraph Task 2 rewrites, but no acceptance line names the first-run claim.
- SPEC §4.3 (`docs/dev/SPEC.md:454-456`) also says anomalies are "detected and surfaced in that date's summary ... not rejected at insert time". It needs the same `stop` caveat that §6.2 gets.
- Fix: add F13, the §3 sentence and the §4.3 sentence to Task 2.

### F4. No acceptance test checks that a refused join between `D` and `D+1` prints once. Medium, testing gap.

- Risk 1 (`plan:122`) and spec §3.1 (`spec:92-94`) require a refused join between `D` and `D+1` to print once, as `D`'s line naming `D+1`.
- Evidence: the shared builder sets this join on both dates, as `refused_join_with_next` on `D` and `refused_join_with_previous` on `D+1` (`src/anomalies.rs:345-356`). A naive "print `D`'s lines, then `D+1`'s" prints it twice.
- No criterion in `plan:69-90` exercises this. The double-tap case (`plan:78`) has `D` = today, so it only covers the previous-date side.
- Fix: add a case. Seed yesterday `start 22:00`, and today `stop 01:00` plus `stop 02:00` (already refused). Then run `mlm stop 21:00 --date -1`. The exact stderr should show yesterday's orphan block, one refused-join line naming today, then today's two orphan blocks.

### F5. The planned read-failure test cannot catch the partial output that "reads first" prevents. Low to medium, testing gap.

- Claim: `plan:28` and `plan:87`; Risk 6 (`plan:127`) points at `start`'s pattern.
- Evidence: that pattern runs `DROP TABLE punches` before the warning pass (`src/commands.rs:1974-1988`), so every read fails at once.
- Both of `stop`'s reads (the date range and the lookback, `src/lookback.rs:227-231`) hit `punches`. A test built this way passes even if the code prints the date warnings after the first read and then fails on the lookback read. Spec §7's "all its reads first, then prints" exists to prevent exactly that.
- Fix: require a seam where the date read succeeds, the date warnings would print, and the lookback read fails. Then assert empty stderr.

### F6. Several stated behaviours have no acceptance test. Low, testing gap.

- "No carry marker" (`plan:32`, spec §4.4) has no criterion. Every `stop` fixture in `plan:71-87` would pass with a marker line, unless one seeds an anomaly older than this week.
- "No own-punch suppression" (`plan:32`) is likewise untested.
- The yesterday-hint threshold has no boundary case. `plan:58` pins it at "under 720 minutes", but `plan:89` tests only "under 12h" and "12h or more". The splice side gets exact 11h 59m and 12h cases (`plan:81`). The hint side should get 719 and 720 minutes too, because the two thresholds must agree (NOTES 113, F6).
- Fix: add the three cases.

### F7. The plan names only CLI tests as breaking, but in-module tests break too. Low, breaking tests not identified.

- Claim: Risks 2-3 (`plan:123-124`) list `tests/start_hint_cli.rs` only. `plan:90` covers the rest generically.
- `src/lookback.rs:739-745` `page_shows_three_line_hint_when_today_is_empty` seeds yesterday `18:02` with `now` at today `18:00` (`src/lookback.rs:338-343`). The start is 23h 58m old, so under the 12h rule the expected `HINT` (`src/lookback.rs:733-737`) loses its `or:` line and the test fails.
- `LineFacts` derives `Default` (`src/lookback.rs:45-53`), and `LookbackFinding::detail_lines()` renders with default facts (`src/lookback.rs:73-76`). Whatever default the new age field takes decides whether default-facts rendering keeps the `or:` line.
- That default also decides `src/lookback.rs:500-510`, which builds `LineFacts` by struct literal. The literal stops compiling once the field is added.
- The plan should pin the default, under 12h or not, so these fixtures change on purpose.
- `tests/week_target_cli.rs:131-141` `other_writers_stay_silent` runs `start 9:00` and then a bare `stop` with no TZ pin. Before 09:00 local time, the stop is an orphan and now prints a warning. The test checks stdout only, so it still passes, but its name now overstates what it checks. Risk 3's "must pin the time" applies to it.

### F8. A fact field nothing reads, plus two criteria that can only check that field. Low, scope creep / internal inconsistency.

- Claim: `plan:25` puts "whether `P`'s minute is inside the window" into the E8 facts.
- Contract `plan:59` says the inclusion rule is "never printed", and spec §1.2 (`spec:73-76`) confirms the hint text does not show it.
- So the flag has no consumer. Clippy's dead-code lint will not fire on a `pub` field, so the flag ships unread.
- The acceptance items "minute included" and "minute excluded" (`plan:88`) can only assert that unread field. At the output level they print the same `(a time between P and T)` as any other window.
- Fix: drop the flag from the facts. Test the inclusion rule as behaviour instead. A start at `P`'s minute should leave `T` paired when `P` closed a stint, and leave `T` orphaned when `P` is an orphan, checked through `classify`.

### F9. The spec changed after it was locked, and the plan cites the wrong decision range. Low, internal inconsistency.

- Evidence: commit `116c92c` edits spec §3.1 and §4.1, but the spec's Status line (`spec:4-8`) still says it was locked after two rounds and cites only entries 112-113.
- `plan:3` says "Decision log: NOTES.md entries 111-113", while Risks 1 and 4 rest on entry 114 (`plan:122`, `NOTES.md:1595-1605`).
- Fix: note the post-lock edits in the spec's Status line, and cite 111-114 in the plan.

### F10. Task 2 is told to copy the spec's known issues, which cite spec-local section numbers. Low, error-prone handoff.

- Claim, `plan:104`: "SPEC §1.2a replaces 'None currently.' with the six spec §1.2 items."
- Evidence: the sixth item ends "(§5)" (`spec:76`), meaning this spec's §5 (the E8 hint). Inside SPEC.md, §5 is "Week accounting — worked example" (`docs/dev/SPEC.md:542`). Spec §4.1 also cites "(§5)" (`spec:139`).
- Task 2 works from spec text alone and is told to copy text exactly (`plan:39`, `:56`), so it is likely to copy these pointers verbatim.
- Fix: tell Task 2 to retarget the working spec's own section references to the matching SPEC sections, which is §7.3 for the E8 window.

### F11. `src/main.rs` is not in either ownership list. Low, ownership gap.

- Evidence: `stop` is dispatched at `src/main.rs:51`. `start`'s best-effort pass takes an injected writer (`src/commands.rs:49-55`), so `stop` may do the same.
- If Task 1 changes `commands::stop`'s signature, `main.rs` must change, and no task owns it.
- Fix: add `src/main.rs` to Task 1, or pin `stop`'s public signature as unchanged.

### U1. The `status --help` glossary contradicts the duplicate-minute block. Undeclared user-visible gap.

- The glossary says an orphaned end means "Add a start before it, or delete it" (`src/cli.rs:41-43`).
- After this change, an orphan that shares its minute with another end prints a delete-only block, because no start pairs with it cleanly (spec §5, `spec:234-237`).
- Spec §9 keeps `--help` unchanged on purpose (`spec:313-314`, `plan:92`). The resulting contradiction between `--help` and the block is not recorded in spec §1.2.

### U2. A genuine overnight session of 12h or more re-warns on every later `stop` that day, and there is no fix. Undeclared user-visible gap.

- Spec §1.2 item 5 (`spec:70-72`) says a genuine 12h+ session warns.
- Item 2 (`spec:61-64`) says a warning repeats "until it is fixed".
- For a correct session there is nothing to fix. A `stop` to `A` or `A+1` checks the splice ending on `A+1` (`plan:31`). So every ordinary `stop` the user types on the morning after a long night re-prints the long-splice block all day.
- The only way to silence it is to change correct data. Neither item says that.

### U3. At the 12h boundary, the hint can offer `mlm stop` and the next `stop` then warns. Undeclared user-visible gap.

- The hint keeps its `or: mlm stop` line while the start is under 720 minutes old at `now` (`plan:58`). The splice warns at 720 or more.
- `now` is truncated to the minute (`src/main.rs:33-41`). A `status` at 11h 59m shows `or: mlm stop`, and a bare `stop` typed a minute later draws the long-splice warning.
- NOTES 113 states the goal as "mlm never suggests a command it then warns about". That holds per instant but not across the two commands, and nothing says so.

### U4. FE5 with a start 15 or more days old gives an orphan warning with no pointer to the cause. Undeclared user-visible gap.

- `stop`'s lookback uses the 14-day window (`src/lookback.rs:17-18`) and prints no carry marker (spec §4.4).
- So a bare `stop` against a start 15+ days old prints only the orphan block. Its `or:` line suggests deleting the end, and nothing names the old start.
- Spec §1.2 item 3 (`spec:65-67`) covers only a backdated `stop` beside an old written date, not today's FE5 against an old start.

### U5. The delete-only `fix:` cannot tell two identical `end` rows apart. Undeclared user-visible gap.

- For `start 09:00`, `stop 17:00`, `stop 17:00`, one 17:00 end pairs and one is orphaned. Because another end shares its minute, `status` prints `fix: mlm delete punch --date D  (lists punches; rerun with the end's number)`.
- The list then shows two rows reading `end 17:00` (`src/commands.rs:137-147`), so "the end's number" does not identify one.
- Either row works, but no doc says so. Neither the spec nor the plan records this.

### U6. Past 12h, the only hint left for a still-working user ends the session on yesterday. Undeclared user-visible gap.

- From 12h on, the `[i]` hint keeps only `fix: mlm stop HH:MM --date <yesterday>  (a time after S)` (spec §6).
- A user who is genuinely still working has no suggested way to end the session now. Following the remaining hint caps the stint before yesterday's midnight and drops today's minutes, unless they pass a bare `stop` and accept the warning.
- Spec §1.2 item 5 records that the `or:` line is dropped, but not that the remaining suggestion truncates a genuine session.

## Verdict

ship-with-followups

The task split, the ownership of the code files and the fact that Task 2 can run from spec text alone all hold. Fold F1-F4 into the plan before the worktrees open:

- F1 leaves Task 1 with requirements it cannot meet together.
- F2 and F3 leave false sentences in SPEC and README.
- F4 leaves the one dedup rule untested.

The rest can be folded at the same time or carried forward. U1-U6 are candidates for spec §1.2, and choosing which to accept is the coordinator's call.
