# overnight-visibility final review

Baseline 6edd67b, tip c2ec743 (clone HEAD matches). Verdict: ship.

## Verification (run in the clone)

- cargo fmt --check: clean.
- cargo build: ok.
- cargo test: all pass (754 lib tests plus 11 integration binaries, 0 failed).
- cargo clippy --all-targets -- -D warnings: clean. A run with
  -W clippy::cognitive_complexity printed no cognitive warnings.

## Smoke runs (TZ=UTC, temp DB, today is Sunday 2026-10-04)

- Lone start yesterday: status prints the `[i]` hint and
  `08:00-now    (29h 19m so far, started yesterday, not counted until stopped)`
  after a blank line, with Day total 00h 00m and no est. EOD. `mlm week` marks
  `Sat 2026-10-03   00h 00m  [i]` and prints the `[i]` footer. Figures unchanged.
- Plus own open start today: overnight line first, own `-now (ongoing)` line
  second. Day total counts only today's stint.
- `status -1`: unchanged older wording ("ongoing - duration as of right now").
- Added an age-2 open start: `[!]` row and footer print above the `[i]` footer
  in the right order. `[i]` never fed `[!]`.
- Monday case: the real clock is Sunday, so it could not be run on the binary.
  Covered by unit tests: monday_current_week_uses_loaded_buckets,
  monday_footer_names_sunday_once, monday_other_weeks_show_nothing,
  past_week_by_id_marks_sunday_row,
  monday_sunday_row_with_anomaly_shows_both_markers,
  monday_footer_hand_built_golden. Their assertions are exact text or
  full-render equality.

## Mutation checks (all reverted)

- Disable the refused-join check in overnight_stint: 5 tests fail, including
  the status/hint agreement test.
- Drop the `holds(today)` condition in monday_overnight:
  monday_other_weeks_show_nothing fails.
- Collapse the `[!] [i]` marker: 3 tests fail.

## Review

- Shared trigger: `lookback::overnight_stint` is the single decision.
  today_page_lines (hint), the status overnight line and the week row mark all
  use it. The agreement test iterates six fixtures and asserts hint presence
  equals line presence.
- week: load_history's range covers yesterday and today whenever the shown
  week holds yesterday or today, and the trigger returns None otherwise.
  Far-past and future weeks show nothing, as the spec requires.
- `[i]` stays out of has_anomaly, the `[!]` footer, the carry marker and the
  accounting. overnight_does_not_change_any_figure and
  overnight_line_precedes_todays_stints_and_figures_unchanged assert this.
- Footer order and blank-line rule: notice_lines plus a 16-mask test.
- Byte-identical output without a stint: no-overnight paths push nothing and
  the existing goldens pass unchanged.
- Glossary, README and SPEC §1.2a ("None currently.") are updated. No stale
  "never marks a row" wording remains.
- Security: output strings are built only from dates and numbers formatted
  internally. No user text goes into shell-replayable strings.
- Complexity: small helpers (row_markers, notice_lines, overnight_footer,
  monday_overnight, push_stint_section), well under 15.

## Findings

None blocking.

Observations, no action required:

- tests/overnight_visibility_cli.rs uses the real clock with TZ=UTC and can
  flip a date if a run straddles UTC midnight. The file header documents it
  and it matches start_hint_cli.rs.
- The Monday case has no binary-level test, which is inherent to the real
  clock.

Verdict: ship
