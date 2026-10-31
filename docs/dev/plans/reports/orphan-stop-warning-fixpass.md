# orphan-stop-warning — fix pass

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

- **Input**: `docs/dev/plans/reports/orphan-stop-warning-final-review.md` (F1-F3)
- **Base**: worktree rebased onto `orphan-stop-warning` tip `a6d2f37` (it was not an ancestor at start)
- **Files touched**: `docs/dev/SPEC.md`, `README.md`, `tests/orphan_stop_cli.rs`, this report

## F1 — SPEC §4.3 lead-in

- **Changed**: `docs/dev/SPEC.md` lines 461-463 now read "`start` and `stop` print stderr warnings at insert time (§7.4), and the punch is stored either way. The edge cases are:". The list colon now hangs off its own sentence.
- **Test**: none. This is doc wording, and no test reads SPEC prose.

## F2 — README unwrapped line

- **Changed**: re-wrapped the stdout/stderr paragraph (`README.md` lines 124-129). Every line is now 70 columns or fewer. Wording is unchanged.
- **Choice**: the paragraph breaks before `` `delete note`/`delete punch` `` so the code spans stay on one line.
- **Test**: none. This is a cosmetic wrap.

## F3 — bare `mlm stop` binary test

- **Added**: `bare_stop_warns_with_status_page_hints` in `tests/orphan_stop_cli.rs`, plus helper `stored_end`.
  - Setup is `start 22:00 --date -2`, then `stop` with no TIME. Neither today nor yesterday has an open start.
  - The test reads the date and `HH:MM` from the stored end row, so it never races the clock or midnight.
  - It asserts exit 0, empty stdout, and the exact block for the stored date and minute (`assert_stop` + `block`).
  - It asserts three stderr lines and the header prefix `warning: orphaned end on <weekday date> at `.
  - It asserts each hint line appears verbatim on `mlm status <date>`'s page.
- **Red run**: the test passed against shipped code. I then swapped `&mut std::io::stderr()` for `&mut std::io::sink()` in `commands::stop` and reran it. It failed at `assert_stop` with `left: ""` against the expected block, which is the right reason. I restored the line and `git diff src/` came back empty.

## Verification

| Check | Result |
|---|---|
| `cargo fmt` | clean, no changes |
| `cargo build` | ok |
| `cargo test` | all pass (761 lib, 11 in `orphan_stop_cli`) |
| `cargo clippy --all-targets -- -D warnings` | clean |
| pre-commit hook (fmt, complexity ≤ 15, coverage ratchet) | ran on commit |

## Notes, not fixed

- `README.md` line 131 ("I recommend using a 1 character shell alias…") also runs past 72 columns. It predates this changeset and is out of scope for F2.
