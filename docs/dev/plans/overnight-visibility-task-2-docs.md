# Task 2 plan: docs (SPEC.md, README.md)

> **Archived — historical planning record.** Not authoritative. Current spec: `docs/dev/SPEC.md`. Design/process history: `docs/dev/NOTES.md`.

Changeset: overnight-visibility. Altitude: task plan (implementation level). No file is edited by this plan.

## Read

- `docs/dev/plans/overnight-visibility-plan.md` (changeset plan, in full)
- `docs/dev/specs/2026-10-04-overnight-visibility.md` (spec, in full)
- `docs/dev/SPEC.md` (all 1467 lines)
- `README.md` (all 536 lines)
- `AGENTS.md` (Documentation rule), `clippy.toml`, `.githooks/pre-commit` (docs-only task, so fmt/clippy/coverage gates see no change)
- `src/status.rs` lines 240-295 (`render`: confirms a blank line precedes the anomaly block and the stint list, so the page layout in the drafted examples below is right)
- Sibling task plans: none exist yet for this changeset (`ls docs/dev/plans | grep overnight` shows only the changeset plan). All shared text below is taken from the changeset plan's interface contracts 1-7 and spec §4/§5/§6.

## Scope

| Owned by this task | Owner of files not to touch |
|---|---|
| `docs/dev/SPEC.md`, `README.md` | `src/lookback.rs`, `src/status.rs`, `src/week_view.rs`, `src/render.rs`, `src/cli.rs`, `tests/*`: Task 1 (code) |
| | `docs/dev/NOTES.md`, the changeset spec/plan/review files, archiving: coordinator |

No handoff needed from Task 1. The only shared text is the pinned literals (contracts 1-6), quoted exactly below. The coordinator checks the docs against the built binary at merge.

## Pinned literals (quote exactly)

1. Today's stint line: `  22:00-now    (15h 30m so far, started yesterday, not counted until stopped)`
2. Week row marker: `  [i]` after the duration; `  [!] [i]` when both apply.
3. Week footer: `[i] open stint: run mlm status to stop it`
4. Monday footer: `[i] open stint on Sun 2026-10-04 (last week): run mlm status to stop it`
5. Footer order: `[!]` footer, `[i]` footer or Monday line, carry marker line, blank line.
6. Glossary last sentence: `It marks yesterday's row in `mlm week` until that start is stopped.`

## Facts gathered (current state)

### SPEC.md

| Line | Heading / passage | What it says now |
|---|---|---|
| 36-39 | §1.2 intro | "...not things known to be broken or confusing today. See §1.2a for that." Stays true. |
| 58-71 | §1.2a "Possible issues to revisit" | Intro paragraph, then one bullet "A stint started yesterday and still open after midnight is not on today's page..." (the last and only item). |
| 196 | §2.4 Derived, "Fulfillment / carry" | "A stint open across midnight gets no live credit (§7.1)." Stays true. |
| 311-312 | §3.5 `mlm status` contents | "- for today only, lookback lines for open starts from earlier dates and an `[i]` hint for a start still open from yesterday (§7.1)" |
| 315-318 | §3.5 tail | Glossary paragraph, no `[i]` detail. |
| 319-329 | §3.6 `mlm week` | Bullets: headline, one line per date, trailing block. No marker mention. |
| 831-835 | §7.1 "Stint lines" bullet | "Section omitted entirely when the date has zero stints..." |
| 867-877 | §7.1 "Yesterday hint, today's page only." | Ends: "The hint is not an anomaly. It does not mark a `mlm week` row, does not count toward the footer, and never shows on `status DATE` for another date. `status -1` shows yesterday's open start as a stint line only." |
| 894-906 | §7.1 closing example `mlm status 2026-01-05` | Past-date example. New overnight example goes before it. |
| 1003-1006 | §7.2 "Any date in the row list with an anomaly..." bullet | Row marker for `[!]` only. |
| 1007-1015 | §7.2 footer bullet | "...No footer when no row is marked. `week` runs no lookback and no yesterday hint. The row marker, the footer and the carry-in marker below are its only anomaly output." |
| 1017-1036 | §7.2 "Carry-in marker." bullet | Layout described as "the day rows, a blank line, the anomaly footer if any row is marked, the marker line directly under it..., one blank line, then the Carry-in block. With no marked row the marker line takes the footer's place". |
| 1045-1046 | §7.3 intro | "`[i] ` marks a hint that is not an anomaly." Stays true. |
| 1059 | §7.3 line order | "...then the yesterday hint. The carry-in marker line comes last." Stays true (the stint line is in the stint list, not the anomaly block). |
| 1184-1192 | §7.3 yesterday hint | "...`[i]` is the quiet marker. It never marks a `week` row and never counts toward the footer:" |
| 1206-1214 | §7.3 `week` inline marker | `  Wed 2026-02-11   08h 00m  [!]`, then "When at least one row carries `[!]`, one footer line follows the table (§7.2)." |
| 1449-1454 | E22 | Last line: "The hint never marks a `week` row and never shows on `status -1`." |

