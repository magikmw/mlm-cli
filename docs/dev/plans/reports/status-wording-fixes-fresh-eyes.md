# mlm fresh-eyes report — status/week wording

Sandbox: `/tmp/claude-659200001/mlm-fresh-eyes-sandbox/`, binary `mlm 0.3.5`, scratch DB via `MLM_DB_PATH`. Real system date during this session: Wed 2026-09-23.

## Scenarios — what you ran, what it printed

**1. Punch in/out today plus backdated earlier days, then `status`/`week`.**

Backdated Monday and Tuesday (two stints each), then punched today (two stints). `status` gave:

```
Wed 2026-09-23

Day total:     06h 00m, 03h 15m left to 24h 00m required by end of Wednesday
Week 2026-39:  19h 15m left by end of Wednesday (fulfillment 20h 45m / target 40h 00m)

  09:00-12:00  (03h 00m)
  13:00-16:00  (03h 00m)

Notes:
  - started today
  - afternoon done
```

`week` gave the matching per-day table with the same "19h 15m left by end of Wednesday" headline. Numbers were internally consistent (24h00m = 3 weekdays × 8h/day pace; 20h45m fulfillment = 24h − 3h15m).

**2. `status` for a date in an already-closed week (`-9`, landing on Mon 2026-09-14).**

```
Mon 2026-09-14

Day total:     00h 00m
Week 2026-38:  Total behind: 40h 00m
```

That week had zero punches at all (never tracked), so "Total behind: 40h 00m" is technically correct but reads identically to "you worked 0 out of 40 and fell behind," with no way to tell "untracked" from "tracked and missed."

**3. `week 38` (a past week number).**

```
Week 2026-38 (2026-09-14 - 2026-09-20)

Total behind: 40h 00m

  Mon 2026-09-14   00h 00m
  ...
Carry-in:      00h 00m
Worked:        00h 00m
Fulfillment:   00h 00m
Target:        40h 00m
```

Matches the README's past-week example shape exactly. Later repopulated week 38 with real punches (Fri 08h, Sat 02h) and re-ran — output shape held up, "Total behind: 30h 00m" tracked correctly against the new 10h00m worked total.

**4. Racking up a lot of open time to push `est. EOD` past midnight.**

Opened a stint today, then repeatedly raised the week's target via `week target` to inflate the "required" gap (so the estimate would land far in the future without needing to fabricate a huge single stint). Results:

| Target | Day-total line | est. EOD |
|---|---|---|
| 50h | `09h 15m left to 30h 00m required...` | `est. EOD 00:17 (tomorrow)` — genuinely next day, correct |
| 200h | `99h 15m left to 120h 00m required...` | `est. EOD 18:17 (tomorrow)` — actually ~5 days out |
| 500h | `279h 15m left to 300h 00m required...` | `est. EOD 06:17 (tomorrow)` — actually ~12 days out |
| 2000h | `1179h 15m left to 1200h 00m required...` | `est. EOD 17:17 (tomorrow)` — actually ~49 days out |

**5. `status` on a Friday/Saturday/Sunday with both lines populated.**

Couldn't move the real "today" off Wednesday (no `faketime` in the sandbox, and I'm not touching the system clock). `status` does accept a future `DATE` though (undocumented — README only says future dates are rejected for `start`/`stop`/`note`), so I queried Fri/Sat/Sun of the *current* week that way:

```
$ mlm status 2026-09-25
Fri 2026-09-25

Day total:     00h 00m
Week 2026-39:  48h 45m left by end of Wednesday (fulfillment -08h 45m = worked 21h 15m + carry-in -30h 00m / target 40h 00m)
```

Same for 2026-09-26 (Sat) and 2026-09-27 (Sun) — identical week line each time, all three still anchored to "end of Wednesday" (the real current day), regardless of which day's page you're looking at.

**6. Read README examples end to end, cross-check Known Issues.**

Reproduced all three documented known issues directly:

