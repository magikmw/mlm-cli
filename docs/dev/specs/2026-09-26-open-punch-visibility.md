# Open-punch visibility and anomaly remedies — design spec

**Baseline**: written against `v0.3.6` (`main` at `267060b`).
**Status**: adversarially reviewed 2026-09-26
(`docs/dev/plans/reports/open-punch-visibility-spec-review.md`,
9 findings, folded — NOTES.md entry 77). Locked.

## 1. Summary

Four SPEC.md §1.2a known issues share one root. `mlm` detects pairing
problems but says little about them, and says nothing at all about the
commonest one, a forgotten `stop`.

| §1.2a issue | Fix in this spec |
|---|---|
| A forgotten `stop` from days ago shows only on its own date's `status` | §3.1 stale open start becomes an anomaly; §3.3 today's `status` lists stale starts from the last 14 days; §3.6 `mlm start` warns |
| A lone open `start` is silent while two are flagged, unexplained | §3.1 a lone *stale* start is flagged; §3.4 E7 text says why two is wrong |
| `[!]` lines name a problem but no remedy | §3.4 every `status` anomaly line gets `fix:` lines; §3.5 `week` gets a footer |
| The cross-midnight join gate (§4.3.1) is unobservable | §3.2 a refused join becomes an anomaly with its reason |

## 1.1 Non-goals

- No change to pairing (§4.3) or to the splice rule itself (§4.3.1).
  Only what gets reported changes.
- No automatic repair. Fixes stay user-run commands.
- No change to open-stint line rendering (`ongoing`, the yesterday
  caption, `(unclosed)`, §7.1).
- No lookback on `status DATE` for any date other than today, and none
  on `week`.
- No warning from `stop`, `note`, `delete` or `week target`.
- No change to exit codes (§6.3). Anomalies still exit `0`.

## 1.2 Known issues left after this changeset

- An unclosed start more than 14 days old shows only on its own
  date's `status` and its own `week` row. Today's `status` and
  `mlm start` stop mentioning it once it leaves the window. Replaces
  the current §1.2a forgotten-`stop` bullet.
- The lookback (§3.3, §3.6) counts open starts only. An orphaned end
  older than yesterday stays visible only on its own date and its
  `week` row. An orphaned end adds no time, so it cannot inflate a
  total the way a forgotten `stop` hides one.
- Every plain `status` for today now reads 15 extra dates (one range
  query). A future cheap prompt query (AGENTS.md stretch) would skip
  the lookback.
- A lone open start dated yesterday is never flagged, because an
  overnight session may still be running. Intended; recorded in SPEC
  §4.3, not filed in §1.2a.

## 2. Terms

- **today**: the local calendar date of `now` (§2.1). Always the real
  today, never a queried `DATE`.
- **age** of date `D`: `today − D` in whole calendar days.
- **open start**: a trailing unmatched `start` left on a date after
  `classify_at` (§4.3.1), so splice resolution has already happened.
- **stale open start**: the only open start on a date of age `>= 2`.
  The threshold matches the existing `(unclosed)` threshold (§7.1).
- **lookback window**: dates of age 2 through 14 inclusive.
- **refused join**: adjacent dates `A`, `A+1` where, after
  `classify_at`, `A` has at least one open start and `A+1` has at least
  one orphaned end. By construction the §4.3.1 gate rejected them, or
  they would have spliced.

## 3. Behavior

### 3.1 Stale open start is an anomaly

A date with exactly one open start and age `>= 2` now has an anomaly.
`has_anomaly()` and `anomalies()` both reflect it, so `status` for that
date prints a `[!]` line (§3.4) and its `week` row gets `[!]` (§3.5).

A date with two or more open starts keeps the existing E7 anomaly
only. It gets no extra stale line, since E7 already covers every open
start on that date.

Yesterday's lone open start (age 1) and today's (age 0) stay
non-anomalies.

