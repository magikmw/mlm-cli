## Scenarios — what you ran, what it printed

Run on Saturday 2026-10-03 around 17:00 local, fresh DB per scenario (week 2026-40).

1. Open stint, waited 90 s. `start 14:00 "s1"` on an empty week. The first run printed an `[INFO rusqlite_migration] Database migrated` log line.
   - Status, then 90 s later: `Day total: 00h 00m (+ ongoing), 37h 02m left to 40h 00m required today, est. EOD 06:00 (tomorrow)`, then `37h 01m left`, est. EOD unchanged. The stint line read `14:00-now (02h 58m, ongoing)`, then `02h 59m`.
   - `Week 2026-40: 40h 00m left by end of Saturday (fulfillment 00h 00m / target 40h 00m)` stayed frozen. `week` showed `Sat ... 00h 00m (ongoing)`.
2. Lunch. `08:00-12:00`, `start 13:00`. Status: `Day total: 04h 00m (+ ongoing), 32h 00m left ..., est. EOD 01:00 (tomorrow)`. A minute later it read `31h 59m left`, est. EOD still 01:00. Week line: `36h 00m left by end of Saturday (fulfillment 04h 00m ...)`.
   - With Tue-Fri also at 8h: `Day total: 04h 00m (+ ongoing), 00h 01m over 40h 00m required today, target already met`, beside `Week 2026-40: 04h 00m left by end of Saturday`.
3. Over target. Mon-Fri at 10h each, `start 08:00` today. Status: `Day total: 00h 00m (+ ongoing), 19h 01m over 40h 00m required today, target already met`; `Week: 10h 00m ahead of target (fulfillment 50h 00m / target 40h 00m)`; stint `08:00-now (09h 01m, ongoing)`. `week`: `10h 00m ahead of target`, `Sat 00h 00m (ongoing)`, `Worked: 50h 00m`.
4. Two starts (09:00, 13:00). Status: `Day total: 00h 00m (+ ongoing), 35h 59m left ..., est. EOD 05:00 (tomorrow)`, plus a `[!] 2 open stints` block. Fixes offered: delete a punch, or `stop HH:MM (a time after 13:00 closes the 13:00 start)`. Both stints listed `-now` (`08h 01m` and `04h 01m`). `week` marked Sat `[!]`.
   - `stop 12:00` then worked silently. It paired 09:00-12:00 and left 13:00 ongoing.
5. Future start `start 23:30` (now ~17:00) was accepted silently, exit 0. Status: `Day total: 00h 00m (+ ongoing), 40h 00m left ..., est. EOD 15:30 (tomorrow)`, stint `23:30-now (00h 00m, ongoing)`. After `stop 23:45` it showed a normal 15m stint. `status 2026-10-05` (future date) printed `Week 2026-41: Total behind: 80h 00m`.
   - A `stop 19:00` with `start 20:00` gave an orphaned-end `[!]` with sensible fixes, and the 20:00 start stayed open.
6. Compared against the README below.

## Findings

F1. Scenarios 1, 2, 4, 5. "(tomorrow)" on est. EOD is wrong.
- Expected: a date or weekday that matches the arithmetic.
- Got: est. EOD is the open start plus the remaining time, so 14:00 plus 40h reads `06:00 (tomorrow)`. That is really Monday morning, two days later. The same happens with 01:00, 02:00, 05:00, 12:00 and 15:30.
- Why it confuses: any estimate over 24 h away is labelled "tomorrow". Nothing says it is days away.

F2. Scenarios 1, 2, 4, 5. On Saturday the "required today" figure is the entire week's target.
- Expected: "when can I call it a day" to give an hour today.
- Got: a result like `37h 02m left to 40h 00m required today, est. EOD 06:00 (tomorrow)` with nothing worked this week.
- Why it confuses: Saturday is treated as the deadline for the whole target, so the answer is technically consistent but useless. A person who owes 40h and works ~8h a day reads it as "keep working until Monday". The README never says the week's deadline is Saturday, or why. Its examples all happen to be Saturdays.

F3. Scenarios 1, 3, 4. `Day total: 00h 00m (+ ongoing)` sits next to a stint line showing hours.
- Expected: the day total to include running time, or the "(+ ongoing)" to say how much.
- Got: `00h 00m (+ ongoing)` with `14:00-now (02h 58m)` below it, and `00h 00m (+ ongoing)` beside `19h 01m over` in scenario 3. The week line and `week` ignore the running stint too (`Fulfillment 00h 00m`, `Sat 00h 00m (ongoing)`).
- Why it confuses: the numbers on one screen disagree. README Known issues partly covers this, but nothing in the tool's own output explains it.

F4. Scenarios 2, 3. The pace hint and the week line disagree in direction.
- Got: `00h 01m over 40h 00m required today, target already met` beside `Week: 04h 00m left by end of Saturday`.
- Why it confuses: a user reads "target met" and "4h left" at once. It is documented as a known issue. It is still the first thing seen after a lunch break near the end of a week.

F5. Scenario 3. `19h 01m over 40h 00m required today`.
- Expected: surplus relative to week-to-date.
- Got: the week line says 10h ahead, and the hint says 19h over. Day total is 0.
- Why it confuses: three different figures (0h, 10h, 19h) describe "how far ahead am I". The wording "over 40h 00m required today" is hard to parse.

F6. Scenario 5. A future start time is accepted with no warning and exit 0.
- Got: `23:30-now (00h 00m, ongoing)` and `est. EOD 15:30 (tomorrow)`. The stint is shown as running when it has not begun.
- Why it confuses: a typo is not flagged. The estimate is meaningless. `start`'s help says dates must not be in the future, but says nothing about times.

F7. Scenario 4. The offered fix is narrow.
- Got: the `or:` fix reads "a time after 13:00 closes the 13:00 start". The natural fix for a forgotten stop (a time between 09:00 and 13:00) is not mentioned. It works (`stop 12:00`) but I had to guess.
- Also: both stints show `-now`, `08h 01m` and `04h 01m`, which looks like double counting. The pace hint counted only the 13:00 one (35h 59m left). The README does not say which one counts.

F8. Scenario 5. `status` for a future week (`2026-10-05`) says `Week 2026-41: Total behind: 80h 00m` for a week that has not started. It was accepted silently. This is partly in Known issues, but "behind" on a future week reads as a real debt.

F9. Scenario 1. The first run on a new DB prints `[... INFO rusqlite_migration] Database migrated to version 1` with a raw UTC timestamp. The README says `start` is silent on success. A new user sees log noise and a UTC time that differs from local time (14:58Z vs 16:58). It happens only once.

F10. Week view, scenarios 1-3. `week` for the current week shows `Sat ... 00h 00m (ongoing)` even with 9 hours running, and `Worked` excludes it. The README says it stays frozen until `stop`, so it matches the README but still looks wrong.

F11. README. The Known issues list is long and several entries describe confusing live behaviour (pace hint, est. EOD, frozen week line). The Features and `status` sections do not point to it. The `status` section says est. EOD "holds still until you add or remove a punch", which is accurate. It does not say that est. EOD can be days away or that it stays put while the day total and week line do not move.

## Verdict
findings(11)