- Forgotten `stop` from days ago (started `-6`, never stopped): visible on that exact date's `status` (`Day total: 00h 00m (+ unclosed)`), completely absent from today's `status`, today's `week`, and even that day's own row in `week 38`'s per-day table (shows `00h 00m` there too). Matches the documented issue exactly.
- Lone unclosed `start` on a date: no flag. A second unclosed `start` added to the same date: triggers `[!] 2 open stints for this date (unmatched starts)`. Matches documented issue exactly.
- A stint crossing midnight (started 23:00 yesterday, stopped 01:00 today, both via separate `start`/`stop` calls): auto-resolved onto the previous day. Today's `status` header grew an annotation, `Wed 2026-09-23  (01:00 continues previous day's stint)`, and the 2h stint counted entirely toward *yesterday's* total, not today's, with no entry in today's stint list. The annotation is a genuinely helpful touch not mentioned in the README's Known Issues text (which only says the resolution rule "isn't explained" — here it partly is, just not in the doc).

## Findings — each with the scenario, what you expected, what you got, why it would confuse someone

**Finding 1 — The "Day total" line secretly reports week-level numbers, not day totals.**
Scenario: 1, 5, and README's own Saturday example.
Expected: everything after "Day total:" describes *today's* hours.
Got: `Day total: 06h 00m, 03h 15m left to 24h 00m required by end of Wednesday` — the first number (`06h 00m`) is today's total, but the clause after the comma (`03h 15m left to 24h 00m required...`) is actually the week's cumulative pro-rated shortfall, computed from week fulfillment vs. a pro-rated week target — not from today's `06h 00m` at all.
Why it confuses: read literally, "3h15m left to 24h00m required... today" sounds like "you must work 24 hours today," which is alarming and wrong. It's really "by the end of today, the week should have accumulated 24h; it's short by 3h15m." The label ("Day total:") promises a day-scoped number and delivers a week-scoped one glued onto it with no visual separation. This is the same root cause behind the redundancy concern in scenario 5: the day line and the week line end up saying the same "X left/over" number twice, once framed as "required by end of day" and once as "left by end of day," because both derive from the identical week-fulfillment figure.

**Finding 2 — `est. EOD ... (tomorrow)` is wrong, not just ambiguous, when the estimate lands more than one day out.**
Scenario: 4.
Expected: either the estimate always resolves to a real day/date once it crosses midnight, or the annotation scales with how far out it actually is.
Got: `est. EOD 17:17 (tomorrow)` when the true completion time was roughly 49 days away (computed from `1179h 15m` remaining at test time). The annotation appears to be a flat "did the clock wrap past midnight at all → print `(tomorrow)`" check, with no accounting for *how many* midnights were crossed.
Why it confuses: this is worse than the scenario worried about. The scenario asked whether a large estimate could be misread as "later today" — instead the tool actively mislabels a ~7-week-out estimate as tomorrow, which is a false, specific, and wrong claim rather than a vague one. Anyone who trusted it would be misled in a concrete way ("I'll be done by 17:17 tomorrow") rather than merely confused.

**Finding 3 — `status` accepts future dates and shows current-week framing anchored to the real "today," not the date on the page.**
Scenario: 5 (and 2/3, since it's a gap versus documented date-arg behavior).
Expected: either future dates are rejected (matching the future-date ban documented for `start`/`stop`/`note`), or, if accepted, the week line frames the deadline relative to the date being viewed.
Got: `mlm status 2026-09-25` (a future Friday) succeeds and prints `Week 2026-39: 48h 45m left by end of Wednesday...` — "Wednesday" being the real current day, unrelated to the Friday page you asked for. Same text, unchanged, for Sat 09-26 and Sun 09-27 too.
Why it confuses: the page is headed "Fri 2026-09-25" but talks about a deadline "by end of Wednesday" with no Wednesday anywhere else on the page. A reader has to already know today's real weekday to realize "Wednesday" refers to *now*, not to the Friday being displayed. The README never documents that `status` accepts future dates at all, so this behavior — accept it, but keep all framing pinned to real-time "today" — is undiscoverable except by trying it.

**Finding 4 — The expanded `fulfillment = worked + carry-in` formula is never shown in README, only the simple `fulfillment X / target Y` form is.**
Scenario: 6.
Expected: README's output-format section would cover the format a user is going to see the first time they carry a shortfall into a new week (which `week target`'s own carry-in feature makes routine).
Got: every README status/week example has `Carry-in: 00h 00m`, so every shown line is the short `(fulfillment 44h 35m / target 40h 00m)` form. The moment carry-in is nonzero, `status` switches to `(fulfillment -08h 45m = worked 21h 15m + carry-in -30h 00m / target 40h 00m)` — a format with no precedent in the doc.
Why it confuses: not wrong, just a documentation gap for a feature (carry-in) the README explicitly advertises as a headline feature of `week target`. First encounter with negative fulfillment plus an unfamiliar three-term breakdown, with nothing in the doc to say "yes, this is what it looks like."

**Finding 5 — "Total behind: Nh" reads the same for an untouched week as for a worked-but-short week.**
Scenario: 2.
Expected: some way to tell "I never tracked this week" apart from "I tracked this week and came up short."
Got: a week with literally zero punches (`Worked: 00h 00m`) prints `Total behind: 40h 00m`, phrased identically to how a week with partial work would read.
Why it confuses: minor, but "behind" implies an expectation that applied to you at the time, which may not be true for a week before you started using the tool, or one you deliberately didn't track (vacation, etc). Not a big deal in isolation, more of a nitpick worth a second look.

## Verdict

findings(5)

VERDICT: findings(5)
FILE: /home/magikmw/projects/mlm-cli/docs/dev/plans/reports/status-wording-fixes-fresh-eyes.md
"Day total" line silently carries week-level "left/required" numbers derived from week fulfillment, not from the day total next to it — reads like a daily quota.
`est. EOD ... (tomorrow)` is flatly wrong once the estimate is more than one day out (tested up to ~49 days out, still says "tomorrow").
`status` silently accepts future dates (undocumented) and shows week framing anchored to the real current day/weekday, not the date being viewed — mismatched page context.
The carry-in-expanded fulfillment format (`fulfillment X = worked Y + carry-in Z / target W`) never appears in any README example, only the simple form does.
"Total behind: Nh" phrasing doesn't distinguish an untouched week from a worked-but-short one.
