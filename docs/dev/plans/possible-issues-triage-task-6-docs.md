# Task 6 plan: Docs (possible-issues-triage)

> **Archived — historical planning record.** Not authoritative. Current spec: `docs/dev/SPEC.md`. Design/process history: `docs/dev/NOTES.md`.

## What I read

- `docs/dev/plans/possible-issues-triage-plan.md` (full): architecture, F6/F7/F8 decisions, global constraints, interface contracts 1-9, T1-T6 sections.
- `docs/dev/specs/2026-10-03-possible-issues-triage.md` (full, locked): §3 fixes, §4 polish, §5 documentation (5.1-5.4), §6 visible consequences, §7 testing.
- `docs/dev/plans/reports/possible-issues-triage-validation-review.md` (full): item ids I1-I34, README/SPEC drift table (F8).
- `docs/dev/SPEC.md` (full, 1269 lines) and `README.md` (full, 555 lines): every passage edited below was re-read at its current line.
- `AGENTS.md` (project instructions, Layout and Documentation sections), `clippy.toml`, `.githooks/pre-commit`.
- `src/cli.rs:11-13` (global `-v`/`--verbose`), `src/main.rs:15-23` (log level today), `src/week_view.rs:100-120` (footer sits between the row table and the Carry-in block), `src/render.rs:375` (footer constant), `src/date.rs:123-161` (future-date checked resolver exists), `tests/` listing.
- Sibling task plans: `docs/dev/plans/possible-issues-triage-task-1-foundation.md` (module names and responsibilities, the `mlm week` carry-marker layout) and `docs/dev/plans/possible-issues-triage-task-5-cli-surface.md` (glossary, `week target` help/usage/error texts, echo line, log level). T2-T4 have no plan yet. This plan quotes the literals pinned by changeset-plan contracts 6-8 and spec §3-§4. Only the AGENTS.md Layout text uses a sibling's names (T1's four modules, `week_targets_in_range`, the render helpers).

## Scope

This task owns exactly three files.

- `docs/dev/SPEC.md`
- `README.md`
- `AGENTS.md`

Files this task must not touch.

| File or area | Owner |
|---|---|
| everything under `src/`, `examples/`, `tests/` | T1 (`status`, `week_view`, `render`, `stint`, `lib`, `week`, `storage`, `date`, new `anomalies`, `lookback`, `history`, `carry_scan` stub, `tests/status_cli.rs`, `tests/e6_db_failure.rs`, `tests/ascii_sweep.rs`, `tests/week_read_errors.rs`) |
| `src/anomalies.rs`, `tests/overlap_cli.rs` | T2 |
| `src/lookback.rs`, `src/commands.rs`, `tests/start_hint_cli.rs` | T3 |
| `src/carry_scan.rs`, `tests/carry_marker_cli.rs` | T4 |
| `src/cli.rs`, `src/main.rs`, `src/week_target.rs`, `tests/week_target_cli.rs`, the `--help` glossary text | T5 |
| `docs/dev/NOTES.md`, the spec file, `coverage-baseline.json` | coordinator |

No file is edited by this plan itself.

## Dependencies

| Dependency | Landed? | What this task uses | Stub to write against today | Swap at integration |
|---|---|---|---|---|
| T1 (carry marker rendering, `done for today`, future `status DATE`, module names) | no | literals of changeset-plan contract 6, spec §3.3, §4.4, §4.5; the four module file names (contract 1) | the spec's literal blocks, copied verbatim | coordinator greps each quoted literal in `src/` and `tests/` (Test plan 1.7) |
| T2 (overlap line, windowed E7/stale hints) | no | spec §3.2.1 and §3.2.2 literals | same | same |
| T3 (yesterday hint, lookback ages 1-14, `mlm start` warning) | no | spec §3.1 literals | same | same |
| T4 (carry scan) | no | spec §3.3 conditions | same | same |
| T5 (`week target` echo, argument-order message, log level, glossary) | no | spec §3.5, §4.1, §4.3 literals. The glossary wording is not quoted. | same | same |

Every non-trivial block below is marked `[VERIFY AT MERGE]` where it quotes program output. The implementer of this task re-pulls those strings from the merged binary or from the merged tests before landing, as in earlier docs tasks. Where this plan and the merged binary disagree, the binary wins and the deviation goes in the task report.

## Facts gathered

SPEC.md current anchors (line numbers at baseline `cccfb8f`; re-locate by heading and content).

| Anchor | Lines | Fact |
|---|---|---|
| Header status | 7-8 | `draft, complete through section 8` (unchanged) |
| §1.2a | 58-189 | 34 bullets, one per item, in I1-I34 order. No bullet merges two items. |
| §2.4 Fulfillment / carry bullet | 291-312 | ends "...stays display-only. An **orphaned `end`**..." |
| §3 preamble | 334-339 | no mention of `--verbose` or log level anywhere in SPEC or README |
| §3.5 | 392-411 | no future-date text of its own |
| §3.7 | 429-435 | says nothing about output |
| §4.3 nested-entry paragraph | 512-515 | "...without treating it as malformed." Becomes false once overlap is an anomaly. |
| §4.3 edge-case list | 517-541 | five bullets |
| §4.3.1 last paragraph | 589-593 | ends with a pointer to `docs/dev/plans/reports/boundary-context-cues-*`. This fails the self-contained grep and must go. |
| §6.1 `--date` bullet | 652-654 | "`status`'s `DATE` argument (§3.5) has no such restriction." |
| §6.3 | 691-695 | "Nonzero on any §6.1 hard error." |
| §7.1 est. EOD bullet | 782-804 | `target already met` at lines 792-793 |
| §7.1 week line bullet | 805-836 | ends with the Friday sample |
| §7.1 stint lines / lookback bullets | 838-869 | line 856-859 points at §1.2a; line 865-869 is the lookback bullet |
| §7.2 | 885-989 | row-marker bullet 976-979, footer bullet 980-989 |
| §7.3 | 991-1077 | order paragraph 1002-1006, fix-command details 1008-1014 |
| §7.4 | 1079-1117 | silent list at 1081, `start` lookback 1090-1113 |
| §8 | 1119-1269 | `target already met` at 1157, 1167, 1170; E15 line 1238 and E18 line 1260 point at §1.2a |

README.md anchors: Features `week target` bullet 18-19; Possible issues 43-160; Usage silent claim 227-231; `start` stderr 260-273; status 298-410 (Day total sample 308, `target already met` once); "(see Possible issues)" at 366-367 and 450-451; `week target` 453-480, "Silent on success" at 459.

AGENTS.md Layout: lines 33-49, one bullet per module.

Grep baselines to drive the checks (run before editing).

- `grep -c "target already met" docs/dev/SPEC.md README.md` gives SPEC 7 hits (§1.2a x4 lines, §7.1 line 793, §8 three lines), README 5 hits.
- `grep -n "docs/dev/plans\|docs/dev/specs\|possible-issues-triage" docs/dev/SPEC.md README.md` gives exactly SPEC line 593.

## Edit order and method

Edit SPEC.md from the bottom up so earlier line numbers stay valid, then README.md the same way, then AGENTS.md. Match each passage by its quoted content (Edit tool `old_string`), not by line number. All new text is plain ASCII, short declarative sentences, no em dashes (the existing docs have many, new text adds none).

## SPEC.md edit list

Item ids are those of the validation review. "Drafted" means the replacement text is below.

### S1. §1.2a, remove 33 bullets, keep one (spec §5.1)

Keep the section intro paragraph ("Shipped, working-as-designed behavior ... a possible issue works but has a rough edge."). Delete every bullet from "An unclosed start more than 14 days old shows only on its own date's" down to the "first run against a new database logs a line" bullet. Add one bullet, drafted:

