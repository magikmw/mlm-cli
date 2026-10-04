# Possible-issues triage spec review

Spec under review: `docs/dev/specs/2026-10-03-possible-issues-triage.md` (draft, baseline `main` @ be85617).
Checked against `src/status.rs`, `src/stint.rs`, `src/render.rs`, `src/week.rs`, `src/week_view.rs`, `src/commands.rs`, `src/week_target.rs`, `src/cli.rs`, `src/main.rs`, `src/date.rs`, `src/storage.rs`, SPEC.md, README.md, NOTES.md entry 96 and the validation report.

Method note. The sealed box could not resolve a path dependency on the repo, so the LIFO claims below come from reading `stint.rs:205-245` (`close_group`) line by line, not from a run. The logic is small (one stack, one pop per `end`) and I traced each example by hand.

## Findings

### F1 (high) The §3.2.2 safe-time rule is inverted and produces the overlap it adds

- Claim. spec:111-122. `T` is the latest punch on the date at or after the open start. "A stop later than every punch after the open start pops that start (§4.3), while a stop between them pairs with a later start and nests."
- Evidence. `stint.rs:215-224` pops the stack top on every `end`. Take `start 09:00, start 10:00, stop 11:00` (09:00 stays open, 10:00-11:00 closed), so `T` = 11:00.
  - `stop 12:00` (after `T`). Sorted scan: push 09:00, push 10:00, 11:00 pops 10:00, 12:00 pops 09:00. Stints are 09:00-12:00 and 10:00-11:00. That is a nested pair, exactly what §3.2.1 flags.
  - `stop 09:30` (between the open start and the next punch). 09:30 pops 09:00 before 10:00 is pushed. Stints are 09:00-09:30 and 10:00-11:00. Disjoint, no overlap.
  - `stop 10:30` (inside). Pairs 10:00-10:30, then 11:00 pops 09:00. Nested again.
- So the rule is backwards. Whenever a later punch exists, "a time after T" always nests the target start around every later closed stint, and the hint walks the user into the new `[!] overlapping stints` line. The only nest-free stop time is strictly between the start and the next later punch. The spec's sentence "while a stop between them pairs with a later start and nests" is only true for a stop after the first later start.
- The spec's own sentence that "common output does not change" when nothing follows is right. When something follows, the output changes to a hint that is wrong.
- Same flaw for E7 (`start 9, 10, 11, end 11:30` leaves `[9,10]` open, `T` = 11:30, `stop 12:00` makes 10:00-12:00 contain 11:00-11:30).
- Fix direction. Either name the window `(start, next later punch)` when one exists, or say plainly that no clean stop exists and point at `delete punch`. Add a test that follows every hint and asserts no overlap anomaly results.

### F2 (high) Glossary text states a false carry rule

- Claim. spec:189-190. "carry-in (shortfall carried from earlier weeks; surplus never carries)".
- Evidence. `week.rs:185-187` `carry_out = fulfillment - target`, unclamped, and `week.rs:202` feeds it forward. `week.rs:55-58` ("nothing is clamped"). SPEC §1.3:208-212 ("positive if it exceeded target"). README:19 ("surplus carries into the next week"). SPEC §7.1 example (carry-in +06h 55m). `render.rs` tests `t37_carry_in_surplus_worked_example`, `t11d`.
- The glossary would ship a definition the product contradicts. §4.2 also says wording is "checked against §7.1/§7.2", but this bullet is wrong at the spec level already.

### F3 (medium) The overlap rule omits open stints, so I2's most common shape stays silent

