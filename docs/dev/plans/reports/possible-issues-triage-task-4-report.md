# Report: possible-issues-triage T4, carry-in anomaly scan

## Base

The worktree started on an unrelated branch (tip f76ad3b, not a descendant of `possible-issues-triage`). Branch `pit-task-4` was created from `possible-issues-triage` (f8789ee), so no rebase was needed.

## Implemented

- `src/carry_scan.rs`: `carry_marker` now scans punch dates from `earliest_date` through `shown.start() - 1`, using `anomalies::date_facts(..).has_any()` on `buckets.trio(date)`. Helpers `anomalous_weeks` (BTreeSet of weeks, per-week short circuit) and `date_is_anomalous`. No storage reads. T1's `stub_returns_none` test is replaced by 18 unit tests.
- `tests/carry_marker_cli.rs`: 20 binary-level tests.

## First failing tests

With the stub: 11 unit tests failed on the missing marker (for example `kind_e7_raises_marker`, `only_dates_before_shown_week_start_scanned`), and 15 of 20 CLI tests failed (for example `kind_e8_names_its_week`). The negative tests passed against the stub, as expected.

## Deviations

1. **`src/status.rs` edited (not a T4-owned file).** T1's test `status::tests::resolve_status_for_non_today_date_runs_no_lookback` seeds an open start 5 days before the fixture today. That date falls in an earlier week than the shown date, so the real scan now (correctly) emits the carry-in marker line, and the test's `anomaly_lines.is_empty()` assertion failed. I changed that one assertion to ignore lines starting with `[!] carry-in includes anomalies`, so it still rules out lookback lines. The edit is test-only. The coordinator may reject it and fix the test another way. The commit hook runs the suite, so the commit could not land without it.
2. Test count and shape differ slightly from the plan's list. CLI `no_earlier_anomaly_output_unchanged_*` byte-golden tests became `no_marker_means_no_flag_and_unchanged_layout` (no `[!]`, blank line before `Carry-in:`). `notes_only` and `clean` CLI cases follow the plan.
3. `weeks.contains` plus `date_is_anomalous` are one `&&` condition instead of a `continue`. Same behavior.

## Public signature (unchanged)

```rust
pub fn carry_marker(
    history: &History,
    shown: WeekId,
    today: NaiveDate,
    now_utc: DateTime<Utc>,
) -> Option<CarryMarker>
```

## Timing (release, `seed_test_data --weeks 150 --seed 42`, 2024 punches)

| Command | Wall time |
|---|---|
| `mlm status` | 0.01 s (3 runs) |
| `mlm week` | 0.00 to 0.01 s (3 runs) |

The seeded history is anomaly-heavy (about 58 injected anomalies), so the per-week short circuit trims work. A fully clean history pays one `date_facts` per punch date, about 1000 dates for 150 weeks, and stays in memory.

## Mishap

One smoke command (`cargo run -- start "9:00" "note"`) ran once without `MLM_DB_PATH` and wrote a punch and a note to the real app-data DB for today. I listed today's entries (only those two existed), deleted both with `mlm delete punch 1` and `mlm delete note 1`, and re-ran the smoke commands against `/tmp/t4/mlm-check.db`. The real DB has a gap in its autoincrement ids only.

## Open questions

- Overlap-only tests are deferred until T2 merges: CLI `overlap_only_earlier_week_raises_marker` (fixture week 2026-06, date 02-03: start 09:00, end 12:00, start 10:00, end 11:00; also asserts a `[!]` row on `mlm week 2026-06` and the overlap block on `mlm status 2026-02-03`) and unit `kind_overlap_raises_marker` / `overlap_only_date_counts_once_t2_merged`.
- The refused-join term cannot be isolated at the CLI (a past open start is always stale). Unit `refused_join_with_recent_open_start_flags_later_date` only checks that the date flags.
