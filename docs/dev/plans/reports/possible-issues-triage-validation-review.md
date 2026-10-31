# Possible-issues triage validation

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

Branch `possible-issues-triage` (baseline `main` @ be85617). Date of run Sat 2026-10-03, local clock about 22:40 CEST.

The dispatch named `possible-issues-triage-validation.md`. A write hook refused that name (it only allows `*-review.md` or `*-consistency.md`), so this file carries the `-review` suffix.

## Method and limits

- The sealed sandbox cannot see the repo, so I built with `cargo build --target-dir /tmp/cd-scratch/pit/target` and ran that binary against fresh DBs under `/tmp/cd-scratch/pit/` (not the `/home/magikmw/.claude/jobs/...` path, which `mkdir` could not create).
- The real clock was Saturday, so Monday-Thursday behavior (`by end of <weekday>` form, `required` below target) is verified from source and the unit tests that already pin it, not by running.
- Exact `gap == 0` and midnight rollover were not run live. Source only.
- Count mismatch. SPEC §1.2a has **34** bullets (entry 96 and the dispatch say 32). README has **33**. IDs below follow SPEC order, I1..I34.

## Summary table

| id | short name | verdict | entry 96 tier |
|---|---|---|---|
| I1 | unclosed start > 14 days old leaves today's view | valid (see F7) | 3 (14-day window) |
| I2 | LIFO nested entry double-counts, no anomaly | valid | 1(b) |
| I3 | lone open start dated yesterday never flagged | valid | 1(a) |
| I4 | past-week anomaly skews later carry-in, no pointer | partly valid | deferred (kept) |
| I5 | `status DATE` accepts future date, weekday names "now" | partly valid | 2 |
| I6 | zero boundary: `left` (current) vs `Total ahead` (closed) | valid | 3 (framing wording) |
| I7 | two `status` runs differ (`left to` to `over`) | valid | 3 |
| I8 | week-line jumps come from backdated punches | valid | 3 |
| I9 | `est. EOD` meaning, `target already met`, break shifts it | valid | 3 |
| I10 | `est. EOD` fixed per stint, day suffix from first status | valid | 3 |
| I11 | start typed later than now projects `est. EOD` from it | valid | 3 |
| I12 | 2+ open starts, week line credits only latest | valid | 3 |
| I13 | `fix:`/`or:` with 2 open starts suggest only later start | valid | not named |
| I14 | start from yesterday still open after midnight | valid | 1(a) |
| I15 | from Friday `required` = full target, `est. EOD` days out | valid | 3 |
| I16 | week-line form change Thursday to Friday | valid | 3 |
| I17 | Mon-Thu `X over 32h by end of Thursday` | valid | 3 |
| I18 | whole-week remainder not printed for current week | valid | 3 |
| I19 | Day total / `mlm week` change minute by minute | valid | 3 |
| I20 | earlier date in current week, week line has live minutes | valid | 3 (earlier-date week line) |
| I21 | `(incl. ongoing)` vs `(+ ongoing)` | valid | 3 |
| I22 | future-typed start shows `(incl. ongoing)`, nothing added | valid | 3 |
| I23 | `est. EOD` placement, `target already met` scope, `gap == 0` | valid | 2 (reword) |
| I24 | future week `Carry-in` moves minute by minute | valid | 3 |
| I25 | zero-target week wording | valid | not named |
| I26 | non-multiple-of-5 target override, `required` up to 4 min high | valid | not named |
| I27 | no due-today figure on Day total after `stop` | valid | not named |
| I28 | storage read failure in `mlm week` shown as no data | partly valid | 1(c) |
| I29 | `status DATE` future week reads `Total behind` | valid | 2 (via future rejection) |
| I30 | debt week reads more left than target | valid | 3 (debt-week left) |
| I31 | `mlm week target` silent on success | valid | 1(d) |
| I32 | `week WEEK_ID target DURATION` shows plain `week` usage | valid | 2 |
| I33 | `--help` defines no glossary terms | valid | 2 |
| I34 | first-run migration log line on stderr | valid | 2 |