- Claim. spec:94-109 and non-goal spec:38-40. Overlap means "a stint whose range lies inside another stint". `DayStints.completed` is the only list that has ranges.
- Evidence. `start 09:00, start 10:00, stop 11:00` leaves 09:00 open (`stint.rs:172-182`) and 10:00-11:00 closed. Today's total is `completed_minutes() + live` (`status.rs:646`, `stint.rs:116-118`), so 10:00-11:00 is counted twice (once in completed, once inside live from 09:00). No E7 (one open start), no stale line. The spec never says whether an open stint can be the outer member.
- Same gap for the spliced case. `classify_at` (`stint.rs:315-327`) builds a completed stint `[open start, next-day end]`. That stint can contain completed stints on date A that sit above the open start on the stack (e.g. `start 22:00, start 22:30, stop 23:00` plus a next-day first-punch orphan `end 00:30`). Spec:108-109 says spliced stints are checked "on each date with the part that falls on that date". On A+1 the part is `[00:00, end]`, and `end` is A+1's first punch, so nothing on A+1 can overlap it (the clause is dead). On A the part is `[start, 24:00)`, which is where a real overlap can occur, and the spec does not describe it.
- Fix direction. State that an open stint is a range `[start, now]` (or `[start, end of date]`) for the check, or state that open-outer overlaps are out of scope and keep a line about it in §1.2a. Drop or rewrite the A+1 clause.

### F4 (medium) The spec cites exit codes that do not exist

- Claims. spec:158-160 "exits with the storage-error code (§6.3)"; spec:208-209 "the usage-error exit code (§6.3)"; spec:223-224 "same message shape and exit code" as `start`/`stop`/`note`.
- Evidence. SPEC §6.3 (SPEC.md:691-695) defines only `0` and "nonzero". `main.rs:67-78` and `lib.rs:21-27` return `1` for every `anyhow` error ("One nonzero exit code for every hard error"). Only clap-owned errors exit `2` (`main.rs:14-15`; validation report I32 saw rc=2).
- Consequences. A storage failure in `mlm week` exits 1, there is no "storage-error code". A custom §4.3 message exits 1 if raised through `anyhow` and 2 if raised through a clap error, and the spec does not pick. §4.5's "same exit code" as `start --date future` is 1 (`date.rs:161-167` through `anyhow`), which is not the "usage-error" 2 of §4.3. The spec needs either "exit 1 like every hard error" or a decision to introduce distinct codes (which is a SPEC §6.3 change it does not list).

### F5 (medium) §3.1's age-1 hint needs today's punches, and the shared lookback does not read them

