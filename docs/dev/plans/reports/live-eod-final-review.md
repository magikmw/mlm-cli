# live-eod final review

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

- **Scope**: `c9e980b..live-eod` (code in `src/status.rs`; docs in `docs/dev/SPEC.md`, `README.md`, `docs/dev/NOTES.md`)
- **Against**: `docs/dev/specs/2026-10-03-live-eod.md` (locked), `docs/dev/plans/live-eod-plan.md` incl. "Review folds (phase 7)"
- **Date**: 2026-10-03 (Saturday), real clock
- **Verdict**: ship-with-followups (2 low-severity doc findings, no code findings)

## 1. Verification run

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo build` | ok |
| `cargo test` | 511 + 10 pass, 0 fail |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `clippy -W clippy::cognitive_complexity` (threshold 15) | no warnings |
| `coverage-baseline.json` | ratcheted up, not lowered |

## 2. Hand runs against the built binary

Baseline binary built from `git archive c9e980b` in the scratchpad, so each DB was read by both. Fresh DB per scenario under the scratchpad, `MLM_DB_PATH` set, real clock ~17:14 Sat. Target `3h` (Sat `required` = 180) unless noted.

| # | Punches today | New line | Old line | Spec rule |
|---|---|---|---|---|
| A | 09:00-10:00, start 16:45 | `01h 31m left to 03h 00m required today, est. EOD 18:45` | `02h 00m left …, est. EOD 19:14` | §3.1: 16:45 + 120 = 18:45; gap 120 − 29 |
| B | 09:00-10:00, start 15:00 | `00h 14m over …, target already met` | `02h 00m left …, est. EOD 19:14` | §3.1 / T3: live tips gap negative |
| C | 09:00-10:00, start 20:00 (future) | `02h 00m left …, est. EOD 22:00` | `02h 00m left …, est. EOD 19:14` | §3.3: live 0, EOD from start |
| D | 09:00-10:00, starts 16:00 + 16:45 | `01h 31m left …, est. EOD 18:45` plus E7 `[!]` | `02h 00m left …, est. EOD 19:14` | §3.2: most recent start only |
| E | 09:00-10:00 | `02h 00m left …` (no EOD) | identical | §3.1: no open stint matches baseline |
| F | target 40h; 09:00-10:00, start 16:45 | `38h 31m left to 40h 00m required today, est. EOD 07:45 (tomorrow)` | `39h 00m left …, est. EOD 08:14 (tomorrow)` | `(tomorrow)` still fires (bare word past one midnight, documented) |
| G | target 30m; 09:00-10:00, start 20:00 | `00h 30m over …, target already met` | identical | T9 variant |
| H | none today; open start 20:00 yesterday | `03h 00m left …` (no EOD) | identical | §3.2 / T8 |
| I | 09-10, 15:00-15:30, start 16:00 | `00h 16m left …, est. EOD 17:30` | `01h 30m left …, est. EOD 18:44` | break moves EOD later by its length (§1.2) |

Stability (A and F re-read three times, 61 s apart):

```
17:14:43  01h 31m left … est. EOD 18:45   |  38h 31m left … est. EOD 07:45 (tomorrow)
17:15:44  01h 30m left … est. EOD 18:45   |  38h 30m left … est. EOD 07:45 (tomorrow)
17:16:45  01h 29m left … est. EOD 18:45   |  38h 29m left … est. EOD 07:45 (tomorrow)
```

The hint counts down one minute per minute and `est. EOD` holds still, as §1 and F9 require.

## 3. Correctness

- `gap = base_gap − live` with `live = ctx.day.open.last().minutes_so_far` matches spec §2 and §3.1.
- `est_eod` adds `base_gap` to `open.start.at_utc` and branches on the post-live `gap_minutes`. This matches the §3.1 table, §3.3, and the `target already met` threshold.
- `stint::classify` leaves `open` in ascending start order (stack drains bottom-up), so `.last()` is the most recent start (§3.2).
- `ctx.day` (post-splice) is the source, matching spec §2. `has_open_stint` reads the same `ctx.day`, so the hint and the `(+ ongoing)` flag cannot disagree.
- When `gap_minutes > 0`, `live ≤ base_gap − 1`, so `start + base_gap > now`. A clock time printed under `est. EOD` is never in the past.
- `minutes_so_far` is floored and clamped at 0 in `stint.rs`. Punches are minute-aligned, so the `NaiveTime` inside `EodState::At` has zero seconds.
- Past-date `status` is untouched, because the whole block sits under `is_today`.
- DST: adding minutes to a `DateTime<Local>` is physical-time addition, the same as the old `now + gap`. No new hazard.

No correctness findings.

## 4. Test strength (mutation-checked)

Each mutant was built from `git archive live-eod` in the scratchpad with its own target dir. `cargo test --lib live_eod` was run on each.

| Mutant | Failing live-eod tests |
|---|---|
| Old formula (`gap = base_gap`, `eod = now + gap`) | T1, T2, T3, T4, T6, T7, T9, T10 |
| Sum all open stints' live | T6 only |
| First (earliest) open start instead of last | T6 only |
| `ctx.day_raw` instead of `ctx.day` | splice test only |
| `eod = now + post-live gap` (display-equivalent for past starts) | T2, T9 |
| `live` rounded to nearest minute instead of floored | T2, T9 |

- **T2**: fails under the old formula (the EOD differs and the gap delta is 0). The 20 s / 45 s seconds catch rounding drift, and the `NaiveTime` comparison catches seconds leaking from `now` into `EodState::At`.
- **T6**: the only guard against both summing and picking the earliest start. It asserts both `gap_minutes` (150, versus 120 summed and 135 earliest) and `est. EOD` (20:30, versus 20:15 earliest).
- **Splice**: the only guard against the `day_raw` regression. It asserts `has_open_stint == false`, `eod == None` and `gap == 1920 − 405`.
- **T9**: the only test besides T2 that separates `start + base_gap` from `now + gap`, because the two differ only for a future start (21:45 vs 20:45).
- **T1 / 18:00 sample line (F6)**: rebuilt from punches through `resolve` + `render`. It pins the full SPEC §7.1 "after" line and fails under the old formula. The 18:30 follow-up also pins the "still 20:30" claim from spec §3.1.
- **T9 variant**: passes under the old formula too. It checks the branch, not the formula, and that is acceptable because T9 carries the formula check.
- Pre-existing resolve assertions (`resolve_fix_b_eod_estimate_crossing_midnight_is_marked_tomorrow` and the `gap_minutes` checks the plan names) still hold post-live. Their comments now describe the post-live meaning (F2/F3).
- Hand-built fixtures moved to `150` / `20:30` (F4). `grep 20:45 src/status.rs` finds nothing stale.

## 5. Spec compliance

| Item | Status |
|---|---|
| SPEC §2.4 (live enters the pace hint and est. EOD, display-only) | done |
| SPEC §2.4 `required` bullet (fulfillment less live) | done, see finding 2 |
| SPEC §1.2a (all spec §1.2 items) | done; the overnight item is its own bullet that cites the yesterday bullet rather than being merged into it, which is acceptable under plan fold F5 |
| SPEC §3.5 (pace hint/EOD depend on the running stint and now) | done |
| SPEC §7.1 sample, pace-hint bullet, EOD bullet, day-total wording | done; the stale `now + (required − fulfillment)` / "excludes live" text is gone |
| SPEC §8.1 F9 stability, F9b over-by-live | done |
| Changeset spec §1.2 known issues | all nine are present in SPEC §1.2a; README carries the week-line divergence and the EOD meaning |
| Changeset spec §3.1-§3.3 | matches code and hand runs |
| Changeset spec §5 T1-T10 + F7 extras | all present, see §4 |
| README feature bullet, known issues, Saturday sample, pace paragraph | done; `−275 − 319 = −594 = 09h 54m` checks out |

## 6. Doc self-containment

- SPEC.md and README.md contain no pointers into `docs/dev/specs/` or `docs/dev/plans/`.
- The new NOTES.md entries (84-87) and the "Current state" block cite spec and report paths as provenance. That follows NOTES' existing convention (36 such references at baseline), and none of them stands in for behavior detail.
- Wording is present-state. One historical phrase remains in §3.5 ("the original, confusing behavior — fixed here"), but it predates this changeset.
- `--help` text in `src/cli.rs` does not mention the pace hint or EOD, so it needs no change.

## 7. Code quality

- `est_eod` is small and documented, and it keeps `resolve` under the complexity gate.
- The new tests use local helpers (`punch`, `now_at`, `seed_thursday_completed`) and follow the existing test style.
- AGENTS.md conventions hold: no platform paths, no new modules, `Cargo.lock` unchanged.

## 8. Security

- The change builds no strings from user input. It reads two integers and one timestamp and formats a `NaiveTime`.
- The shell-replayable `fix:` lines are unchanged. They interpolate only parsed dates and times, never note text.
- No new SQL. Reads go through the existing parameterized `storage` functions.

No security findings.

## 9. Findings

1. **Low, doc formatting.** SPEC.md §7.1's pace-hint bullet leaves two lines unwrapped (`less live minutes under …` through `… is a **display-only pace`, and `enters the week line's \`owed\` … \`X\` is always …`). §8.1 F9 does the same (`(line omitted). Stability: with the same punches, two`). All three run well past the file's ~72-column wrap. To fix, reflow the three paragraphs.
2. **Low, doc clarity.** In the SPEC.md §2.4 `required` bullet, the text now reads "fulfillment (…) less the running stint's live minutes (§7.1) — i.e. carry-in *does* count here, same fulfillment figure the week-level `owed` uses". A reader can take "same figure the `owed` uses" to mean that `owed` also subtracts live minutes, but the sentence before says it never does. To fix, attach the "same figure" clause to fulfillment before mentioning live, e.g. "Compared against the week's fulfillment (…), the same figure `owed` uses, less the running stint's live minutes (§7.1)."

## 10. Verdict

**ship-with-followups.** The code is correct and matches the locked spec. The load-bearing tests fail under every plausible regression tried. The two followups are cosmetic SPEC.md edits.