Verdict counts. 31 valid, 3 partly valid (I4, I5, I28), 0 stale/fixed, 0 wrong.

## Findings (each non-valid item or broken entry-96 claim)

- F1 I4 partly valid. Today's `status` does point back to an unclosed start for 14 days.
- F2 I5 partly valid. SPEC §6.1 does document the behavior, and the weekday phrase is only wrong for a few dates.
- F3 I28 partly valid. The main read errors out loudly. Only the target-override and carry-walk reads are swallowed.
- F4 Entry 96 1(c) "fixed by propagating the error" is not a small change. The accounting traits are infallible.
- F5 Entry 96 says rejecting future `status DATE` retires the future-week `Total behind` item. It does not. `mlm week 2026-45` prints the same line.
- F6 Entry 96 says "32 items". SPEC has 34.
- F7 Tier 3 I1 (14-day window) produces a wrong carry-in with no signal. It should not be accepted without I4's pointer.
- F8 README and SPEC disagree. README merges I9/I10 and words I26 differently.
- F9 Entry 96 1(d) (echo the target) contradicts SPEC §7.4, which says `week target` prints nothing. The spec and tests must change with it.

## Per-item detail

### I1 14-day window. valid

Ran on a fresh DB with unclosed starts dated -10, -16 and -20 days.
- Today's `status` and `mlm start` warn only for the -10 start ("unclosed start on Wed 2026-09-23 at 09:00 (10 days ago)").
- `mlm status -16` shows the `[!] unclosed start ... 16 days old` block, and `mlm week 2026-38` shows `[!]` on Thu 2026-09-17. Both match the bullet.
- Code: `src/status.rs:441` walks ages 2 through 14 only.
- Side effect seen in the same run. Today's `status` showed `Carry-in -120h 00m` and no `[!]` for the 16- and 20-day-old starts. See F7.

### I2 LIFO nested entry. valid

Today: `start 09:00`, `start 10:00`, `stop 11:00`, `stop 12:00`, then `status`.
```
Day total:     04h 00m
Week 2026-40:  36h 00m left to 40h 00m target (fulfillment 04h 00m / ...)
  09:00-12:00  (03h 00m)
  10:00-11:00  (01h 00m)
```
No `[!]`, and `mlm week` also counts 04h 00m. The two stint lines overlap by an hour. The first `start 10:00` was accepted with rc=0.

### I3 yesterday's lone open start. valid

`start 20:00 --date -1`, then today's `status` printed no hint at all (week line `40h 00m left`). `mlm start 21:00` also printed nothing. `mlm status -1` showed `20:00-now (26h 39m, ongoing - ...)`.

### I4 past-week anomaly skews later carry. partly valid

- The skew is real. With an unclosed start in week 2026-39, the week 2026-40 line read `fulfillment -120h 00m = worked 00h 00m + carry-in -120h 00m` and nothing named week 39.
- Change from the bullet's wording. The pointer is not entirely absent. For an anomaly inside the last 14 days, today's `status` prints the lookback `[!] unclosed start on ...` block, and `mlm week PAST` marks the row `[!]`. What is missing is any pointer on the carry-in figure itself, and any pointer at all past 14 days (F7).
- Suggested rewording. "No `[!]` ties the carry-in figure to its source week, and nothing points at it once the anomaly is older than 14 days."

### I5 future `status DATE`. partly valid

- Accepts a future date. `mlm status 2026-10-10` and `2026-11-10` rc=0, printing `Week 2026-41:  Total behind: 40h 00m`. `+1` is rejected as a bad DATE (only `-N` shorthand exists).
- Wrong word. The bullet says "undocumented". SPEC §6.1 (`docs/dev/SPEC.md:653-654`) says `status`'s `DATE` argument "has no such restriction". The behavior is documented, the oddity is not called out.
- The weekday-phrase claim is narrower than written. `render::current_week_headline` (`src/render.rs:59-78`) only prints `by end of <weekday>` for the current week, Monday to Thursday. A future date in a later week gets the Closed framing (`Total behind`), with no weekday. So the stale weekday only appears for a future date inside the current week on a Mon-Thu run. On Saturday, `status 2026-10-04` printed `20h 00m left to 20h 00m target (...)` with no weekday phrase. Mon-Thu part verified from source only.

