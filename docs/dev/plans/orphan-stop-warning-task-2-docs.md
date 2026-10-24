# Task 2 plan: docs (SPEC.md, README.md, AGENTS.md)

Changeset: orphan-stop-warning. Altitude: task plan. No file is edited by this plan.

## Read

- `docs/dev/plans/orphan-stop-warning-plan.md` (changeset plan, all 97 lines)
- `docs/dev/specs/2026-10-24-orphan-stop-warning.md` (spec, all 127 lines)
- `docs/dev/plans/reports/orphan-stop-warning-rescope-plan-review.md` (lines 1-30, for F1/F2, which put AGENTS.md and spec §3-§5 in this task)
- `docs/dev/SPEC.md` (all 1593 lines)
- `README.md` (all 571 lines)
- `AGENTS.md` (all 165 lines, "Layout" and "Documentation")
- `docs/dev/README.md` (archive convention)
- `docs/dev/plans/overnight-visibility-task-2-docs.md` (lines 1-60, house format for a docs task plan)
- Sibling task plans: `docs/dev/plans/orphan-stop-warning-task-1-code.md` does not exist yet (`ls` fails). All shared text below comes from the changeset plan's interface contracts 1-5 and spec §3-§5.

## Scope

| Owned by this task | Files not to touch, and their owner |
|---|---|
| `docs/dev/SPEC.md`, `README.md`, `AGENTS.md` | `src/commands.rs`, `src/anomalies.rs`, `tests/orphan_stop_cli.rs`: Task 1 (code) |
| | `docs/dev/NOTES.md`, the working spec's archive banner, the changeset plan and reports: coordinator |

`--help` text is out of scope (spec §7).

## Dependencies

| Dependency | State | What this task uses | Stub today | Swap at integration |
|---|---|---|---|---|
| Task 1 (code) | not landed, plan not written | Only the pinned output text (contract 1) and the rules in spec §3-§5. No signature. | None. The docs quote contract 1 directly. | None. The coordinator checks the quoted block against the built binary at merge (see check 2.10). |

## Pinned literals (quote byte for byte)

Contract 1, with spec §4's FE5 values. `fix:` has 4 leading spaces, `or:` has 5, and two spaces precede each parenthetical.

```
warning: orphaned end on Mon 2026-10-05 at 14:16 (no matching start)
    fix: mlm start HH:MM --date 2026-10-05  (a time before 14:16)
     or: mlm delete punch --date 2026-10-05  (lists punches; rerun with the end's number)
```

The commands that produce it (contract 4): `$ mlm start 22:00 --date -2`, then `$ mlm stop`.

## Facts gathered

### SPEC.md

| Line | Section | Current text (relevant part) |
|---|---|---|
| 64 | §1.2a | `None currently.` |
| 225-227 | §3 intro | "...so a run that creates and migrates a new database prints nothing." |
| 256-258 | §3.3 | "Same shape as `start`, inserts an `end` punch." |
| 356 | §3.7 | "`start`, `stop` and `note` stay silent (§7.4)." This already ignores `start`'s warnings. |
| 454-456 | §4.3 edge-case lead-in | "...not silently fixed and not rejected at insert time (no editing/validation — see §1.2):" |
| 585-590 | §6 lead-in | "**anomalies** (§4.3) are accepted, stored, and surfaced later in `status`/`week` output instead..." |
| 644-646 | §6.2 | "...the anomaly shows up in that date's next `status`/`week` output, not at insert time." |
| 1227-1233 | §7.3 E8 block | The two hint lines that contract 1 copies. |
| 1330 | §7.4 opening | "`start`, `stop` and `note` print nothing on success." |
| 1340-1387 | §7.4 `start` exception | Ends with the best-effort paragraph at 1385-1387. The new `stop` text goes after line 1387, before `## 8.` at 1389. |
| 1460-1461 | F13 | "First run on a fresh database prints nothing on stderr." |
| 1488-1492 | E8 | No mention of a write-time warning. |
| 1541-1549 | E19 | `start`'s warning. E27 goes directly after it. |
| 1583 | E26 | Highest id today, so E27 is free. |

Recent §8.2 entries (E20-E26) use ` - ` after the id. E19 and older use ` — `. E27 follows the recent form.

### README.md

