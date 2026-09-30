# Fresh-eyes report — open-punch-visibility

Ran against the built `mlm` binary and its bundled `README.md`, with
`MLM_DB_PATH` pointed at scratch databases per scenario. No prior
knowledge of the source used — only `--help` output and the README.

## Scenarios — what you ran, what it printed

**1. Forgot to clock out Monday, checked today and started today's work.**

```
$ mlm start --date -2 09:00 "started work"    # Monday, 2 days ago
$ mlm status
Wed 2026-09-30
Day total:     00h 00m, 24h 00m left to 24h 00m required by end of Wednesday
Week 2026-40:  40h 00m left by end of Wednesday (fulfillment 00h 00m / target 40h 00m)

[!] unclosed start on Mon 2026-09-28 at 09:00 (2 days ago)
    fix: mlm stop HH:MM --date 2026-09-28  (a time after 09:00)
     or: mlm delete punch --date 2026-09-28  (lists punches; rerun with the start's number)
$ mlm start 09:00
warning: unclosed start on Mon 2026-09-28 at 09:00 (2 days ago); fix: mlm stop HH:MM --date 2026-09-28 (a time after 09:00)
```

The tool told me plainly, on both `status` and `start`, which date was
broken and roughly what to do. Yes, I could fix it from this alone,
modulo the issue in scenario 2 below.

**2. Followed the fix exactly as printed.**

```
$ mlm stop HH:MM --date 2026-09-28
error: invalid time "HH:MM": expected HH:MM, HHMM, or HH (24-hour)
```

Typing the `fix:` line character-for-character fails. `HH:MM` is a
placeholder standing in for a real time, but nothing marks it as one
(no angle brackets, no distinct casing convention, nothing) — it's
formatted identically to the literal, copy-pasteable date and flag
around it. Substituting a real time worked as expected:

```
$ mlm stop 17:30 --date 2026-09-28
$ mlm status
... (warning gone from today's status)
$ mlm status 2026-09-28
Mon 2026-09-28
Day total:     08h 30m
Week 2026-40:  31h 30m left by end of Wednesday (fulfillment 08h 30m / target 40h 00m)
  09:00-17:30  (08h 30m)
```

Confirmed fixed and the warning disappeared from today's `status`.

**3. Two starts, no stop, same past day.**

```
$ mlm start --date -3 09:00 "first start"
$ mlm start --date -3 13:00 "second start by mistake"
warning: 2 open stints on Sun 2026-09-27 (3 days ago); see mlm status 2026-09-27
$ mlm status 2026-09-27
Sun 2026-09-27
Day total:     00h 00m (+ unclosed)
Week 2026-39:  Total behind: 40h 00m

[!] 2 open stints for this date (only one can be open at a time)
    fix: mlm delete punch --date 2026-09-27  (lists punches; rerun with the extra start's number)
     or: mlm stop HH:MM --date 2026-09-27  (a time after 13:00 closes the 13:00 start)

  09:00-now    (unclosed)
  13:00-now    (unclosed)
```

Clear enough on its own. But `mlm week` (bare, current week 2026-40)
showed no flag at all and a `Carry-in: -40h 00m` with no explanation —
see Finding 3 below.

```
$ mlm week
Week 2026-40 (2026-09-28 - 2026-10-04)
80h 00m left by end of Wednesday
  Mon 2026-09-28   00h 00m
  ...
Carry-in:      -40h 00m
Worked:        00h 00m
Fulfillment:   -40h 00m
Target:        40h 00m
```

`mlm week 2026-39` (the week actually containing the bad day) does
show the `[!]` footer correctly.

**4. Late-night session, stop just after midnight, plus a stray extra stop the next day.**

```
$ mlm start --date -3 22:00 "late session"
$ mlm stop --date -2 00:30 "clocked out after midnight"
$ mlm stop --date -2 18:00 "stray extra stop"
$ mlm status -3
Sun 2026-09-27
Day total:     00h 00m (+ unclosed)
Week 2026-39:  Total behind: 40h 00m

[!] unclosed start at 22:00, 3 days old (no stop recorded)
    fix: mlm stop HH:MM --date 2026-09-27  (a time after 22:00)
     or: mlm delete punch --date 2026-09-27  (lists punches; rerun with the start's number)
[!] not joined to Mon 2026-09-28 across midnight (a join needs exactly one open start on Sun 2026-09-27 and exactly one unmatched end, as Mon 2026-09-28's first punch)
  22:00-now    (unclosed)

$ mlm status -2
Mon 2026-09-28
Day total:     00h 00m
Week 2026-40:  80h 00m left by end of Wednesday (fulfillment -40h 00m = worked 00h 00m + carry-in -40h 00m / target 40h 00m)

[!] orphaned end at 00:30 (no matching start)
    fix: mlm start HH:MM --date 2026-09-28  (a time before 00:30)
     or: mlm delete punch --date 2026-09-28  (lists punches; rerun with the end's number)
[!] orphaned end at 18:00 (no matching start)
    fix: mlm start HH:MM --date 2026-09-28  (a time before 18:00)
     or: mlm delete punch --date 2026-09-28  (lists punches; rerun with the end's number)
[!] not joined to Sun 2026-09-27 across midnight (a join needs exactly one open start on Sun 2026-09-27 and exactly one unmatched end, as Mon 2026-09-28's first punch)
```

