# Task 2 report: Anomaly rendering and fix-line wording

Worktree branch: `worktree-agent-a40b1f9b82a311c94`.
Commit: `13a6248`.

## What was implemented

`src/render.rs` gained, additively, beside the existing `Anomalies`
struct/impl/tests (all untouched):

- `pub struct DateAnomalies` (fields: `date`, `open_stint_count`,
  `latest_open_start`, `orphaned_end_times`, `stale_open_start`,
  `refused_join_with_previous`, `refused_join_with_next`) with
  `has_any()`, `detail_lines()`, `row_marker()`.
- `pub struct StaleOpenStart { time: NaiveTime, age_days: i64 }`.
- `pub struct LookbackFinding { date: NaiveDate, age_days: i64, kind: LookbackKind }`
  with `detail_lines()` and `stderr_line()`.
- `pub enum LookbackKind { SingleOpenStart { time: NaiveTime }, MultipleOpenStints { count: usize } }`.
- `pub const WEEK_ANOMALY_FOOTER: &str = "[!] needs a look: run mlm status DATE for details and fixes"`.
- Private helpers `e7_lines`, `stale_lines`, `e8_lines`,
  `refused_join_lines` (methods on `DateAnomalies`) and a free function
  `refused_join_line(named, earlier, later) -> String`.

All signatures match the task plan's Published Interface (Contract A)
verbatim.

## TDD sequence

Test-first: appended the full set of new-type tests (`t_e7_reworded_matches_spec_example`,
`t_stale_matches_spec_example`, `t_e8_gains_fix_lines_text_unchanged`,
`t_refused_join_previous_side`/`_next_side`, `t_lookback_single_block_matches_spec_example`,
`t_lookback_multi_block_matches_spec_example`, `t_ordering_all_kinds_at_once`,
`t_lookback_ordering_oldest_first`, `t_week_footer_matches_spec_example`,
`t_lookback_stderr_single_matches_spec_example`/`_multi_...`,
`t_lookback_stderr_single_space_differs_from_block_form`,
`t_date_anomalies_new_fixtures_never_contain_old_e7_suffix`) before
adding any of the new types. `cargo build --tests` failed to compile
(39 errors, all "cannot find type `DateAnomalies`/`LookbackKind`/etc.
in this scope" plus `WEEK_ANOMALY_FOOTER` not found) — confirmed
red. Then implemented the types/methods per the plan and reran; one
test (`t_ordering_all_kinds_at_once`) failed once on a fixture bug of
mine (I'd assumed 2026-09-20 was a Monday; it's a Sunday), fixed the
test's own expected string, then all green.

## Deviations from the plan

1. **Worktree was not based on branch tip at task start.** The
   worktree's branch (`worktree-agent-a40b1f9b82a311c94`) was rooted
   on an old, unrelated branch (last commit `f76ad3b`, a
   status-wording-fixes-era release commit), not on
   `open-punch-visibility`'s tip (`6af5766`). Per instructions, I ran
   `git rebase open-punch-visibility` before doing any other work; this
   was a clean rebase (no conflicts, nothing of mine to replay), and
   `git merge-base --is-ancestor 6af5766 HEAD` confirmed the worktree
   was based on tip afterward. This is a worktree-setup issue, not a
   plan or scope problem — no plan content or file ownership was
   affected.

2. **AC6's ASCII sweep test was split into two test functions, not
   one.** The plan's §1.8/AC6 explicitly says to extend the existing
   `t33_every_produced_string_is_ascii` sweep "rather than adding a
   second, parallel sweep test." Doing that literally (appending all
   the new `DateAnomalies`/`LookbackFinding`/`WEEK_ANOMALY_FOOTER`
   fixtures and loops directly into `t33`'s body) pushed `t33`'s
   cognitive complexity to 16, one over the repo's threshold of 15
   (`clippy.toml`), which the pre-commit hook enforces and which
   failed my first commit attempt with exactly that finding. I split
   the new-literal coverage into a second function,
   `t33b_open_punch_visibility_literals_are_ascii`, immediately after
   `t33`, rather than adding an `#[allow]` (out of scope per the task
   instructions — "if clippy flags dead code anyway, report it rather
   than adding blanket allows," and the spirit applies to cognitive
   complexity too: don't suppress the gate, restructure instead).
   `t33` itself is otherwise unchanged from before this task. This is
   a deliberate, minimal deviation from "one sweep test" to satisfy the
   complexity gate; the coverage/ASCII guarantee AC6 asks for is fully
   intact, just split across two `#[test]` functions instead of one.

No other deviation. No published signature differs from the plan's
Contract A section. No existing file outside `src/render.rs` was
touched. `render::Anomalies` (struct, impl, all its `t16`-`t38`
tests) is byte-for-byte unchanged.

## Final public signatures (verbatim, for Task 3/Task 5)

```rust
pub struct DateAnomalies {
    pub date: NaiveDate,
    pub open_stint_count: usize,
    pub latest_open_start: Option<NaiveTime>,
    pub orphaned_end_times: Vec<NaiveTime>,
    pub stale_open_start: Option<StaleOpenStart>,
    pub refused_join_with_previous: Option<NaiveDate>,
    pub refused_join_with_next: Option<NaiveDate>,
}

impl DateAnomalies {
    pub fn has_any(&self) -> bool;
    pub fn detail_lines(&self) -> Vec<String>;
    pub fn row_marker(&self) -> &'static str;
}

pub struct StaleOpenStart {
    pub time: NaiveTime,
    pub age_days: i64,
}

pub struct LookbackFinding {
    pub date: NaiveDate,
    pub age_days: i64,
    pub kind: LookbackKind,
}

pub enum LookbackKind {
    SingleOpenStart { time: NaiveTime },
    MultipleOpenStints { count: usize },
}

impl LookbackFinding {
    pub fn detail_lines(&self) -> Vec<String>;
    pub fn stderr_line(&self) -> String;
}

pub const WEEK_ANOMALY_FOOTER: &str =
    "[!] needs a look: run mlm status DATE for details and fixes";
```

## Verification (all green)

- `cargo fmt` — clean.
- `cargo build` — clean.
- `cargo test` — all suites pass, including the untouched
  `tests/status_cli.rs` (5 tests) and every other integration test
  file; `render::` unit tests: 57 passed (56 pre-existing plus new).
- `cargo clippy --all-targets -- -D warnings` — clean.
- Pre-commit hook (fmt / cognitive complexity / coverage ratchet) —
  passed on the final commit; coverage improved
  (99.07323979920196% -> 99.07347831681892%), baseline auto-ratcheted
  and included in the commit (`coverage-baseline.json`).

## Open questions for the reviewer

None beyond what the task plan's own "Risks, ambiguities" section
already flags (the `latest_open_start` field addition to Contract A,
and the intentionally-invariant-free ordering test). The AC6 test
split (deviation 2 above) is the one thing I'd want a reviewer's eyes
on, in case "one sweep test" was meant more strictly than I read it.
