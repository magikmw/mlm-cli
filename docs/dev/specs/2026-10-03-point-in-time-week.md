# Point-in-time week line — design spec

> **Archived — historical changeset spec.** Superseded by `docs/dev/SPEC.md`, which folds in the decisions made here.

**Baseline**: written against `main` at `efd97e4` (`v0.3.6` plus the
unreleased `open-punch-visibility` and `live-eod` changesets).
**Status**: adversarially reviewed 2026-10-03
(`docs/dev/plans/reports/point-in-time-week-spec-review.md`,
needs-rework, 9 findings + 10 undeclared gaps, folded — NOTES.md
entry 91). Locked. Plan review added four §6 entries (entry 92).

## 1. Summary

The current week's headline in `status` and `mlm week` shows
`owed = target − fulfillment` and phrases it as due by end of today
(`12h 00m left by end of Thursday`). That figure is the whole week's
remainder, which is not due until the week ends. Meanwhile the pace
hint on the Day total line shows the figure that is actually due
today (`left to 32h 00m required by end of Thursday`). The screen
names two different amounts as due by end of the same day.

After this changeset the current week has one point-in-time figure:

- The week line shows `required − fulfillment`, with `required` the
  prorated slice of the target due by end of today.
- Fulfillment, worked and today's day total all count the running
  stint's minutes so far.
- The Day total line drops its pace clause and keeps `est. EOD`.
- `mlm week` for the current week shows the same headline and the
  same live-inclusive figures as `status`.

| Figure (current week) | Before | After |
|---|---|---|
| Week headline | `target − fulfillment`, "left by end of `<today>`" / "ahead of target" | `required − fulfillment`, "left to / over `<required>` by end of `<today>`" (§3.1) |
| Fulfillment, worked | closed stints only | closed stints + live |
| Day total (today) | closed stints, `(+ ongoing)` | closed stints + live, `(incl. ongoing)` |
| Pace clause on Day total line | present | removed |
| `est. EOD` | Day total line | Day total line; value changes only per §2.2 |
| `mlm week` today row, Worked, Fulfillment | closed stints only | closed stints + live |

## 1.1 Non-goals

- No change to past weeks. Their headline stays `Total behind` /
  `Total ahead`, and they have no live minutes.
- No change to stored data. Nothing live-inclusive is written to the
  database.
- No change to which open stint counts as live (§2.1). An open start
  dated yesterday still gets no live credit.
- No change to `est. EOD`'s formula or its day-suffix rule.
- No change to stint lines (`17:25-now (00h 40m, ongoing)`), anomaly
  markers, the header, notes, or the `week` footer.
- No change to pairing (SPEC §4.3) or the splice (§4.3.1).
- No change to how storage read errors are handled in `mlm week`
  (`DbWeekData` treats a failed read as no data). Pre-existing.

## 2. Definitions

### 2.1 Live minutes

Unchanged from SPEC §7.1. `live` is the whole minutes from the most
recent open start dated today to now, clamped at 0, and 0 when no
stint is open today. An open start from an earlier date never counts.
With two or more open starts today only the most recent counts. A
start typed for a time later than now has `live` 0. An open start two
or more calendar days old is stale and gets no live credit.

`live` always comes from today's punches, whatever date `status`
shows. `status DATE` for an earlier date in the current week must
classify today separately from `DATE` to get it (the code currently
computes `live` only when `DATE` is today, `src/status.rs` `resolve`).

### 2.2 Required by end of today

- Mon–Thu: `required = floor(target / 5) × weekday`, with Mon=1 …
  Thu=4. Unchanged.
- Fri–Sun: `required = target`. This replaces `5 × floor(target / 5)`,
  which sat up to 4 minutes under a target that is not a multiple of
  5 minutes. With `required` now on the week line, Friday's figure
  must match the target the line names. For such targets this moves
  Fri–Sun `est. EOD` and the `target already met` threshold up to 4
  minutes later. The default 40h target is unaffected.

`required` uses today's weekday, even when `status DATE` shows another
date in the current week. It is computed for every `status` of the
current week, not only when `DATE` is today.

