# Boundary & context disclosure cues — design spec

**Baseline**: `v0.3.3` (commit `f04e77c`).

**Status**: locked — adversarially reviewed:
`docs/dev/plans/reports/boundary-context-cues-review.md` (needs-rework,
6 findings, all folded in). Amended once since: phase-6 plan review
(`docs/dev/plans/reports/boundary-context-cues-plan-review.md`)
surfaced an undeclared gap between §4's stint-line wording and Day
total's own `(+ ongoing)` suffix — §4 extended to cover Day total too
(NOTES.md decision 62). Implemented and merged
(`docs/dev/plans/reports/boundary-context-cues-final-review.md`,
ship-with-followups). §4's caption wording amended post-final-review
— "elapsed since now" read backwards to a fresh-eyes reader (elapsed
time is measured *since the start*, as of now); replaced with
"duration as of right now" (NOTES.md decision 64).

## 1. Scope

Fixes SPEC.md §1.2a bullets 1, 2, 6, 7. Bullet 4 (`[!]` anomaly flags
carry no remedy pointer) and bullets 3/5 (the splice-vs-flag gate and
the lone-unclosed-`start` distinction are undocumented) are out of
scope — held for later (§8).

| §1.2a bullet | Fix |
|---|---|
| 1 — cross-midnight stint has no span cue | §2 |
| 2 — receiving date shows no trace of the stop punch | §3 |
| 6 — open-stint duration has no backdated caption | §4 |
| 7 — negative fulfillment has no carry-in context | §5 |

No change to `classify`/`classify_at`/week accounting math. All four
are render-only changes.

## 2. Cross-midnight stint — span suffix

A completed stint whose end falls on a later calendar date than its
start gets a clause added to its duration parenthetical — same slot
`, ongoing` already occupies, so the `HH:MM-HH:MM` time field stays
fixed-width:

```
  23:30-00:45  (01h 15m, spans to next day)
```

Non-spanning stints and open stints are unaffected. Applies to
`status`'s stint list only.

## 3. Receiving date — header suffix

When a date's chronologically-first punch was consumed by a §4.3.1
splice onto the previous date, `status`'s date header for that date
gets a suffix — no new line, no bracket marker:

```
Fri 2026-02-13  (00:45 continues previous day's stint)
```

At most one such suffix per date (§4.3.1 splicing is 1:1). Omitted
when no splice consumed that date's first punch.

`status`-only: `week`'s row for the same date gets no equivalent
marker and keeps showing a plain total, so the same fact is now
visible from `status` and invisible from `week` for this one date. An
accepted asymmetry for this changeset (§8), not a `week`-side gap to
fix here.

## 4. Open stint duration — backdated caption

Today's open stints are unchanged:

```
  17:45-now    (00h 15m, ongoing)
```

An open stint dated yesterday keeps its live duration, with a caption
(still a plausible number — an overnight stint):

```
  23:10-now    (10h 35m, ongoing - duration as of right now, not a running total)
```

An open stint dated two or more days back drops the duration figure
entirely — at that age the number is noise, not information — and
prints instead:

```
  09:00-now    (unclosed)
```

The **Day total** line's own `(+ ongoing)` suffix (§7.1) tracks this
same distinction, so it never contradicts the stint line directly
below it. Today's and yesterday's stints still say "ongoing" is a
live, in-progress fact, so Day total is unchanged for those two cases.
A two-or-more-days-back stint has already had its "ongoing" wording
withdrawn on its own line (replaced by `(unclosed)`), so Day total
matches:

```
Day total:     00h 00m (+ unclosed)
```

instead of today's `(+ ongoing)`, whenever that date's open stint is
two or more days old. No other change to Day total's line — the
total itself, still computed excluding the open stint's minutes
(§7.1), is unaffected.

## 5. Fulfillment — carry-in inline

When `carry_in` is non-zero, `status`'s week line expands:

```
Week 2026-07:  10h 45m left by end of Thursday (fulfillment 29h 15m = worked 31h 25m + carry-in -02h 10m / target 40h 00m)
```

Each figure is signed independently per §4.2 — a carry-in deficit
larger than `worked` drives `fulfillment` itself negative, printed the
same way:

```
Week 2026-09:  42h 00m left by end of Monday (fulfillment -02h 00m = worked 03h 00m + carry-in -05h 00m / target 40h 00m)
```

A carry-in surplus (positive) expands the same way, no `+` sign
printed — matching §4.2's convention that only negative values carry
a sign:

```
Week 2026-10:  00h 00m left by end of Wednesday (fulfillment 40h 00m = worked 39h 10m + carry-in 00h 50m / target 40h 00m)
```

When `carry_in` is zero, unchanged:

```
Week 2026-07:  10h 45m left by end of Thursday (fulfillment 29h 15m / target 40h 00m)
```

This applies only to the current-week framing (`status`'s week line
always uses it — §7.1). A past/future week's plain `Total still
owed`/`Total ahead` line has no fulfillment parenthetical today
(SPEC.md §7.1) and gains none here — bullet 7's confusion is specific
to the current-week line, the only one that shows a fulfillment figure
at all.

## 6. Testing plan

- §2: `, spans to next day` present on spanning stints, absent on
  same-date and open stints.
- §3: header suffix present exactly when a splice consumed the date's
  first punch, absent otherwise, never more than one suffix; including
  the case where the consumed punch was the receiving date's only
  punch (stint-list section omitted per SPEC.md §7.1 — header suffix,
  day total, and week line are all that renders for that date).
- §4: today's open stint unchanged; yesterday's shows duration +
  caption; two-or-more-days-back shows `(unclosed)` with no duration
  figure at all. Day total's `(+ ongoing)`/`(+ unclosed)` suffix
  matches the stint line's wording in all three cases.
- §5: `carry_in == 0` unchanged; `carry_in != 0` shows the expanded
  form with `worked = fulfillment - carry_in`, sign formatting per
  §4.2 — covering a carry-in deficit (negative), a carry-in surplus
  (positive), and the case where `fulfillment` itself goes negative
  from a deficit larger than `worked`.
- Regression: existing `status`/`week` golden-output tests in
  `src/status.rs`/`src/week_view.rs` pass unchanged unless they hit
  one of the above conditions.

## 7. SPEC.md / NOTES.md updates

- §1.2a: remove bullets 1, 2, 6, 7. Keep bullets 3, 4, 5.
- §4.3.1: replace the "no visual marker" closing line with the
  duration-suffix (§2) and header-suffix (§3) description.
- §7.1: document the header suffix (§3) and the carry-in-expanded
  parenthetical (§5).
- §7.2: note the caption (§4) at the existing stale-open discussion,
  and the new status/week asymmetry (§3) alongside it.
- §7.1: also document Day total's `(+ unclosed)` variant (§4)
  alongside its existing `(+ ongoing)` description.
- NOTES.md: decision entry for this changeset.

## 8. Out of scope

- §1.2a bullet 4.
- §1.2a bullets 3 and 5 (splice-vs-flag gate, lone-unclosed-`start`
  distinction) — undecided how to disclose; held pending further
  thought, not committed to a doc-only fix.
- `classify`/`classify_at`/week accounting math.
- The §4.3.1 residual (non-1:1) case.
- A `week`-side marker for the date §3's header suffix covers — the
  resulting status/week asymmetry (§3) is accepted, not fixed here.
