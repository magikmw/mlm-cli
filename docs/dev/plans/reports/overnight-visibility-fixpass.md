# overnight-visibility fix pass

Branch `ov-fixpass`, created from the `overnight-visibility` tip `ca58e72`
(the worktree started on an older base, so the branch was cut fresh from
the integration tip).

## FE2 + FE6 (fixed)

- The plain `[i]` footer is now `[i] open stint on Sat 2026-10-03: mlm status shows how to stop it`.
- The Monday footer is now `[i] open stint on Sun 2026-10-04 (last week): mlm status shows how to stop it`.
- `render::WEEK_OVERNIGHT_FOOTER` (a const) became `render::week_overnight_footer(label)`.
  `week_view::overnight_footer` builds the label from the marked row's weekday and date.
  Placement, conditions and footer order are unchanged.
- Tests updated to the new texts: `src/week_view.rs` (four tests plus the shared-row
  golden), `src/render.rs` (`overnight_footers_have_exact_text`, the ASCII test),
  `tests/overnight_visibility_cli.rs`, `tests/start_hint_cli.rs`.
- First failures: `week_by_id_marks_yesterday_row` and
  `week_without_id_follows_the_weekday` in `tests/overnight_visibility_cli.rs`
  (old footer text). The lib-test changes failed to compile until the new function existed.
- Docs: SPEC §7.2 (both blocks and the Monday line), E26, README week example.

## FE1 + FE8 (fixed)

- Two glossary entries added to `GLOSSARY` in `src/cli.rs`, before the `[!], [i]` entry
  so the glossary still ends on the `[i]` text: `so far, started yesterday`, and
  `spans to next day, continues previous day's stint`. Wording checked against SPEC §4.3.1
  (minutes land on the start date) and §7.1.
- Tests: `glossary_is_ascii_and_ends_cleanly` (`src/cli.rs`) and `GLOSSARY_TERMS` plus body
  asserts in `tests/week_target_cli.rs` (`long_help_carries_glossary`).
- The glossary tests were written before the entries, but the first red run stopped at the
  footer binary and the lib tests did not compile, so I did not watch these two fail
  on their own. They passed after the entries were added.
- Docs: SPEC §3.5 glossary paragraph now names the two entries. README only says a glossary
  exists and quotes none of it, so it is unchanged for this part.

## Not done

FE3, FE4, FE5, FE7 accepted at triage and untouched.

## Verification

`cargo fmt --check`, `cargo build`, `cargo test` (all green), and
`cargo clippy --all-targets -- -D warnings` (clean). Smoke on a temp DB:
`status --help` shows the new entries; a seeded `mlm week` renders normally.
The overnight footer itself is covered by the binary-level tests, which seed yesterday's start.

## Notes

- The Monday branch is covered by unit tests only when the real clock is not a Monday,
  as before.
