# doc-drift — final review

- Baseline: `main` at `341ceaa`
- Branch: `doc-drift`, tip `6a8b0e1` (clone `/tmp/cd-scratch/doc-drift-final`, HEAD verified)
- Spec: `docs/dev/specs/2026-10-31-doc-drift.md` (locked), NOTES entries 159-162
- Toolchain in the box: rustc 1.99.0

## Verdict

**ship-with-followups.** Three low findings, none blocking. Two describe
behavior the changeset documents but does not change. One is cosmetic.

## Verification set (run in the clone)

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo build` | ok |
| `cargo test` | 1023 passed, 0 failed |
| `cargo clippy --all-targets -- -D warnings` | clean |

## Correctness

### Source diff outside comments and attributes

I filtered `git diff 341ceaa...HEAD -- src/ tests/ Cargo.toml .github coverage-baseline.json`
down to lines that are not comments, `#[allow]`/`#![allow]` attributes or
blank. What remains:

- `src/commands.rs`: the R1 fix. A new `note_deleted_line` owns the
  backslash branch, `note_recreate_line` loses it, `delete_note` emits
  the new line. Plus two new unit tests and the updated helper test.
- `src/commands.rs`: the `debug_assert!` message text (Milestone 13
  wording). This message only shows on a debug-build panic.
