# Milestone 10 — Independent adversarial review

Reviewer: independent pass, not the implementer. Scope: `src/status.rs`,
`src/cli.rs`, `src/main.rs`, `tests/status_cli.rs` against
`plans/milestone-10-status-command.md`, the implementer's own report,
SPEC.md §3.5/§7.1/§7.3/§7.4, PLAN.md's pinned interface contracts, and
Milestone 9's independent review (the two landmines), plus the already-
merged `src/render.rs`, `src/week.rs`, `src/stint.rs`, `src/storage.rs`,
`src/date.rs` this milestone consumes.

Verification performed directly (not taken on faith):
- `cargo build`: clean.
- `cargo test`: **274 passed** (lib) **+ 3 passed** (`tests/status_cli.rs`)
  = **277 passed, 0 failed** — matches the report's claim exactly.
- `cargo clippy --all-targets -- -D warnings`: clean, no warnings.
- `git diff 5cf70c2 9fe965a --stat`: touches only `src/cli.rs` (+6),
  `src/main.rs` (+2), new `src/status.rs`, new `tests/status_cli.rs`, and
  the report — confirms no changes to `render.rs`, `week.rs`, `stint.rs`,
  `storage.rs`, `date.rs`, `db.rs`, or any Milestone 7/8 command logic.
- Ran the actual compiled binary by hand against a temp `MLM_DB_PATH`
  (two dangling `start`s, then `status`, then a malformed-date `status`):
  confirmed plain-ASCII multi-open anomaly rendering, exit 0 with the
  anomaly line on stdout and empty stderr for the happy path, and exit 1
  with the error message on stderr / nothing on stdout for
  `status 2026-02-30`. Ran a second time isolating stdout/stderr to
  separate files and confirmed `status`'s own stdout carries only the
  rendered view, nothing from logging. Temp DB files removed after each
  run.
- Traced real signatures directly rather than trusting the plan's
  guessed shapes: `render::status_week_line(week, owed_minutes,
  fulfillment_minutes, target_minutes, today)` (src/render.rs:76-100),
  `week::WeekAccounting` fields (src/week.rs:59-77),
  `week::daily_target_minutes` (src/week.rs:225-227),
  `stint::DayStints` (`open`, `orphaned_ends`, `completed_minutes()`,
  `is_ongoing()`, and the separate `has_anomaly()`/`anomalies()` pair —
  src/stint.rs:28-115), `storage::{punches_for_date, notes_for_date,
  earliest_data_date}` (src/storage.rs), `date::{parse_date,
  format_date_with_weekday, week_range}` and `WeekId::{from_date, start,
  end}` (src/date.rs).
- Hand-computed both §7.1 worked examples against the golden test
  strings (T7, T11) byte-for-byte; both match SPEC.md's literal text
  exactly, including the label-column padding, the `(+ ongoing)` flag
  placement, the signed gap format, and the parenthetical's presence/
  absence.

## Findings

1. **Both §7.1 worked examples are golden-tested byte-for-byte and both
   pass.** T11 (`t11_golden_full_first_spec_example`,
   src/status.rs:759-809) reconstructs the "today" example exactly —
   day total `07h 25m (+ ongoing)`, `00h 35m left to 08h 00m daily
   target, est. EOD 18:35`, week line `10h 45m left by end of Thursday
   (fulfillment 29h 15m / target 40h 00m)`, three stint lines, two
   notes — and asserts full string equality against a literal built from
   the same characters as SPEC.md's block. T7
   (`t7_f10_golden_second_spec_example`, src/status.rs:645-674)
   reconstructs the past-date example — no daily-target/EOD segment,
   plain `Total still owed: 01h 40m` week form, no parenthetical, single
   stint line — also via full string equality. Both were re-verified by
   hand against SPEC.md §7.1's literal blocks; no discrepancy. No
   finding.

2. **F11's exact guarantee is traced through an actual test, not a
   comment.** `resolve_f11_is_today_false_but_week_is_current`
   (src/status.rs:980-1004) seeds a real DB with `status 2026-02-09`
   (Monday) resolved against `now = Thursday 2026-02-12 18:00`, asserts
   `view.daily_target.is_none()` and `view.eod.is_none()` (DATE isn't
   today), asserts the week line contains `"left by end of Thursday"`
   and not `"Monday"`, and additionally calls `render::week_framing`
   directly to independently confirm the week is judged `Current` from
   the week id and `today` alone — never from `target_date`. The render-
   level companion (T8, src/status.rs:679-689) exercises the same
   scenario at the pure-render layer. Both pass. No finding.

