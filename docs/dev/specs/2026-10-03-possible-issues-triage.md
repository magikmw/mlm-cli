# Possible-issues triage — design spec

> **Archived — historical changeset spec.** Superseded by `docs/dev/SPEC.md`, which folds in the decisions made here.

**Baseline**: written against `main` at `be85617` (`v0.3.6` plus the
unreleased `open-punch-visibility`, `live-eod` and
`point-in-time-week` changesets).
**Status**: adversarially reviewed 2026-10-03
(`docs/dev/plans/reports/possible-issues-triage-spec-review.md`,
needs-rework, 11 findings + 10 undeclared gaps, folded — NOTES.md
entry 97). Locked.
**Inputs**: NOTES.md entry 96 (triage), validation report
`docs/dev/plans/reports/possible-issues-triage-validation-review.md`
(item ids I1-I34 follow SPEC §1.2a order at baseline).

## 1. Summary

SPEC §1.2a lists 34 possible issues. A few produce a wrong figure or
hide a mistake. Most describe correct behavior of the live,
point-in-time week line. This changeset:

- fixes the items that hide mistakes or skew numbers silently (§3);
- applies cheap polish to wording, `--help` and logging (§4);
- moves accepted behavior out of §1.2a into the spec sections it
  belongs to, and trims README's list (§5).

| Item(s) | Change | Section |
|---|---|---|
| I3, I14 | Hint on today's `status` and `mlm start` for a start still open from yesterday | §3.1 |
| I2 | Overlapping stints become an anomaly; stop hints name a time that pairs cleanly | §3.2 |
| I1, I4 | `[!]` line for the carry-in when an earlier week has an anomaly | §3.3 |
| I28 | `mlm week` read failures become hard errors | §3.4 |
| I31 | `mlm week target` echoes the stored target | §3.5 |
| I34 | Default log level `Warn` | §4.1 |
| I33 | Term glossary in `status --help` and `week --help` | §4.2 |
| I32 | Targeted error and usage line for `week target` argument order | §4.3 |
| I23 | `target already met` on the Day total line becomes `done for today` | §4.4 |
| I5 | `status DATE` rejects a future date | §4.5 |
| rest | Restated as behavior in SPEC; removed from §1.2a | §5 |

## 2. Non-goals

- No change to pairing (SPEC §4.3) or the splice (§4.3.1). Overlapping
  stints still both count in the day total; §3.2 only flags them.
- No live credit for a stint open across midnight. Its minutes still
  leave today's figures until `stop` (I14's counting half stays in
  §1.2a).
- No change to the 2-day stale threshold. A start open from yesterday
  gets a hint, not an anomaly.
- No carry marker or lookback change in `mlm start`'s stderr output
  beyond §3.1. The carry marker is a read-side view (`status`, `week`).
- No change to `est. EOD`'s formula, the week-line wording, or carry
  arithmetic.
- No change to stored data or the schema.
- An open start at `23:59` with nothing after it keeps its existing
  hint (`a time after 23:59`), which no `HH:MM` satisfies.
  Pre-existing; a same-instant stop pairs zero-length (E14).

## 3. Fixes

### 3.1 Start still open from yesterday (I3, I14)

Today's `status` gains a hint when yesterday has exactly one open
start that stays unjoined once today's punches are taken into account
(§4.3.1). The check reads yesterday's and today's punches. A `stop`
typed today as today's first punch joins the start and clears the
hint.

The hint is not an anomaly. It prints with `[i]`, not `[!]`, does not
mark `mlm week` rows, and does not count toward the week footer.

```
[i] start on Fri 2026-10-02 at 18:02 still open (yesterday; not counted until stopped)
    fix: mlm stop HH:MM --date 2026-10-02  (a time after 18:02)
     or: mlm stop  (still working: ends it now, across midnight)
```

- The `or:` line prints only when today has no punches yet. Only then
  does a `stop` now become today's first punch and join.
- The `fix:` time follows the clean-pair rule (§3.2.2).
- No hint when today's page already shows a refused-join line naming
  yesterday (§7.3). That line and the E7/E8 lines already cover the
  case.
- Placement: after today's anomaly block and lookback lines, before
  the carry-in line (§3.3).
