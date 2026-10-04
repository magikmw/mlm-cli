# Review: overnight-visibility spec

Spec under review: `docs/dev/specs/2026-10-04-overnight-visibility.md` (draft, baseline 6edd67b).

## Findings

### F1 (medium) Footer pointer leads to a page with no stop command

- Claim: spec:72 `[i] open stint: run mlm status DATE to stop it`, and spec:86 `run mlm status 2026-10-04 to stop it`.
- Evidence: `status DATE` for a non-today date runs no lookback and no hint (`src/status.rs:444-453`, `lookback::today_page_lines` is called only when `is_today`). SPEC.md §7.1 says "`status -1` shows yesterday's open start as a stint line only" (SPEC.md ~line 1450, E22). The yesterday page shows the line `(..., ongoing - duration as of right now, not a running total)` (`src/status.rs:246-251`) and nothing that says how to stop.
- Effect: the footer promises a stop path and the named page holds none. The page that carries the `fix: mlm stop HH:MM --date D` text is today's `mlm status` with no argument.
- Suggested fix: point the footer at `mlm status` (today's page, which holds the hint), or print the `mlm stop HH:MM --date D` form directly.

### F2 (medium) `[!]` precedence drops the stint from the week view, and the Monday line ignores that rule

- Claim: spec:67-68 "A row that has both an anomaly and the overnight stint shows `[!]` only." The footer fires "when any row in the table shows `[i]`" (spec:69). The Monday line fires "only when the overnight stint's date is in the previous week" (spec:89-91).
- Evidence: yesterday can carry a single open start plus an overlap or an orphaned end, so `has_any()` is true (`src/anomalies.rs:188-193`) while `lookback` still reports the hint (`src/lookback.rs:226-237` only counts `day.open`). Today's `status` then prints the hint and the new stint line (§3 trigger holds).
- Effect on a same-week yesterday: the row shows `[!]`, no `[i]` row exists, so no `[i]` footer prints. The week view says nothing about the open stint.
- Effect on Monday: last week's Sunday row shows `[!]` only, but the current week's Monday line still prints, since it has no row-marker precondition. The same case is handled two ways.
- Suggested fix: say whether the footer/Monday line depend on the row marker or on the trigger alone, and test the shared-row case through both paths. The spec's §8 only tests precedence on "a shared row".

### F3 (medium) The overnight line cannot come out of the existing stint builder in the stated position

- Claim: spec:48-49 the line goes "before today's own stints (it started earlier)".
- Evidence: `build_stint_lines` sorts all rows by start time (`src/status.rs:330`). A `22:00` open stint would sort after a `09:00` stint, so it cannot come from that function. The overnight stint also is not in `ctx.day` for today (it belongs to yesterday's `DayStints`), so `resolve` needs a second source. `StintLine` is a `Copy` struct with a fixed field set and `OpenStintAge::Yesterday` already renders different text (`src/status.rs:246-251`, `:128-139`). Reusing that variant would change `status -1` too, and the spec promises that page stays as is (spec:60-61).
- Also: the `StatusView.stints` doc comment says "already sorted by start time ascending" (`src/status.rs:167`); that becomes false.
- Suggested fix: name the new variant or field, say where the line is prepended, and update the doc comment.

### F4 (medium) Stale text beyond what §6 lists

§6 names §1.2a, §7.1, §7.2, the glossary and README. These other statements contradict the new behavior and are not listed:

- SPEC.md:1185-1186 (§7.3) "`[i]` is the quiet marker. It never marks a `week` row".
- SPEC.md:1450-1452 (E22) "The hint never marks a `week` row".
- SPEC.md:869-871 (§7.1 "The hint is not an anomaly. It does not mark a `mlm week` row").
- SPEC.md:311-312 (§3.5 status contents), which lists only the hint for today's page.
- SPEC.md §7.2 "`week` runs no lookback and no yesterday hint. The row marker, the footer and the carry-in marker below are its only anomaly output." `[i]` becomes a fourth output.
- `src/cli.rs:44-48` glossary (listed), but `src/lookback.rs:959-968` `hint_does_not_mark_the_yesterday_week_row` asserts the opposite of the new behavior and must be inverted. §8 does not mention it. `src/week_view.rs:896-903` (b11) only asserts `has_anomaly` is false, so it survives.
- SPEC.md §4.3.1/§7.1 (the "Yesterday hint" bullet) also lacks a mention of the new stint line.

Suggested fix: list the SPEC locations and the existing test to rewrite.

### F5 (low-medium) Data model for the week side is not stated, and it touches a pure renderer

- Claim: spec §5 adds a row marker, a footer line, a Monday line.
- Evidence: `render_week` is pure and takes `WeekView` only (`src/week_view.rs:75-83`). `WeekRow` and `WeekView` derive `PartialEq, Eq` and are built as struct literals in about a dozen tests (`row()` at `:261`, `example_a/b`, `a4`, `a5`). New fields change all of them.
- The Monday line names a date that is not among the seven rows, so `build_rows_from` (`:162-183`, rows only) cannot supply it. `build_week_view` must classify the previous-week Sunday from `history.buckets`. That works without a new read, because `load_history` walks from `min(earliest data week, shown)` with one day of padding (`src/history.rs:80-121`), and today's bucket is in range for a Sunday row. The spec does not say so, and it does not say that yesterday's row facts (`refused_join_with_next`, `open_stint_count`, `src/anomalies.rs:328-360`) equal the lookback trigger. They do: `refused_neighbor` is symmetric across the two views.
- Answer to the probe: yes, `mlm week` can evaluate the trigger from the loader's buckets with no new storage read, for the current week and for a past week ending yesterday. Spec should say it.

### F6 (low) §1.2a empty heading vs README removal

- Claim: spec:103 "§1.2a is left with its heading and an empty-list sentence", while spec:104-105 removes or empties README's Possible issues section.
- Evidence: SPEC.md:39 and `:56` reference §1.2a by number, and the heading explains itself ("Kept separate from §1.2", SPEC.md:60-62). An empty section is coherent as a place for future entries, but it is a different policy from README, and the two are asked to differ without a reason. AGENTS.md says SPEC.md must be self-contained, so a dangling empty section plus the §1.2 sentence "See §1.2a for that" is acceptable only if the sentence is kept true.
- Suggested fix: pick one (keep both as "none currently", or drop both and fix SPEC.md:39).

### F7 (low-medium) §6's list of moved behavior drops two statements

- Evidence: current §1.2a (SPEC.md:64-71) says there is no `est. EOD` for this stint, and that after Sunday midnight the closed week's `Worked` drops by the stint's minutes. §6 (spec:98-103) lists no live credit, minutes return on stop, the week line jump, and "listed and marked". It omits the missing `est. EOD` and the Sunday-midnight `Worked` drop. After §1.2a is removed those facts have no home.

### F8 (low) Spec narrates the discarded option

- Evidence: spec:8-9 "User chose to make it visible without counting it (option B)". The user's standing preference (`no-rejected-alternatives-in-docs`) is that specs do not narrate discarded options. "Option B" also has no referent inside the document, which AGENTS.md requires to be self-contained.

### F9 (low-medium) Testing gaps in §8

- No test for a past week viewed by id that holds yesterday (Sunday yesterday, `mlm week` for last week), nor for a future week (no marker, no footer).
- No test where today also has an open stint (two `-now` lines, one counted, one not).
- No test for a stint older than a day (`22:00` yesterday, `now` near midnight tonight: 47h-plus duration) or for the 2-day transition where the line disappears (see U4).
- No test that `has_any` is untouched for a yesterday row that carries both an overlap and the open start (F2).
- No end-to-end test through `resolve` for the new line (only `today_page_lines` and render tests are named).
- DST/midnight: `today` comes from `now.date_naive()` (`src/status.rs:420`, `src/week_view.rs:202`) and the elapsed minutes from `(now - start).num_minutes()` in UTC (`src/stint.rs:180`). Both are DST-safe, and the Monday rule is ISO Monday via `WeekId`, so it fires on Monday only. No finding, recorded as checked.

## Probe answers

- Trigger match: §3 matches `lookback` (`src/lookback.rs:215-249`, `:255-266`) and `status.rs:444-453`. Joined case yields no entry (`classify_at` consumes the start), refused join suppresses via `facts.refused_join_with_previous` (`:449`), today-has-punches only affects the `or:` line (`lookback.rs:182`) and is not a condition. Spec states this correctly. `Multiple open starts` give no hint (`lookback.rs:236`).
- Stint line vs column layout: `"  {:<11}  (..."` gives `22:00-now` plus 4 spaces, matching spec's example (`src/status.rs:233`, `:241`). The text is longer than any existing parenthetical but no layout rule is broken.
- Monday footer: fires only when today is Monday, because the previous-week condition holds only then. Matches the spec.

## Undeclared user-visible gaps

Each is a deliberate choice whose visible consequence the spec does not record. Candidates for known-issues.

- U1. Duplicate information on today's page. The `[i]` hint stays unchanged (spec:30-31) and the new stint line is added, so the same stint appears twice, in two sections, with two wordings ("still open (yesterday; not counted until stopped)" and "started yesterday, not counted until stopped").
- U2. Two phrasings for one stint across pages. Today's page says "so far, started yesterday, not counted until stopped". The page for yesterday keeps "ongoing - duration as of right now, not a running total" (`src/status.rs:246-251`) and Day total `(+ ongoing)` (`:202`). Today's Day total has no caption for the overnight stint.
- U3. Today's page with no punches now shows a Stint section (previously none, `src/status.rs:279`) under `Day total: 00h 00m`, with `so far` implying a running stint and no `est. EOD` (est. EOD stays `None`, `:457-461`).
- U4. The stint line vanishes at the 2-day mark. Once the start is two days old it is a `[!] unclosed start` lookback line (`src/lookback.rs:93-95`), and the stint line is gone from today's page, while the stint stays open. The spec's non-goal (spec:33-34) names the `[!]` case but not the loss of the line or the `[i]` week marker (the row then gets `[!]` through `stale_open_start`, `src/anomalies.rs:344`).
- U5. `[!]` hides `[i]` on a shared row (F2), so a user can see the stint on today's page and not on the week row.
- U6. Week line jump and the Sunday-midnight drop of the closed week's `Worked` persist and become undocumented once §1.2a is removed (F7). Only the `[i]` marker is new.
- U7. On Mondays the current week's view shows no marked row, only the footer; the marked row lives in last week's view (spec:113-114 records this) but the footer pointer in that view says "run mlm status DATE to stop it" (F1).
- U8. Scripts reading stdout. A reader summing today's stint list now gets a line whose minutes are not in Day total (spec:109 records the display effect, not the parsing effect). Same for `mlm week` output for any week containing yesterday (spec:115-116, recorded).

Consequence-free check: the work changes stdout for two commands, so this category is not empty. Exit codes and stderr (`mlm start` warnings, `src/lookback.rs:106-131`) are untouched.

## Verdict

ship-with-followups

Fix F1, F2 and F4 before implementation. The rest can be settled in the plan. No finding invalidates the design.