No other SPEC passage references §1.2a (grep: only line 39, which stays true). Grep for `[i]` finds only lines 70, 312, 869, 1045, 1184-1189, 1450 (plus the hint literal line 1189), all covered below.

### README.md

| Line | Heading / passage | What it says now |
|---|---|---|
| 43-54 | `### Possible issues` | Intro "Shipped behavior that's rough or confusing in a way worth fixing later:" then the overnight bullet. |
| 259-282 | `mlm status` hint paragraph | "A lone start still open from yesterday prints a quiet hint, which marks no `week` row:" then the `[i]` block, then the `or:` explanation. |
| 316-319 | `mlm status` estimated-EOD paragraph | "A stint still open after midnight is not on the new day's page, so it gets no live credit until you `stop` it." |
| 393-400 | `mlm week` footer paragraph | "When any row in the table above carries `[!]`, one line follows after a blank line... No footer line prints when nothing in the week is flagged." |
| 402-420 | `mlm week` carry-in paragraph | "It goes directly under the footer line when a row is flagged, and takes the footer's place otherwise, with one blank line before the Carry-in block." |

## Edit list

Apply SPEC.md edits bottom to top so earlier line numbers stay valid, or locate each by the quoted passage. Relocate by content.

### SPEC.md

#### S1. §1.2a, the one bullet

Keep heading and the intro paragraph. Replace the whole bullet (from "- A stint started yesterday and still open after midnight" through "...that points to `stop` (§7.1).") with:

```
None currently.
```

Reference check: SPEC.md line 39 ("See §1.2a for that.") stays true.

#### S2. §3.5, status contents bullet

Replace:

```
- for today only, lookback lines for open starts from earlier dates and
  an `[i]` hint for a start still open from yesterday (§7.1)
```

with:

```
- for today only, lookback lines for open starts from earlier dates, an
  `[i]` hint for a start still open from yesterday, and a stint line for
  that start ahead of today's own stints (§7.1)
```

Also replace the glossary paragraph's end. After "...its help line gains clap's `(see more with '--help')` suffix." append one sentence:

```
The `[i]` entry ends `It marks yesterday's row in `mlm week` until that start is stopped.`
```

#### S3. §3.6 `mlm week`, bullets

Insert after the "one line per date in the week with that date's total" bullet:

```
- yesterday's row ends with `[i]` while a start from yesterday is still
  open and unjoined, with an `[i]` footer line, and on a Monday the
  current week's view names Sunday's stint in that footer line (§7.2)
```

#### S4. §7.1, Stint lines bullet

Replace the sentence:

```
Section omitted
  entirely when the date has zero stints (a note-only day, or a fully
  empty one) — same treatment as Notes below, no empty-list
  placeholder either way.
```

with:

```
Section omitted
  entirely when the date has zero stints and no overnight stint line
  (below) (a note-only day, or a fully empty one) — same treatment as
  Notes below, no empty-list placeholder either way.
```

#### S5. §7.1, "Yesterday hint" bullet: rewrite the last two sentences, add two bullets

In the bullet beginning "Yesterday hint, today's page only.", replace the final text

```
  The hint is not an anomaly. It does not mark a `mlm week` row, does not
  count toward the footer, and never shows on `status DATE` for another
  date. `status -1` shows yesterday's open start as a stint line only.
