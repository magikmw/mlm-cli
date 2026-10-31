# doc-drift Task 1 (code) — completion report

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

- Spec: `docs/dev/specs/2026-10-31-doc-drift.md` (locked)
- Base: rebased onto `doc-drift` @ `7b283e5` (the worktree started on `main`)
- Gates: `cargo fmt --check`, `cargo build`, `cargo test` (all suites green, lib 855), `cargo clippy --all-targets -- -D warnings`, all clean

## Implemented

| Item | Change |
|---|---|
| R1 | New `note_deleted_line` in `src/commands.rs` builds the whole `delete note` line. A backslash body prints `deleted note (<date>): <body>`, with the date always shown and no `...`. Every other body keeps `deleted. to recreate: <note_recreate_line>`. `note_recreate_line` lost its backslash branch. |
| F8 | `DeleteEntryArgs.id` help reads the spec's text, ending `... is rejected` (clap drops the final period). |
| F9 | Glossary first sentence reads `An end punch with no open start to pair with.` The other two sentences stay. |
| F20 | Every `PLAN.md`, milestone, interface-contract, `<name> spec §N`, unqualified `spec §N`, `plan §N`, `plans/...`, `Finding N` and `Task N` citation under `src/` is rewritten to a SPEC.md section or dropped. Wrong `SPEC §N` targets are fixed: glossary §4.2 to §3.5, log level §4.1 to §3, carry-in marker §3.3 to §7.3, `week target` echo §3.5 to §3.7, lookback window §3.1 to §7.1, history §3.4 dropped. The `status.rs` §7.4 claim and the `date.rs` `Month` domain note are fixed. |
| F3 follow-ons | `Cargo.toml` binstall comment and `.github/workflows/sign-artifacts.yml` cite `docs/SIGNING.md`. |
| Package | `exclude` names `docs/dev/` and drops `NOTES.md`, `PLAN.md`, `plans/`. `cargo package --list --allow-dirty` shows nothing under `docs/dev/`, and `docs/SIGNING.md` is present. Before the fix it listed SPEC.md, `docs/dev/README.md` and all 17 specs. |

## Tests (first failure)

- `tests/delete_cli.rs` (new): `backslash_note_delete_today_prints_plain_line_with_date` and `backslash_note_delete_other_day_prints_plain_line_with_date` failed first with `left: "deleted. to recreate: deleted note (2026-10-31): path C:\\dir\\file...\n"`. `delete_id_help_says_fresh_list_and_rejection` failed on the old help text.
- `tests/week_target_cli.rs` `long_help_carries_glossary` gained the F9 sentence assert and failed on the old text.
- `src/commands.rs` unit tests: `backslash_body_falls_back_to_plain_description` takes the new text. `backslash_body_today_still_shows_its_date` and `plain_body_deleted_line_keeps_recreate_prefix` are new.

## Deviations

1. Removed `#![allow(dead_code)]` from seven modules, not just `src/time.rs`. `date.rs`, `storage.rs`, `week.rs`, `render.rs`, `stint.rs` and `week_target.rs` carried the same false "built ahead of its consumers (Milestone N)" note as `time.rs`, and `db.rs` had a variant-level allow with a "Milestone 4+" reason. Clippy `-D warnings` stays clean with every allow gone. The spec's F20 rule ("other false statements in those comments are fixed too") covers it, but the spec names only `time.rs`.
2. `tests/delete_cli.rs` is a new file.

## Open questions

1. Bare `§N` citations with no `spec`/`SPEC` prefix are outside F20's rule as written, and many of them name archived plan sections, not SPEC.md. Examples include the `week.rs` test headers (`§7.8`, `§7.9`, `§7.2 first tracked week`), the `status.rs` test headers (`§2`, `§3`, `§4`, `§5`), `status.rs` `§3.7` and `§4's three-way classification`, `anomalies.rs` `§3.2.1`/`§3.2.2`, and `lookback.rs`/`storage.rs` `§1.2's residual gap`. I fixed only the ones that sat next to a site I was already editing, or named a section that was clearly wrong (`week_target.rs` `§3.5`, `status.rs` `§4.5`, `stint.rs` `§3.2 step N`, `cli.rs` `§2.1 clap footgun`). A follow-up could apply the same rule to bare `§N`.
2. Test-plan labels (`T1`-`T125`, `F4`, `E5`-style tags) from archived plans remain in test comments. They name no document, so I left them.
3. `DbError::Query` is never constructed outside a test. Its stale allow is gone and clippy does not flag it because it is `pub`. Removing the variant would be a code change outside this task.
