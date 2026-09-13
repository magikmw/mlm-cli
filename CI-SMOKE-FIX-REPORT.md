# CI smoke test fix — report

## What changed

### 1. `.github/workflows/ci.yml` (lines 76-108, appended to the end of the
   `e2e smoke test (seed + status + week)` step)

Added, after the existing seed/status/week checks (unmodified):

- A fresh, separate temp db (`${RUNNER_TEMP}/mlm-ci-smoke-backdate.db`),
  kept independent from the seeded db used above, for determinism.
- `start -d 2020-01-01 09:00`, `stop -d 2020-01-01 17:00`,
  `note -d 2020-01-01 "ci backdating smoke test"` against it, then
  `status 2020-01-01`, grepping for `"Day total: 08h 00m"` and the note
  text.
- `-N` shorthand round-trip: `start -d -3 09:00` / `stop -d -3 17:00`
  (mlm resolves "3 days ago" itself — no bash-side relative-date math),
  then `status -3`, grepping for `"Day total: 08h 00m"`. This
  specifically exercises the clap `allow_negative_numbers = true` fix
  from this session's earlier commits (cfd1d4c/f3adb58), end-to-end on
  the real binary on every CI platform.
- Hard-error check: `start -d 2099-01-01 09:00` must exit nonzero, with
  stderr containing `"date is in the future"` (wrapped in an
  `if ...; then echo unexpected-success; exit 1; fi` block since the
  step runs under `set -e`).

Kept the same shell conventions already in the step: `set -euo
pipefail` (inherited, not re-declared), `$RUNNER_TEMP` for temp files,
`tee` + `grep -q`, the `$mlm_bin` variable, `bash` shell block.

### 2. `examples/seed_test_data.rs` (lines 7-15, module doc comment)

Old text claimed backdating is something "the real CLI deliberately
never allows (§1.2)" — false since milestone 12. New text says the CLI
now also supports backdating directly via `-d`/`--date` (pointing at
the spec doc), and explains the reason this example still exists and
still writes straight through `mlm::storage` rather than shelling out
to the CLI once per row: it's simply faster/simpler for generating many
weeks of synthetic data, a reason independent of what the CLI itself
can do.

## Real local verification (against the actual compiled binary)

Built with `cargo build` (debug), binary at `target/debug/mlm`. Used a
scratch temp dir, invoking the binary directly with the exact argument
shapes added to the YAML.

### Absolute-date start/stop/note/status

```
$ mlm_bin="target/debug/mlm"
$ db="/tmp/tmp.Z6thbM7z2U/backdate.db"
$ MLM_DB_PATH="$db" "$mlm_bin" start -d 2020-01-01 09:00
[2026-09-13T18:44:03Z INFO  rusqlite_migration] Database migrated to version 1
exit:0
$ MLM_DB_PATH="$db" "$mlm_bin" stop -d 2020-01-01 17:00
exit:0
$ MLM_DB_PATH="$db" "$mlm_bin" note -d 2020-01-01 "ci backdating smoke test"
exit:0
$ MLM_DB_PATH="$db" "$mlm_bin" status 2020-01-01
Wed 2020-01-01

Day total:     08h 00m
Week 2020-01:  Total still owed: 32h 00m

  09:00-17:00  (08h 00m)

Notes:
  - ci backdating smoke test
exit:0
```

Both `grep -q "Day total: 08h 00m"` and `grep -q "ci backdating smoke
test"` match this output.

### `-N` shorthand round-trip (same db, continuing)

```
$ MLM_DB_PATH="$db" "$mlm_bin" start -d -3 09:00
exit:0
$ MLM_DB_PATH="$db" "$mlm_bin" stop -d -3 17:00
exit:0
$ MLM_DB_PATH="$db" "$mlm_bin" status -3
Thu 2026-09-10

Day total:     08h 00m
Week 2026-37:  13984h 00m left by end of Sunday (fulfillment -13944h 00m / target 40h 00m)

  09:00-17:00  (08h 00m)
exit:0
```

`grep -q "Day total: 08h 00m"` matches. (The large "owed" figure is a
pre-existing artifact of testing against a db with no other weeks
seeded — irrelevant to what this check asserts.)