- Claim. spec:53-55, 69-74. A hint fires when yesterday has one open start "that the splice (§4.3.1) does not join to today". Window becomes ages 1-14.
- Evidence. `status.rs:435-437` reads `[today-15, today-1]` only. `lookback_finding_for_date` (`status.rs:385-412`) takes `next` from that same map. For age 1, `next` = today is never loaded, so `classify_at(prev, cur, [])` cannot splice. After the user runs `mlm stop 00:30` (the spec's own `fix:`), the next `status` and the next `mlm start` would still print the hint (false positive). The shared function also serves `mlm start` (`commands.rs:56`), so the bug lands in both places.
- Also unmentioned. `LookbackFinding`/`LookbackKind` assert ages `2..=14` (`render.rs:314-317`, `349-352`) and carry no "yesterday" shape; the single-open-start variant has only `{time, punch_id}`, so the `T` from F1 and the `[i]` vs `[!]` split need new fields.
- Fix direction. Widen `window_end` to `today` (and keep ages 2-14 from changing), and list the type changes.

### F6 (medium) §3.4 does not choose a mechanism, and the choice is the whole cost

- Claim. spec:153-160. Three reads in `mlm week` become hard errors.
- Evidence. `WeekData::earliest_data_week`, `WeekData::worked_minutes` and `WeekTargets::target_override` are infallible (`week.rs:33-53`), and `DbWeekData` collapses errors at `week_view.rs:271-275`, `283`, `315-318`. Two ways out: change both traits to return `Result` (touches `week_series`, `week_accounting`, `WeekLedger`, all `week.rs` tests, `status::resolve`), or prefetch a `WeekLedger` the way `status::build_ledger` does (`status.rs:517-575`). The second changes `mlm week` from one range query per week to four queries per day over the whole walk (`status.rs:546-563`), a visible slowdown on a multi-year database. The validation report (F4) flagged exactly this; the spec drops it and gives no decision or test list.
- Also missing. No stated test for each of the three reads failing separately (rename `week_targets`, rename `punches`, a failing `earliest_data_date`), and no statement on whether the printed message is the raw rusqlite text (`status` prints `error: no such table: week_targets`, `week` prints `error: database error: ...`; the validation report shows the two differ).

### F7 (medium) §4.3 under-specifies how to intercept `week WEEK_ID target DURATION` and what the help shows

- Claim. spec:202-210.
- Evidence.
  - Today clap rejects this input itself, before any of our code runs (`cli.rs:173-182`, `args_conflicts_with_subcommands`; validation report I32: `error: the subcommand 'target' cannot be used with '[WEEK_ID]'`, rc 2). The spec's message can only be produced by catching `ErrorKind::ArgumentConflict` from `Cli::try_parse()` and re-reading argv, or by reshaping `WeekArgs` (e.g. a `Vec` positional). The second approach changes error text for other junk (`mlm week 7 foo`). The spec states neither approach, nor what happens for `week 2026-41 target` (no duration), `week 2026-41 target 20h extra`, or `week target 2026-41 20h` with the arguments already right.
  - `override_usage` only rewrites the `Usage:` line. `cli.rs:205-210` keeps `value_name = "ARGS"`, so `week target --help` still prints an `Arguments:` section with `<ARGS>...`, and the missing-argument error (E10, SPEC.md:1220-1221) still says `<ARGS>...`. Spec:205-206 claims the help shows the new line "in place of `<ARGS>...`", which is only half true.
  - Whether the message prints clap's trailing `Usage:` block is unspecified.

### F8 (medium) §3.3 has no single place to collect anomalies, and a third `DateAnomalies` builder is implied

- Claim. spec:124-151 (the walk "from the earliest week with data").
- Evidence.
  - "Earliest week with data" in code is the min over punches **and notes** (`storage.rs:501-515`, `status.rs:524-528`, `week.rs:127-130`), so a notes-only first week counts. That is harmless (notes cannot be anomalies) but the spec should say it.
  - `status::build_ledger` (`status.rs:546-563`) already visits every date, but classifies with `classify_at` (post-splice). Stale, refused-join and overlap facts need the raw `classify` of prev/day/next (`status.rs:318-330`, `337-370`) plus `today`, so the loop needs a second classification per day.
  - `mlm week` has no such loop at all. Carry is computed through the infallible `DbWeekData::worked_minutes` (`week_view.rs:278-310`), which sees only minutes. The marker needs a separate scan, so `mlm week` pays for a full second walk.
  - Anomaly facts are already built in two places (`status.rs:337-370` and `week_view.rs:186-236`), with a comment noting the duplication risk (`status.rs:20-26`). The spec adds new facts (overlap, `T`) and a third consumer (the carry walk) and does not say to share one builder. `render::DateAnomalies::has_any` (`render.rs:181-187`) also needs the overlap field, or the row `[!]` and the block disagree.
  - Far-future `mlm week` ids walk every week from earliest to the shown week already (`week.rs:162-207`). Adding per-date classification for every week multiplies that cost unless the scan skips weeks with no punches (one range query).

### F9 (medium) Documentation and test scope is thin

- Claim. spec:226-261. SPEC changes are §1.2a, the Tier 3 restatements, README. §3.7 and §7.4 are named in §3.5, §6.1 in §4.5.
- Evidence. The new behavior also edits SPEC §7.1 (hint line, lookback window text at SPEC.md:865-869, 856-857), §7.3 (new line kinds, the fixed-order sentence at SPEC.md:1002-1006, the three stop-hint examples that show `a time after <start>` at 1022, 1030, 1058), §7.2 (carry marker, footer layout at SPEC.md:980-989), §4.3 (edge-case list, overlap), §7.4 (start warnings), §8 flows E16/E18/E19 (SPEC.md:1244-1268), §3.5 (future date), §3.6. None is listed. AGENTS.md requires SPEC to be self-contained and the changeset to fold content in.
- No test plan or acceptance list exists in the spec. "Exact wording is a task-plan item" covers only the glossary. The pre-commit gate ratchets coverage, so new branches (overlap, hint, marker) need named tests.

### F10 (low) Overlap definition leaves shapes undefined

- Claim. spec:94-97 "lies inside another stint ... only touch at an endpoint are not overlaps".
- Evidence. LIFO cannot produce partial overlaps (it is parenthesis matching, `stint.rs:205-245`), so "inside" is the only real shape. But two reachable nested shapes share an endpoint and the wording does not say which side they fall on.
  - `start 09:00, start 09:00, stop 10:00, stop 11:00` gives 09:00-10:00 inside 09:00-11:00 (shared start; a real double count of one hour).
  - `start 09:00, start 10:00, stop 11:00, stop 11:00` gives 10:00-11:00 inside 09:00-11:00 (shared end; same-instant group, `stint.rs:215-224`).
  - `start 09:00, start 09:00, stop 10:00, stop 10:00` gives two identical ranges. "Inside" holds both ways, so it is unclear whether that is one pair or two.
- "One anomaly line per nested pair" grows quadratically for deep nesting (3 levels give 3 lines, 4 give 6), and the inner of a 3-level chain "counts" three times, not "twice".
- The fix line says "rerun with the extra punch's number" but does not say which punch is extra.

### F11 (low) Smaller inconsistencies

- spec:20-24, 69-75 vs `status.rs:359`/`365`. Where yesterday has a lone open start and today has an orphaned end that is not the first punch, today's page already prints the existing refused-join line (`status.rs:365-368`, `stint.rs:394-402`) and would now also print the `[i]` hint. The spec says only that the hint appears when "the splice does not join" and does not address the duplicate.
- spec:61-62. The `or:` line offers `mlm stop` but no `delete punch` alternative, unlike the age-2+ stale block (`render.rs:236-239`). A user whose "open" start was a typo has only the `stop HH:MM` route.
- spec:116-122, `render.rs:216-220`. The E7 parenthetical `a time after {latest} closes the {latest} start` names the same time twice. With `T` replacing the first, the sentence reads "a time after 11:30 closes the 10:00 start". The spec does not give the new text. `DateAnomalies` (`render.rs:130-164`) has no field for `T` at all.
- `T` can be 23:59, leaving no legal later `TIME` (24:00 is rejected, SPEC.md:666-667). The rule needs a fallback sentence.
- spec:4-9, 13. Entry 96 (NOTES.md:1272-1297) says "34" is "32 items" and records I4 (past-week anomaly skewing carry) as "Deferred, kept in §1.2a". The spec fixes I1 and I4 instead (§3.3, §5.1). The spec is the later and more complete document, but no NOTES entry records the scope change.
- spec:180-181 vs 170-172. §4.1 says README's "silent on success" claim then holds on first run, while §3.5 makes `week target` print on success, which breaks the same README sentence (README.md:227). §5.3 does not name this edit.
- spec:29 (table) marks I1 fixed. The marker (§3.3) appears on `status` and `week` only. `mlm start` still stops mentioning an unclosed start after 14 days and gets no marker (see U3).

## Undeclared user-visible gaps

Each item is a deliberate choice whose visible consequence the spec does not record. Candidates for a known-issues entry, not defects.

- U1. Retroactive effect on existing data. Overlap detection (§3.2.1) and the carry marker (§3.3) apply to all stored history. After upgrade, any past date with a nested pair or an old orphaned end lights `[!]` on its `mlm week` row and puts the `[!] carry-in includes anomalies ...` line on every later `status`/`week`. The spec describes new behavior only for new data.
- U2. The carry marker is permanent noise. It prints on every `status` (any date) and every later `mlm week` until the old anomaly is repaired. It also fires for anomalies that change no minute (a refused join, an E8 whose missing time was never worked) and when the carry-in nets to `00h 00m`. `status` never prints a "Carry-in" figure for a closed or future week (it shows `Total behind/ahead`, `render.rs:86-96`) and prints it for the current week only in the expanded parenthetical, so the line names a figure the reader of `status` often cannot see. Repair needs `delete punch` on old data.
- U3. `mlm start` gets no carry marker and, for anything older than 14 days, still says nothing (spec:76-88 covers only the age-1 case). §5.1's "I1 fixed" holds for `status` and `week` only.
- U4. `done for today` (§4.4) prints next to `(incl. ongoing)` and next to a live stint when the user has worked nothing yet. With a surplus carry-in on Monday, the gap is already `<= 0` the moment a stint opens (`status.rs:581-584`), so a fresh `mlm start` shows `Day total: 00h 00m (incl. ongoing), done for today`. A zero-target week shows it all week. The wording also reads as an instruction to stop. At exactly `gap == 0` the week line still says `00h 00m left to ...` (`render.rs` `gap >= 0`) while the Day total says `done for today`, so I23's disagreement survives the rename. The spec removes I23 from §1.2a without restating that.
- U5. Raising the default log level to `Warn` (§4.1) removes the only visible sign that a fresh database was just created. A typo in `MLM_DB_PATH`, or a first run in the wrong directory, now creates an empty database in silence.
- U6. Rejecting a future `status DATE` (§4.5) removes the way to preview a later date inside the current week. `mlm week FUTURE_WEEK` still prints `Total behind: <full target>` for a week that has not started (validation report F5), so I29's figure is still reachable; §5.2 files it under §7.2 as accepted behavior without saying it survives.
- U7. The `fix:` hint text changes for every existing stop hint (§3.2.2). Output that SPEC §7.3 and README show (`a time after 09:00`) now prints a later time whenever other punches follow, with no mention that the old hint was one the user could follow cleanly (see F1).
- U8. The `[i]` hint (§3.1) appears on every `status` after midnight for a legitimate overnight session, and the `or: mlm stop` line is suggested on every run until the user stops. The text "not counted until stopped" is declared, but the repeated nag for working users is not. `status` on the working user's own yesterday page shows no equivalent.
- U9. After the overlap anomaly (§3.2.1) the day and week totals still double count. Non-goal spec:38-40 says so, but §5.1 removes I2 from the known-issues list as "fixed", which drops the only note that the minutes stay wrong until a punch is deleted.
- U10. `mlm week target` now writes to stdout (§3.5). Scripts that relied on a silent success, and the README sentence that says all four write commands are silent (README.md:227), change. §3.5 states the output but not that it is a compatibility change.

## Checks that held

- `earliest_data_date` and `.ok().flatten()`/`unwrap_or_default` sites exist as described (`storage.rs:501-515`, `week_view.rs:271-283`, `315-318`); the "before" claim in §3.4 is accurate.
- `log::info!`/`warn!` are not used anywhere in `src/`, so the `Warn` default hides only the migration line (`main.rs:19-24`).
- clap can produce `Usage: mlm week target [OPTIONS] [WEEK_ID] <DURATION>` with `override_usage` (see F7 for what it leaves behind). `after_long_help` keeps `-h` unchanged.
- `resolve_future_checked_date` already exists (`date.rs:161-167`) and `status` calls the plain `resolve_date` (`status.rs:620`), so §4.5 is a one-line swap plus docs.
- Sort order `E7, stale, E8, refused join` matches `render.rs:195-202`; the overlap slot (§3.2.1) fits between E8 and refused join without disturbing it.

## Verdict

needs-rework

F1 and F2 are wrong statements about the code that would ship as wrong behavior (a hint that leads to the new anomaly, a false glossary line). F3-F9 leave mechanisms or exit codes undefined in ways that change cost or output. The rest can follow in a revision pass.
