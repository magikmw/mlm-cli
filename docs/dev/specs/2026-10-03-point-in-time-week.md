# Point-in-time week line — design spec

**Baseline**: written against `main` at `efd97e4` (`v0.3.6` plus the
unreleased `open-punch-visibility` and `live-eod` changesets).
**Status**: draft.

## 1. Summary

The current week's headline in `status` and `mlm week` shows
`owed = target − fulfillment` and phrases it as due by end of today
(`12h 00m left by end of Thursday`). That figure is the whole week's
remainder, which is not due until the week ends. Meanwhile the pace
hint on the Day total line shows the figure that is actually due
today (`left to 32h 00m required by end of Thursday`). The screen
shows two deadlines for the same day, 8h apart.

After this changeset the current week has one point-in-time figure:

- The week line shows `required − fulfillment`, with `required` the
  prorated slice of the target due by end of today.
- Fulfillment, worked and the day total all count the running
  stint's minutes so far.
- The Day total line drops its pace clause and keeps `est. EOD`.
- `mlm week` for the current week shows the same headline and the
  same live-inclusive figures as `status`.

| Figure (current week) | Before | After |
|---|---|---|
| Week headline | `target − fulfillment`, "left by end of `<today>`" | `required − fulfillment`, "left to `<required>` by end of `<today>`" |
| Fulfillment, worked | closed stints only | closed stints + live |
| Day total (today) | closed stints, `(+ ongoing)` | closed stints + live, `(incl. ongoing)` |
| Pace clause on Day total line | present | removed |
| `est. EOD` | Day total line | Day total line, unchanged value |
| `mlm week` today row, Worked, Fulfillment | closed stints only | closed stints + live |

## 1.1 Non-goals

- No change to past or future weeks. Their headline stays
  `Total behind` / `Total ahead`, and they have no live minutes.
- No change to carry. Carry-out is only computed for closed weeks, and
  live minutes only exist in the current week, so no carried figure
  changes.
- No change to which open stint counts as live (§2.1). An open start
  dated yesterday still gets no live credit.
- No change to `est. EOD`'s value or its day-suffix rule.
- No change to stint lines (`17:25-now (00h 40m, ongoing)`), anomaly
  markers, the header, notes, or the `week` footer.
- No change to pairing (SPEC §4.3) or the splice (§4.3.1).

## 2. Definitions

### 2.1 Live minutes

Unchanged from SPEC §7.1. `live` is the whole minutes from the most
recent open start dated today to now, clamped at 0, and 0 when no
stint is open today. An open start from an earlier date never counts.
With two or more open starts today only the most recent counts. A
start typed for a time later than now has `live` 0. An open start two
or more calendar days old is stale and gets no live credit.

### 2.2 Required by end of today

- Mon–Thu: `required = floor(target / 5) × weekday`, with Mon=1 …
  Thu=4. Unchanged.
- Fri–Sun: `required = target`. This replaces `5 × floor(target / 5)`,
  which sat up to 4 minutes under a target that is not a multiple of
  5 minutes. With `required` now on the week line, Friday's figure
  must match the target the line names.

`required` uses today's weekday, even when `status DATE` shows another
date in the current week (SPEC §7.1 already does this for the week
line).

### 2.3 Live-inclusive figures (current week only)

- `worked = closed-stint minutes this week + live`
- `fulfillment = carry_in + worked`
- `gap = required − fulfillment`
- today's day total `= closed-stint minutes today + live`

`fulfillment = worked + carry-in` holds on screen. `gap` equals the old
post-live pace-hint gap, so the figure the user saw on the Day total
line moves to the week line unchanged.

Nothing live-inclusive is stored or carried (SPEC §2.4). `owed` as a
whole-week figure stays an internal quantity for carry. It is no
longer printed for the current week.

## 3. Output

### 3.1 `status` week line, current week

Format: `Week <id>:  <headline> (<parenthetical>)`.

Headline, by `gap` and today's weekday:

| | `gap >= 0` | `gap < 0` |
|---|---|---|
| Mon–Thu | `X left to <required> by end of <Weekday>` | `X over <required> by end of <Weekday>` |
| Fri–Sun | `X left to <target> target` | `X over <target> target` |

- `X` is `abs(gap)`. `gap == 0` reads `00h 00m left to …`.
- `<Weekday>` is today's full weekday name.
- On Fri–Sun `required` equals the target (§2.2), so the line names
  the target alone.

Parenthetical: unchanged shape. The expanded form
`fulfillment F = worked W + carry-in C / target T` prints when
carry-in is non-zero, the short form `fulfillment F / target T` when it
is zero. `F` and `W` are the live-inclusive figures (§2.3).

The week line prints for the current week whether or not `DATE` is
today. It always describes now.

### 3.2 `status` Day total line

- Today with a live stint: `Day total:     <total> (incl. ongoing)`,
  then `, est. EOD …` per SPEC §7.1. `<total>` includes `live`.