| Line | Section | Current text (relevant part) |
|---|---|---|
| 48 | Possible issues | `None currently.` |
| 115-117 | Usage | "Every `start`/`stop`/`note` call is silent on success, and nothing prints unless something went wrong. That holds on the first run too, when the database is created." |
| 176-184 | `mlm stop` | Intro paragraph and a `$ mlm stop 13:00` example. No warning text. |

### AGENTS.md

| Line | Section | Current text |
|---|---|---|
| 40-42 | Layout, `src/anomalies.rs` bullet | "...and their rendered lines. `status`, `week` rows and the carry scan all call it." |

## Edits

Each step gives the current text (the `old_string`) and the exact replacement. Keep surrounding lines as they are. Line numbers are from the current branch tip (`90e628e`) and shift as earlier steps land, so match on text.

### SPEC.md

#### S1. §1.2a gains the known issue (line 64)

Current:

```
None currently.
```

Replacement:

```
- Entering a stop before its start warns once. `mlm stop 17:00` then
  `mlm start 09:00` is legal on any date (§3.2), but the first command
  prints an orphan warning (§7.4). The second command clears it.
```

`None currently.` occurs once in SPEC.md (line 64), so the match is unique.

#### S2. §3 first-run sentence (lines 225-227)

Current:

```
The global `-v`/`--verbose` flag turns on debug log lines on stderr.
Without it only warnings and errors are logged, so a run that creates
and migrates a new database prints nothing.
```

Replacement:

```
The global `-v`/`--verbose` flag turns on debug log lines on stderr.
Without it only warnings and errors are logged, so a run that creates
and migrates a new database prints nothing. The one exception is a
first `mlm stop`, which warns about its orphaned end (§7.4).
```

#### S3. §3.3 points to §7.4 (line 258)

Current:

```
Same shape as `start`, inserts an `end` punch.
```

Replacement:

```
Same shape as `start`, inserts an `end` punch. After the write it warns
on stderr about each orphaned `end` on the date it wrote to (§7.4).
```

#### S4. §3.7 names the `stop` exception (line 356)

Current:

```
`start`, `stop` and `note` stay silent (§7.4).
```

Replacement:

```
`note` stays silent. `start` and `stop` print nothing to stdout, and
print only warnings to stderr (§7.4).
```

#### S5. §4.3 edge-case lead-in (lines 454-456)

Current:

```
Edge cases, still just **detected and surfaced in that date's
summary** (§3.5/§3.6 output), not silently fixed and not rejected at
insert time (no editing/validation — see §1.2):
```

Replacement:

```
Edge cases, still just **detected and surfaced in that date's
summary** (§3.5/§3.6 output), not silently fixed and not rejected at
insert time (no editing/validation — see §1.2). The one insert-time
signal is `stop`'s stderr warning for an orphaned `end` on the date it
wrote to (§7.4). The punch is stored either way:
```

#### S6. §6 lead-in (lines 585-590)

Current:

```
**anomalies** (§4.3) are accepted, stored, and surfaced later in
`status`/`week` output instead — because there's no in-place editing
(§1.2), refusing to store a punch that merely produces a weird pairing
would leave the user with no way to fix it.
```

Replacement:

```
**anomalies** (§4.3) are accepted, stored, and surfaced later in
`status`/`week` output instead — because there's no in-place editing
(§1.2), refusing to store a punch that merely produces a weird pairing
would leave the user with no way to fix it. `stop` also surfaces an
orphaned `end` right away, as a stderr warning after the write (§7.4),
and still exits `0`.
```

#### S7. §6.2 "not at insert time" (lines 644-646)

Current:

```
- Any §4.3 pairing anomaly (extra open stints, an orphaned `end`): the
  punch is still inserted; the anomaly shows up in that date's next
  `status`/`week` output, not at insert time.
```

Replacement:

```
- Any §4.3 pairing anomaly (extra open stints, an orphaned `end`): the
  punch is still inserted, and the anomaly shows up in that date's next
  `status`/`week` output. A `stop` to a date that holds an orphaned
  `end` also warns on stderr right after the write (§7.4), and still
  exits `0`.
```

#### S8. §7.4 opening rule (line 1330)

Current:

```
`start`, `stop` and `note` print nothing on success. They are silent,
```

Replacement:

```
`start`, `stop` and `note` print nothing on success, apart from the
stderr warnings `start` and `stop` print (below). They are silent,
```

The rest of the paragraph (from "Unix-conventional, and exit `0`" on) stays as it is.

#### S9. §7.4 gains `stop`'s trigger, block and best-effort rule (insert after line 1387)

Current (the anchor, lines 1385-1387, unchanged):

```
The check is best-effort: if the lookback read fails after the write
already committed, `start` still prints nothing extra and still exits
`0`.
```

Insert after it, with one blank line before and after:

````
`stop` has its own narrow exception, also on stderr only. After a
successful write to date `D`, it classifies `D` with the splice
applied, exactly as `D`'s `status` page does (§4.3.1). It prints one
warning block per orphaned end on `D`, ascending by instant, with no
blank line between blocks. That includes an end the new punch
displaced, and an orphan that was already on `D` before the write, so
every `stop` to `D` reports the orphans `D` holds. An end that splices
with the previous date's open start is not orphaned and draws no
warning. `--date` changes `D`, not the rule. `stop` checks no other
date and warns about nothing else, so an overlap or a second open start
it creates waits for `status`. Stdout stays empty and the exit code
stays `0`.

The header names the date, which `status`'s E8 header leaves to its
page. The two hint lines are `status`'s E8 lines (§7.3) word for word.
With today Mon 2026-10-05, a start from two days back is still open and
a bare `stop` lands on today at 14:16. The splice joins adjacent dates
only, so today's end stays orphaned:

```
$ mlm start 22:00 --date -2
$ mlm stop
warning: orphaned end on Mon 2026-10-05 at 14:16 (no matching start)
    fix: mlm start HH:MM --date 2026-10-05  (a time before 14:16)
     or: mlm delete punch --date 2026-10-05  (lists punches; rerun with the end's number)
```

`status` shows the stale start that the end failed to close. Entering a
stop before its start warns once (§1.2a).

This check is best-effort too. If any read after the committed write
fails, `stop` prints nothing at all and still exits `0`. A failed
neighbor read never counts as an empty date, since that would turn a
spliced end into a false orphan. A failed write to stderr is ignored
and never changes the exit code. A hard error before the write (§6.1)
is unchanged.
````

#### S10. F13 (lines 1460-1461)

Current:

```
- **F13** - First run on a fresh database prints nothing on stderr. With
  `--verbose` the migration line and debug lines appear (§3).
```

Replacement:

```
- **F13** - First run on a fresh database prints nothing on stderr,
  except that a first `mlm stop` warns about its orphaned end (§7.4,
  E27). With `--verbose` the migration line and debug lines appear (§3).
```

#### S11. E8 names E27 (line 1492)

Current:

```
  Two or more orphaned ends on the same date each get their own line.
```

Replacement:

```
  Two or more orphaned ends on the same date each get their own line.
  A `stop` to that date also warns about each one on stderr (E27).
```

This line occurs once in SPEC.md (line 1492). Grep before editing to confirm.

#### S12. E27 directly after E19 (insert after line 1549)

Anchor (end of E19, unchanged):

```
  `or:` line (§7.4). `start` prints no carry-in marker.
```

Insert directly after it, with no blank line (the §8.2 list has none):

```
- **E27** - `mlm stop`'s orphaned-end warning (§7.4): a successful
  `stop`, today or backdated, classifies the date it wrote to with the
  splice applied and prints one `warning: orphaned end on <Wdy
  YYYY-MM-DD> at <HH:MM> (no matching start)` block per orphaned end
  there, ascending, each with `status`'s E8 `fix:`/`or:` lines. Stdout
  stays silent and exit stays `0`. The FE5 case (a start two or more
  days old, then a bare `stop` today) warns. So does an end the new
  punch displaced (`start 09:00`, `stop 17:00`, `stop 12:00` warns for
  17:00) and an orphan already on the date. A clean pair and an end that
  splices with the previous date's open start stay silent. A read
  failure after the write prints nothing, and the punch is stored.
```

### README.md

#### R1. Possible issues mirrors §1.2a (line 48)

Current:

```
None currently.
```

Replacement:

