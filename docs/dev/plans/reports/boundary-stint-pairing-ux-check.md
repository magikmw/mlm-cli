# Boundary stint pairing — first-time-user UX check

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.


Role-played as a fresh user of `mlm`, built from `cargo build --release`, driving the
binary directly (`./target/release/mlm`) against scratch SQLite DBs
(`MLM_DB_PATH=/tmp/mlm-ux-check.db` and a second `/tmp/mlm-ux-check2.db`). No spec/plan
docs were consulted; judgments are "would a normal user reading this be confused"
only. Today's real date in the environment is 2026-09-19, so all backdated dates below
were chosen in the past.

---

## Scenario 1 — overnight session (clean midnight crossing)

Commands:
```
mlm start 23:30 -d 2026-09-10
mlm stop  00:45 -d 2026-09-11
mlm status 2026-09-10
mlm status 2026-09-11
mlm week 2026-37
```

Output for `status 2026-09-10`:
```
Thu 2026-09-10

Day total:     01h 15m
Week 2026-37:  Total still owed: 38h 45m

  23:30-00:45  (01h 15m)
```

Output for `status 2026-09-11`:
```
Fri 2026-09-11

Day total:     00h 00m
Week 2026-37:  Total still owed: 38h 45m
```

Output for `week 2026-37`: the full 1h15m lands on the Thu row only; Fri row is 00h 00m.

**Reaction: mostly clear, one nitpick.**
- The stint line `23:30-00:45 (01h 15m)` is genuinely legible — the duration in
  parentheses removes any ambiguity about whether `00:45` means "45 minutes before
  midnight" or "just after". A user glancing at just the two times without the
  duration might do a double-take (end < start numerically), but the parenthetical
  duration resolves it immediately, so overall this reads fine.
- **Nitpick:** there is no visual marker (e.g. a `+1d` suffix, or a note) telling the
  user *this stint crosses midnight*. It's inferable only by noticing the times look
  "backwards" and doing the math. A small `(01h 15m, ends next day)` or similar would
  remove all doubt instead of relying on the reader noticing.
- `status 2026-09-11` shows a completely empty day (0 stints, 0 total) even though a
  `stop` punch was explicitly typed against that date. A user who ran
  `mlm stop 00:45 -d 2026-09-11` and then immediately checked `mlm status 2026-09-11`
  would see nothing acknowledging that punch happened — it's fully absorbed into the
  previous day with zero trace. This is the single biggest first-read confusion point
  in this scenario: **"I told it to record something on the 11th, why does the 11th
  show nothing at all?"** No hint, no footnote, nothing pointing back to the 10th.

---

## Scenario 2 — `stop` and `start` at the identical time, same day

Commands:
```
mlm start 08:00 -d 2026-09-15
mlm stop  09:00 -d 2026-09-15
mlm start 09:00 -d 2026-09-15
mlm stop  17:00 -d 2026-09-15
mlm status 2026-09-15
```

Output:
```
Tue 2026-09-15

Day total:     09h 00m
Week 2026-38:  69h 45m left by end of Saturday (fulfillment -29h 45m / target 40h 00m)

  08:00-09:00  (01h 00m)
  09:00-17:00  (08h 00m)
```

**Reaction: neutral-to-clear, reads naturally, but slightly ambiguous.**
- Two adjacent stints sharing a boundary time (`...09:00` then `09:00...`) render as
  two touching lines. This reads reasonably as "two separate stints" rather than a
  glitch — good.
- **Nitpick:** because the two lines touch exactly (`09:00` end = `09:00` start) with
  no gap and no indication of *why* there were two punches instead of one continuous
  8:00–17:00 stint, a user skimming quickly could plausibly misread this as a
  rendering duplicate/bug (same timestamp appearing twice) rather than two
  deliberate, separate work sessions. There's nothing distinguishing "user briefly
  stepped away for 0 minutes" from "user typo'd an extra start/stop pair". Day total
  is correctly summed (9h00m), so no math error, just an ambiguous read.