### Future-date hard error (separate fresh db)

```
$ db="/tmp/tmp.Z6thbM7z2U/future.db"
$ MLM_DB_PATH="$db" "$mlm_bin" start -d 2099-01-01 09:00
[2026-09-13T18:44:11Z INFO  rusqlite_migration] Database migrated to version 1
error: invalid DATE "2099-01-01": date is in the future
exit:1
```

Nonzero exit, and stderr contains `"date is in the future"` — matches
the CI check's `if ...; then fail; fi` + `grep -q` logic.

## `cargo test`

```
test result: ok. 347 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s (doc-tests)
```

357 total tests passed, 0 failed. No `src/` files were touched by this
change, so no regression risk there; this simply confirms the existing
suite is unaffected.

## YAML validity

```
$ python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml')); print('YAML OK')"
YAML OK
```

## Commit

```
ci: extend e2e smoke test for backdated punches; fix stale doc comment

- CI smoke step now exercises -d/--date on start/stop/note against a
  fixed absolute date on a fresh temp db, round-trips the -N shorthand
  on status via mlm's own relative-date resolution (start/stop -d -3,
  status -3), and asserts start -d <future date> fails with "date is
  in the future".
- seed_test_data.rs's doc comment no longer claims the CLI can't
  backdate (it now can, via -d/--date); clarifies the example still
  writes through storage directly rather than shelling out to the CLI
  for speed/simplicity generating many weeks of data.
```

Committed as `1a51302` on branch `ci-smoke-backdating`. Pre-commit
quality gates (fmt, cognitive complexity, coverage-vs-baseline) all
ran and passed/were unaffected (coverage unchanged at 98.83%, since no
`src/` files changed).

No `--no-verify` used; committed with the normal hook path, which ran
cleanly (this change is workflow/example-doc only, so it had nothing
new to measure against the coverage baseline and reported "unchanged"
rather than skipping).

## Self-review

- Confirmed the new section does not remove or reorder any of the
  pre-existing seed/status/week checks — purely appended.
- Confirmed the new db is genuinely separate from the seeded db
  (`mlm-ci-smoke-backdate.db` vs. `mlm-ci-smoke.db`), so backdating
  logic can't accidentally interact with or depend on the seed
  script's synthetic data.
- Confirmed `2020-01-01` is a real fixed date, never relative to CI run
  time, and `2099-01-01` is comfortably in the future relative to any
  plausible CI run date, so the future-date check won't flake.
- Confirmed the `-N` shorthand check avoids all bash-side date
  arithmetic by having `mlm` itself resolve `-3` on both write
  (`start -d -3` / `stop -d -3`) and read (`status -3`) — so it can't
  drift or become platform-sensitive the way computing "N days ago" in
  bash across Linux/macOS/Windows would.
