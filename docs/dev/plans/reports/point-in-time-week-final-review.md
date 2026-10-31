# point-in-time-week final review

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

- **Scope**: `git diff efd97e4..point-in-time-week` (code in `src/`, SPEC.md, README.md, NOTES.md)
- **Spec**: `docs/dev/specs/2026-10-03-point-in-time-week.md` (locked)
- **Plan**: `docs/dev/plans/point-in-time-week-plan.md` (with review folds)
- **Reviewed**: 2026-10-03, real clock Saturday 18:39 CEST
- **Verdict**: ship-with-followups, 3 findings, all doc or code-quality, none behavioral

## Gate

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo build` | clean |
| `cargo test` | 550 lib + 10 integration/doc, all pass |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `clippy::cognitive_complexity` at threshold 15 (`clippy.toml`) | clean, forced re-check after touching `src/lib.rs` |

## Correctness

The code matches the spec on every rule I checked.

- `week::required_minutes` gives `floor(target/5) × weekday` Mon–Thu and `target` Fri–Sun (spec §2.2).
- `render::current_week_headline` builds the §3.1 table exactly: `left to` at `gap >= 0`, `over` below, the weekday Mon–Thu, `<target> target` Fri–Sun. It always uses today's weekday.
- `status::live_minutes_today` classifies today's punches with both neighbours, so a splice with tomorrow's orphan end applies. `resolve` computes it for every `DATE`, as spec §2.1 requires.
- `status::credit_live` adds live to the current week before the carry walk, and only when the walk reaches it. Past weeks get none, and future weeks inherit it through carry (spec §2.3).
- `DbWeekData::worked_minutes` and `week_view::build_rows` both add `live_minutes()` only on `date == today`. That gives one source for the rows and `Worked:`, so the rows sum to `Worked:` (plan fold F6).
- The day total adds live only when `DATE` is today. The `(incl. ongoing)` / `(+ ongoing)` / `(+ unclosed)` flag follows `OpenStintAge` (spec §3.2).
- `est. EOD` is `open start + (gap + live)`, which is the gap without live. It stays fixed while the stint runs (spec §3.2).
- The pace clause, `ahead of target`, `left by end of` and `required today` are gone from the code. The removed `StatusView::weekday_name` and `DailyTargetHint::day_reaches_week_cap` leave no callers behind.

No behavioral defect found.

## Hand runs

All runs used `MLM_DB_PATH` under `/tmp` (`/tmp/mlm-pit-review*.db`, `/tmp/mlm-seed-pit.db`). I rebuilt the §3.4 week on the real Saturday clock. Week 39 has 5 × 9h 23m, which gives carry-in +06h 55m. Mon–Wed have 4h 30m, 4h 40m and 4h 30m, and Thu has 09:00–13:00 and 13:30–16:55, all backdated with `-d`. Then I ran `mlm start 18:00` at 18:39.

| Scenario | Output | Matches |
|---|---|---|
| `status`, no live | `12h 00m left to 40h 00m target (fulfillment 28h 00m = worked 21h 05m + carry-in 06h 55m ...)` | §3.1 Fri–Sun |
| `status`, live 40m | `Day total: 00h 40m (incl. ongoing), est. EOD 06:00 (tomorrow)` and `11h 20m left to 40h 00m target (fulfillment 28h 40m = worked 21h 45m + ...)` | §2.3, §3.2. Worked and fulfillment equal §3.4's 21h 45m and 28h 40m |
| `week`, live | headline equals `status`, `Sat ... 00h 40m (incl. ongoing)`, `Worked: 21h 45m`, rows sum to it | §3.3 |
| `status 2026-10-01` and `status -2` | week line byte-equal to `status`, Day total `07h 25m` with no live and no flag | §3.1, §6 |
| `week 2026-41` | `Carry-in: -11h 20m` (28h 40m − 40h, live included), `Total behind: 51h 20m` | §2.3 |
| `week 2026-39` | `Total ahead: 06h 55m`, no `incl. ongoing` | §1.1 |
| `week target 28h43m` (Sat) | `00h 03m left to 28h 43m target`, `est. EOD 18:43` | §2.2 exact target |
| `week target 0m` (Sat) | `28h 40m over 00h 00m target`, `target already met` | §3.1 zero target |
| open start yesterday, `status -1` | `Day total: 00h 00m (+ ongoing)`, today's week line unchanged | §3.2 |
| open start 5 days back | `(+ unclosed)`, no live | §3.2 |
| `start 23:50` (later than now) | `00h 00m (incl. ongoing), est. EOD 15:50 (Mon)`, week line not counting down | §3.2, §6 |
| seeded DB (`--seed 42`) | `status`, `status 2026-10-01` and `week` agree on `05h 16m over 40h 00m target`. Rows sum to `Worked:` | §3.3 |

The real clock is a weekend, so I could not run these claims by hand:

- the Mon–Thu headline form (`left to 32h 00m by end of Thursday`)
- the Thursday `est. EOD 21:25` and the §3.4 Thursday blocks
- the §3.4 Friday line
- the Thursday-to-Friday and midnight form changes

I verified the first three through byte-exact fixed-clock tests, which I read and ran:

- `status::tests::pit_worked_example_status_block`
- `week_view::tests::pit_worked_example_week_block`
- `status::tests::pit_worked_example_friday_line`

They seed punches through real storage and resolve at Thu 2026-10-01 18:05 and Fri 18:00. The midnight cases are declared possible issues, and I did not exercise them.

## Test strength

I ran mutation tests on a scratch copy with a separate target dir. The repo was not touched.

| Mutation | Killed by |
|---|---|
| M1 status ledger drops live | 15 tests (`live_eod_*`, `pit_*`) |
| M2 `mlm week` `Worked:` drops live | 6 tests, incl. `b8_rows_sum_to_week_accounting_worked`, worked example |
| M3 status day total adds live on any `DATE` | `pit_earlier_date_week_line_counts_live_day_total_does_not` |
| M4 week row adds live on any date | `b3b_past_date_open_start_row_gets_no_live`, `b5_...` |
| M5 Fri–Sun `required` back to `5 × floor(target/5)` | 5 tests across `week`, `render`, `status` |
| M6 week today row drops live (rows ≠ `Worked:`) | 5 tests incl. `b8`, `pit_current_week_counts_live_...` |
| M7 status live only when `DATE` is today | `pit_earlier_date_...`, `pit_future_week_carry_in_includes_live` |
| M9 `Worked:` counts live on every date | `pit_yesterday_open_start_earns_no_live` |
| M10 live from the first open start, not the latest | 3 tests incl. `live_eod_t6_...` |
| M8 live credited to the walked week instead of the current week | **survives** |

M8 survives, but it changes no output. `status DATE` for a future week prints only `owed`, and `owed` is the same whether live sits in the current week's `worked` or the future week's. The `mlm week` path is a separate implementation, and `pit_future_week_carry_in_includes_current_week_live` pins its `Carry-in`. I don't count M8 as a finding.

Every regression the brief named fails at least one test:

- live left out of the ledger (M1, M2)
- live counted for a non-today date (M3, M4, M9)
- Fri–Sun `5 × floor` (M5)
- rows not summing to `Worked:` (M6)
- `status DATE` diverging from `status` (M7, plus `assert_eq!(monday.week_line, today_view.week_line)`)

The assertions are exact string or exact minute equalities. None are loose `contains` checks on the figures that matter.

## Spec and doc compliance

- SPEC §1.2a removes the five sentences spec §5 lists, rewords the pace-hint entries to name the week line, and adds every spec §6 entry. The zero-boundary entry is reworded.
- SPEC §2.4 has the Fri–Sun rule, the live-inclusive worked and fulfillment for the current week only, and future carry-in.
- SPEC §3.5 and §3.6 describe the point-in-time gap. The pace hint is no longer a separate item.
- SPEC §5 has the `owed` vs `gap` paragraph.
- SPEC §7.1 and §7.2 have the new sample, which is byte-equal to spec §3.4, the headline table, `(incl. ongoing)`, and no `est. EOD` in `week`.
- SPEC §8.1 sweeps F7, F9, F9b, F10 and F11.
- The README samples check out arithmetically. Saturday status `12h 44m` = 4h 00m + 3h 25m + 5h 19m. Week rows sum to 49h 54m, which is 9h 54m over. The `2026-22` line gives 32h − 19h 30m = 12h 30m.
- NOTES entries 90–93 supersede entry 37 for displayed figures, as spec §4 asks.
- AGENTS.md doc rule: no new behavior pointers into `docs/dev/plans/` or `specs/`. The one at SPEC.md:577 predates this changeset.

Exceptions are findings 1 and 3.

## Security

The changeset adds no string built from user input. The new strings are `format_minutes` output, weekday names and fixed literals. The `fix:`/`or:` shell-replayable lines are untouched and still contain only dates and times the code formats itself. Note bodies print verbatim as before. Nothing new reaches SQL, since all reads use the existing parameterized storage functions.

## Findings

### 1. `est. EOD` formula in the docs now reads as live-inclusive (doc, moderate)

- **Where**:
  - SPEC.md §7.1 "Estimated EOD" bullet: `it's open start + (required − fulfillment)`
  - README.md `status` prose: "`est. EOD` is the open start plus the time left to reach the required figure"
  - `src/status.rs` `EodState::At` doc comment: "the open stint's start plus `required - fulfillment`"
