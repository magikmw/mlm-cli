# Fix pass — current-week-ahead-wording

Actioned the triaged findings from
`current-week-ahead-wording-fresh-eyes.md`. Docs only, no `src/`
changes.

## F1 — day-total pace hint uses week fulfillment, not the day total

Decision: file as a known issue, no behavior change.

- `docs/dev/SPEC.md` §1.2a: added a bullet stating the `Day total:`
  line's pace-hint clause is built from the week's running
  fulfillment, not from the day total printed on the same line, and
  that nothing on the line marks the switch.
- `README.md` "Known issues": added the matching bullet in the same
  terms.

## F2 — README never explains what the pace hint measures

Decision: fix.

- `README.md`, in the `status` section right after the existing
  sentence about the pace hint and estimated-EOD line only showing up
  for today, added a sentence explaining the pace-hint clause compares
  the week's running fulfillment (worked time plus carry-in) against
  how much of the week's target should be done by end of today, so it
  isn't derived from the day total printed before it. Matches SPEC.md
  §7.1's pace-hint bullet ("This gap is driven by *fulfillment*, not by
  the 'Day total' figure printed right before it on the same line").

Verified against real output: built `target/release/mlm`, ran against
a scratch DB (`MLM_DB_PATH` under the session scratchpad, never the
real app-data DB), logged 6h across two backdated days plus 1h today
against a 10h/week target. `status` printed `Day total: 01h 00m, 03h
00m over 10h 00m required today` — the 3h-over figure is `13h`
fulfillment vs `10h` required, not derived from the `01h 00m` day
total. Confirms the README's new explanation.

## F3 — zero-boundary wording (`00h 00m left` vs `Total ahead: 00h 00m`)

Decision: already a known issue, decided against changing. No edit
made.

## F4 — Mon–Thu pace hint always names today's own weekday

Decision: file as a known issue, no behavior change.

- `docs/dev/SPEC.md` §1.2a: added a bullet noting that Monday through
  Thursday, `required by end of <weekday>` always names today's own
  weekday (since `required` is only ever prorated through today), that
  `required today` would read correctly every day, and that the
  wording only switches to that form on Friday/Saturday/Sunday once
  `required` plateaus at the week's full target.
- `README.md` "Known issues": added the matching bullet.

## F5 — positive note

Nothing to do.

## Verification

- `cargo fmt` — clean, no diff.
- `cargo build` — clean.
- `cargo test` — all passing.
- `cargo clippy --all-targets -- -D warnings` — clean.
- Manual `status` run against a scratch DB, described under F2 above.

## Files touched

- `README.md`
- `docs/dev/SPEC.md`
