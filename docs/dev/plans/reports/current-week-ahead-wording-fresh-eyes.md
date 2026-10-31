# Fresh-eyes report — current-week ahead-of-target wording

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

Ran the built `mlm` binary and read `README.md` from a clean sandbox
directory, with `MLM_DB_PATH` pointed at a fresh scratch `.db` file per
scenario. "Today" for these runs was Thu 2026-09-24, 20:00 local.

## Scenarios — what you ran, what it printed

### Scenario 1 — already past the weekly goal

Set `week target 10h`, logged Mon 6h and Tue 6h (12h total, no hours
today).

```
$ mlm status
Thu 2026-09-24

Day total:     00h 00m, 04h 00m over 08h 00m required by end of Thursday
Week 2026-39:  02h 00m ahead of target (fulfillment 12h 00m / target 10h 00m)

$ mlm week
Week 2026-39 (2026-09-21 - 2026-09-27)

02h 00m ahead of target

  Mon 2026-09-21   06h 00m
  Tue 2026-09-22   06h 00m
  Wed 2026-09-23   00h 00m
  Thu 2026-09-24   00h 00m
  Fri 2026-09-25   00h 00m
  Sat 2026-09-26   00h 00m
  Sun 2026-09-27   00h 00m

Carry-in:      00h 00m
Worked:        12h 00m
Fulfillment:   12h 00m
Target:        10h 00m
```

The `week` line and the `week` view agree with each other and are easy
to read: 2h ahead of a 10h target, 12h worked. But the `status` day-total
line is confusing on its own — see Findings.

### Scenario 2 — crossing the goal by a small margin

Set `week target 10h`, logged Mon 5h + Tue 4h30m (9h30m, just short),
checked, then logged 35m more today and checked again.

Before crossing:
```
Day total:     00h 00m, 01h 30m over 08h 00m required by end of Thursday
Week 2026-39:  00h 30m left by end of Thursday (fulfillment 09h 30m / target 10h 00m)
```
(week view headline: `00h 30m left by end of Thursday`)

After logging 18:00–18:35 today (35m, total 10h05m):
```
Day total:     00h 35m, 02h 05m over 08h 00m required by end of Thursday
Week 2026-39:  00h 05m ahead of target (fulfillment 10h 05m / target 10h 00m)

  18:00-18:35  (00h 35m)

Notes:
  - small task
```
(week view headline: `00h 05m ahead of target`)

The crossover wording itself reads well: "X left by end of Thursday"
cleanly becomes "X ahead of target" once fulfillment passes target, with
no dangling "0 left"/"negative left" artifact. This is the one place
where the wording behaved exactly as a reasonable person would want.

### Scenario 3 — landing exactly on target

Set `week target 8h`, logged Mon 4h + Tue 4h (8h exactly).

```
$ mlm status
Day total:     00h 00m, 01h 36m over 06h 24m required by end of Thursday
Week 2026-39:  00h 00m left by end of Thursday (fulfillment 08h 00m / target 08h 00m)

$ mlm week
00h 00m left by end of Thursday
...
Fulfillment:   08h 00m
Target:        08h 00m
```

Also reproduced the exact known-issue case from the README (closed week
exactly at target reads differently from the open one):

```
$ mlm week 34   # closed week, exactly at target
Total ahead: 00h 00m
```

So: currently-open week exactly at target says "00h 00m left by end of
Thursday"; a closed week exactly at target says "Total ahead: 00h 00m".
Confirmed reproducible, exactly as the README's own known-issues list
already says. "00h 00m left" reads as "done" well enough on its own,
though the reader has to notice the number is zero rather than being
told outright — a small ask, and already flagged by the project itself,
so not counted as a new finding.

### Scenario 4 — today's day-total line vs. the week line, past goal

Same setup as Scenario 1, plus 3h logged today, so day total is nonzero
too:

```
Thu 2026-09-24

Day total:     03h 00m, 07h 00m over 08h 00m required by end of Thursday
Week 2026-39:  05h 00m ahead of target (fulfillment 15h 00m / target 10h 00m)

  09:00-12:00  (03h 00m)

Notes:
  - thu morning
```

