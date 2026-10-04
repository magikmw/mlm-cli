# Overnight stint visibility — design spec

**Baseline**: written against `main` at `6edd67b` (`v0.3.6` plus the
unreleased `open-punch-visibility`, `live-eod`, `point-in-time-week`
and `possible-issues-triage` changesets).
**Status**: draft.
**Input**: SPEC §1.2a's last item (a stint open across midnight).
User chose to make it visible without counting it (option B), and to
mark `mlm week` rows (NOTES.md entry 103).

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
- The line prints even when today has punches of its own. It is the
  only stint line on today's page that is not part of today's day
  total.
- `status DATE` for any other date is unchanged. Yesterday's own page
  keeps its existing open-stint line.

## 5. `mlm week`

- The overnight stint's date row (yesterday) ends with `  [i]`, in
  the same column as `  [!]`.
- A row that has both an anomaly and the overnight stint shows `[!]`
  only.
- When any row in the table shows `[i]`, one footer line prints:

  ```
  [i] open stint: run mlm status DATE to stop it
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
  [i] open stint on Sun 2026-10-04 (last week): run mlm status 2026-10-04 to stop it
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
  credit, its minutes return to yesterday on `stop`, the week line
  jumps at 00:00 because `required` steps up and the stint is not
  counted, and the overnight stint is listed and marked as above.
  §1.2a is left with its heading and an empty-list sentence.
- README: the Possible issues section is removed or reads "none
  currently". Output examples follow §4 and §5.

## 7. Visible consequences

- Today's page can list a stint whose time is not in the Day total
  right above it. The tag says so.
- A legitimate overnight session shows `[i]` on yesterday's row and the
  footer until `stop`.
- On a Monday the current week's view has no `[i]` row, only the
  footer line naming Sunday; last week's view marks the Sunday row.
- `mlm week` output for any week containing yesterday changes while
  such a start is open.

## 8. Testing

- §4: hint case with no punches today and with punches today; joined
  (no line); refused join (no line); two open starts yesterday (no
  line, the E7 lookback covers it); duration formatting.
- §5: row marker, footer text, `[!]` precedence on a shared row,
  footer order with both footers and the carry marker; Monday case (current week footer line names Sunday, last week's Sunday row marked, no line on other weeks).
- `has_any`, the `[!]` footer and the carry marker ignore `[i]`.
- Glossary golden updated.