```

with:

```
  The hint is not an anomaly. It never feeds `[!]`, the `[!]` footer, the
  carry-in marker or `has_any`, and it never shows on `status DATE` for
  another date. `status -1` shows yesterday's open start as a stint line
  only. The hint, the overnight stint line below and the `[i]` row mark
  in `mlm week` (§7.2) all fire on the same condition, so they appear
  and disappear together. Call that start the overnight stint.
```

Then insert two new bullets directly after it (before "Carry-in marker, any `DATE`."):

```
- Overnight stint line, today's page only. While the hint prints, the
  stint list opens with one extra line for the overnight stint, ahead of
  today's own stints:

  ```
    22:00-now    (15h 30m so far, started yesterday, not counted until stopped)
  ```

  The range uses the same column layout as other stint lines. The
  duration is whole minutes from the start to now in the standard
  formatter (§4.2), so it can pass 24h. The line is rendered on its own,
  outside today's pairing and sorting, so the list is not in clock order
  when it prints. It prints even when today has punches of its own, and
  it is the only line in the list whose minutes are not in the Day total.
  With no punches today the list holds just this line under `Day total:
  00h 00m`. With an open stint of today's own, both `-now` lines print,
  the overnight one first, and only today's own stint counts and carries
  `(incl. ongoing)` and `est. EOD`. `status DATE` for any other date,
  `status -1` included, keeps its own wording and prints no such line.
  The line goes away together with the hint: when a `stop` joins the
  start to today, when a refused-join line names yesterday, or when
  yesterday has two or more open starts. When the start turns two days
  old, the line and every `[i]` mark disappear and the `[!]` stale
  lookback line and week row marker take over, though the stint is still
  open.
- Overnight stint accounting. The overnight stint gets no live credit
  and no `est. EOD`. Its minutes stay out of the Day total, `Worked`,
  `Fulfillment` and the week line until `stop`, and `stop` returns them to
  yesterday. The Day total shows no `(incl. ongoing)` for it. At 00:00
  the week line jumps, because `required` steps up a weekday and the
  stint is not counted. Past Sunday midnight the closed week's `Worked`
  leaves the stint out until `stop` (§7.2).
```

#### S6. §7.1, new example before the `mlm status 2026-01-05` paragraph

Insert before "`mlm status 2026-01-05` (a past date...":

```
`mlm status` on Monday 2026-10-05 at 13:30, no punches today, with a
start at 22:00 still open from Sunday and a zero carry-in (week 2026-41
runs Monday to Sunday). The line is listed under the Day total and is
not part of it:

```
Mon 2026-10-05

Day total:     00h 00m
Week 2026-41:  08h 00m left to 08h 00m by end of Monday (fulfillment 00h 00m / target 40h 00m)

[i] start on Sun 2026-10-04 at 22:00 still open (yesterday; not counted until stopped)
    fix: mlm stop HH:MM --date 2026-10-04  (a time after 22:00)
     or: mlm stop  (still working: ends it now, across midnight)

  22:00-now    (15h 30m so far, started yesterday, not counted until stopped)
```
```

(Drafting note for the editor: the nested fence above is shown flat. In the file the example is one fenced block following one lead-in paragraph.)

#### S7. §7.2, footer bullet and `week` anomaly output

Replace the block from "- When at least one row carries `[!]`, one footer line follows the" through "...are its only anomaly output." with:

```
- When at least one row carries `[!]`, one footer line follows the
  table after a blank line:

```
[!] needs a look: run mlm status DATE for details and fixes
```

  No footer when no row is marked. `week` runs no lookback and prints no
  hint block. Its anomaly output has four parts: the row marker, the
  footer, the carry-in marker below, and the `[i]` row mark with its
  footer (below).