### I6 zero boundary. valid

`src/render.rs:62` uses `gap >= 0` for `left to`. `src/render.rs:92-95` uses `owed > 0` for `Total behind` and falls to `Total ahead` at zero. The unit test `t8_zero_owed_on_a_closed_week_is_total_ahead` pins it. Exact zero was not reproduced live.

### I7 two runs differ. valid

`status.rs:581-584` makes `est_eod` return `TargetAlreadyMet` when the live-inclusive gap is `<= 0`. Observed both forms with target 5h. A 17:00 start showed `05h 39m (incl. ongoing), target already met` and `00h 39m over 05h 00m target`. A 19:00 start with a 1h earlier stint showed `00h 21m left ... est. EOD 23:00`.

### I8 jumps from backdated punches. valid

Target 5h, stint 17:00-18:00 plus open 19:00 showed `00h 21m left` (live 03h 39m counted). After `stop 20:00` the line read `03h 00m left` (fulfillment 04h 39m to 02h 00m). Plain `stop` at now cannot jump because live minutes already count. The backdated-start half follows from `credit_live` (`status.rs:484-494`), not run.

### I9 `est. EOD` meaning. valid

- `src/status.rs:581-590`. `est_eod` returns `TargetAlreadyMet` once the gap is covered, and otherwise the open start plus the gap without live.
- Break shift confirmed. A 17:00-18:00 stint, then a 19:00 start, showed `est. EOD 23:00` (19:00 + 4h).
- After a `stop`, no clock time appears.

### I10 `est. EOD` fixed per stint, day suffix. valid

`status.rs:581-590` computes `open.start + base_gap`, independent of the current time. Observed `est. EOD 13:00 (Mon)` on the first `status` after `mlm start 21:00` and `est. EOD 15:50 (Mon)` for a 23:50 start. The `(Mon)` suffix is there from the first run.

### I11 start later than now. valid

`start 23:50` at 22:41 gave `Day total: 00h 00m (incl. ongoing), est. EOD 15:50 (Mon)` with `40h 00m left` on the week line. 15:50 Monday is 23:50 + 40h.

### I12 two open starts credit only the latest. valid

Starts 09:00 and 10:00. The stint list showed `13h 39m` and `12h 39m`. Day total and week fulfillment were `12h 39m`, so only the 10:00 start counted. Pinned by `stint.rs:480` `live_minutes_uses_only_the_most_recent_of_two_open_starts`.

### I13 `fix:`/`or:` lines. valid

Output named `a time after 10:00 closes the 10:00 start` only. Then `mlm stop 09:30` was accepted and paired with the 09:00 start, leaving 10:00 open (`09:00-09:30 (00h 30m)` plus `10:00-now`). Entry 96 does not triage this. It is a cheap wording fix if anyone wants it.

### I14 yesterday's start open after midnight. valid

- With `start 20:00 --date -1` and nothing else, today's page showed `00h 00m`, no ongoing stint and `40h 00m left`, so no live credit and no `est. EOD`.
- `live_minutes_today` (`status.rs:499-510`) and `DbWeekData::worked_minutes` (`week_view.rs:268-310`) both credit live minutes only for the date equal to `today`. Past-Sunday Worked drop and the 00:00 jump follow from source (the `required` step is `required_minutes`, `week.rs:240`). Not run across midnight.

### I15 Friday on. valid

`src/week.rs:240-246`. `required_minutes` returns the whole target for weekday number >= 5. Saturday run showed `est. EOD 13:00 (Mon)` and `... left to 40h 00m target`.

### I16 form change Thursday to Friday. valid