### 3.2 Refused join is an anomaly on both dates

For a refused join between `A` and `A+1`, both dates get an anomaly.
`A`'s names `A+1` and `A+1`'s names `A`. A date can carry two, one
per neighbor. This applies at every age, including yesterday–today.

Existing E7/E8/stale anomalies on either date still print alongside
it. The refused-join line explains why nothing joined, and the other
lines carry the fixes.

### 3.3 Today's `status` lists stale starts in the lookback window

When `status` shows today (`DATE` omitted, or today's date given
explicitly), it
also classifies every date in the lookback window. Each window date
with one or more open starts adds one line to today's anomaly block,
oldest date first. Each line is a normal `[!]` line with `fix:` lines
(§3.4).

`status` for any other date does no lookback.

### 3.4 `status` anomaly lines

Order in the anomaly block (position unchanged, §7.1/§7.3):

1. E7 multiple open stints
2. Stale open start
3. E8 orphaned ends, ascending by instant
4. Refused join with the previous date, then with the next date
5. Lookback lines (today only), oldest date first

`D` below is the date the anomaly belongs to, as `YYYY-MM-DD`. `Wdy D`
is the `<weekday abbrev> <YYYY-MM-DD>` header form. `HH:MM` inside a
`fix:` command is a literal placeholder the user replaces with the
real time. Every other value is filled in. Each `fix:`/`or:` line is
indented 4 spaces so its text aligns under the text after `[!] `.

Two properties of the fix commands shape their wording:

- `mlm delete punch --date D` only lists and numbers that date's
  punches (§7.4). Deleting takes a second run with the number. The
  hint says so.
- `mlm stop HH:MM --date D` pairs by time (§4.3). An end before the
  open start becomes a new orphaned end, and with several open starts
  the end closes the latest one before it. So the stop hint names the
  start time the new end must follow, and E7 leads with delete.

In the E7 hint, `14:00` is the latest open start on the date.

E7 (reworded):

```
[!] 2 open stints for this date (only one can be open at a time)
    fix: mlm delete punch --date D  (lists punches; rerun with the extra start's number)
     or: mlm stop HH:MM --date D  (a time after 14:00 closes the 14:00 start)
```

Stale open start (new):

```
[!] unclosed start at 09:00, 5 days old (no stop recorded)
    fix: mlm stop HH:MM --date D  (a time after 09:00)
     or: mlm delete punch --date D  (lists punches; rerun with the start's number)
```

E8 (text unchanged, fix lines added):

```
[!] orphaned end at 18:00 (no matching start)
    fix: mlm start HH:MM --date D  (a time before 18:00)
     or: mlm delete punch --date D  (lists punches; rerun with the end's number)
```

Refused join (new). No `fix:` lines of its own, because the E7/E8/stale
lines on the two dates already carry them. The reason text is the same
on both dates. It states the gate's full condition even when an E7 or
extra E8 line on the same page already shows which part failed.

```
[!] not joined to Tue 2026-09-22 across midnight (a join needs exactly one open start on Mon 2026-09-21 and exactly one unmatched end, as Tue 2026-09-22's first punch)
```

`A+1`'s line reads `not joined to Mon 2026-09-21 across midnight`
followed by the same parenthetical.

Lookback, one open start on the window date:

```
[!] unclosed start on Mon 2026-09-21 at 09:00 (5 days ago)
    fix: mlm stop HH:MM --date 2026-09-21  (a time after 09:00)
     or: mlm delete punch --date 2026-09-21  (lists punches; rerun with the start's number)
```

Lookback, two or more open starts on the window date:

```
[!] 2 open stints on Mon 2026-09-21 (5 days ago)
    fix: see mlm status 2026-09-21
```

### 3.5 `week`

- A row gets `[!]` whenever its date has any anomaly under the new
  definition, stale starts and refused joins included. This replaces
  §7.2's rule that a stale open-start date renders with no marker.