- Overnight stint mark. While today's `status` prints the `[i]` hint
  (§7.1), the row for yesterday ends with `  [i]`, after the duration and
  in the same column as `  [!]`. A row with both an anomaly and the
  overnight stint ends `  [!] [i]`, and both footers print. The mark shows
  in whichever shown week contains yesterday, the current week or a past
  week requested by `WEEK_ID`. No other row is marked, and a future week
  shows nothing. The mark follows the hint's own trigger, so it is absent
  when the start is joined to today, when a refused-join line names
  yesterday, when yesterday has two or more open starts, and when the
  start is two days old. When any row shows `[i]`, one footer line prints:

```
[i] open stint: run mlm status to stop it
```

  On a Monday, yesterday is last week's Sunday. The current week's view
  (the week containing today) has no marked row, so the `[i]` footer slot
  names the stint instead:

```
[i] open stint on Sun 2026-10-04 (last week): run mlm status to stop it
```

  The weekday and date are the stint's own, in the row display form. This
  line prints only in the view of the week containing today and only when
  the stint's date lies in the previous week, so a past week viewed by
  `WEEK_ID` never prints it and marks the Sunday row of last week's view
  instead. The plain footer and the Monday line never print together. Both
  depend only on the overnight trigger, never on whether a row also shows
  `[!]`. The Monday line still prints when last week's Sunday row carries
  `[!]`.
  `[i]` never feeds `[!]`, the `[!]` footer, the carry-in marker, `has_any`
  or any figure. A week with only an `[i]` row prints no `[!]` footer.
  `Worked`, `Fulfillment`, `Target` and the headline are unchanged by the
  mark, and past Sunday midnight the closed week's `Worked` leaves the
  stint out until `stop` (§7.1).
```

Sketch of the excerpt to add after the Monday-line text (a plain fenced block in the file):

```
  Sat 2026-10-03   00h 00m
  Sun 2026-10-04   00h 00m  [i]

[i] open stint: run mlm status to stop it
```

Placement: put this excerpt after the `[i]` footer literal, with the lead-in "The Sunday row of last week's view, on that Monday:". The `00h 00m` is correct because the overnight stint is not counted.

#### S8. §7.2, carry-in marker bullet, layout sentence

In the bullet "- Carry-in marker. When an earlier week has an anomaly...", replace:

```
The layout is the day rows, a blank line, the anomaly footer if
  any row is marked, the marker line directly under it (no blank line
  between), one blank line, then the Carry-in block. With no marked row
  the marker line takes the footer's place:
```

with:

```
The layout is the day rows, a blank line, the `[!]` footer if any
  row is marked, the `[i]` footer or Monday line if it applies, the
  marker line directly under the last footer printed (no blank line
  between), one blank line, then the Carry-in block. With no footer line
  the marker line takes the footers' place:
```

Also add one sentence at the end of that bullet's last paragraph ("It prints for any shown week, ... never counts the shown week's own anomalies."):

```
The blank line before the Carry-in block prints exactly when any footer or marker line printed.
```

#### S9. §7.3, yesterday hint passage

Replace:

```
Yesterday with exactly one unjoined open start gets an `[i]` hint
instead of an anomaly line (§7.1). `[i]` is the quiet marker. It never
marks a `week` row and never counts toward the footer:
```

with:

```
Yesterday with exactly one unjoined open start gets an `[i]` hint
instead of an anomaly line (§7.1). `[i]` is the quiet marker. It never
feeds `[!]`, the `[!]` footer, the carry-in marker or `has_any`. It also
marks yesterday's `week` row and adds the `[i]` footer there (§7.2),
and today's stint list gets a line for the same start (§7.1):
```

#### S10. §7.3, `week` inline marker passage

Replace the tail:

```
(detail deferred to that date's own `status` output rather than
repeated in the week table). When at least one row carries `[!]`, one
footer line follows the table (§7.2).
```

with:

```
(detail deferred to that date's own `status` output rather than
repeated in the week table). When at least one row carries `[!]`, one
footer line follows the table (§7.2). The overnight stint's row ends
with `  [i]` in the same column, or `  [!] [i]` when it also carries an
anomaly, and prints the `[i]` footer (§7.2).
```

#### S11. E22

Replace the E22 text from "- **E22** - Start still open from yesterday" through the end of its last line ("The hint never marks a `week` row and never shows on `status -1`.") with:

```
- **E22** - Start still open from yesterday (§7.1, §7.4): today's
  `status` prints the `[i]` hint, with the `or:` line only when no punch
  follows the start on yesterday's date and today has no punches. The
  stint list opens with `HH:MM-now (DURATION so far, started yesterday,
  not counted until stopped)` ahead of today's own stints, and no figure
  changes. A `stop` as today's first punch joins the start and the hint
  and the line are gone. A refused-join line naming yesterday, or two
  open starts yesterday, suppresses both. At age two the line and hint
  go and the `[!]` stale lookback takes over. The hint never shows on
  `status -1`, which keeps its own open-stint line.