- Also worth noting: the `Week 2026-38: 69h 45m left by end of Saturday (fulfillment
  -29h 45m / target 40h 00m)` line is confusing on its own — see the cross-cutting
  note on "fulfillment"/"carry-in" below.

---

## Scenario 3 — messy case: two dangling starts + an orphaned end the next day

Commands:
```
mlm start 09:00 -d 2026-09-01
mlm start 14:00 -d 2026-09-01
mlm stop  10:00 -d 2026-09-02
mlm status 2026-09-01
mlm status 2026-09-02
mlm week 2026-36
```

Output for `status 2026-09-01`:
```
Tue 2026-09-01

Day total:     00h 00m (+ ongoing)
Week 2026-36:  Total still owed: 40h 00m

[!] 2 open stints for this date (unmatched starts)

  09:00-now    (443h 06m, ongoing)
  14:00-now    (438h 06m, ongoing)
```

Output for `status 2026-09-02`:
```
Wed 2026-09-02

Day total:     00h 00m
Week 2026-36:  Total still owed: 40h 00m

[!] orphaned end at 10:00 (no matching start)
```

Output for `week 2026-36` (row totals only):
```
  Tue 2026-09-01   00h 00m  [!]
  Wed 2026-09-02   00h 00m  [!]
```

**Reaction: confusing, and directly exposes an inconsistency with Scenario 1.**
- This is the headline finding. In Scenario 1, a single `start` on day N followed by a
  `stop` on day N+1 **silently merges** into one clean cross-midnight stint with no
  flag, no warning, nothing unusual in the output at all. Here, two `start`s on day N
  followed by one `stop` on day N+1 **does not merge at all** — neither start is
  paired with the next-day stop, both remain "open"/ongoing forever, and the stop
  becomes a separately-flagged "orphaned end". A user who saw Scenario-1-style
  behavior first would reasonably expect the *closest* start (14:00, the second one)
  to pair with the next day's 10:00 stop the same way it did before. Instead nothing
  pairs, and the tool gives no explanation of *why* this case is different from the
  clean case — there's no way for a user to tell, from the output alone, that "2 open
  starts on one day" is what breaks the automatic pairing that otherwise works fine
  across midnight. **This inconsistency (sometimes silently merges, sometimes flags
  and refuses to merge) is exactly the kind of thing that erodes trust in the tool:
  the user can't predict which behavior they'll get.**
- The `[!]` markers are a good idea in isolation (much better than silently doing
  something wrong), but they appear with **zero explanatory text** anywhere in the
  output about what a user should *do* about them — no suggested fix, no pointer to
  `delete`/edit commands, nothing. "2 open stints for this date (unmatched starts)"
  and "orphaned end at 10:00 (no matching start)" describe the state but not the
  remedy.
- **The `(443h 06m, ongoing)` / `(438h 06m, ongoing)` durations look alarming at a
  glance.** These are technically correct given the backdated punch is being measured
  against the real current time, but a user who backdates a punch (a first-class,
  documented feature via `-d`) and then checks status would see "443 hours" attached
  to a stint they created moments ago — that reads as a bug or overflow at first
  glance, not as "this is still open and it's been N real days since". No caption
  clarifies this is elapsed-since-real-now rather than some computed total.
- Week view: the `[!]` suffix on the Tue/Wed rows is a nice signal that something's
  off on those specific days, but again **no legend** anywhere in the `week` output
  explains what `[!]` means — a user would have to already know, or go dig into
  `status` for that date, to find out.

---

## Additional observations (scenario 4 / general poking)

- **`status` for a date with only one dangling `start` (no second start, no stop at
  all) shows NO `[!]` flag** — just `Day total: 00h 00m (+ ongoing)` and a normal
  `HH:MM-now (…, ongoing)` line. This is a reasonable design (a single open start really
  could just mean "still clocked in"), but combined with the Scenario 3 finding above,
  it means the exact same surface signal ("ongoing" stint, no stop yet) is silent in
  one case and loudly flagged in another, and the difference (1 open start = fine, 2
  open starts = anomaly) is not explained anywhere in `--help` or in the output itself.
