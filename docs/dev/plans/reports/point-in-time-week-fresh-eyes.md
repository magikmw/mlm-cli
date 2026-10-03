# point-in-time-week fresh-eyes report

The system clock read Sat 2026-10-03 18:27 CEST (week 2026-40, Mon 09-28 to Sun 10-04). I could not reach a Monday-to-Thursday state, so the `by end of <Weekday>` headline form never appeared in my runs. Every current-week screen used the Friday-on form `left to 40h 00m target`.

Databases used: s1 (heavy week), s2 (light week), s3 (owed), s4 (ahead), all inside the scratch dir.

## Scenarios — what you ran, what it printed

1. Mid-week check-in (s1: Mon-Fri backdated with `-d -5..-1`, then `start 09:00 "morning"` today, still clocked in)
   - `status`: `Day total: 09h 27m (incl. ongoing), target already met` and `Week 2026-40: 06h 27m over 40h 00m target (fulfillment 46h 27m / target 40h 00m)`.
   - `week`: headline `06h 27m over 40h 00m target`, Sat row `09h 27m (incl. ongoing)`, Worked 46h 27m.
   - Light week (s2: Mon 6h, Tue 5h, Thu 3h, then `start 17:00` today): `Day total: 01h 27m (incl. ongoing), est. EOD 19:00 (tomorrow)` and `Week: 24h 33m left to 40h 00m target (fulfillment 15h 27m / target 40h 00m)`. `week` headline `24h 33m left to 40h 00m target`.
2. Clock out (s2, `stop 18:20`)
   - `status`: `Day total: 01h 20m` with no est. EOD or "left today" figure. Week line now `24h 40m left to 40h 00m target (fulfillment 15h 20m / target 40h 00m)`.
   - `week`: headline `24h 40m left to 40h 00m target`. The Sat row lost "(incl. ongoing)".
3. `status -5` (Mon) while clocked in today (s1) and after stop (s2)
   - Day total `08h 30m` (s1), week line `06h 27m over 40h 00m target`, identical to today's status. `week` showed the same headline and Mon `08h 30m`.
   - In s2 after stop, `status -5` printed Day total `06h 00m` and week line `24h 40m left to 40h 00m target`.
4. Carry-in
   - Owed (s3: week 39 Mon 8h, Tue 4h; week 40 Thu 8h, Sat 09:00-now): week 39 reads `Total behind: 28h 00m`. Today's status reads `50h 33m left to 40h 00m target (fulfillment -10h 33m = worked 17h 27m + carry-in -28h 00m / target 40h 00m)`, est. EOD `21:00 (Mon)`. `week` Carry-in `-28h 00m`, Worked `17h 27m`, Fulfillment `-10h 33m`.
   - Ahead (s4: week 39 Mon-Fri 10h/day): week 39 reads `Total ahead: 10h 00m`. Today shows `20h 33m left to 40h 00m target (fulfillment 19h 27m = worked 09h 27m + carry-in 10h 00m / target 40h 00m)`, est. EOD `15:00 (tomorrow)`.
   - The arithmetic adds up in both cases (40h - fulfillment = the "left" figure).
5. Custom target (s2, `week target 33h30m`)
   - Silent. Status then shows `18h 10m left to 33h 30m target (fulfillment 15h 20m / target 33h 30m)`. `week` headline `18h 10m left to 33h 30m target`, Target `33h 30m`.
   - `week target 33` gives `error: invalid duration "33": expected a form like 20h, 33h30m, or 45m` (exit 1).
   - `week target 0h` is accepted silently (exit 0). Status then reads `15h 20m over 00h 00m target (fulfillment 15h 20m / target 00h 00m)`.
   - `week 2026-41 target 20h` gives `error: the subcommand 'target' cannot be used with '[WEEK_ID]'` plus the usage of plain `week`.
6. Other weeks
   - Last week (s2, no data): `Week 2026-39 ... Total behind: 40h 00m`, all rows 00h 00m.
   - Next week (s2, after target 33h30m): `Total behind: 58h 10m`, Carry-in `-18h 10m`.
   - Week 2026-45: `Total behind: 218h 10m`, Carry-in `-178h 10m`. Week 2026-53: `Total behind: 504h 40m`, Carry-in `-464h 40m`. Week 2025-52 (a year ago, no data): `Total behind: 40h 00m`.
   - `status 2026-10-10` (future): `Week 2026-41: Total behind: 58h 10m`. `status 2026-09-20`: `Week 2026-38: Total behind: 40h 00m`.