- **E26** - Overnight stint in `week` (§7.2): while the hint prints,
  yesterday's row ends with `  [i]` and the `[i] open stint: run mlm
  status to stop it` footer prints once, after the `[!]` footer and
  before the carry-in marker line. A row with an anomaly too ends `  [!]
  [i]` and both footers print. A row with only `[i]` prints no `[!]`
  footer. On a Monday the current week's view has no marked row and
  prints `[i] open stint on Sun DATE (last week): run mlm status to stop
  it`, last week's view marks the Sunday row, and no other week prints
  anything. No `[i]` mark or footer appears when the start is joined,
  refused-join, doubled, or two days old.
```

Number E26 follows E25 (last item), so append it after E22's replacement only if the editor prefers topical order. Recommended: place E26 at the end of §8.2 after E25 to keep numbering sequential. Then E22 holds only its own bullet.

### README.md

#### R1. Possible issues

Replace the intro line and bullet (lines 45-54) with:

```
Shipped behavior that's rough or confusing in a way worth fixing
later.

None currently.
```

Heading `### Possible issues` stays.

#### R2. `mlm status` hint paragraph (lines 259-282)

Replace the sentence starting "A lone start still open from yesterday prints a quiet hint, which marks no `week` row:" with:

```
A lone start still open from yesterday prints a quiet hint. It is not
counted until you `stop` it, and it marks no anomaly:
```

Keep the existing `[i]` block and the `or:` paragraph. After the `or:` paragraph ("...the hint is gone. Yesterday with two or more open starts prints the `[!]` line ... instead.") insert, before "If an earlier week has a flagged date":

Not that sentence; keep the existing text order. Insert a new paragraph and example immediately after the `[i]` block's `or:` explanation paragraph ends ("...Yesterday with two or more open starts prints the `[!]` line `2 open stints on Fri 2026-10-02 (yesterday)` instead."), splitting so that "If an earlier week has a flagged date, the last line of the page names it:" starts its own paragraph:

```
The stint list also opens with a line for that start, ahead of today's
own stints. The time is not in the day total, and the line goes away
together with the hint:

```sh
$ mlm status
Mon 2026-10-05

Day total:     00h 00m
Week 2026-41:  08h 00m left to 08h 00m by end of Monday (fulfillment 00h 00m / target 40h 00m)

[i] start on Sun 2026-10-04 at 22:00 still open (yesterday; not counted until stopped)
    fix: mlm stop HH:MM --date 2026-10-04  (a time after 22:00)
     or: mlm stop  (still working: ends it now, across midnight)

  22:00-now    (15h 30m so far, started yesterday, not counted until stopped)
```
```

#### R3. `mlm status` est.-EOD paragraph (line 317-319)

Replace:

```
A stint
still open after midnight is not on the new day's page, so it gets no
live credit until you `stop` it.
```

with:

```
A stint
still open after midnight is listed on the new day's page but not
counted, so it gets no live credit and no `est. EOD` until you `stop` it.
```

#### R4. `mlm week` footer paragraph (lines 393-400)

After "No footer line prints when nothing in the week is flagged." insert:

```
While a start from yesterday is still open (the same case as the `[i]`
hint in `status`), yesterday's row ends with `[i]`, and a second line
follows the table. A row with an anomaly too ends `[!] [i]` and prints
both footers, `[!]` first. The mark is a hint and counts as no anomaly, and no figure changes:

```
  Sat 2026-10-03   00h 00m
  Sun 2026-10-04   00h 00m  [i]