```
- Entering a stop before its start warns once. `mlm stop 17:00` then
  `mlm start 09:00` is fine on any date, but the first command prints
  an orphaned-end warning (see `mlm stop` below). The second command
  clears it.
```

`None currently.` occurs once in README.md (line 48).

#### R2. Silent-on-success and first-run sentences (lines 115-117)

Current:

```
Every `start`/`stop`/`note` call is silent on success, and nothing
prints unless something went wrong. That holds on the first run too,
when the database is created. `week target` prints one line confirming
```

Replacement:

```
Every `start`/`stop`/`note` call prints nothing to stdout on success.
On stderr, `start` warns about open starts it finds and `stop` warns
about orphaned ends on the date it wrote to (see below). Nothing else
prints unless something went wrong. That holds on the first run too,
when the database is created, except that a first `mlm stop` warns
about its orphaned end, since no start exists yet for it to pair with.
`week target` prints one line confirming
```

The remainder of the paragraph (from "the stored target." on) stays as it is. The implementer may reflow the last line with the next.

#### R3. `mlm stop` gains the warning and the FE5 example (insert after line 184)

Anchor (lines 182-184, unchanged):

````
```sh
$ mlm stop 13:00
```
````

Insert after it, with one blank line before and after:

````
After a successful write, `stop` checks the date it wrote to and prints
a `warning:` block to stderr for each orphaned end there (an end with
no start to pair with), earliest first. Stdout and the exit code are
unaffected. That includes an end the new punch displaced and one that
was already on the date. An end that joins the previous day's open
start across midnight is not orphaned and draws no warning. The common
case is a start left open two or more days ago, then a bare `stop`
today. A join reaches across one midnight only, so today's end stays
orphaned. With today Mon 2026-10-05:

```sh
$ mlm start 22:00 --date -2
$ mlm stop
warning: orphaned end on Mon 2026-10-05 at 14:16 (no matching start)
    fix: mlm start HH:MM --date 2026-10-05  (a time before 14:16)
     or: mlm delete punch --date 2026-10-05  (lists punches; rerun with the end's number)
```

`status` shows the old start that the end failed to close. If `stop`
cannot read the database after the write, it skips the warning and
still exits 0.
````

### AGENTS.md

#### A1. `src/anomalies.rs` bullet names `stop` (lines 40-42)

Current:

```
- `src/anomalies.rs` — per-date anomaly facts (E7, stale, E8, overlap,
  refused joins) built from raw punch buckets, and their rendered
  lines. `status`, `week` rows and the carry scan all call it.
```

Replacement:

```
- `src/anomalies.rs` — per-date anomaly facts (E7, stale, E8, overlap,
  refused joins) built from raw punch buckets, and their rendered
  lines. `status`, `week` rows, the carry scan and `stop`'s
  orphaned-end warning all call it.
```

## Check plan

Run every grep from `/home/magikmw/projects/mlm-cli`. This task touches no Rust, so `cargo fmt`, `clippy` and coverage see no change. No test reads doc text. `tests/status_cli.rs`, `tests/e6_db_failure.rs` and `tests/ascii_sweep.rs` mention SPEC.md only in module comments, so `cargo test` is optional for this task.

### 2.1 SPEC §1.2a gains the known issue

- `grep -n "Entering a stop before its start warns once" docs/dev/SPEC.md` gives one hit, inside §1.2a (between `### 1.2a` and `### 1.3`).
- `grep -c "None currently" docs/dev/SPEC.md` gives 0.

### 2.2 SPEC §3 and F13 note the first `mlm stop`

- `grep -n "first \`mlm stop\`" docs/dev/SPEC.md` gives two hits, one in §3's intro (before `### 3.1`) and one in the F13 bullet.

### 2.3 SPEC §3.3 points to §7.4

- Read the `### 3.3` section. Its body ends with "(§7.4)." and mentions an orphaned `end`.

### 2.4 SPEC §3.7 names the `stop` exception

- `grep -n "stay silent (§7.4)" docs/dev/SPEC.md` gives 0 hits.
- `grep -n "\`start\` and \`stop\` print nothing to stdout" docs/dev/SPEC.md` gives one hit in §3.7.

### 2.5 SPEC §4.3, §6 and §6.2 name the `stop` warning