- Yesterday with two or more open starts prints the existing lookback
  E7 line with `(yesterday)` in place of `(N days ago)`:
  `[!] 2 open stints on Fri 2026-10-02 (yesterday)` /
  `fix: see mlm status 2026-10-02`. Today's lookback window becomes
  ages 1-14 for that case; age 1 with one open start uses the hint
  above.

`mlm start` gains the same check in its stderr lookback (§7.4). After
a successful write, a single unjoined open start dated yesterday
warns:

```
warning: start on Fri 2026-10-02 at 18:02 still open (yesterday)
    fix: mlm stop HH:MM --date 2026-10-02  (a time after 18:02)
```

No `or:` line, because today now has a punch. Yesterday with two or
more open starts warns in the existing one-line form with
`(yesterday)`. The just-written-date exemption and the best-effort
rule from §7.4 apply unchanged.

### 3.2 Overlapping stints (I2)

#### 3.2.1 Overlap anomaly

Two stints on one date overlap when each starts before the other
ends (`a.start < b.end` and `b.start < a.end`). Both must have
nonzero length. Identical ranges and nests that share one endpoint
overlap. Stints that only touch at an endpoint do not.

The check runs over the stints the date's page shows: closed stints,
and, for today only, the most recent open stint taken up to now. A
past date's open stint counts zero minutes, so it is left out. A
spliced cross-midnight stint takes part on each date with the part
that falls on that date.

Each overlapping pair gets one line:

```
[!] overlapping stints 09:00-12:00 and 10:00-11:00 (the overlap counts twice)
    fix: mlm delete punch --date D  (lists punches; rerun with the number of the punch to remove)
```

- An open stint prints as `09:00-now`.
- Pairs sort by the first stint's start, then the second's start.
- Order in a date's anomaly block: after orphaned ends (E8), before a
  refused join.
- The date's `mlm week` row gets `[!]` and the date counts toward the
  week footer, like any other anomaly.

#### 3.2.2 Clean-pair rule for stop hints

A stop hint targets one open start `S`. Let `N` be the first punch on
that date after `S`. A stop between `S` and `N` pairs with `S` and
nothing else. A stop after `N` pairs with a later start or nests `S`
around later stints (§4.3).

- No punch after `S`: the hint reads `(a time after S)`, as today.
- A punch after `S`: the hint reads `(a time between S and N)`.
- The window includes `N`'s minute. `N` is always a start, and a stop
  at the same instant pairs with `S` (§4.3 step 2), so the window holds
  a legal minute even when `N` is one minute after `S`.
- Punches at the same minute as `S` do not count as after `S`.

E7 already targets the latest open start, and its hint keeps its
`closes the S start` tail:
`(a time between 14:00 and 15:00 closes the 14:00 start)`.

This applies to the stale-start line, the E7 line, today's lookback
lines, `mlm start`'s warnings, and the hint in §3.1. Output changes
only where a punch follows the targeted start.

### 3.3 Carry-in marker (I1, I4)

The shown week's carry-in comes from every earlier week, from the
earliest week with any punch up to the week before the shown one.
When any of those weeks has at least one date whose `mlm week` row
would carry `[!]` (E7, stale start, E8, refused join, overlap), a
marker line prints. The lookback window does not limit it.

`status` (any date) prints the line after its anomaly block and any
§3.1 hint:

```
[!] carry-in includes anomalies from 2 earlier weeks, oldest 2026-38 (see mlm week 2026-38)
```

With one week it reads `from 1 earlier week, 2026-38 (see mlm week
2026-38)`.

`mlm week` marks the Carry-in figure and prints the same line after
the footer:

```
Carry-in:      -80h 00m  [!]
```

A week with no earlier data, or whose earlier weeks have no anomaly,
prints neither. The marker uses the same per-date anomaly predicate
as the `mlm week` row marker, so the two never disagree.

### 3.4 `mlm week` read failures (I28)

Every storage read that `mlm week` makes is a hard error, matching
`status`: the target-override read, the earliest-week read and the
carry walk's reads. A failed read prints the storage error to stderr
and exits `1`, like every other runtime error (`src/main.rs`). Before,
those three reads fell back to defaults (the 40h target, no earlier
weeks), and `mlm week` exited `0` with wrong figures. Successful
output does not change. The mechanism (fallible traits, or a
prefetched ledger like `status::build_ledger`) is a plan decision.