[i] open stint: run mlm status to stop it
```

On a Monday, yesterday is last week's Sunday. The current week's view
has no marked row, so its footer line names the stint instead, and last
week's view marks the Sunday row. Other weeks print nothing:

```
[i] open stint on Sun 2026-10-04 (last week): run mlm status to stop it
```
```

(Reword "No footer line prints when nothing in the week is flagged." to "No `[!]` footer line prints when nothing in the week is flagged." because an `[i]`-only week prints the `[i]` footer without a `[!]` one.)

#### R5. `mlm week` carry-in paragraph (lines 402-408)

Replace "It goes directly under the footer line when a row is flagged, and takes the footer's place otherwise, with one blank line before the Carry-in block:" with:

```
It goes directly under the last footer line printed, and takes the
footers' place when none printed, with one blank line before the
Carry-in block:
```

## Test plan (grep/literal checks)

Run from the repo root after the edits. "Absent" means grep returns no hit.

### AC1. SPEC §1.2a reads "None currently." (spec §6, review F4)

- `sed -n '/^### 1.2a/,/^### 1.3/p' docs/dev/SPEC.md` shows the heading, the intro paragraph and `None currently.` and nothing else.
- Absent: `grep -n 'started yesterday and still open after midnight' docs/dev/SPEC.md`
- `grep -n '§1.2a' docs/dev/SPEC.md` finds only the §1.2 intro line, which still points at an existing heading.

### AC2. SPEC §7.1/§7.2 hold the moved behavior and the contract 1-4 texts (spec §6, §4, §5, review F7)

Each must appear exactly as pinned:

- `grep -c '22:00-now    (15h 30m so far, started yesterday, not counted until stopped)' docs/dev/SPEC.md` is at least 2 (§7.1 bullet and example).
- `grep -c '\[i\] open stint: run mlm status to stop it' docs/dev/SPEC.md` is at least 2 (§7.2 and E26).
- `grep -n '\[i\] open stint on Sun 2026-10-04 (last week): run mlm status to stop it' docs/dev/SPEC.md` finds the §7.2 line.
- `grep -n '\[!\] \[i\]' docs/dev/SPEC.md` finds the §7.2 shared-row text.
- Moved statements: `grep -n 'no live credit and no .est. EOD.' docs/dev/SPEC.md`; `grep -n 'returns them to' docs/dev/SPEC.md`; `grep -n 'steps up a weekday and the' docs/dev/SPEC.md`; `grep -n "leaves the stint out until" docs/dev/SPEC.md` each hit once in §7.1/§7.2.
- Footer order: §7.2's carry-in layout sentence names the `[!]` footer, then the `[i]` footer or Monday line, then the marker line, then the blank line.
- Age-two statement: `grep -n 'turns two days' docs/dev/SPEC.md` hits §7.1.

### AC3. No SPEC passage still says the hint never marks a week row or that week has no yesterday hint (spec §6, §8; plan AC3)

- Absent: `grep -n 'never marks a .week. row' docs/dev/SPEC.md`
- Absent: `grep -n 'does not mark a .mlm week. row' docs/dev/SPEC.md`
- Absent: `grep -n 'no yesterday hint' docs/dev/SPEC.md`
- Absent: `grep -n 'never marks a .week. row' docs/dev/SPEC.md README.md`
- Absent: `grep -n 'marks no .week. row' README.md`
- Review each of the passages by content: §3.5 status contents, §7.1 hint bullet, §7.2 anomaly-output sentence, §7.3 `[i]` quiet-marker passage (and the `week` inline marker), E22.

### AC4. SPEC states `[i]` never feeds `[!]`, the footer, the carry marker or `has_any` (spec §2)

- `grep -n 'never feeds' docs/dev/SPEC.md` hits §7.1, §7.2 and §7.3, and each hit names `[!]`, the `[!]` footer, the carry-in marker and `has_any`.

### AC5. README (spec §6)