```
- A stint started yesterday and still open after midnight is not on
  today's page, so it gets no live credit and no `est. EOD`. Its minutes
  leave the day total, `Worked`, `Fulfillment` and the week line until
  `stop`. At 00:00 the week line jumps, because `required` steps up a
  weekday while `live` drops to 0. When this happens past Sunday
  midnight, the closed week's `Worked` drops by those minutes. Today's
  `status` prints an `[i]` hint for the open start that points to `stop`
  (§7.1).
```

Removal ledger. Each id and where its behavior now lives.

| Id | Removed because | Lives in |
|---|---|---|
| I1 | wrong carry-in now marked; 14-day drop-out stated | S10 (lookback bullet, marker bullet) |
| I2 | overlap anomaly | S7, S12 |
| I3 | yesterday hint | S10, S12, S13 |
| I4 | carry marker | S10, S11, S12 |
| I5 | future `status DATE` rejected | S5, S8 |
| I6 | Tier 3 | S10 week-line paragraph |
| I7 | Tier 3 | S10 est. EOD paragraph |
| I8 | Tier 3 | S3 |
| I9 | Tier 3 | S10 est. EOD paragraph |
| I10 | Tier 3 | S10 est. EOD paragraph |
| I11 | Tier 3 | S10 est. EOD paragraph |
| I12 | Tier 3 | S12 (E7) |
| I13 | Tier 3 | S12 (E7) |
| I14 | counting half kept, reworded | the kept bullet |
| I15 | Tier 3 | S10 est. EOD paragraph |
| I16 | Tier 3 | S10 week-line paragraph |
| I17 | Tier 3 | S10 week-line paragraph |
| I18 | Tier 3 | S10 week-line paragraph |
| I19 | Tier 3 | S3 |
| I20 | Tier 3 | S10 week-line paragraph |
| I21 | Tier 3 | already stated at §7.1 Day total bullet (`(incl. ongoing)` and `(+ ongoing)`). Nothing added. |
| I22 | Tier 3 | already stated at §7.1 Day total bullet ("including one typed for a time later than now (`live` 0, nothing added)"). Nothing added. |
| I23 | wording | S10 est. EOD paragraph (`done for today`) |
| I24 | Tier 3 | S3 |
| I25 | Tier 3 | already stated at §7.1 week-line bullet ("A zero-target week follows the same rule, e.g. ..."). Nothing added. |
| I26 | Tier 3 | already stated at §2.4 Required-by-day ("That exact target can sit up to 4 minutes above `5 x daily target` ...") and flow F9b. Nothing added. |
| I27 | Tier 3 | already stated at §7.1 Day total bullet ("The Day total line carries no due-today clause. That figure lives on the week line.") and the est. EOD bullet ("appears only when today has an open stint"). Nothing added. |
| I28 | hard error | S3, S8, S11 |
| I29 | Tier 3 | S11 |
| I30 | Tier 3 | S10 week-line paragraph |
| I31 | echo | S6, S13 |
| I32 | argument-order message | S6, S8 |
| I33 | glossary | S5 (one sentence) |
| I34 | log level | S4 |

### S2. §4.3.1 last paragraph, drop the plans pointer (self-contained rule)

Current ending of the paragraph that starts "A spliced stint is not indistinguishable from an ordinary same-date one":

```
... gets a `(HH:MM continues
previous day's stint)` suffix (§7.1) — added by the boundary-context-cues
changeset, see `docs/dev/plans/reports/boundary-context-cues-*`.
```

Replace the tail from "suffix (§7.1) — added" with:

```
suffix (§7.1).
```

This is not listed in spec §5.3. It is required by the self-contained grep (plan T6 acceptance, last bullet) because line 593 matches `docs/dev/plans`. See Risks R2.

### S3. §2.4 Fulfillment / carry bullet (I8, I19, I24, I28)

After the sentence "Nothing live-inclusive is stored." and before "`est. EOD` is computed from fulfillment without live", insert. Drafted:

```
Live minutes come from now, so every figure that includes them changes
minute by minute while a stint runs: the day total, `Worked`,
`Fulfillment`, the week line, and a future week's `Carry-in`. A plain
`stop` at the current time does not move the week line, because those
minutes already counted. A backdated `stop HH:MM` raises the `left to`
figure by the minutes it cuts off, and a backdated `start HH:MM` lowers
it by the minutes it adds. A stint open across midnight gets no live
credit (§7.1).
```

At the end of the same bullet, after the orphaned-`end` sentence ("...never as time in a day/week sum."), append. Drafted:

```
Every storage read behind this walk (the earliest data week, the target
overrides, the punches) is a hard error when it fails (§6.1). No read
falls back to a default target or to "no earlier weeks".
```

### S4. §3 preamble, log level (I34, spec §4.1)

After the first paragraph of §3 ("All commands operate on the local machine's current date/time ... output converts back to local."), add a paragraph. Drafted:

```
The global `-v`/`--verbose` flag turns on debug log lines on stderr.
Without it only warnings and errors are logged, so a run that creates
and migrates a new database prints nothing.
```

SPEC and README describe `--verbose` nowhere today, so this is the one place the SPEC states it (§5.3 row "wherever `--verbose` is described"). See Ambiguity A1.

### S5. §3.5 `mlm status [DATE]` (spec §3.1, §3.3, §4.2, §4.5)

1. First line "Today's (or `DATE`'s, `YYYY-MM-DD`) view:" becomes:

```
Today's (or `DATE`'s, `YYYY-MM-DD`) view. `DATE` must not be after today
(§6.1). `mlm week WEEK_ID` is the way to look at a future week (§7.2).
```

2. Replace the last bullet "any pairing anomalies for that date (§4.3), if present" with:

```
- any pairing anomalies for that date (§4.3), if present, including
  overlapping stints
- for today only, lookback lines for open starts from earlier dates and
  an `[i]` hint for a start still open from yesterday (§7.1)
- a carry-in marker line when an earlier week has an anomaly (§7.1)
```

3. After the bullet list add one sentence. Drafted:

```
`mlm status --help` and `mlm week --help` end with a short glossary of
the terms the week line and the markers use. Short `-h` does not carry
it, and its help line gains clap's `(see more with '--help')` suffix.
```

### S6. §3.7 `mlm week target [WEEK_ID] DURATION` (spec §3.5, §4.3)

Append after the existing paragraph. Drafted:

```
On success it prints one line to stdout, with the week id it wrote to
(the current week when omitted) and the stored target in the standard
duration format (§4.2):

\`\`\`
Week 2026-41 target: 20h 00m
\`\`\`

`start`, `stop` and `note` stay silent (§7.4).

`WEEK_ID` goes after `target`. `mlm week WEEK_ID target DURATION` fails
with one line on stderr and exit `2`, the same code as parser errors:

\`\`\`
error: put WEEK_ID after target: mlm week target 2026-41 20h
\`\`\`

The line echoes the week id and duration as typed, always spells `mlm
week target` even when the user typed the `w` alias, and prints no
`Usage:` block. The success echo above uses the normalized week id, so
`mlm week target 2026-7 33h30m` prints `Week 2026-07 target: 33h 30m`.
`mlm week target --help` shows the usage `mlm week target [OPTIONS]
[WEEK_ID] <DURATION>` and an Arguments section naming both arguments. No
parser error for this subcommand mentions `<ARGS>`.
```