### 2.3 Live-inclusive figures (current week only)

- `worked = closed-stint minutes this week + live`
- `fulfillment = carry_in + worked`
- `gap = required − fulfillment`
- today's day total `= closed-stint minutes today + live`

`fulfillment = worked + carry-in` holds on screen. Except for the
Fri–Sun change in §2.2, `gap` equals the old post-live pace-hint gap,
so the figure the user saw on the Day total line moves to the week
line unchanged.

Live enters the week accounting as part of the current week's worked
minutes, before carry is walked. A future week shown by `status DATE`
or `mlm week WEEK_ID` therefore takes its `Carry-in` from the current
week's live-inclusive `fulfillment − target`, matching what the current
week's page shows. Past weeks never contain live minutes, so their
carry is unchanged. Nothing live-inclusive is stored (SPEC §2.4).

`owed = target − fulfillment` stays the quantity behind carry and the
closed-week `Total behind` / `Total ahead`. It is no longer printed for
the current week.

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
- A zero-target week follows the same rule, e.g.
  `00h 00m left to 00h 00m by end of Monday` with nothing worked, or
  `02h 00m over 00h 00m by end of Monday` after two hours.

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
- An earlier date whose open stint is not stale (e.g. yesterday's
  page with a session still open): `(+ ongoing)` stays, no live credit.
  Unchanged.
- No open stint: `Day total:     <total>`. Unchanged.
- The pace clause (`X left to/over <required> required …`) is removed
  everywhere, including the Fri–Sun `required today` form.

`est. EOD` keeps its formula: `open start + (required − fulfillment
without live)`, equal to `now + gap` for a start at or before now.
`target already met` still replaces it when `gap <= 0`.

### 3.3 `mlm week`, current week

- Headline: the same text as the `status` week-line headline (§3.1),
  on its own line.
- Today's row: live-inclusive total, marker `(incl. ongoing)` in place
  of `(ongoing)`.
- `Worked:` and `Fulfillment:` show the live-inclusive figures. The
  rows still sum to `Worked:`.
- `Carry-in:` and `Target:` are unchanged.
- `mlm week` shows no `est. EOD`.

Past weeks: unchanged. Future weeks: unchanged except `Carry-in`
(§2.3).

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

## 4. Doc and comment changes

SPEC.md:

- §1.2a (possible issues): per §5 and §6 below. Remove only the
  sentences §5 lists, since several share a bullet with entries that
  stay.
- §1.2a zero-boundary entry: the current week's zero case now reads
  `00h 00m left to <required> …`; the closed week's still reads
  `Total ahead: 00h 00m`. Reword; drop the "ahead of target" contrast.
- §2.4 open-stint and required-by-day bullets: Fri–Sun rule (§2.2);
  fulfillment and worked count live for the current week (§2.3); the
  figure is the week-line figure, not a `status`-only pace hint.
- §3.5, §3.6: the week figure is the point-in-time gap, not "time
  still owed"; the pace hint is no longer a separate item.
- §5: rewrite the `owed` vs required-by-day paragraph and the closing
  note about `X ahead of target`. `owed` drives carry and closed-week
  totals. The current week shows `gap`.
- §7.1: new week-line and Day total rules and sample. Remove the pace
  clause bullet, the Fri–Sun `required today` wording, the `ahead of
  target` form and the notes contrasting the day line with the week
  line. Replace the rationale for keeping live out of the day total
  ("avoids the total silently changing mid-read") with the
  point-in-time rule.
- §7.2: new current-week sample, headline rule, `(incl. ongoing)`
  marker.

README.md: `status` and `week` samples (including the Saturday
sample), the prose describing the pace hint, `owed`, `ahead of target`,
and `(+ ongoing)`, and its possible-issues list per §5 and §6.

NOTES.md (coordinator): a decision entry superseding entry 37's "live
minutes never enter that math" for displayed fulfillment and worked.

