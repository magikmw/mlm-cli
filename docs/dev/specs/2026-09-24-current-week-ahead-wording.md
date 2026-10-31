# Current-week "ahead of target" wording — design spec

> **Archived — historical changeset spec.** Superseded by `docs/dev/SPEC.md`, which folds in the decisions made here.

**Baseline**: written against `v0.3.6`.
**Status**: adversarially reviewed 2026-09-24
(`docs/dev/plans/reports/current-week-ahead-wording-spec-review.md`,
ship-with-followups, 3 findings, folded — NOTES.md entry 73). Locked.

## 1. Summary

The current-week headline (`render.rs`'s `week_headline`,
`WeekFraming::Current` branch) always renders
`"{owed} left by end of {weekday}"` with `owed` signed per §4.2. Once
the week's fulfillment passes its target, `owed` goes negative and the
line reads `-04h 35m left by end of Saturday`. A negative "left"
figure is a double negative, and the weekday deadline no longer means
anything once the target is met.

The same headline feeds both surfaces:

- `status`'s week line (`status_week_line`, §7.1), label and
  fulfillment/target parenthetical around it
- `week`'s headline line (§7.2), bare

Both already sit next to wording that avoids bare signs. The day-total
pace hint switches between `left to` and `over` and prints an absolute
value (§7.1). The closed-week headline switches between `Total behind`
and `Total ahead` (decision 18). The current-week headline is the one
figure still printing a signed "left".

The fix switches wording on the sign of `owed`, like its siblings:

| `owed` | Current-week headline |
|---|---|
| `> 0` | `{owed} left by end of {weekday}` (unchanged) |
| `== 0` | `00h 00m left by end of {weekday}` (unchanged) |
| `< 0` | `{abs(owed)} ahead of target` |

The ahead form drops the weekday deadline entirely. `ahead` pairs with
the closed-week `Total ahead`, so a week that ends ahead keeps the
same word across the Current → Closed transition.

## 1.1 Non-goals

- No change to how `owed`, `fulfillment`, or `carry_in` are computed.
  Presentation only.
- No change to the closed-week headline (`Total behind`/`Total
  ahead`), including its zero case, which reads `Total ahead: 00h 00m`.
- No change to the `status` week line's fulfillment/target
  parenthetical. It still prints in the ahead case, signed per §4.2.
- No change to the day-total pace hint or `est. EOD`.
- No change to the §1.2a known issue about future `DATE`s naming
  today's weekday. The ahead form drops the weekday, so that issue
  simply doesn't surface on ahead weeks.

## 1.2 Known issues

- The zero boundary differs between framings. A current week at
  exactly target reads `00h 00m left by end of <weekday>`, but the
  same week once closed reads `Total ahead: 00h 00m`. Both behaviours
  predate this changeset. The current-week zero case stays "left" to
  match the pace hint's `gap == 0` rule (§7.1) and the existing test
  at `render.rs:269`.
- On an ahead week, `status` for today can show the day-total pace
  hint naming a weekday (`required by end of Thursday`) directly
  above a week line that names none. Before this changeset both lines
  on today's page always named the deadline. The two figures measure
  different things (§7.1), and the week line has no deadline left
  once the target is met, so this is accepted rather than worked
  around. SPEC.md §7.1 records it as intended; it is not filed in
  §1.2a.

## 2. Prerequisites

None.

## 3. Behavior

`week_headline`'s `WeekFraming::Current` arm splits on the sign of
`owed_minutes`:

- `owed_minutes >= 0` → `format!("{} left by end of {}", format_minutes(owed_minutes), format_weekday_full(today))` (unchanged)
- `owed_minutes < 0` → `format!("{} ahead of target", format_minutes(-owed_minutes))`

`status_week_line` needs no change. It wraps whatever
`week_headline` returns, so the `status` week line becomes, for
example:

```
Week 2026-37:  04h 35m ahead of target (fulfillment 44h 35m / target 40h 00m)
```

and with a non-zero carry-in:

```
Week 2026-37:  01h 00m ahead of target (fulfillment 41h 00m = worked 38h 00m + carry-in 03h 00m / target 40h 00m)
```

`week`'s headline becomes the bare `04h 35m ahead of target`.

A carry-in surplus can put a week ahead before any work is logged.
The ahead form applies the same way, since it keys only on `owed`.

## 4. Tests

- `render.rs:340` `t11_current_week_already_ahead_passes_sign_through_verbatim`
  pins the old `-02h 30m left by end of Thursday`. Rewrite it to
  expect `02h 30m ahead of target` and rename it to match.
- Add a `status_week_line` case for the ahead form with the short
  parenthetical, and one with the expanded carry-in parenthetical.
- Add a boundary case, `owed_minutes == -1` →
  `00h 01m ahead of target`. The existing zero case (`render.rs:269`)
  already covers `== 0` → `left`.
- Add one pipeline test in `status.rs`'s test module. It drives
  `resolve()` then `render()` for a current week whose fulfillment
  exceeds target, and asserts the week line reads `… ahead of target
  (fulfillment …`.
- Add one pipeline test in `week_view.rs`'s test module. It drives
  `build_week_view()` then `render_week()` for the same shape, and
  asserts the headline reads `… ahead of target`. These two cover the
  path a user actually hits. Unit tests alone only prove the pure
  function.
- Implementation re-greps `src/` and `tests/` for `left by end of`
  and for negative-left literals rather than trusting this list.

## 5. Docs

- **README.md**: three examples show the old form (lines 193, 213,
  261, all `-04h 35m left by end of Saturday`). They become
  `04h 35m ahead of target`. Line 290's `00h 25m left by end of
  Saturday` stays unchanged.
- **SPEC.md §7.1** (week-line bullet, around line 676) gains the
  ahead form alongside the `left by end of` form, with the sign rule
  and the reason the weekday drops.
  It also states that on an ahead week the pace hint above may name
  a weekday while the week line names none, and that this is
  intended (§1.2 second bullet).
- **SPEC.md §7.2** (headline bullet, around line 780) refers back to
  §7.1 for the same split.
- **SPEC.md §5** (around line 502) keeps "a negative `owed` … means
  already-ahead-of-pace". Add that the headline renders it as
  `X ahead of target`.
- **SPEC.md §1.2a** gains the zero-boundary known issue from §1.2
  above.
- **README "Known issues"** mirrors that bullet.

## 6. Task split

- **Task 1 (code)**: `src/render.rs`, plus the `#[cfg(test)]`
  modules of `src/status.rs` and `src/week_view.rs`. The
  `week_headline` change and all tests in §4. No non-test code in
  `status.rs` or `week_view.rs` changes.
- **Task 2 (docs)**: `README.md` and `docs/dev/SPEC.md` only.

## Interaction with the project spec

- §7.1 week-line bullet: gains the `owed < 0` form.
- §7.2 headline bullet: same, by reference to §7.1.
- §5 negative-`owed` paragraph: gains the rendered form.
- §1.2a: gains one known issue.
- NOTES.md decision 18 framed the current week only as "`X` left by
  end of `<weekday>`". The decision log entry for this changeset
  supersedes that for `owed < 0`. Decision 18's closed-week strings
  are also stale (`Total still owed`, since renamed to `Total
  behind`), so the new entry quotes the shipped strings
  (`render.rs:68,70`), not decision 18's.