- `src/cli.rs:43`: F9 glossary sentence. `src/cli.rs:205`: F8 `ID` help
  (a doc comment, so clap's help text).
- `src/date.rs`: the MSRV commit's `collapsible_if` rewrite into a let
  chain. I read it and it is equivalent, falling through to
  `parse_date(s)` on the same conditions.
- `src/storage.rs`: one `#[expect]`/`#[allow]` `reason =` string.
- `Cargo.toml`: `rust-version`, `exclude`, a comment. `sign-artifacts.yml`: a comment.
- `coverage-baseline.json`: ratchet up.

The removed allows are seven `dead_code` allows, among them
`#![allow(dead_code)]` in `src/time.rs` and the one on `DbError::Query`.
Removing an allow cannot change codegen, and clippy `-D warnings` stays
clean, so every item they covered is reachable or public. R1 is the only
output change besides F8 and F9.

### R1 by hand

```
$ mlm note 'C:\dir\file'; mlm delete note 4
deleted note (2026-10-31): C:\dir\file        # exit 0, date shown for today
```

The other-day form (`--date 2026-02-10`) prints the same shape. Plain
bodies keep `deleted. to recreate: mlm note '...'`.

### Mutation tests (each reverted with `git checkout`)

| Mutation | Caught by |
|---|---|
| Restore the doubled prefix and `...` (`commands.rs:268`) | both `tests/delete_cli.rs` backslash tests, two unit tests |
| Omit the date when the note is today's | `backslash_note_delete_today_prints_plain_line_with_date` |
| F9 text back to `no start before it` | `tests/week_target_cli.rs` `long_help_carries_glossary` |
| F8 `taken fresh` changed to `fetched fresh` | `delete_id_help_says_fresh_list_and_rejection` |

The new tests pin full strings and fail on each mutation.

## Spec compliance (SPEC §3.9, §6.1, §7.4, E39/E40 vs the binary)

All checked by running `target/debug/mlm` with `MLM_DB_PATH` under the clone.

- Aliases `del`, `n`, `p` work. Bare `mlm delete` prints help, exit 2.
- `--date` works both before and after `ID`.
- List format `<n>  <kind> <HH:MM>` and `<n>  <body>` match.
  `nothing to delete for <date>.` matches. Ordering SQL
  (`storage.rs:390`, `:473`) matches the stated tie rules.
- Recreate lines match: `mlm stop 13:00` for today's `end`,
  `mlm start 08:00 --date 2026-10-30` for another day, `'"'"'` for `'`.
- E39: `id must be 1 or greater (got 0); 1 entry for this date`, exit 1.
- E40: `id 2 is out of range; only 1 entry for this date`, and
  `only 0 entries` on an empty date, exit 1.
- `-1`, `abc`, `4294967296` exit 2 from clap.
- Future and invalid `--date` on `delete` are hard errors, exit 1.
- F5's `TIME is required` text and order are unchanged code. SPEC now states them.

One SPEC sentence overstates the code (finding 1).

## README and SPEC examples touched by this changeset

- F1 `deleted. to recreate: mlm stop 13:00` matches.
- F12 lookback blocks match `src/lookback.rs` output line for line. I
  reproduced both shapes with starts 5 and 6 days back. 2026-09-25 is a Friday.
- F13 `week --notes` for 2026-43 renders `      - refactored the parser`
  under Sat, the same indent as the binary. Export examples list the same notes.
- F11: `date.rs:414` uses the ISO year for a bare week and `:493` the
  calendar year for a bare month. 2025-12-30 is ISO 2026-W01, so the
  README examples hold.
- F14 matches `mlm --help` and `mlm week --help`.
- The README backslash example matches the binary's shape.

## MSRV

`cargo metadata` over the locked graph gives a maximum `rust_version` of
1.95 (`rusqlite_migration` 2.6.0). Every other package declares 1.88 or
lower. Clippy's `incompatible_msrv` lint, a default warn under
`-D warnings`, reads `rust-version = "1.95"` and stays clean, so no std
API newer than 1.95 is used. The box has only stable 1.99, so I could not
build on 1.95 itself.

## Package contents

`cargo package --list` shows no `docs/dev/` file. It still ships
`README.md`, `LICENSE`, `CHANGELOG.md`, `SECURITY.md`, `docs/SIGNING.md`,
`Cargo.lock`, `src/`, `tests/` and `examples/`.

## Security: the recreate line

I replayed each printed line with `bash -c` and `fish -c`, then compared
the stored body with the original.

| Body | bash | fish |
|---|---|---|
| `it's a test`, `a'b'c''` | ok | ok |
| `cost $HOME \`id\` $(id) ; rm -rf / & \| > < * ? ~ !! # "dq"` | ok, nothing expanded | ok |
| `--date 2026-01-01 leading dash`, `-1 x`, `-- y`, `--verbose x`, `x -- y` | ok | ok |
| tab inside | ok | ok |
| exactly `--`, `-h`, `--help`, `-v`, `--verbose`, `-d` | does not recreate (finding 1) | same |
| backslash | plain line, no command | — |

No body made a shell run anything. Newlines cannot reach the quoting
path because the insert path collapses them. The plain backslash line is
not a command, so it needs no quoting. SQL is all parameterized. The only
`format!` SQL builds in `db.rs` use fixed table names.

## Findings

### 1. Low: SPEC §3.9 overstates flag protection for the recreate line

- Location: `docs/dev/SPEC.md:597-599`, and `src/commands.rs:289` for the behavior
- The SPEC says one token stops a body that "starts with `--verbose`, or
  holds `--`" from being read as flags. A body that is exactly a flag
  (`--verbose`, `-v`, `-h`, `--help`, `-d`) or exactly `--` prints a line
  that does not recreate it. `mlm note '--help'` prints help and exits 0.
  The others exit 2. Nothing is lost silently and nothing runs, but the
  line is not ready to run as the SPEC promises.
- The code is unchanged by this changeset. SPEC text from this changeset
  now claims the case works.
- Fix: narrow the sentence now. For example: "a body that starts with a
  flag and goes on past it, such as `--verbose x`". Or, as a follow-up
  and a user call since it changes output, emit `mlm note [--date D] -- '<body>'`.
  `mlm note -- '-h'` already stores `-h`.

### 2. Low: a legacy multi-line body with a backslash prints more than one line

- Location: `src/commands.rs:268`
- The old fallback printed `body.lines().next()`. The new one prints the
  whole body. Normalization landed on 2026-09-14 (`4c1e1f9`), a day after
  `v0.1.0`, so a note stored under `v0.1.0` can hold a newline. If it also
  holds a backslash, the delete line spans several lines. SPEC §7.4 and
  spec §3 R1 say one line. Only one release's data can hit this.
- Fix: replace `\r`/`\n` runs with a space in that branch, or accept the
  gap and say so in SPEC §7.4. Leaving it as is is reasonable under the
  edge-case triage rule.

### 3. Cosmetic: unwrapped doc-comment lines after the F20 rewrites

- Location: `src/status.rs:289` (109 columns), `src/status.rs:450`,
  `src/commands.rs:364`, `src/cli.rs:757`
- The rewrites left lines past the surrounding wrap width. rustfmt does
  not wrap comments.
- Fix: rewrap the paragraphs.

## Not re-filed

F21, bare `§N` comments, test-plan labels such as `§7.1`/`§7.10` in
`src/week.rs` tests, and the unbuilt `DbError::Query` were dropped
upstream. The coordinator banners the four unbannered `doc-drift-*` docs at
close-out, per spec §4 F6.