`src/render.rs:68-77`. `number_from_monday() >= 5` selects `<amount> <word> <required> target` instead of `by end of <weekday>`. Saturday run confirmed the `target` form. Thursday form from source and `pit_*` tests.

### I17 Mon-Thu `over` wording. valid

`src/render.rs:59-77` has `over` and no "ahead of target" anywhere in the current-week path. Grep for `ahead of target` finds only test negative assertions (`src/status.rs:1662`, `src/week_view.rs:1342`).

### I18 whole-week remainder. valid

The current-week headline prints `required - fulfillment` only. `mlm week` prints `Target: 40h 00m` in the footer. Both facts from source and observed output.

### I19 minute by minute. valid

Observed `Carry-in 02h 41m` on week 2026-41 at 22:41 for a 20:00 start (see I24), and the Day total with `incl. ongoing` moves with the clock.

### I20 earlier date, current week. valid

`mlm status -1` (Fri) with today's 21:00 start open showed `Day total: 00h 00m (+ ongoing)` and `fulfillment 01h 39m`. The week line counted today's live minutes. `resolve` passes `live` to `build_ledger` regardless of `is_today` (`status.rs`, `resolve`).

### I21 `(incl. ongoing)` vs `(+ ongoing)`. valid

Both strings observed (today `(incl. ongoing)`, yesterday `(+ ongoing)`).

### I22 future-typed start. valid

`start 23:50` at 22:41 showed `00h 00m (incl. ongoing)` and `23:50-now (00h 00m, ongoing)`. `live_minutes_clamps_a_future_start_at_zero` (`stint.rs:486`).

### I23 `est. EOD` placement and `gap == 0`. valid

`render::current_week_headline` uses `gap >= 0` for `left to` and `est_eod` uses `gap_minutes <= 0` for `target already met`, so they disagree at exactly zero (`render.rs:62` vs `status.rs:582`). Not run live (needs an exact minute). `target already met` scope for Mon-Thu follows from `required_and_gap`.

### I24 future week `Carry-in`. valid

With a zero-target current week and a 20:00 start, `mlm week 2026-41` showed `Carry-in: 02h 41m` at 22:41. `credit_live` adds live to the current week whenever `through >= current` (`status.rs:484-494`).

### I25 zero-target week. valid

Target `0h`, Saturday, open 20:00 start. `status` printed `02h 39m over 00h 00m target` and idle printed `00h 00m left to 00h 00m target`. The `by end of <weekday>` form for Mon-Thu follows from `render.rs:70-76`.

### I26 non-multiple of 5. valid

`required_minutes` (`week.rs:240-246`) returns the exact target Fri-Sun and `floor(t/5)*n` Mon-Thu. For 2011 min, `required` 2011 vs `5 * 402 = 2010`. Pinned by `resolve_f9b_*` tests. SPEC and README word it differently (F8).

### I27 after `stop`. valid

After `stop 18:00` the Day total line read `01h 00m` with no `est. EOD`, and the week line read `04h 00m left to 05h 00m target`.

### I28 storage read failure. partly valid

What I ran. In a DB with a start punch:
- `alter table week_targets rename to wt_bak`. `mlm week` printed `37h 20m left to 40h 00m target` (the stored 20h target silently replaced by the 40h default, rc=0). `mlm status` printed `error: no such table: week_targets`, rc=1.
- Renaming `punches` instead. `mlm week` printed `error: database error: no such table: punches`, rc=1.

What that means.
- The main read (`punches_in_range(...)?` at `src/week_view.rs:333`) propagates, so the bullet is not accurate for the rows. A failure on that query already aborts.
- The swallowed reads are in `DbWeekData` (`week_view.rs:268-318`). `earliest_data_week` and `target_override` use `.ok().flatten()` (lines 273-274, 316-317), and `worked_minutes` uses `unwrap_or_default()` at line 283. These feed the headline, `Worked`, `Fulfillment`, `Carry-in` and `Target`.
- So "shows as no data" is true for the summary and carry walk but not for the 7 rows. "Hides live minutes from Worked, Fulfillment and the headline" needs a failure on only the second punches query, which is a narrow window (transient lock or I/O error). I could not reproduce that case, only the target one.
- `status` has no such hole. `status::build_ledger` (`status.rs:517-575`) uses `?` throughout.
- The doc comment on `DbWeekData` (`week_view.rs`, just above the struct) records the choice as intentional ("not expected to fail").