Worked out the arithmetic by reading `src/status.rs`: the day-total
line's "X over/left to Y required by end of Z" clause is computed from
`required_minutes - fulfillment_minutes`, where `fulfillment` is the
**week's** running total (carry-in + everything worked in the week so
far), not `day_total_minutes`. The code comment even says so explicitly
("NOT `day_total_minutes`"). So "Day total: 03h 00m, 07h 00m over 08h 00m
required..." is really saying "today you worked 3h; separately, the
week overall is 7h ahead of a prorated 8h-by-Thursday pace" — two
different numerators glued onto one line with no word marking the
switch. See Findings for why this is a problem.

### Scenario 5 — README examples vs. actual output

Re-ran the README's own worked examples with equivalent scratch data
(different absolute numbers, same shapes):

- Past-date-in-closed-week format (`Day total: X` / `Week N: Total
  behind: Y`, no parenthetical) — matches README exactly.
- Carry-in expansion (`fulfillment F = worked W + carry-in C / target
  T`) — reproduced it with a deficit carried from a prior week and the
  format matches README's documented shape exactly, including negative
  fulfillment being allowed (`fulfillment -33h 00m = worked 03h 00m +
  carry-in -36h 00m / target 10h 00m`).
- Current-week "ahead of target" headline and its `(fulfillment X /
  target Y)` short form — matches.

All formatting/spacing (label column alignment, duration format,
weekday names) matched the README's examples with no surprises.

## Findings

**Finding 1 (Scenario 1, 2, 3, 4 — the big one): the `status` day-total
line silently switches its subject partway through the sentence.**
Expected: everything after "Day total:" on that line describes *today's*
hours. Got: the trailing clause ("X over/left to Y required by end of
Z") is computed from the **week's** fulfillment-to-date against a
weekday-prorated target, not from the day total printed two words
earlier on the same line. Concretely, in Scenario 4, "Day total: 03h
00m, 07h 00m over 08h 00m required by end of Thursday" looks like it's
saying "you did 3h today against an 8h requirement for today, so you're
7h over" — which is arithmetically false (3h is *under* 8h, not 7h
over). The real meaning is "the week so far (15h) is 7h ahead of the
8h-by-Thursday pace." A reader has no way to know the second clause
changed subjects; nothing in the line, and nothing in the README, says
so. This is the single most confusing thing across all five scenarios,
and it reproduces on every status call where `DATE` is today, including
the README's own worked example (`Day total: 07h 25m ..., 04h 35m over
40h 00m required today, target already met` — the 4h35m there is also
week fulfillment (44h35m) vs. the day's prorated target, not derived
from the 07h25m day total shown right before it).

**Finding 2 (Scenario 5 / README gap): the README never explains what
the day-total pace-hint clause is actually computed from.** README.md
lines ~208-209 only say "The required-by-day pace hint... only show up
when DATE is today (they need 'now' to mean anything)" — it names the
hint and shows one example, but never states that the hint's numerator
is the week's running fulfillment rather than the day's own total. Since
the two numbers sit on the same labeled line ("Day total:"), a reader
has every reason to assume they're related, and the README does nothing
to correct that assumption. This directly compounds Finding 1.

**Finding 3 (Scenario 3, minor): "00h 00m left by end of Thursday" asks
the reader to notice a zero rather than telling them plainly they're
done.** Already listed in the README's own known-issues section (the
open/closed-week wording-flip item), so not new, but worth confirming
it reproduces exactly as described, including the closed-week
counterpart reading "Total ahead: 00h 00m" for the same underlying
figure of zero.

**Finding 4 (Scenario 4, minor): "required by end of Thursday" when the
date in question already *is* Thursday reads slightly oddly** — it
names the weekday instead of saying "today" even though the header two
lines up already says "Thu 2026-09-24." Not confusing, just a small
stylistic wrinkle; a reader figures out what it means immediately, it
just doesn't quite match how a person would phrase a same-day deadline
out loud.

**Finding 5 (Scenario 2, positive — noted so it isn't lost): the
specific thing this changeset was about, the current-week ahead/behind
wording, works correctly and reads well.** "X left by end of
<weekday>" flips cleanly to "X ahead of target" the moment fulfillment
crosses the target, with no leftover "0 left" or negative-left artifact,
and the parenthetical (`fulfillment X / target Y`) stays consistent
across the crossover. This is the one piece of wording in the whole
report that gave zero pause.

## Verdict

findings(4)