- When at least one row carries `[!]`, one footer line follows the
  table after a blank line:

```
[!] needs a look: run mlm status DATE for details and fixes
```

- No footer when no row is marked. No lookback lines in `week`.

### 3.6 `mlm start` warns about stale starts

After a successful `start` write (with or without `--date`), `start`
runs the §3.3 lookback relative to today. For each window date with
open starts it prints one line to **stderr**, oldest first. The date
the command just wrote to counts only when it now has two or more
open starts (so the new punch made an E7); a single open start there
is the punch just written and gets no line.

```
warning: unclosed start on Mon 2026-09-21 at 09:00 (5 days ago); fix: mlm stop HH:MM --date 2026-09-21 (a time after 09:00)
warning: 2 open stints on Mon 2026-09-21 (5 days ago); see mlm status 2026-09-21
```

- stdout stays silent and exit stays `0`. §7.4's silence rule applies
  to stdout. SPEC §7.4 gains this as a named exception.
- The check is best-effort. If the lookback read fails after the write
  committed, `start` prints nothing extra and still exits `0`.
- The written-date rule keeps a backdated `start --date D` from
  warning about its own punch, but still reports an E7 it caused.
- `start` for today writes to a date outside the window, so the rule
  only matters for a backdated `start`.

### 3.7 Output stays plain ASCII

All new strings follow §7's plain-ASCII rule. They use no em dashes or
unicode arrows.

## 4. Docs

- SPEC.md
  - §1.2a: drop the four bullets; add §1.2's 14-day bullet.
  - §4.3: stale open start joins the edge-case list; note that a lone
    open start at age 0 or 1 is intentionally silent.
  - §4.3.1: replace "the gate's own on/off condition is still
    undocumented" with the refused-join anomaly.
  - §7.1: anomaly bullet, lookback lines, and the `(unclosed)` bullet's
    §1.2a pointer.
  - §7.2: stale-date and refused-join `[!]`, and the footer. Rewrite
    the "accepted, silent consequence" passage.
  - §7.3: new line shapes and `fix:` lines.
  - §7.4: the `start` stderr warning exception.
  - §8.2: E7/E8 expected output gains the fix lines. New E-cases for
    the stale start, refused join, lookback and `start` warning follow
    E15 (E16 onward).
- README.md
  - Known issues: drop the four bullets; add the 14-day bullet.
  - `status` section: `[!]` lines, `fix:` lines, today's lookback.
  - `week` section: the footer.
  - `start` section: the stderr warning.
  - A short paragraph on when overnight stints join and when they do
    not, in plain terms.
- `--help`: no change needed. It names no anomaly behavior today.

## 5. Test plan

- Stale boundary: lone open start at age 0 and 1 is not an anomaly;
  at age 2 it is; at age 14 it is listed in today's lookback; at age 15
  it is not.
- Two open starts at age 5 give E7 only, no stale line.
- A clean splice gives no refused join. A refused join (two opens on
  `A`, or the orphan not first on `A+1`, or two orphans on `A+1`) flags
  both dates with the right neighbor named.
- A refused join between yesterday and today shows on today's `status`.
- Lookback: one line per window date, oldest first; the multi-open
  form; nothing on `status DATE` for a non-today date.
- `fix:`/`or:` lines carry the real date for every anomaly kind.
- `week`: stale-date row and refused-join rows marked; footer present
  exactly when some row is marked.
- `start`: warning on stderr, stdout empty, exit `0`; no warning for
  the date just written; no warning when the window is clean.
- All new output is plain ASCII.
- Fix commands resolve what they claim: running the stale-start hint
  (a stop after the start time), then `status` for that date, shows no
  anomaly; the delete hint lists the offending punch.
- `status` with an explicit today's date does the lookback.
- Backdated `start --date D` into a stale date with an existing open
  start warns with the multi-open form; into an empty stale date it
  does not warn.
- An orphaned end inside the window produces no lookback line.