Both days explain, in their own words, why the join didn't happen —
the "not joined ... across midnight" line on each date spells out the
exact condition (one open start, one unmatched end as the very first
punch of the next day) that wasn't met. Piecing together *why my two
extra ends broke the join* took a careful re-read but was
answerable from the printed text alone. Reasonably satisfied with
this one.

**5. Clocked in yesterday evening, still working past midnight; check today and the week.**

```
$ mlm start --date -1 22:00 "evening session"
$ mlm status
Wed 2026-09-30
Day total:     00h 00m, 24h 00m left to 24h 00m required by end of Wednesday
Week 2026-40:  40h 00m left by end of Wednesday (fulfillment 00h 00m / target 40h 00m)
```

No stint list, no notes, no mention anywhere on today's view that
there's a session still running from yesterday. Nothing is
incorrectly flagged — but nothing is flagged *at all*, and that's the
finding: see Finding 4.

```
$ mlm status -1
Tue 2026-09-29
Day total:     00h 00m (+ ongoing)
Week 2026-40:  40h 00m left by end of Wednesday (fulfillment 00h 00m / target 40h 00m)
  22:00-now    (17h 44m, ongoing - duration as of right now, not a running total)
```

Only visible by explicitly checking yesterday. `mlm week` for the
current week shows nothing off either — consistent with the above,
same gap.

**6. Week view whenever something looked wrong.**

Covered inline above (scenarios 3 and 4). Bare `mlm week` on the
*current* week is reliable when the flagged day is inside that week
(scenario 4's Week 2026-40 correctly showed `[!]` on Mon 2026-09-28
and pointed me at `mlm status DATE`). It's unreliable — silently so —
when the flagged day is in a *previous*, closed week whose bad numbers
have already flowed into the current week's carry-in (scenario 3).

## Findings — each with the scenario, what you expected, what you got, why it would confuse someone

**Finding 1 — `fix:` line's `HH:MM` placeholder is not distinguishable from literal text (Scenarios 1, 2, 3, 4).**
Expected: a command I can type character-for-character, since the task
literally says "run with no ID first" and other fix lines elsewhere in
the README are meant to be copy-pasted verbatim (e.g. `delete note`'s
"to recreate: ..." line, which *is* fully literal). Got: `mlm stop
HH:MM --date 2026-09-28` printed as the fix, which errors verbatim
(`invalid time "HH:MM"`) because `HH:MM` is a placeholder for a real
clock time, not a literal token. Every `fix:`/`or:` line that includes
a time repeats this (scenarios 1, 3, 4 all show it). A first-time user
following the tool's own printed instructions hits an error on the
very first attempt. Cheap fix: bracket placeholders (`<HH:MM>`) or use
some other visual marker so the fully-literal `delete punch ...`
alternative line and the templated `stop`/`start` line don't look like
they're both meant to be typed as-is.

**Finding 2 — no distinction in severity/urgency between "still-open, recent" and "broken" states across the two paths that report open starts (Scenarios 1, 5).**
`start`'s post-write staleness scan and `status`'s "last 14 days" scan
both use a 2-day-or-older threshold before anything is surfaced. That
is a deliberate, sensible design (don't nag about a session you
plausibly haven't closed yet), and it's documented. But it means the
only way to *confirm* "yes, my clock-in from last night registered and
is still running" is to explicitly query yesterday's date — today's
`status` gives zero signal, not even a friendly "you have an open
session from yesterday, X hours so far" note. This isn't a bug against
the documented threshold, but it is a real gap for the exact case this
branch is named after: someone working past midnight, checking `mlm
status` on their new day out of habit, gets a page that looks like a
totally fresh, empty day with no hint that a session is still
ticking.

**Finding 3 — a past week's anomaly silently poisons the current week's carry-in with no flag anywhere in the current week's view (Scenario 3).**
Expected: if the current week's `Carry-in` figure is unreliable
because it was computed from a week that has an unresolved `[!]`
problem, I'd expect some indication of that on the view that actually
shows the tainted number. Got: `mlm week` (bare, showing the current
week 2026-40) displayed `Carry-in: -40h 00m` and no `[!]` footer at
all, because the flagged day (two open starts, no stop) fell in the
*previous* week 2026-39, and the footer only fires for `[!]` rows
inside the week being displayed. Only `mlm week 2026-39` explicitly
shows the problem. A reasonable person who runs their normal `mlm
week` and sees a scary `-40h 00m` carry-in with no accompanying
explanation, and no `[!]` telling them to look further, has no way to
know from that screen that the number traces back to an unresolved
punch problem, let alone which week to go check. This is exactly the
scenario the task description points at ("look at the week view
whenever something looks wrong ... do you know what to do next") —
here the honest answer is no, because nothing on that screen says
anything looks wrong in the first place.

**Finding 4 (minor) — README doesn't mention the carry-in-poisoning interaction at all.**
The README documents carry-in mechanics (weekly rollover of
over/under against target) and documents `[!]` anomaly reporting
separately, but nowhere connects the two — that an *unresolved*
anomaly in a past week will produce a carry-in figure in the following
week(s) that doesn't mean what it normally means (a real
under/over-target deficit) until the source day is fixed. Given
Finding 3 above, this is a real behavior gap in the documentation, not
just an omission of a nice-to-have.

## Verdict

findings(4)