- Today with an open start later than now: same `(incl. ongoing)`
  flag, `live` is 0.
- A stale open start: `(+ unclosed)` stays, no live credit. Unchanged.
- No open stint: `Day total:     <total>`. Unchanged.
- Any other date: unchanged. No live credit, no `est. EOD`.
- The pace clause (`X left to/over <required> required …`) is removed
  everywhere.

`est. EOD` keeps its value: `open start + (required − fulfillment
without live)`, equal to `now + gap` for a start at or before now.
`target already met` still replaces it when `gap <= 0`.

### 3.3 `mlm week`, current week

- Headline: the same text as the `status` week-line headline (§3.1),
  on its own line.
- Today's row: live-inclusive total, marker `(incl. ongoing)` in place
  of `(ongoing)`.
- `Worked:` and `Fulfillment:` show the live-inclusive figures.
  `Carry-in:` and `Target:` are unchanged.
- `mlm week` shows no `est. EOD`.

Past and future weeks: unchanged.

### 3.4 Worked example

Thursday, now 18:05. Target 40h, carry-in +06h 55m. Mon–Wed closed
13h 40m. Today: 09:00–13:00, 13:30–16:55, 17:25–now.

- live = 40m, today closed = 07h 25m, day total = 08h 05m
- worked = 13h 40m + 07h 25m + 40m = 21h 45m
- fulfillment = 21h 45m + 06h 55m = 28h 40m
- required = 4 × 08h 00m = 32h 00m, gap = 03h 20m
- est. EOD = 17:25 + (32h 00m − 28h 00m) = 21:25

```
Thu 2026-10-01

Day total:     08h 05m (incl. ongoing), est. EOD 21:25
Week 2026-40:  03h 20m left to 32h 00m by end of Thursday (fulfillment 28h 40m = worked 21h 45m + carry-in 06h 55m / target 40h 00m)

  09:00-13:00  (04h 00m)
  13:30-16:55  (03h 25m)
  17:25-now    (00h 40m, ongoing)
```

```
Week 2026-40 (2026-09-28 - 2026-10-04)

03h 20m left to 32h 00m by end of Thursday

  Mon 2026-09-28   04h 30m
  Tue 2026-09-29   04h 40m
  Wed 2026-09-30   04h 30m
  Thu 2026-10-01   08h 05m (incl. ongoing)
  Fri 2026-10-02   00h 00m
  Sat 2026-10-03   00h 00m
  Sun 2026-10-04   00h 00m

Carry-in:      06h 55m
Worked:        21h 45m
Fulfillment:   28h 40m
Target:        40h 00m
```

Friday, fulfillment 41h 10m including live:

```
Week 2026-40:  01h 10m over 40h 00m target (fulfillment 41h 10m = worked 34h 15m + carry-in 06h 55m / target 40h 00m)
```

## 4. SPEC.md and README changes

- §2.4 Required-by-day: Fri–Sun rule (§2.2). Fulfillment counts live
  for the current week (§2.3). "status-only pace hint" becomes the
  week-line figure.
- §3.5, §3.6: the week figure is the point-in-time gap, not "time
  still owed".
- §5: rewrite the `owed` vs required-by-day paragraph. `owed` drives
  carry and closed-week totals. The current week shows `gap`.
- §7.1: new week-line and Day total rules and sample. Remove the pace
  clause bullet and the Fri–Sun `required today` wording.
- §7.2: new current-week sample, headline rule, `(incl. ongoing)`
  marker.
- §1.2 possible issues: remove entries this changeset resolves (§5).
  Add the new ones (§6).
- README: `status` and `week` samples and the prose describing the
  pace hint, `owed`, and `(+ ongoing)`.

## 5. Possible issues resolved

These SPEC §1.2 entries stop being true and are removed:

- Day total line mixes today's hours with a week-scoped pace clause.
- Pace hint counts live while the day total does not.
- Pace hint counts down while the week line's `owed` stays frozen,
  including the Fri–Sun direction and magnitude disagreement.
- `mlm week` stays frozen while `status` counts live.
- Pace hint's `required by end of <weekday>` vs `required today`
  wording asymmetry.

Entries about the pace hint that still apply (minute-by-minute change,
jumps on backdated punches, `est. EOD` meaning, two open starts, the
midnight jump, Friday treating the whole week as due) are reworded to
name the week line instead of the pace hint.

## 6. Possible issues left after this changeset

- `mlm week` output now changes minute by minute while a stint runs,
  like `status`.
- `status DATE` for an earlier date in the current week shows a week
  line that counts today's live minutes, beside a day total for `DATE`
  that has none. The week line describes now, not `DATE`.
- A week's displayed `Worked:` drops by the live minutes when the week
  closes with a stint still open over midnight Sunday, because the open
  stint stops being live.
- The current week no longer prints the whole-week remainder anywhere.
  It is `target − fulfillment`, computable from the parenthetical.