## Findings — each with the scenario, what you expected, what you got, why it would confuse someone

1. Scenario 1/2. Nothing answers "how much do I still need to do today". The README sells `status` as a "how much is left today" estimate. On Friday through Sunday the only figure is the whole week's remainder (`24h 33m left to 40h 00m target`). After `stop` there is no day-level number at all, and while clocked in the only day-level hint is `est. EOD`. The `left` figure reads as today's but is the week's. A person who works 8h days gets no "3h more today".
2. Scenario 1. `Day total: 09h 27m (incl. ongoing), target already met` sits on the day line. I read it as "today's target is met", but it means the week's required figure is met. The README's Possible issues lists this, but the screen carries no hint.
3. Scenario 1/4. `est. EOD` lands at odd, unusable times. `21:00 (Mon)` on Saturday evening (s3) and `19:00 (tomorrow)` (s2) assume I work nonstop through the night and next day. They are arithmetic, not a plan, and the page does not say so. It also does not say that a `stop` for lunch invalidates it. Both stay frozen while the clock moves.
4. Scenario 2. Stopping can raise the "left" number. s2 went from `24h 33m left` to `24h 40m left` because I stopped at 18:20, seven minutes before "now". Explainable, but with no explanation on screen it looks like a bug right after clocking out.
5. Scenario 3. `status <earlier date>` while clocked in prints a Day total that is fixed (`08h 30m`) next to a week line that includes today's running minutes. The week line does not change by date, so a per-date page cannot show "where I stood that day". The README mentions this only under Possible issues, and `--help` says nothing. Also, `status -5` in s1 shows `over 40h 00m target` under a Mon day total, which reads as if Monday alone put me over.
6. Scenario 6. Empty past and future weeks print `Total behind: 40h 00m`, and future weeks keep compounding: week 41 is `58h 10m`, week 45 `218h 10m`, week 53 `504h 40m`, with Carry-in `-464h 40m`. A week a year ago, before I used the tool, reads `Total behind: 40h 00m`. Nothing says these weeks haven't started or have no data. The README lists the future-week case as a rough edge, but a person reading a future week for planning gets absurd numbers.
7. Scenario 6. The week headline says `Total behind` / `Total ahead` for closed weeks but `left to` / `over` for the current one. Next week's `Total behind: 58h 10m` is not "this much to do" in any obvious sense, since 18h 10m of it is carried-in debt. The Carry-in row explains this, but only a reader who checks it.
8. Scenario 5. `week target 0h` is accepted silently, and the screen then reads `over 00h 00m target`. Also `week target 33h30m` for the current week is silent, so only `status` or `week` confirms it. The README says this is by design, but a typo like `week target 3h` also lands silently.
9. Scenario 5. `week 2026-41 target 20h` (the wrong order, which I tried first by instinct from `week 2026-41`) errors with clap's `the subcommand 'target' cannot be used with '[WEEK_ID]'` and then prints the usage for `week`, not `week target`. The usage never shows the order `week target [WEEK_ID] DURATION`. `mlm week target --help` shows only `<ARGS>...` with the real shape hidden in the description.
10. First run. A new database prints `[2026-10-03T16:27:14Z INFO rusqlite_migration] Database migrated to version 1` to stderr in UTC, which is 18:27 local. The README says commands are silent on success. The Possible issues list mentions this, but the Usage section contradicts it.
11. Docs vs tool. The `--help` pages never explain `fulfillment`, `est. EOD`, `incl. ongoing` or the `by end of Thursday` rule. All of it lives only in the long README `status` section. `mlm status --help` is one line. A person who never reads the README cannot decode the week line.
12. Docs vs tool. The README carries a long "Possible issues" list of shipped confusing behavior (about 25 items) inside the main README. Several of them are exactly what I hit (findings 2, 5, 6, 10). A user reading it learns the tool is known to be rough, but cannot tell which items matter for daily use.
13. Scenario 4. The expanded parenthetical `fulfillment -10h 33m = worked 17h 27m + carry-in -28h 00m / target 40h 00m` is accurate but long, and the `/` reads like division. `status` for an owed week says `50h 33m left to 40h 00m target`, which exceeds the target itself. It adds up (40h + 10h 33m), but the sentence reads oddly without the parenthetical.
14. Limitation. I could not exercise the Monday to Thursday form because the system date is Saturday. No scenario showed `by end of <Weekday>`, so the README's claims about it are unchecked.

## Verdict — clear | findings(N) | blocked

findings(14)