- `grep -n "not at insert time" docs/dev/SPEC.md` gives 0 hits.
- Read the §4.3 edge-case lead-in, the §6 lead-in paragraph and the §6.2 anomaly bullet. Each names `stop`'s stderr warning and cites §7.4.

### 2.6 SPEC §7.4 states trigger, block, best-effort rule and the exception

- Read §7.4. The opening sentence names the `start` and `stop` stderr warnings as the exception to "print nothing on success".
- The new paragraphs cover the splice-applied classification of `D`, displaced and pre-existing orphans, the splice exemption, `--date`, ascending order with no blank line, the FE5 block, and the best-effort rule including the failed-neighbor-read sentence.

### 2.7 SPEC §8.2 gains E27 next to E19, and E8 names it

- `grep -n "E27" docs/dev/SPEC.md` gives hits in E8, F13 and the E27 bullet.
- `grep -n "^- \*\*E" docs/dev/SPEC.md` lists E27 directly after E19.

### 2.8 README.md changes

- `grep -n "silent on success" README.md` gives 0 hits, and `grep -n "\`stop\` warns" README.md` gives one hit in the Usage paragraph.
- `grep -n "first \`mlm stop\` warns" README.md` gives one hit.
- `grep -n "warning: orphaned end on" README.md` gives one hit, inside the `### \`mlm stop|e` section.
- `grep -n "Entering a stop before its start warns once" README.md` gives one hit under `### Possible issues`, and `grep -c "None currently" README.md` gives 0.

### 2.9 AGENTS.md `src/anomalies.rs` bullet names `stop`

- `grep -n "orphaned-end warning all call it" AGENTS.md` gives one hit.

### 2.10 Every quoted warning matches contract 1 byte for byte

`-F` makes grep match the literal text, spaces included.

```sh
grep -cF 'warning: orphaned end on Mon 2026-10-05 at 14:16 (no matching start)' README.md docs/dev/SPEC.md
grep -cF '    fix: mlm start HH:MM --date 2026-10-05  (a time before 14:16)' README.md docs/dev/SPEC.md
grep -cF '     or: mlm delete punch --date 2026-10-05  (lists punches; rerun with the end'"'"'s number)' README.md docs/dev/SPEC.md
```

Expected counts are README.md 1 and docs/dev/SPEC.md 1 for each line. Also confirm the E8 lines in §7.3 still read `    fix: mlm start HH:MM --date D  (a time before 18:00)` and `     or: mlm delete punch --date D  (lists punches; rerun with the end's number)`, unchanged. If Task 1 has merged by then, run the FE5 sequence against a scratch DB with `TZ=UTC` and an explicit TIME (`mlm stop 14:16`) and diff its stderr against the README block.

### 2.11 No pointer out to working docs

- `grep -n "docs/dev/specs\|docs/dev/plans\|orphan-stop-warning" README.md docs/dev/SPEC.md AGENTS.md` gives no new hit.
- Every section reference in the new text is a SPEC.md section (§1.2a, §3.2, §4.3.1, §6.1, §7.3, §7.4), never a working-spec section.

## Risks and ambiguities

- **E27's position.** Spec and plan both say "next to E19", so E27 sits between E19 and E20 and breaks numeric order. Putting it after E26 would keep order, but this plan follows the pinned wording.
- **§3.7 and §6.2 already ignored `start`'s warnings.** The current §3.7 says all three commands "stay silent", and §6.2 says anomalies never show at insert time, though `start` already warns about E7 on the date it wrote to. S4 and S7 reword both so they stay true for `start` as well. That is a little wider than "name the `stop` exception", but leaving the old wording would keep a false statement next to the new one.
- **`src/history.rs` bullet in AGENTS.md.** Task 1 uses the three-date bucket view from `src/history.rs`, but that bullet says the buckets serve "`status` and `mlm week`". The changeset plan scopes only the `src/anomalies.rs` bullet. The coordinator may want the history bullet touched too. This plan leaves it alone.
- **`delete` exception wording in §7.4.** The paragraph calls `delete` "a narrow, deliberate exception to the write-command-silence rule above". After S8 that rule already names two exceptions. The sentence stays true, so it is left alone.
- **No Task 1 plan to cross-check.** The docs rely only on contract 1 text and the spec's rules. If Task 1 deviates (say, a blank line between blocks), check 2.10's binary diff catches it at merge.