3. **F9's three EOD states are each exercised distinctly, including the
   full-omission case.** T6a (gap > 0: `est. EOD HH:MM`, value checked
   against the literal `18:35`), T6b (gap == 0 and gap < 0, both taking
   the `target already met` branch — the boundary case is explicitly
   tested, not just gap < 0), and T6c (no open stint: neither `est. EOD`
   nor `target already met` appears anywhere in the string — a true
   substring-absence check, not an empty-but-present segment). The
   implementation (`day_total_line`, src/status.rs:114-138) matches:
   `view.eod` is `Option<EodState>`, and the `None` arm of the `match`
   pushes nothing at all, so the omitted case really produces no
   trailing characters rather than an empty string fragment. No finding.

4. **E7/E8 verified through `render::Anomalies` exclusively; no direct
   call to `DayStints::has_anomaly()`/`anomalies()` anywhere in this
   milestone's new code.** Grepped `src/status.rs` and `tests/` for
   `.has_anomaly()`/`.anomalies()` — zero matches. `to_anomalies`
   (src/status.rs:201-218) builds `render::Anomalies` directly from
   `DayStints`'s raw `open` (via `.len()`) and `orphaned_ends` fields,
   converting each orphan's UTC instant to a local `NaiveTime`; from
   there, `Anomalies::detail_lines()` is the sole source of the anomaly
   lines. `resolve_e7_multi_open_and_e8_orphaned_end_via_anomalies_only`
   (src/status.rs:1006-1018) exercises a real DB with two dangling
   starts and one orphaned end, asserting exactly 2 anomaly lines (the
   multi-open line first, then the orphan by timestamp) and that the
   orphan produces zero stint lines. T9 and T10 at the render layer
   further confirm 2+ open stints and 2+ orphaned ends (including
   never-coalesced identical-shaped orphan lines) render correctly. This
   is exactly the seam Milestone 9's review flagged as unguarded, and
   Milestone 10 goes through the guarded path throughout. No finding.

5. **No argument-order transposition at the `status_week_line` call
   site, and the regression guard is real, not a tautology.** The single
   call site is `week_line()` (src/status.rs:197-199):
   `render::status_week_line(acct.week, acct.owed, acct.fulfillment,
   acct.target, today)` — matches the real signature's parameter order
   (`week, owed_minutes, fulfillment_minutes, target_minutes, today`,
   src/render.rs:76-82) field-for-field by name. The dedicated test
   `week_line_matches_status_week_line_field_for_field`
   (src/status.rs:1052-1058) is a same-module consistency check (it
   would only catch a *future* edit to `week_line`'s body diverging from
   a separately-written expected value — it does not independently prove
   the order is correct against SPEC on its own). The real
   spec-correctness guard is T11/T7's golden byte-equality: T11 uses
   `current_week_acct(owed=645, fulfillment=1755, target=2400)`, whose
   distinct values (`10h 45m`, `29h 15m`, `40h 00m`) each appear in a
   different, unambiguous position in the expected string — a swap of
   any two of the three would surface a different number in the wrong
   slot and fail the golden assertion. Confirmed by hand-tracing: this
   would in fact catch owed/fulfillment and fulfillment/target swaps.
   No finding.

6. **F4/E11 section omission confirmed as true omission, not
   empty-but-present.** T4 (note-only day) asserts no line both starts
   with two spaces and contains `(` — the shape only a stint line has —
   and T5 (E11, zero punches and zero notes) asserts the output is
   *exactly* four lines via full string equality plus `out.lines().count()
   == 4`, the tightest possible omission test. `resolve_note_only_day_...`
   repeats the same shape check at the `resolve` level with a real DB.
   `render()`'s structure (src/status.rs:158-185) only pushes a stint/
   notes block (blank line + content) inside an `if !empty` guard, so
   there is no code path that could emit a header with no body. No
   finding.