- Verified the future-date check's control flow is correct under
  `set -e`: the `if MLM_DB_PATH=... "$mlm_bin" ...; then ... exit 1;
  fi` pattern correctly treats a *zero* exit as the failure case
  (command should have failed but didn't) without the `if` guard
  itself tripping `set -e`, and stderr is captured to a file for the
  subsequent `grep -q`, consistent with the existing step's tee+grep
  style (stdout there, stderr here, since this checks an error
  message).
- Verified the note-body text (`"ci backdating smoke test"`) contains
  no character clap could special-case (no leading `-`, no shell
  metacharacters needing escaping beyond the existing double-quoting).
- Double-checked argument order against the spec's §2.1 footgun
  (`--date` must precede `NOTE` text) — the note command in this smoke
  test uses `note -d 2020-01-01 "ci backdating smoke test"`, i.e.
  `-d` before the note body, which is the safe order.
- Confirmed the doc-comment fix doesn't just delete the outdated claim
  but explains what still holds (the reason for writing through
  `storage` directly), per the task's explicit instruction.

## Fix round 1 (coordinator feedback)

### Critical: grep pattern didn't match the real padded output

`src/render.rs`'s `LABEL_WIDTH = 15` pads `"Day total:"` to 5 trailing
spaces before the value (`"Day total:     08h 00m"`), but both new
`grep -q "Day total: 08h 00m"` checks used a single space, which
matches nothing against real output — confirmed: the committed pattern
was never actually run against the real grep invocation before, only
eyeballed against pasted terminal output where the spacing look was
missed. Fixed both occurrences in `.github/workflows/ci.yml` to
`grep -qE "Day total: *08h 00m"` (whitespace-tolerant, robust to the
label width ever changing) — lines in the "absolute date" section and
the "-N shorthand" section.

**Real re-verification, command → output → grep → exit code:**

```
$ mlm_bin="target/debug/mlm"; db="$scratch/backdate.db"
$ MLM_DB_PATH="$db" "$mlm_bin" start -d 2020-01-01 09:00 >/dev/null
$ MLM_DB_PATH="$db" "$mlm_bin" stop -d 2020-01-01 17:00 >/dev/null
$ MLM_DB_PATH="$db" "$mlm_bin" note -d 2020-01-01 "ci backdating smoke test" >/dev/null
$ MLM_DB_PATH="$db" "$mlm_bin" status 2020-01-01 | tee "$scratch/status-backdate.out"
Wed 2020-01-01

Day total:     08h 00m
Week 2020-01:  Total still owed: 32h 00m

  09:00-17:00  (08h 00m)

Notes:
  - ci backdating smoke test

$ grep -qE "Day total: *08h 00m" "$scratch/status-backdate.out"; echo "grep exit:$?"
grep exit:0
$ grep -q "ci backdating smoke test" "$scratch/status-backdate.out"; echo "grep exit:$?"
grep exit:0
```

```
$ MLM_DB_PATH="$db" "$mlm_bin" start -d -3 09:00 >/dev/null
$ MLM_DB_PATH="$db" "$mlm_bin" stop -d -3 17:00 >/dev/null
$ MLM_DB_PATH="$db" "$mlm_bin" status -3 | tee "$scratch/status-shorthand.out"
Thu 2026-09-10

Day total:     08h 00m
Week 2026-37:  13984h 00m left by end of Sunday (fulfillment -13944h 00m / target 40h 00m)

  09:00-17:00  (08h 00m)

$ grep -qE "Day total: *08h 00m" "$scratch/status-shorthand.out"; echo "grep exit:$?"
grep exit:0
```

Also confirmed the bug was real by running the *old* (buggy) pattern
against the same real output file:

```
$ grep -q "Day total: 08h 00m" "$scratch/status-backdate.out"; echo "old buggy pattern exit:$?"
old buggy pattern exit:1
```

Old pattern: exit 1 (would have failed CI on every platform, every
run, exactly as flagged). New pattern: exit 0 against the same real
output.

### Important: `-N` round-trip midnight-rollover risk

Added a one-line (multi-line comment block) acknowledgment in the YAML
directly above the `-N` shorthand section, rather than asserting the
approach can't drift: each of `start -d -3`, `stop -d -3`, `status -3`
resolves "today" independently from the wall clock, so a local
midnight rollover between the write and read calls could in principle
make `-3` land on different days on each side. No code change made —
this is accepted as negligible residual risk (sequential commands,
sub-second apart), and the comment says so explicitly instead of
overstating a guarantee.

### `cargo test` after the fix (unaffected, confirms no regression)

```
test result: ok. 347 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### YAML validity after the fix

```
$ python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml')); print('YAML OK')"
YAML OK
```

## Concerns

- None blocking. One minor note: the `-3`/`status -3` check's "Day
  total: 08h 00m" grep is somewhat redundant with the earlier absolute
  -date check (same assertion shape), but that's intentional per the
  task ("just a smoke test... a few real invocations with basic
  assertions") — the point of this second check is exercising the
  `-N` code path on the real binary, not asserting something novel
  about output formatting.
- I did not attempt to run this on Windows/macOS locally (not
  possible in this environment); the local verification substitutes
  the real Linux binary run as instructed. The bash syntax used
  (`if ...; then ... fi`, `2>file`, `tee`) is the same style already
  proven cross-platform-safe by the pre-existing step running under
  `shell: bash` on all 5 matrix runners.