Code doc comments that state the old rule: `src/week.rs` `WeekData`
(worked excludes live), `src/stint.rs` `completed_minutes` ("the day
total"), `src/week_view.rs` row `minutes` ("completed-stint minutes
only").

## 5. Possible issues resolved

These SPEC §1.2a sentences stop being true and are removed:

- The Day total line names today's hours, then adds a week-scoped pace
  clause.
- The pace hint counts live while the day total beside it does not.
- The pace hint counts down while the week line's `owed` stays frozen,
  including the Fri–Sun direction and magnitude disagreement.
- `mlm week` for the current week stays frozen while `status` counts
  live.
- The `required by end of <weekday>` vs `required today` wording
  asymmetry.

Entries about the pace hint that still apply (minute-by-minute change,
jumps on backdated punches, `est. EOD` meaning, two open starts, the
midnight jump, Friday treating the whole week as due) are reworded to
name the week line instead of the pace hint.

## 6. Possible issues left after this changeset

- Mon–Thu, the week line reads `X over 32h 00m by end of Thursday`
  once today's slice is met, while the week's target may still be far
  off. The `ahead of target` wording is gone for the current week.
- The current week no longer prints the whole-week remainder anywhere.
  It is `target − fulfillment`, computable from the parenthetical.
  Mon–Thu, `mlm week`'s `Target: 40h 00m` and its headline's
  `left to 32h 00m` name different amounts.
- `status`'s Day total and `mlm week`'s output now change minute by
  minute while a stint runs.
- `status DATE` for an earlier date in the current week shows a week
  line that counts today's live minutes, beside a day total for `DATE`
  that has none. The week line describes now, not `DATE`.
- `(incl. ongoing)` on today's page means the running stint is
  counted. `(+ ongoing)` on an earlier date's page means it is not.
- A start typed for a time later than now shows `(incl. ongoing)`
  with nothing added.
- `est. EOD` sits on the Day total line, a line above the week line it
  is computed from. Mon–Thu, `target already met` refers to today's
  slice, not the target the week line's parenthetical names. At
  `gap == 0` the week line reads `00h 00m left to …` while `est. EOD`
  reads `target already met`.
- A stint still open after midnight is not live on the new day, so its
  minutes leave the day total, `Worked`, `Fulfillment` and the week
  line until `stop`. When this happens past Sunday midnight, the closed
  week's `Worked` drops by those minutes. This generalizes the existing
  midnight-jump entry.
- A future week's `Carry-in` changes minute by minute while a stint
  runs in the current week (§2.3).
- A zero-target week reads `… left to 00h 00m by end of <Weekday>` or
  `… over 00h 00m …`.
- With a target override that is not a multiple of 5 minutes, Fri–Sun
  `est. EOD` lands up to 4 minutes later than before (§2.2).
- The headline changes form at Thursday-to-Friday midnight, from
  `left to 32h 00m by end of Thursday` to `left to 40h 00m target`.
- After `stop`, the Day total line shows no due-today figure, because
  the pace clause is gone and `est. EOD` shows only while a stint is
  open. The week line carries that figure instead.
- A storage read failure in `mlm week` already shows as no data. It
  now also hides live minutes from `Worked`, `Fulfillment` and the
  headline.
- For an earlier date in the current week, `status DATE`'s day total
  stays fixed while its week line moves. `mlm week` moves only today's
  row.

## 7. Testing

Existing pins of the old strings in `src/status.rs`, `src/week_view.rs`
and `src/render.rs` are updated to the new output. New coverage:

- `status` for today: week line Mon–Thu and Fri–Sun, `left to` and
  `over`, `gap` exactly 0 and ±1 minute.
- `status DATE` for an earlier date in the current week with a live
  stint today: week line counts live, Day total for `DATE` does not.
- Fri–Sun `required = target` with a target not a multiple of 5.
- Zero-target week.
- Day total `(incl. ongoing)` with live, with a future-dated start
  (live 0), and `(+ ongoing)` / `(+ unclosed)` unchanged on earlier
  dates.
- `mlm week` current week: headline matches `status`, today's row
  includes live, rows sum to `Worked:`.
- Future week `Carry-in` includes the current week's live minutes.
- Past week output unchanged.
- §3.4 worked example pinned at the resolve/render level with a fixed
  clock.
