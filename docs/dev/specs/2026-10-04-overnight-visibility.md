# Overnight stint visibility — design spec

> **Archived — historical changeset spec.** Superseded by `docs/dev/SPEC.md`, which folds in the decisions made here.

**Baseline**: written against `main` at `6edd67b` (`v0.3.6` plus the
unreleased `open-punch-visibility`, `live-eod`, `point-in-time-week`
and `possible-issues-triage` changesets).
**Status**: adversarially reviewed 2026-10-04
(`docs/dev/plans/reports/overnight-visibility-spec-review.md`, 9
findings + U1-U8, folded — NOTES.md entry 104). Locked.
**Input**: SPEC §1.2a's last item (a stint open across midnight).
Decisions: NOTES.md entries 103-104.

## 1. Summary

A start left open from yesterday counts nothing after midnight. Today
it shows only as the `[i]` hint on today's `status`. Week view shows
nothing on yesterday's row.

After this changeset the stint is obvious in both places. No figure
changes.

| Where | Before | After |
|---|---|---|
| Today's `status` stint list | not listed | `22:00-now (15h 30m so far, started yesterday, not counted until stopped)` |
| `mlm week` row for yesterday | plain | `[i]` after the duration |
| `mlm week` footer | `[!]` line only | adds an `[i]` line; on Mondays the current week names Sunday's stint |

## 2. Non-goals

- No live credit. Day total, `Worked`, `Fulfillment`, the week line,
  `est. EOD` and carry are unchanged.
- No change to the `[i]` hint's text or conditions, the splice, or
  pairing.
- No change for an open start dated today (it already shows with
  `(incl. ongoing)`), or for one 2+ days old (it is a `[!]` stale
  anomaly).
- `[i]` is still not an anomaly. It never feeds `[!]`, the `[!]`
  footer, the carry-in marker, or `has_any`.

## 3. Trigger

Both changes apply exactly when today's `status` prints the `[i]`
hint: yesterday has exactly one open start that the splice does not
join to today, and today's page shows no refused-join line naming
yesterday (SPEC §7.1). Call it the overnight stint.

## 4. Today's `status`

Today's stint list gains one line for the overnight stint, before
today's own stints (it started earlier):

```
  22:00-now    (15h 30m so far, started yesterday, not counted until stopped)
```

- The range uses the same column layout as other stint lines.
- `15h 30m` is whole minutes from the start to now, in the standard
  formatter (§4.2).
- The line is rendered on its own, ahead of the date-sorted stint
  list, so it is not part of today's stint pairing or sorting.
- The line prints even when today has punches of its own. It is the
  only stint line on today's page that is not part of today's day
  total.
- `status DATE` for any other date is unchanged. Yesterday's own page
  keeps its existing open-stint line.

## 5. `mlm week`

- The overnight stint's date row (yesterday) ends with `  [i]`, in
  the same column as `  [!]`.
- A row that has both an anomaly and the overnight stint shows both
  markers, `  [!] [i]`, and both footers print.
- When any row in the table shows `[i]`, one footer line prints:

  ```
  [i] open stint: run mlm status to stop it
  ```

- Footer order: the `[!]` footer, then the `[i]` footer, then the
  carry-in marker line, then the blank line before the Carry-in block
  (contract 6 of `possible-issues-triage` keeps its place relative to
  the `[!]` footer).
- The row marker shows on whichever week contains yesterday. On a
  Monday that is last week's Sunday row.
- On a Monday, the current week's view (the week containing today)
  names the stint in the `[i]` footer slot, since no row of its own is
  marked:

  ```
  [i] open stint on Sun 2026-10-04 (last week): run mlm status to stop it
  ```

  This line prints only in the view of the week containing today, and
  only when the overnight stint's date is in the previous week. It
  takes the `[i]` footer's place in the footer order.

## 6. Glossary and docs

- The `[i]` glossary entry in `status --help` and `week --help` stops
  saying it never marks a row. It says `[i]` marks yesterday's row in
  `mlm week` while a start from yesterday is still open.
- SPEC §1.2a: the last item is removed. Its behavior moves into §7.1
  (status) and §7.2 (week) as plain statements: the stint gets no live
  credit and no `est. EOD`; its minutes return to yesterday on `stop`;
  the week line jumps at 00:00 because `required` steps up and the
  stint is not counted; past Sunday midnight the closed week's
  `Worked` leaves the stint out until `stop`; the overnight stint is
  listed and marked as above. §1.2a keeps its heading and reads "None
  currently."
- README: the Possible issues section keeps its heading and reads
  "None currently." Output examples follow §4 and §5.
- Every other SPEC passage this changes is updated, including the ones
  that say the `[i]` hint never marks a row or that `mlm week` has no
  yesterday hint (SPEC.md around lines 311, 869-875, 1185, 1450 at
  baseline; relocate by content). The existing test that asserts the
  hint does not mark yesterday's week row (`src/lookback.rs` near line
  959) is rewritten to the new rule.

## 7. Visible consequences

- Today's page can list a stint whose time is not in the Day total
  right above it. The tag says so.
- A legitimate overnight session shows `[i]` on yesterday's row and the
  footer until `stop`.
- On a Monday the current week's view has no `[i]` row, only the
  footer line naming Sunday; last week's view marks the Sunday row.
- `mlm week` output for any week containing yesterday changes while
  such a start is open.
- Today's page names the same stint twice: once in the `[i]` hint
  (with fix lines) and once in the stint list. Yesterday's own page
  keeps its older wording (`ongoing - duration as of right now`,
  `(+ ongoing)`).
- A today page with no punches now shows a stint line under `Day
  total: 00h 00m`, with no `est. EOD`.
- When the start turns 2 days old, the stint line and `[i]` marks
  disappear and the `[!]` stale lookback line and row marker take over,
  though the stint is still open.
- On a Monday the current week's view has a footer line and no marked
  row.
- Scripts that sum today's stint lines now see a line whose minutes
  are not in the Day total.
- Today's stint list is no longer in clock order when the overnight
  line prints (it comes first).
- With today's own open stint as well, two `-now` lines print with
  different tags.
- The Day total shows no ongoing marker while the overnight line
  prints, because the stint is not counted.
- A past week viewed by `WEEK_ID` that contains yesterday changes
  with today's data (the mark goes once the start is joined).
- The `[!]` footer points to `mlm status DATE`, the `[i]` footer to
  `mlm status`; a row with both markers prints both footers.
- The Monday footer line prints only in the current week's view, not
  in a past week viewed by `WEEK_ID`.

## 8. Testing

- §4: hint case with no punches today and with punches today; joined
  (no line); refused join (no line); two open starts yesterday (no
  line, the E7 lookback covers it); duration formatting.
- §5: row marker, footer text, the `  [!] [i]` shared row,
  footer order with both footers and the carry marker; Monday case (current week footer line names Sunday, last week's Sunday row marked, no line on other weeks).
- `has_any`, the `[!]` footer and the carry marker ignore `[i]`.
- Glossary golden updated.
- Also: a past week viewed by `WEEK_ID` that contains yesterday (row
  marked); any other past or future week (nothing); today with its own
  open stint as well (both lines, correct order); a stint running over
  24h (formatter); the age-2 transition (line gone, `[!]` lookback
  instead); an end-to-end `status` test with the hint and the line
  together.