- **Problem**: These formulas were correct at baseline, when `fulfillment` excluded live. This changeset redefines `fulfillment` as live-inclusive two bullets earlier in §7.1, and in `DailyTargetHint`'s doc comment, but leaves the formula unchanged. Read literally, §3.4's example gives 17:25 + 03h 20m = 20:45, while the code and §7.1's own arithmetic line give 21:25. The README's "time left" is now the week line's displayed `left to` figure, which is the live-inclusive one, so a reader gets 20:45 too. SPEC §2.4 and spec §3.2 state the rule correctly ("fulfillment without live").
- **Fix**: say `open start + (required − (fulfillment − live))`, or "fulfillment without the running stint", in all three places.

### 2. Two copies of the gap, one feeding a field nothing renders (code quality, minor)

- **Where**: `src/status.rs` `today_hint` / `DailyTargetHint` / `StatusView::daily_target`, and `src/render.rs` `current_week_headline`.
- **Problem**: Both functions compute `required_minutes(target, weekday) − fulfillment` on their own. `DailyTargetHint` is no longer rendered, and outside `today_hint`'s `est. EOD` call only tests read its two fields. Tests assert both copies, so drift would be caught, but the view carries a dead struct and the rule lives in two places.
- **Related nit**: the `_use_datelike` shim in `src/status.rs` exists to silence an unused `Datelike` import. `Datelike` is now really used (`today.weekday()`), so the shim can go.
- **Fix (followup)**: expose one `week::gap(acct, today)` or similar helper used by both, and either drop `daily_target` from `StatusView` or make tests read the gap through that helper.

### 3. SPEC §4.2 names a negative display that never happens (doc, trivial)

- **Where**: SPEC.md §4.2, "A negative value (a closed week's ahead-of-target `owed`, ...) keeps the same padding with the sign in front".
- **Problem**: A closed week renders ahead-of-target `owed` as a positive magnitude under `Total ahead:` (§7.2), and the current week no longer prints `owed` at all. This changeset edited the sentence to add "a closed week's", which makes the wrong claim more specific. The line is also overlong compared with the surrounding wrap.
- **Fix**: drop the `owed` example and keep `carry_out` / `Carry-in:` / negative `fulfillment` as the signed cases.

## Verdict

Ship with followups. The behavior matches the locked spec and the updated SPEC sections. The worked example reproduces exactly, by hand for the live-inclusive figures and through fixed-clock tests for the weekday forms. The load-bearing tests kill every regression named in the brief. Findings 1 and 3 are doc fixes that fit in the release's docs pass. Finding 2 is a cleanup.