- `sed -n '/^### Possible issues/,/^## Install/p' README.md` shows the heading and `None currently.` and no bullet.
- Absent: `grep -n 'jumps at 00:00' README.md`
- `grep -c '22:00-now    (15h 30m so far, started yesterday, not counted until stopped)' README.md` is 1.
- `grep -n '\[i\] open stint: run mlm status to stop it' README.md` and the Monday literal each hit once.
- Pinned text exact, in both files: run the five literal greps from the pinned list against `docs/dev/SPEC.md` and `README.md` and diff each hit against contract text.

### AC6. No new reader-facing pointers (plan AC6; AGENTS.md Documentation rule)

- `git diff -U0 docs/dev/SPEC.md README.md | grep '^+' | grep -n 'docs/dev/plans\|docs/dev/specs\|overnight-visibility'` finds nothing.
- Same grep over the full files shows no more hits than at baseline (`git stash`-free check: `git show HEAD:docs/dev/SPEC.md | grep -c 'docs/dev/plans\|docs/dev/specs'` equals the new count; likewise README.md).

### AC7. Formatting and ASCII

- `grep -nP '[^\x00-\x7F]' ` on the added lines shows only characters already used in the file (SPEC.md already uses em dashes in prose; do not add any in new literal lines). The pinned literals are pure ASCII.
- Backtick nesting in the glossary sentence matches the pinned contract text (`mlm week` in backticks).

### AC8. Consistency with the built binary (coordinator, at merge)

- After Task 1 merges, run `MLM_DB_PATH=<tmp> mlm status` and `mlm week` against a start left open from yesterday, and diff the printed lines against the SPEC §7.1 example, the README example and the pinned literals. The anchoring is the seeded run with `examples/seed_test_data.rs` plus one hand-typed `start --date -1 22:00`.

## Risks, ambiguities, disagreements

1. **No sibling plan exists yet.** Task 1's task plan is not on disk. Shared text comes only from the changeset plan's contracts. If Task 1 plans different wording for anything not pinned (for example, extra blank lines), the coordinator reconciles at merge via AC8.
2. **Example arithmetic assumes zero carry-in.** The Monday status example uses `fulfillment 00h 00m / target 40h 00m`, `08h 00m left to 08h 00m by end of Monday` (daily target 8h, weekday 1). This matches the SPEC's `required` rule (§2.4). If the coordinator wants a nonzero carry-in example, the expanded parenthetical form applies.
3. **README intro for Possible issues.** The spec says the section "keeps its heading and reads 'None currently.'" The plan keeps the one-line intro (with a period instead of a colon) and adds `None currently.`. SPEC §1.2a keeps its intro paragraph the same way. A stricter reading drops the intro. Cosmetic.
4. **Week-view dependence on `has_any`.** The plan says `[i]` never feeds `has_any`. A week with only a marked row but no other data is a corner the spec does not describe beyond "ignore `[i]`". The docs state the rule without describing that corner.
5. **Spec §8 "`[!]` precedence on a shared row" is stale** (changeset plan global constraints). This task does not copy that line. It writes the §5 rule (`  [!] [i]`, both footers).
6. **Spec §6 cites SPEC.md "around lines 311, 869-875, 1185, 1450".** Confirmed at those places. Two further passages the spec does not list also need edits: the §7.2 sentence "`week` runs no lookback and no yesterday hint..." (covered by S7, listed by the changeset plan) and the §7.2 carry-in layout sentence plus §7.3 week-row paragraph (S8, S10), which the spec's list omits. Treat the spec's list as non-exhaustive, as its "relocate by content" says.
7. **E26 is an addition.** Neither the spec nor the changeset plan asks for a new edge case. It exists because E22 is about `status` and the `week` behavior needs a user-flow entry (SPEC §8 is the test basis). If the coordinator prefers no new numbered case, fold E26 into E22.
8. **Nested code fences.** The drafted replacement text above shows fenced examples inside fenced drafts. In the file each example is one plain fenced block. The editor drafts flat.
9. **Status page blank-line layout.** The example places a blank line before the `[i]` block and before the stint list, taken from `src/status.rs` `render` (blank line before each non-empty section). Task 1 changes the section-print condition only, so the layout holds.
10. **Monday footer in a past-week view by id.** The docs state the spec's rule: never printed there, the Sunday row of last week's view is marked instead. Because the stint's date is in the shown week for last week's view, no conflict.