### I29 future-week `Total behind`. valid

`mlm status 2026-10-10` printed `Week 2026-41:  Total behind: 40h 00m`. `mlm week 2026-45` and `mlm week 2027-01` print the same `Total behind: 40h 00m` (see F5).

### I30 debt week. valid

DB with a zero-minute punch pair in week 39 (so carry-in -40h) and a 9h day in week 40.
```
Week 2026-40:  71h 00m left to 40h 00m target (fulfillment -31h 00m = worked 09h 00m + carry-in -40h 00m / target 40h 00m)
```
The left figure exceeds the target and the parenthetical explains why.

### I31 `week target` silent. valid

`mlm week target 20h` printed nothing, rc=0. `SPEC.md:1081` §7.4 states this as the contract.

### I32 `week WEEK_ID target`. valid

```
error: the subcommand 'target' cannot be used with '[WEEK_ID]'

Usage: mlm week [OPTIONS] [WEEK_ID]
       mlm week <COMMAND>
```
rc=2. The first line already names the conflict, which the bullet does not say. `mlm week target --help` lists only `<ARGS>...` with the shape in the description (`src/cli.rs:202-211`). The bullet's claim holds.

### I33 `--help` has no glossary. valid

`mlm status --help` and `mlm week --help` describe only the arguments. No definition of fulfillment, `est. EOD`, `incl. ongoing`, or the by-end-of rule.

### I34 first-run log line. valid

First run printed `[2026-10-03T20:39:10Z INFO  rusqlite_migration] Database migrated to version 1`. Second run on the same DB printed nothing.

## README vs SPEC §1.2a

| Point | SPEC | README | Note |
|---|---|---|---|
| Count | 34 bullets | 33 bullets | README merges I9 and I10 into one bullet |
| I9/I10 | two bullets, with I10's `06:20 (Mon)` example | one bullet, example `06:00 (Mon)` | README omits "A break moves it later by the break's length" |
| I8 | backdated `stop` raises the figure, backdated `start` lowers it | only "come from backdated punches, not from stop" | README less detailed |
| I26 | "`required` is the exact target ... up to 4 minutes above five times the daily target" | "`est. EOD` can land up to 4 minutes later than five times the daily target would put it" | different claim, README speaks about `est. EOD`, SPEC about `required` |
| I5 | says "undocumented" | says "silently accepts" | SPEC's "undocumented" conflicts with §6.1 (I5 above) |
| I33 | "Only this spec and the README" | "Only this README and the spec" | cosmetic |
| I34 | gives full example line | abbreviates with `...` | cosmetic |

Every other README bullet has a SPEC counterpart with the same meaning. No README-only bullet exists.

## Entry 96 claim check

| Claim | Holds | Evidence |
|---|---|---|
| `unwrap_or_default` hides storage errors, `src/week_view.rs` ~283 | partly | line 283 is `worked_minutes`. `.ok().flatten()` at 273 and 316 hides two more reads. The rows' main query propagates (333). See I28. |
| default log level `Info`, `src/main.rs` ~22 | yes | `src/main.rs:20-22`. Debug if `--verbose`, else Info. |
| `mlm week target` silent on success | yes | rc=0, no output. |
| `status DATE` accepts future dates | yes | rc=0 for 2026-10-10, 2026-11-10. |
| "32 items" | no | 34 in SPEC (F6) |

### Tier 1