- **Bad time input errors are good and clear:**
  - `mlm start 99:99 -d 2026-09-06` → `error: time "99:99" is out of range: valid
    times are 00:00 through 23:59` — clear, actionable.
  - `mlm start abc -d 2026-09-06` → `error: invalid time "abc": expected HH:MM, HHMM,
    or HH (24-hour)` — clear, actionable.
  - Future-date guard is also clear: `error: invalid DATE "2026-09-20": date is in the
    future` (hit this by accident while picking scratch dates — good error, no
    complaints).
- **`mlm start --help` / `mlm stop --help` leak an internal doc path into user-facing
  help text**, which a real user has no access to and can't open:
  > `Must come before NOTE text on the command line, or it is silently absorbed into
  > the note body instead of being parsed as this flag -- see
  > docs/dev/specs/2026-09-13-backdated-punches.md §2.1`
  This isn't part of the boundary-pairing scenarios per se, but it's a clear rough
  edge: shipped `--help` output pointing at a repo-internal design-spec path that
  won't exist for anyone who installed the binary from a release. The behavior itself
  (`-d` must precede note text) should really just be explained inline instead of
  citing a document.
- **The `week`/`status` "fulfillment" and "carry-in" wording is opaque without
  context.** In Scenario 2, `status` shows:
  `Week 2026-38: 69h 45m left by end of Saturday (fulfillment -29h 45m / target 40h 00m)`
  despite that day alone showing a full 9h00m worked. Only by separately running
  `mlm week` does the "Carry-in: -38h 45m" line (debt rolled over from the prior,
  under-worked week) explain the negative fulfillment. On the `status` page itself,
  "carry-in" is never mentioned, so a user who only ever runs `status` sees a
  confusing negative "fulfillment" number with no idea where the negative number came
  from, right after logging a perfectly good 9-hour day. Not part of the
  midnight-pairing feature under test, but a real point of first-read confusion
  encountered while poking around.
- Minor: `mlm delete` requires a subcommand (`note`/`punch`) and does not accept a bare
  date positionally (`mlm delete 2026-09-01` errors with "unrecognized subcommand").
  Not part of the tested scenarios, only noted in passing.

---

## Summary of flagged UX concerns

1. Cross-midnight stint (`23:30-00:45`) gives no visual cue that it spans two calendar
   days — relies on the reader noticing end < start.
2. The day that received the actual `stop` punch (day N+1 in an overnight stint) shows
   a totally empty status with zero trace of the punch that was recorded against it.
3. Same-instant back-to-back stop/start renders as two touching stint lines with no
   distinguishing detail — plausible to misread as a duplicate/rendering glitch rather
   than two deliberate punches.
4. **Core inconsistency:** a single start + next-day stop silently merges across
   midnight (Scenario 1); two starts + next-day stop does not merge at all and instead
   flags both as unmatched/orphaned (Scenario 3) — this difference in behavior is not
   explained anywhere in the output, docs, or `--help`.
5. `[!]` anomaly flags (in both `status` and `week`) describe the problem but give no
   guidance on how to fix it.
6. Ongoing-stint durations computed against real wall-clock "now" against a backdated
   punch produce alarming-looking numbers (e.g. "443h 06m, ongoing") with no caption
   explaining they're elapsed-since-real-now rather than a bug.
7. Whether a lone open start is flagged as an anomaly depends on whether a second open
   start also exists that day — inconsistent signal for what looks like the same
   underlying situation ("stint never got a stop").
8. `start --help` / `stop --help` cite an internal repo doc path
   (`docs/dev/specs/2026-09-13-backdated-punches.md §2.1`) that a real end user has no
   access to.
9. Negative "fulfillment" shown on `status` (driven by carried-over deficit from a
   prior week) is unexplained on that screen — "carry-in" context only appears on the
   separate `week` command.