(The backslashes above are only to keep this plan's fences intact. The SPEC gets plain triple backticks.) `[VERIFY AT MERGE]` the two literal lines against T5's tests.

### S7. §4.3 Stint pairing, nested entry and the overlap bullet (spec §3.2.1)

1. Nested-entry paragraph. Current: "This also correctly handles legitimately nested entry (e.g. two `start`s before either `end`: the second `start` pairs with the *first* subsequent `end`, nearest first) without treating it as malformed." Replace the tail "without treating it as malformed." with:

```
without treating the pairing as malformed. The nested stints overlap,
though, and an overlap is flagged (below) because the overlapping
minutes count twice.
```

This is a contradiction the spec does not list. See Risks R3.

2. Add a bullet to the edge-case list after "A `start` and its paired `end` at the identical instant". Drafted:

```
- **Overlapping stints**: two stints on one date overlap when each
  starts before the other ends (`a.start < b.end` and `b.start <
  a.end`) and both have nonzero length. Identical ranges and nests that
  share one endpoint overlap. Stints that only touch at an endpoint do
  not. The check covers the stints the date's page shows. These are
  closed stints and, for today only, the most recent open stint taken up
  to now. A past date's open stint counts zero minutes and is left out. A
  spliced cross-midnight stint (§4.3.1) takes part on each date with the
  part that falls on that date. Each overlapping pair is flagged (§7.3).
  Pairing and totals do not change, so the overlapping minutes still
  count twice in the day total.
```

### S8. §6.1 Hard errors (spec §3.4, §4.3, §4.5)

1. Replace the bullet that starts "`--date`/`-d` on `start`/`stop`/`note` resolving to a date later than today (§3.4a)". Drafted:

```
- `--date`/`-d` on `start`/`stop`/`note` resolving to a date later than
  today (§3.4a), or `status`'s `DATE` argument (§3.5) resolving to one.
  The first three never write into the future, and `status` is no
  preview of a future date. `status DATE` uses the same message shape and
  exit code as the `--date` case. `mlm week WEEK_ID` for a future week is
  not an error (§7.2).
```

2. Add two bullets after the "Database open/migration failure" bullet. Drafted:

```
- A storage read failure in `status` or `mlm week`, on any of the
  reads they make (the earliest data week, the target overrides, the
  punches behind the carry walk). The command prints `error:` and the
  storage error text to stderr, prints nothing to stdout, and exits `1`.
  `mlm week` never falls back to the default target or to "no earlier
  weeks".
- `mlm week WEEK_ID target DURATION`, the week id before `target`
  (§3.7). It is rejected with the one-line message there and exit `2`.
```

### S9. §6.3 Exit codes (plan T6 acceptance, spec plan F7)

Replace the paragraph. Drafted:

```
`0` on success (including a status view that reports anomalies, since
the anomaly is data to look at and not a tool failure). Nonzero on any
§6.1 hard error. A runtime error (a bad value, a storage failure) exits
`1`. A usage error from the argument parser (an unknown flag, a missing
argument, the `week WEEK_ID target DURATION` shape) exits `2`.
```

### S10. §7.1 `mlm status` (spec §3.1, §3.3, §4.4, §4.5; items I1, I6-I11, I15-I18, I20, I23, I30)

Seven sub-edits, applied bottom-up inside §7.1.

a. Lookback bullet ("For today only (`DATE` omitted, or today's own date given explicitly), the anomaly block also lists every date 2 to 14 days old ..."). Replace the whole bullet with these four bullets. Drafted:

```
- For today only (`DATE` omitted, or today's own date given
  explicitly), the anomaly block also lists dates that still have open
  starts, oldest first, each as its own `[!]` line with `fix:`/`or:`
  hints (§7.3). A date 2 to 14 days old with one open start gives the
  unclosed-start line. Yesterday with two or more open starts gives the
  open-stints line with `(yesterday)`. A start open for more than 14
  days drops out of today's `status` and out of `mlm start`'s warnings.
  It stays on its own date's `status` and its own `week` row, and the
  carry-in marker below flags its week. `status DATE` for any other date
  runs no lookback and no hint.
- Yesterday hint, today's page only. When yesterday has exactly one
  open start and it stays unjoined once today's punches are taken into
  account (§4.3.1), `status` prints an `[i]` hint after the anomaly
  block and the lookback lines (line shape in §7.3). A `stop` typed today
  as today's first punch joins the start and clears the hint. There is no
  hint when today's page already shows a refused-join line naming
  yesterday. The `or:` line prints only when today has no punches yet.
  The hint is not an anomaly. It does not mark a `mlm week` row, does not
  count toward the footer, and never shows on `status DATE` for another
  date. `status -1` shows yesterday's open start as a stint line only.
- Carry-in marker, any `DATE`. The shown week's carry-in comes from
  every earlier week, from the earliest week with any punch up to the
  week before the shown one. When at least one date in those weeks would
  carry `[!]` on a `mlm week` row (E7, stale start, E8, overlap, refused
  join), `status` prints one marker line after the anomaly block, the
  lookback lines and the hint (§7.3). The 14-day lookback window does not
  limit it. It uses the same per-date check as the row marker, so the two
  never disagree. It looks only at weeks before the shown week, so the
  anomalous week's own page shows `[!]` rows and no marker, and a week
  with no earlier data or no earlier anomaly prints nothing. It counts
  every anomaly kind, including an orphaned end that adds no minutes, so
  it can name a week whose figures are right. It stays on every later
  week until the old anomaly is fixed, and it applies to stored history,
  so an old anomaly shows it on the first run with no new punch.
- `status DATE` with `DATE` after today is a hard error (§6.1).
```

b. Stint lines bullet, open-stint sub-bullet. Current ending: "...today's own `status` surfaces it too (the lookback bullet below).\n    See §1.2a for what's still not covered once a date passes the\n    14-day window." Replace with:

```
    ... today's own `status` surfaces it too (the lookback bullet below).
```

(delete the "See §1.2a" sentence; the 14-day drop-out is now in sub-edit a.) Also in the same sentence change "the 2-to-14-day lookback window" to "the 14-day lookback window".

c. Week line bullet. After the Friday sample block (the fenced `Week 2026-40:  01h 10m over 40h 00m target (...)`), add a paragraph inside the bullet. Drafted:

```
  The week line changes form at Thursday-to-Friday midnight, from `left
  to 32h 00m by end of Thursday` to `left to 40h 00m target`. Monday
  through Thursday, `over` means today's slice is met while the week's
  target may still be far off. The current week prints no whole-week
  remainder. It is `target - fulfillment`, which the parenthetical lets
  a reader compute, so Monday through Thursday `mlm week`'s `Target:`
  and the headline's `left to` figure name different amounts. A week
  that starts in debt can read more left than the target. With a
  carry-in of `-28h 00m` and 17h 27m worked on a Friday, the headline
  reads `50h 33m left to 40h 00m target`, because `gap` is `required -
  fulfillment` and fulfillment includes the debt. The same zero reads
  differently by framing. A current week at `gap == 0` reads `00h 00m
  left to ...`, and the same week once closed reads `Total ahead: 00h
  00m` (§7.2). For an earlier date in the current week, the week line
  counts today's live minutes beside a day total for `DATE` that has
  none, so the day total stays fixed while the week line moves. `mlm
  week` moves only today's row.
```

d. Est. EOD bullet (three small edits).

1. "or replaced with\n  `target already met` when the gap is already zero or negative." becomes "or replaced with\n  `done for today` when the gap is already zero or negative."
2. In the same bullet, after "...and `est. EOD` projects from that start's own time." insert: "It therefore sits further ahead of the clock than the week line's `left to` figure, and the week line does not count down."
3. After the day-suffix sub-list (the `est. EOD 06:20 (Mon 2026-10-12)` item) add a paragraph. Drafted:

```
  `est. EOD` is fixed per stint, so its day suffix shows from the first
  `status` after the `start` and does not change while the stint runs. A
  break (`stop`, then `start`) moves it later by the break's length. From
  Friday on `required` is the full target, so with little worked it can
  land days out, e.g. `est. EOD 06:00 (Mon)`. It sits on the Day total
  line, one line above the week line it is computed from. Monday through
  Thursday, `done for today` means today's slice is covered, not the
  target the week line's parenthetical names. At `gap == 0` the week
  line still reads `00h 00m left to ...` beside `done for today`, and
  `done for today` can show next to `(incl. ongoing)` while a stint
  runs, or with little or no time worked when carry-in already covers
  what is due. Two `status` runs with no punch between them can print
  different week-line figures, because live minutes grow with the clock.
  When the running stint closes the gap, the line flips from `left to`
  to `over` and `est. EOD` becomes `done for today`.
```

Already stated, nothing added for I22 (Day total bullet) and I21.

e. Nothing for I26/I27/I25 (see S1 ledger).

f. Past-or-future wording in the week-line bullet (spec §4.5). `status DATE` rejects a future date, so a future week is no longer a `status` page. Two passages change, matched by content.

1. In the sentence "A past or future week gets the plain `Total behind`/`Total ahead` form when `DATE`'s week isn't the current one.", replace "A past or future week gets" with "A past week gets" and append a sentence. The result, drafted:

```
A past week gets the plain `Total behind`/`Total ahead` form when
`DATE`'s week isn't the current one. A future week is viewable only
through `mlm week WEEK_ID` (§7.2), because `status DATE` rejects a
future date (§6.1).
```

2. In "The past/future `Total behind`/`Total ahead` form never gets this parenthetical, zero or non-zero carry-in alike.", replace "The past/future" with "The past-week". The `mlm week` past/future wording in §7.2 stays.

Grep afterwards that no other §7.1 passage says `status` shows a future week. The remaining hits (§2.4, §7.2, §8 E13) concern `mlm week` and stay.

### S11. §7.2 `mlm week` (spec §3.2, §3.3, §3.4; I28, I29)

a. Row-marker bullet. "Any date in the row list with an anomaly (§4.3, plus the stale-open-start and refused-join anomalies) gets a marker" becomes "...(§4.3, plus the stale-open-start, overlap and refused-join anomalies) gets a marker...".

b. Footer closing paragraph. Replace "No footer when no row is marked. `week` runs no lookback of any kind — the row marker and this one footer line are the only new things here." with. Drafted:

```
  No footer when no row is marked. `week` runs no lookback and no
  yesterday hint. The row marker, the footer and the carry-in marker
  below are its only anomaly output.
```

c. New bullet after the footer bullet, the carry-in marker. Drafted:

```
- Carry-in marker. When an earlier week has an anomaly (the same
  condition as in §7.1), the Carry-in figure gets a trailing marker and
  the line `status` prints for the same week sits above the Carry-in
  block. The layout is the day rows, a blank line, the anomaly footer if
  any row is marked, the marker line directly under it (no blank line
  between), one blank line, then the Carry-in block. With no marked row
  the marker line takes the footer's place:

\`\`\`
  Sun 2026-10-11   00h 00m

[!] needs a look: run mlm status DATE for details and fixes
[!] carry-in includes anomalies from 2 earlier weeks, oldest 2026-38 (see mlm week 2026-38)

Carry-in:      -80h 00m  [!]
\`\`\`

  With one week the line reads `from 1 earlier week, 2026-38 (see mlm
  week 2026-38)`. Nothing prints for a week with no earlier data or no
  earlier anomaly. It prints for any shown week, future and current
  included, and never counts the shown week's own anomalies.
```

`[VERIFY AT MERGE]` the layout against T1's rendering and the footer literal in `src/render.rs` (changeset-plan contract 6, resolved; T1 plan step 5). The footer line in the sample must be copied from the merged `WEEK_ANOMALY_FOOTER`, not from this plan.

d. Read errors, new bullet. Drafted:

```
- Every storage read `mlm week` makes is a hard error, as in `status`
  (§6.1). The storage error goes to stderr as `error: ...`, stdout stays
  empty, and the exit code is `1`. `mlm week` never falls back to the
  default 40h target or to "no earlier weeks".
```

e. Past/future week paragraph ("A **past or future** week has no "today" to frame a deadline against ..."). After the closing "(or `Total ahead: 00h 00m` ... to it.)" paragraph add. Drafted:

```
A future week has not started, so with no carry-in its headline reads
`Total behind` for the whole target. `mlm week WEEK_ID` is the way to
view a future week, because `status DATE` rejects a future date (§6.1).
```

### S12. §7.3 Anomaly rendering (spec §3.1, §3.2, §3.3)

Edit bottom-up.

a. After the "Two or more open starts on the window date" block, add the `(yesterday)` forms, the hint, and the marker. Drafted:

```
Yesterday with two or more open starts uses `(yesterday)` in place of
`(N days ago)`:

\`\`\`
[!] 2 open stints on Fri 2026-10-02 (yesterday)
    fix: see mlm status 2026-10-02
\`\`\`

Yesterday with exactly one unjoined open start gets an `[i]` hint
instead of an anomaly line (§7.1). `[i]` is the quiet marker. It never
marks a `week` row and never counts toward the footer:

\`\`\`
[i] start on Fri 2026-10-02 at 18:02 still open (yesterday; not counted until stopped)
    fix: mlm stop HH:MM --date 2026-10-02  (a time after 18:02)
     or: mlm stop  (still working: ends it now, across midnight)
\`\`\`

The `or:` line prints only when today has no punches yet, because only
then does a `stop` now become today's first punch and join the start.

The carry-in marker (§7.1, §7.2) is one `[!]` line, last on the page:

\`\`\`
[!] carry-in includes anomalies from 2 earlier weeks, oldest 2026-38 (see mlm week 2026-38)
\`\`\`
```

b. Lookback single-start block ("Lookback lines (today's `status` only, §7.1)"). Keep the 5-days-ago example and add the window form after it: "With a later punch on that date, the hint names a window (see the stop-hint rule above):" then block with `(a time between 09:00 and 10:30)` on the `fix:` line.

c. Stale open start block. After the existing block add: "With a later punch the hint names a window:" and

```
    fix: mlm stop HH:MM --date D  (a time between 09:00 and 10:30)
```

d. E7 block. After the existing block add. Drafted:

```
With a later punch after the 14:00 start, the stop hint names a window:

\`\`\`
     or: mlm stop HH:MM --date D  (a time between 14:00 and 15:00 closes the 14:00 start)
\`\`\`

The hint names only the latest open start, but a `stop` between two open
starts also works. It pairs with the earlier start and leaves the later
one open. The stint list shows every open stint with its own duration,
while the week line and `est. EOD` credit only the most recent start
(§7.1), so fixing the anomaly makes both jump.
```

e. New block between the E8 block and the refused-join block (order: E8, overlap, refused join). Drafted:

```
Overlapping stints (§4.3), one line per overlapping pair:

\`\`\`
[!] overlapping stints 09:00-12:00 and 10:00-11:00 (the overlap counts twice)
    fix: mlm delete punch --date D  (lists punches; rerun with the number of the punch to remove)
\`\`\`

An open stint prints as `09:00-now`. A spliced cross-midnight stint
(§4.3.1) is labelled with its real start and end times, as in the stint
list (e.g. `23:30-00:45`), on both dates. Pairs sort by the first
stint's start, then the second's. The date's `week` row gets `[!]` and the date
counts toward the footer. The overlap still counts twice in the day and
week totals. The line flags the double count and does not fix it.
```

f. Order paragraph ("Lines for one date print in this fixed order ..."). Replace. Drafted:

```
Lines for one date print in this fixed order: multiple open stints
(E7), stale open start, orphaned ends (E8, ascending by instant),
overlapping stints (one per pair), a refused join with the previous date
then the next date, and, for today only, one lookback line per earlier
date with open starts (oldest first, §7.1), then the yesterday hint. The
carry-in marker line comes last (§7.1).
```

g. Fix-command details paragraph (ends "...so the stop hint names the start time the new end must follow."). Append the clean-pair rule. Drafted:

```
A stop hint targets one open start `S`. Let `N` be the first punch on
that date after `S`. A stop between `S` and `N` pairs with `S` and
nothing else, and a stop after `N` pairs with a later start or nests `S`
around later stints. With no punch after `S` the hint reads `(a time
after S)`. With one it reads `(a time between S and N)`. The window
includes `N`'s minute, because a stop at the same instant as a start
pairs with `S` (§4.3, step 2). Punches at the same minute as `S` do not
count as after `S`. An open start at `23:59` with nothing after it keeps
`(a time after 23:59)`, which no `HH:MM` satisfies (a same-instant stop
pairs zero-length, E14). The rule applies to the stale-start line, the E7
line, the lookback lines, the yesterday hint and `mlm start`'s warnings.
```

h. Opening paragraph. "Plain-ASCII marker, no color/unicode dependency: prefix `[!] `." Append: "`[i] ` marks a hint that is not an anomaly." Also adjust the next sentence's "Most `status` anomaly lines" unchanged.

### S13. §7.4 Write-command output (spec §3.1, §3.5)

1. Opening paragraph. Replace the first sentence and the exception sentence. Drafted:

```
`start`, `stop` and `note` print nothing on success. They are silent,
Unix-conventional, and exit `0` (§6.3) is the only signal. A hard error
(§6.1) still prints its message to stderr as usual. `status` and `week`
produce stdout output (§7.1, §7.2). `week target` prints one line (§3.7).
`delete note`/`delete punch` are a narrow, deliberate exception to the
write-command-silence rule above rather than a repeal of it. List mode
prints that date's numbered entries (or `nothing to delete for <date>.`
when there are none), and a successful delete prints a ready-to-run
recreate command.
```

2. `start` stderr paragraph. "it runs the same 14-day lookback `status` uses for today (§7.1) and prints one warning per stale date found, oldest first" stays, plus add after the two existing warning blocks. Drafted:

```
A single unjoined open start dated yesterday warns too, with no `or:`
line because today now has a punch (§7.1):

\`\`\`
warning: start on Fri 2026-10-02 at 18:02 still open (yesterday)
    fix: mlm stop HH:MM --date 2026-10-02  (a time after 18:02)
\`\`\`

Yesterday with two or more open starts warns in the one-line form with
`(yesterday)`:

\`\`\`
warning: 2 open stints on Fri 2026-10-02 (yesterday); see mlm status 2026-10-02
\`\`\`

`start` repeats the yesterday warning on every call while that start
stays open. With a later punch on a warned date, the `fix:` time is a
window, `(a time between S and N)` (§7.3). `start` prints no carry-in
marker.
```

Existing "(a time after 09:00)" example in §7.4 stays as the no-later-punch form.

### S14. §8 User flows

Bottom-up. All edits keep the flow ids stable.

- F7: append "and `week target` printed `Week 2026-07 target: 33h 30m`" after "then `status`/`week` during that week reflect the new target ...". Wording: "`week target 2026-07 33h30m` prints `Week 2026-07 target: 33h 30m`, then `status`/`week` ...".
- F9 (line 1157): `target already met` becomes `done for today` in "(shows `target already met`)".
- F9b (lines 1166-1170): "(or `target already met` for est. EOD)" and "est. EOD to `target already met`" both become `done for today`.
- E7 flow: append "The `or:` hint names a window when a punch follows the latest open start (§7.3), and the anomaly block and `status` list overlapping nested stints as their own anomaly (E21)."
- E10: replace "clap-level missing-argument error" with "parser missing-argument error whose usage names `[WEEK_ID] <DURATION>` and never `<ARGS>`, exit `2`".
- E15: replace "(§1.2a notes this as a known rough edge, not a bug)" with "(§4.3.1)".
- E16: append "With a later punch after the start, the `fix:` time is a window, `(a time between 09:00 and 10:30)` (§7.3)."
- E18: change "2 to 14 days old" to "1 to 14 days old" semantics. New text: "Today's lookback (§7.1): a plain `status` for today (`DATE` omitted, or given explicitly) additionally lists dates 2 to 14 days old that still have an open start, and yesterday when it has two or more open starts, oldest first, each as its own `[!]` line with `fix:` lines. `status DATE` for any other date runs no lookback. A date whose open start is 15 or more days old falls outside the window and isn't listed (§7.1), though the carry-in marker still flags its week (E23)."
- E19: append "A single unjoined open start dated yesterday warns too, with no `or:` line (§7.4). `start` prints no carry-in marker."
- New flows after E19 (SPEC §8.2). Each is two to four lines, drafted.

```
- **E20** - `mlm week` read failure (§6.1, §7.2): an unreadable
  `week_targets` table, `notes` table, or punch range makes `mlm week`
  and `mlm week WEEK_ID` print `error: ...` to stderr, nothing to
  stdout, and exit `1`. `status` does the same. Successful output is
  unchanged.
- **E21** - Overlapping stints (§4.3, §7.3): `start 09:00`, `start
  10:00`, `stop 11:00`, `stop 12:00` pairs into 09:00-12:00 and
  10:00-11:00. The date's `status` shows the overlap line, its `week`
  row gets `[!]` and the footer prints once. The day and week totals
  still count the overlap twice. Stints that only touch at an endpoint
  and zero-length stints are not flagged.
- **E22** - Start still open from yesterday (§7.1, §7.4): today's
  `status` prints the `[i]` hint, with the `or:` line only when today has
  no punches. A `stop` as today's first punch joins the start and the
  hint is gone. A refused-join line naming yesterday suppresses the hint.
  The hint never marks a `week` row and never shows on `status -1`.
- **E23** - Carry-in marker (§7.1, §7.2): an anomaly in a week older
  than 14 days puts the marker line on `status` and on `mlm week`, with
  `[!]` on the Carry-in figure. Several anomalous weeks give the plural
  form naming the oldest. The marker never counts the shown week's own
  anomalies and prints nothing when no earlier week has one.
- **E24** - Argument order (§3.7, §6.1): `mlm week 2026-41 target 20h`
  prints `error: put WEEK_ID after target: mlm week target 2026-41 20h`,
  no `Usage:` block, exit `2`. `mlm week 7 foo` keeps the parser's own
  error.
- **E25** - Future `status DATE` (§6.1): `status` with a date after
  today exits `1` with the future-date message. `mlm week` for a
  future week still prints.
```

- New F-flow after F12. Drafted:

```
- **F13** - First run on a fresh database prints nothing on stderr. With
  `--verbose` the migration line and debug lines appear (§3).
```

- SPEC §6.2 is unchanged.

## README.md edit list

Bottom-up. All examples follow SPEC literals above.

### R1. `week target` section (lines 453-480; spec §3.5, §4.3)

1. "Silent on success:" becomes "On success it prints one line with the stored target:" and the example gets the echo line. Drafted:

```sh
$ mlm week target 45h
Week 2026-37 target: 45h 00m
$ mlm week
Week 2026-37 (2026-09-07 - 2026-09-13)
... (rest unchanged)
```

2. After that example add. Drafted:

```
The duration echoes in the standard format, so `90m` prints as `01h 30m`.
`WEEK_ID` goes after `target`. Typing it before `target` fails with a
reminder and exit code 2:

```sh
$ mlm week 2026-41 target 20h
error: put WEEK_ID after target: mlm week target 2026-41 20h
```

`mlm week target --help` shows the arguments as `[WEEK_ID] <DURATION>`.
```

### R2. `week` section (lines 412-451; spec §3.3, §3.4)

1. Replace the final paragraph ("An unfixed anomaly in a past week also throws off every later week's `Carry-in` ... (see Possible issues).") with. Drafted:

```
An unfixed anomaly in a past week also throws off every later week's
`Carry-in`, since carry-in is just the prior week's fulfillment against
target. When any earlier week has a flagged date, the Carry-in figure
gets a `[!]` and a line naming how many earlier weeks are affected and
the oldest one sits above the Carry-in block. It goes directly under
the footer line when a row is flagged, and takes the footer's place
otherwise, with one blank line before the Carry-in block:

```
[!] carry-in includes anomalies from 2 earlier weeks, oldest 2026-38 (see mlm week 2026-38)

Carry-in:      -80h 00m  [!]
```

With one affected week the line reads `from 1 earlier week, 2026-38 (see
mlm week 2026-38)`. The line also prints on `status` (see above). It
stays until the old anomaly is fixed. If `mlm week` cannot read the
database, it prints an `error:` line and exits 1 instead of showing
default figures.
```

2. Footer paragraph (line 439-446) unchanged except add "Overlapping stints mark their row too." after "carries `[!]`" sentence? Drafted append to "No footer line prints when nothing in the week is flagged.": nothing. Instead, extend the sentence list in the status section (R3).

### R3. `status` section (lines 298-410; spec §3.1-§3.3, §4.4, §4.5)

1. Day total sample (line 308) `Day total:     12h 44m (incl. ongoing), target already met` becomes `Day total:     12h 44m (incl. ongoing), done for today`.
2. Intro paragraph ("`DATE` accepts `YYYY-MM-DD` or `-N` ...") append: "`DATE` must not be after today. `status` for a future date fails, and `mlm week WEEK_ID` shows a future week."
3. After the `[!]` example block (the stale start block) add the windowed hint and overlap. Drafted:

```
When a later punch follows the open start, the `fix:` time is a window
that pairs cleanly with that start:

```
    fix: mlm stop HH:MM --date 2026-09-25  (a time between 09:00 and 10:30)
```

Overlapping stints are flagged too. A day like `start 09:00, start
10:00, stop 11:00, stop 12:00` pairs into 09:00-12:00 and 10:00-11:00,
and the overlapping hour counts twice in the day total:

```
[!] overlapping stints 09:00-12:00 and 10:00-11:00 (the overlap counts twice)
    fix: mlm delete punch --date 2026-09-25  (lists punches; rerun with the number of the punch to remove)
```

The `week` row for that date gets `[!]` as well.
```

4. Today's-plain-status paragraph (line 331-335 "also checks dates 2 to 14 days old"). Replace. Drafted:

```
Today's plain `status` (no `DATE`, or today's own date given
explicitly) also checks dates 2 to 14 days old and appends one such
line for every date it finds with an unclosed start, oldest first, so a
forgotten `stop` from last week still shows up today. Yesterday gets
its own treatment, because an overnight session may still be running. A
lone start still open from yesterday prints a quiet hint, which marks no
`week` row:

```
[i] start on Fri 2026-10-02 at 18:02 still open (yesterday; not counted until stopped)
    fix: mlm stop HH:MM --date 2026-10-02  (a time after 18:02)
     or: mlm stop  (still working: ends it now, across midnight)
```

The `or:` line only shows when today has no punches yet. A `stop` typed
as today's first punch joins the start and clears the hint. Yesterday
with two or more open starts prints the `[!]` line `2 open stints on
Fri 2026-10-02 (yesterday)` instead. If an earlier week has a flagged
date, the last line of the page names it:

```
[!] carry-in includes anomalies from 2 earlier weeks, oldest 2026-38 (see mlm week 2026-38)
```
```

5. Est. EOD paragraph (line 345-369, the long one). Edits inside it.
   - After "It assumes you keep working without a break." add " A break moves it later by the break's length. When the running stint has closed the gap, the line reads `done for today` instead of a clock time."
   - Remove "(see Possible issues)" from "`mlm week` for the current week shows the same headline and the same live-inclusive figures (see Possible issues)." leaving "...live-inclusive figures."
   - Append a sentence near the end of the paragraph. Drafted: "A stint still open after midnight is not on the new day's page, so it gets no live credit until you `stop` it."
   - After the paragraph, add: "`mlm status --help` and `mlm week --help` end with a short glossary of these terms."

### R4. `start` section (lines 260-273; spec §3.1, §3.2.2)

Replace the paragraph and add examples. Drafted:

```
After a successful write, `start` also checks yesterday and dates up to
14 days old for open starts and prints a `warning:` block per date found
to stderr, oldest first. Stdout and the exit code are unaffected either
way. The date you just wrote to draws no warning only when its single
open start is the exact punch this command just inserted. Two or more
open starts there, or a single older open start the new punch left
behind, still warn. A start still open from yesterday warns on every
`start` until you stop it.
```

Keep the existing 5-days-ago example. Add after it:

```sh
$ mlm start 09:00
warning: start on Fri 2026-10-02 at 18:02 still open (yesterday)
    fix: mlm stop HH:MM --date 2026-10-02  (a time after 18:02)
```

Add one sentence: "When a later punch follows the warned start, the `fix:` time is a window, `(a time between 09:00 and 10:30)`."

### R5. Usage paragraph (lines 227-231; spec §3.5, §4.1)

Replace. Drafted:

```
Every `start`/`stop`/`note` call is silent on success, and nothing
prints unless something went wrong. That holds on the first run too,
when the database is created. `week target` prints one line confirming
the stored target. `status` and `week` are the commands that produce
output. `delete note`/`delete punch` are a narrow exception too (see
below). A `status` after punching in/out is still how you confirm things
landed correctly. Add `-v`/`--verbose` to any command to see debug log
lines on stderr, including the one-time migration line.
```

### R6. Possible issues section (lines 43-160; spec §5.4)

Keep the heading and the intro line. Delete all 33 bullets. Add one bullet, drafted:

```
- A stint started yesterday and still open after midnight is not on
  today's page, so it gets no live credit and no `est. EOD`. Its minutes
  leave the day total, `Worked`, `Fulfillment` and the week line until
  `stop`, and the week line jumps at 00:00. When this happens past
  Sunday midnight, the closed week's `Worked` drops by those minutes.
  Today's `status` shows an `[i]` hint for the open start that points to
  `stop`
```

README bullets in the old list map to SPEC items one to one except one merge. The `est. EOD` bullet of README merges I9 and I10 and goes whole. No removed bullet shares text with the kept item. The README/SPEC drift (validation F8) disappears with the list.

### R7. Features list (lines 16-19)

`week target` bullet. Replace "override a week's target hours; any shortfall or surplus carries into the next week" with "override a week's target hours and print the stored target back; any shortfall or surplus carries into the next week". Add to the `week` bullet: "flags a carry-in that includes an earlier week's anomaly" is not needed. Leave the rest.

## AGENTS.md edit list

### A1. Layout (lines 33-49; plan T6 acceptance)

Add four bullets after the `src/week.rs` bullet and extend two others. Drafted from T1's plan:

```
- `src/anomalies.rs` — per-date anomaly facts (E7, stale, E8, overlap,
  refused joins) built from raw punch buckets, and their rendered
  lines. `status`, `week` rows and the carry scan all call it.
- `src/lookback.rs` — today's lookback scan (ages 1-14), the §3.1
  yesterday hint and `mlm start`'s stderr warnings
- `src/history.rs` — the shared history loader. One earliest-date read,
  one ranged punch read and one ranged target read build the week
  ledger and the bucketed punches for `status` and `mlm week`. Any read
  failure is an error.
- `src/carry_scan.rs` — the carry-in marker data (earlier weeks with an
  anomaly), computed over the loader's punches with no reads of its own
```

Extend `src/storage.rs` bullet: "`Punch`/`PunchKind`/`Note`, punch/note insert+read, one ranged week-target-override read". Extend `src/render.rs` bullet: "shared rendering helpers used by both `status`/`week`, including the clean-pair stop-hint helper and phrases and the carry-in marker line". `[VERIFY AT MERGE]` against the merged names. T1 plans `week_targets_in_range` in `storage.rs`, and `next_punch_after`, `stop_time_hint`, `stop_time_hint_closes` and `carry_marker_line` in `render.rs`. The Layout bullets name no function, so only the module-level wording needs to hold.

Do not touch "Current state". It already says all commands are implemented and match SPEC. Optional one-line addition is not needed.

## Test plan

A docs task has no unit tests. Each acceptance criterion gets a check made of greps and, after the code tasks merge, a literal comparison against the built binary and the merged tests. Scratch binary for any run: `CARGO_TARGET_DIR=/tmp/cd-scratch/task-6 cargo build`.

### 1.1 §1.2a keeps only the I14 counting half (spec §5.1)

- `awk '/^### 1.2a/,/^### 1.3/' docs/dev/SPEC.md | grep -c '^- '` prints `1`.
- The one bullet contains `no live credit`, `until` and `stop`, `00:00`, and `[i]` hint wording, and no `shares a root`.
- `grep -n "est. EOD means\|LIFO and accepts nested\|lone open start dated yesterday\|silently accepts a future" docs/dev/SPEC.md` finds nothing.
- Entries that shared a bullet with a removed item: none exist in SPEC. README's merged I9/I10 bullet is removed whole.

### 1.2 Tier 3 behavior stated once in its owner (spec §5.2)

Per item, the check is a grep for one distinctive phrase, once, in the named section.

| Items | Phrase to find | Section |
|---|---|---|
| I6 | `Total ahead: 00h 00m` near `gap == 0` | §7.1 |
| I7 | `Two `status` runs with no punch between them` | §7.1 |
| I8, I19, I24 | `changes minute by minute`, `backdated `stop HH:MM` raises` | §2.4 |
| I9-I11, I22 | `moves it later by the break's length`, `fixed per stint`, `further ahead of the clock` | §7.1 |
| I12, I13 | `a `stop` between two open starts also works` | §7.3 |
| I15, I16, I17, I18, I30 | `From Friday on `required` is the full target`, `changes form at Thursday-to-Friday`, `no whole-week remainder`, `50h 33m left to 40h 00m target` | §7.1 |
| I20 | `counts today's live minutes beside a day total for `DATE`` | §7.1 |
| I23 | `done for today` with `(incl. ongoing)` and `gap == 0` | §7.1 |
| I29 | `A future week has not started` | §7.2 |
| I21, I22, I25, I26, I27 | already stated, nothing added (ledger in S1) | §7.1, §2.4 |

Each phrase appears exactly once (`grep -c`). I1's 14-day drop-out: `grep -n "drops out of today's" docs/dev/SPEC.md` hits §7.1 once.

### 1.3 Spec §5.3 table applied (spec §5.3)

One check per row.

- §3.1 hint and ages 1-14 (§7.1, §7.3, §7.4). `grep -c "still open (yesterday" docs/dev/SPEC.md` is at least 2 (hint block, warning block). `(yesterday)` appears in the `2 open stints` line in both §7.3 and §7.4.
- §3.2 overlap (§4.3, §7.3). `grep -c "overlapping stints 09:00-12:00" docs/dev/SPEC.md` is 1. §4.3 has the `Overlapping stints` bullet. The nested-entry paragraph no longer says "without treating it as malformed." alone.
- §3.2.2 hint times (§7.3, §7.4). `grep -n "a time between" docs/dev/SPEC.md` hits the E7, stale, lookback and §7.4 examples. Every old `(a time after <start>)` example sits beside a windowed one.
- §3.3 carry marker (§7.1, §7.2, §7.3). `grep -c "carry-in includes anomalies from 2 earlier weeks" docs/dev/SPEC.md` is 2 (§7.2 layout block, §7.3 line). The §7.2 block also shows `Carry-in:      -80h 00m  [!]` once.
- §3.4 read errors (§6, §7.2). §6.1 has the storage-read bullet. §7.2 has the `Every storage read` bullet. The last sentence of §2.4 states it.
- §3.5 echo (§3.7, §7.4). §3.7 has `Week 2026-41 target: 20h 00m`. §7.4 no longer lists `week target` among the silent commands: `grep -n "week target. print nothing\|and `week target`, print nothing" docs/dev/SPEC.md` finds nothing.
- §4.1 log level. §3 preamble has the `--verbose` paragraph.
- §4.3 argument order (§3.7, §6.1). `grep -c "put WEEK_ID after target" docs/dev/SPEC.md` is at least 2.
- §4.4 wording (§7.1). `grep -c "target already met" docs/dev/SPEC.md` is `0`.
- §4.5 future `status DATE` (§3.5, §6.1). `grep -n "has no such restriction" docs/dev/SPEC.md` finds nothing. §3.5 and §6.1 each state the rejection. `grep -n "past or future week gets\|past/future .Total behind" docs/dev/SPEC.md` finds nothing in §7.1 (S10f).
- §8 flows. F7, F9, F9b, E7, E10, E15, E16, E18, E19 amended, E20-E25 and F13 added. `grep -n "§1.2a" docs/dev/SPEC.md` hits only §1.2's own pointer line (the two §8 pointers and the §7.1 pointer are gone).

### 1.4 §6.3 exit codes (plan T6 acceptance)

- §6.3 contains `Nonzero on any §6.1 hard error`, `exits `1``, and `exits `2``.
- It still says `0` on success. `main.rs` exits `1` for every hard error today, so the doc matches the binary (changeset plan contract 9).

### 1.5 Visible consequences recorded (spec §6)

The four survivors named by the plan plus the retroactive effect, each found once.

- Persistent carry marker: `stays on every later week until the old anomaly is fixed` (§7.1).
- Double count flagged and not fixed: `flags the double count and does not fix it` (§7.3).
- `done for today` beside `(incl. ongoing)`: §7.1 est. EOD paragraph.
- `00h 00m left to ...` beside `done for today` at `gap == 0`: same paragraph.
- Retroactivity: `applies to stored history` (§7.1).
- The rest of spec §6 is covered where a section already says it (`[i]` today-only and `status -1`, repeated `start` warning, age 1 versus age 2, marker counts every kind, marker looks only at earlier weeks, argument-order message spelling `week target`, echo formatter).

### 1.6 README (spec §5.4)

- `awk '/^### Possible issues/,/^## Install/' README.md | grep -c '^- '` prints `1`.
- `grep -c "target already met" README.md` is `0`. The Day total sample reads `done for today`.
- `grep -n "see Possible issues\|(see Possible issues)" README.md` finds nothing.
- `grep -n "Silent on success\|silent on success" README.md` finds only the `start`/`stop`/`note` claim in the Usage paragraph, and the `week target` section no longer says silent. The Usage claim is true because T5 makes `Warn` the default (no migration line) and `week target` prints.
- The `week target` example shows `Week 2026-37 target: 45h 00m`.
- The usage sample for the argument-order message is present.

### 1.7 Quoted literals match the binary (cross-check at merge)

After T1-T5 merge, from the integration branch, compare each literal below with `src` and `tests` by line shape. The shape is the fixed text around the dates and times. The sample dates (`Fri 2026-10-02`, `2026-10-02`, `D`) and times (`18:02`, `09:00-12:00`) are not matched, because the T1-T3 tests use their own fixtures (T3 unit tests `Wed 2026-02-11`, T3 integration tests a clock-derived date, T2 `--date 2026-09-01`). For each literal, run `grep -rnF '<fragment>' src tests` on the fixed fragments listed after it and expect at least one hit per fragment in a test or a constant. This proves the doc quotes what the code prints.

- `[i] start on Fri 2026-10-02 at 18:02 still open (yesterday; not counted until stopped)`. Fragments: `[i] start on `, ` still open (yesterday; not counted until stopped)`.
- `    fix: mlm stop HH:MM --date 2026-10-02  (a time after 18:02)`. Fragments: `fix: mlm stop HH:MM --date `, `  (a time after `.
- `     or: mlm stop  (still working: ends it now, across midnight)`. Fragment: the text after `or: `.
- `warning: start on Fri 2026-10-02 at 18:02 still open (yesterday)`. Fragments: `warning: start on `, ` still open (yesterday)`.
- `[!] 2 open stints on Fri 2026-10-02 (yesterday)` and the `warning: 2 open stints on ... (yesterday); see mlm status` form. Fragments: `2 open stints on `, ` (yesterday)`, `; see mlm status `.
- `[!] overlapping stints 09:00-12:00 and 10:00-11:00 (the overlap counts twice)` and its `fix:` line. Fragments: `[!] overlapping stints `, ` (the overlap counts twice)`, `fix: mlm delete punch --date `, `  (lists punches; rerun with the number of the punch to remove)`. The doc's `--date D` and a test's real date have the same shape.
- `[!] carry-in includes anomalies from 2 earlier weeks, oldest 2026-38 (see mlm week 2026-38)` and the `1 earlier week` form
- `Carry-in:      -80h 00m  [!]`
- The `mlm week` layout of S11c, from a merged T1 test or a scratch run: footer line (if a row is marked), marker line directly under it with no blank between, one blank line, then the Carry-in block. With no marked row the marker line takes the footer's place.
- `Week 2026-41 target: 20h 00m`
- `error: put WEEK_ID after target: mlm week target 2026-41 20h`
- `done for today`

Windowed examples (`a time between 09:00 and 10:30`, `14:00 and 15:00`) use made-up times, so check the shape `(a time between S and N)` and the E7 tail ` closes the S start`, not the digits.

Then smoke-run the scratch binary once. `MLM_DB_PATH=/tmp/cd-scratch/task-6/x.db mlm week target 20h` must print `Week <current id> target: 20h 00m`. `mlm week 2026-41 target 20h` must print the argument-order line and exit `2`. A first `mlm start 9:00` prints nothing on stderr.

### 1.8 AGENTS.md Layout (plan T6 acceptance)

- `grep -c "src/anomalies.rs\|src/lookback.rs\|src/history.rs\|src/carry_scan.rs" AGENTS.md` is 4.
- The storage bullet names the ranged target-override read.
- Check each module name exists on the integration branch (`git show <branch>:src/<name>.rs`).

### 1.9 Self-contained check (spec §5.4, AGENTS.md Documentation rule)

- `grep -n "docs/dev/plans\|docs/dev/specs\|possible-issues-triage" docs/dev/SPEC.md README.md` finds nothing. Baseline had one hit (SPEC line 593, removed by S2).
- `grep -n "docs/dev/plans\|docs/dev/specs" AGENTS.md` finds only the Documentation section's own rule text, which is not edited.
- Non-ASCII check on added lines only: `git diff -U0 -- docs/dev/SPEC.md README.md AGENTS.md | grep '^+' | grep -nP '[^\x00-\x7F]'` finds nothing. (AGENTS.md already contains em dashes in old lines, which `git diff -U0` does not show.)
- Style check on added prose: no sentence with a colon followed by a clause outside a literal block or list of three or more.
- Pre-commit hook: docs-only commit passes `cargo fmt --check` and the complexity/coverage gates trivially. Do not let the hook rewrite `coverage-baseline.json` in this worktree. Revert it if it does (changeset plan global constraints).

## Risks

R1. The quoted literals come from the spec, not from the code. T1-T5 are not built. The cross-check in 1.7 closes this at merge, and a binary that disagrees wins. Only spec-given literals are quoted. Windowed example times and the `warning: start on ... (yesterday)` sample date come from the spec's own dates or are invented for illustration.

R2. SPEC line 593 holds a pointer to `docs/dev/plans/reports/boundary-context-cues-*`. Spec §5 does not mention it, but the self-contained grep in the plan's acceptance criteria fails while it stands. S2 removes it. This is also an AGENTS.md Documentation-rule fix.

R3. Spec and plan do not mention the SPEC §4.3 nested-entry sentence ("legitimately nested entry ... without treating it as malformed"). Once overlap is an anomaly it reads as a contradiction. S7 rewrites its tail.

R4. Spec §5.3 lists §7.1, §7.3, §7.4 for the hint and does not list §7.2 for the overlap row marker. S11 adds the overlap to the §7.2 row-marker sentence, since the row `[!]` and the footer follow from it (spec §3.2.1 last bullet).

R5. Spec §5.2 names §7.1 as the owner of I26 (non-multiple-of-5 override). §2.4 already states it, and flow F9b tests it. This plan adds nothing there and records "already stated at §2.4". A reviewer who wants §7.1 to carry it can add one sentence. I judged a duplicate worse than a pointer.

R6. README's wording drift (validation F8) is gone with the list, but README prose in the `status` section (lines 345-367) still describes est. EOD and the week line in its own words. S/R edits add only the new facts (break shift, `done for today`, midnight). The two documents may still differ in phrasing there. They do not disagree in content.

R7. Doc order of operations. T6 runs in parallel with the code tasks and quotes their literals. If a code task deviates from the spec literal (T5's message, T1's marker placement), this task's quoted blocks go stale until 1.7 runs. The coordinator must run 1.7 before the final merge gate.

## Ambiguities

A1. `--verbose` is described in neither SPEC nor README today, so "wherever `--verbose` is described" has no existing location. I added one paragraph to SPEC §3's preamble and one sentence to README's Usage paragraph. The alternative of adding nothing would leave spec §4.1's "README's silent-on-success claim then holds" without a stated flag. The flag also has a short form `-v` (`src/cli.rs:12`), which the added text names.

A2. Resolved. Changeset-plan contract 6 and T1's plan step 5 pin the `mlm week` layout: day rows, blank line, anomaly footer if any row is marked, the marker line directly under it (no blank between), one blank line, then the Carry-in block with `Carry-in:      -80h 00m  [!]`. With no marked row the marker line takes the footer's place. S11c and R2 are written to this layout. `[VERIFY AT MERGE]` the footer literal in the S11c sample.

A3. The sample date of the windowed example in README and SPEC (`09:00` and `10:30`) is illustrative. The spec gives `14:00` and `15:00` for E7 and nothing for stale and lookback lines.

A4. Spec §6 says SPEC records "visible consequences that survive". I read "survive" as "not already restated elsewhere", and recorded them in the sections that own the behavior (S10, S12) instead of adding a new "Consequences" section, because SPEC has no such section and §1.2a is being emptied.

A5. For `mlm start`'s yesterday warning with a windowed `fix:` line, spec §3.1 gives only the unwindowed shape. I state that the §3.2.2 rule applies to every warning (spec §3.2.2 last paragraph) and show no extra example for it.

## Disagreements with the changeset plan or spec

- None with the approach. The plan's wording "AGENTS.md Layout lists ... and the storage note" is vague about what the storage note is. I took it as the ranged target-override read that T1 adds to `src/storage.rs` (plan F6 constraints) and phrased it generally in case T1 names it differently.
- Spec §5.1 lists I23 in "Removed (fixed)" and spec §5.2 lists it in the §7.1 owner row. Both are satisfied by S10d.
- None on the marker position. Contract 6 now pins it (A2).