- (a) Hint line for yesterday's open start, 2-day stale rule kept. Makes sense. I3 and I14 reproduce, and `mlm start` stays silent today. The hint must also cover today's `start`, which accepted a new start with yesterday's still open. Today's week line ignores yesterday's open minutes (I14), so the hint text should say those minutes are not counted yet.
- (b) Overlap anomaly, LIFO unchanged. Makes sense. I2 reproduces. The stale-open-start `fix:` line ("a time after 09:00") produces this shape, so the anomaly wording should be reachable from it.
- (c) Propagate the storage error. The goal is right but the fix is bigger than the text suggests (F4). `WeekData::worked_minutes` and `WeekTargets::target_override` are infallible by trait design (`src/week.rs:35-55`). Cleanest fix is to build a ledger up front with fallible reads, as `status::build_ledger` already does (`status.rs:517-575`), and share it with `week_view`. That also removes duplicated logic and the three `.ok()` or `unwrap_or_default` sites. Reachability is low, so this is hardening and not a wrong-number fix today.
- (d) Echo the stored target. Makes sense. It contradicts SPEC §7.4 (`docs/dev/SPEC.md:1081`, "`week target` print nothing on success"), the §3.7 text and README (F9). Existing tests that assert silence will need to change. §7.4 already has precedent for narrow exceptions (`delete`, `start`'s stderr warning).

### Tier 2

- Default log `Warn`. Safe. `grep` finds no `log::info!` or `log::warn!` anywhere in `src/` (only `log::debug!`). The only Info line users see is the `rusqlite_migration` one. The stale-start warning is a plain stderr write, not a log line.
- Glossary in `status --help`. Valid (I33). `week --help` has the same gap.
- Targeted error for `week WEEK_ID target`. Valid. The current clap error already names the conflict, so the gain is showing the right order. Quick to do with a custom message.
- Reword `target already met` on Day total. Valid (I23).
- Reject future `status DATE`. Retires I5's weekday issue, since that issue only exists for a future date inside the current week. It does not retire the future-week `Total behind` line (F5). `mlm week 2026-45` reaches the same Closed framing. It also needs §6.1 line 653-654 and §3.5 rewritten, and removes a way to preview the rest of the current week.

### Tier 3

All Tier 3 items hold as plain live-model behavior except as noted.

- I1 (14-day window) produces a wrong number. See F7.
- I30 (debt-week left, `71h 00m left to 40h 00m target`) is arithmetically correct and the parenthetical shows the carry. I would not flag it as wrong.
- I12 (two open starts) understates against the stint list, but E7 already prints `[!]`. Acceptable.
- I29 `Total behind: 40h 00m` for an unstarted week is the one figure that reads as debt before any time passed. It sits in Tier 2 only through `status`, and `mlm week` still shows it (F5).

## Finding detail

- F1 (I4). Reword to separate "no pointer on the carry-in" from "no pointer at all". Today's `status` already prints the lookback block for 14 days, and `mlm week PAST` marks the row.
- F2 (I5). Fix the "undocumented" word (§6.1 documents it). Narrow the weekday claim to a future date in the current week, Monday to Thursday.
- F3 (I28). Reword. The rows and the main read propagate. The silent paths are the target-override read, the earliest-week read, and the carry-walk punches read. Add the one-line repro (rename `week_targets`).
- F4 (entry 96 1(c)). Plan a ledger prefetch, not a one-line `?`.
- F5 (entry 96 Tier 2). `mlm week FUTURE_WEEK` still prints `Total behind: <target>`. Either decide that line is fine, or add a separate item and fix (for example a "starts <date>" form for a future week).
- F6 (entry 96). Count is 34 in SPEC, 33 in README. The "32" number should be corrected or explained (README merges I9/I10).
- F7 (Tier 3 I1). A start left open more than 14 days ago drops out of every `status`/`start` warning and still depresses every later carry-in. The run showed `Carry-in -120h 00m` on today's week line with no marker. Do not accept this one together with the live-model items. Either keep it beside I4 in §1.2a, or extend the lookback.
- F8 (README/SPEC). Align the I26 wording, restore the break-shift sentence in README, and fix the I5 word. If README trims to open items this resolves mostly by itself.
- F9 (entry 96 1(d)). Update §7.4, §3.7, README and the silence tests together with the echo change.
