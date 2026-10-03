# Completion report: possible-issues-triage T2 (overlap anomaly, clean-pair stop hints)

Branch `pit-task-2`. The worktree opened on old `main`, so I rebased it onto `possible-issues-triage` (f8789ee) before starting.

## Implemented

All in `src/anomalies.rs`, plus new `tests/overlap_cli.rs`.

- `StintSpan`, `OverlapPair`, `overlap_pairs`, and two new `DateAnomalies` fields (`overlaps`, `latest_open_next`) with `none`, `has_any` and `build_facts` wired.
- Span construction: `local_midnight`, `day_end`, `closed_span`, `live_span`, `date_spans`.
- Rendering: `overlap_lines` between E8 and refused joins, `label`.
- E7 `or:` and stale `fix:` lines now call `render::stop_time_hint_closes` and `render::stop_time_hint` with `latest_open_next`.
- `build_facts` no longer discards `now_utc`.

## First failing test

The whole `overlap_tests` module failed to compile (`StintSpan`, `overlap_pairs`, `OverlapPair`, `local_midnight` and the two `DateAnomalies` fields missing). After implementing, two fixture-level tests failed for fixture reasons (see Deviations).

## Deviations

1. `received_span` is not implemented. The receiving side of a splice can never overlap anything. The spliced orphaned end must be the date's first punch, and a Start sorts before an End at the same instant. Every other stint on that date therefore starts at or after the orphan's end, so none can intersect the part from local midnight to that end. The span would be dead code with no reachable test. `date_spans` takes no `b` parameter as a result, and its doc comment says why. Behaviour matches the spec (the start date still compares the part up to its midnight, and both dates print the real range). The plan's AC1 receiving-side overlap tests and the zero-length-at-midnight test are replaced by one test asserting the receiving date shows no overlap and no orphan.
2. The start-side spliced fixture differs from the plan's. Plan punches (start 22:30, end 23:30, start 23:00) pair LIFO into 23:00-23:30 with 22:30 spliced, so the outer stint is the spliced one. The test now asserts on `first` (label `22:30-01:00`, end at next local midnight).
3. Re-pin in my own file: `t_ordering_all_kinds_at_once` in `src/anomalies.rs` is a full struct literal, so it gained `overlaps: Vec::new(), latest_open_next: None`. Assertions are unchanged. No T1-owned file needed a re-pin, and no existing test turned red.

## Final public signatures

```rust
pub struct StintSpan { pub start: DateTime<Utc>, pub end: DateTime<Utc>, pub full_start: DateTime<Utc>, pub full_end: DateTime<Utc>, pub open: bool }
pub struct OverlapPair { pub first: StintSpan, pub second: StintSpan }
pub fn overlap_pairs(spans: Vec<StintSpan>) -> Vec<OverlapPair>
// DateAnomalies gains:
pub overlaps: Vec<OverlapPair>,
pub latest_open_next: Option<NaiveTime>,
```

`build_facts`, `date_facts`, `classify_context` keep T1's signatures.

## Notes for the reviewer

- The `local_midnight` DST-gap fallback closure is not covered (needs a zone whose midnight does not exist).
- `tests/overlap_cli.rs` tests passed on first run because the implementation already existed when they were written. The unit tests were red first.