### 3.5 `mlm week target` output (I31)

A successful `mlm week target` prints one line to stdout:

```
Week 2026-41 target: 20h 00m
```

The week id is the one written to (the current week when omitted).
The duration uses the standard formatter (§4.2). §3.7 and §7.4 drop
`week target` from the silent commands. `start`, `stop` and `note`
stay silent.

## 4. Polish

### 4.1 Default log level (I34)

Without `--verbose`, the log level is `Warn`. The first-run migration
line (`INFO rusqlite_migration`) no longer prints. `--verbose` keeps
`Debug`. README's "silent on success" claim then holds on first run.

### 4.2 Term glossary in `--help` (I33)

`mlm status --help` and `mlm week --help` end with a short glossary.
It defines, in one or two lines each:

- worked (closed stints, plus the running stint in the current week);
- carry-in (the balance carried from earlier weeks; a shortfall is
  negative, a surplus positive, per SPEC §1.3 and §2.4);
- fulfillment (worked plus carry-in);
- the week line's `left to` / `over` figure, and `by end of
  <weekday>` (Mon-Thu, the slice due by end of today) versus
  `<target> target` (Fri-Sun);
- `est. EOD` (when the open stint closes today's gap) and `done for
  today`;
- `(incl. ongoing)` and `(+ ongoing)`;
- `[!]` and `[i]`.

Short `-h` keeps its text, except that clap's help line gains its standard `(see more with '--help')` suffix once a long help exists. Exact wording is a task-plan item, checked
against SPEC §1.3, §2.4, §7.1 and §7.2.

### 4.3 `week target` argument order (I32)

- `mlm week target --help`, and every clap error for that subcommand,
  name the arguments `[WEEK_ID] <DURATION>` instead of `<ARGS>...`, in
  both the usage line and the Arguments section.
- `mlm week WEEK_ID target DURATION` fails with this message on
  stderr and exit `2`, the same code as clap's own usage errors:
  `error: put WEEK_ID after target: mlm week target 2026-41 20h`
  The message echoes the user's own week id and duration. Clap
  rejects this shape at parse time today, so the check has to catch
  it before or instead of clap's generic error. The mechanism is a
  plan decision.

### 4.4 `done for today` (I23)

The Day total line's `target already met` becomes `done for today`.
Where it shows and what triggers it do not change (§7.1). It reads as
"what's due by end of today is covered", which holds Mon-Thu (the
slice) and Fri-Sun (the full target).

### 4.5 `status DATE` rejects a future date (I5)

`status DATE` with `DATE` after today fails like `start`/`stop`/`note`
with a future `--date` (§6.1). It uses the same message shape and
exits `1`. §6.1's line that says `status` has no future restriction
is replaced. `mlm week WEEK_ID` for a future week stays allowed.

### 4.6 Old unclosed stint line (fresh-eyes FE4, added at phase 15)

A stint line for an open start two or more days old prints its range
as `09:00-?`, not `09:00-now`, so a 21-day-old start no longer reads
as running now. The `(unclosed)` tag and column alignment stay:

```
  09:00-?      (unclosed)