7. **Plain-ASCII output, `[!] ` prefix, exit 0 with anomalies on stdout
   verified live against the compiled binary, not just unit tests.**
   T13 asserts every byte of representative rendered output (including
   an anomaly line) is in `0x20..=0x7E` or `\n`, with a companion
   assertion that non-ASCII note bodies still pass through unmangled
   (correctly scoped — SPEC §2.3 permits non-ASCII in note bodies only).
   `tests/status_cli.rs`'s T14 drives the real binary with two dangling
   starts, asserts exit code `Some(0)`, the anomaly line on stdout, and
   empty stderr. This reviewer independently re-ran the same scenario by
   hand against a fresh temp DB (see Verification list above) and
   observed identical behavior: plain-ASCII output, `[!] 2 open stints
   for this date (unmatched starts)`, exit 0, empty stderr. No finding.

8. **Scope is exactly as claimed.** `git diff` against the milestone-9
   merge commit touches only `src/cli.rs` (+6 lines, the new `Status`
   variant), `src/main.rs` (+2 lines, module declaration and dispatch
   arm), the new `src/status.rs`, and the new `tests/status_cli.rs`. No
   changes to `render.rs`, `week.rs`, `stint.rs`, `storage.rs`,
   `date.rs`, or `db.rs`. The `Log` scaffold-removal item from the plan
   was correctly identified as already moot (Milestone 7 had already
   removed it) rather than papered over. No finding.

9. **`src/status.rs:340-354` — the EOD-estimate/gap arithmetic mixes an
   `i64` minute gap with `now: DateTime<Local>` via
   `ChronoDuration::minutes(gap_minutes)`, requiring `gap_minutes > 0`
   at that call site.** This is correctly guarded by the `if gap_minutes
   > 0` branch immediately above it, so it is not reachable with a
   non-positive value today. Purely a note for a future maintainer: the
   guard and the arithmetic are two lines apart rather than encoded in
   a type, so a refactor that reorders the branches could reintroduce a
   silent bug (e.g. computing `now + Duration::minutes(gap)` for a
   negative gap, which chrono would happily accept and produce a
   plausible-looking but wrong past "EOD"). Not a defect in the code as
   written — all paths are correctly tested (T6a/T6b/T6c) — flagging
   only as a low-value hardening idea. Severity: **minor**.

10. **`build_ledger` (src/status.rs:250-301) re-scans every date from the
    earliest data date through the target week's Sunday on every
    `status` call, calling `punches_for_date`/`notes_for_date` once per
    date rather than using a bulk range read.** This is explicitly
    flagged by the implementer's own report as a known complexity
    tradeoff (mirrors `week::week_series`'s own O(weeks) walk, and
    PLAN.md contract 10's `punches_in_range`/bulk adapter was left to a
    later integration step, not this milestone's scope). Confirmed by
    reading the loop: no early-exit, no caching, one DB round-trip pair
    per calendar date in the span. For an MVP single-user tool with a
    modest history this is unlikely to matter in practice, but it is
    the one place this milestone's own implementation deviates from the
    "cheap, already-tested logic, just wire real data" framing by
    redoing a linear-in-history-length scan on every single `status`
    invocation (not just once per `week` call, which is inherently
    less frequent). Severity: **minor** (accepted tradeoff, correctly
    disclosed, not a correctness bug).

## Verdict

**APPROVE** — this milestone is a precise, byte-for-byte faithful
realization of the plan and both of SPEC.md §7.1's worked examples;
build, the full test suite (277/277: 274 lib + 3 binary), and clippy are
all independently confirmed clean, and the compiled binary was run by
hand and behaved exactly as specified (plain ASCII, exit 0 with
anomalies on stdout, nonzero exit with stderr-only errors). Both
landmines flagged by Milestone 9's review were verified as actually
closed in this milestone's code, not just claimed: no call anywhere to
`DayStints::has_anomaly()`/`anomalies()` (anomaly rendering goes through
`render::Anomalies` exclusively via `to_anomalies`), and the one call
site for `render::status_week_line`'s adjacent same-typed `i64` params
is a single named-field wrapper whose correctness is additionally
guarded by golden tests (T7/T11) that would catch a transposition. F11's
"today's weekday, not the queried date's" guarantee and F4/E11's true
section omission were both traced through real tests operating on a real
in-memory DB, not taken on faith. Scope is exactly as claimed: no edits
to any already-merged milestone's module. The two minor notes above
(gap-sign/arithmetic proximity, and `build_ledger`'s per-call linear
rescan) are hardening/performance observations, not defects, and do not
block merge.
