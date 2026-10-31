# doc-drift — fix pass

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

- Base: worktree rebased onto `doc-drift` at `e58dbdf` (it started on `main` at `962ac9f`)
- Inputs: `doc-drift-final-review.md`, `doc-drift-fresh-eyes.md`, NOTES entry 163

## Verification

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo build` | ok |
| `cargo test` | 1024 passed, 0 failed (1023 before plus one new test) |
| `cargo clippy --all-targets -- -D warnings` | clean |

## Final review

### (a) Flag-only note bodies

- Changed `docs/dev/SPEC.md` §3.9 recreate-line bullet (the reviewer's "§7.4" sentence sits in §3.9, around line 597).
- New text says one token protects a body that starts with a flag and goes on past it, such as `--verbose x`.
- New text says a body that is exactly one of mlm's flags, or exactly `--`, does not replay, because the shell passes the quoted token intact but mlm's parser still reads it as a flag.
- Recreate line output unchanged. No `--` added.
- No test, since this is docs only.

### (b) Backslash fallback with a legacy newline

- Failing test first: `commands::tests::backslash_body_with_legacy_newlines_prints_one_line`. Body `a\b\r\n\nc\rd`, dated 2026-02-10.
- It failed with `left: "deleted note (2026-02-10): a\\b\r\n\nc\rd\n"`, expected `a\\b c d`.
- Fix in `src/commands.rs` `note_deleted_line`. The backslash branch splits on `\r`/`\n`, drops empty parts and joins with one space. Each run collapses to one space, as `storage::normalize_newlines` does on insert.
- I wrote a local collapse because `normalize_newlines` is private to `src/storage.rs`, which this dispatch does not own.
- The doc comment on `note_deleted_line` gains two sentences on the collapse.

### (c) Doc-comment rewraps

Rewrapped with no wording change:

- `src/status.rs`, the `render` doc comment (was 109 columns)
- `src/status.rs`, the `resolve` doc comment
- `src/commands.rs`, the shared `start`/`stop` doc comment (line 364 at the reviewed tip `6a8b0e1`)
- `src/cli.rs`, the `note_date_flag_after_body_text_is_absorbed_into_the_note_body` test doc comment

I also rewrapped one more comment in the same class. The `delete_note` doc comment in `src/commands.rs` broke `The delete stays when` / `the output fails.` across two short lines, and it now reads as one line. The wording is unchanged.

## Fresh-eyes, README only

### (1) Clone step

- "From source" now says to clone the repository and build inside it. Its code block opens with `git clone https://github.com/magikmw/mlm-cli.git` and `cd mlm-cli`.
- "Build (local dev)" says to run inside a clone, and its block opens with the same two lines.
- The URL matches `repository` in `Cargo.toml`.

### (2) Dev example time

- `cargo run -- start 9:00` is now `start 09:00`.

### (3) Quoting note on delete

- The Features bullet now says the command uses POSIX single quotes, so check it before running it in Windows cmd or PowerShell. It also says a note mlm cannot quote safely prints with no command, and you retype it.
- The `delete note` section replaces the backslash-specific sentence with a general one. The recreate command uses POSIX single quotes, cmd and PowerShell quote differently, and an unquotable note prints its date and body with no command. After the example it tells the reader to retype with `mlm note --date 2026-09-11 '...'` in their own shell's quoting.
- It does not say which characters trigger the fallback.

### (9) Status block placement

- Code order is from `status::render` and `compose_anomaly_lines`. Header, day total and week line come first. Then the date's own `[!]` block, the lookback blocks (`[i]` yesterday hint and older unclosed starts), the carry-in marker, the stint list and notes.
- "appends a block" is now "prints a block". A new sentence says these blocks print above the stint list, after any `[!]` lines for today itself.
- "the last line of the page names it" now says one more line names it, the last of the `[!]` and `[i]` lines, above the stint list.

## Not done

- The nine other fresh-eyes findings, per the dispatch.

## Notes for the coordinator (not fixed)

- A legacy body with a newline but no backslash still goes to `note_recreate_line`. A debug build panics on its `debug_assert!`. A release build prints a recreate line that spans several lines. It is the same one-release data gap as (b), but on the other branch.
- The reviewer cited the flag sentence as SPEC §7.4. It lives in §3.9. §7.4 only holds the output examples.