```

Today's and yesterday's open stints keep `-now`.

## 5. Documentation

### 5.1 SPEC §1.2a after this changeset

Removed (fixed): I2, I3, I4, I5, I23 (wording), I28, I31, I32, I33,
I34, and I1 (its wrong carry-in is now marked; dropping out of today's
lookback after 14 days becomes stated behavior in §7.1).

Removed (restated as behavior, §5.2): I6-I13, I15-I22, I24-I27,
I29, I30.

Kept: I14's counting half. A stint open across midnight gets no live
credit, its minutes leave today's figures until `stop`, and the week
line jumps at 00:00. Reworded to point at the §3.1 hint.

### 5.2 Restated behavior

Each removed Tier 3 item becomes a plain statement in the section
that owns the behavior, with no "issue" framing. If that section
already says it, nothing is added.

| Items | Owner |
|---|---|
| I6, I7, I15-I18, I23, I25-I27, I30 | §7.1 week line / Day total |
| I8, I19, I24 | §2.4 (live minutes, carry) |
| I9-I11, I22 | §7.1 `est. EOD` |
| I12, I13 | §7.3 E7 |
| I20, I21 | §7.1 `status DATE` |
| I29 | §7.2 future week |

### 5.3 SPEC sections this changeset rewrites

| Change | Sections |
|---|---|
| §3.1 hint, lookback ages 1-14 | §7.1, §7.3, §7.4 |
| §3.2 overlap anomaly | §4.3 edge cases, §7.3 |
| §3.2.2 hint times (every `a time after <start>` example) | §7.3, §7.4 |
| §3.3 carry marker | §7.1, §7.2, §7.3 |
| §3.4 read errors | §6 (runtime errors), §7.2 |
| §3.5 echo | §3.7, §7.4 |
| §4.1 log level | wherever `--verbose` is described |
| §4.3 argument order | §3.7, §6.1 |
| §4.4 wording | §7.1 |
| §4.5 future `status DATE` | §3.5, §6.1 |
| new and changed flows | §8 (including E16, E18, E19 where they touch hint times or `status DATE`) |

### 5.4 README

The Possible issues section keeps only what stays in §1.2a (one
item). README's Usage and output examples follow §3 and §4, including
the `week target` output and the "silent on success" claim. The
README/SPEC wording drift from the validation report (F8) goes away
with the list.

## 6. Visible consequences

- Overlap detection and the carry marker apply to stored history. A
  user with an old unresolved anomaly sees new `[!]` lines on first
  run after upgrade.
- The carry marker stays on every later week until the old anomaly is
  fixed. That is the point; it is also persistent noise for a user who
  chooses to leave old data alone.
- The carry marker line names a carry-in that `status`'s week line
  shows only inside its parenthetical.
- `done for today` can sit next to `(incl. ongoing)` while a stint
  runs, and can show with little or no time worked when carry-in
  already covers what is due. At `gap == 0` the week line still reads
  `00h 00m left to ...` beside `done for today`.
- With `Warn` as default, a newly created database prints nothing.
  `--verbose` shows the migration line.
- `status` for a future date is no longer a preview. `mlm week` for a
  future week still prints its figures, including `Total behind`.
- Stop hints with a later punch after the targeted start now name a
  window (`between S and N`) instead of an open-ended time.
- A legitimate overnight session shows the `[i]` hint on every
  `status` until it is stopped.
- Overlapping stints still double count. The anomaly line says so,
  and I2 leaves §1.2a because the double count is no longer silent.
- `mlm week target` writes to stdout. Scripts that expected silence
  see one line.
- `mlm week` exits `1` on a storage read failure where it used to exit
  `0` with default figures.
- The `[i]` hint shows on today's page only. `status -1` shows
  yesterday's open start as a stint line with no hint.
- `mlm start` repeats the yesterday warning on every `start` while
  that start stays open.
- At age 1 an open start gets a quiet `[i]` hint. At age 2 it becomes
  a `[!]` stale anomaly that marks its `mlm week` row and the footer.
- The carry marker counts every anomaly kind, including an orphaned
  end that adds no minutes. It can name a week whose figures are
  right.
- The marker looks only at weeks before the shown week. Viewing an
  older week does not mention anomalies in later weeks, and viewing
  the anomalous week itself shows `[!]` rows but no marker.
- The argument-order message always spells `mlm week target`, even
  when the user typed the `w` alias, and prints no `Usage:` block.
- The `week target` echo uses the standard formatter, so `90m` echoes
  as `01h 30m`.
- `mlm status -h` and `mlm week -h` show `Print help (see more with
  '--help')` on the help line.

## 7. Testing

- Each §3 and §4 item gets unit or integration tests written first
  (TDD), against the exact output lines above.
- §3.1: no punches today (both lines), punches today (no `or:`), a
  `stop` today that joins (no hint), a refused join (no hint), two
  open starts yesterday, `mlm start` warning.
- §3.2: nested, identical, shared-endpoint, touching (no anomaly),
  zero-length (no anomaly), today's open stint containing a closed
  one, past open stint (no anomaly), deep nest pair count.
- §3.2.2: each hint kind with and without a following punch.
- §3.3: anomaly in a week older than 14 days, none, several weeks;
  `status` and `mlm week` agree.
- §3.4: each of the three reads failing exits `1` with no stdout.
- §4.3: `week 2026-41 target 20h` message and exit code; `--help`
  usage and Arguments.
- Coverage must not drop (pre-commit ratchet).
